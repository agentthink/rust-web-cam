<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">录像文件</h1>
      <div class="page-toolbar">
        <el-button :icon="Refresh" :loading="loading" @click="fetchFiles">刷新</el-button>
      </div>
    </div>
    <div class="page-body">
      <div class="page-grid stats-grid">
        <MetricCard v-for="stat in statCards" :key="stat.label" :label="stat.label" :value="stat.value" />
      </div>

      <DataCard>
        <el-skeleton animated :loading="loading" :rows="5">
          <el-table :data="filteredFiles" style="margin-top: 0">
              <el-table-column label="文件名" prop="filename" min-width="200" show-overflow-tooltip>
                <template #default="{ row }">
                  <span class="filename-cell">{{ row.filename }}</span>
                </template>
              </el-table-column>
              <el-table-column label="文件路径" prop="path" min-width="280" show-overflow-tooltip />
              <el-table-column label="流标识" prop="stream_key" min-width="160" show-overflow-tooltip />
              <el-table-column label="媒体服务器" prop="media_server_name" width="140" show-overflow-tooltip />
              <el-table-column label="大小" width="110">
                <template #default="{ row }"><span>{{ formatSize(row.size) }}</span></template>
              </el-table-column>
              <el-table-column label="时长" width="100">
                <template #default="{ row }"><span>{{ formatDuration(row.duration_secs) }}</span></template>
              </el-table-column>
              <el-table-column label="录制时间" width="160">
                <template #default="{ row }"><span>{{ formatDate(row.created_at) }}</span></template>
              </el-table-column>
              <el-table-column label="操作" width="100">
                <template #default="{ row }">
                  <el-button size="small" type="primary" plain @click="downloadFile(row)">下载</el-button>
                </template>
              </el-table-column>
            </el-table>
        </el-skeleton>
      </DataCard>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Search, Refresh } from '@element-plus/icons-vue'
import { getAllRecordingFiles } from '../api/recordings'
import { useToast } from '../composables/useToast'
import MetricCard from '../components/common/MetricCard.vue'
import DataCard from '../components/common/DataCard.vue'

const toast = useToast()

const files = ref([])
const loading = ref(false)
const searchQuery = ref('')

const totalSize = computed(() => files.value.reduce((sum, f) => sum + (f.size || 0), 0))
const totalDuration = computed(() => files.value.reduce((sum, f) => sum + (f.duration_secs || 0), 0))

const statCards = computed(() => [
  { label: '文件总数', value: files.value.length },
  { label: '总大小', value: formatSize(totalSize.value) },
  { label: '总时长', value: formatDuration(totalDuration.value) },
])

const filteredFiles = computed(() => {
  const q = searchQuery.value.toLowerCase()
  if (!q) return files.value
  return files.value.filter(f =>
    f.filename?.toLowerCase().includes(q) ||
    f.stream_key?.toLowerCase().includes(q) ||
    f.path?.toLowerCase().includes(q)
  )
})

onMounted(() => fetchFiles())

async function fetchFiles() {
  loading.value = true
  try {
    files.value = await getAllRecordingFiles()
  } catch (e) {
    toast.error('加载录像文件失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

function downloadFile(row) {
  const url = row.path || row.url
  if (!url) {
    toast.error('文件路径无效')
    return
  }
  const a = document.createElement('a')
  a.href = url
  a.download = row.filename || 'recording'
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
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

function formatDate(ts) {
  if (!ts) return '-'
  return new Date(ts * 1000).toLocaleString('zh-CN', { hour12: false })
}
</script>

<style scoped>
.stats-grid {
  grid-template-columns: repeat(3, 1fr);
}
.filename-cell {
  font-family: monospace;
  font-size: 12px;
}
</style>
