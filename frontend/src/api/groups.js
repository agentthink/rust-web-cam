import { request } from '../utils/request'

export async function getGroupTree() {
  const res = await request.get('/groups/tree')
  return res.data.data || []
}

export async function getGroups() {
  const res = await request.get('/groups')
  return res.data.data || []
}

export async function createGroup(data) {
  const res = await request.post('/groups', data)
  return res.data.data
}

export async function updateGroup(id, data) {
  const res = await request.put(`/groups/${id}`, data)
  return res.data
}

export async function deleteGroup(id) {
  const res = await request.delete(`/groups/${id}`)
  return res.data
}

export async function assignDeviceGroup(deviceId, groupId) {
  const res = await request.put(`/devices/${deviceId}/group`, { group_id: groupId })
  return res.data.success
}
