<template>
  <div class="page-container">
    <div class="page-header">
      <h1 class="page-title">用户管理</h1>
      <div class="page-toolbar">
        <el-button type="primary" :icon="Plus" @click="openCreateModal">创建用户</el-button>
      </div>
    </div>
    <div class="page-body">
      <DataCard>
        <el-skeleton animated :loading="loading" :rows="4">
          <template #default>
            <el-table :data="users" style="margin-top: 0">
              <el-table-column label="用户名" prop="username" />
              <el-table-column label="角色" min-width="160">
                <template #default="{ row }">
                  <el-tag v-for="role in row.roles" :key="role" size="small" style="margin-right: 4px">{{ role }}</el-tag>
                  <span v-if="!row.roles?.length">-</span>
                </template>
              </el-table-column>
              <el-table-column label="创建时间" width="180">
                <template #default="{ row }">{{ formatDate(row.created_at) }}</template>
              </el-table-column>
              <el-table-column label="操作" width="220">
                <template #default="{ row }">
                  <el-button size="small" @click="openEditModal(row)">编辑</el-button>
                  <el-button size="small" @click="openRolesModal(row)">角色</el-button>
                  <el-button v-if="row.id !== currentUserId" size="small" type="danger" plain @click="confirmDelete(row)">删除</el-button>
                </template>
              </el-table-column>
            </el-table>
            <el-empty v-if="users.length === 0" description="暂无用户" />
          </template>
        </el-skeleton>
      </DataCard>
    </div>

    <el-dialog v-model="showCreateModal" title="创建用户" width="420px" :close-on-click-modal="false">
      <el-form label-position="top">
        <el-form-item label="用户名 *" required>
          <el-input v-model="createForm.username" />
        </el-form-item>
        <el-form-item label="密码 *" required>
          <el-input v-model="createForm.password" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateModal = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">创建</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showEditModal" :title="`编辑用户: ${editUser?.username}`" width="420px" :close-on-click-modal="false">
      <el-form-item label="新密码 (留空保持不变)">
        <el-input v-model="editForm.password" type="password" show-password placeholder="留空保持不变" />
      </el-form-item>
      <template #footer>
        <el-button @click="showEditModal = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitEdit">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showRolesModal" :title="`分配角色: ${rolesUser?.username}`" width="420px" :close-on-click-modal="false">
      <div style="display: flex; flex-direction: column; gap: 8px">
        <div
          v-for="role in availableRoles"
          :key="role.name"
          style="display: flex; align-items: center; gap: 8px; padding: 8px; border: 1px solid var(--border); border-radius: 4px; cursor: pointer"
          @click="toggleRole(role.name)"
        >
          <el-checkbox :model-value="rolesForm.roles.includes(role.name)" />
          <div>
            <div style="font-weight: 500">{{ role.name }}</div>
            <div style="font-size: 12px; color: var(--text-muted)">{{ role.description || role.name }}</div>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button @click="showRolesModal = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitRoles">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { useAuthStore } from '../stores/authStore'
import { useToast } from '../composables/useToast'
import * as userApi from '../api/users'
import DataCard from '../components/common/DataCard.vue'

const auth = useAuthStore()
const toast = useToast()

const users = ref([])
const availableRoles = ref([])
const loading = ref(false)
const submitting = ref(false)
const showCreateModal = ref(false)
const showEditModal = ref(false)
const showRolesModal = ref(false)
const editUser = ref(null)
const rolesUser = ref(null)
const currentUserId = auth.currentUser?.id

const createForm = ref({ username: '', password: '' })
const editForm = ref({ password: '' })
const rolesForm = ref({ roles: [] })

onMounted(async () => { await Promise.all([fetchUsers(), fetchRoles()]) })

async function fetchUsers() { loading.value = true; try { users.value = await userApi.getUsers() } catch (e) { toast.error('获取用户列表失败: ' + e.message) } finally { loading.value = false } }
async function fetchRoles() { try { availableRoles.value = await userApi.getRoles() } catch (e) { toast.error('获取角色列表失败') } }

function openCreateModal() { createForm.value = { username: '', password: '' }; showCreateModal.value = true }
async function submitCreate() { submitting.value = true; try { await userApi.createUser(createForm.value); toast.success('用户创建成功'); showCreateModal.value = false; await fetchUsers() } catch {} finally { submitting.value = false } }

function openEditModal(user) { editUser.value = user; editForm.value = { password: '' }; showEditModal.value = true }
async function submitEdit() { if (!editForm.value.password) { showEditModal.value = false; return }; submitting.value = true; try { await userApi.updateUser(editUser.value.id, { password: editForm.value.password }); toast.success('用户更新成功'); showEditModal.value = false } catch {} finally { submitting.value = false } }

function openRolesModal(user) { rolesUser.value = user; rolesForm.value = { roles: [...(user.roles || [])] }; showRolesModal.value = true }
function toggleRole(name) { const idx = rolesForm.value.roles.indexOf(name); if (idx >= 0) rolesForm.value.roles.splice(idx, 1); else rolesForm.value.roles.push(name) }
async function submitRoles() { submitting.value = true; try { await userApi.assignUserRoles(rolesUser.value.id, rolesForm.value.roles); toast.success('角色分配成功'); showRolesModal.value = false; await fetchUsers() } catch {} finally { submitting.value = false } }

async function confirmDelete(user) { try { await ElMessageBox.confirm(`确定删除用户 "${user.username}" 吗？`, '确认删除', { type: 'warning' }); await userApi.deleteUser(user.id); toast.success('用户已删除'); await fetchUsers() } catch {} }

function formatDate(dateStr) { if (!dateStr) return '-'; return new Date(dateStr).toLocaleString('zh-CN', { hour12: false }) }
</script>

<style scoped>
</style>
