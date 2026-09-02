<template>
  <div class="page-container">
    <div class="page-body">
      <el-row :gutter="12" align="middle" class="page-toolbar">
        <el-col :span="12">
          <el-select v-model="selectedLayoutId" placeholder="选择布局" style="width: 240px" @change="onLayoutChange">
            <el-option value="" label="-- 选择布局 --" />
            <el-option v-for="l in layoutStore.sortedLayouts" :key="l.id" :label="`${l.name} (${l.rows}×${l.cols})${l.is_default ? ' ★' : ''}`" :value="l.id" />
          </el-select>
          <el-tag v-if="wsStatusText" style="margin-left: 12px" :type="wsStatus === 'connected' ? 'success' : 'info'" size="small">{{ wsStatusText }}</el-tag>
        </el-col>
        <el-col :span="12" style="text-align: right">
          <el-button :icon="Edit" @click="$router.push('/video-wall/designer')">设计布局</el-button>
          <el-button v-if="selectedLayoutId" type="danger" plain :icon="Delete" @click="deleteLayout">删除布局</el-button>
          <el-button :icon="VideoPlay" @click="stopAll">全部停止</el-button>
        </el-col>
      </el-row>

      <div class="wall-layout">
        <div class="wall-sidebar">
          <el-card shadow="never">
            <template #header>
              <span>设备</span>
              <el-radio-group v-model="groupMode" size="small" style="margin-left: 8px">
                <el-radio-button value="region">地区</el-radio-button>
                <el-radio-button value="group">分组</el-radio-button>
              </el-radio-group>
            </template>
            <el-scrollbar height="480px">
              <el-empty v-if="deviceStore.loading && deviceStore.devices.length === 0" description="加载中..." :image-size="40" />
              <el-empty v-else-if="deviceStore.devices.length === 0" description="暂无设备" :image-size="40" />
              <el-tree
                v-else
                :data="deviceTree"
                :props="{ label: 'name', children: 'children' }"
                :expand-on-click-node="false"
                node-key="key"
              >
                <template #default="{ data }">
                  <div
                    v-if="data.isDevice"
                    class="device-item"
                    draggable="true"
                    @dragstart="onDragStart($event, data.device)"
                    @dragend="onDragEnd"
                  >
                    <span class="status-dot" :class="data.device.status?.toLowerCase() === 'online' ? 'online' : 'offline'" />
                    <el-text truncated style="flex: 1">{{ data.name }}</el-text>
                    <el-tag size="small">{{ data.device.protocol }}</el-tag>
                  </div>
                  <span v-else>{{ data.name }} ({{ data.deviceCount || 0 }})</span>
                </template>
              </el-tree>
            </el-scrollbar>
          </el-card>

          <el-card shadow="never" style="margin-top: 8px">
            <template #header><span>绑定信息</span></template>
            <el-empty v-if="Object.keys(bindings).length === 0" description="暂无绑定" :image-size="40" />
            <div v-for="(binding, slotId) in bindings" :key="slotId" class="binding-item">
              <el-text truncated style="flex: 1">{{ binding.name || binding.deviceTag }}</el-text>
              <el-button size="small" type="danger" plain circle :icon="Close" @click="stopSlot(slotId)" />
            </div>
          </el-card>
        </div>

        <div
          ref="playerPanel"
          class="wall-player-area"
          @dragover.prevent="onDragOver"
          @drop="onDrop"
        >
          <el-empty v-if="!selectedLayoutId" description="请先选择布局" />
          <el-empty v-else-if="!playerReady" description="播放器加载中..." />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { Edit, Delete, VideoPlay, Close } from '@element-plus/icons-vue'
import { ElMessageBox } from 'element-plus'
import { usePlayerLayoutStore } from '../stores/playerLayoutStore'
import { useDeviceStore } from '../stores/deviceStore'
import { useRegionStore } from '../stores/regionStore'
import { useGroupStore } from '../stores/groupStore'
import { useToast } from '../composables/useToast'
import { request } from '../utils/request'

const layoutStore = usePlayerLayoutStore()
const deviceStore = useDeviceStore()
const regionStore = useRegionStore()
const groupStore = useGroupStore()
const toast = useToast()

const groupMode = ref('region')

const deviceTree = computed(() => {
  if (groupMode.value === 'region') {
    return buildRegionTree()
  } else {
    return buildGroupTree()
  }
})

function buildRegionTree() {
  const roots = JSON.parse(JSON.stringify(regionStore.regionTree))
  const map = new Map()
  function collect(nodes) {
    for (const node of nodes) {
      map.set(node.code, node)
      node.children = node.children || []
      collect(node.children)
    }
  }
  collect(roots)
  for (const device of deviceStore.devices) {
    const region = map.get(device.region_code)
    if (region) {
      region.children.push({ name: device.name, key: `device_${device.device_tag}`, isDevice: true, device, children: [] })
    }
  }
  calcDeviceCount(roots)
  const allDevices = deviceStore.devices.map(d => ({
    name: d.name, key: `device_${d.device_tag}`, isDevice: true, device: d, children: []
  }))
  return [{ name: '全部', key: 'all', children: allDevices }, ...roots]
}

function buildGroupTree() {
  const roots = JSON.parse(JSON.stringify(groupStore.groupTree))
  const map = new Map()
  function collect(nodes) {
    for (const node of nodes) {
      map.set(node.id, node)
      node.children = node.children || []
      collect(node.children)
    }
  }
  collect(roots)
  for (const device of deviceStore.devices) {
    const group = map.get(device.group_id)
    if (group) {
      group.children.push({ name: device.name, key: `device_${device.device_tag}`, isDevice: true, device, children: [] })
    }
  }
  calcDeviceCount(roots)
  const allDevices = deviceStore.devices.map(d => ({
    name: d.name, key: `device_${d.device_tag}`, isDevice: true, device: d, children: []
  }))
  return [{ name: '全部', key: 'all', children: allDevices }, ...roots]
}

function calcDeviceCount(nodes) {
  for (const node of nodes) {
    calcDeviceCount(node.children || [])
    const count = (node.children || []).reduce((sum, c) => sum + (c.deviceCount || (c.isDevice ? 1 : 0)), 0)
    node.deviceCount = count
  }
}

const selectedLayoutId = ref('')
const playerPanel = ref(null)
const playerReady = ref(false)
const wsStatus = ref('disconnected')
const wsStatusText = ref('')
const bindings = ref({})

let playerWindow = null
let draggingDeviceId = null
let draggingDevice = null
let draggingRtspUrl = null

async function getPlayLinks(deviceTag, channelTag) {
  const res = await request.get(`/channels/${deviceTag}/${channelTag}/play-links`)
  return res.data.data || {}
}

async function getStreamRtspUrl(deviceTag) {
  const res = await request.get(`/streams/by-device/${deviceTag}`)
  const streams = res.data.data || []
  const stream = streams.find(s => s.device_tag === deviceTag && s.state === 'Active')
  if (!stream) return null
  const urlRes = await request.get(`/streams/${stream.id}/play`, { params: { protocol: 'rtsp' } })
  return urlRes.data.data
}

onMounted(async () => {
  await deviceStore.fetchDevices(500, 0)
  await regionStore.fetchRegionTree()
  await groupStore.fetchGroupTree()
  await layoutStore.fetchLayouts()

  if (window.clientApp) {
    wsStatus.value = 'connected'
    wsStatusText.value = '播放器就绪'
  } else {
    wsStatusText.value = '播放器未加载'
  }

  if (layoutStore.defaultLayout) {
    selectedLayoutId.value = layoutStore.defaultLayout.id
    await onLayoutChange()
  }
})

onUnmounted(() => {
  if (playerWindow) { playerWindow.close(); playerWindow = null }
})

async function onLayoutChange() {
  if (playerWindow) { await playerWindow.close(); playerWindow = null; bindings.value = {} }
  if (!selectedLayoutId.value) return
  if (!window.clientApp) { toast.error('播放器未加载'); return }

  const layout = await layoutStore.fetchLayout(Number(selectedLayoutId.value))
  if (!layout) return

  await nextTick()

  try {
    playerWindow = await window.clientApp.create(`视频墙 - ${layout.name}`, { rows: layout.rows, cols: layout.cols }, playerPanel.value)

    playerWindow.onPlayerStateChange = (playerId, state, extra) => {
      for (const [slotId, binding] of Object.entries(bindings.value)) {
        if (binding.playerId === playerId) {
          if (state === 'error') toast.error(`播放错误: ${extra?.message || state}`)
          break
        }
      }
    }

    playerWindow.onPlayerAdded = (player) => {}
    playerWindow.onPlayerRemoved = (playerId) => {
      for (const [slotId, binding] of Object.entries(bindings.value)) {
        if (binding.playerId === playerId) { delete bindings.value[slotId]; break }
      }
    }

    playerWindow.onError = (type, data) => { toast.error(`播放错误: ${type}`) }
    playerWindow.onClose = () => { playerWindow = null; bindings.value = {}; playerReady.value = false }

    playerWindow.layout.setGrid(layout.rows, layout.cols)
    playerReady.value = true
  } catch (err) {
    toast.error('创建视频墙失败: ' + (err.message || String(err)))
    playerReady.value = false
  }
}

async function onDragStart(event, device) {
  const deviceTag = device.device_tag
  draggingDeviceId = deviceTag; draggingDevice = device; draggingRtspUrl = null
  const channelTag = deviceTag
  const payload = JSON.stringify({ deviceTag, name: device.name, rtspUrl: null })
  event.dataTransfer.setData('application/json', payload); event.dataTransfer.effectAllowed = 'copy'
  try {
    const links = await getPlayLinks(deviceTag, channelTag)
    let rtspUrl = links.rtsp_signaling || links.rtsp_media || null
    if (!rtspUrl) rtspUrl = await getStreamRtspUrl(deviceTag)
    if (rtspUrl) {
      draggingRtspUrl = rtspUrl
      event.dataTransfer.setData('application/json', JSON.stringify({ deviceTag, name: device.name, rtspUrl }))
    }
  } catch (err) { console.warn('[PlayerWall] pre-fetch rtsp url failed:', err) }
}

function onDragEnd() { draggingDeviceId = null; draggingDevice = null; draggingRtspUrl = null }
function onDragOver(event) { event.dataTransfer.dropEffect = 'copy' }

async function onDrop(event) {
  event.preventDefault()
  if (!playerWindow) { toast.error('请先选择布局'); return }
  const rawData = event.dataTransfer.getData('application/json')
  if (!rawData) return
  let deviceTag, deviceName, rtspUrl
  try {
    const data = JSON.parse(rawData)
    deviceTag = data.deviceTag; deviceName = data.name; rtspUrl = data.rtspUrl
  } catch { toast.error('数据格式错误'); return }
  if (!rtspUrl) { toast.error('该设备无可用播放地址'); return }

  const selectedItemId = playerWindow.getSelectedLayoutItemId()
  const layoutItems = playerWindow.layout.items
  let targetItemId = selectedItemId
  if (!targetItemId) {
    const emptyItem = layoutItems.find(item => !item.occupied)
    targetItemId = emptyItem ? emptyItem.id : (layoutItems[0] ? layoutItems[0].id : null)
  }
  if (!targetItemId) { toast.error('布局无空闲格子'); return }

  try {
    playerWindow.selectLayoutItem(targetItemId)
    const player = await playerWindow.playOnSelected(rtspUrl, { deviceTag, name: deviceName })
    bindings.value[targetItemId] = { deviceTag, name: deviceName, playerId: player.id, rtspUrl, slotId: targetItemId }
  } catch (err) { toast.error('播放失败: ' + (err.message || String(err))) }
}

async function stopSlot(slotId) {
  const binding = bindings.value[slotId]
  if (!binding || !playerWindow) return
  try { await playerWindow.stopPlayer(binding.playerId); delete bindings.value[slotId] } catch (err) { console.error('[PlayerWall] stopSlot failed:', err) }
}

async function stopAll() {
  if (!playerWindow) return
  for (const slotId of Object.keys(bindings.value)) await stopSlot(slotId)
  bindings.value = {}
}

async function deleteLayout() {
  if (!selectedLayoutId.value) return
  try {
    await ElMessageBox.confirm('确定删除此布局?', '确认删除', { type: 'warning' })
    const ok = await layoutStore.deleteLayout(Number(selectedLayoutId.value))
    if (ok) {
      selectedLayoutId.value = ''
      if (playerWindow) { await playerWindow.close(); playerWindow = null }
      bindings.value = {}
      toast.success('布局已删除')
    }
  } catch {}
}
</script>

<style scoped>
.wall-layout { display: flex; gap: 12px; height: calc(100vh - 120px); margin-top: 12px; }
.wall-sidebar { width: 280px; flex-shrink: 0; display: flex; flex-direction: column; gap: 0; }
.wall-player-area { flex: 1; border: 1px dashed var(--border); border-radius: 8px; display: flex; align-items: center; justify-content: center; overflow: hidden; background: var(--bg-surface); }
.device-item { display: flex; align-items: center; gap: 8px; padding: 6px 4px; cursor: grab; border-bottom: 1px solid var(--border); }
.device-item:last-child { border-bottom: none; }
.binding-item { display: flex; align-items: center; gap: 8px; padding: 4px 0; }
</style>
