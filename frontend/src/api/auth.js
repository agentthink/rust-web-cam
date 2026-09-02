import { request, setTokens, clearTokens } from '../utils/request'

export async function login(username, password) {
  const res = await request.post('/auth/login', { username, password })
  const data = res.data.data
  setTokens(data.access_token, data.refresh_token)
  localStorage.setItem('rustcam_user', JSON.stringify(data.user))
  return data.user
}

export async function refreshToken() {
  const refresh_token = localStorage.getItem('rustcam_refresh_token')
  if (!refresh_token) return false
  try {
    const res = await request.post('/auth/refresh', { refresh_token })
    const data = res.data.data
    setTokens(data.access_token, data.refresh_token)
    localStorage.setItem('rustcam_user', JSON.stringify(data.user))
    return true
  } catch {
    clearTokens()
    return false
  }
}

export async function getCurrentUser() {
  const res = await request.get('/auth/me')
  return res.data.data
}

export async function logout() {
  clearTokens()
}
