import { request } from '../utils/request'

export async function getServers() {
  const res = await request.get('/servers')
  return res.data.data
}

export async function getServer(tag) {
  const res = await request.get(`/servers/${encodeURIComponent(tag)}`)
  return res.data.data
}

export async function getServerStatus(tag) {
  const res = await request.get(`/servers/${encodeURIComponent(tag)}/status`)
  return res.data.data
}

export async function getServerSessions(tag) {
  const res = await request.get(`/servers/${encodeURIComponent(tag)}/sessions`)
  return res.data.data
}

export async function createServer(data) {
  const res = await request.post('/servers', data)
  return res.data.data
}

export async function updateServer(tag, data) {
  const res = await request.put(`/servers/${encodeURIComponent(tag)}`, data)
  return res.data.data
}

export async function deleteServer(tag) {
  const res = await request.delete(`/servers/${encodeURIComponent(tag)}`)
  return res.data.data
}

export async function refreshServer(tag) {
  const res = await request.post(`/servers/${encodeURIComponent(tag)}/refresh`)
  return res.data.data
}

export async function enableServer(tag) {
  const res = await request.post(`/servers/${encodeURIComponent(tag)}/enable`)
  return res.data.data
}

export async function disableServer(tag) {
  const res = await request.post(`/servers/${encodeURIComponent(tag)}/disable`)
  return res.data.data
}
