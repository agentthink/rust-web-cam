import { defineStore } from 'pinia'
import { ElMessage } from 'element-plus'
import * as api from '../api/devices'

export const useDeviceStore = defineStore('devices', {
  state: () => ({
    devices: [],
    loading: false,
    error: null,
    total: 0,
    limit: 50,
    offset: 0,
  }),

  actions: {
    async fetchDevices(limit = 50, offset = 0) {
      this.loading = true
      this.error = null
      try {
        const data = await api.getDevices({ limit, offset })
        this.devices = data.items
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

    async createDevice(data) {
      const device = await api.createDevice(data)
      this.devices.unshift(device)
      this.total++
      ElMessage.success('设备创建成功')
      return device
    },

    async updateDevice(id, data) {
      const device = await api.updateDevice(id, data)
      const index = this.devices.findIndex(d => d.id === id)
      if (index !== -1) this.devices[index] = device
      ElMessage.success('设备更新成功')
      return device
    },

    async deleteDevice(deviceTag) {
      await api.deleteDevice(deviceTag)
      this.devices = this.devices.filter(d => d.device_tag !== deviceTag)
      this.total--
      ElMessage.success('设备删除成功')
    },

    async playDevice(deviceTag) {
      const data = await api.playDevice(deviceTag)
      ElMessage.success('播放已启动')
      return data
    },

    async stopDevice(deviceTag) {
      await api.stopDevice(deviceTag)
      ElMessage.success('播放已停止')
      return true
    },

    async fetchDevice(deviceTag) {
      this.loading = true
      try {
        return await api.getDevice(deviceTag)
      } finally {
        this.loading = false
      }
    },

    async getPlayLinks(deviceTag) {
      return await api.getPlayLinks(deviceTag)
    },

    async fetchPtzPresets(deviceTag) {
      return await api.getPtzPresets(deviceTag)
    },

    async createPtzPreset(deviceTag, name) {
      const preset = await api.createPtzPreset(deviceTag, name)
      ElMessage.success('预制位已创建')
      return preset
    },

    async deletePtzPreset(deviceTag, token) {
      await api.deletePtzPreset(deviceTag, token)
      ElMessage.success('预制位已删除')
    },

    async getPtzStatus(deviceTag) {
      return await api.getPtzStatus(deviceTag)
    },

    async ptzControl(deviceTag, command, speed = 50, presetToken = null) {
      await api.ptzControl(deviceTag, command, speed, presetToken)
      if (command === 'stop' || presetToken) {
        ElMessage.success(command === 'stop' ? '已停止' : '预制位已调用')
      }
    },
  },
})
