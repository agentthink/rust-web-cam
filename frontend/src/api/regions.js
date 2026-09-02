import { request } from '../utils/request'

export async function getRegions(parentCode = null) {
  const params = parentCode ? { parent: parentCode } : {}
  const res = await request.get('/regions', { params })
  return res.data.data || []
}

export async function getRegionTree() {
  const res = await request.get('/regions/tree')
  return res.data.data || []
}
