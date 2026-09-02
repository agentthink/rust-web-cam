<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">媒体服务器管理</h1>
      <div class="page-toolbar">
        <el-button type="default" :loading="refreshingAll" @click="refreshAll">
          <el-icon><Refresh /></el-icon> 刷新全部
        </el-button>
        <el-button type="primary" @click="openAddModal">
          <el-icon><Plus /></el-icon> 添加服务器
        </el-button>
      </div>
    </div>
    <div class="page-body">
      <el-empty v-if="servers.length === 0 && !loading" description="暂无媒体服务器，点击上方按钮添加" />

      <div v-else class="server-grid">
        <div v-for="server in servers" :key="server.server_tag" class="server-card" :class="{ selected: selectedServer?.server_tag === server.server_tag }" @click="selectServer(server)">
          <div class="server-card__header">
            <StatusDot :status="server.online ? 'online' : 'offline'" />
            <span>媒体服务器</span>
            

            <el-tag size="small" :type="server.online ? 'success' : 'info'">
              {{ server.online ? '在线' : '离线' }}
            </el-tag>
            <div class="server-card__actions">
              <el-button text size="small" @click.stop="checkStatus(server)" :loading="checkingStatus.has(server.server_tag)" title="检测">
                <el-icon><Refresh /></el-icon>
              </el-button>
              <el-button text size="small" :type="server.enabled ? 'danger' : 'success'" @click.stop="toggleServer(server)" :title="server.enabled ? '禁用' : '启用'">
                {{ server.enabled ? '禁用' : '启用' }}
              </el-button>
              <el-button text size="small" @click.stop="openEditModal(server)" title="编辑">
                <el-icon><Edit /></el-icon>
              </el-button>
              <el-button text size="small" type="danger" @click.stop="confirmDelete(server)" title="删除">
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>
          <div class="server-card__body">
            <div class="metric-row">
              <span class="metric-row__label">名称</span>
              <span class="metric-row__value">{{ server.name }}</span>
            </div>
            <div class="metric-row">
              <span class="metric-row__label">Tag</span>
              <span class="metric-row__value">{{ server.server_tag }}</span>
            </div>
            <div class="metric-row">
              <span class="metric-row__label">类型</span>
              <span class="metric-row__value">{{ server.server_type?.toUpperCase() }}</span>
            </div>
            <div class="metric-row">
              <span class="metric-row__label">会话</span>
              <span class="metric-row__value">{{ server.session_count || 0 }}</span>
            </div>
            <div class="metric-row">
              <span class="metric-row__label">权重</span>
              <span class="metric-row__value">{{ server.weight }}</span>
            </div>
            <div class="metric-row">
              <span class="metric-row__label">CPU</span>
              <el-progress :percentage="Math.round(server.cpu_usage || 0)" :color="cpuColor(server.cpu_usage)" size="small" style="width: 120px" />
            </div>
            <div class="metric-row">
              <span class="metric-row__label">内存</span>
              <el-progress :percentage="Math.round((server.memory_usage || 0) / 10)" :color="cpuColor(server.memory_usage / 10)" size="small" style="width: 120px" />
            </div>
            <div class="metric-row">
              <span class="metric-row__label">带宽入</span>
              <span class="metric-row__value">{{ formatBandwidth(server.bandwidth_in) }}</span>
            </div>
            <div class="metric-row">
              <span class="metric-row__label">带宽出</span>
              <span class="metric-row__value">{{ formatBandwidth(server.bandwidth_out) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <el-dialog v-model="showModal" :title="isEdit ? '编辑服务器' : '添加服务器'" width="640px" draggable>
      <el-form :model="form" label-width="100px">
        <el-form-item label="服务器名称" required>
          <el-input v-model="form.name" placeholder="例如: ZLM-Beijing-01" />
        </el-form-item>
        <el-form-item label="服务器类型" required>
          <el-radio-group v-model="form.server_type">
            <el-radio-button v-for="type in serverTypes" :key="type.value" :value="type.value">
              {{ type.label }} — {{ type.desc }}
            </el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="服务器地址" required>
          <el-input v-model="form.url" :placeholder="serverTypeHint" />
          <div style="margin-top: 4px; font-size: 12px; color: var(--el-text-color-secondary);">{{ serverTypeHint }}</div>
        </el-form-item>
        <el-form-item label="API 密钥" required>
          <el-input v-model="form.api_key" :placeholder="apiKeyPlaceholder" show-password />
          <div style="margin-top: 4px; font-size: 12px; color: var(--el-text-color-secondary);">{{ apiKeyHint }}</div>
        </el-form-item>
        <el-form-item label="服务器标签" required>
          <el-input v-model="form.server_tag" placeholder="如: BJ / SH / GZ" maxlength="20" />
          <div style="margin-top: 4px; font-size: 12px; color: var(--el-text-color-secondary);">字母组合，用于唯一标识服务器</div>
        </el-form-item>
        <el-form-item label="协议端口">
          <el-table :data="portRows" size="small" border style="width: 100%;">
            <el-table-column prop="label" label="协议" width="100" />
            <el-table-column label="端口" min-width="200">
              <template #default="{ row }">
                <el-input-number
                  v-model="row.port"
                  :min="1"
                  :max="65535"
                  placeholder="留空表示使用默认端口"
                  controls-position="right"
                  style="width: 100%;"
                />
              </template>
            </el-table-column>
            <el-table-column label="" width="80" align="center">
              <template #default="{ row }">
                <el-button link type="primary" @click="row.port = null" v-if="row.port !== null">
                  <el-icon><Close /></el-icon>
                </el-button>
              </template>
            </el-table-column>
          </el-table>
          <div style="margin-top: 6px; font-size: 12px; color: var(--el-text-color-secondary);">留空表示不指定端口，使用服务器默认值</div>
        </el-form-item>
        <el-form-item label="权重">
          <el-input-number v-model="form.weight" :min="1" :max="1000" />
          <div style="margin-top: 4px; font-size: 12px; color: var(--el-text-color-secondary);">权重越高越优先分配请求 (1-1000)</div>
        </el-form-item>
        <el-form-item>
          <el-switch v-model="form.enabled" active-text="启用服务器" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="closeModal">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitForm">
          {{ isEdit ? '保存修改' : '确认添加' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useMediaServerStore } from '../stores/mediaServerStore'
import { Refresh, Plus, Edit, Delete, Close } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import StatusDot from '../components/common/StatusDot.vue'

const store = useMediaServerStore()
const servers = computed(() => store.servers)
const loading = computed(() => store.loading)
const checkingStatus = computed(() => store.checkingStatus)

const showModal = ref(false)
const isEdit = ref(false)
const submitting = ref(false)
const selectedServer = ref(null)
const refreshingAll = ref(false)
const editingTag = ref(null)

const form = ref({
  name: '', url: '', api_key: '', server_type: 'zlmediakit', weight: 100, enabled: true, server_tag: ''
})

const serverTypes = [
  { value: 'zlmediakit', label: 'ZLMediaKit', desc: '国标/RTSP' },
  { value: 'srs', label: 'SRS', desc: '直播/WebRTC' },
  { value: 'xiu', label: 'Xiu', desc: '轻量/RTSP' }
]

const portRows = ref([
  { key: 'rtsp', label: 'RTSP', port: null },
  { key: 'rtmp', label: 'RTMP', port: null },
  { key: 'hls', label: 'HLS', port: null },
  { key: 'http', label: 'HTTP', port: null },
  { key: 'https', label: 'HTTPS', port: null },
  { key: 'webrtc', label: 'WebRTC', port: null },
  { key: 'rtp_tcp', label: 'RTP/TCP', port: null },
  { key: 'http_flv', label: 'HTTP-FLV', port: null },
  { key: 'ws_flv', label: 'WS-FLV', port: null },
])

const serverTypeHint = computed(() => ({
  zlmediakit: 'http://192.168.1.100:8080',
  srs: 'http://192.168.1.100:1985',
  xiu: 'http://192.168.1.100:8000'
})[form.value.server_type])

const apiKeyPlaceholder = computed(() => ({
  zlmediakit: 'ZLMediaKit 密钥 (默认: 035c73f7-bb6b-4889-a715-d9eb2d1925cc)',
  srs: 'SRS HTTP API 密钥',
  xiu: 'Xiu Bearer Token'
})[form.value.server_type])

const apiKeyHint = computed(() => ({
  zlmediakit: '控制台 → HTTP API → 配置文件中的 secret',
  srs: 'srs.conf 中的 api_auth_secret',
  xiu: '启动参数中的 --token 或默认无认证'
})[form.value.server_type])

function cpuColor(val) {
  if (val >= 80) return '#ef4444'
  if (val >= 50) return '#f59e0b'
  return '#22c55e'
}

function formatBandwidth(bps) {
  if (!bps) return '-'
  if (bps >= 1e9) return `${(bps / 1e9).toFixed(1)} Gbps`
  if (bps >= 1e6) return `${(bps / 1e6).toFixed(1)} Mbps`
  if (bps >= 1e3) return `${(bps / 1e3).toFixed(1)} Kbps`
  return `${bps} bps`
}

function selectServer(server) {
  selectedServer.value = selectedServer.value?.server_tag === server.server_tag ? null : server
}

function buildPortsPayload() {
  const ports = {}
  for (const row of portRows.value) {
    ports[row.key] = row.port || null
  }
  const hasValue = Object.values(ports).some(v => v !== null)
  return hasValue ? ports : null
}

function loadPortsFromServer(server) {
  const p = server.protocol_ports || {}
  for (const row of portRows.value) {
    row.port = p[row.key] || null
  }
}

function openAddModal() {
  isEdit.value = false; editingTag.value = null
  form.value = { name: '', url: '', api_key: '', server_type: 'zlmediakit', weight: 100, enabled: true, server_tag: '' }
  for (const row of portRows.value) { row.port = null }
  showModal.value = true
}

function openEditModal(server) {
  isEdit.value = true; editingTag.value = server.server_tag
  form.value = { name: server.name, url: server.url, api_key: server.api_key || '', server_type: server.server_type, weight: server.weight, enabled: server.enabled, server_tag: server.server_tag }
  loadPortsFromServer(server)
  showModal.value = true
}

function closeModal() { showModal.value = false }

async function submitForm() {
  if (!form.value.server_tag.trim()) {
    ElMessage.error('服务器标签不能为空')
    return
  }
  submitting.value = true
  try {
    const payload = {
      name: form.value.name,
      url: form.value.url,
      api_key: form.value.api_key,
      server_type: form.value.server_type,
      weight: form.value.weight,
      enabled: form.value.enabled,
      server_tag: form.value.server_tag.trim(),
      protocol_ports: buildPortsPayload(),
    }
    if (isEdit.value) {
      await store.updateServer(editingTag.value, payload)
    } else {
      await store.createServer(payload)
    }
    closeModal()
  } catch (e) {
    console.error(e)
    ElMessage.error(e.message || '操作失败')
  } finally {
    submitting.value = false
  }
}

async function checkStatus(server) { await store.checkServerStatus(server.server_tag) }
async function toggleServer(server) {
  try {
    if (server.enabled) await store.disableServer(server.server_tag)
    else await store.enableServer(server.server_tag)
  } catch (e) { console.error(e) }
}
async function refreshAll() { refreshingAll.value = true; await store.checkAllStatus(); refreshingAll.value = false }
async function confirmDelete(server) {
  try {
    await store.deleteServer(server.server_tag)
    if (selectedServer.value?.server_tag === server.server_tag) selectedServer.value = null
  } catch (e) { console.error(e) }
}

let refreshInterval = null
onMounted(async () => { await store.fetchServers(); refreshInterval = setInterval(() => { if (servers.value.length > 0) store.checkAllStatus() }, 30000) })
onUnmounted(() => { if (refreshInterval) clearInterval(refreshInterval) })
</script>

<style scoped>
.server-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
  gap: var(--space-4);
}
.server-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-base);
  overflow: hidden;
  transition: border-color 0.15s, box-shadow 0.15s;
  cursor: pointer;
}
.server-card:hover {
  border-color: var(--border-accent);
  box-shadow: var(--shadow-accent);
}
.server-card.selected {
  border-color: var(--color-accent);
}
.server-card__header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  background: var(--bg-elevated);
  min-width: 0;
}
.server-name {
  flex: 1;
  font-weight: var(--weight-semibold);
  font-size: var(--text-base);
  color: var(--text-primary);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.server-card__actions {
  display: flex;
  gap: 2px;
  margin-left: auto;
  flex-shrink: 0;
}
.server-card__body {
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.metric-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}
.metric-row__label {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  font-family: var(--font-cn);
}
.metric-row__value {
  font-family: var(--font-mono);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
}
</style>
