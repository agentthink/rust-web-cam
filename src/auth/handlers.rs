use std::sync::Arc;
use axum::{
    extract::{Extension, Path, State},
    Json,
};
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
};
use password_hash::phc::SaltString;
use password_hash::PasswordHasher;
use uuid::Uuid;
use crate::api::FullState;
use crate::error::{AppError, Result as AppResult};
use crate::auth::models::*;
use crate::auth::middleware::CurrentUser;
use crate::auth::repository::{UserRepository, RoleRepository, PermissionRepository};

// ═══════════════════════════════════════════════════════════════
// 认证 Handlers
// ═══════════════════════════════════════════════════════════════

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<Arc<FullState>>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    tracing::debug!("[Auth] Login attempt for user: {}", req.username);

    // 查找用户
    let user = UserRepository::find_by_username(&*state.auth.repo, &req.username)
        .await
        .map_err(|e| {
            tracing::debug!("[Auth] Login failed - DB error: {}", e);
            AppError::Auth(e.to_string())
        })?
        .ok_or_else(|| {
            tracing::debug!("[Auth] Login failed - user not found: {}", req.username);
            AppError::Auth("Invalid credentials".to_string())
        })?;

    tracing::debug!("[Auth] User found: {} (id={})", user.username, user.id);

    // 验证密码
    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| {
            tracing::debug!("[Auth] Login failed - password hash parse error: {}", e);
            AppError::Auth(format!("Password hash error: {}", e))
        })?;

    let password_valid = Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .is_ok();

    if !password_valid {
        tracing::debug!("[Auth] Login failed - invalid password for user: {}", req.username);
        return Err(AppError::Auth("Invalid credentials".to_string()));
    }

    tracing::debug!("[Auth] Password verified for user: {}", user.username);

    // 获取角色
    let roles = UserRepository::get_user_roles(&*state.auth.repo, user.id)
        .await
        .map_err(|e| {
            tracing::debug!("[Auth] Login failed - get roles error: {}", e);
            AppError::Auth(e.to_string())
        })?;

    tracing::debug!("[Auth] User {} has {} roles: {:?}", user.username, roles.len(), roles);

    let user_info = UserInfo {
        id: user.id,
        username: user.username.clone(),
        roles: roles.clone(),
    };

    let (access_token, refresh_token) =
        crate::auth::jwt::generate_tokens(&user_info, &state.auth.jwt_config)
            .map_err(|e| {
                tracing::debug!("[Auth] Login failed - JWT generation error: {}", e);
                AppError::Auth(e.to_string())
            })?;

    tracing::info!("[Auth] Login successful: {} (id={})", user.username, user.id);

    Ok(Json(ApiResponse::success(LoginResponse {
        access_token,
        refresh_token,
        expires_in: state.auth.jwt_config.access_token_expire_minutes * 60,
        token_type: "Bearer".to_string(),
        user: user_info,
    })))
}

/// POST /api/v1/auth/refresh
pub async fn refresh(
    State(state): State<Arc<FullState>>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let claims = crate::auth::jwt::decode_token(&req.refresh_token, &state.auth.jwt_config.secret)
        .map_err(|e| AppError::Auth(e.to_string()))?;

    if claims.token_type != "refresh" {
        return Err(AppError::Auth("Invalid token type".to_string()));
    }

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Auth("Invalid user ID".to_string()))?;

    // ✅ UserRepository:: 限定
    let user = UserRepository::find_by_id(&*state.auth.repo, user_id)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?
        .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    // ✅ UserRepository:: 限定
    let roles = UserRepository::get_user_roles(&*state.auth.repo, user_id)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    let user_info = UserInfo { id: user.id, username: user.username.clone(), roles };
    let (access_token, refresh_token) = crate::auth::jwt::generate_tokens(&user_info, &state.auth.jwt_config)
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(Json(ApiResponse::success(LoginResponse {
        access_token, refresh_token,
        expires_in: state.auth.jwt_config.access_token_expire_minutes * 60,
        token_type: "Bearer".to_string(), user: user_info,
    })))
}

/// GET /api/v1/auth/me
pub async fn me(
    Extension(current_user): Extension<CurrentUser>,
) -> Json<ApiResponse<UserInfo>> {
    Json(ApiResponse::success(current_user.0))
}

// ═══════════════════════════════════════════════════════════════
// 用户管理 Handlers
// ═══════════════════════════════════════════════════════════════

/// GET /api/v1/users
pub async fn list_users(
    State(state): State<Arc<FullState>>,
) -> AppResult<Json<ApiResponse<Vec<User>>>> {
    // ✅ UserRepository:: 限定
    let mut users = UserRepository::list(&*state.auth.repo)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    for u in &mut users {
        u.password_hash = "***".to_string();
    }

    Ok(Json(ApiResponse::success(users)))
}

/// POST /api/v1/users
pub async fn create_user(
    State(state): State<Arc<FullState>>,
    Json(req): Json<CreateUserRequest>,
) -> AppResult<Json<ApiResponse<User>>> {
    // 检查用户名唯一性 — find_by_username 只有 UserRepository 有
    if UserRepository::find_by_username(&*state.auth.repo, &req.username)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?
        .is_some()
    {
        return Err(AppError::BadRequest("Username already exists".to_string()));
    }

    // 哈希密码
    let salt = SaltString::generate();
    let password_hash = Argon2::default()
        .hash_password(req.password.as_bytes())
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {}", e)))?
        .to_string();

    // ✅ UserRepository:: 限定
    let mut user = User::new(req.username, password_hash);
    UserRepository::create(&*state.auth.repo, &user)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    if let Some(role_ids) = req.role_ids {
        UserRepository::set_user_roles(&*state.auth.repo, user.id, &role_ids)
            .await
            .map_err(|e| AppError::Auth(e.to_string()))?;
    }

    user.password_hash = "***".to_string();
    Ok(Json(ApiResponse::success(user)))
}

/// GET /api/v1/users/:id
pub async fn get_user(
    State(state): State<Arc<FullState>>,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<UserWithRoles>>> {
    // ✅ UserRepository:: 限定
    let user = UserRepository::find_by_id(&*state.auth.repo, user_id)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let role_names = UserRepository::get_user_roles(&*state.auth.repo, user_id)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    let mut user_safe = user;
    user_safe.password_hash = "***".to_string();

    Ok(Json(ApiResponse::success(UserWithRoles { user: user_safe, roles: role_names })))
}

/// PUT /api/v1/users/:id
pub async fn update_user(
    State(state): State<Arc<FullState>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<User>>> {
    // ✅ UserRepository:: 限定
    let mut user = UserRepository::find_by_id(&*state.auth.repo, user_id)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if let Some(username) = &req.username {
        if let Some(existing) = UserRepository::find_by_username(&*state.auth.repo, username)
            .await
            .map_err(|e| AppError::Auth(e.to_string()))?
        {
            if existing.id != user_id {
                return Err(AppError::BadRequest("Username already exists".to_string()));
            }
        }
        user.username = username.clone();
    }

    if let Some(password) = req.password {
        let salt = SaltString::generate();
        user.password_hash = Argon2::default()
            .hash_password(password.as_bytes())
            .map_err(|e| AppError::BadRequest(format!("Password hash error: {}", e)))?
            .to_string();
    }

    user.updated_at = chrono::Utc::now();

    // ✅ UserRepository:: 限定
    UserRepository::update(&*state.auth.repo, &user)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    user.password_hash = "***".to_string();
    Ok(Json(ApiResponse::success(user)))
}

/// DELETE /api/v1/users/:id
pub async fn delete_user(
    State(state): State<Arc<FullState>>,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<()>>> {
    // ✅ UserRepository:: 限定
    UserRepository::delete(&*state.auth.repo, user_id)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// PUT /api/v1/users/:id/roles
pub async fn assign_user_roles(
    State(state): State<Arc<FullState>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<AssignRolesRequest>,
) -> AppResult<Json<ApiResponse<Vec<String>>>> {
    UserRepository::set_user_roles(&*state.auth.repo, user_id, &req.role_ids)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    let roles = UserRepository::get_user_roles(&*state.auth.repo, user_id)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(Json(ApiResponse::success(roles)))
}

// ═══════════════════════════════════════════════════════════════
// 角色管理 Handlers
// ═══════════════════════════════════════════════════════════════

/// GET /api/v1/roles
pub async fn list_roles(
    State(state): State<Arc<FullState>>,
) -> AppResult<Json<ApiResponse<Vec<Role>>>> {
    // ✅ RoleRepository:: 限定
    let roles = RoleRepository::list(&*state.auth.repo)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(Json(ApiResponse::success(roles)))
}

/// POST /api/v1/roles
pub async fn create_role(
    State(state): State<Arc<FullState>>,
    Json(req): Json<CreateRoleRequest>,
) -> AppResult<Json<ApiResponse<Role>>> {
    let role = Role::new(req.name, req.description);

    // ✅ RoleRepository:: 限定
    RoleRepository::create(&*state.auth.repo, &role)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    if let Some(perm_ids) = req.permission_ids {
        RoleRepository::set_role_permissions(&*state.auth.repo, role.id, &perm_ids)
            .await
            .map_err(|e| AppError::Auth(e.to_string()))?;
    }

    Ok(Json(ApiResponse::success(role)))
}

/// PUT /api/v1/roles/:id/permissions
pub async fn set_role_permissions(
    State(state): State<Arc<FullState>>,
    Path(role_id): Path<Uuid>,
    Json(req): Json<SetPermissionsRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    // ✅ RoleRepository:: 限定
    RoleRepository::set_role_permissions(&*state.auth.repo, role_id, &req.permission_ids)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════
// 权限管理 Handlers
// ═══════════════════════════════════════════════════════════════

/// GET /api/v1/permissions
pub async fn list_permissions(
    State(state): State<Arc<FullState>>,
) -> AppResult<Json<ApiResponse<Vec<Permission>>>> {
    // ✅ PermissionRepository:: 限定
    let permissions = PermissionRepository::list(&*state.auth.repo)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(Json(ApiResponse::success(permissions)))
}

/// POST /api/v1/permissions
pub async fn create_permission(
    State(state): State<Arc<FullState>>,
    Json(req): Json<CreatePermissionRequest>,
) -> AppResult<Json<ApiResponse<Permission>>> {
    let permission = Permission::new(req.name, req.description);

    // ✅ PermissionRepository:: 限定
    PermissionRepository::create(&*state.auth.repo, &permission)
        .await
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(Json(ApiResponse::success(permission)))
}