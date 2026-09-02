import { request } from '../utils/request'

export async function getUsers() {
  const res = await request.get('/users')
  return res.data.data || []
}

export async function getUser(id) {
  const res = await request.get(`/users/${id}`)
  return res.data.data
}

export async function createUser(data) {
  const res = await request.post('/users', data)
  return res.data.data
}

export async function updateUser(id, data) {
  const res = await request.put(`/users/${id}`, data)
  return res.data.data
}

export async function deleteUser(id) {
  const res = await request.delete(`/users/${id}`)
  return res.data
}

export async function assignUserRoles(id, roles) {
  const res = await request.put(`/users/${id}/roles`, { roles })
  return res.data
}

export async function getRoles() {
  const res = await request.get('/roles')
  return res.data.data || []
}

export async function createRole(data) {
  const res = await request.post('/roles', data)
  return res.data.data
}

export async function setRolePermissions(id, permissions) {
  const res = await request.put(`/roles/${id}/permissions`, { permissions })
  return res.data
}

export async function getPermissions() {
  const res = await request.get('/permissions')
  return res.data.data || []
}
