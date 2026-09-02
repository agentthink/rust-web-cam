<template>
  <div class="page-container">
    <div class="page-body">
      <el-row :gutter="12" align="middle" class="page-toolbar">
        <el-col :span="12">
          <el-select v-model="selectedLayoutId" placeholder="选择布局" style="width: 240px" @change="onLayoutChange">
            <el-option value="" label="-- 选择布局 --" />
            <el-option v-for="l in layoutStore.sortedLayouts" :key="l.id" :label="`${l.name} (${l.rows}×${l.cols})${l.is_default ? ' ★' : ''}`" :value="l.id" />
          </el-select>
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
                default-expand-all
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

        <div class="wall-main-area">
          <div
            ref="playerGrid"
            class="wall-player-area"
            @dragover.prevent
            @drop="onDrop"
          >
            <el-empty v-if="!selectedLayoutId" description="请先选择布局" />
            <el-empty v-else-if="!layout" description="加载中..." />
            <div v-else class="player-grid" :style="gridStyle">
              <div
                v-for="(item, index) in layoutItems"
                :key="item.id"
                class="grid-cell"
                :class="{ selected: selectedSlot === item.id }"
                :style="itemStyle(item)"
                @click="selectedSlot = item.id"
                @dragover.prevent="onCellDragOver($event, item.id)"
                @drop.prevent="onCellDrop($event, item.id)"
              >
                <div v-if="!bindings[item.id]" class="cell-empty">{{ index + 1 }}</div>
                <template v-else>
                  <VideoPlayer
                    :liveSrc="bindings[item.id]?.flvUrl"
                    :isLive="true"
                    :autoplay="true"
                    type="wsflv"
                  />
                  <div class="cell-label">{{ bindings[item.id]?.name }}</div>
                </template>
              </div>
            </div>
          </div>

          <div class="wall-ptz-wrapper">
            <div class="wall-ptz-toggle" @click="ptzPanelVisible = !ptzPanelVisible">
              <el-icon :size="16"><DArrowLeft v-if="ptzPanelVisible" /><DArrowRight v-else /></el-icon>
            </div>
            <div class="wall-ptz-panel" :class="{ collapsed: !ptzPanelVisible }">

              <div class="ptz-hd">
                <div class="ptz-hd-l">
                  <div class="ptz-hd-title">PTZ</div>
                  <div class="ptz-hd-sub">云台控制</div>
                </div>
                <div class="ptz-hd-r">
                  <div class="ptz-hd-signal">
                    <div class="ptz-hd-dot" :class="selectedBinding ? (ptzSupported ? 'sig-ok' : 'sig-no') : 'sig-off'" />
                    <span>{{ selectedBinding ? (ptzSupported ? '在线' : '无云台') : '空闲' }}</span>
                  </div>
                </div>
              </div>

              <div class="ptz-ch">
                <div class="ptz-ch-name">{{ selectedBinding?.name || '— 未选择设备 —' }}</div>
                <div class="ptz-ch-info">
                  <span class="ptz-ch-tag" :class="ptzSupported ? 'ptz-tag-ok' : 'ptz-tag-off'">
                    {{ selectedBinding?.protocol || '---' }}
                  </span>
                  <span class="ptz-ch-spd">
                    <span class="ptz-ch-spd-lbl">速度</span>
                    <span class="ptz-ch-spd-val">{{ ptzSpeed }}</span>
                  </span>
                </div>
              </div>

              <div class="ptz-scan" />

              <div class="ptz-body">

                <div class="ptz-zone ptz-zone-dir">
                  <div class="ptz-crosshair">
                    <div class="ptz-ch-v" />
                    <div class="ptz-ch-h" />
                    <button class="ptz-dir-btn ptz-dir-up"
                      :class="{ on: ptzActiveDir === 'up' }"
                      :disabled="!ptzSupported"
                      @mousedown.prevent="ptzActiveDir = 'up'; ptzCmd('up')"
                      @mouseup="ptzActiveDir = null; ptzStop()"
                      @mouseleave="ptzActiveDir === 'up' && ptzStop()"
                    ><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 19V5M5 12l7-7 7 7"/></svg></button>
                    <button class="ptz-dir-btn ptz-dir-lf"
                      :class="{ on: ptzActiveDir === 'left' }"
                      :disabled="!ptzSupported"
                      @mousedown.prevent="ptzActiveDir = 'left'; ptzCmd('left')"
                      @mouseup="ptzActiveDir = null; ptzStop()"
                      @mouseleave="ptzActiveDir === 'left' && ptzStop()"
                    ><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M19 12H5M12 5l-7 7 7 7"/></svg></button>
                    <button class="ptz-stop-btn"
                      :disabled="!ptzSupported"
                      @click="ptzStop()"
                    ><svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg></button>
                    <button class="ptz-dir-btn ptz-dir-rt"
                      :class="{ on: ptzActiveDir === 'right' }"
                      :disabled="!ptzSupported"
                      @mousedown.prevent="ptzActiveDir = 'right'; ptzCmd('right')"
                      @mouseup="ptzActiveDir = null; ptzStop()"
                      @mouseleave="ptzActiveDir === 'right' && ptzStop()"
                    ><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M5 12h14M12 5l7 7-7 7"/></svg></button>
                    <button class="ptz-dir-btn ptz-dir-dn"
                      :class="{ on: ptzActiveDir === 'down' }"
                      :disabled="!ptzSupported"
                      @mousedown.prevent="ptzActiveDir = 'down'; ptzCmd('down')"
                      @mouseup="ptzActiveDir = null; ptzStop()"
                      @mouseleave="ptzActiveDir === 'down' && ptzStop()"
                    ><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12l7 7 7-7"/></svg></button>
                  </div>
                  <div class="ptz-spd-track">
                    <div class="ptz-spd-fill" :style="{ width: ptzSpeed + '%' }" />
                    <input type="range" min="1" max="100" v-model="ptzSpeed" class="ptz-spd-range" :disabled="!ptzSupported" />
                  </div>
                </div>

                <div class="ptz-sep">
                  <span class="ptz-sep-ln" /><span class="ptz-sep-txt">光学</span><span class="ptz-sep-ln" />
                </div>

                <div class="ptz-zone ptz-zone-opt">
                  <div class="ptz-opt-group">
                    <div class="ptz-opt-label">ZOOM</div>
                    <div class="ptz-opt-row">
                      <button class="ptz-opt-btn"
                        :class="{ on: ptzActiveDir === 'zoom_in' }"
                        :disabled="!ptzSupported"
                        @mousedown.prevent="ptzActiveDir = 'zoom_in'; ptzCmd('zoom_in')"
                        @mouseup="ptzActiveDir = null; ptzStop()"
                        @mouseleave="ptzActiveDir === 'zoom_in' && ptzStop()"
                      ><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35M11 8v6M8 11h6"/></svg></button>
                      <button class="ptz-opt-btn"
                        :class="{ on: ptzActiveDir === 'zoom_out' }"
                        :disabled="!ptzSupported"
                        @mousedown.prevent="ptzActiveDir = 'zoom_out'; ptzCmd('zoom_out')"
                        @mouseup="ptzActiveDir = null; ptzStop()"
                        @mouseleave="ptzActiveDir === 'zoom_out' && ptzStop()"
                      ><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35M8 11h6"/></svg></button>
                    </div>
                  </div>
                  <div class="ptz-opt-group">
                    <div class="ptz-opt-label">FOCUS</div>
                    <div class="ptz-opt-row">
                      <button class="ptz-opt-btn"
                        :class="{ on: ptzActiveDir === 'focus_in' }"
                        :disabled="!ptzSupported"
                        @mousedown.prevent="ptzActiveDir = 'focus_in'; ptzCmd('focus_in')"
                        @mouseup="ptzActiveDir = null; ptzStop()"
                        @mouseleave="ptzActiveDir === 'focus_in' && ptzStop()"
                      ><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="3"/><path d="M3 12h3M18 12h3M12 3v3M12 18v3"/></svg></button>
                      <button class="ptz-opt-btn"
                        :class="{ on: ptzActiveDir === 'focus_out' }"
                        :disabled="!ptzSupported"
                        @mousedown.prevent="ptzActiveDir = 'focus_out'; ptzCmd('focus_out')"
                        @mouseup="ptzActiveDir = null; ptzStop()"
                        @mouseleave="ptzActiveDir === 'focus_out' && ptzStop()"
                      ><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="3"/><path d="M3 12h3M18 12h3M12 3v3M12 18v3"/></svg></button>
                    </div>
                  </div>
                </div>

                <div class="ptz-sep">
                  <span class="ptz-sep-ln" /><span class="ptz-sep-txt">预制位</span><span class="ptz-sep-ln" />
                </div>

                <div class="ptz-zone ptz-zone-ps">
                  <button class="ptz-ps-save"
                    :disabled="!ptzSupported"
                    @click="savePreset()"
                  >+ 保存位置</button>
                  <div class="ptz-ps-list">
                    <div v-if="presets.length === 0" class="ptz-ps-empty">暂无预制位</div>
                    <div
                      v-for="(p, i) in presets" :key="p.token"
                      class="ptz-ps-row"
                      :class="{ disabled: !ptzSupported }"
                      @click="ptzSupported && gotoPreset(p)"
                    >
                      <span class="ptz-ps-idx">{{ String(i + 1).padStart(2, '0') }}</span>
                      <span class="ptz-ps-nm">{{ p.name }}</span>
                      <button class="ptz-ps-del"
                        :disabled="!ptzSupported"
                        @click.stop="removePreset(p)"
                      ><svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 6L6 18M6 6l12 12"/></svg></button>
                    </div>
                  </div>
                </div>

              </div>

              <div class="ptz-ft">
                <span class="ptz-ft-sys">SYS</span>
                <span class="ptz-ft-st" :class="ptzActiveDir ? 'st-run' : 'st-idle'">
                  {{ ptzActiveDir ? { up:'上移中', down:'下移中', left:'左移中', right:'右移中', zoom_in:'放大中', zoom_out:'缩小中', focus_in:'近焦中', focus_out:'远焦中' }[ptzActiveDir] : '— 就绪 —' }}
                </span>
              </div>

            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { Edit, Delete, VideoPlay, Close, ArrowUp, ArrowDown, ArrowLeft as ArrowLeftIcon, ArrowRight, DArrowLeft, DArrowRight } from '@element-plus/icons-vue'
import { ElMessageBox } from 'element-plus'
import VideoPlayer from '../components/VideoPlayer.vue'
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
const selectedSlot = ref(null)
const ptzSpeed = ref(50)
let ptzTimers = {}
const layout = ref(null)
const bindings = ref({})
const slotCounter = ref(0)
const presets = ref([])
let presetsTimer = null

const ptzPanelVisible = ref(false)
const ptzActiveDir = ref(null)
const selectedBinding = computed(() => {
  if (!selectedSlot.value || !bindings.value[selectedSlot.value]) return null
  const binding = bindings.value[selectedSlot.value]
  const device = deviceStore.devices.find(d => d.id === binding.deviceId)
  return device ? { ...binding, protocol: device.protocol } : null
})

const ptzSupported = computed(() => {
  if (!selectedBinding.value) return false
  const proto = (selectedBinding.value.protocol || '').toLowerCase()
  return proto === 'onvif' || proto === 'gb28181'
})

let draggingDeviceId = null
let draggingDevice = null
let draggingFlvUrl = null
const layoutItems = computed(() => {
  if (!layout.value) return []
  const items = layout.value.layout_json || []
  if (items.length === 0) {
    const arr = []
    for (let r = 0; r < layout.value.rows; r++) {
      for (let c = 0; c < layout.value.cols; c++) {
        arr.push({ id: `slot_${slotCounter.value}_${r}_${c}`, row: r, col: c, row_span: 1, col_span: 1 })
      }
    }
    return arr
  }
  return items.map(it => ({
    ...it,
    row_span: it.row_span ?? 1,
    col_span: it.col_span ?? 1,
  }))
})

const gridStyle = computed(() => {
  if (!layout.value) return {}
  return { gridTemplateColumns: `repeat(${layout.value.cols}, 1fr)`, gridTemplateRows: `repeat(${layout.value.rows}, 1fr)` }
})

function itemStyle(item) {
  return {
    gridColumn: `${(item.col ?? 0) + 1} / span ${item.col_span ?? 1}`,
    gridRow: `${(item.row ?? 0) + 1} / span ${item.row_span ?? 1}`,
  }
}

async function getFlvUrl(deviceTag) {
  try {
    const linksRes = await request.get(`/channels/${deviceTag}/${deviceTag}/play-links`)
    const links = linksRes.data.data || {}
    if (links.web_flv) return links.web_flv
    if (links.flv) return links.flv
    const streamsRes = await request.get(`/streams/by-device/${deviceTag}`)
    const streams = streamsRes.data.data || []
    const stream = streams.find(s => s.device_tag === deviceTag && s.state === 'Active')
    if (!stream) return null
    const flvRes = await request.get(`/streams/${stream.id}/play`, { params: { protocol: 'wsflv' } })
    return flvRes.data.data
  } catch { return null }
}

onMounted(async () => {
  await deviceStore.fetchDevices(500, 0)
  await regionStore.fetchRegionTree()
  await groupStore.fetchGroupTree()
  await layoutStore.fetchLayouts()
  if (layoutStore.defaultLayout) { selectedLayoutId.value = layoutStore.defaultLayout.id; layout.value = layoutStore.defaultLayout }
})

onUnmounted(() => { stopAll() })

async function onLayoutChange() {
  stopAll()
  if (!selectedLayoutId.value) { layout.value = null; return }
  layout.value = await layoutStore.fetchLayout(Number(selectedLayoutId.value))
}

async function onDragStart(event, device) {
  const deviceTag = device.device_tag
  draggingDeviceId = deviceTag; draggingDevice = device; draggingFlvUrl = null
  event.dataTransfer.setData('application/json', JSON.stringify({ deviceTag, name: device.name, flvUrl: null }))
  event.dataTransfer.effectAllowed = 'copy'
  try {
    const flvUrl = await getFlvUrl(deviceTag)
    if (flvUrl) {
      draggingFlvUrl = flvUrl
      event.dataTransfer.setData('application/json', JSON.stringify({ deviceTag, name: device.name, flvUrl }))
    }
  } catch (err) { console.warn('[PlayerWallFlv] pre-fetch flv url failed:', err) }
}

function onDragEnd() { draggingDeviceId = null; draggingDevice = null; draggingFlvUrl = null }
function onCellDragOver(event, slotId) { event.dataTransfer.dropEffect = 'copy' }

async function onCellDrop(event, slotId) {
  event.preventDefault()
  const rawData = event.dataTransfer.getData('application/json')
  if (!rawData) return
  let deviceTag, deviceName, flvUrl
  try {
    const data = JSON.parse(rawData)
    deviceTag = data.deviceTag; deviceName = data.name; flvUrl = data.flvUrl
  } catch { toast.error('数据格式错误'); return }
  if (!flvUrl) flvUrl = await getFlvUrl(deviceTag)
  if (!flvUrl) { toast.error('该设备无可用 FLV 播放地址'); return }
  if (bindings.value[slotId]) toast.info('该格子已有播放，替换中...')
  bindings.value[slotId] = { deviceTag, name: deviceName, flvUrl, slotId }
}

async function onDrop(event) {
  event.preventDefault()
  if (!layout.value || layoutItems.value.length === 0) return
  const rawData = event.dataTransfer.getData('application/json')
  if (!rawData) return
  let deviceTag, deviceName, flvUrl
  try {
    const data = JSON.parse(rawData)
    deviceTag = data.deviceTag; deviceName = data.name; flvUrl = data.flvUrl
  } catch { return }
  if (!flvUrl) flvUrl = await getFlvUrl(deviceTag)
  if (!flvUrl) { toast.error('该设备无可用 FLV 播放地址'); return }
  const emptySlot = layoutItems.value.find(item => !bindings.value[item.id])
  const targetSlot = emptySlot || layoutItems.value[0]
  bindings.value[targetSlot.id] = { deviceTag, name: deviceName, flvUrl, slotId: targetSlot.id }
}

function stopSlot(slotId) { delete bindings.value[slotId] }
function stopAll() {
  bindings.value = {}
  selectedSlot.value = null
}

async function ptzCmd(cmd) {
  if (!selectedBinding.value) return
  clearTimeout(ptzTimers[selectedSlot.value])
  try {
    await deviceStore.ptzControl(selectedBinding.value.deviceId, cmd, ptzSpeed.value)
  } catch (e) { toast.error('PTZ控制失败') }
}

function ptzStop() {
  if (!selectedBinding.value) return
  clearTimeout(ptzTimers[selectedSlot.value])
  ptzTimers[selectedSlot.value] = setTimeout(async () => {
    try { await deviceStore.ptzControl(selectedBinding.value.deviceId, 'stop', ptzSpeed.value) } catch {}
  }, 200)
}

async function fetchPresets() {
  if (!selectedBinding.value) return
  const proto = selectedBinding.value.protocol?.toLowerCase()
  if (proto !== 'onvif' && proto !== 'gb28181') { presets.value = []; return }
  try {
    const res = await request.get(`/devices/${selectedBinding.value.deviceId}/ptz/presets`)
    presets.value = res.data.data || []
  } catch { presets.value = [] }
}

async function savePreset() {
  if (!selectedBinding.value) return
  const name = `预置 ${Date.now()}`
  try {
    await request.post(`/devices/${selectedBinding.value.deviceId}/ptz/presets`, { name })
    toast.success('预制位已保存')
    await fetchPresets()
  } catch (e) { toast.error('保存失败') }
}

async function gotoPreset(preset) {
  if (!selectedBinding.value) return
  try {
    await deviceStore.ptzControl(selectedBinding.value.deviceId, 'goto_preset', ptzSpeed.value, preset.token)
  } catch (e) { toast.error('调用预制位失败') }
}

async function removePreset(preset) {
  if (!selectedBinding.value) return
  try {
    await request.delete(`/devices/${selectedBinding.value.deviceId}/ptz/presets/${preset.token}`)
    toast.success('已删除')
    await fetchPresets()
  } catch (e) { toast.error('删除失败') }
}

watch(selectedSlot, async (slotId) => {
  if (slotId && bindings.value[slotId]) {
    ptzPanelVisible.value = true
    if (ptzSupported.value) {
      await fetchPresets()
    } else {
      presets.value = []
    }
  } else {
    ptzPanelVisible.value = false
    presets.value = []
  }
})

async function deleteLayout() {
  if (!selectedLayoutId.value) return
  try {
    await ElMessageBox.confirm('确定删除此布局?', '确认删除', { type: 'warning' })
    const ok = await layoutStore.deleteLayout(Number(selectedLayoutId.value))
    if (ok) { selectedLayoutId.value = ''; layout.value = null; bindings.value = {}; toast.success('布局已删除') }
  } catch {}
}
</script>

<style scoped>
.wall-layout { display: flex; gap: 12px; height: calc(100vh - 120px); margin-top: 12px; }
.wall-sidebar { width: 280px; flex-shrink: 0; display: flex; flex-direction: column; gap: 0; }
.wall-main-area { flex: 1; display: flex; gap: 8px; min-width: 0; padding-left: 20px; }
.wall-player-area { flex: 1; border: 1px dashed var(--border); border-radius: 8px; display: flex; align-items: center; justify-content: center; overflow: hidden; background: var(--bg-surface); padding: 8px; }
.wall-ptz-wrapper { position: relative; flex-shrink: 0; display: flex; align-items: stretch; }
.wall-ptz-panel {
  width: 200px; flex-shrink: 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  display: flex; flex-direction: column; overflow: hidden;
  transition: width 0.25s cubic-bezier(.4,0,.2,1), opacity 0.25s;
  font-family: 'Courier New', 'Lucida Console', monospace;
  box-shadow: 0 4px 24px rgba(0,0,0,.12);
}
.wall-ptz-panel.collapsed { width: 0; border: none; opacity: 0; pointer-events: none; }
.wall-ptz-toggle {
  position: absolute; left: -20px; top: 50%; transform: translateY(-50%);
  width: 20px; height: 48px;
  display: flex; align-items: center; justify-content: center;
  cursor: pointer;
  color: var(--ptz-text-dim); background: var(--bg-card);
  border: 1px solid var(--border); border-right: none;
  border-radius: 6px 0 0 6px;
  transition: color 0.2s, background 0.2s;
  z-index: 10;
}
.wall-ptz-toggle:hover { color: var(--color-accent); }

.ptz-hd {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 12px 8px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-elevated);
}
.ptz-hd-l {}
.ptz-hd-title {
  font-size: 18px; font-weight: 900; letter-spacing: 4px;
  color: var(--color-accent);
  text-shadow: 0 0 14px var(--color-accent-glow);
  line-height: 1;
}
.ptz-hd-sub {
  font-size: 8px; letter-spacing: 2px; color: var(--ptz-text-dim);
  margin-top: 2px;
}
.ptz-hd-r {}
.ptz-hd-signal {
  display: flex; align-items: center; gap: 5px;
  font-size: 8px; letter-spacing: 1px; color: var(--ptz-text-dim);
}
.ptz-hd-dot {
  width: 7px; height: 7px; border-radius: 50%;
  background: var(--ptz-text-dim);
  transition: background 0.3s, box-shadow 0.3s;
}
.ptz-hd-dot.sig-ok {
  background: var(--color-accent);
  box-shadow: 0 0 6px var(--color-accent-glow);
  animation: ptz-pulse 2s infinite;
}
.ptz-hd-dot.sig-no { background: var(--el-color-warning); }
.ptz-hd-dot.sig-off { background: var(--ptz-text-dim); }
@keyframes ptz-pulse {
  0%,100% { opacity: 1; }
  50% { opacity: .4; }
}

.ptz-ch {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--color-accent-dim);
}
.ptz-ch-name {
  font-size: 11px; color: var(--text-primary); letter-spacing: .5px;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  margin-bottom: 4px;
}
.ptz-ch-info { display: flex; align-items: center; gap: 6px; }
.ptz-ch-tag {
  font-size: 8px; letter-spacing: 1px; padding: 1px 5px;
  border-radius: 2px; border: 1px solid;
}
.ptz-tag-ok { color: var(--color-accent); border-color: var(--color-accent); background: var(--color-accent-dim); }
.ptz-tag-off { color: var(--ptz-text-dim); border-color: var(--border); background: transparent; }
.ptz-ch-spd { display: flex; align-items: center; gap: 3px; margin-left: auto; }
.ptz-ch-spd-lbl { font-size: 8px; color: var(--ptz-text-dim); letter-spacing: 1px; }
.ptz-ch-spd-val { font-size: 10px; color: var(--color-accent); font-weight: bold; letter-spacing: 1px; }

.ptz-scan {
  height: 2px; background: linear-gradient(90deg, transparent, var(--color-accent-dim), transparent);
  animation: ptz-scan 3s linear infinite;
}
@keyframes ptz-scan {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(300%); }
}

.ptz-body { flex: 1; overflow-y: auto; padding: 10px 12px; }

.ptz-zone { margin-bottom: 10px; }

.ptz-crosshair {
  position: relative; width: 120px; height: 120px;
  margin: 0 auto 10px;
  display: grid; grid-template-columns: repeat(3, 40px);
  grid-template-rows: repeat(3, 40px); gap: 0;
}
.ptz-ch-v, .ptz-ch-h {
  position: absolute; top: 50%; left: 0; right: 0; height: 1px;
  background: var(--border); pointer-events: none;
}
.ptz-ch-h { transform: rotate(90deg) translateX(-50%); width: 120px; left: 0; }

.ptz-dir-btn {
  width: 38px; height: 38px;
  border: 1px solid var(--ptz-btn-border); background: var(--ptz-btn-bg);
  border-radius: 6px; color: var(--ptz-text);
  display: flex; align-items: center; justify-content: center;
  cursor: pointer; transition: all 0.1s;
  outline: none; padding: 0;
}
.ptz-dir-btn:hover:not(:disabled) {
  border-color: var(--color-accent); color: var(--color-accent);
  background: var(--color-accent-dim);
}
.ptz-dir-btn.on {
  border-color: var(--color-accent); color: var(--color-accent);
  background: var(--color-accent-dim);
  box-shadow: 0 0 12px var(--color-accent-glow), inset 0 0 8px var(--color-accent-dim);
}
.ptz-dir-btn:disabled { opacity: .3; cursor: not-allowed; }
.ptz-dir-up    { grid-column: 2; grid-row: 1; justify-self: center; }
.ptz-dir-lf    { grid-column: 1; grid-row: 2; justify-self: center; }
.ptz-stop-btn  {
  grid-column: 2; grid-row: 2; justify-self: center;
  width: 34px; height: 34px;
  border: 1px solid rgba(239, 68, 68, 0.25); background: rgba(239, 68, 68, 0.06);
  border-radius: 50%; color: var(--el-color-danger);
  display: flex; align-items: center; justify-content: center;
  cursor: pointer; transition: all 0.1s; outline: none; padding: 0;
}
.ptz-stop-btn:hover:not(:disabled) {
  border-color: var(--el-color-danger); background: rgba(239, 68, 68, 0.1);
  box-shadow: 0 0 10px rgba(239, 68, 68, 0.2);
}
.ptz-stop-btn:disabled { opacity: .3; cursor: not-allowed; }
.ptz-dir-rt    { grid-column: 3; grid-row: 2; justify-self: center; }
.ptz-dir-dn    { grid-column: 2; grid-row: 3; justify-self: center; }

.ptz-spd-track {
  position: relative; height: 4px;
  background: var(--ptz-btn-bg); border-radius: 2px;
  border: 1px solid var(--ptz-border); overflow: visible;
}
.ptz-spd-fill {
  position: absolute; left: 0; top: 0; bottom: 0;
  background: linear-gradient(90deg, var(--color-accent), var(--color-accent));
  border-radius: 2px;
  box-shadow: 0 0 6px var(--color-accent-glow);
  transition: width 0.1s;
}
.ptz-spd-range {
  position: absolute; inset: -6px 0;
  width: 100%; height: 16px; opacity: 0; cursor: pointer;
  -webkit-appearance: none;
}
.ptz-spd-range:disabled { cursor: not-allowed; }

.ptz-sep { display: flex; align-items: center; gap: 6px; margin: 10px 0; }
.ptz-sep-ln { flex: 1; height: 1px; background: var(--ptz-divider); }
.ptz-sep-txt { font-size: 8px; letter-spacing: 2px; color: var(--ptz-text-dim); }

.ptz-zone-opt { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.ptz-opt-group {}
.ptz-opt-label { font-size: 8px; letter-spacing: 1.5px; color: var(--ptz-text-dim); margin-bottom: 4px; text-align: center; }
.ptz-opt-row { display: flex; gap: 4px; }
.ptz-opt-btn {
  flex: 1; height: 30px;
  border: 1px solid var(--ptz-btn-border); background: var(--ptz-btn-bg);
  border-radius: 4px; color: var(--ptz-text-dim);
  display: flex; align-items: center; justify-content: center; gap: 3px;
  cursor: pointer; transition: all 0.1s; font-size: 9px; letter-spacing: .5px;
  outline: none; padding: 0;
}
.ptz-opt-btn:hover:not(:disabled) {
  border-color: var(--color-accent); color: var(--color-accent);
  background: var(--color-accent-dim);
}
.ptz-opt-btn.on {
  border-color: var(--color-accent); color: var(--color-accent);
  background: var(--color-accent-dim);
  box-shadow: 0 0 8px var(--color-accent-glow);
}
.ptz-opt-btn:disabled { opacity: .3; cursor: not-allowed; }

.ptz-zone-ps {}
.ptz-ps-save {
  width: 100%; height: 26px; margin-bottom: 6px;
  border: 1px dashed var(--ptz-btn-border); background: transparent;
  border-radius: 4px; color: var(--ptz-text-dim);
  font-size: 9px; letter-spacing: 1px;
  cursor: pointer; transition: all 0.15s; outline: none; padding: 0;
  font-family: inherit;
}
.ptz-ps-save:hover:not(:disabled) {
  border-color: var(--color-accent); color: var(--color-accent);
  background: var(--color-accent-dim);
}
.ptz-ps-save:disabled { opacity: .3; cursor: not-allowed; }
.ptz-ps-list { max-height: 110px; overflow-y: auto; }
.ptz-ps-empty { font-size: 8px; letter-spacing: 1.5px; color: var(--ptz-text-dim); text-align: center; padding: 10px 0; }
.ptz-ps-row {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 5px; border-radius: 3px;
  cursor: pointer; transition: background 0.1s;
  border-bottom: 1px solid var(--border);
}
.ptz-ps-row:last-child { border-bottom: none; }
.ptz-ps-row:hover:not(.disabled) { background: var(--color-accent-dim); }
.ptz-ps-row.disabled { opacity: .4; cursor: default; }
.ptz-ps-idx { font-size: 8px; color: var(--ptz-text-dim); letter-spacing: 1px; min-width: 16px; }
.ptz-ps-nm { flex: 1; font-size: 10px; color: var(--ptz-text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; letter-spacing: .3px; }
.ptz-ps-del {
  width: 18px; height: 18px; flex-shrink: 0;
  border: none; background: transparent; color: var(--ptz-text-dim);
  display: flex; align-items: center; justify-content: center;
  cursor: pointer; border-radius: 2px; transition: color 0.1s; padding: 0;
}
.ptz-ps-del:hover:not(:disabled) { color: var(--el-color-danger); }
.ptz-ps-del:disabled { opacity: .3; cursor: not-allowed; }

.ptz-ft {
  display: flex; align-items: center; gap: 6px;
  padding: 7px 12px;
  border-top: 1px solid var(--border);
  background: var(--bg-elevated);
}
.ptz-ft-sys { font-size: 7px; letter-spacing: 2px; color: var(--ptz-text-dim); }
.ptz-ft-st { font-size: 9px; letter-spacing: 1px; transition: color 0.15s; }
.st-run { color: var(--color-accent); text-shadow: 0 0 8px var(--color-accent-glow); }
.st-idle { color: var(--ptz-text-dim); }

.player-grid { display: grid; gap: 4px; width: 100%; height: 100%; }
.grid-cell { border: 1px dashed var(--border); border-radius: 4px; overflow: hidden; position: relative; background: var(--video-bg); display: flex; align-items: center; justify-content: center; cursor: pointer; }
.grid-cell.selected { border-color: var(--color-accent); border-style: solid; box-shadow: 0 0 10px var(--color-accent-glow); }
.cell-empty { font-size: 24px; color: var(--text-muted); font-weight: 700; }
.cell-label { position: absolute; bottom: 0; left: 0; right: 0; background: rgba(0,0,0,.5); color: #fff; font-size: 11px; padding: 2px 6px; text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.device-item { display: flex; align-items: center; gap: 8px; padding: 6px 4px; cursor: grab; border-bottom: 1px solid var(--border); }
.binding-item { display: flex; align-items: center; gap: 8px; padding: 4px 0; }
</style>
