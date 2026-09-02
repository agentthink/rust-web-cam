import { request } from '../utils/request'

export async function getAlarms(params = {}) {
  const res = await request.get('/alarms', { params })
  return res.data.data
}

export async function markAlarmProcessed(id, processed = true) {
  const res = await request.put(`/alarms/${id}/processed`, { processed })
  return res.data.data
}

export async function getUnprocessedCount() {
  const res = await request.get('/alarms', { params: { page: 1, page_size: 1 } })
  return res.data.data.total || 0
}