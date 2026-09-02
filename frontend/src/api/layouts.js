import { request } from '../utils/request'

export async function getLayouts() {
  const res = await request.get('/layouts')
  return res.data.data || []
}

export async function getLayout(id) {
  const res = await request.get(`/layouts/${id}`)
  return res.data.data
}

export async function createLayout(data) {
  const res = await request.post('/layouts', data)
  return res.data.data
}

export async function updateLayout(id, data) {
  const res = await request.put(`/layouts/${id}`, data)
  return res.data.data
}

export async function deleteLayout(id) {
  const res = await request.delete(`/layouts/${id}`)
  return res.data
}

export async function setLayoutDefault(id) {
  const res = await request.put(`/layouts/${id}/default`)
  return res.data
}
