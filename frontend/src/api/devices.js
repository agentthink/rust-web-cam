import { request } from '../utils/request'

export async function getDevices(params) {
  const res = await request.get('/devices', { params })
  return res.data.data
}

export async function getOnlineDevices(params) {
  const res = await request.get('/devices/online', { params })
  return res.data.data
}

export async function getDevice(deviceTag) {
  const res = await request.get(`/devices/${deviceTag}`)
  return res.data.data
}

export async function createDevice(data) {
  const res = await request.post('/devices', data)
  return res.data.data
}

export async function updateDevice(id, data) {
  const res = await request.put(`/devices/${id}`, data)
  return res.data.data
}

export async function deleteDevice(deviceTag) {
  const res = await request.delete(`/devices/${deviceTag}`)
  return res.data
}

export async function playDevice(deviceTag) {
  const res = await request.post(`/devices/${deviceTag}/play`)
  return res.data.data
}

export async function startDevice(deviceTag) {
  const res = await request.post(`/devices/${deviceTag}/start`)
  return res.data.data
}

export async function stopDevice(deviceTag) {
  const res = await request.post(`/devices/${deviceTag}/stop`)
  return res.data
}

export async function getPlayLinks(deviceTag) {
  const channelTag = deviceTag
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/play-links`)
  return res.data.data
}

export async function getPtzPresets(deviceTag) {
  const channelTag = deviceTag
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/ptz/presets`)
  return res.data.data?.presets || []
}

export async function createPtzPreset(deviceTag, name) {
  const channelTag = deviceTag
  const res = await request.post(`/channels/${deviceTag}/${channelTag}/ptz/presets`, { name })
  return res.data.data
}

export async function deletePtzPreset(deviceTag, token) {
  const channelTag = deviceTag
  const res = await request.delete(`/channels/${deviceTag}/${channelTag}/ptz/presets/${token}`)
  return res.data
}

export async function getPtzStatus(deviceTag) {
  const channelTag = deviceTag
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/ptz/status`)
  return res.data.data?.status || null
}

export async function ptzControl(deviceTag, command, speed = 50, presetToken = null) {
  const channelTag = deviceTag
  const body = { command, speed }
  if (presetToken) {
    body.move_type = 'goto_preset'
    body.preset_token = presetToken
  }
  const res = await request.post(`/channels/${deviceTag}/${channelTag}/ptz`, body)
  return res.data
}

export async function startDevicePlayback(deviceTag, startTime, endTime) {
  const res = await request.post(`/devices/${deviceTag}/playback`, {
    start_time: startTime,
    end_time: endTime,
  })
  return res.data.data
}
