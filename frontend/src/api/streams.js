import { request } from '../utils/request'

export async function getStreams(params) {
  const res = await request.get('/streams', { params })
  return res.data.data
}

export async function getStream(id) {
  const res = await request.get(`/streams/${id}`)
  return res.data.data
}

export async function getStreamPlayUrl(id, protocol = 'hls') {
  const res = await request.get(`/streams/${id}/play`, { params: { protocol } })
  return res.data.data
}

export async function getStreamPlayLinks(id) {
  const res = await request.get(`/streams/${id}/play-links`)
  return res.data.data
}

export async function isStreamOnline(id) {
  const res = await request.get(`/streams/${id}/online`)
  return res.data.data
}

export async function startStream(data) {
  const res = await request.post('/streams', data)
  return res.data.data
}

export async function stopStream(id) {
  const res = await request.delete(`/streams/${id}`)
  return res.data
}

export async function restartStream(id) {
  const res = await request.post(`/streams/${id}/restart`)
  return res.data
}

export async function getStreamsByDevice(deviceTag) {
  const res = await request.get(`/streams/by-device/${deviceTag}`)
  return res.data.data
}
