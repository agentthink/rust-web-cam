use serde::Serialize;

/// 统一 API 响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    /// 错误响应
    pub fn error(code: u16, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
        }
    }
}