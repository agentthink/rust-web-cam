import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as api from '../api/servers'

export const useMediaServerStore = defineStore('mediaServers', () => {
  const servers = ref([])
  const loading = ref(false)
  const error = ref(null)
  const checkingStatus = ref(new Set())

  async function fetchServers() {
    loading.value = true
    try {
      servers.value = await api.getServers()
    } catch (e) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function createServer(data) {
    try {
      const server = await api.createServer(data)
      await fetchServers()
      return server
    } catch (e) {
      error.value = e.message
      throw e
    }
  }

  async function updateServer(tag, data) {
    try {
      const server = await api.updateServer(tag, data)
      await fetchServers()
      return server
    } catch (e) {
      error.value = e.message
      throw e
    }
  }

  async function deleteServer(tag) {
    try {
      await api.deleteServer(tag)
      servers.value = servers.value.filter(s => s.server_tag !== tag)
    } catch (e) {
      error.value = e.message
      throw e
    }
  }

  async function checkServerStatus(tag) {
    checkingStatus.value.add(tag)
    try {
      const data = await api.refreshServer(tag)
      const idx = servers.value.findIndex(s => s.server_tag === tag)
      if (idx !== -1) {
        servers.value[idx] = { ...servers.value[idx], ...data }
      }
      return data
    } catch (e) {
      error.value = e.message
    } finally {
      checkingStatus.value.delete(tag)
    }
  }

  async function checkAllStatus() {
    await Promise.all(servers.value.map(s => checkServerStatus(s.server_tag)))
  }

  async function enableServer(tag) {
    try {
      const data = await api.enableServer(tag)
      const idx = servers.value.findIndex(s => s.server_tag === tag)
      if (idx !== -1) {
        servers.value[idx] = { ...servers.value[idx], ...data }
      }
    } catch (e) {
      error.value = e.message
      throw e
    }
  }

  async function disableServer(tag) {
    try {
      const data = await api.disableServer(tag)
      const idx = servers.value.findIndex(s => s.server_tag === tag)
      if (idx !== -1) {
        servers.value[idx] = { ...servers.value[idx], ...data }
      }
    } catch (e) {
      error.value = e.message
      throw e
    }
  }

  return { servers, loading, error, checkingStatus, fetchServers, createServer, updateServer, deleteServer, checkServerStatus, checkAllStatus, enableServer, disableServer }
})
