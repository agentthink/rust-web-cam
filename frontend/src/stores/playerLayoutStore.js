import { defineStore } from 'pinia'
import { ElMessage } from 'element-plus'
import * as api from '../api/layouts'

export const usePlayerLayoutStore = defineStore('playerLayout', {
  state: () => ({
    layouts: [],
    currentLayout: null,
    defaultLayout: null,
    loading: false,
    error: null,
  }),

  getters: {
    sortedLayouts: (state) => {
      return [...state.layouts].sort((a, b) => {
        if (a.is_default && !b.is_default) return -1
        if (!a.is_default && b.is_default) return 1
        return a.id - b.id
      })
    },
    currentId: (state) => state.currentLayout?.id ?? null,
  },

  actions: {
    async fetchLayouts() {
      this.loading = true
      this.error = null
      try {
        this.layouts = await api.getLayouts()
        this.defaultLayout = this.layouts.find(l => l.is_default) || null
      } catch (err) {
        this.error = err.message
      } finally {
        this.loading = false
      }
    },

    async fetchLayout(id) {
      this.loading = true
      this.error = null
      try {
        this.currentLayout = await api.getLayout(id)
        return this.currentLayout
      } catch (err) {
        this.error = err.message
        return null
      } finally {
        this.loading = false
      }
    },

    async fetchDefault() {
      await this.fetchLayouts()
      if (this.defaultLayout) {
        this.currentLayout = this.defaultLayout
      }
      return this.defaultLayout
    },

    async createLayout(data) {
      const layout = await api.createLayout(data)
      this.layouts.push(layout)
      if (layout.is_default) {
        this.layouts.forEach(l => { if (l.id !== layout.id) l.is_default = false })
        this.defaultLayout = layout
      }
      return layout
    },

    async updateLayout(id, data) {
      const updated = await api.updateLayout(id, data)
      const idx = this.layouts.findIndex(l => l.id === id)
      if (idx !== -1) {
        this.layouts[idx] = { ...this.layouts[idx], ...data }
      }
      if (this.currentLayout?.id === id) {
        this.currentLayout = { ...this.currentLayout, ...data }
      }
      if (data.is_default) {
        this.layouts.forEach(l => { if (l.id !== id) l.is_default = false })
        this.defaultLayout = this.layouts.find(l => l.id === id)
      }
      return true
    },

    async setDefault(id) {
      await api.setLayoutDefault(id)
      this.layouts.forEach(l => {
        l.is_default = (l.id === id)
      })
      this.defaultLayout = this.layouts.find(l => l.id === id) || null
      return true
    },

    async deleteLayout(id) {
      await api.deleteLayout(id)
      this.layouts = this.layouts.filter(l => l.id !== id)
      if (this.currentLayout?.id === id) {
        this.currentLayout = null
      }
      if (this.defaultLayout?.id === id) {
        this.defaultLayout = null
      }
      return true
    },

    selectLayout(id) {
      const layout = this.layouts.find(l => l.id === id)
      if (layout) {
        this.currentLayout = layout
      }
    },
  },
})
