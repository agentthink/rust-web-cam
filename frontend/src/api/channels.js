import { request } from '../utils/request'

export async function getChannels(params) {
  const res = await request.get('/channels', { params })
  return res.data.data
}

export async function getChannel(deviceTag, channelTag) {
  const res = await request.get(`/channels/${deviceTag}/${channelTag}`)
  return res.data.data
}

export async function getChannelPlayLinks(deviceTag, channelTag, params) {
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/play-links`, { params })
  return res.data.data
}

export async function getChannelStatus(deviceTag, channelTag) {
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/status`)
  return res.data.data
}

export async function getChannelPtzPresets(deviceTag, channelTag) {
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/ptz/presets`)
  return res.data.data?.presets || []
}

export async function createChannelPtzPreset(deviceTag, channelTag, name) {
  const res = await request.post(`/channels/${deviceTag}/${channelTag}/ptz/presets`, { name })
  return res.data.data
}

export async function deleteChannelPtzPreset(deviceTag, channelTag, token) {
  const res = await request.delete(`/channels/${deviceTag}/${channelTag}/ptz/presets/${token}`)
  return res.data
}

export async function getChannelPtzStatus(deviceTag, channelTag) {
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/ptz/status`)
  return res.data.data?.status || null
}

export async function channelPtzControl(deviceTag, channelTag, command, speed = 50, presetToken = null) {
  const body = { command, speed }
  if (presetToken) {
    body.move_type = 'goto_preset'
    body.preset_token = presetToken
  }
  const res = await request.post(`/channels/${deviceTag}/${channelTag}/ptz`, body)
  return res.data
}
