<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">公开摄像头</h1>
      <div class="page-toolbar">
        <el-input v-model="searchQuery" placeholder="搜索..." clearable style="max-width: 200px">
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-select v-model="protocolFilter" clearable placeholder="全部协议" style="width: 120px">
          <el-option value="RTSP" label="RTSP" />
          <el-option value="GB28181" label="GB28181" />
          <el-option value="ONVIF" label="ONVIF" />
          <el-option value="RTMP" label="RTMP" />
        </el-select>
      </div>
    </div>
    <div class="page-body">
      <DataCard>
        <el-skeleton animated :rows="3" :loading="loading && cameras.length === 0">
          <el-table
            v-if="filteredCameras.length > 0"
            :data="filteredCameras"
            border
            stripe
            style="width: 100%"
            @row-click="openPreview"
          >
            <el-table-column label="ID" prop="id" width="80" align="center" />
            <el-table-column label="名称" prop="name" min-width="150" show-overflow-tooltip />
            <el-table-column label="协议" prop="protocol" width="100" align="center">
              <template #default="{ row }">
                <el-tag size="small">{{ row.protocol }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="100" align="center">
              <template #default="{ row }">
                <StatusDot :status="row.status?.toLowerCase() === 'online' ? 'online' : 'offline'" />
                {{ row.status }}
              </template>
            </el-table-column>
            <el-table-column label="主机" width="180" show-overflow-tooltip>
              <template #default="{ row }">
                <code class="stream-key">{{ row.host }}:{{ row.port }}</code>
              </template>
            </el-table-column>
            <el-table-column label="观看次数" width="100" align="center">
              <template #default="{ row }">
                <span style="display: flex; align-items: center; justify-content: center; gap: 3px;">
                  <el-icon><User /></el-icon>{{ row.view_count }}
                </span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120" align="center" fixed="right">
              <template #default="{ row }">
                <el-button size="small" type="primary" plain @click.stop="openPreview(row)">播放</el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-empty v-else :description="searchQuery || protocolFilter ? '没有找到匹配的摄像头' : '暂无可用公开摄像头'" />
        </el-skeleton>
      </DataCard>
    </div>

    <el-dialog v-model="previewCamera" :title="previewCameraData?.name" width="800px" destroy-on-close>
      <div style="aspect-ratio: 16/9; background: #000; display: flex; align-items: center; justify-content: center">
        <VideoPlayer v-if="previewStreamUrl" :liveSrc="previewStreamUrl" :type="previewStreamType" :isLive="true" :autoplay="true" />
        <el-skeleton v-else-if="previewLoading" animated />
        <el-empty v-else description="视频预览需要先启动拉流">
          <el-button type="primary" @click="$router.push(`/devices/${previewCameraData?.id}`)">前往设备详情</el-button>
        </el-empty>
      </div>
      <el-descriptions :column="2" border size="small" style="margin-top: 12px">
        <el-descriptions-item label="协议">{{ previewCameraData?.protocol }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <span class="status-dot" :class="previewCameraData?.status?.toLowerCase() === 'online' ? 'online' : 'offline'" style="margin-right: 6px" />{{ previewCameraData?.status }}
        </el-descriptions-item>
        <el-descriptions-item label="主机">{{ previewCameraData?.host || '-' }}:{{ previewCameraData?.port || '-' }}</el-descriptions-item>
        <el-descriptions-item label="观看次数">{{ previewCameraData?.view_count }} 次</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { Search, VideoCamera, User } from '@element-plus/icons-vue'
import { request } from '../utils/request'
import VideoPlayer from '../components/VideoPlayer.vue'
import { useDeviceStore } from '../stores/deviceStore'
import StatusDot from '../components/common/StatusDot.vue'
import DataCard from '../components/common/DataCard.vue'

const deviceStore = useDeviceStore()
const cameras = ref([])
const loading = ref(false)
const searchQuery = ref('')
const protocolFilter = ref('')
const previewCamera = ref(false)
const previewCameraData = ref(null)
const previewLoading = ref(false)
const previewStreamUrl = ref(null)
const previewStreamType = ref('auto')

const filteredCameras = computed(() => {
  return cameras.value.filter(camera => {
    const matchesSearch = !searchQuery.value || camera.name.toLowerCase().includes(searchQuery.value.toLowerCase())
    const matchesProtocol = !protocolFilter.value || camera.protocol === protocolFilter.value
    return matchesSearch && matchesProtocol
  })
})

onMounted(async () => {
  loading.value = true
  try {
    const res = await request.get('/public/streams')
    cameras.value = res.data.data || []
  } catch (e) { console.error('Failed to fetch cameras:', e) }
  finally { loading.value = false }
})

async function openPreview(camera) {
  previewCameraData.value = camera
  previewStreamUrl.value = null
  previewStreamType.value = 'auto'
  previewLoading.value = true
  previewCamera.value = true
  try {
    const links = await deviceStore.getPlayLinks(camera.device_tag)
    if (links) {
      previewStreamUrl.value = links.hls || links.web_flv || links.rtsp_signaling || null
      previewStreamType.value = links.hls ? 'hls' : (links.web_flv || links.flv ? 'flv' : 'auto')
    }
  } catch (e) { console.error('Failed to fetch play links:', e) }
  finally { previewLoading.value = false }
}
</script>

<style scoped>
.stream-key {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
</style>
