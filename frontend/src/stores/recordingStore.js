import { defineStore } from 'pinia'
import { ElMessage } from 'element-plus'
import * as api from '../api/recordings'

export const useRecordingStore = defineStore('recordings', {
  state: () => ({
    recordings: [],
    stats: null,
    loading: false,
    error: null,
    total: 0,
  }),

  actions: {
    async fetchRecordings() {
      this.loading = true
      this.error = null
      try {
        const data = await api.getRecordings()
        this.recordings = data.items || data
        this.total = data.total || this.recordings.length
      } catch (e) {
        this.error = e.message
        throw e
      } finally {
        this.loading = false
      }
    },

    async fetchStats() {
      this.stats = await api.getRecordingStats()
    },

    async createRecording(data) {
      const recording = await api.createRecording(data)
      this.recordings.unshift(recording)
      this.total++
      ElMessage.success('录像任务已创建')
      return recording
    },

    async startRecording(id) {
      const recording = await api.startRecording(id)
      const idx = this.recordings.findIndex(r => r.id === id)
      if (idx !== -1) this.recordings[idx] = recording
      ElMessage.success('录像已开始')
      return recording
    },

    async stopRecording(id) {
      const recording = await api.stopRecording(id)
      const idx = this.recordings.findIndex(r => r.id === id)
      if (idx !== -1) this.recordings[idx] = recording
      ElMessage.success('录像已停止')
      return recording
    },

    async pauseRecording(id) {
      const recording = await api.pauseRecording(id)
      const idx = this.recordings.findIndex(r => r.id === id)
      if (idx !== -1) this.recordings[idx] = recording
      ElMessage.success('录像已暂停')
      return recording
    },

    async resumeRecording(id) {
      const recording = await api.resumeRecording(id)
      const idx = this.recordings.findIndex(r => r.id === id)
      if (idx !== -1) this.recordings[idx] = recording
      ElMessage.success('录像已继续')
      return recording
    },

    async deleteRecording(id) {
      await api.deleteRecording(id)
      this.recordings = this.recordings.filter(r => r.id !== id)
      this.total--
      ElMessage.success('录像任务已删除')
    },

    async fetchFiles(id) {
      return await api.getRecordingFiles(id)
    },
  },
})
