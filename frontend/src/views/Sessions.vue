<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">会话管理</h1>
      <div class="page-toolbar">
        <el-input
          v-model="searchQuery"
          placeholder="搜索..."
          clearable
          style="max-width: 200px"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <el-select v-model="stateFilter" clearable placeholder="全部状态" style="width: 120px">
          <el-option value="Active" label="活跃" />
          <el-option value="Initializing" label="初始化" />
          <el-option value="Inactive" label="不活跃" />
        </el-select>
        <el-button :icon="Refresh" :loading="loading" @click="fetchSessions">刷新</el-button>
      </div>
    </div>
    <div class="page-body">
      <DataCard>
        <el-skeleton animated :loading="loading && sessions.length === 0" :rows="5">
          <el-table :data="filteredSessions" border stripe style="width: 100%">
              <el-table-column label="ID" prop="id" width="80" align="center" />
              <el-table-column label="类型" prop="session_type" width="100" align="center">
                <template #default="{ row }">
                  <el-tag size="small">{{ sessionTypeLabel(row.session_type) }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column label="设备标识" prop="device_tag" width="120" align="center" />
              <el-table-column label="流标识" prop="stream_id" min-width="180" show-overflow-tooltip />
              <el-table-column label="用户ID" prop="user_id" width="100" align="center" />
              <el-table-column label="客户端IP" width="130" align="center">
                <template #default="{ row }">
                  <code class="stream-key">{{ row.client_ip }}</code>
                </template>
              </el-table-column>
              <el-table-column label="协议" prop="protocol" width="100" align="center">
                <template #default="{ row }">{{ row.protocol || '-' }}</template>
              </el-table-column>
              <el-table-column label="状态" width="100" align="center">
                <template #default="{ row }">
                  <el-tag :type="sessionStateType(row.state)" size="small">{{ sessionStateLabel(row.state) }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column label="创建时间" width="180" align="center">
                <template #default="{ row }">{{ formatTime(row.created_at) }}</template>
              </el-table-column>
              <el-table-column label="最后活动" width="180" align="center">
                <template #default="{ row }">{{ formatTime(row.last_activity) }}</template>
              </el-table-column>
            </el-table>
        </el-skeleton>
      </DataCard>

      <div class="pagination-wrapper" v-if="total > pageSize">
        <el-pagination
          v-model:current-page="currentPage"
          :page-size="pageSize"
          :total="total"
          layout="prev, pager, next"
          @current-change="onPageChange"
        />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Search, Refresh } from '@element-plus/icons-vue'
import { getSessions } from '../api/sessions'
import DataCard from '../components/common/DataCard.vue'

const sessions = ref([])
const loading = ref(false)
const searchQuery = ref('')
const stateFilter = ref('')
const currentPage = ref(1)
const pageSize = 20
const total = ref(0)

const filteredSessions = computed(() => {
  return sessions.value.filter(s => {
    const q = searchQuery.value.toLowerCase()
    const matchSearch = !q
      || String(s.id).includes(q)
      || (s.device_tag || '').includes(q)
      || (s.stream_id || '').toLowerCase().includes(q)
      || (s.client_ip || '').includes(q)
    const matchState = !stateFilter.value || s.state === stateFilter.value
    return matchSearch && matchState
  })
})

let refreshTimer = null

onMounted(async () => {
  await fetchSessions()
  refreshTimer = setInterval(fetchSessions, 15000)
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
})

async function fetchSessions() {
  loading.value = true
  try {
    const offset = (currentPage.value - 1) * pageSize
    const data = await getSessions({ limit: pageSize, offset })
    sessions.value = data?.items || []
    total.value = data?.total || 0
  } catch (e) {
    console.error('fetchSessions error:', e)
  } finally {
    loading.value = false
  }
}

async function onPageChange(page) {
  currentPage.value = page
  await fetchSessions()
}

function sessionTypeLabel(type) {
  const map = { Play: '播放', Talk: '对讲', Push: '推流' }
  return map[type] || type || '-'
}

function sessionStateLabel(state) {
  const map = { Active: '活跃', Initializing: '初始化', Inactive: '不活跃' }
  return map[state] || state || '-'
}

function sessionStateType(state) {
  const map = { Active: 'success', Initializing: 'warning', Inactive: 'info' }
  return map[state] || ''
}

function formatTime(ts) {
  if (!ts) return '-'
  const d = new Date(ts)
  if (isNaN(d)) return ts
  return d.toLocaleString('zh-CN', { hour12: false })
}
</script>

<style scoped>
.stream-key {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
.pagination-wrapper { display: flex; justify-content: center; margin-top: 16px; }
</style>
