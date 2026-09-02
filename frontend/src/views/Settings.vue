<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">设置</h1>
      <div class="page-toolbar">
        <el-button type="primary" :loading="saving" @click="saveServers">保存配置</el-button>
      </div>
    </div>
    <div class="page-body">
      <el-row :gutter="12">
        <el-col :span="12">
          <DataCard>
            <template #header><span>视频墙播放器</span></template>
            <el-form-item label="播放器类型">
              <el-select v-model="playerType" style="width: 100%" @change="savePlayerType">
                <el-option value="rtsp" label="RTSP (rtsp-player, 低延迟)" />
                <el-option value="flv" label="FLV/WebSocket (flv.js)" />
              </el-select>
            </el-form-item>
            <el-text type="info" size="small">
              RTSP: 通过 WebAssembly 解码，低延迟但需要媒体服务器支持 RTSP-over-WS<br/>
              FLV: 通过 WebSocket 传输 FLV 流，兼容性好
            </el-text>
          </DataCard>

          <DataCard>
            <template #header><span>服务器配置</span></template>
            <el-text type="info" size="small" style="margin-bottom: 12px; display: block">从配置文件加载，无法在此页面修改</el-text>
            <el-descriptions :column="1" border size="small">
              <el-descriptions-item label="主机地址">{{ config.server.host }}</el-descriptions-item>
              <el-descriptions-item label="端口">{{ config.server.port }}</el-descriptions-item>
              <el-descriptions-item label="PostgreSQL URL"><code class="mono">{{ config.database.url }}</code></el-descriptions-item>
              <el-descriptions-item label="Redis URL"><code class="mono">{{ config.redis.url }}</code></el-descriptions-item>
            </el-descriptions>
          </DataCard>
        </el-col>

        <el-col :span="12">
          <DataCard>
            <template #header>
              <div style="display: flex; align-items: center; justify-content: space-between">
                <span>媒体服务器</span>
                <el-button size="small" :icon="Plus" @click="addServer">添加服务器</el-button>
              </div>
            </template>
            <el-skeleton animated :loading="loading" :rows="3">
              <template #default>
                <div v-for="(server, index) in config.media_servers.servers" :key="index" style="margin-bottom: 16px; padding-bottom: 16px; border-bottom: 1px solid var(--border)">
                  <div style="display: flex; justify-content: flex-end; margin-bottom: 8px">
                    <el-button size="small" type="danger" plain :icon="Delete" @click="deleteServer(index)">删除</el-button>
                  </div>
                  <el-form label-position="top" size="small">
                    <el-row :gutter="8">
                      <el-col :span="12">
                        <el-form-item label="名称"><el-input v-model="server.name" /></el-form-item>
                      </el-col>
                      <el-col :span="12">
                        <el-form-item label="类型">
                          <el-select v-model="server.server_type" style="width: 100%">
                            <el-option value="zlmediakit" label="ZLMediaKit" />
                            <el-option value="srs" label="SRS" />
                            <el-option value="xiu" label="Xiu" />
                          </el-select>
                        </el-form-item>
                      </el-col>
                    </el-row>
                    <el-form-item label="API URL"><el-input v-model="server.url" placeholder="http://localhost:8090" /></el-form-item>
                    <el-form-item label="API Key"><el-input v-model="server.api_key" type="password" show-password /></el-form-item>
                  </el-form>
                </div>
                <el-empty v-if="config.media_servers.servers.length === 0" description="暂无媒体服务器" :image-size="60" />
              </template>
            </el-skeleton>
          </DataCard>
        </el-col>
      </el-row>

      <el-affix position="bottom" :offset="20" v-if="successMsg">
        <el-alert :title="successMsg" type="success" :closable="false" show-icon />
      </el-affix>
      <el-affix position="bottom" :offset="20" v-if="errorMsg">
        <el-alert :title="errorMsg" type="error" :closable="false" show-icon />
      </el-affix>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { Plus, Delete } from '@element-plus/icons-vue'
import { useSettingsStore } from '../stores/settingsStore'
import { authFetch } from '../utils/authFetch'
import DataCard from '../components/common/DataCard.vue'

const settingsStore = useSettingsStore()
const playerType = ref(settingsStore.playerType)

function savePlayerType() { settingsStore.setPlayerType(playerType.value) }

const config = ref({
  server: { host: '0.0.0.0', port: 8080 },
  database: { url: 'postgres://postgres:postgres@localhost:5432/rustcam' },
  redis: { url: 'redis://localhost:6379' },
  media_servers: { servers: [] }
})

const loading = ref(false)
const saving = ref(false)
const successMsg = ref('')
const errorMsg = ref('')

onMounted(async () => { await fetchServers() })

async function fetchServers() {
  loading.value = true
  try {
    const res = await authFetch('/api/v1/servers')
    const data = await res.json()
    if (data.code === 200) {
      config.value.media_servers.servers = data.data.map(s => ({
        id: s.id, name: s.name, url: s.url, api_key: s.api_key, server_type: s.server_type, weight: s.weight, enabled: s.enabled
      }))
    }
  } catch (e) { console.error('Failed to fetch servers:', e); errorMsg.value = '加载媒体服务器失败'; setTimeout(() => { errorMsg.value = '' }, 5000) }
  finally { loading.value = false }
}

function addServer() { config.value.media_servers.servers.push({ name: '', url: '', api_key: '', server_type: 'zlmediakit', weight: 100, enabled: true }) }
function deleteServer(index) { config.value.media_servers.servers.splice(index, 1) }

async function saveServers() {
  saving.value = true; errorMsg.value = ''
  try {
    for (const server of config.value.media_servers.servers) {
      if (server.id) await authFetch(`/api/v1/servers/${server.id}`, { method: 'PUT', body: JSON.stringify(server) })
      else await authFetch('/api/v1/servers', { method: 'POST', body: JSON.stringify(server) })
    }
    successMsg.value = '保存成功'; setTimeout(() => { successMsg.value = '' }, 3000)
    await fetchServers()
  } catch (e) { console.error('Failed to save servers:', e); errorMsg.value = '保存媒体服务器失败'; setTimeout(() => { errorMsg.value = '' }, 5000) }
  finally { saving.value = false }
}
</script>

<style scoped>
.mono { font-family: var(--font-mono); }
</style>
