<template>
  <div class="page-container">
    <div class="page-header">
      <div style="display: flex; align-items: center; gap: 12px;">
        <el-button v-if="deviceTagFilter" text @click="clearDeviceFilter" style="margin-right: 8px;">
          <el-icon><ArrowLeft /></el-icon> 返回设备列表
        </el-button>
        <h1 class="page-title">{{ deviceTagFilter ? `设备通道: ${deviceTagFilter}` : '通道管理' }}</h1>
      </div>
      <div class="page-toolbar">
        <el-input v-model="searchQuery" placeholder="搜索通道..." clearable style="max-width: 200px">
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-select v-model="statusFilter" clearable placeholder="全部状态" style="width: 110px">
          <el-option value="Online" label="在线" />
          <el-option value="Offline" label="离线" />
          <el-option value="Maintaining" label="维护中" />
          <el-option value="Error" label="错误" />
        </el-select>
        <el-button :icon="Refresh" @click="fetchChannels">刷新</el-button>
      </div>
    </div>

    <div class="page-body">
      <el-text type="info" style="margin-bottom: 12px; display: block">共 {{ channelStore.total }} 个通道</el-text>
      <div class="data-card">
        <el-skeleton animated :rows="5" :loading="channelStore.loading">
          <el-table :data="filteredChannels" row-key="id" empty-text="暂无通道">
            <el-table-column label="状态" width="70" align="center">
              <template #default="{ row }">
                <StatusDot :status="getChannelStatusClass(row.status)" />
              </template>
            </el-table-column>
            <el-table-column label="通道名称" min-width="180">
              <template #default="{ row }">
                <el-link type="primary" @click="$router.push(`/channels/${row.device_tag}/${row.channel_tag}`)">{{ row.name }}</el-link>
              </template>
            </el-table-column>
            <el-table-column label="设备ID" min-width="180">
              <template #default="{ row }">
                <code class="stream-key" style="font-size: 11px">{{ row.device_tag }}</code>
              </template>
            </el-table-column>
            <el-table-column label="通道ID" min-width="180">
              <template #default="{ row }">
                <code class="stream-key" style="font-size: 11px">{{ row.channel_tag }}</code>
              </template>
            </el-table-column>
            <el-table-column label="类型" width="100" align="center">
              <template #default="{ row }">
                <el-tag size="small" type="primary">{{ row.device_type || 'IPC' }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="地址" min-width="140">
              <template #default="{ row }">
                <span v-if="row.ip_address">{{ row.ip_address }}:{{ row.port }}</span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column label="厂商/型号" min-width="140">
              <template #default="{ row }">
                <span>{{ [row.manufacturer, row.model].filter(Boolean).join(' ') || '-' }}</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="180" align="center">
              <template #default="{ row }">
                <el-button size="small" type="primary" @click="$router.push(`/channels/${row.device_tag}/${row.channel_tag}`)">详情</el-button>
                <el-button size="small" @click="startChannelStream(row)">播放</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-skeleton>

        <el-pagination
          v-if="!channelStore.loading"
          v-model:current-page="currentPage"
          :page-size="channelStore.limit"
          :total="channelStore.total"
          layout="prev, pager, next"
          style="margin-top: 12px; justify-content: flex-end"
          @current-change="onPageChange"
        />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Search, Refresh, ArrowLeft } from '@element-plus/icons-vue'
import { useChannelStore } from '../stores/channelStore'
import { useStreamStore } from '../stores/streamStore'
import { ElMessage } from 'element-plus'
import StatusDot from '../components/common/StatusDot.vue'

const route = useRoute()
const router = useRouter()
const channelStore = useChannelStore()
const streamStore = useStreamStore()

const searchQuery = ref('')
const statusFilter = ref('')
const currentPage = ref(1)

const deviceTagFilter = computed(() => route.query.device_tag || '')

const filteredChannels = computed(() => {
  return channelStore.channels.filter(channel => {
    const matchesSearch = !searchQuery.value || 
      channel.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      channel.device_tag.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      channel.channel_tag.toLowerCase().includes(searchQuery.value.toLowerCase())
    const matchesStatus = !statusFilter.value || channel.status === statusFilter.value
    return matchesSearch && matchesStatus
  })
})

function getChannelStatusClass(status) {
  if (!status) return 'offline'
  const s = status.toLowerCase()
  if (s === 'online') return 'online'
  if (s === 'maintaining') return 'maintaining'
  if (s === 'error') return 'error'
  return 'offline'
}

async function fetchChannels() {
  await channelStore.fetchChannels({
    limit: channelStore.limit,
    offset: (currentPage.value - 1) * channelStore.limit,
    device_tag: deviceTagFilter.value || undefined,
  })
}

function onPageChange(page) {
  channelStore.offset = (page - 1) * channelStore.limit
  fetchChannels()
}

function clearDeviceFilter() {
  router.push('/channels')
}

async function startChannelStream(channel) {
  try {
    await streamStore.startStream({
      device_tag: channel.device_tag,
      channel_tag: channel.channel_tag,
    })
    ElMessage.success('已开始播放')
    router.push('/streams')
  } catch (e) {
    ElMessage.error('启动播放失败: ' + (e?.message || e))
  }
}

onMounted(() => {
  fetchChannels()
})

watch(() => route.query.device_tag, () => {
  currentPage.value = 1
  channelStore.offset = 0
  fetchChannels()
})
</script>
