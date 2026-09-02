<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">流管理</h1>
      <div class="page-toolbar">
        <el-input
          v-model="searchQuery"
          placeholder="搜索流..."
          clearable
          style="max-width: 200px"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <el-select v-model="stateFilter" clearable placeholder="全部状态" style="width: 120px">
          <el-option value="Active" label="活跃" />
          <el-option value="Idle" label="空闲" />
          <el-option value="Starting" label="启动中" />
          <el-option value="Stopping" label="停止中" />
          <el-option value="Recovering" label="恢复中" />
          <el-option value="Error" label="错误" />
          <el-option value="Stopped" label="已停止" />
        </el-select>
        <el-button :icon="Refresh" :loading="streamStore.loading" @click="fetchStreams">刷新</el-button>
        <el-button type="primary" :icon="Plus" @click="showAddModal = true">添加流</el-button>
      </div>
    </div>
    <div class="page-body">
      <DataCard>
        <el-table
          v-loading="streamStore.loading && streams.length === 0"
          :data="filteredStreams"
          border
          stripe
          @row-click="(row) => $router.push(`/streams/${row.id}`)"
          style="width: 100%"
        >
          <el-table-column label="ID" prop="id" width="80" align="center" />
          <el-table-column label="流标识" min-width="200">
            <template #default="{ row }">
              <code class="stream-key">{{ row.stream_key || row.id }}</code>
            </template>
          </el-table-column>
          <el-table-column label="设备标识" prop="device_tag" width="120" align="center" />
          <el-table-column label="状态" width="110" align="center">
            <template #default="{ row }">
              <el-tag :type="stateType(row.state)" size="small">{{ stateLabel(row.state) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="观看人数" width="100" align="center">
            <template #default="{ row }">
              <span>
                <el-icon><User /></el-icon>
                {{ row.viewer_count || 0 }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="媒体服务器" width="150" align="center">
            <template #default="{ row }">
              {{ row.media_server_tag || '-' }}
            </template>
          </el-table-column>
          <el-table-column label="创建时间" width="180" align="center">
            <template #default="{ row }">
              {{ formatTime(row.created_at) }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="200" align="center" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="primary" plain @click.stop="$router.push(`/streams/${row.id}`)">详情</el-button>
              <el-button 
                v-if="row.state === 'Idle' || row.state === 'Stopped' || row.state === 'Error'" 
                size="small" 
                type="success" 
                plain 
                @click.stop="confirmStart(row)"
              >启动</el-button>
              <el-button 
                v-if="row.state === 'Active' || row.state === 'Starting' || row.state === 'Recovering'" 
                size="small" 
                type="warning" 
                plain 
                @click.stop="confirmMaintenance(row)"
              >维护</el-button>
            </template>
          </el-table-column>
        </el-table>
      </DataCard>

      <div class="pagination-wrapper" v-if="streamStore.total > streamStore.pageSize">
        <el-pagination
          v-model:current-page="currentPage"
          :page-size="streamStore.pageSize"
          :total="streamStore.total"
          layout="prev, pager, next"
          @current-change="onPageChange"
        />
      </div>

      <el-dialog v-model="showAddModal" title="创建流" width="480px" destroy-on-close>
        <el-form label-position="top" @submit.prevent="addStream">
          <el-form-item label="设备 ID" required>
            <el-input v-model="addForm.deviceId" placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" />
          </el-form-item>
          <el-form-item label="RTSP 拉流地址（可选，GB28181 设备留空）">
            <el-input v-model="addForm.rtspUrl" placeholder="rtsp://camera-ip:554/stream" />
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button @click="showAddModal = false">取消</el-button>
          <el-button type="primary" :loading="addLoading" :disabled="!addForm.deviceId" @click="addStream">
            {{ addLoading ? '创建中...' : '创建' }}
          </el-button>
        </template>
      </el-dialog>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessageBox } from 'element-plus'
import { Search, Refresh, Plus, User } from '@element-plus/icons-vue'
import { useStreamStore } from '../stores/streamStore'
import { useDeviceStore } from '../stores/deviceStore'
import DataCard from '../components/common/DataCard.vue'

const route = useRoute()
const streamStore = useStreamStore()
const deviceStore = useDeviceStore()
const streams = ref([])
const showAddModal = ref(false)
const addForm = ref({ deviceId: '', rtspUrl: '' })
const addLoading = ref(false)
const searchQuery = ref('')
const stateFilter = ref('')
const currentPage = ref(1)

async function onPageChange(page) {
  currentPage.value = page
  await fetchStreams((page - 1) * streamStore.pageSize)
}

const filteredStreams = computed(() => {
  return streams.value.filter(s => {
    const q = searchQuery.value.toLowerCase()
    const matchSearch = !q || (s.stream_key || '').toLowerCase().includes(q) || (s.id || '').toLowerCase().includes(q) || (s.media_server_id || '').toLowerCase().includes(q)
    const matchState = !stateFilter.value || s.state === stateFilter.value
    return matchSearch && matchState
  })
})

let refreshTimer = null

watch(() => route.query.deviceId, (newId) => {
  if (newId) {
    addForm.value.deviceId = newId
    showAddModal.value = true
  }
})

onMounted(async () => {
  currentPage.value = 1
  await fetchStreams(0)
  refreshTimer = setInterval(() => fetchStreams((currentPage.value - 1) * streamStore.pageSize), 15000)
  if (route.query.deviceId) {
    addForm.value.deviceId = route.query.deviceId
    showAddModal.value = true
  }
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
})

async function fetchStreams(offset = 0) {
  await streamStore.fetchStreams(offset)
  streams.value = streamStore.streams
}

async function playStream(stream, protocol) {
  if (playingStream.value?.id === stream.id && playingProtocol.value === protocol) {
    stopStream()
    return
  }
  const url = await streamStore.getPlayUrl(stream.id, protocol)
  if (url) {
    playingStream.value = stream
    playingUrl.value = url
    playingType.value = protocol === 'flv' ? 'flv' : protocol === 'hls' ? 'hls' : 'auto'
    playingProtocol.value = protocol
  }
}

async function addStream() {
  if (!addForm.value.deviceId) return
  addLoading.value = true
  try {
    const rtspUrl = addForm.value.rtspUrl || null
    await streamStore.startStream(addForm.value.deviceId, rtspUrl)
    showAddModal.value = false
    addForm.value = { deviceId: '', rtspUrl: '' }
    await fetchStreams()
  } catch {
  } finally {
    addLoading.value = false
  }
}

async function confirmMaintenance(stream) {
  try {
    await ElMessageBox.confirm(`确定让设备 "${stream.device_tag}" 进入维护状态吗？`, '确认维护', { type: 'warning' })
    await deviceStore.updateDevice(stream.device_tag, { status: 'Maintaining' })
  } catch {}
}

async function confirmStart(stream) {
  try {
    await ElMessageBox.confirm(`确定启动流 "${stream.stream_key || stream.id}" 吗？`, '确认启动', { type: 'warning' })
    await streamStore.restartStream(stream.id)
  } catch {}
}

function stateType(state) {
  const map = { Active: 'success', Idle: 'info', Starting: 'warning', Stopping: 'warning', Recovering: 'warning', Error: 'danger', Stopped: 'info' }
  return map[state] || 'info'
}

function stateLabel(state) {
  const map = { Active: '活跃', Idle: '空闲', Starting: '启动中', Stopping: '停止中', Recovering: '恢复中', Error: '错误', Stopped: '已停止' }
  return map[state] || state
}

function formatTime(ts) {
  if (!ts) return '-'
  return new Date(ts).toLocaleString('zh-CN')
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
