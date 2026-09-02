<template>
  <div class="page-container">
    <div class="page-header">
      <div class="header-left">
        <el-button text @click="$router.back()">
          <el-icon><ArrowLeft /></el-icon> 返回
        </el-button>
        <h1 class="page-title">{{ channel?.name || '通道详情' }}</h1>
      </div>
      <div class="page-toolbar">
        <el-button :icon="Refresh" @click="fetchData">刷新</el-button>
      </div>
    </div>

    <div class="page-body" v-if="channel">
      <div class="detail-layout">
        <div class="detail-main">
          <el-card shadow="never" class="player-card">
            <template #header>
              <div class="card-header">
                <span>视频预览</span>
                <div class="stream-selector">
                  <el-radio-group v-model="selectedStreamType" size="small">
                    <el-radio-button value="hls" :disabled="!playLinks?.hls">HLS</el-radio-button>
                    <el-radio-button value="flv" :disabled="!playLinks?.flv">FLV</el-radio-button>
                    <el-radio-button value="wsflv" :disabled="!playLinks?.web_flv">WS-FLV</el-radio-button>
                    <el-radio-button value="rtsp" :disabled="!playLinks?.rtsp_signaling">RTSP</el-radio-button>
                  </el-radio-group>
                </div>
              </div>
            </template>
            <div class="player-wrapper">
              <VideoPlayer
                v-if="currentStreamUrl && !loading"
                :liveSrc="currentStreamUrl"
                :type="selectedStreamType"
                :isLive="true"
                :autoplay="true"
              />
              <div v-else-if="loading" class="loading-state">
                <el-icon class="is-loading"><Loading /></el-icon>
                <span>加载中...</span>
              </div>
              <div v-else class="empty-state">
                <el-icon><VideoPlay /></el-icon>
                <span>暂无视频流</span>
              </div>
            </div>
          </el-card>

          <el-card shadow="never" class="links-card">
            <template #header><span>播放链接</span></template>
            <div class="links-list" v-if="playLinks">
              <div class="link-item" v-if="playLinks.rtsp_signaling">
                <div class="link-info">
                  <el-tag size="small" type="info">RTSP</el-tag>
                  <span class="link-url">{{ playLinks.rtsp_signaling }}</span>
                </div>
                <el-button size="small" @click="copyLink(playLinks.rtsp_signaling)">复制</el-button>
              </div>
              <div class="link-item" v-if="playLinks.flv">
                <div class="link-info">
                  <el-tag size="small" type="warning">FLV</el-tag>
                  <span class="link-url">{{ playLinks.flv }}</span>
                </div>
                <el-button size="small" @click="copyLink(playLinks.flv)">复制</el-button>
              </div>
              <div class="link-item" v-if="playLinks.web_flv">
                <div class="link-info">
                  <el-tag size="small" type="danger">HTTP-FLV</el-tag>
                  <span class="link-url">{{ playLinks.web_flv }}</span>
                </div>
                <el-button size="small" @click="copyLink(playLinks.web_flv)">复制</el-button>
              </div>
              <div class="link-item" v-if="playLinks.hls">
                <div class="link-info">
                  <el-tag size="small" type="success">HLS</el-tag>
                  <span class="link-url">{{ playLinks.hls }}</span>
                </div>
                <el-button size="small" @click="copyLink(playLinks.hls)">复制</el-button>
              </div>
              <div class="link-item" v-if="playLinks.webrtc">
                <div class="link-info">
                  <el-tag size="small" type="primary">WebRTC</el-tag>
                  <span class="link-url">{{ playLinks.webrtc }}</span>
                </div>
                <el-button size="small" @click="copyLink(playLinks.webrtc)">复制</el-button>
              </div>
            </div>
            <el-empty v-else description="暂无播放链接" />
          </el-card>
        </div>

        <div class="detail-sidebar">
          <el-card shadow="never">
            <template #header><span>通道信息</span></template>
            <el-descriptions :column="1" border size="small">
              <el-descriptions-item label="设备标识">{{ channel.device_tag }}</el-descriptions-item>
              <el-descriptions-item label="通道标识">{{ channel.channel_tag }}</el-descriptions-item>
              <el-descriptions-item label="状态">
                <el-tag size="small" :type="statusType">{{ channel.status }}</el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="类型">{{ channel.device_type }}</el-descriptions-item>
              <el-descriptions-item label="地址">{{ channel.ip_address || '-' }}:{{ channel.port || '-' }}</el-descriptions-item>
              <el-descriptions-item label="厂商">{{ channel.manufacturer || '-' }}</el-descriptions-item>
              <el-descriptions-item label="型号">{{ channel.model || '-' }}</el-descriptions-item>
              <el-descriptions-item label="创建时间">{{ formatDate(channel.created_at) }}</el-descriptions-item>
            </el-descriptions>
          </el-card>

          <el-card v-if="channel.device_type === 'ONVIF' || channel.device_type === 'GB28181'" shadow="never" style="margin-top: 12px">
            <template #header><span>PTZ 云台控制</span></template>
            <div class="ptz-control">
              <div class="ptz-grid">
                <button class="ptz-dir-btn ptz-dir-up" @mousedown="ptzControl('up', 1)" @mouseup="ptzStop" @mouseleave="ptzStop">▲</button>
                <button class="ptz-dir-btn ptz-dir-lf" @mousedown="ptzControl('left', 1)" @mouseup="ptzStop" @mouseleave="ptzStop">◀</button>
                <button class="ptz-stop-btn" @click="ptzControl('stop', 1)">●</button>
                <button class="ptz-dir-btn ptz-dir-rt" @mousedown="ptzControl('right', 1)" @mouseup="ptzStop" @mouseleave="ptzStop">▶</button>
                <button class="ptz-dir-btn ptz-dir-dn" @mousedown="ptzControl('down', 1)" @mouseup="ptzStop" @mouseleave="ptzStop">▼</button>
              </div>
              <div class="ptz-zoom">
                <button class="ptz-opt-btn" @mousedown="ptzControl('zoom_in', 1)" @mouseup="ptzStop" @mouseleave="ptzStop">ZOOM+</button>
                <button class="ptz-opt-btn" @mousedown="ptzControl('zoom_out', 1)" @mouseup="ptzStop" @mouseleave="ptzStop">ZOOM-</button>
              </div>
              <div class="ptz-speed">
                <span>速度</span>
                <el-slider v-model="ptzSpeed" :min="1" :max="100" show-stops style="width: 100px" />
                <span>{{ ptzSpeed }}</span>
              </div>
            </div>
          </el-card>

          <el-card v-if="channel.device_type === 'ONVIF' || channel.device_type === 'GB28181'" shadow="never" style="margin-top: 12px">
            <template #header>
              <div class="card-header">
                <span>预制位</span>
                <el-button size="small" type="primary" @click="showPresetDialog = true">保存当前</el-button>
              </div>
            </template>
            <div v-if="presets.length === 0" class="empty-presets">暂无预制位</div>
            <div v-else class="preset-list">
              <div v-for="preset in presets" :key="preset.token" class="preset-item">
                <span class="preset-name">{{ preset.name }}</span>
                <div class="preset-actions">
                  <el-button size="small" @click="gotoPreset(preset)">调用</el-button>
                  <el-button size="small" type="danger" plain @click="removePreset(preset)">删除</el-button>
                </div>
              </div>
            </div>
          </el-card>
        </div>
      </div>
    </div>

    <el-dialog v-model="showPresetDialog" title="保存预制位" width="400px">
      <el-input v-model="newPresetName" placeholder="例如: 大门入口" />
      <template #footer>
        <el-button @click="showPresetDialog = false">取消</el-button>
        <el-button type="primary" :disabled="!newPresetName.trim()" @click="savePreset">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Refresh, VideoPlay, Loading } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useChannelStore } from '../stores/channelStore'
import VideoPlayer from '../components/VideoPlayer.vue'

const route = useRoute()
const router = useRouter()
const channelStore = useChannelStore()

const channel = ref(null)
const playLinks = ref(null)
const loading = ref(false)
const selectedStreamType = ref('hls')
const ptzSpeed = ref(50)
const presets = ref([])
const showPresetDialog = ref(false)
const newPresetName = ref('')
let ptzStopTimer = null

const statusType = computed(() => {
  if (!channel.value) return 'info'
  const s = channel.value.status?.toLowerCase()
  if (s === 'online') return 'success'
  if (s === 'maintaining') return 'warning'
  if (s === 'error') return 'danger'
  return 'info'
})

const currentStreamUrl = computed(() => {
  if (!playLinks.value) return null
  switch (selectedStreamType.value) {
    case 'hls': return playLinks.value.hls
    case 'flv': return playLinks.value.flv
    case 'wsflv': return playLinks.value.web_flv
    case 'rtsp': return playLinks.value.rtsp_signaling
    default: return playLinks.value.hls || playLinks.value.web_flv || null
  }
})

async function fetchData() {
  loading.value = true
  try {
    const { deviceTag, channelTag } = route.params
    channel.value = await channelStore.getChannel(deviceTag, channelTag)
    playLinks.value = await channelStore.getChannelPlayLinks(deviceTag, channelTag)
    if (channel.value.device_type === 'ONVIF' || channel.value.device_type === 'GB28181') {
      await fetchPresets()
    }
    updateDefaultStreamType()
  } catch (e) {
    ElMessage.error('获取通道信息失败')
  } finally {
    loading.value = false
  }
}

function updateDefaultStreamType() {
  if (playLinks.value?.hls) selectedStreamType.value = 'hls'
  else if (playLinks.value?.web_flv) selectedStreamType.value = 'wsflv'
  else if (playLinks.value?.flv) selectedStreamType.value = 'flv'
  else if (playLinks.value?.rtsp_signaling) selectedStreamType.value = 'rtsp'
}

async function fetchPresets() {
  try {
    const { deviceTag, channelTag } = route.params
    presets.value = await channelStore.getChannelPtzPresets(deviceTag, channelTag)
  } catch {
    presets.value = []
  }
}

async function ptzControl(cmd) {
  clearTimeout(ptzStopTimer)
  try {
    const { deviceTag, channelTag } = route.params
    await channelStore.channelPtzControl(deviceTag, channelTag, cmd, ptzSpeed.value)
  } catch (e) {
    console.error('PTZ control failed:', e)
  }
}

function ptzStop() {
  ptzStopTimer = setTimeout(async () => {
    try {
      const { deviceTag, channelTag } = route.params
      await channelStore.channelPtzControl(deviceTag, channelTag, 'stop', ptzSpeed.value)
    } catch {}
  }, 100)
}

async function savePreset() {
  if (!newPresetName.value.trim()) return
  try {
    const { deviceTag, channelTag } = route.params
    await channelStore.createChannelPtzPreset(deviceTag, channelTag, newPresetName.value.trim())
    await fetchPresets()
    showPresetDialog.value = false
    newPresetName.value = ''
    ElMessage.success('预制位已保存')
  } catch {
    ElMessage.error('保存预制位失败')
  }
}

async function gotoPreset(preset) {
  try {
    const { deviceTag, channelTag } = route.params
    await channelStore.channelPtzControl(deviceTag, channelTag, 'goto_preset', ptzSpeed.value, preset.token)
  } catch {
    ElMessage.error('调用预制位失败')
  }
}

async function removePreset(preset) {
  try {
    const { deviceTag, channelTag } = route.params
    await channelStore.deleteChannelPtzPreset(deviceTag, channelTag, preset.token)
    await fetchPresets()
    ElMessage.success('预制位已删除')
  } catch {
    ElMessage.error('删除预制位失败')
  }
}

function copyLink(url) {
  navigator.clipboard.writeText(url)
  ElMessage.success('已复制到剪贴板')
}

function formatDate(dateStr) {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString('zh-CN')
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
.page-container {
  padding: 20px;
  min-height: 100vh;
  background: var(--el-bg-page);
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.page-toolbar {
  display: flex;
  gap: 8px;
}

.detail-layout {
  display: grid;
  grid-template-columns: 1fr 360px;
  gap: 16px;
}

.detail-main {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.detail-sidebar {
  display: flex;
  flex-direction: column;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.player-card :deep(.el-card__header) {
  padding: 12px 16px;
}

.player-wrapper {
  aspect-ratio: 16 / 9;
  background: #000;
  border-radius: 4px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: #666;
  font-size: 14px;
}

.empty-state .el-icon {
  font-size: 48px;
  color: #999;
}

.links-card .links-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.link-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: var(--el-fill-color-lighter);
  border-radius: 6px;
  gap: 12px;
}

.link-info {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
}

.link-url {
  font-family: 'SF Mono', Monaco, monospace;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.stream-selector {
  display: flex;
  gap: 4px;
}

.stream-selector :deep(.el-radio-button__inner) {
  padding: 5px 12px;
  font-size: 12px;
}

.ptz-control {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.ptz-grid {
  display: grid;
  grid-template-columns: repeat(3, 38px);
  grid-template-rows: repeat(3, 38px);
  gap: 4px;
}

.ptz-dir-btn {
  width: 38px;
  height: 38px;
  border: 1px solid var(--el-border-color);
  background: var(--el-fill-color-light);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  transition: all 0.15s;
}

.ptz-dir-btn:hover {
  border-color: var(--el-color-primary);
  color: var(--el-color-primary);
}

.ptz-dir-btn:active {
  background: var(--el-color-primary-light-9);
}

.ptz-dir-up { grid-column: 2; grid-row: 1; justify-self: center; }
.ptz-dir-lf { grid-column: 1; grid-row: 2; justify-self: center; }

.ptz-stop-btn {
  grid-column: 2; grid-row: 2; justify-self: center;
  width: 34px; height: 34px;
  border: 1px solid rgba(239, 68, 68, 0.3);
  background: rgba(239, 68, 68, 0.08);
  border-radius: 50%;
  color: var(--el-color-danger);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  transition: all 0.15s;
}

.ptz-stop-btn:hover {
  border-color: var(--el-color-danger);
  background: rgba(239, 68, 68, 0.15);
}

.ptz-dir-rt { grid-column: 3; grid-row: 2; justify-self: center; }
.ptz-dir-dn { grid-column: 2; grid-row: 3; justify-self: center; }

.ptz-zoom {
  display: flex;
  gap: 8px;
  width: 100%;
}

.ptz-opt-btn {
  flex: 1;
  height: 32px;
  border: 1px solid var(--el-border-color);
  background: var(--el-fill-color-light);
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
  font-weight: 500;
  transition: all 0.15s;
}

.ptz-opt-btn:hover {
  border-color: var(--el-color-primary);
  color: var(--el-color-primary);
}

.ptz-speed {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  width: 100%;
}

.ptz-speed span:first-child {
  width: 28px;
}

.ptz-speed span:last-child {
  width: 28px;
  text-align: right;
}

.empty-presets {
  text-align: center;
  padding: 24px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.preset-list {
  display: flex;
  flex-direction: column;
}

.preset-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 0;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.preset-item:last-child {
  border-bottom: none;
}

.preset-name {
  font-size: 13px;
  color: var(--el-text-color-primary);
}

.preset-actions {
  display: flex;
  gap: 4px;
}

@media (max-width: 1024px) {
  .detail-layout {
    grid-template-columns: 1fr;
  }
}
</style>
