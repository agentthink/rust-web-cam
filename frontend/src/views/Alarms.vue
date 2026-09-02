<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">报警记录</h1>
      <div class="page-toolbar">
        <el-button :icon="Refresh" :loading="loading" @click="fetchAll">刷新</el-button>
        <el-button v-if="unprocessedCount > 0" type="warning" :icon="Bell" @click="markAllProcessed">标记全部已处理</el-button>
      </div>
    </div>
    <div class="page-body">
      <div class="page-grid stats-grid">
        <MetricCard label="未处理" :value="unprocessedCount" />
        <MetricCard label="总计" :value="total" />
      </div>

      <el-row :gutter="12" align="middle" class="filter-bar">
        <el-col :span="12">
          <el-input v-model="searchQuery" placeholder="搜索设备ID或描述..." clearable style="max-width: 280px">
            <template #prefix><el-icon><Search /></el-icon></template>
          </el-input>
          <el-select v-model="processedFilter" clearable placeholder="全部状态" style="width: 130px; margin-left: 8px">
            <el-option :value="false" label="未处理" />
            <el-option :value="true" label="已处理" />
          </el-select>
          <el-text type="info" style="margin-left: 12px">{{ filteredAlarms.length }} / {{ alarms.length }}</el-text>
        </el-col>
      </el-row>

      <DataCard>
        <el-skeleton animated :loading="loading" :rows="5">
            <el-table :data="filteredAlarms" style="margin-top: 0">
              <el-table-column label="状态" width="100">
                <template #default="{ row }">
                  <el-tag :type="row.processed ? 'info' : 'danger'" size="small">
                    {{ row.processed ? '已处理' : '未处理' }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column label="设备" width="160">
                <template #default="{ row }">
                  <span>{{ deviceName(row.device_id) || row.device_tag }}</span>
                </template>
              </el-table-column>
              <el-table-column label="设备标签" prop="device_tag" width="160" show-overflow-tooltip />
              <el-table-column label="报警类型" prop="alarm_type" width="120" />
              <el-table-column label="描述" prop="description" min-width="200" show-overflow-tooltip />
              <el-table-column label="报警时间" width="160">
                <template #default="{ row }"><span>{{ formatDate(row.alarm_time) }}</span></template>
              </el-table-column>
              <el-table-column label="创建时间" width="160">
                <template #default="{ row }"><span>{{ formatDate(row.created_at) }}</span></template>
              </el-table-column>
              <el-table-column label="操作" width="120">
                <template #default="{ row }">
                  <el-button v-if="!row.processed" size="small" type="success" plain @click="markProcessed(row)">标记已处理</el-button>
                  <el-button v-else size="small" type="info" plain @click="markProcessed(row, false)">取消处理</el-button>
                </template>
              </el-table-column>
            </el-table>
        </el-skeleton>
      </DataCard>

      <div v-if="total > pageSize" class="pagination-wrapper">
        <el-pagination
          v-model:current-page="currentPage"
          :page-size="pageSize"
          :total="total"
          layout="prev, pager, next"
          @current-change="fetchAlarms"
        />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessageBox, ElMessage, ElNotification } from 'element-plus'
import { Search, Refresh, Bell } from '@element-plus/icons-vue'
import { getAlarms, markAlarmProcessed } from '../api/alarms'
import { getDevices } from '../api/devices'
import MetricCard from '../components/common/MetricCard.vue'
import DataCard from '../components/common/DataCard.vue'

const alarms = ref([])
const devices = ref([])
const loading = ref(false)
const searchQuery = ref('')
const processedFilter = ref('')
const currentPage = ref(1)
const pageSize = 20
const total = ref(0)
let ws = null
let wsReconnectTimer = null

const unprocessedCount = computed(() => alarms.value.filter(a => !a.processed).length)

function initAlarmWs() {
  if (ws && ws.readyState === WebSocket.OPEN) return
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${window.location.host}/ws`)
  ws.onopen = () => { console.log('[Alarms] WebSocket connected') }
  ws.onclose = () => {
    console.log('[Alarms] WebSocket closed, reconnecting...')
    wsReconnectTimer = setTimeout(initAlarmWs, 5000)
  }
  ws.onerror = (e) => { console.warn('[Alarms] WebSocket error', e) }
  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data)
      if (msg.msg_type === 'alarm') {
        handleNewAlarm(msg.data)
      }
    } catch (e) { /* ignore parse errors */ }
  }
}

function closeAlarmWs() {
  clearTimeout(wsReconnectTimer)
  if (ws) {
    ws.close()
    ws = null
  }
}

function handleNewAlarm(data) {
  ElNotification.warning({
    title: '新报警',
    message: `${data.alarm_type || '未知类型'}: ${data.message || '无描述'}`,
    duration: 5000,
  })
  if (currentPage.value === 1) {
    fetchAlarms()
  }
}

const filteredAlarms = computed(() => {
  const q = searchQuery.value.toLowerCase()
  return alarms.value.filter(a => {
    const matchSearch = !q || 
      a.device_tag?.toLowerCase().includes(q) || 
      a.description?.toLowerCase().includes(q) ||
      a.device_id?.toString().includes(q)
    const matchProcessed = processedFilter.value === '' || a.processed === processedFilter.value
    return matchSearch && matchProcessed
  })
})

onMounted(async () => {
  await fetchAlarms()
  await fetchDevices()
  initAlarmWs()
})

onUnmounted(() => {
  closeAlarmWs()
})

async function fetchAlarms() {
  loading.value = true
  try {
    const offset = (currentPage.value - 1) * pageSize
    const res = await getAlarms({ page: currentPage.value, page_size: pageSize })
    alarms.value = res.items || []
    total.value = res.total || 0
  } catch (e) {
    ElMessage.error('获取报警记录失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

async function fetchDevices() {
  try {
    const res = await getDevices({ limit: 1000 })
    devices.value = res.items || []
  } catch (e) {
    // silent fail
  }
}

async function markProcessed(alarm, processed = true) {
  try {
    await markAlarmProcessed(alarm.id, processed)
    ElMessage.success(processed ? '已标记为处理' : '已取消处理')
    await fetchAlarms()
  } catch (e) {
    ElMessage.error('操作失败: ' + e.message)
  }
}

async function markAllProcessed() {
  try {
    await ElMessageBox.confirm(`确定要将所有 ${unprocessedCount.value} 条未处理报警标记为已处理吗？`, '确认操作', { type: 'warning' })
    for (const alarm of alarms.value.filter(a => !a.processed)) {
      await markAlarmProcessed(alarm.id, true)
    }
    ElMessage.success('已全部标记为处理')
    await fetchAlarms()
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error('操作失败: ' + e.message)
    }
  }
}

function deviceName(id) {
  const d = devices.value.find(d => d.id === id)
  return d ? d.name : ''
}

function formatDate(dateStr) {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString('zh-CN', { hour12: false })
}
</script>

<style scoped>
.stats-grid {
  grid-template-columns: repeat(2, 1fr);
}
.filter-bar { margin-bottom: 8px; }
.pagination-wrapper {
  display: flex;
  justify-content: center;
  margin-top: 16px;
}
</style>