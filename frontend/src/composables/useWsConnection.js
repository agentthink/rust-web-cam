import { ref, onUnmounted } from 'vue'

const wsConnected = ref(false)
let ws = null
let reconnectTimer = null
let reconnectDelay = 1000

export function useWsConnection() {
  function connect() {
    if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) return

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    ws = new WebSocket(`${protocol}//${window.location.host}/ws`)

    ws.onopen = () => {
      wsConnected.value = true
      reconnectDelay = 1000
      console.log('[WS] Connected')
    }

    ws.onclose = () => {
      wsConnected.value = false
      ws = null
      console.log('[WS] Disconnected, reconnecting...')
      scheduleReconnect()
    }

    ws.onerror = (e) => {
      console.warn('[WS] Error', e)
      wsConnected.value = false
    }
  }

  function disconnect() {
    clearTimeout(reconnectTimer)
    if (ws) {
      ws.close()
      ws = null
    }
    wsConnected.value = false
  }

  function scheduleReconnect() {
    clearTimeout(reconnectTimer)
    reconnectTimer = setTimeout(() => {
      connect()
      reconnectDelay = Math.min(reconnectDelay * 2, 30000)
    }, reconnectDelay)
  }

  function onMessage(handler) {
    if (!ws) return
    ws.onmessage = handler
  }

  return { wsConnected, connect, disconnect, onMessage }
}
