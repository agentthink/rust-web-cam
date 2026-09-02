<template>
  <div class="page-root">
    <div class="body-content">
      <div class="page-header">
        <div class="header-left">
          <el-button text @click="$router.back()">
            <el-icon><Back /></el-icon> 返回
          </el-button>
          <el-divider direction="vertical" />
          <span class="page-title">{{ stream?.stream_key || `流 #${id}` }}</span>
          <el-tag v-if="stream" :type="stateType(stream.state)" size="small" style="margin-left: 8px">
            {{ stateLabel(stream.state) }}
          </el-tag>
        </div>
        <div class="header-right">
          <el-button :icon="Refresh" size="small" :loading="loading" @click="fetchData">刷新</el-button>
          <el-button 
            v-if="stream?.state === 'Error' || stream?.state === 'Recovering' || stream?.state === 'Idle' || stream?.state === 'Stopped'" 
            type="success" 
            plain 
            size="small" 
            :loading="starting" 
            @click="confirmRestart"
          >重启</el-button>
          <el-button 
            v-if="stream?.state === 'Active' || stream?.state === 'Starting'" 
            type="danger" 
            plain 
            size="small" 
            :loading="stopping" 
            @click="confirmStop"
          >停止流</el-button>
        </div>
      </div>

      <el-row :gutter="16" v-if="stream">
        <el-col :span="16">
          <el-card shadow="never">
            <template #header>
              <span>直播预览</span>
            </template>
            <div class="player-wrapper">
              <VideoPlayer
                v-if="playingUrl"
                ref="playerRef"
                :liveSrc="playingUrl"
                :type="playingType"
                :isLive="true"
                :autoplay="true"
              />
              <div v-else class="player-placeholder">
                <el-icon size="48"><VideoPlay /></el-icon>
                <p>暂无直播流</p>
                <el-button type="primary" size="small" @click="startPreview" :loading="previewLoading">
                  开始预览
                </el-button>
              </div>
            </div>
          </el-card>

          <el-card shadow="never" style="margin-top: 16px">
            <template #header>
              <span>直播链接</span>
              <el-button text size="small" style="float: right" @click="copyAll">
                复制全部
              </el-button>
            </template>
            <el-table :data="playLinkRows" border size="small">
              <el-table-column label="协议" width="100" align="center">
                <template #default="{ row }">
                  <el-tag size="small">{{ row.protocol }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column label="地址" min-width="300">
                <template #default="{ row }">
                  <el-tooltip :content="row.url" placement="top" :show-after="300">
                    <span class="url-text">{{ row.url }}</span>
                  </el-tooltip>
                </template>
              </el-table-column>
              <el-table-column label="操作" width="120" align="center">
                <template #default="{ row }">
                  <el-button size="small" @click="playProtocol(row.protocol, row.url)">播放</el-button>
                  <el-button size="small" @click="copyUrl(row.url)">复制</el-button>
                </template>
              </el-table-column>
            </el-table>
          </el-card>
        </el-col>

        <el-col :span="8">
          <el-card shadow="never">
            <template #header>基本信息</template>
            <el-descriptions :column="1" border size="small">
              <el-descriptions-item label="流 ID">{{ stream.id }}</el-descriptions-item>
              <el-descriptions-item label="流标识">{{ stream.stream_key }}</el-descriptions-item>
              <el-descriptions-item label="状态">
                <el-tag :type="stateType(stream.state)" size="small">{{ stateLabel(stream.state) }}</el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="自动恢复">
                <el-tag size="small" :type="stream.auto_recover ? 'success' : 'info'">
                  {{ stream.auto_recover ? '是' : '否' }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="重试次数">{{ stream.retry_count || 0 }} / {{ stream.max_retries || 20 }}</el-descriptions-item>
              <el-descriptions-item label="错误信息" v-if="stream.last_error">
                <span style="color: var(--color-danger); font-size: 12px;">{{ stream.last_error }}</span>
              </el-descriptions-item>
              <el-descriptions-item label="观看人数">{{ stream.viewer_count || 0 }}</el-descriptions-item>
              <el-descriptions-item label="媒体服务器">{{ stream.media_server_id }}</el-descriptions-item>
              <el-descriptions-item label="设备 ID">{{ stream.device_id }}</el-descriptions-item>
              <el-descriptions-item label="创建时间">{{ formatTime(stream.created_at) }}</el-descriptions-item>
            </el-descriptions>
          </el-card>

          <el-card shadow="never" style="margin-top: 16px">
            <template #header>
              <span>设备信息</span>
              <el-button text size="small" style="float: right" @click="$router.push(`/devices/${device.device_tag}`)">
                查看详情
              </el-button>
            </template>
            <div v-if="device">
              <el-descriptions :column="1" border size="small">
                <el-descriptions-item label="名称">{{ device.name }}</el-descriptions-item>
                <el-descriptions-item label="协议">
                  <el-tag size="small">{{ device.protocol }}</el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="状态">
                  <span class="status-dot" :class="device.status?.toLowerCase()" />
                  {{ device.status }}
                </el-descriptions-item>
                <el-descriptions-item label="设备地址">{{ device.host }}:{{ device.port }}</el-descriptions-item>
                <el-descriptions-item label="媒体服务器">{{ device.media_server_tag || '-' }}</el-descriptions-item>
                <el-descriptions-item label="应用">{{ device.app || '-' }}</el-descriptions-item>
                <el-descriptions-item label="流标识">{{ device.stream_key || '-' }}</el-descriptions-item>
                <el-descriptions-item label="设备标签">{{ device.device_tag || '-' }}</el-descriptions-item>
                <el-descriptions-item label="父设备">{{ device.parent_device_tag || '-' }}</el-descriptions-item>
                <el-descriptions-item label="公开流">
                  <el-tag size="small" :type="device.is_public ? 'success' : 'info'">
                    {{ device.is_public ? '是' : '否' }}
                  </el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="有流">
                  <el-tag size="small" :type="device.has_stream ? 'success' : 'info'">
                    {{ device.has_stream ? '是' : '否' }}
                  </el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="创建时间">{{ formatTime(device.created_at) }}</el-descriptions-item>
                <el-descriptions-item label="推流地址" v-if="device.push_urls?.length">
                  <div v-for="(url, i) in device.push_urls" :key="i" class="url-item">
                    <el-tag size="small" type="warning">{{ url.protocol }}</el-tag>
                    <span class="url-text">{{ url.url }}</span>
                  </div>
                </el-descriptions-item>
                <el-descriptions-item label="拉流地址" v-if="device.pull_urls?.length">
                  <div v-for="(url, i) in device.pull_urls" :key="i" class="url-item">
                    <el-tag size="small" type="info">{{ url.protocol }}</el-tag>
                    <span class="url-text">{{ url.url }}</span>
                  </div>
                </el-descriptions-item>
              </el-descriptions>
            </div>
            <el-skeleton v-else animated :rows="3" />
          </el-card>
        </el-col>
      </el-row>

      <el-skeleton v-else-if="loading" animated :rows="8" />

      <el-empty v-else description="流不存在" />
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Back, VideoPlay } from '@element-plus/icons-vue'
import VideoPlayer from '../components/VideoPlayer.vue'
import { useStreamStore } from '../stores/streamStore'
import { useDeviceStore } from '../stores/deviceStore'
import * as api from '../api/streams'

const route = useRoute()
const router = useRouter()
const streamStore = useStreamStore()
const deviceStore = useDeviceStore()

const id = computed(() => Number(route.params.id))
const loading = ref(false)
const stopping = ref(false)
const starting = ref(false)
const previewLoading = ref(false)
const playingUrl = ref('')
const playingType = ref('auto')
const stream = ref(null)
const playLinks = ref(null)
const device = ref(null)
const playerRef = ref(null)

const stateType = (state) => {
  const map = { Active: 'success', Idle: 'info', Starting: 'warning', Stopping: 'warning', Recovering: 'warning', Error: 'danger', Stopped: 'info' }
  return map[state] || 'info'
}

const stateLabel = (state) => {
  const map = { Active: '活跃', Idle: '空闲', Starting: '启动中', Stopping: '停止中', Recovering: '恢复中', Error: '错误', Stopped: '已停止' }
  return map[state] || state
}

const playLinkRows = computed(() => {
  if (!playLinks.value) return []
  const rows = []
  if (playLinks.value.hls) rows.push({ protocol: 'HLS', url: playLinks.value.hls })
  if (playLinks.value.flv) rows.push({ protocol: 'FLV', url: playLinks.value.flv })
  if (playLinks.value.web_flv) rows.push({ protocol: 'WebFLV', url: playLinks.value.web_flv })
  if (playLinks.value.rtsp_signaling) rows.push({ protocol: 'RTSP', url: playLinks.value.rtsp_signaling })
  if (playLinks.value.webrtc) rows.push({ protocol: 'WebRTC', url: playLinks.value.webrtc })
  return rows
})

const protocolMap = { 'HLS': 'hls', 'FLV': 'flv', 'WebFLV': 'wsflv', 'RTSP': 'rtsp', 'WebRTC': 'webrtc' }

const playLinksFromStore = computed(() => playLinks.value)

async function fetchData() {
  loading.value = true
  try {
    const [streamData, linksData] = await Promise.all([
      api.getStream(id.value),
      api.getStreamPlayLinks(id.value),
    ])
    stream.value = streamData
    playLinks.value = linksData

    if (stream.value?.device_id) {
      const dev = await deviceStore.fetchDevice(stream.value.device_id)
      device.value = dev || null
    }
  } catch (e) {
    ElMessage.error('加载失败: ' + (e?.message || e))
  } finally {
    loading.value = false
  }
}

async function startPreview() {
  if (!playLinks.value?.hls && !playLinks.value?.flv && !playLinks.value?.web_flv) {
    ElMessage.warning('无可用播放链接')
    return
  }
  playingUrl.value = playLinks.value.hls || playLinks.value.flv || playLinks.value.web_flv
  if (playingUrl.value.includes('.m3u8')) {
    playingType.value = 'hls'
  } else if (playingUrl.value.startsWith('ws://') || playingUrl.value.startsWith('wss://')) {
    playingType.value = 'wsflv'
  } else {
    playingType.value = 'flv'
  }
}

function playProtocol(protocol, url) {
  playingUrl.value = url
  playingType.value = protocolMap[protocol] || 'auto'
}

async function copyUrl(url) {
  try {
    await navigator.clipboard.writeText(url)
  } catch {
    const el = document.createElement('textarea')
    el.value = url
    document.body.appendChild(el)
    el.select()
    document.execCommand('copy')
    document.body.removeChild(el)
  }
  ElMessage.success('已复制')
}

async function copyAll() {
  const text = playLinkRows.value.map(r => `${r.protocol}: ${r.url}`).join('\n')
  await copyUrl(text)
}

async function confirmStop() {
  try {
    await ElMessageBox.confirm('确定停止此流吗？', '确认', { type: 'warning' })
    stopping.value = true
    await api.stopStream(id.value)
    ElMessage.success('流已停止')
    router.push('/streams')
  } catch (e) {
    if (e !== 'cancel') ElMessage.error('停止失败')
  } finally {
    stopping.value = false
  }
}

async function confirmRestart() {
  try {
    await ElMessageBox.confirm('确定重启此流吗？', '确认', { type: 'warning' })
    starting.value = true
    await streamStore.restartStream(id.value)
    ElMessage.success('流已启动')
    await fetchData()
  } catch (e) {
    if (e !== 'cancel') ElMessage.error('启动失败')
  } finally {
    starting.value = false
  }
}

function formatTime(ts) {
  if (!ts) return '-'
  const d = new Date(ts)
  return d.toLocaleString()
}

onMounted(fetchData)

onUnmounted(() => {
  if (playerRef.value) {
    playerRef.value.destroy()
  }
})
</script>

<style scoped>
.page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px; }
.header-left { display: flex; align-items: center; }
.header-right { display: flex; gap: 8px; }
.page-title { font-size: var(--text-xl); font-weight: var(--weight-semibold); color: var(--text-primary); }

.player-wrapper { background: var(--video-bg, #000); border-radius: var(--radius-base); overflow: hidden; }
.player-placeholder { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 300px; color: var(--text-muted); gap: var(--space-3); }
.url-text { font-family: var(--font-mono); font-size: var(--text-xs); word-break: break-all; }
.status-dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: var(--space-1); }
.status-dot.online { background: var(--color-success); }
.status-dot.offline { background: var(--text-muted); }
.url-item { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
</style>
