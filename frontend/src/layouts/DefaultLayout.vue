<template>
  <el-container class="app-shell">
    <el-aside :width="collapsed ? '56px' : '200px'" class="app-aside">
      <div class="aside-logo">
        <div class="logo-mark">
          <svg viewBox="0 0 24 24"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>
        </div>
        <span v-if="!collapsed" class="logo-text">Rust<span>Cam</span></span>
      </div>

      <el-scrollbar class="nav-scrollbar">
        <el-menu
          :default-active="currentPath"
          :collapse="collapsed"
          :collapse-transition="false"
          @select="onMenuSelect"
        >
          <el-menu-item index="/">
            <el-icon><Monitor /></el-icon>
            <template #title><span>监控台</span></template>
          </el-menu-item>
          <el-sub-menu index="/device-group">
            <template #title>
              <el-icon><VideoCamera /></el-icon>
              <span>设备管理</span>
            </template>
            <el-menu-item index="/devices">设备列表</el-menu-item>
            <el-menu-item index="/channels">通道列表</el-menu-item>
          </el-sub-menu>
          <el-sub-menu index="/streams-group">
            <template #title>
              <el-icon><DataLine /></el-icon>
              <span>视频流</span>
            </template>
            <el-menu-item index="/streams">流列表</el-menu-item>
            <el-menu-item index="/sessions">直播会话</el-menu-item>
          </el-sub-menu>
          <el-sub-menu index="/recordings-group">
            <template #title>
              <el-icon><VideoPlay /></el-icon>
              <span>录像</span>
            </template>
            <el-menu-item index="/recordings">录像任务</el-menu-item>
            <el-menu-item index="/recordings/files">录像文件</el-menu-item>
          </el-sub-menu>
          <el-menu-item index="/servers">
            <el-icon><Cpu /></el-icon>
            <template #title><span>服务器</span></template>
          </el-menu-item>
          <el-menu-item index="/public">
            <el-icon><Compass /></el-icon>
            <template #title><span>公开</span></template>
          </el-menu-item>
          <el-menu-item index="/video-wall">
            <el-icon><Grid /></el-icon>
            <template #title><span>视频墙</span></template>
          </el-menu-item>
          <el-sub-menu v-if="isAdmin" index="admin">
            <template #title>
              <el-icon><User /></el-icon>
              <span>管理</span>
            </template>
            <el-menu-item index="/users">用户管理</el-menu-item>
            <el-menu-item index="/settings">设置</el-menu-item>
          </el-sub-menu>
        </el-menu>
      </el-scrollbar>

      <div class="aside-footer">
        <div class="aside-footer-top">
          <span class="status-dot" :class="wsConnected ? 'online' : 'offline'"></span>
          <span v-if="!collapsed" class="ws-label">{{ wsConnected ? '在线' : '离线' }}</span>
        </div>
        <div class="aside-footer-bottom">
          <el-button text @click="theme.toggle()">
            <el-icon v-if="theme.isDark"><Sunny /></el-icon>
            <el-icon v-else><Moon /></el-icon>
          </el-button>
          <el-button text @click="collapsed = !collapsed">
            <el-icon><DArrowLeft v-if="!collapsed" /><DArrowRight v-else /></el-icon>
          </el-button>
          <el-dropdown trigger="click" placement="top-start">
            <el-button text>
              <el-avatar :size="20">{{ currentUsername.charAt(0).toUpperCase() }}</el-avatar>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="logout">退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </div>
    </el-aside>

    <el-container>
      <el-header height="36px" class="app-header">
        <div class="header-left">
          <el-icon><FolderOpened /></el-icon>
          <span class="header-label">最近</span>
        </div>
        <div class="header-tabs">
          <el-tag
            v-for="item in recentItems"
            :key="item.path"
            :type="$route.path === item.path || $route.path.startsWith(item.path + '/') ? 'primary' : 'info'"
            :closable="true"
            size="small"
            @click="navigateTo(item.path)"
            @close.stop="removeItem(item.path)"
            class="header-tag"
          >
            {{ item.label }}
          </el-tag>
        </div>
      </el-header>

      <el-main class="app-main">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/authStore'
import { useThemeStore } from '../stores/themeStore'
import { Monitor, VideoCamera, DataLine, VideoPlay, Cpu, Compass, Grid, User, Sunny, Moon, FolderOpened, DArrowLeft, DArrowRight } from '@element-plus/icons-vue'


const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const theme = useThemeStore()

const collapsed = ref(false)
const wsConnected = ref(false)
const recentItems = ref([])
const currentUsername = computed(() => auth.user?.username || 'admin')
const isAdmin = computed(() => auth.isAdmin)
const currentPath = computed(() => route.path)
let ws = null

const routeMap = {
  '/': '监控台', '/devices': '设备管理', '/devices/': '设备详情',
  '/streams': '视频流', '/sessions': '直播会话',   '/recordings': '录像任务', '/recordings/files': '录像文件', '/servers': '服务器',
  '/public': '公开', '/video-wall': '视频墙', '/video-wall/designer': '布局设计',
  '/users': '用户管理', '/settings': '设置',
}

function getTitle(path) {
  if (routeMap[path]) return routeMap[path]
  for (const key of Object.keys(routeMap).sort((a, b) => b.length - a.length)) {
    if (path.startsWith(key)) return routeMap[path]
  }
  return path
}

function addRecentItem(path) {
  if (path === '/login') return
  const existing = recentItems.value.findIndex(i => i.path === path)
  if (existing === -1) {
    recentItems.value.push({ path, label: getTitle(path) })
    if (recentItems.value.length > 8) recentItems.value.shift()
  } else {
    recentItems.value.splice(existing, 1)
    recentItems.value.push({ path, label: getTitle(path) })
  }
}

router.afterEach((to) => {
  addRecentItem(to.path)
})

function onMenuSelect(index) {
  router.push(index)
}

function navigateTo(path) {
  router.push(path)
}

function removeItem(path) {
  const idx = recentItems.value.findIndex(i => i.path === path)
  if (idx === -1) return
  recentItems.value.splice(idx, 1)
}

function connectWs() {
  try {
    ws = new WebSocket(`ws://${window.location.host}/ws`)
    ws.onopen = () => { wsConnected.value = true }
    ws.onclose = () => { wsConnected.value = false; setTimeout(connectWs, 5000) }
    ws.onerror = () => { wsConnected.value = false }
  } catch (e) { wsConnected.value = false }
}

onMounted(() => connectWs())
onUnmounted(() => { if (ws) ws.close() })

function logout() {
  auth.logout()
  router.push('/login')
}
</script>
