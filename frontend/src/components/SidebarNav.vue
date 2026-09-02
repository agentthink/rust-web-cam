<template>
  <aside class="sidebar" :class="{ collapsed }">
    <div class="sidebar-header">
      <div class="logo-area">
        <div class="logo-mark">
          <svg viewBox="0 0 24 24"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>
        </div>
        <span class="logo-text" v-show="!collapsed">Rust<span>Cam</span></span>
      </div>
      <button class="sidebar-collapse-btn" @click="collapsed = !collapsed" :title="collapsed ? '展开' : '收起'">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path v-if="!collapsed" d="M15 18l-6-6 6-6"/>
          <path v-else d="M9 18l6-6-6-6"/>
        </svg>
      </button>
    </div>

    <nav class="sidebar-nav" v-show="!collapsed">
      <el-tree
        ref="menuTreeRef"
        :data="menuTree"
        :props="{ label: 'label', children: 'children' }"
        node-key="path"
        :current-node-key="currentPath"
        :highlight-current="true"
        :expand-on-click-node="true"
        :default-expand-all="false"
        class="sidebar-menu-tree"
        @node-click="onTreeNodeClick"
      >
        <template #default="{ data }">
          <div class="tree-node-item">
            <span v-html="data.icon"></span>
            <span>{{ data.label }}</span>
            <span v-if="data.badge" class="nav-badge">{{ data.badge }}</span>
          </div>
        </template>
      </el-tree>
    </nav>

    <nav class="sidebar-nav-collapsed" v-show="collapsed">
      <button
        v-for="item in navItems"
        :key="item.path"
        class="collapsed-nav-item"
        :class="{ active: item.path === '/' ? $route.path === '/' : $route.path.startsWith(item.path) }"
        @click="onNavClick(item.path)"
        :title="item.label"
      >
        <span v-html="item.icon"></span>
      </button>
      <button
        v-for="item in adminItems"
        :key="item.path"
        class="collapsed-nav-item"
        @click="onNavClick(item.path)"
        :title="item.label"
      >
        <span v-html="item.icon"></span>
      </button>
    </nav>

    <div class="sidebar-footer">
      <div class="ws-indicator">
        <span class="status-dot" :class="wsConnected ? 'online' : 'offline'"></span>
        <span class="ws-label" v-show="!collapsed">{{ wsConnected ? '在线' : '离线' }}</span>
      </div>

      <button class="theme-btn" @click="theme.toggle()" :title="theme.isDark ? '切换亮色' : '切换暗色'">
        <svg v-if="theme.isDark" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="5"/>
          <line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/>
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
          <line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/>
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
        </svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
      </button>

      <div class="user-area">
        <div class="user-avatar">{{ currentUsername.charAt(0).toUpperCase() }}</div>
        <div class="user-info" v-show="!collapsed">
          <span class="user-name">{{ currentUsername }}</span>
          <span class="user-role" v-if="isAdmin">管理员</span>
        </div>
        <el-dropdown trigger="click" placement="top-start">
          <button class="user-menu-btn">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="5" r="1"/><path d="M9 20h6"/><path d="M12 4v16"/>
            </svg>
          </button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item @click="logout">退出登录</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>
  </aside>
</template>

<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../stores/authStore'
import { useThemeStore } from '../stores/themeStore'

const emit = defineEmits(['navigate'])

const router = useRouter()
const route = useRoute()
const auth = useAuthStore()
const theme = useThemeStore()
const collapsed = ref(false)
const wsConnected = ref(false)
const menuTreeRef = ref(null)
const currentUsername = computed(() => auth.user?.username || 'admin')
const isAdmin = computed(() => auth.isAdmin)
const currentPath = computed(() => route.path)
let ws = null

const navItems = [
  { path: '/', label: '监控台', icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/></svg>' },
  {
    path: '__devices__',
    label: '设备管理',
    icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>',
    children: [
      { path: '/devices', label: '设备列表' },
      { path: '/channels', label: '通道列表' },
    ]
  },
  {
    path: '__streams__',
    label: '视频流',
    icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2"/></svg>',
    children: [
      { path: '/streams', label: '流列表' },
      { path: '/sessions', label: '直播会话' },
    ]
  },
  { path: '/recordings', label: '录像', icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>' },
  { path: '/servers', label: '服务器', icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="2" width="20" height="8" rx="2"/><rect x="2" y="14" width="20" height="8" rx="2"/><line x1="6" y1="6" x2="6.01" y2="6"/><line x1="6" y1="18" x2="6.01" y2="18"/></svg>' },
  { path: '/public', label: '公开', icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>' },
  { path: '/video-wall', label: '视频墙', icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/><line x1="7" y1="8" x2="17" y2="8"/><line x1="7" y1="12" x2="17" y2="12"/></svg>' },
]

const adminItems = computed(() => {
  if (!isAdmin.value) return []
  return [{ path: '/users', label: '用户', icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>' }]
})

const menuTree = computed(() => {
  const items = [...navItems]
  if (adminItems.value.length) {
    items.push({ path: 'admin', label: '管理', icon: '', children: adminItems.value })
  }
  return items
})

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

function onNavClick(path) {
  emit('navigate', path)
}

function onTreeNodeClick(data) {
  // Skip parent nodes (those with __ prefix) - they are just for grouping
  if (data.path && !data.path.startsWith('__') && data.path !== 'admin') {
    emit('navigate', data.path)
  }
}

function logout() {
  auth.logout()
  router.push('/login')
}
</script>
