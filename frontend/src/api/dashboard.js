import { request } from '../utils/request'

export async function getDashboard() {
  const res = await request.get('/dashboard')
  return res.data
}

export async function getStats() {
  const res = await request.get('/stats')
  return res.data.data
}
