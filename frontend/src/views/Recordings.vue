<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">录像管理</h1>
      <div class="page-toolbar">
        <el-button :icon="Refresh" :loading="loading" @click="fetchAll">刷新</el-button>
        <el-button type="primary" :icon="Plus" @click="openCreateModal">创建任务</el-button>
      </div>
    </div>
    <div class="page-body">
      <div class="page-grid stats-grid">
        <MetricCard v-for="stat in statCards" :key="stat.label" :label="stat.label" :value="stat.value" :extras="stat.label === '存储占用' ? [{ label: formatSize(stats?.total_size_bytes || 0) }] : []" />
      </div>

      <el-row :gutter="12" align="middle" class="filter-bar">
        <el-col :span="12">
          <el-input v-model="searchQuery" placeholder="搜索..." clearable style="max-width: 280px">
            <template #prefix><el-icon><Search /></el-icon></template>
          </el-input>
          <el-select v-model="stateFilter" clearable placeholder="全部状态" style="width: 130px; margin-left: 8px">
            <el-option value="Starting" label="启动中" />
            <el-option value="Recording" label="录制中" />
            <el-option value="Paused" label="已暂停" />
            <el-option value="Stopping" label="停止中" />
            <el-option value="Completed" label="已完成" />
            <el-option value="Error" label="错误" />
          </el-select>
          <el-text type="info" style="margin-left: 12px">{{ filteredRecordings.length }} / {{ recordings.length }}</el-text>
        </el-col>
      </el-row>

      <DataCard>
        <el-skeleton animated :loading="loading" :rows="5">
            <el-table :data="filteredRecordings" style="margin-top: 0">
              <el-table-column label="状态" prop="state" width="100">
                <template #default="{ row }">
                  <el-tag :type="stateTagType(row.state)" size="small">{{ stateLabel(row.state) }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column label="设备" width="160">
                <template #default="{ row }"><span>{{ deviceName(row.device_tag) }}</span></template>
              </el-table-column>
              <el-table-column label="流标识" prop="stream_key" show-overflow-tooltip />
              <el-table-column label="格式" prop="format" width="80" />
              <el-table-column label="文件名" prop="filename" min-width="180" show-overflow-tooltip>
                <template #default="{ row }"><span>{{ row.filename || '-' }}</span></template>
              </el-table-column>
              <el-table-column label="文件路径" prop="output_path" min-width="200" show-overflow-tooltip>
                <template #default="{ row }"><span>{{ row.output_path || '-' }}</span></template>
              </el-table-column>
              <el-table-column label="媒体服务器" prop="media_server_name" show-overflow-tooltip />
              <el-table-column label="时长" width="100">
                <template #default="{ row }"><span>{{ formatDuration(row.duration_secs) }}</span></template>
              </el-table-column>
              <el-table-column label="大小" width="100">
                <template #default="{ row }"><span>{{ formatSize(row.file_size) }}</span></template>
              </el-table-column>
              <el-table-column label="创建时间" width="160">
                <template #default="{ row }"><span>{{ formatDate(row.created_at) }}</span></template>
              </el-table-column>
              <el-table-column label="操作" width="280">
                <template #default="{ row }">
                  <el-button v-if="row.state === 'Starting'" size="small" @click="startRecording(row.id)">开始</el-button>
                  <el-button v-if="row.state === 'Recording'" size="small" @click="pauseRecording(row.id)">暂停</el-button>
                  <el-button v-if="row.state === 'Paused'" size="small" @click="resumeRecording(row.id)">继续</el-button>
                  <el-button v-if="row.state === 'Recording' || row.state === 'Paused'" size="small" type="warning" @click="confirmStop(row)">停止</el-button>
                  <el-button v-if="row.state === 'Completed'" size="small" @click="playRecording(row)">播放</el-button>
                  <el-button v-if="row.state === 'Completed' || row.state === 'Error'" size="small" type="danger" plain @click="confirmDelete(row)">删除</el-button>
                  <el-button v-if="row.filename" size="small" type="primary" plain @click="downloadFile(row)">下载</el-button>
                </template>
              </el-table-column>
            </el-table>
        </el-skeleton>
      </DataCard>
    </div>

        <el-dialog v-model="showCreateModal" title="创建录像任务" width="500px" destroy-on-close>
      <el-form label-position="top">
        <el-form-item label="设备 *" :error="deviceSelectorError">
          <el-select
            v-model="form.device_id"
            placeholder="输入设备名称或地址搜索..."
            filterable
            remote
            :remote-method="searchDevices"
            :loading="deviceLoading"
            style="width: 100%"
            reserve-keyword
            clearable
            @change="deviceSelectorError = ''"
          >
            <template #label>
              <span>{{ deviceOptions.find(d => d.id === form.device_id)?.name || '' }}</span>
            </template>
            <template #loading>
              <el-icon class="is-loading"><el-icon-loading /></el-icon>
            </template>
            <el-option
              v-for="d in deviceOptions"
              :key="d.id"
              :value="d.id"
              :label="d.name"
            >
              <div style="display: flex; align-items: center; gap: 6px">
                <span class="status-dot" :class="d.status === 'Online' ? 'online' : 'offline'" style="width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0"></span>
                <span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{{ d.name }}</span>
                <el-tag size="small" type="info">{{ d.protocol }}</el-tag>
              </div>
            </el-option>
          </el-select>
          <div v-if="deviceTotal > 0" style="font-size: 12px; color: #999; margin-top: 4px">
            共 {{ deviceTotal }} 台设备，已加载前 {{ deviceOptions.length }} 台（输入搜索更多）
          </div>
        </el-form-item>
        <el-form-item label="录像格式">
          <el-select v-model="form.format" style="width: 100%">
            <el-option value="Mp4" label="MP4" />
            <el-option value="Hls" label="HLS" />
            <el-option value="Flv" label="FLV" />
            <el-option value="Ts" label="TS" />
          </el-select>
        </el-form-item>
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="最大时长 (秒)">
              <el-input-number v-model="form.duration_secs" :min="0" placeholder="0=不限制" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大文件 (MB)">
              <el-input-number v-model="form.max_file_size_mb" :min="0" placeholder="0=不限制" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="标签">
          <el-input v-model="form.labels" placeholder="用逗号分隔多个标签" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateModal = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">创建任务</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showPlaybackModal" :title="`录像回放: ${playbackRecording?.stream_key}`" width="900px">
      <el-skeleton animated :loading="playbackLoading">
        <template #default>
          <VideoPlayer v-if="playbackUrl" :src="playbackUrl" :is-live="false" :autoplay="true" />
          <el-empty v-else description="暂无播放地址" />
        </template>
      </el-skeleton>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, shallowRef } from 'vue'
import { ElMessageBox, ElMessage } from 'element-plus'
import { Search, Refresh, Plus } from '@element-plus/icons-vue'
import VideoPlayer from '../components/VideoPlayer.vue'
import { useRecordingStore } from '../stores/recordingStore'
import { useDeviceStore } from '../stores/deviceStore'
import { useToast } from '../composables/useToast'
import { request } from '../utils/request'
import MetricCard from '../components/common/MetricCard.vue'
import DataCard from '../components/common/DataCard.vue'

const recStore = useRecordingStore()
const deviceStore = useDeviceStore()
const toast = useToast()

const recordings = ref([])
const stats = ref(null)
const loading = ref(false)
const showCreateModal = ref(false)
const submitting = ref(false)
const searchQuery = ref('')
const stateFilter = ref('')
const showPlaybackModal = ref(false)
const playbackRecording = ref(null)
const playbackUrl = ref('')
const playbackLoading = ref(false)
const deviceOptions = shallowRef([])
const deviceTotal = ref(0)
const deviceLoading = ref(false)
const deviceSelectorError = ref('')
const DEVICE_PAGE_SIZE = 50
let devicePageOffset = 0

const statCards = computed(() => [
  { label: '正在录制', value: stats.value?.recording || 0 },
  { label: '已完成', value: stats.value?.completed || 0 },
  { label: '总任务', value: stats.value?.total || 0 },
  { label: '存储占用', value: formatSize(stats.value?.total_size_bytes || 0) },
])

const filteredRecordings = computed(() => {
  const q = searchQuery.value.toLowerCase()
  return recordings.value.filter(r => {
    const matchSearch = !q || (r.device_tag || '').toLowerCase().includes(q) || (r.channel_tag || '').toLowerCase().includes(q)
    const matchState = !stateFilter.value || r.state === stateFilter.value
    return matchSearch && matchState
  })
})

const form = ref({ device_id: '', format: 'Mp4', duration_secs: 0, max_file_size_mb: 0, labels: '' })

onMounted(async () => {
  await fetchAll()
  await searchDevices('')
})

async function fetchAll() {
  loading.value = true
  try {
    await Promise.all([recStore.fetchRecordings(), recStore.fetchStats()])
    recordings.value = recStore.recordings
    stats.value = recStore.stats
  } finally {
    loading.value = false
  }
}

function openCreateModal() {
  form.value = { device_id: '', format: 'Mp4', duration_secs: 0, max_file_size_mb: 0, labels: '' }
  deviceSelectorError.value = ''
  showCreateModal.value = true
  searchDevices('')
}

async function searchDevices(query) {
  if (deviceLoading.value) return
  deviceLoading.value = true
  devicePageOffset = 0
  try {
    const res = await request.get('/devices', {
      params: { limit: DEVICE_PAGE_SIZE, offset: 0, search: query || undefined },
    })
    deviceOptions.value = res.data?.data?.items || []
    deviceTotal.value = res.data?.data?.total || 0
  } catch (e) {
    // silent fail
  } finally {
    deviceLoading.value = false
  }
}

async function submitCreate() {
  if (!form.value.device_id) {
    deviceSelectorError.value = '请选择设备'
    return
  }
  submitting.value = true
  try {
    const labels = form.value.labels ? form.value.labels.split(',').map(s => s.trim()).filter(Boolean) : null
    await recStore.createRecording({ device_id: form.value.device_id, format: form.value.format, duration_secs: form.value.duration_secs || null, max_file_size_mb: form.value.max_file_size_mb || null, labels })
    showCreateModal.value = false
    await fetchAll()
  } finally {
    submitting.value = false
  }
}

async function startRecording(id) { await recStore.startRecording(id); await fetchAll() }
async function stopRecording(id) { await recStore.stopRecording(id); await fetchAll() }
async function pauseRecording(id) { await recStore.pauseRecording(id); await fetchAll() }
async function resumeRecording(id) { await recStore.resumeRecording(id); await fetchAll() }

async function playRecording(rec) {
  playbackRecording.value = rec
  showPlaybackModal.value = true
  playbackUrl.value = ''
  playbackLoading.value = true
  try {
    const files = await recStore.fetchFiles(rec.id)
    if (files?.length > 0) {
      const file = files[0]
      playbackUrl.value = file.playback_url || file.url || `${window.location.origin}${file.path}`
    }
  } catch (e) {
    toast.error('获取播放地址失败: ' + e.message)
  } finally {
    playbackLoading.value = false
  }
}

async function confirmDelete(rec) {
  try {
    await ElMessageBox.confirm(`确定删除录像任务 "${rec.stream_key}" 吗？`, '确认删除', { type: 'warning' })
    await recStore.deleteRecording(rec.id)
    await fetchAll()
  } catch {}
}

async function confirmStop(rec) {
  try {
    await ElMessageBox.confirm(`确定停止录像任务 "${rec.stream_key}" 吗？`, '确认停止', { type: 'warning' })
    await recStore.stopRecording(rec.id)
    await fetchAll()
  } catch {}
}

async function downloadFile(rec) {
  try {
    const files = await recStore.fetchFiles(rec.id)
    if (!files?.length) {
      toast.error('无可下载文件')
      return
    }
    const file = files[0]
    const url = file.playback_url || file.url || `${window.location.origin}${file.path}`
    const a = document.createElement('a')
    a.href = url
    a.download = file.filename || rec.filename || 'recording'
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
  } catch (e) {
    toast.error('下载失败: ' + e.message)
  }
}

function stateTagType(state) {
  const map = { Starting: 'warning', Recording: 'success', Paused: 'warning', Stopping: 'danger', Completed: 'info', Error: 'danger' }
  return map[state] || 'info'
}

function stateLabel(state) {
  const map = { Starting: '启动中', Recording: '录制中', Paused: '已暂停', Stopping: '停止中', Completed: '已完成', Error: '错误' }
  return map[state] || state
}

function formatDuration(secs) {
  if (!secs) return '-'
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = secs % 60
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  return `${m}:${String(s).padStart(2, '0')}`
}

function formatSize(bytes) {
  if (!bytes) return '-'
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`
  if (bytes >= 1e3) return `${(bytes / 1e3).toFixed(0)} KB`
  return `${bytes} B`
}

function formatDate(dateStr) {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString('zh-CN', { hour12: false })
}

function deviceName(deviceTag) {
  const d = deviceStore.devices.find(d => d.device_tag === deviceTag)
  return d ? d.name : deviceTag
}
</script>

<style scoped>
.stats-grid {
  grid-template-columns: repeat(4, 1fr);
}
.filter-bar { margin-bottom: 8px; }
</style>
