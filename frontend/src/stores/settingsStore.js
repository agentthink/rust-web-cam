import { defineStore } from 'pinia'

export const useSettingsStore = defineStore('settings', {
  state: () => ({
    playerType: localStorage.getItem('playerType') || 'rtsp',
  }),

  actions: {
    setPlayerType(type) {
      this.playerType = type
      localStorage.setItem('playerType', type)
    },
  },
})
