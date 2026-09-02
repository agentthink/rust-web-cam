<template>
  <div class="login-page">
    <div class="login-bg">
      <div class="grid-overlay"></div>
      <div class="glow-orb orb-1"></div>
      <div class="glow-orb orb-2"></div>
    </div>

    <div class="login-card">
      <div class="login-card-header">
        <div class="login-logo">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M12 2L2 7l10 5 10-5-10-5z"/>
          <path d="M2 17l10 5 10-5"/>
          <path d="M2 12l10 5 10-5"/>
        </svg>
        <span class="login-brand">Rust<span>Cam</span></span>
      </div>
      <el-button text class="theme-toggle" @click="theme.toggle()" :title="theme.isDark ? '切换到浅色模式' : '切换到深色模式'">
        <el-icon v-if="theme.isDark"><Sunny /></el-icon>
        <el-icon v-else><Moon /></el-icon>
      </el-button>
    </div>

      <div class="login-title">欢迎回来</div>
      <div class="login-subtitle">视频监控管理平台</div>

      <el-form class="login-form" @submit.prevent="handleLogin">
        <div class="form-field">
          <label class="form-label">用户名</label>
          <el-input
            v-model="form.username"
            placeholder="请输入用户名"
            size="large"
            :prefix-icon="User"
            autocomplete="off"
          />
        </div>
        <div class="form-field">
          <label class="form-label">密码</label>
          <el-input
            v-model="form.password"
            type="password"
            placeholder="请输入密码"
            size="large"
            show-password
            :prefix-icon="Lock"
            autocomplete="off"
            @keyup.enter="handleLogin"
          />
        </div>
        <el-button
          type="primary"
          size="large"
          :loading="loading"
          class="login-btn"
          @click="handleLogin"
        >
          {{ loading ? '登录中...' : '登录' }}
        </el-button>
      </el-form>

      <div class="login-hint">
        <span class="hint-label">默认账号</span>
        <span class="hint-value">admin / admin123</span>
      </div>
    </div>

    <div class="login-footer">
      <span>RustCam-Media v1.0</span>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { User, Lock, Sunny, Moon } from '@element-plus/icons-vue'
import { useThemeStore } from '../stores/themeStore'

const theme = useThemeStore()
import { useAuthStore } from '../stores/authStore'

const router = useRouter()
const auth = useAuthStore()

const form = ref({ username: 'admin', password: '' })
const loading = ref(false)

async function handleLogin() {
  if (!form.value.username || !form.value.password) {
    ElMessage.warning('请输入用户名和密码')
    return
  }
  loading.value = true
  try {
    await auth.login(form.value.username, form.value.password)
    router.push('/')
  } catch (e) {
    ElMessage.error(e.message || '登录失败')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-base);
  position: relative;
  overflow: hidden;
}

.login-bg {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.grid-overlay {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(var(--border-subtle) 1px, transparent 1px),
    linear-gradient(90deg, var(--border-subtle) 1px, transparent 1px);
  background-size: 40px 40px;
  opacity: 0.5;
}

.glow-orb {
  position: absolute;
  border-radius: 50%;
  filter: blur(80px);
  opacity: 0.15;
}
.orb-1 {
  width: 400px; height: 400px;
  background: var(--color-accent);
  top: -100px; left: -100px;
}
.orb-2 {
  width: 300px; height: 300px;
  background: var(--protocol-gb28181);
  bottom: -50px; right: -50px;
}

.login-card {
  position: relative;
  z-index: 1;
  width: 380px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-8);
  box-shadow: var(--shadow-lg);
}

.login-logo {
  display: flex;
  align-items: center;
  gap: var(--space-3);

  svg {
    width: 32px;
    height: 32px;
    color: var(--color-accent);
  }
}

.login-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: var(--space-6);
}

.theme-toggle {
  color: var(--text-muted) !important;
  font-size: var(--text-lg) !important;
  padding: var(--space-1) !important;
  &:hover { color: var(--color-accent) !important; }
}

.login-brand {
  font-family: var(--font-mono);
  font-size: var(--text-xl);
  font-weight: var(--weight-bold);
  color: var(--color-accent);
  letter-spacing: -0.5px;

  span { color: var(--text-primary); }
}

.login-title {
  font-size: var(--text-2xl);
  font-weight: var(--weight-bold);
  color: var(--text-primary);
  font-family: var(--font-cn);
  margin-bottom: var(--space-1);
}

.login-subtitle {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-8);
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.form-label {
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  color: var(--text-secondary);
  font-family: var(--font-cn);
}

.login-btn {
  width: 100%;
  height: 40px !important;
  font-size: var(--text-md) !important;
  margin-top: var(--space-2);
}

.login-hint {
  margin-top: var(--space-6);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  font-size: var(--text-xs);
}

.hint-label {
  color: var(--text-muted);
}

.hint-value {
  color: var(--color-accent);
  font-family: var(--font-mono);
}

.login-footer {
  position: absolute;
  bottom: var(--space-6);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
</style>
