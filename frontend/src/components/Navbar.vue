<template>
  <nav>
    <div>
      <div>
        <svg viewBox="0 0 24 24"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>
      </div>
      <span>Rust<span>Cam</span>-Media</span>
    </div>

    <div>
      <router-link to="/">监控台</router-link>
      <router-link to="/devices">设备管理</router-link>
      <router-link to="/streams">流</router-link>
      <router-link to="/recordings">录像</router-link>
      <router-link to="/servers">服务器</router-link>
      <router-link to="/users" v-if="isAdmin">用户</router-link>
      <router-link to="/public">公开</router-link>
      <router-link to="/video-wall">视频墙</router-link>
      <router-link to="/settings">设置</router-link>
    </div>

    <div>
      <div>
        <span ></span>
        <span>{{ wsConnected ? '在线' : '离线' }}</span>
      </div>
      <router-link to="/settings" title="设置">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/>
        </svg>
      </router-link>
      <el-dropdown trigger="click">
        <div>
          <div>{{ currentUsername.charAt(0).toUpperCase() }}</div>
          <span>{{ currentUsername }}</span>
        </div>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item @click="logout">退出登录</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </nav>
</template>

<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/authStore'

const router = useRouter()
const auth = useAuthStore()
const wsConnected = ref(false)
const currentUsername = computed(() => auth.user?.username || 'admin')
const isAdmin = computed(() => auth.isAdmin)
let ws = null

onMounted(() => connectWs())
onUnmounted(() => { if (ws) ws.close() })

function connectWs() {
  try {
    ws = new WebSocket(`ws://${window.location.host}/ws`)
    ws.onopen = () => { wsConnected.value = true }
    ws.onclose = () => { wsConnected.value = false; setTimeout(connectWs, 5000) }
    ws.onerror = () => { wsConnected.value = false }
  } catch (e) { wsConnected.value = false }
}

function logout() {
  auth.logout()
  router.push('/login')
}
</script>


