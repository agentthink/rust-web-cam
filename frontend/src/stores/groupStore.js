import { defineStore } from 'pinia'
import * as api from '../api/groups'

export const useGroupStore = defineStore('groups', {
  state: () => ({
    groupTree: [],
    loading: false,
  }),

  actions: {
    async fetchGroupTree() {
      this.loading = true
      try {
        this.groupTree = await api.getGroupTree()
      } finally {
        this.loading = false
      }
    },

    async createGroup(data) {
      const group = await api.createGroup(data)
      await this.fetchGroupTree()
      return group
    },

    async updateGroup(id, data) {
      await api.updateGroup(id, data)
      await this.fetchGroupTree()
    },

    async deleteGroup(id) {
      await api.deleteGroup(id)
      await this.fetchGroupTree()
    },

    async assignDeviceGroup(deviceId, groupId) {
      return await api.assignDeviceGroup(deviceId, groupId)
    },
  },
})
