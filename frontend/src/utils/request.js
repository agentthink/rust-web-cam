import axios from 'axios'
import { ElMessage } from 'element-plus'

const ACCESS_KEY = 'rustcam_access_token'
const REFRESH_KEY = 'rustcam_refresh_token'

const request = axios.create({
  baseURL: '/api/v1',
  timeout: 15000,
  headers: {
    'Content-Type': 'application/json',
  },
})

function getToken() {
  return localStorage.getItem(ACCESS_KEY)
}

function getRefreshToken() {
  return localStorage.getItem(REFRESH_KEY)
}

function setTokens(accessToken, refreshToken) {
  localStorage.setItem(ACCESS_KEY, accessToken)
  localStorage.setItem(REFRESH_KEY, refreshToken || '')
}

function clearTokens() {
  localStorage.removeItem(ACCESS_KEY)
  localStorage.removeItem(REFRESH_KEY)
  localStorage.removeItem('rustcam_user')
}

request.interceptors.request.use(
  (config) => {
    const token = getToken()
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }
    return config
  },
  (error) => Promise.reject(error)
)

request.interceptors.response.use(
  (response) => {
    const res = response.data
    if (res.code !== undefined && res.code !== 200) {
      ElMessage.error(res.message || res.msg || '请求失败')
      return Promise.reject(new Error(res.message || res.msg || '请求失败'))
    }
    return response
  },
  async (error) => {
    const originalRequest = error.config

    if (error.response?.status === 401 && !originalRequest._retry) {
      originalRequest._retry = true
      clearTokens()
      
      ElMessage.warning('登录已过期，请重新登录')
      window.location.href = '/login'
      return Promise.reject(error)
    }

    if (error.response?.status === 403) {
      ElMessage.error('权限不足')
    } else if (error.response?.status === 404) {
      ElMessage.error('资源不存在')
    } else if (error.response?.status >= 500) {
      ElMessage.error('服务器错误')
    } else if (!error.response) {
      ElMessage.error('网络连接失败')
    }

    return Promise.reject(error)
  }
)

export { request, getToken, getRefreshToken, setTokens, clearTokens }
