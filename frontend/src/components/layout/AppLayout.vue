<template>
  <div class="app-shell">
    <AppSidebar />
    <div class="app-content">
      <div class="app-header">
        <div class="header-left">
          <el-icon><FolderOpened /></el-icon>
          <span>{{ pageTitle }}</span>
        </div>
        <div class="header-right">
          <span class="time-display">{{ currentTime }}</span>
        </div>
      </div>
      <div class="app-body">
        <router-view />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { FolderOpened } from '@element-plus/icons-vue'
import AppSidebar from './AppSidebar.vue'

const route = useRoute()
const currentTime = ref('')
let timer = null

const routeNames = {
  '/': '系统概览',
  '/devices': '设备列表',
  '/streams': '流列表',
  '/sessions': '实时会话',
  '/recordings': '录像任务',
  '/recordings/files': '录像文件',
  '/servers': '媒体服务器',
  '/public': '公开直播',
  '/video-wall': '监控墙',
  '/video-wall/designer': '布局设计',
  '/users': '用户管理',
  '/settings': '系统设置',
}

const pageTitle = computed(() => {
  const path = route.path
  if (routeNames[path]) return routeNames[path]
  for (const [key, val] of Object.entries(routeNames).sort((a,b) => b[0].length - a[0].length)) {
    if (path.startsWith(key)) return val
  }
  return 'RustCam'
})

function updateTime() {
  currentTime.value = new Date().toLocaleString('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false
  })
}

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<style scoped>
.app-shell {
  display: flex;
  height: 100%;
  width: 100%;
  overflow: hidden;
}

.app-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.app-body {
  flex: 1;
  overflow: hidden;
  position: relative;
}

.time-display {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--text-muted);
}
</style>
