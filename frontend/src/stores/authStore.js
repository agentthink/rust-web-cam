import { defineStore } from 'pinia'
import { login as apiLogin, logout as apiLogout, getCurrentUser, refreshToken as apiRefresh } from '../api/auth'

const ACCESS_KEY = 'rustcam_access_token'
const REFRESH_KEY = 'rustcam_refresh_token'
const USER_KEY = 'rustcam_user'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    accessToken: localStorage.getItem(ACCESS_KEY) || null,
    refreshToken: localStorage.getItem(REFRESH_KEY) || null,
    user: JSON.parse(localStorage.getItem(USER_KEY) || 'null'),
    isAuthenticated: !!localStorage.getItem(ACCESS_KEY),
  }),

  getters: {
    currentUser: (state) => state.user,
    isAdmin: (state) => state.user?.roles?.includes('Admin') || false,
    token: (state) => state.accessToken,
  },

  actions: {
    async login(username, password) {
      const user = await apiLogin(username, password)
      this.user = user
      this.isAuthenticated = true
      return user
    },

    async refresh() {
      return await apiRefresh()
    },

    async logout() {
      await apiLogout()
      this.accessToken = null
      this.refreshToken = null
      this.user = null
      this.isAuthenticated = false
    },

    async fetchCurrentUser() {
      try {
        this.user = await getCurrentUser()
        localStorage.setItem(USER_KEY, JSON.stringify(this.user))
      } catch {
        this.logout()
      }
    },
  },
})
