import { request } from '../utils/request'

export async function getSessions(params) {
  const res = await request.get('/sessions', { params })
  return res.data.data
}
