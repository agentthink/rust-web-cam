export async function authFetch(url, options = {}) {
  const token = localStorage.getItem('rustcam_access_token')
  const headers = {
    'Content-Type': 'application/json',
    ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    ...(options.headers || {}),
  }
  const res = await fetch(url, { ...options, headers })
  return res
}
