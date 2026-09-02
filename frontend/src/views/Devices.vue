<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">设备管理</h1>
      <div class="page-toolbar">
        <el-input v-model="searchQuery" placeholder="搜索设备..." clearable style="max-width: 200px">
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-select v-model="protocolFilter" clearable placeholder="全部协议" style="width: 110px">
          <el-option value="Rtsp" label="RTSP" />
          <el-option value="Gb28181" label="GB28181" />
          <el-option value="Onvif" label="ONVIF" />
          <el-option value="Rtmp" label="RTMP" />
        </el-select>
        <el-select v-model="statusFilter" clearable placeholder="全部状态" style="width: 120px">
          <el-option value="Online" label="在线" />
          <el-option value="Offline" label="离线" />
          <el-option value="Maintaining" label="维护中" />
          <el-option value="Error" label="错误" />
        </el-select>
        <el-select v-model="deviceTypeFilter" clearable placeholder="全部类型" style="width: 100px">
          <el-option value="NVR" label="NVR" />
          <el-option value="IPC" label="IPC" />
          <el-option value="DVR" label="DVR" />
          <el-option value="Camera" label="Camera" />
          <el-option value="Platform" label="Platform" />
          <el-option value="Other" label="Other" />
        </el-select>
        <el-button type="primary" :icon="Plus" @click="openAddModal()">添加设备</el-button>
      </div>
    </div>
    <div class="page-body">
      <el-row :gutter="12" class="devices-layout">
        <el-col :span="6">
          <div class="data-card">
            <div class="data-card__header">
              <div class="group-header">
                <span>分组</span>
                <el-radio-group v-model="groupMode" size="small">
                  <el-radio-button value="region">地区</el-radio-button>
                  <el-radio-button value="custom">自定义</el-radio-button>
                </el-radio-group>
              </div>
            </div>
            <div class="data-card__body" v-if="groupMode === 'region'">
              <el-skeleton animated :rows="3" :loading="regionLoading">
                <div class="region-all-item" :class="{ active: selectedRegion === '' }" @click="onRegionAllClick">
                  全部 <span class="count">{{ deviceStore.total }}</span>
                </div>
                <el-tree
                  ref="regionTreeRef"
                  :data="regionStore.regionTree"
                  :props="{ label: 'name', children: 'children' }"
                  node-key="gb28181_code"
                  :default-expand-all="false"
                  :expand-on-click-node="false"
                  highlight-current
                  @node-click="onRegionNodeClick"
                >
                  <template #default="{ data }">
                    <span>{{ data.name }} ({{ data.device_count || 0 }})</span>
                  </template>
                </el-tree>
                <el-empty v-if="!regionLoading && regionStore.regionTree.length === 0" description="暂无地区数据" :image-size="60" />
              </el-skeleton>
            </div>
            <div class="data-card__body" v-else>
              <el-button size="small" :icon="Plus" style="margin-bottom: 8px" @click="addRootGroup">新建分组</el-button>
              <el-skeleton animated :rows="3" :loading="groupLoading">
                <div class="region-all-item" :class="{ active: selectedGroup === '' }" @click="onGroupAllClick">
                  全部 <span class="count">{{ deviceStore.total }}</span>
                </div>
                <el-tree
                  ref="groupTreeRef"
                  :data="groupStore.groupTree"
                  :props="{ label: 'name', children: 'children' }"
                  node-key="id"
                  :default-expand-all="false"
                  :expand-on-click-node="false"
                  highlight-current
                  :default-expanded-keys="Array.from(expandedGroupIds)"
                  @node-click="onGroupNodeClick"
                  @node-contextmenu="onGroupContextmenu"
                >
                  <template #default="{ data }">
                    <span class="group-node-row">
                      <span class="group-node-label">{{ data.name }} ({{ data.device_count || 0 }})</span>
                      <span class="group-node-actions" @click.stop>
                        <el-tooltip content="添加子分组" :show-after="400">
                          <el-button :icon="FolderAdd" size="small" circle text @click="addChildGroup(data)" />
                        </el-tooltip>
                        <el-tooltip content="重命名" :show-after="400">
                          <el-button :icon="Edit" size="small" circle text @click="editGroup(data)" />
                        </el-tooltip>
                        <el-tooltip content="删除" :show-after="400">
                          <el-button :icon="Delete" size="small" circle text type="danger" @click="deleteGroup(data)" />
                        </el-tooltip>
                      </span>
                    </span>
                  </template>
                </el-tree>
                <el-empty v-if="!groupLoading && groupStore.groupTree.length === 0" description="暂无自定义分组" :image-size="60" />
              </el-skeleton>

              <teleport to="body">
                <div
                  v-if="contextmenuVisible"
                  class="group-contextmenu"
                  :style="{ left: contextmenuX + 'px', top: contextmenuY + 'px' }"
                  @click.stop
                >
                  <div class="ctx-item" @click="handleCtxAddChild"><el-icon><FolderAdd /></el-icon> 新建子分组</div>
                  <div class="ctx-item" @click="handleCtxRename"><el-icon><Edit /></el-icon> 重命名</div>
                  <div class="ctx-item ctx-danger" @click="handleCtxDelete"><el-icon><Delete /></el-icon> 删除</div>
                </div>
              </teleport>
            </div>
          </div>
        </el-col>

        <el-col :span="18">
          <el-text type="info" style="margin-bottom: 12px; display: block">共 {{ deviceStore.total }} 台设备</el-text>
          <div class="data-card">
            <el-skeleton animated :rows="5" :loading="tableLoading">
              <el-table :data="filteredDevices" row-key="id" empty-text="暂无设备">
                  <el-table-column label="状态" width="70" align="center">
                    <template #default="{ row }">
                      <StatusDot :status="getDeviceStatusClass(row.status)" />
                    </template>
                  </el-table-column>
                  <el-table-column label="名称" min-width="180">
                    <template #default="{ row }">
                      <el-link type="primary" @click="$router.push(`/devices/${row.device_tag}`)">{{ row.name }}</el-link>
                    </template>
                  </el-table-column>
                  <el-table-column label="协议" width="100" align="center">
                    <template #default="{ row }">
                      <el-tag size="small" class="protocol-tag" :class="`protocol-${row.protocol?.toLowerCase()}`">{{ row.protocol }}</el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column label="类型" min-width="100" align="center">
                    <template #default="{ row }">
                      <el-tag v-if="row.is_channel" size="small" type="warning">通道</el-tag>
                      <el-tag v-else-if="row.protocol && row.protocol.toLowerCase().includes('gb') && row.device_tag && row.device_tag.length >= 13" size="small" type="primary">
                        {{ getDeviceTypeLabel(row.device_tag.substring(10, 13)) || row.device_type || 'Other' }}
                      </el-tag>
                      <el-tag v-else size="small" type="info">{{ row.device_type || 'Other' }}</el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column label="地址" min-width="140">
                    <template #default="{ row }">
                      <code class="stream-key" style="font-size: 11px">{{ row.host ? `${row.host}:${row.port}` : '-' }}</code>
                    </template>
                  </el-table-column>
                  <el-table-column label="流源" width="110">
                    <template #default="{ row }">
                      <span v-if="row.pull_urls?.length">拉 {{ row.pull_urls.length }}</span>
                      <span v-if="row.push_urls?.length">推 {{ row.push_urls.length }}</span>
                      <span v-if="!row.pull_urls?.length && !row.push_urls?.length">-</span>
                    </template>
                  </el-table-column>
                  <el-table-column label="观看" prop="view_count" width="80" align="center" />
                  <el-table-column label="媒体服务器" prop="media_server_name" min-width="120" show-overflow-tooltip />
                  <el-table-column label="操作" width="220" align="center">
                    <template #default="{ row }">
                      <template v-if="row.has_stream">
                        <el-button size="small" disabled>播放</el-button>
                        <el-button size="small" type="primary" plain @click="router.push('/streams')">流</el-button>
                      </template>
                      <el-button v-else size="small" type="primary" @click="startStreamAndPlay(row)">播放</el-button>
                      <el-button size="small" @click="router.push(`/channels?device_tag=${row.device_tag}`)">通道</el-button>
                      <el-button size="small" @click="router.push(`/devices/${row.device_tag}`)">详情</el-button>
                      <el-button size="small" @click="openEditModal(row)">编辑</el-button>
                      <el-button size="small" type="danger" plain @click="deleteDevice(row)">删除</el-button>
                    </template>
                  </el-table-column>
                </el-table>
            </el-skeleton>

            <el-pagination
              v-if="!tableLoading"
              v-model:current-page="currentPage"
              :page-size="deviceStore.limit"
              :total="deviceStore.total"
              layout="prev, pager, next"
              style="margin-top: 12px; justify-content: flex-end"
              @current-change="fetchDevices"
            />
          </div>
        </el-col>
      </el-row>
    </div>

    <el-dialog v-model="showModal" :title="isEdit ? '编辑设备' : '添加设备'" width="640px" draggable destroy-on-close>
      <el-form label-position="top">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="设备名称 *" required>
              <el-input v-model="form.name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="设备类型">
              <el-select v-model="form.protocol" style="width: 100%">
                <el-option value="Gb28181" label="GB28181" />
                <el-option value="Rtsp" label="RTSP" />
                <el-option value="Onvif" label="ONVIF" />
                <el-option value="Rtmp" label="RTMP" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-form-item label="媒体服务器">
          <el-select v-model="form.media_server_name" clearable style="width: 100%">
            <el-option value="">自动选择</el-option>
            <el-option v-for="s in mediaServerStore.servers" :key="s.name" :label="`${s.name} (${s.server_type})`" :value="s.name" />
          </el-select>
        </el-form-item>

          <el-row :gutter="12">
            <el-col :span="12">
              <el-form-item :label="form.protocol === 'Gb28181' ? '所属地区 *' : '所属地区'" :required="form.protocol === 'Gb28181'">
                <el-cascader
                  v-model="gbRegionCascader"
                  :options="regionStore.regionTree"
                  :props="{ value: 'gb28181_code', label: 'name', children: 'children' }"
                  :placeholder="form.protocol === 'Gb28181' ? '选择行政区划' : '选择行政区划（可选）'"
                  :clearable="form.protocol !== 'Gb28181'"
                  style="width: 100%"
                  @change="onGbRegionChange"
                />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="自定义分组">
                <el-select
                  v-model="form.group_id"
                  placeholder="选择分组（可选）"
                  clearable
                  style="width: 100%"
                >
                  <template v-for="group in groupStore.groupTree" :key="group.id">
                    <el-option :label="group.name" :value="group.id" />
                    <template v-for="child in (group.children || [])" :key="child.id">
                      <el-option :label="'  ├─ ' + child.name" :value="child.id" />
                      <template v-for="grandchild in (child.children || [])" :key="grandchild.id">
                        <el-option :label="'    └─ ' + grandchild.name" :value="grandchild.id" />
                      </template>
                    </template>
                  </template>
                </el-select>
              </el-form-item>
            </el-col>
        </el-row>

        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="应用名称 (App)">
              <el-input v-model="form.app" placeholder="live" />
            </el-form-item>
          </el-col>
        </el-row>

        <div v-if="form.protocol === 'Gb28181'">
          <el-divider>GB28181</el-divider>

          <el-tabs v-model="gb28181Tab">
            <el-tab-pane label="行业类型 *" name="industry">
              <div class="tag-selector">
                <el-tag
                  v-for="opt in gbIndustryCodeOptions"
                  :key="opt.value"
                  :type="form.gb_industry_code === opt.value ? 'primary' : 'info'"
                  class="tag-item selectable"
                  @click="form.gb_industry_code = opt.value"
                >
                  {{ opt.label }}
                </el-tag>
              </div>
            </el-tab-pane>
            <el-tab-pane label="设备类型 *" name="device_type">
              <div class="tag-selector">
                <template v-for="opt in gbDeviceTypeOptions" :key="opt.value || opt.label">
                  <div v-if="opt.type === 'header'" class="tag-category-header">{{ opt.label }}</div>
                  <el-tag
                    v-else
                    :type="form.gb_device_type === opt.value ? 'primary' : 'info'"
                    class="tag-item selectable"
                    @click="form.gb_device_type = opt.value"
                  >
                    {{ opt.label }}
                  </el-tag>
                </template>
              </div>
            </el-tab-pane>
            <el-tab-pane label="网络类型 *" name="network">
              <div class="tag-selector">
                <el-tag
                  v-for="opt in gbNetworkCodeOptions"
                  :key="opt.value"
                  :type="form.gb_network_code === opt.value ? 'primary' : 'info'"
                  class="tag-item selectable"
                  @click="form.gb_network_code = opt.value"
                >
                  {{ opt.label }}
                </el-tag>
              </div>
            </el-tab-pane>
          </el-tabs>

          <div class="gb28181-selected-row">
            <el-tag 
              class="selection-tag" 
              :type="form.gb_industry_code ? 'success' : 'info'"
              @click="gb28181Tab = 'industry'"
            >
              {{ getIndustryCodeLabel(form.gb_industry_code) || '未选行业' }}
            </el-tag>
            <el-tag 
              class="selection-tag" 
              :type="form.gb_device_type ? 'success' : 'info'"
              @click="gb28181Tab = 'device_type'"
            >
              {{ getDeviceTypeLabel(form.gb_device_type) || '未选设备' }}
            </el-tag>
            <el-tag 
              class="selection-tag" 
              :type="form.gb_network_code ? 'success' : 'info'"
              @click="gb28181Tab = 'network'"
            >
              {{ getNetworkCodeLabel(form.gb_network_code) || '未选网络' }}
            </el-tag>
          </div>

          <div class="sip-id-row">
            <el-form-item label="行政区划" class="sip-field">
              <el-input :model-value="sipIdPrefix" disabled placeholder="6位" />
            </el-form-item>
            <span class="sip-plus">+</span>
            <el-form-item label="基层" class="sip-field">
              <el-input v-model="sipGbCode" placeholder="2位" maxlength="2" @input="sipGbCode = sipGbCode.replace(/\D/g, '')" />
            </el-form-item>
            <span class="sip-plus">+</span>
            <el-form-item label="行业" class="sip-field">
              <el-input :model-value="form.gb_industry_code" disabled placeholder="2位" />
            </el-form-item>
            <span class="sip-plus">+</span>
            <el-form-item label="设备" class="sip-field">
              <el-input :model-value="form.gb_device_type" disabled placeholder="3位" />
            </el-form-item>
            <span class="sip-plus">+</span>
            <el-form-item label="网络" class="sip-field">
              <el-input :model-value="form.gb_network_code" disabled placeholder="1位" />
            </el-form-item>
          </div>

          <div class="sip-id-row sip-id-row-bottom">
            <el-form-item label="后缀" class="sip-field">
              <el-input v-model="sipIdSuffix" placeholder="6位" maxlength="6" />
            </el-form-item>
            <span class="sip-equals">=</span>
            <el-form-item label="SIP ID" class="sip-field sip-preview">
              <el-input :model-value="sipIdPreview" disabled placeholder="20位">
                <template #append>
                  <span style="min-width: 40px">{{ sipIdPreview.length }}/20</span>
                </template>
              </el-input>
            </el-form-item>
          </div>

          <el-row :gutter="12">
            <el-col :span="12">
              <el-form-item label="SIP 密码 *" required>
                <el-input v-model="form.device_password" type="password" show-password placeholder="设备SIP认证密码" />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="传输协议">
                <el-select v-model="form.gb_transport" placeholder="选择协议" style="width: 100%">
                  <el-option label="UDP (推荐)" value="UDP" />
                  <el-option label="TCP" value="TCP" />
                </el-select>
              </el-form-item>
            </el-col>
          </el-row>
        </div>

        <div v-if="form.protocol === 'Onvif'">
          <el-divider>ONVIF</el-divider>
          <el-row :gutter="12">
            <el-col :span="24">
              <el-form-item label="设备地址 (XAddr)">
                <el-input v-model="form.host" placeholder="http://192.168.1.100:80/onvif/device_service" />
              </el-form-item>
            </el-col>
          </el-row>
          <el-row :gutter="12">
            <el-col :span="12">
              <el-form-item label="用户名 *">
                <el-input v-model="form.device_username" placeholder="admin" />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="密码 *">
                <el-input v-model="form.device_password" type="password" show-password />
              </el-form-item>
            </el-col>
          </el-row>
          <div v-if="form.pull_urls.length > 0">
            <el-divider content-position="left">
              已发现 RTSP 通道
              <el-button type="primary" link size="small" :loading="onvifFetchingProfiles" @click="onvifFetchFormProfiles" style="margin-left: 8px">重新获取</el-button>
            </el-divider>
            <el-tag v-for="url in form.pull_urls" :key="url.url" type="info" style="margin-right: 8px; margin-bottom: 4px">{{ url.url }}</el-tag>
          </div>
        </div>

        <div v-if="form.protocol === 'Rtsp'">
          <el-divider>RTSP</el-divider>
          <el-form-item label="传输模式">
            <el-radio-group v-model="form.rtsp_mode">
              <el-radio value="pull">拉取（从设备拉流）</el-radio>
              <el-radio value="push">推送（向媒体服务器推流）</el-radio>
            </el-radio-group>
          </el-form-item>

          <div v-if="form.rtsp_mode === 'push'">
            <el-form-item v-if="rtspPushAddress && form.stream_key" label="推送地址预览">
              <el-input :model-value="`${rtspPushAddress.replace(/\/$/, '')}:${rtspPushPort}/${form.app}/${form.stream_key}`" disabled placeholder="请先在基础设置中选择媒体服务器" />
            </el-form-item>
            <el-form-item v-if="!form.media_server_name && form.rtsp_mode === 'push'" label="&nbsp;">
              <el-alert type="info" :closable="false" show-icon>
                请在「基础设置」中选择媒体服务器，推送地址将自动生成
              </el-alert>
            </el-form-item>
          </div>

          <div v-if="form.rtsp_mode === 'pull'">
            <el-row :gutter="12">
              <el-col :span="24">
                <el-form-item label="RTSP 全链接 *" required>
                  <el-input v-model="form.rtsp_full_url" placeholder="rtsp://192.168.1.100:554/live/stream1" />
                </el-form-item>
              </el-col>
            </el-row>
            <el-row :gutter="12">
              <el-col :span="12">
                <el-form-item label="用户名">
                  <el-input v-model="form.device_username" placeholder="admin" />
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="密码">
                  <el-input v-model="form.device_password" type="password" show-password />
                </el-form-item>
              </el-col>
            </el-row>
          </div>
        </div>

        <div v-if="form.protocol === 'Rtmp'">
          <el-divider>RTMP</el-divider>
          <el-form-item v-if="rtmpPushAddress && form.stream_key" label="推送地址预览">
            <el-input :model-value="`${rtmpPushAddress.replace(/\/$/, '')}:${rtmpPushPortActual}/${form.stream_key}`" disabled placeholder="请先在基础设置中选择媒体服务器" />
          </el-form-item>
          <el-form-item v-if="!form.media_server_name" label="&nbsp;">
            <el-alert type="info" :closable="false" show-icon>
              请在「基础设置」中选择媒体服务器，推送地址将自动生成
            </el-alert>
          </el-form-item>
        </div>

        <el-divider>播放认证</el-divider>
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="播放用户名">
              <el-input v-model="form.playback_username" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="播放密码">
              <el-input v-model="form.playback_password" type="password" show-password />
            </el-form-item>
          </el-col>
        </el-row>

        <!-- GB28181 完整流配置 -->
        <div v-if="form.protocol === 'Gb28181'">
          <el-divider>
            <span style="font-size: 12px; color: #909399">SIP INVITE 流配置</span>
            <el-switch v-model="form.enable_stream_config" size="small" style="margin-left: 8px" />
          </el-divider>
          <div v-if="form.enable_stream_config" style="background: #f5f7fa; padding: 16px; border-radius: 4px; margin-bottom: 16px">
            <el-row :gutter="12">
              <el-col :span="6">
                <el-form-item label="视频编码">
                  <el-select v-model="form.stream_config.video_codec" placeholder="选择编码" style="width: 100%">
                    <el-option label="PS (MPEG-PS, 海康大华)" value="PS" />
                    <el-option label="H.264 (通用)" value="H264" />
                    <el-option label="H.265 (新设备)" value="H265" />
                    <el-option label="JPEG (MJPEG)" value="JPEG" />
                  </el-select>
                </el-form-item>
              </el-col>
              <el-col :span="6">
                <el-form-item label="音频编码">
                  <el-select v-model="form.stream_config.audio_codec" placeholder="选择编码" style="width: 100%">
                    <el-option label="PCMA (G.711 A律)" value="PCMA" />
                    <el-option label="PCMU (G.711 U律)" value="PCMU" />
                    <el-option label="AAC" value="AAC" />
                    <el-option label="禁用" value="NONE" />
                  </el-select>
                </el-form-item>
              </el-col>
              <el-col :span="6">
                <el-form-item label="Profile/Level">
                  <el-input v-model="form.stream_config.profile_level_id" placeholder="4D001F">
                    <template #append>
                      <el-tooltip content="H.264/H.265的profile_level_id参数, 可从设备能力获取">
                        <span style="cursor: help">?</span>
                      </el-tooltip>
                    </template>
                  </el-input>
                </el-form-item>
              </el-col>
              <el-col :span="6">
                <el-form-item label="流模式">
                  <el-select v-model="form.stream_config.stream_mode" placeholder="选择模式" style="width: 100%">
                    <el-option label="recvonly (仅接收)" value="recvonly" />
                    <el-option label="sendonly (仅发送)" value="sendonly" />
                    <el-option label="sendrecv (双向)" value="sendrecv" />
                  </el-select>
                </el-form-item>
              </el-col>
            </el-row>
            <el-row :gutter="12">
              <el-col :span="6">
                <el-form-item label="视频PT">
                  <el-input-number v-model="form.stream_config.video_payload_type" :min="0" :max="127" style="width: 100%" placeholder="96" />
                </el-form-item>
              </el-col>
              <el-col :span="6">
                <el-form-item label="音频PT">
                  <el-input-number v-model="form.stream_config.audio_payload_type" :min="0" :max="127" style="width: 100%" placeholder="8" />
                </el-form-item>
              </el-col>
              <el-col :span="6">
                <el-form-item label="SPS/PPS">
                  <el-input v-model="form.stream_config.sprop_parameter_sets" placeholder="Base64编码, 可留空">
                    <template #append>
                      <el-tooltip content="H.264/H.265的SPS/PPS参数, Base64编码, 通常可留空自动发现">
                        <span style="cursor: help">?</span>
                      </el-tooltip>
                    </template>
                  </el-input>
                </el-form-item>
              </el-col>
              <el-col :span="6">
                <el-form-item label="封装模式">
                  <el-select v-model="form.stream_config.packaging_mode" placeholder="选择模式" style="width: 100%">
                    <el-option label="HIS (96字节头)" value="HIS" />
                    <el-option label="NALU (标准)" value="NALU" />
                  </el-select>
                </el-form-item>
              </el-col>
            </el-row>
          </div>
        </div>

        <!-- ONVIF/RTSP 媒体配置 -->
        <div v-if="form.protocol === 'Onvif' || (form.protocol === 'Rtsp' && form.rtsp_mode === 'pull')">
          <el-divider>
            <span style="font-size: 12px; color: #909399">媒体参数</span>
            <el-switch v-model="form.enable_stream_config" size="small" style="margin-left: 8px" />
          </el-divider>
          <div v-if="form.enable_stream_config" style="background: #f5f7fa; padding: 16px; border-radius: 4px; margin-bottom: 16px">
            <p style="margin: 0 0 12px 0; color: #909399; font-size: 12px;">ONVIF/RTSP 设备会自动获取媒体参数，如有兼容性问题可手动覆盖：</p>
            <el-row :gutter="12">
              <el-col :span="8">
                <el-form-item label="视频编码">
                  <el-select v-model="form.stream_config.video_codec" placeholder="自动" style="width: 100%">
                    <el-option label="自动" value="" />
                    <el-option label="H.264" value="H264" />
                    <el-option label="H.265" value="H265" />
                    <el-option label="MJPEG" value="JPEG" />
                  </el-select>
                </el-form-item>
              </el-col>
              <el-col :span="8">
                <el-form-item label="音频编码">
                  <el-select v-model="form.stream_config.audio_codec" placeholder="自动" style="width: 100%">
                    <el-option label="自动" value="" />
                    <el-option label="AAC" value="AAC" />
                    <el-option label="G.711" value="PCMA" />
                    <el-option label="禁用" value="NONE" />
                  </el-select>
                </el-form-item>
              </el-col>
              <el-col :span="8">
                <el-form-item label="流模式">
                  <el-select v-model="form.stream_config.stream_mode" placeholder="自动" style="width: 100%">
                    <el-option label="自动" value="" />
                    <el-option label="recvonly (仅接收)" value="recvonly" />
                    <el-option label="sendrecv (双向)" value="sendrecv" />
                  </el-select>
                </el-form-item>
              </el-col>
            </el-row>
          </div>
        </div>

        <!-- RTMP 推送配置 -->
        <div v-if="form.protocol === 'Rtmp'">
          <el-divider>
            <span style="font-size: 12px; color: #909399">RTMP 推送配置</span>
          </el-divider>
          <div style="background: #f5f7fa; padding: 12px 16px; border-radius: 4px; margin-bottom: 16px; color: #909399; font-size: 12px;">
            <p style="margin: 0;">RTMP 为推送模式，设备主动推送流到媒体服务器。请配置推送地址和流名称。</p>
          </div>
        </div>

        <el-form-item>
          <el-checkbox v-model="form.is_public">公开设备（无需登录可观看）</el-checkbox>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="closeModal">取消</el-button>
        <el-button type="primary" @click="submitDevice">{{ isEdit ? '保存修改' : '确认添加' }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showPlayLinksModal" :title="`播放链接: ${selectedDevice?.name}`" width="560px" draggable destroy-on-close>
      <el-descriptions :column="1" border v-if="availablePlayLinks.length > 0">
        <el-descriptions-item v-for="link in availablePlayLinks" :key="link.key" :label="link.label">
          <div style="display: flex; gap: 8px; align-items: center">
            <el-text truncated style="flex: 1; font-family: monospace; font-size: 12px">{{ link.url }}</el-text>
            <el-button size="small" @click="copyLink(link.url)">复制</el-button>
          </div>
        </el-descriptions-item>
      </el-descriptions>
      <el-empty v-else description="暂无播放链接" />
    </el-dialog>

      <el-dialog v-model="showDiscoverModal" title="发现(ONVIF)设备" width="640px" draggable destroy-on-close>
      <el-divider content-position="left">单播发现（指定 IP）</el-divider>
      <el-row :gutter="12" style="margin-bottom: 12px">
        <el-col :span="16">
          <el-input v-model="onvifManualIp" placeholder="输入设备 IP 地址，如 192.168.1.100" clearable @keyup.enter="onvifDiscoverManual" />
        </el-col>
        <el-col :span="8">
          <el-button type="primary" :loading="onvifDiscovering" @click="onvifDiscoverManual">单播发现</el-button>
        </el-col>
      </el-row>

      <el-divider content-position="left">多播发现（局域网搜索）</el-divider>
      <div style="margin-bottom: 8px">
        <el-button type="info" plain :loading="discovering" @click="discoverDevices">开始多播搜索</el-button>
      </div>

      <el-skeleton animated :rows="4" :loading="discovering || onvifDiscovering">
        <el-table :data="discoveredDevices" empty-text="未发现设备" highlight-current-row @row-click="(row) => onvifSelectedDevice = row">
          <el-table-column label="名称" show-overflow-tooltip>
            <template #default="{ row }">{{ row.name || row.manufacturer || row.model || 'Unknown Device' }}</template>
          </el-table-column>
          <el-table-column label="地址" show-overflow-tooltip>
            <template #default="{ row }">{{ row.address || row.x_addr }}</template>
          </el-table-column>
          <el-table-column label="来源" width="70" align="center">
            <template #default="{ row }">{{ row._from === 'manual' ? '单播' : '多播' }}</template>
          </el-table-column>
          <el-table-column label="厂商/型号" width="120">
            <template #default="{ row }">{{ [row.manufacturer, row.model].filter(Boolean).join(' ') || '-' }}</template>
          </el-table-column>
        </el-table>
      </el-skeleton>

      <template v-if="onvifSelectedDevice">
        <el-divider content-position="left">设备认证</el-divider>
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="用户名">
              <el-input v-model="onvifDiscoverUser" placeholder="admin" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="密码">
              <el-input v-model="onvifDiscoverPwd" type="password" show-password placeholder="password" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-button type="primary" :loading="onvifFetchingProfiles" @click="onvifFetchProfiles" style="margin-bottom: 12px">
          获取能力（媒体通道/RTSP链接）
        </el-button>

        <template v-if="onvifRtspUrls.length > 0">
          <el-divider content-position="left">发现 RTSP 链接（{{ onvifRtspUrls.length }} 个通道）</el-divider>
          <el-table :data="onvifRtspUrls" size="small" max-height="240">
            <el-table-column label="通道名称" prop="name" show-overflow-tooltip />
            <el-table-column label="RTSP 链接" prop="url" show-overflow-tooltip />
          </el-table>
        </template>
      </template>
      <template #footer>
        <el-button @click="closeDiscoverModal">取消</el-button>
        <el-button type="primary" :disabled="!onvifSelectedDevice || onvifRtspUrls.length === 0" :loading="submitting" @click="confirmOnvifDevice">
          确认添加（将创建1个ONVIF设备 + {{ onvifRtspUrls.length }}个RTSP子设备）
        </el-button>
      </template>
    </el-dialog>

    <el-affix position="bottom" :offset="20" v-if="copied">
      <el-alert title="已复制到剪贴板" type="success" :closable="false" show-icon />
    </el-affix>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessageBox, ElMessage } from 'element-plus'
import { Search, Refresh, Plus, Delete, Edit, FolderAdd } from '@element-plus/icons-vue'
import { useDeviceStore } from '../stores/deviceStore'
import { useMediaServerStore } from '../stores/mediaServerStore'
import { useRegionStore } from '../stores/regionStore'
import { useGroupStore } from '../stores/groupStore'
import { useStreamStore } from '../stores/streamStore'
import { useGb28181RefStore } from '../stores/gb28181RefStore'
import { request } from '../utils/request'
import { useToast } from '../composables/useToast'
import StatusDot from '../components/common/StatusDot.vue'
import DataCard from '../components/common/DataCard.vue'

const deviceStore = useDeviceStore()
const mediaServerStore = useMediaServerStore()
const regionStore = useRegionStore()
const groupStore = useGroupStore()
const streamStore = useStreamStore()
const gb28181RefStore = useGb28181RefStore()
const toast = useToast()
const router = useRouter()
const route = useRoute()



function protocolColor(protocol) {
  const map = {
    'GB28181': 'var(--protocol-gb28181)',
    'RTSP': 'var(--protocol-rtsp)',
    'ONVIF': 'var(--protocol-onvif)',
    'RTMP': 'var(--protocol-rtmp)',
  }
  return map[protocol] || 'var(--text-muted)'
}

function getDeviceStatusClass(status) {
  if (!status) return 'offline'
  const s = status.toLowerCase()
  if (s === 'online') return 'online'
  if (s === 'maintaining') return 'maintaining'
  if (s === 'error') return 'error'
  return 'offline'
}

const tableLoading = computed(() => deviceStore.loading)
const regionLoading = computed(() => regionStore.loading)
const groupLoading = computed(() => groupStore.loading)

const showModal = ref(false)
const searchQuery = ref('')
const protocolFilter = ref('')
const statusFilter = ref('')
const deviceTypeFilter = ref('')
const isEdit = ref(false)
const editingId = ref(null)
const showPlayLinksModal = ref(false)
const selectedDevice = ref(null)
const playLinks = ref(null)
const copied = ref(false)
const discovering = ref(false)
const showDiscoverModal = ref(false)
const discoveredDevices = ref([])
const onvifSelectedDevice = ref(null)
const onvifManualIp = ref('')
const onvifDiscoverUser = ref('')
const onvifDiscoverPwd = ref('')
const onvifRtspUrls = ref([])
const onvifFetchingProfiles = ref(false)
const submitting = ref(false)
const onvifDiscovering = ref(false)
const gbRegionCascader = ref([])
const sipGbCode = ref('00')  // 基层编码，默认00
const sipIdSuffix = ref('')  // 后缀，最多6位数字



watch(sipGbCode, (val) => {
  const num = parseInt(val, 10)
  if (isNaN(num) || num < 0) {
    sipGbCode.value = '00'
  } else if (val.length > 2) {
    sipGbCode.value = val.slice(0, 2)
  } else {
    sipGbCode.value = val.padStart(2, '0')
  }
})

const selectedRegion = ref('')
const groupMode = ref('region')
const selectedGroup = ref('')
const regionTreeRef = ref(null)
const groupTreeRef = ref(null)
const contextmenuVisible = ref(false)
const contextmenuX = ref(0)
const contextmenuY = ref(0)
const contextmenuNode = ref(null)
const expandedGroupIds = ref(new Set())

const currentPage = ref(Math.floor(deviceStore.offset / deviceStore.limit) + 1)

watch(currentPage, (page) => {
  deviceStore.offset = (page - 1) * deviceStore.limit
})

let copiedTimer = null

const hasGroupData = computed(() => {
  if (groupMode.value === 'region') return regionStore.regionTree.length > 0
  return groupStore.groupTree.length > 0
})

const sipIdPrefix = computed(() => {
  if (!gbRegionCascader.value || gbRegionCascader.value.length === 0) return ''
  const code = gbRegionCascader.value[gbRegionCascader.value.length - 1] || ''
  return code  // 6位行政区划
})

function findRegionPath(code, tree, path = []) {
  for (const node of tree) {
    const currentPath = [...path, node.gb28181_code]
    if (node.gb28181_code === code) {
      return currentPath
    }
    if (node.children && node.children.length > 0) {
      const found = findRegionPath(code, node.children, currentPath)
      if (found) return found
    }
  }
  return null
}

const sipIdTotalLength = 20
const sipIdSuffixMaxLength = 6  // suffix is always 6 characters

const sipIdPreview = computed(() => {
  if (!sipIdPrefix.value) return ''
  const suffix = sipIdSuffix.value || ''
  const paddedSuffix = suffix.padEnd(6, '_')
  const gbCode = sipGbCode.value || '00'
  // 6位行政区划 + 2位基层编码 + 2位行业 + 3位设备 + 1位网络 + 6位后缀 = 20位
  return sipIdPrefix.value + gbCode + sipIdIndustryCode.value + (form.value.gb_device_type || '') + sipIdNetworkCode.value + paddedSuffix
})

const groupTreeWithAll = computed(() => {
  return [{ id: '', name: '全部', children: [] }, ...groupStore.groupTree]
})

function flattenGroups(groups, result = []) {
  for (const group of groups) {
    if (group.id != null) {
      result.push({ id: String(group.id), name: group.name })
    }
    if (group.children && group.children.length > 0) {
      flattenGroups(group.children, result)
    }
  }
  return result
}

const flatGroupList = computed(() => flattenGroups(groupStore.groupTree))

function onGbRegionChange(val) {
}



function onRegionAllClick() {
  selectedRegion.value = ''
}

function onGroupAllClick() {
  selectedGroup.value = ''
}

const rtspPushServer = computed(() => {
  if (!form.value.media_server_name) return null
  return mediaServerStore.servers.find(s => s.name === form.value.media_server_name) || null
})

const rtspPushAddress = computed(() => {
  const server = rtspPushServer.value
  if (!server) return ''
  return server.url || ''
})

const rtspPushPort = computed(() => {
  const server = rtspPushServer.value
  if (!server) return 554
  return server.protocol_ports?.rtsp || 554
})

const rtmpPushServer = computed(() => {
  if (!form.value.media_server_name) return null
  return mediaServerStore.servers.find(s => s.name === form.value.media_server_name) || null
})

const rtmpPushAddress = computed(() => {
  const server = rtmpPushServer.value
  if (!server) return ''
  return server.url || ''
})

const rtmpPushPortActual = computed(() => {
  const server = rtmpPushServer.value
  if (!server) return 1935
  return server.protocol_ports?.rtmp || 1935
})


async function onvifDiscover() {
  if (!form.value.host.trim()) { ElMessage.warning('请先填写设备地址'); return }
  onvifDiscovering.value = true
  try {
    const res = await request.post('/onvif/discover', { xaddr: form.value.host })
    const profiles = res.data?.data?.profiles || []
    if (profiles.length > 0) {
      const p = profiles[0]
      form.value.device_username = p.username || form.value.device_username
      form.value.device_password = p.password || form.value.device_password
      if (p.stream_uri) {
        form.value.pull_urls = [{ protocol: 'Rtsp', url: p.stream_uri, priority: 1 }]
      }
      ElMessage.success(`发现 ${profiles.length} 个媒体通道`)
    } else {
      ElMessage.warning('未发现媒体通道，请检查地址和认证信息')
    }
  } catch (e) {
    ElMessage.error('发现失败: ' + (e?.message || e))
  } finally {
    onvifDiscovering.value = false
  }
}

const availablePlayLinks = computed(() => {
  if (!playLinks.value) return []
  return [
    { key: 'rtsp_signaling', label: 'RTSP (信令)', url: playLinks.value.rtsp_signaling },
    { key: 'rtsp_media', label: 'RTSP (直连)', url: playLinks.value.rtsp_media },
    { key: 'flv', label: 'HTTP-FLV', url: playLinks.value.flv },
    { key: 'web_flv', label: 'Web-FLV', url: playLinks.value.web_flv },
    { key: 'hls', label: 'HLS', url: playLinks.value.hls },
    { key: 'webrtc', label: 'WebRTC', url: playLinks.value.webrtc },
  ].filter(l => l.url)
})

const filteredDevices = computed(() => {
  return deviceStore.devices.filter(device => {
    const matchesSearch = !searchQuery.value || device.name.toLowerCase().includes(searchQuery.value.toLowerCase())
    const matchesProtocol = !protocolFilter.value || device.protocol === protocolFilter.value
    const matchesStatus = !statusFilter.value || device.status === statusFilter.value
    const matchesDeviceType = !deviceTypeFilter.value || device.device_type === deviceTypeFilter.value
    const inRegionMode = groupMode.value === 'region'
    if (inRegionMode) {
      const matchesRegion = !selectedRegion.value || device.region_code === selectedRegion.value
      return matchesSearch && matchesProtocol && matchesStatus && matchesDeviceType && matchesRegion
    } else {
      const matchesGroup = !selectedGroup.value || device.group_id === selectedGroup.value
      return matchesSearch && matchesProtocol && matchesStatus && matchesDeviceType && matchesGroup
    }
  })
})

const defaultForm = () => ({
  name: '', protocol: 'Gb28181', host: '', port: 554, device_username: '', device_password: '',
  media_server_name: '', pull_urls: [], push_urls: [], playback_username: '', playback_password: '',
  device_tag: '', group_id: null, is_public: false,
  rtsp_mode: 'pull', app: 'live', rtsp_full_url: '',
  rtmp_mode: 'push',
  gb_device_type: '132',
  gb_industry_code: '00',
  gb_network_code: '0',
  gb_transport: 'UDP',
  sip_id_suffix: '',
  extended: {},
  enable_stream_config: false,
  stream_config: {
    video_codec: 'PS',
    audio_codec: 'PCMA',
    video_payload_type: 96,
    audio_payload_type: 8,
    profile_level_id: '4D001F',
    packaging_mode: 'HIS',
    sprop_parameter_sets: '',
    stream_mode: 'recvonly',
  },
})

const gb28181Tab = ref('industry')
const gbNetworkCodeOptions = computed(() => gb28181RefStore.networkCodeOptions)
const gbIndustryCodeOptions = computed(() => gb28181RefStore.industryCodeOptions)
const gbDeviceTypeOptions = computed(() => gb28181RefStore.deviceTypeOptions)

const getNetworkCodeLabel = (code) => {
  const opt = gbNetworkCodeOptions.value.find(o => o.value === code)
  return opt ? opt.label : ''
}
const getDeviceTypeLabel = (code) => {
  const opt = gbDeviceTypeOptions.value.find(o => o.value === code)
  return opt ? opt.label : ''
}
const getIndustryCodeLabel = (code) => {
  const opt = gbIndustryCodeOptions.value.find(o => o.value === code)
  return opt ? opt.label : ''
}

const sipIdIndustryCode = computed(() => form.value.gb_industry_code || '')
const sipIdNetworkCode = computed(() => form.value.gb_network_code || '')

const form = ref(defaultForm())

watch(() => form.value.protocol, (newProto) => {
  if (newProto !== 'Rtsp') {
    form.value.rtsp_mode = 'pull'
    form.value.app = 'live'
    form.value.rtsp_full_url = ''
  }
  if (newProto !== 'Rtmp') {
    form.value.rtmp_mode = 'push'
  }
})



watch(() => form.value.rtsp_mode, (newMode) => {
  if (form.value.protocol !== 'Rtsp') return
  if (newMode === 'push') {
    form.value.host = ''
    form.value.port = 554
    form.value.device_username = ''
    form.value.device_password = ''
    form.value.rtsp_full_url = ''
  } else {
    form.value.app = 'live'
    form.value.rtsp_full_url = ''
    form.value.host = ''
    form.value.port = 554
    form.value.device_username = ''
    form.value.device_password = ''
  }
})

onMounted(async () => {
  await Promise.all([
    fetchDevices(),
    mediaServerStore.fetchServers(),
    fetchRegionTree(),
    fetchGroupTree(),
    gb28181RefStore.fetchRefData(),
  ])
  groupStore.groupTree.forEach(g => g.expanded = false)
  if (groupStore.groupTree.length > 0) {
    expandedGroupIds.value = new Set(groupStore.groupTree.map(n => n.id))
  }
  document.addEventListener('click', closeContextmenu)

  const editId = route.query.edit
  if (editId) {
    const device = deviceStore.devices.find(d => String(d.device_tag) === String(editId))
    if (device) openEditModal(device)
  }
})

watch(() => form.value.host, async (newHost) => {
  if (form.value.protocol !== 'Onvif' || !newHost?.trim()) return
  if (!form.value.device_username || !form.value.device_password) return
  onvifFetchingProfiles.value = true
  form.value.pull_urls = []
  try {
    const capsRes = await request.post('/onvif/capabilities', {
      x_addr: newHost,
      username: form.value.device_username,
      password: form.value.device_password,
    })
    const capsData = capsRes.data?.data
    const profiles = capsData?.profiles || []
    if (profiles.length === 0) return
    const streamRes = await request.post('/onvif/stream-uris', {
      x_addr: newHost,
      media_x_addr: capsData?.capabilities?.media?.x_addr || null,
      username: form.value.device_username,
      password: form.value.device_password,
      profiles: profiles.map(p => p.token),
    })
    const streams = streamRes.data?.data?.streams || []
    form.value.pull_urls = profiles.map((p, i) => {
      const stream = streams.find(s => s.token === p.token) || streams[i] || {}
      return { protocol: 'Rtsp', url: stream.rtsp_url || '', priority: i + 1 }
    }).filter(r => r.url)
    if (form.value.pull_urls.length > 0) {
      ElMessage.success(`发现 ${form.value.pull_urls.length} 个 RTSP 通道`)
    }
  } catch (e) {
    // silent fail for auto-fetch
  } finally {
    onvifFetchingProfiles.value = false
  }
})

async function onvifFetchFormProfiles() {
  const host = form.value.host?.trim()
  if (!host) { ElMessage.warning('请填写设备地址'); return }
  if (!form.value.device_username || !form.value.device_password) { ElMessage.warning('请填写用户名和密码'); return }
  onvifFetchingProfiles.value = true
  form.value.pull_urls = []
  try {
    const capsRes = await request.post('/onvif/capabilities', {
      x_addr: host,
      username: form.value.device_username,
      password: form.value.device_password,
    })
    const capsData = capsRes.data?.data
    const profiles = capsData?.profiles || []
    if (profiles.length === 0) { ElMessage.warning('未发现媒体通道'); return }
    const streamRes = await request.post('/onvif/stream-uris', {
      x_addr: host,
      media_x_addr: capsData?.capabilities?.media?.x_addr || null,
      username: form.value.device_username,
      password: form.value.device_password,
      profiles: profiles.map(p => p.token),
    })
    const streams = streamRes.data?.data?.streams || []
    form.value.pull_urls = profiles.map((p, i) => {
      const stream = streams.find(s => s.token === p.token) || streams[i] || {}
      return { protocol: 'Rtsp', url: stream.rtsp_url || '', priority: i + 1 }
    }).filter(r => r.url)
    if (form.value.pull_urls.length > 0) {
      ElMessage.success(`发现 ${form.value.pull_urls.length} 个 RTSP 通道`)
    } else {
      ElMessage.warning('未发现 RTSP 链接')
    }
  } catch (e) {
    ElMessage.error('获取通道失败: ' + (e?.message || e))
  } finally {
    onvifFetchingProfiles.value = false
  }
}

async function fetchDevices() {
  await deviceStore.fetchDevices(deviceStore.limit, deviceStore.offset)
}

async function fetchRegionTree() {
  await regionStore.fetchRegionTree()
}

async function fetchGroupTree() {
  await groupStore.fetchGroupTree()
}

watch(groupMode, () => { selectedRegion.value = ''; selectedGroup.value = '' })

function onRegionNodeClick(data) { selectedRegion.value = selectedRegion.value === data.code ? '' : data.code }
function onGroupNodeClick(data) { selectedGroup.value = selectedGroup.value === data.id ? '' : data.id }

function onGroupContextmenu(e, data) {
  selectedGroup.value = data.id
  contextmenuNode.value = data
  contextmenuX.value = e.clientX
  contextmenuY.value = e.clientY
  contextmenuVisible.value = true
}

function closeContextmenu() {
  contextmenuVisible.value = false
  contextmenuNode.value = null
}

function findParentChain(nodeId, tree, chain = []) {
  for (const node of tree) {
    if (node.id === nodeId) return chain
    if (node.children) {
      const found = findParentChain(nodeId, node.children, [...chain, node.id])
      if (found) return found
    }
  }
  return null
}

function restoreGroupExpanded(preserveIds = []) {
  if (!groupTreeRef.value) return
  for (const id of preserveIds) {
    expandedGroupIds.value.add(id)
    try { groupTreeRef.value.expandNode(groupTreeRef.value.getNode(id)) } catch {}
  }
}

function handleCtxAddChild() {
  if (contextmenuNode.value) addChildGroup(contextmenuNode.value)
  closeContextmenu()
}

function handleCtxRename() {
  if (contextmenuNode.value) editGroup(contextmenuNode.value)
  closeContextmenu()
}

function handleCtxDelete() {
  if (contextmenuNode.value) deleteGroup(contextmenuNode.value)
  closeContextmenu()
}

async function addRootGroup() {
  try {
    const raw = await ElMessageBox.prompt('请输入分组名称:', '新建分组', { confirmButtonText: '确定', cancelButtonText: '取消' })
    const name = typeof raw === 'object' ? raw.value : raw
    if (!name || !String(name).trim()) return
    await groupStore.createGroup({ name: String(name).trim() })
    ElMessage.success('分组已创建')
    await fetchGroupTree()
  } catch (e) {
    if (e !== 'cancel') ElMessage.error('创建分组失败: ' + (e?.message || e))
  }
}

async function addChildGroup(parentNode) {
  try {
    const raw = await ElMessageBox.prompt('请输入子分组名称:', '添加子分组', { confirmButtonText: '确定', cancelButtonText: '取消' })
    const name = typeof raw === 'object' ? raw.value : raw
    if (!name || !String(name).trim()) return
    await groupStore.createGroup({ name: String(name).trim(), parent_id: parentNode.id })
    ElMessage.success('子分组已创建')
    await fetchGroupTree()
    expandedGroupIds.value.add(parentNode.id)
  } catch (e) {
    if (e !== 'cancel') ElMessage.error('创建子分组失败: ' + (e?.message || e))
  }
}

async function editGroup(node) {
  try {
    const raw = await ElMessageBox.prompt('请输入分组名称:', '重命名', {
      confirmButtonText: '确定', cancelButtonText: '取消', inputValue: node.name,
    })
    const name = typeof raw === 'object' ? raw.value : raw
    if (!name || !String(name).trim()) return
    await groupStore.updateGroup(node.id, { name: String(name).trim() })
    ElMessage.success('分组已重命名')
    await fetchGroupTree()
    const chain = findParentChain(node.id, groupStore.groupTree, [])
    restoreGroupExpanded([...chain, node.id])
  } catch (e) {
    if (e !== 'cancel') ElMessage.error('重命名失败: ' + (e?.message || e))
  }
}

async function deleteGroup(node) {
  try {
    const childNames = node.children?.map(c => c.name).join('、') || ''
    const msg = childNames
      ? `确定删除分组"${node.name}"吗？删除后以下子分组也会被删除：${childNames}`
      : `确定删除分组"${node.name}"吗？`
    await ElMessageBox.confirm(msg, '确认删除', { type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消' })
    const chain = findParentChain(node.id, groupStore.groupTree, [])
    await groupStore.deleteGroup(node.id)
    if (selectedGroup.value === node.id) selectedGroup.value = ''
    ElMessage.success('分组已删除')
    await fetchGroupTree()
    restoreGroupExpanded(chain)
  } catch (e) {
    if (e !== 'cancel') ElMessage.error('删除失败: ' + (e?.message || e))
  }
}

function openAddModal() {
  isEdit.value = false; editingId.value = null
  const newForm = defaultForm()
  newForm.gb_industry_code = newForm.gb_industry_code || '00'
  newForm.gb_device_type = newForm.gb_device_type || '132'
  newForm.gb_network_code = newForm.gb_network_code || '0'
  form.value = newForm
  gbRegionCascader.value = []
  sipGbCode.value = '00'
  sipIdSuffix.value = ''
  showModal.value = true
}

async function openEditModal(device) {
  isEdit.value = true; editingId.value = device.id
  try {
    const fullDevice = await deviceStore.fetchDevice(device.device_tag)
    const ext = fullDevice.extended || {}
    const devStreamConfig = ext.stream_config || {}
    form.value = {
      name: fullDevice.name || '', protocol: fullDevice.protocol || 'Gb28181', host: fullDevice.host || '',
      port: fullDevice.port || 554, device_username: fullDevice.device_username || '', device_password: fullDevice.device_password || '',
      media_server_name: fullDevice.media_server_tag || '',
      pull_urls: fullDevice.pull_urls ? fullDevice.pull_urls.map(u => ({ ...u })) : [],
      push_urls: fullDevice.push_urls ? fullDevice.push_urls.map(u => ({ ...u })) : [],
      playback_username: fullDevice.playback_username || '', playback_password: fullDevice.playback_password || '',
      device_tag: fullDevice.device_tag || '', group_id: fullDevice.group_id ?? null, is_public: fullDevice.is_public || false,
      rtsp_mode: ext.rtsp_mode || fullDevice.rtsp_mode || 'pull',
      app: fullDevice.app || 'live',
      rtsp_full_url: ext.rtsp_full_url || fullDevice.rtsp_full_url || '',
      rtmp_mode: ext.rtmp_mode || fullDevice.rtmp_mode || 'push',
      extended: ext,
      enable_stream_config: !!devStreamConfig,
      stream_config: {
        video_codec: devStreamConfig.video_codec || 'PS',
        audio_codec: devStreamConfig.audio_codec || 'PCMA',
        video_payload_type: devStreamConfig.video_payload_type || 96,
        audio_payload_type: devStreamConfig.audio_payload_type || 8,
        profile_level_id: devStreamConfig.profile_level_id || '4D001F',
        packaging_mode: devStreamConfig.packaging_mode || 'HIS',
        sprop_parameter_sets: devStreamConfig.sprop_parameter_sets || '',
        stream_mode: devStreamConfig.stream_mode || 'recvonly',
      },
      gb_industry_code: '',
      gb_device_type: '',
      gb_network_code: '',
      gb_transport: ext.gb_transport || 'UDP',
    }
    const isGb28181 = fullDevice.protocol && fullDevice.protocol.toLowerCase().includes('gb')
    if (isGb28181 && fullDevice.device_tag) {
      const tag = fullDevice.device_tag || ''
      if (tag.length !== 20) {
        ElMessage.warning(`设备SIP ID格式错误: ${tag} (${tag.length}位)，应为20位`)
      }
      const regionCode = fullDevice.region_code || (tag.length >= 6 ? tag.substring(0, 6) : '')
      const regionPath = findRegionPath(regionCode, regionStore.regionTree)
      gbRegionCascader.value = regionPath || (regionCode ? [regionCode] : [])
      sipGbCode.value = tag.length >= 8 ? tag.substring(6, 8) : '00'
      form.value.gb_industry_code = tag.length >= 10 ? tag.substring(8, 10) : ''
      form.value.gb_device_type = tag.length >= 13 ? tag.substring(10, 13) : ''
      form.value.gb_network_code = tag.length >= 14 ? tag.substring(13, 14) : ''
      sipIdSuffix.value = tag.length >= 20 ? tag.substring(14, 20) : ''
    } else {
      const regionCode = fullDevice.region_code || ''
      const regionPath = findRegionPath(regionCode, regionStore.regionTree)
      gbRegionCascader.value = regionPath || (regionCode ? [regionCode] : [])
      sipGbCode.value = '00'
      form.value.gb_industry_code = ''
      form.value.gb_device_type = ''
      form.value.gb_network_code = ''
      sipIdSuffix.value = ''
    }
    if (fullDevice.protocol === 'Onvif' && fullDevice.host) {
      nextTick(() => onvifFetchFormProfiles())
    }
    showModal.value = true
  } catch (e) {
    ElMessage.error('获取设备详情失败: ' + (e?.message || e))
  }
}

function closeModal() { showModal.value = false }

function addPullUrl() { form.value.pull_urls.push({ protocol: 'Rtsp', url: '', priority: form.value.pull_urls.length + 1 }) }
function removePullUrl(index) { form.value.pull_urls.splice(index, 1) }
function addPushUrl() { form.value.push_urls.push({ protocol: 'Rtmp', url: '', priority: form.value.push_urls.length + 1 }) }
function removePushUrl(index) { form.value.push_urls.splice(index, 1) }

async function submitDevice() {
  if (!form.value.name.trim()) { ElMessage.error('请填写设备名称'); return }
  if (form.value.protocol === 'Gb28181') {
    if (!sipIdPrefix.value) { ElMessage.error('请选择所属地区'); return }
    if (!form.value.gb_industry_code) { ElMessage.error('请选择行业类型'); return }
    if (!form.value.gb_device_type) { ElMessage.error('请选择设备类型'); return }
    if (!form.value.gb_network_code) { ElMessage.error('请选择网络类型'); return }
    if (!sipIdSuffix.value || sipIdSuffix.value.length !== 6) {
      ElMessage.error('SIP ID 后缀必须填写 6 位数字')
      return
    }
    const previewLength = sipIdPreview.value.length
    if (previewLength !== 20) {
      ElMessage.error(`SIP ID 必须是20位，当前${previewLength}位，请检查：行政区划${sipIdPrefix.value.length}位 + 基层2位 + 行业2位 + 设备3位 + 网络1位 + 后缀6位`)
      return
    }
    if (!form.value.device_password) { ElMessage.error('请填写SIP密码'); return }
  }
  if (form.value.protocol === 'Onvif' && !form.value.host.trim()) { ElMessage.error('请填写设备地址'); return }
  if (form.value.protocol === 'Onvif' && !form.value.device_username) { ElMessage.error('请填写用户名'); return }
  if (form.value.protocol === 'Onvif' && !form.value.device_password) { ElMessage.error('请填写密码'); return }
  if (form.value.protocol === 'Rtsp' && form.value.rtsp_mode === 'pull' && !form.value.rtsp_full_url.trim()) { ElMessage.error('请填写RTSP全链接'); return }

  const finalDeviceTag = isEdit.value
    ? (form.value.protocol === 'Gb28181' ? sipIdPreview.value : editingId.value)
    : (form.value.protocol === 'Gb28181'
      ? sipIdPreview.value
      : form.value.device_tag || null)

  const payload = {
    name: form.value.name, protocol: form.value.protocol,
    device_username: form.value.device_username || null, device_password: form.value.device_password || null,
    media_server_tag: form.value.media_server_name || null,
    pull_urls: form.value.pull_urls.filter(u => u.url).length > 0 ? form.value.pull_urls : undefined,
    push_urls: form.value.push_urls.filter(u => u.url).length > 0 ? form.value.push_urls : undefined,
    playback_username: form.value.playback_username || null, playback_password: form.value.playback_password || null,
    device_tag: finalDeviceTag, region_code: form.value.protocol === 'Gb28181' ? sipIdPrefix.value || null : null, group_id: form.value.group_id || null, is_public: form.value.is_public,
    app: form.value.app || null,
    extended: {
      ...(form.value.extended || {}),
      rtsp_mode: form.value.protocol === 'Rtsp' ? form.value.rtsp_mode : undefined,
      rtsp_full_url: form.value.protocol === 'Rtsp' && form.value.rtsp_mode === 'pull' ? form.value.rtsp_full_url || null : null,
      rtmp_mode: form.value.protocol === 'Rtmp' ? form.value.rtmp_mode : undefined,
      gb_transport: form.value.protocol === 'Gb28181' ? form.value.gb_transport : undefined,
    },
  }

  // GB28181: 完整流配置
  if (form.value.protocol === 'Gb28181' && form.value.enable_stream_config) {
    payload.extended.stream_config = {
      video_codec: form.value.stream_config.video_codec || null,
      audio_codec: form.value.stream_config.audio_codec || null,
      video_payload_type: form.value.stream_config.video_payload_type,
      audio_payload_type: form.value.stream_config.audio_payload_type,
      profile_level_id: form.value.stream_config.profile_level_id || null,
      packaging_mode: form.value.stream_config.packaging_mode || null,
      sprop_parameter_sets: form.value.stream_config.sprop_parameter_sets || null,
      stream_mode: form.value.stream_config.stream_mode || null,
    }
  }
  if (isEdit.value) {
    await deviceStore.updateDevice(editingId.value, payload)
    await fetchDevices(); closeModal()
    return
  }
  if (form.value.protocol === 'Onvif' && form.value.pull_urls.filter(u => u.url).length > 1) {
    const parentPayload = { ...payload }
    parentPayload.pull_urls = [form.value.pull_urls[0]]
    const parent = await deviceStore.createDevice(parentPayload)
    const parentId = parent?.id || parent?.data?.id
    for (let i = 1; i < form.value.pull_urls.length; i++) {
      const url = form.value.pull_urls[i]
      await deviceStore.createDevice({
        name: `${form.value.name} - 通道${i + 1}`, protocol: 'Rtsp',
        device_username: form.value.device_username || null,
        device_password: form.value.device_password || null,
        media_server_tag: form.value.media_server_name || null,
        rtsp_mode: 'pull', rtsp_full_url: url.url,
        pull_urls: [{ protocol: 'Rtsp', url: url.url, priority: 1 }],
        playback_username: form.value.playback_username || null,
        playback_password: form.value.playback_password || null,
        device_tag: null, region_code: null, group_id: form.value.group_id || null, is_public: form.value.is_public || false,
        app: form.value.app || null,
        extended: {},
      })
    }
    ElMessage.success(`成功创建 ${1 + form.value.pull_urls.length - 1} 个设备`)
    await fetchDevices(); closeModal()
    return
  }
  await deviceStore.createDevice(payload)
  await fetchDevices(); closeModal()
}

async function discoverDevices() {
  discovering.value = true; discoveredDevices.value = []
  try {
    const res = await request.post('/onvif/discover')
    discoveredDevices.value = res.data?.data || []
  } catch (e) {
    toast.error('发现设备失败: ' + (e?.message || e))
  } finally {
    discovering.value = false
  }
}

function openOnvifDiscoverModal() {
  onvifSelectedDevice.value = null
  onvifRtspUrls.value = []
  onvifDiscoverUser.value = ''
  onvifDiscoverPwd.value = ''
  onvifManualIp.value = ''
  showDiscoverModal.value = true
  discoveredDevices.value = []
}

function onvifDiscoverManual() {
  if (!onvifManualIp.value.trim()) { ElMessage.warning('请输入 IP 地址'); return }
  onvifDiscovering.value = true
  onvifRtspUrls.value = []
  const ip = onvifManualIp.value.trim()
  const port = ip.includes(':') ? parseInt(ip.split(':')[1]) : 80
  const host = ip.includes(':') ? ip.split(':')[0] : ip
  request.post('/onvif/probe', { host, port }).then(res => {
    const data = res.data?.data
    const devices = Array.isArray(data) ? data : (data ? [data] : [])
    if (devices.length === 0) {
      ElMessage.warning('未发现 ONVIF 设备，请检查 IP 地址或设备是否在线')
      return
    }
    devices.forEach(dev => {
      const devAddr = dev.x_addr || `http://${dev.host}:${dev.port}/onvif/device_service`
      const exists = discoveredDevices.value.find(d => (d.address || d.x_addr) === devAddr)
      if (!exists) {
        discoveredDevices.value.push({ ...dev, address: devAddr, _from: 'manual' })
      }
    })
    onvifSelectedDevice.value = discoveredDevices.value.find(d => d.host === host) || discoveredDevices.value[discoveredDevices.value.length - 1]
  }).catch(e => {
    toast.error('发现失败: ' + (e?.message || e))
  }).finally(() => {
    onvifDiscovering.value = false
  })
}

function closeDiscoverModal() {
  showDiscoverModal.value = false
  onvifSelectedDevice.value = null
  onvifRtspUrls.value = []
  onvifDiscoverUser.value = ''
  onvifManualIp.value = ''
  onvifDiscoverPwd.value = ''
}

async function onvifFetchProfiles() {
  const dev = onvifSelectedDevice.value
  if (!dev) return
  const address = dev.address || dev.x_addr || ''
  if (!address) { ElMessage.warning('设备地址为空'); return }
  onvifFetchingProfiles.value = true
  onvifRtspUrls.value = []
  try {
    const capsRes = await request.post('/onvif/capabilities', {
      x_addr: address,
      username: onvifDiscoverUser.value || '',
      password: onvifDiscoverPwd.value || '',
    })
    const capsData = capsRes.data?.data
    const profiles = capsData?.profiles || []
    if (profiles.length === 0) {
      ElMessage.warning('未发现媒体通道，请检查地址和认证信息')
      return
    }
    const streamRes = await request.post('/onvif/stream-uris', {
      x_addr: address,
      media_x_addr: capsData?.capabilities?.media?.x_addr || null,
      username: onvifDiscoverUser.value || '',
      password: onvifDiscoverPwd.value || '',
      profiles: profiles.map(p => p.token),
    })
    const streams = streamRes.data?.data?.streams || []
    onvifRtspUrls.value = profiles.map((p, i) => {
      const stream = streams.find(s => s.token === p.token) || streams[i] || {}
      return {
        name: p.name || `通道${i + 1}`,
        url: stream.rtsp_url || '',
      }
    }).filter(r => r.url)
    if (onvifRtspUrls.value.length === 0) {
      ElMessage.warning('未发现 RTSP 链接，请检查用户名和密码')
    } else {
      ElMessage.success(`发现 ${onvifRtspUrls.value.length} 个 RTSP 通道`)
    }
  } catch (e) {
    ElMessage.error('获取能力失败: ' + (e?.message || e))
  } finally {
    onvifFetchingProfiles.value = false
  }
}

async function confirmOnvifDevice() {
  const dev = onvifSelectedDevice.value
  if (!dev || onvifRtspUrls.value.length === 0) return
  submitting.value = true
  try {
    const address = dev.address || dev.x_addr || ''
    const manufacturer = dev.manufacturer || ''
    const model = dev.model || ''
    const parentName = form.value.name?.trim() || dev.name || `${manufacturer} ${model}`.trim() || 'ONVIF 设备'
    const parentPayload = {
      name: parentName, protocol: 'Onvif',
      device_username: onvifDiscoverUser.value || null,
      device_password: onvifDiscoverPwd.value || null,
      media_server_tag: form.value.media_server_name || null,
      pull_urls: onvifRtspUrls.value.map((r, i) => ({ protocol: 'Rtsp', url: r.url, priority: i + 1 })),
      playback_username: form.value.playback_username || null,
      playback_password: form.value.playback_password || null,
      device_tag: null, region_code: null, group_id: form.value.group_id || null, is_public: form.value.is_public || false,
      app: form.value.app || null,
      extended: { stream_key: form.value.stream_key || null },
    }
    const parent = await deviceStore.createDevice(parentPayload)
    const parentId = parent?.id || parent?.data?.id
    if (parentId && onvifRtspUrls.value.length > 1) {
      for (let i = 1; i < onvifRtspUrls.value.length; i++) {
        const childName = onvifRtspUrls.value[i].name || `通道${i + 1}`
        await deviceStore.createDevice({
          name: `${parentName} - ${childName}`, protocol: 'Rtsp',
          device_username: onvifDiscoverUser.value || null,
          device_password: onvifDiscoverPwd.value || null,
          media_server_tag: form.value.media_server_name || null,
          rtsp_mode: 'pull',
          rtsp_full_url: onvifRtspUrls.value[i].url,
          pull_urls: [{ protocol: 'Rtsp', url: onvifRtspUrls.value[i].url, priority: 1 }],
          playback_username: form.value.playback_username || null,
          playback_password: form.value.playback_password || null,
          device_tag: null, region_code: null, group_id: form.value.group_id || null, is_public: form.value.is_public || false,
          app: form.value.app || null,
          extended: { stream_key: form.value.stream_key || null },
        })
      }
    }
    ElMessage.success(`成功创建 ${1 + Math.max(0, onvifRtspUrls.value.length - 1)} 个设备`)
    closeDiscoverModal()
    closeModal()
    await fetchDevices()
  } catch (e) {
    ElMessage.error('创建设备失败: ' + (e?.message || e))
  } finally {
    submitting.value = false
  }
}

function addDiscoveredDevice(dev) {
  const address = dev.address || dev.x_addr || ''
  let host = '', port = 554
  if (address) {
    const lastColon = address.lastIndexOf(':')
    if (lastColon > 0) {
      host = address.substring(0, lastColon)
      port = parseInt(address.substring(lastColon + 1)) || 554
    } else {
      host = address
    }
  }
  const manufacturer = dev.manufacturer || ''
  const model = dev.model || ''
  const name = dev.name || `${manufacturer} ${model}`.trim() || `ONVIF ${host}`
  form.value = defaultForm()
  form.value.name = name
  form.value.host = address
  form.value.port = port
  form.value.protocol = 'Onvif'
  form.value.device_username = ''
  form.value.device_password = ''
  showDiscoverModal.value = false
  isEdit.value = false
  editingId.value = null
  showModal.value = true
}

async function openPlayLinks(device) {
  selectedDevice.value = device
  playLinks.value = await deviceStore.getPlayLinks(device.device_tag)
  showPlayLinksModal.value = true
}

async function startStreamAndPlay(device) {
  try {
    const result = await import('../api/devices.js').then(m => m.startDevice(device.device_tag))
    if (result?.message) {
      ElMessage.info(result.message)
    } else {
      ElMessage.success('流已启动')
    }
  } catch (e) {
    const msg = e?.response?.data?.message || e?.message || String(e)
    ElMessage.error('启动流失败: ' + msg)
    return
  }
  await router.push('/streams')
}

async function deleteDevice(device) {
  try {
    await ElMessageBox.confirm(`确定删除设备 "${device.name}" 吗？`, '确认删除', { type: 'warning' })
    await deviceStore.deleteDevice(device.device_tag)
    await fetchDevices()
  } catch {}
}

async function copyLink(text) {
  try {
    await navigator.clipboard.writeText(text)
  } catch {
    const el = document.createElement('textarea')
    el.value = text; document.body.appendChild(el); el.select(); document.execCommand('copy'); document.body.removeChild(el)
  }
  copied.value = true; clearTimeout(copiedTimer); copiedTimer = setTimeout(() => { copied.value = false }, 2000)
}
</script>

<style scoped>
.stream-key {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
.mono { font-family: var(--font-mono); }

.group-header { display: flex; align-items: center; justify-content: space-between; }
.devices-layout { margin-top: 0; }

.group-node-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding-right: 4px;
}
.group-node-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.group-node-actions {
  display: none;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
.group-node-row:hover .group-node-actions {
  display: flex;
}
.group-node-actions .el-button {
  padding: 2px;
  font-size: 12px;
}

.group-contextmenu {
  position: fixed;
  z-index: 9999;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-base);
  box-shadow: var(--shadow-lg);
  padding: var(--space-2);
  min-width: 140px;
}
.ctx-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  cursor: pointer;
  font-size: var(--text-sm);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  transition: background 0.15s;
  font-family: var(--font-sans);
}
.ctx-item:hover {
  background: var(--color-accent-dim);
  color: var(--color-accent);
}
.ctx-danger { color: var(--color-danger); }
.ctx-danger:hover {
  background: rgba(239, 68, 68, 0.1);
  color: var(--color-danger);
}
.region-all-item {
  padding: var(--space-2) var(--space-3);
  cursor: pointer;
  border-radius: var(--radius-base);
  font-size: var(--text-sm);
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: var(--text-primary);
  margin-bottom: var(--space-1);
  transition: background 0.15s, color 0.15s;
  font-family: var(--font-sans);
}
.region-all-item:hover {
  background: var(--color-accent-dim);
  color: var(--color-accent);
}
.region-all-item.active {
  background: var(--color-accent-dim);
  color: var(--color-accent);
  font-weight: var(--weight-semibold);
}
.region-all-item .count {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.protocol-tag {
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: var(--weight-semibold);
  border: none;
  color: #fff;
}
.protocol-tag.protocol-gb28181 { background-color: var(--protocol-gb28181); }
.protocol-tag.protocol-rtsp    { background-color: var(--protocol-rtsp); }
.protocol-tag.protocol-onvif   { background-color: var(--protocol-onvif); }
.protocol-tag.protocol-rtmp    { background-color: var(--protocol-rtmp); }

.tag-selector {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 8px 0;
  min-height: 80px;
  max-height: 160px;
  overflow-y: auto;
}

.gb28181-selected-row {
  display: flex;
  gap: 12px;
  margin-top: 12px;
}
.selection-tag {
  cursor: pointer;
}
.tag-item {
  cursor: pointer;
  transition: all 0.2s;
}
.tag-item.selectable {
  user-select: none;
}
.tag-item:hover {
  transform: translateY(-1px);
}
.tag-item.clear-btn {
  border-style: dashed;
}
.tag-selected-info {
  font-size: 12px;
  color: var(--text-secondary);
  padding: 4px 8px;
  background: var(--bg-color);
  border-radius: 4px;
  margin-top: 8px;
}
.tag-category-header {
  width: 100%;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 8px 0 4px 0;
  padding-left: 4px;
  border-left: 3px solid var(--color-primary);
}
.tag-category-header:first-child {
  margin-top: 0;
}

.gb28181-selected-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-color);
  border-radius: 6px;
  margin-bottom: 12px;
  font-size: 13px;
}
.gb28181-selected-summary .summary-item {
  color: var(--text-secondary);
}
.gb28181-selected-summary .summary-item strong {
  color: var(--text-primary);
  font-weight: 600;
}
.gb28181-selected-summary .summary-divider {
  color: var(--border-color);
}

.sip-id-row {
  display: flex;
  align-items: center;
  gap: 4px;
  margin: 16px 0;
  flex-wrap: nowrap;
  overflow-x: auto;
}
.sip-id-row-bottom {
  margin-top: 8px;
}
.sip-id-row .el-form-item {
  margin-bottom: 0;
  flex-shrink: 0;
}
.sip-field {
  width: 90px;
}
.sip-field .el-input__inner {
  font-family: var(--font-mono);
  font-size: 12px;
  text-align: center;
  padding: 0 4px;
}
.sip-field.sip-preview {
  width: 280px;
}
.sip-field.sip-preview .el-input__inner {
  font-size: 14px;
  letter-spacing: 1px;
}
.sip-plus, .sip-equals {
  color: var(--text-muted);
  font-weight: bold;
  flex-shrink: 0;
  align-self: flex-end;
  padding-bottom: 6px;
}
</style>
