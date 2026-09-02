<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">系统概览</h1>
      <div class="page-toolbar">
        <StatusDot :status="wsConnected ? 'online' : 'offline'" />
        <span style="font-size: var(--text-xs); color: var(--text-muted);">
          {{ wsConnected ? '已连接' : '未连接' }}
        </span>
        <el-tag :type="healthTagType" size="small">
          {{ healthLabel }} {{ healthScore }}%
        </el-tag>
        <el-button type="primary" :loading="loading" size="small" @click="fetchAll">
          <el-icon><Refresh /></el-icon> 刷新
        </el-button>
      </div>
    </div>

    <div class="page-body">
      <div class="page-grid stats-grid">
        <MetricCard label="设备总数" :value="dashData?.devices?.total || 0">
          <template #extra>在线 {{ dashData?.devices?.online || 0 }}</template>
        </MetricCard>
        <MetricCard label="活跃流" :value="dashData?.streams?.active || 0">
          <template #extra>总计 {{ dashData?.streams?.total || 0 }}</template>
        </MetricCard>
        <MetricCard label="实时会话" :value="dashData?.servers?.aggregate?.active_sessions || 0">
          <template #extra>总会话 {{ dashData?.servers?.aggregate?.total_sessions || 0 }}</template>
        </MetricCard>
        <MetricCard label="媒体服务器" :value="dashData?.servers?.total || 0">
          <template #extra>在线 {{ dashData?.servers?.online || 0 }}</template>
        </MetricCard>
      </div>

      <div class="page-grid tables-grid">
        <DataCard title="在线设备">
          <el-table :data="onlineDevices" size="small" :loading="onlineDevicesLoading">
            <el-table-column label="状态" width="44" align="center">
              <template #default="{ row }">
                <StatusDot :status="row.status === 'Online' ? 'online' : 'offline'" />
              </template>
            </el-table-column>
            <el-table-column prop="name" label="设备名称" min-width="120" show-overflow-tooltip />
            <el-table-column prop="protocol" label="协议" width="80" align="center">
              <template #default="{ row }">
                <el-tag size="small">{{ row.protocol?.toUpperCase() }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </DataCard>

        <DataCard title="活跃流">
          <el-table :data="activeStreams" size="small" :loading="streamsLoading">
            <el-table-column prop="stream_key" label="流标识" min-width="140" show-overflow-tooltip>
              <template #default="{ row }">
                <code class="stream-key">{{ row.stream_key }}</code>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag size="small" :type="row.state === 'Active' ? 'success' : 'info'">{{ row.state }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="观看" width="60" align="center">
              <template #default="{ row }">
                <span class="mono">{{ row.viewer_count || 0 }}</span>
              </template>
            </el-table-column>
          </el-table>
        </DataCard>

        <DataCard title="实时会话">
          <el-table :data="activeSessions" size="small" :loading="sessionsLoading">
            <el-table-column prop="session_type" label="类型" width="70" align="center" />
            <el-table-column prop="client_ip" label="客户端IP" min-width="120" show-overflow-tooltip>
              <template #default="{ row }">
                <code class="stream-key">{{ row.client_ip }}</code>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag size="small" :type="row.state === 'active' ? 'success' : 'info'">{{ row.state }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </DataCard>
      </div>

      <div class="page-grid server-grid">
        <DataCard title="媒体服务器状态">
          <el-table :data="dashData?.servers?.servers || []" size="small" :loading="statsLoading">
            <el-table-column label="状态" width="44" align="center">
              <template #default="{ row }">
                <StatusDot :status="row.online ? 'online' : 'offline'" />
              </template>
            </el-table-column>
            <el-table-column prop="name" label="名称" min-width="120" />
            <el-table-column label="类型" width="90" align="center">
              <template #default="{ row }">
                <el-tag size="small" type="info">{{ row.server_type?.toUpperCase() }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="会话" width="60" align="center">
              <template #default="{ row }">
                <span class="mono">{{ row.session_count || 0 }}</span>
              </template>
            </el-table-column>
            <el-table-column label="CPU" width="90" align="center">
              <template #default="{ row }">
                <el-progress
                  :percentage="Math.round(row.cpu_usage || 0)"
                  :color="cpuColor(row.cpu_usage)"
                  size="small"
                  style="width: 60px; display: inline-block;"
                />
              </template>
            </el-table-column>
            <el-table-column label="内存(MB)" width="100" align="center">
              <template #default="{ row }">
                <span class="mono">{{ row.memory_usage?.toFixed(0) || 0 }}</span>
              </template>
            </el-table-column>
          </el-table>
        </DataCard>

        <DataCard title="系统健康">
          <div class="health-inner">
            <div class="health-gauge">
              <v-chart :option="healthGaugeOption" style="width: 160px; height: 160px;" autoresize />
            </div>
            <div class="health-metrics">
              <div class="health-metric">
                <StatusDot status="online" />
                <span>服务器</span>
                <span class="mono-val">{{ dashData?.servers?.online || 0 }}/{{ dashData?.servers?.total || 0 }}</span>
              </div>
              <div class="health-metric">
                <StatusDot status="online" />
                <span>平均CPU</span>
                <span class="mono-val">{{ (dashData?.servers?.aggregate?.avg_cpu || 0).toFixed(1) }}%</span>
              </div>
              <div class="health-metric">
                <StatusDot status="online" />
                <span>平均内存</span>
                <span class="mono-val">{{ (dashData?.servers?.aggregate?.avg_memory || 0).toFixed(0) }} MB</span>
              </div>
              <div class="health-metric">
                <StatusDot status="online" />
                <span>活跃会话</span>
                <span class="mono-val">{{ dashData?.servers?.aggregate?.active_sessions || 0 }}</span>
              </div>
            </div>
          </div>
        </DataCard>
      </div>

      <DataCard>
        <div class="device-status-bar">
          <div class="status-item">
            <span class="status-value" style="color: var(--color-success);">{{ dashData?.devices?.online || 0 }}</span>
            <span class="status-name">在线</span>
          </div>
          <div class="status-divider"></div>
          <div class="status-item">
            <span class="status-value" style="color: var(--text-muted);">{{ (dashData?.devices?.total || 0) - (dashData?.devices?.online || 0) }}</span>
            <span class="status-name">离线</span>
          </div>
          <div class="status-divider"></div>
          <div class="status-item">
            <span class="status-value" style="color: var(--color-accent);">{{ dashData?.devices?.public || 0 }}</span>
            <span class="status-name">公开</span>
          </div>
          <div class="status-divider"></div>
          <div class="status-item">
            <span class="status-value" style="color: var(--color-accent);">{{ deviceOnlineRate }}%</span>
            <span class="status-name">在线率</span>
          </div>
        </div>
      </DataCard>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getDashboard } from '../api/dashboard'
import { authFetch } from '../utils/authFetch'
import { getStreams } from '../api/streams'
import { getSessions } from '../api/sessions'
import { getOnlineDevices } from '../api/devices'
import StatusDot from '../components/common/StatusDot.vue'
import MetricCard from '../components/common/MetricCard.vue'
import DataCard from '../components/common/DataCard.vue'

const wsConnected = ref(false)
const dashData = ref(null)
const statsLoading = ref(true)
const onlineDevicesLoading = ref(true)
const streamsLoading = ref(true)
const sessionsLoading = ref(true)
const loading = ref(false)
const reloading = ref(false)
const currentTime = ref('')
let refreshTimer = null
let ws = null

const onlineDevices = ref([])
const activeStreams = ref([])
const activeSessions = ref([])
const totalOnlineDevices = ref(0)
const totalStreams = ref(0)
const totalSessions = ref(0)
const onlineOffset = ref(0)
const streamOffset = ref(0)
const sessionOffset = ref(0)
const onlinePageSize = 10
const streamPageSize = 10
const sessionPageSize = 10

const onlinePage = computed(() => Math.floor(onlineOffset.value / onlinePageSize) + 1)
const streamPage = computed(() => Math.floor(streamOffset.value / streamPageSize) + 1)
const sessionPage = computed(() => Math.floor(sessionOffset.value / sessionPageSize) + 1)

const healthScore = computed(() => Math.round(dashData.value?.health?.score || 0))
const healthLabel = computed(() => {
  const l = dashData.value?.health?.level || 'critical'
  return { healthy: '健康', degraded: '降级', critical: '危险' }[l] || '未知'
})
const healthTagType = computed(() => {
  const s = healthScore.value
  if (s >= 80) return 'success'
  if (s >= 50) return 'warning'
  return 'danger'
})
const deviceOnlineRate = computed(() => {
  const t = dashData.value?.devices?.total || 0, o = dashData.value?.devices?.online || 0
  return t === 0 ? 0 : Math.round((o / t) * 100)
})

function cpuColor(val) {
  if (val >= 80) return '#ef4444'
  if (val >= 50) return '#f59e0b'
  return '#22c55e'
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

const healthGaugeOption = computed(() => ({
  series: [{
    type: 'gauge',
    startAngle: 180,
    endAngle: 0,
    center: ['50%', '70%'],
    radius: '90%',
    min: 0,
    max: 100,
    splitNumber: 4,
    itemStyle: { color: healthScore.value >= 80 ? '#22c55e' : healthScore.value >= 50 ? '#f59e0b' : '#ef4444' },
    progress: { show: true, width: 12 },
    pointer: { show: false },
    axisLine: { lineStyle: { width: 12, color: [[1, 'var(--border)']] } },
    axisTick: { show: false },
    splitLine: { show: false },
    axisLabel: { show: false },
    detail: {
      valueAnimation: true,
      fontSize: 28,
      fontFamily: 'JetBrains Mono, monospace',
      fontWeight: 600,
      color: 'var(--text-primary)',
      formatter: '{value}',
      offsetCenter: [0, '-10%'],
    },
    data: [{ value: healthScore.value }],
    title: { show: false },
  }],
}))

onMounted(async () => {
  updateTime()
  setInterval(updateTime, 1000)
  fetchDashboard().finally(() => { statsLoading.value = false })
  loadOnlineDevicesByPage(1).finally(() => { onlineDevicesLoading.value = false })
  loadStreamsByPage(1).finally(() => { streamsLoading.value = false })
  loadSessionsByPage(1).finally(() => { sessionsLoading.value = false })
  refreshTimer = setInterval(() => { fetchDashboard() }, 10000)
  initWs()
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
  if (ws) ws.close()
})

async function fetchDashboard() {
  try { dashData.value = await getDashboard() } catch (e) { console.error('Dashboard error:', e) }
}

async function loadOnlineDevicesByPage(page) {
  onlineOffset.value = (page - 1) * onlinePageSize
  try {
    await sleep(100)
    const data = await getOnlineDevices({ limit: onlinePageSize, offset: onlineOffset.value })
    onlineDevices.value = data?.items || []
    totalOnlineDevices.value = data?.total || 0
  } catch (e) { console.error(e) }
}

async function loadStreamsByPage(page) {
  streamOffset.value = (page - 1) * streamPageSize
  try {
    await sleep(100)
    const data = await getStreams({ limit: streamPageSize, offset: streamOffset.value })
    activeStreams.value = data?.items || []
    totalStreams.value = data?.total || 0
  } catch (e) { console.error(e) }
}

async function loadSessionsByPage(page) {
  sessionOffset.value = (page - 1) * sessionPageSize
  try {
    await sleep(100)
    const data = await getSessions({ limit: sessionPageSize, offset: sessionOffset.value })
    activeSessions.value = data?.items || []
    totalSessions.value = data?.total || 0
  } catch (e) { console.error(e) }
}

function initWs() {
  try {
    ws = new WebSocket(`ws://${window.location.host}/ws`)
    ws.onopen = () => { wsConnected.value = true }
    ws.onclose = () => { wsConnected.value = false; setTimeout(initWs, 5000) }
    ws.onmessage = (event) => {
      try { const m = JSON.parse(event.data); if (m.msg_type) fetchDashboard() } catch (e) {}
    }
    ws.onerror = () => { wsConnected.value = false }
  } catch (e) { wsConnected.value = false }
}

function updateTime() {
  currentTime.value = new Date().toLocaleString('zh-CN', { hour12: false })
}

async function confirmReload() {
  try {
    await ElMessageBox.confirm(
      '将从数据库重新加载所有设备缓存。如果有设备在外部被直接修改，此操作会同步最新数据。',
      '重新加载设备缓存',
      { confirmButtonText: '确定', cancelButtonText: '取消', type: 'warning' }
    )
    await reloadCache()
  } catch {}
}

async function reloadCache() {
  reloading.value = true
  try {
    const res = await authFetch('/api/v1/admin/reload-cache', { method: 'POST' })
    const data = await res.json()
    if (data.code === 0) {
      ElMessage.success(`缓存已重新加载，共 ${data.data.count} 个设备`)
    } else {
      ElMessage.error(data.msg || '重新加载失败')
    }
  } catch (e) {
    ElMessage.error('重新加载失败')
  } finally {
    reloading.value = false
  }
}

function fetchAll() {
  loading.value = true
  statsLoading.value = true
  onlineDevicesLoading.value = true
  streamsLoading.value = true
  sessionsLoading.value = true
  Promise.all([
    fetchDashboard().finally(() => { statsLoading.value = false }),
    loadOnlineDevicesByPage(1).finally(() => { onlineDevicesLoading.value = false }),
    loadStreamsByPage(1).finally(() => { streamsLoading.value = false }),
    loadSessionsByPage(1).finally(() => { sessionsLoading.value = false }),
  ]).finally(() => { loading.value = false })
}
</script>

<style scoped>
.stats-grid {
  grid-template-columns: repeat(4, 1fr);
}

.tables-grid {
  grid-template-columns: repeat(3, 1fr);
}

.server-grid {
  grid-template-columns: 1fr 340px;
}

.health-inner {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-2) 0;
}

.health-metrics {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.health-metric {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-secondary);

  span:nth-child(2) {
    flex: 1;
    font-family: var(--font-cn);
  }
}

.mono-val {
  font-family: var(--font-mono);
  font-weight: var(--weight-medium);
  color: var(--text-primary);
}

.stream-key {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-mono);
}

.device-status-bar {
  display: flex;
  align-items: center;
  justify-content: space-around;
  padding: var(--space-2) 0;
}

.status-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-1);
}

.status-value {
  font-family: var(--font-mono);
  font-size: var(--text-2xl);
  font-weight: var(--weight-bold);
  line-height: 1;
}

.status-name {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.status-divider {
  width: 1px;
  height: 40px;
  background: var(--border);
}
</style>
