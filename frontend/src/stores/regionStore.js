import { defineStore } from 'pinia'
import * as api from '../api/regions'

export const useRegionStore = defineStore('regions', {
  state: () => ({
    regionTree: [],
    flatRegions: [],
    loading: false,
  }),

  actions: {
    async fetchRegionTree() {
      this.loading = true
      try {
        this.regionTree = await api.getRegionTree()
      } finally {
        this.loading = false
      }
    },

    async fetchRegions(parentCode = null) {
      return await api.getRegions(parentCode)
    },
  },
})
