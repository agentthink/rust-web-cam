import { defineStore } from 'pinia'
import * as api from '../api/channels'

export const useChannelStore = defineStore('channels', {
  state: () => ({
    channels: [],
    loading: false,
    error: null,
    total: 0,
    limit: 50,
    offset: 0,
  }),

  actions: {
    async fetchChannels(params = {}) {
      this.loading = true
      this.error = null
      try {
        const data = await api.getChannels({
          limit: params.limit || this.limit,
          offset: params.offset || this.offset,
          device_tag: params.device_tag || undefined,
        })
        this.channels = data.items
        this.total = data.total
        this.limit = data.limit
        this.offset = data.offset
      } catch (e) {
        this.error = e.message
        throw e
      } finally {
        this.loading = false
      }
    },

    async getChannelPlayLinks(deviceTag, channelTag, streamKey) {
      return await api.getChannelPlayLinks(deviceTag, channelTag, { stream_key: streamKey })
    },

    async getChannelPtzPresets(deviceTag, channelTag) {
      return await api.getChannelPtzPresets(deviceTag, channelTag)
    },

    async createChannelPtzPreset(deviceTag, channelTag, name) {
      return await api.createChannelPtzPreset(deviceTag, channelTag, name)
    },

    async deleteChannelPtzPreset(deviceTag, channelTag, token) {
      return await api.deleteChannelPtzPreset(deviceTag, channelTag, token)
    },

    async getChannelPtzStatus(deviceTag, channelTag) {
      return await api.getChannelPtzStatus(deviceTag, channelTag)
    },

    async channelPtzControl(deviceTag, channelTag, command, speed = 50, presetToken = null) {
      return await api.channelPtzControl(deviceTag, channelTag, command, speed, presetToken)
    },
  },
})
