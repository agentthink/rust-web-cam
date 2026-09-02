import { request } from '../utils/request'

export async function getRecordings() {
  const res = await request.get('/recordings')
  return res.data.data
}

export async function getRecording(id) {
  const res = await request.get(`/recordings/${id}`)
  return res.data.data
}

export async function getRecordingStats() {
  const res = await request.get('/recordings/stats')
  return res.data.data
}

export async function getRecordingFiles(id) {
  const res = await request.get(`/recordings/${id}/files`)
  return res.data.data || []
}

export async function createRecording(data) {
  const res = await request.post('/recordings', data)
  return res.data.data
}

export async function startRecording(id) {
  const res = await request.post(`/recordings/${id}/start`)
  return res.data.data
}

export async function stopRecording(id) {
  const res = await request.post(`/recordings/${id}/stop`)
  return res.data.data
}

export async function pauseRecording(id) {
  const res = await request.post(`/recordings/${id}/pause`)
  return res.data.data
}

export async function resumeRecording(id) {
  const res = await request.post(`/recordings/${id}/resume`)
  return res.data.data
}

export async function deleteRecording(id) {
  const res = await request.delete(`/recordings/${id}`)
  return res.data
}

export async function getAllRecordingFiles() {
  const res = await request.get('/recordings/files')
  return res.data.data || []
}
