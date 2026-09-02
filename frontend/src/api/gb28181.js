import { request } from '../utils/request'

export async function getGb28181RefData() {
  const res = await request.get('/gb28181/ref-data')
  return res.data.data || { device_types: [], industry_codes: [], network_codes: [] }
}
