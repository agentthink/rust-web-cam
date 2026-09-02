<template>
  <div class="app-aside" :style="{ width: collapsed ? 'var(--sidebar-collapsed)' : 'var(--sidebar-expanded)' }">
    <div class="aside-logo">
      <div class="logo-mark">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2L2 7l10 5 10-5-10-5z"/>
          <path d="M2 17l10 5 10-5"/>
          <path d="M2 12l10 5 10-5"/>
        </svg>
      </div>
      <span v-if="!collapsed" class="logo-text">RustCam</span>
    </div>

    <div class="nav-menu">
      <el-menu
        :default-active="currentRoute"
        :collapse="collapsed"
        :collapse-transition="false"
        class="nav-menu"
        @select="onMenuSelect"
      >
        <el-menu-item index="/">
          <el-icon><Monitor /></el-icon>
          <template #title><span>系统概览</span></template>
        </el-menu-item>

        <el-sub-menu index="/device-group">
          <template #title>
            <el-icon><VideoCamera /></el-icon>
            <span>设备管理</span>
          </template>
          <el-menu-item index="/devices">设备列表</el-menu-item>
          <el-menu-item index="/channels">通道列表</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="/stream-group">
          <template #title>
            <el-icon><DataLine /></el-icon>
            <span>媒体流</span>
          </template>
          <el-menu-item index="/streams">流列表</el-menu-item>
          <el-menu-item index="/sessions">实时会话</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="/recording-group">
          <template #title>
            <el-icon><VideoPlay /></el-icon>
            <span>录像管理</span>
          </template>
          <el-menu-item index="/recordings">录像任务</el-menu-item>
          <el-menu-item index="/recordings/files">录像文件</el-menu-item>
        </el-sub-menu>

        <el-menu-item index="/servers">
          <el-icon><Cpu /></el-icon>
          <template #title><span>媒体服务器</span></template>
        </el-menu-item>

        <el-menu-item index="/public">
          <el-icon><Compass /></el-icon>
          <template #title><span>公开直播</span></template>
        </el-menu-item>

        <el-menu-item index="/alarms">
          <el-icon>
            <Bell />
          </el-icon>
          <template #title>
            <span>报警记录</span>
            <el-badge v-if="unprocessedAlarms > 0" :value="unprocessedAlarms > 99 ? '99+' : unprocessedAlarms" type="danger" style="margin-left: 8px" />
          </template>
        </el-menu-item>

        <el-sub-menu index="/wall-group">
          <template #title>
            <el-icon><Grid /></el-icon>
            <span>监控墙</span>
          </template>
          <el-menu-item index="/video-wall">监控墙</el-menu-item>
          <el-menu-item index="/video-wall/designer">布局设计</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="isAdmin" index="admin-group">
          <template #title>
            <el-icon><User /></el-icon>
            <span>管理</span>
          </template>
          <el-menu-item index="/users">用户管理</el-menu-item>
          <el-menu-item index="/settings">系统设置</el-menu-item>
        </el-sub-menu>
      </el-menu>
    </div>

    <div class="aside-footer">
      <div class="aside-footer-row" style="justify-content: space-between;">
        <div class="aside-footer-row">
          <span class="status-dot" :class="wsConnected ? 'online' : 'offline'"></span>
          <span v-if="!collapsed" class="ws-indicator">{{ wsConnected ? '在线' : '离线' }}</span>
        </div>
        <el-tooltip v-if="collapsed" content="退出登录" placement="right">
          <el-button text size="small" @click="handleLogout">
            <el-icon><SwitchButton /></el-icon>
          </el-button>
        </el-tooltip>
        <el-button v-else text size="small" @click="handleLogout">
          <el-icon><SwitchButton /></el-icon>
        </el-button>
      </div>
      <div class="aside-footer-row" style="justify-content: flex-end;">
        <el-tooltip v-if="collapsed" content="切换主题" placement="right">
          <el-button text @click="theme.toggle()" size="small">
            <el-icon v-if="theme.isDark"><Sunny /></el-icon>
            <el-icon v-else><Moon /></el-icon>
          </el-button>
        </el-tooltip>
        <el-button v-else text @click="theme.toggle()" size="small">
          <el-icon v-if="theme.isDark"><Sunny /></el-icon>
          <el-icon v-else><Moon /></el-icon>
        </el-button>
        <el-tooltip v-if="collapsed" content="展开" placement="right">
          <el-button text size="small" @click="collapsed = !collapsed">
            <el-icon><DArrowRight /></el-icon>
          </el-button>
        </el-tooltip>
        <el-button v-else text size="small" @click="collapsed = !collapsed">
          <el-icon><DArrowLeft /></el-icon>
        </el-button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessageBox } from 'element-plus'
import { Monitor, VideoCamera, DataLine, VideoPlay, Cpu, Compass, Grid, User, Sunny, Moon, DArrowLeft, DArrowRight, SwitchButton, Bell } from '@element-plus/icons-vue'
import { useThemeStore } from '../../stores/themeStore'
import { useAuthStore } from '../../stores/authStore'
import { useWsConnection } from '../../composables/useWsConnection'
import { getUnprocessedCount } from '../../api/alarms'

const route = useRoute()
const router = useRouter()
const theme = useThemeStore()
const auth = useAuthStore()
const { wsConnected, connect, disconnect } = useWsConnection()

const collapsed = ref(false)
const isAdmin = computed(() => auth.isAdmin)
const currentRoute = computed(() => route.path)
const unprocessedAlarms = ref(0)

onMounted(async () => {
  if (auth.isAuthenticated) {
    connect()
    await fetchUnprocessedAlarms()
  }
})

onUnmounted(() => {
  disconnect()
})

function onMenuSelect(index) {
  router.push(index)
}

async function fetchUnprocessedAlarms() {
  try {
    unprocessedAlarms.value = await getUnprocessedCount()
  } catch (e) {
    console.warn('[Sidebar] Failed to fetch unprocessed alarms:', e)
  }
}

async function handleLogout() {
  disconnect()
  await ElMessageBox.confirm('确定要退出登录吗？', '退出确认', { type: 'warning', confirmButtonText: '退出', cancelButtonText: '取消' })
  await auth.logout()
  router.push('/login')
}
</script>
