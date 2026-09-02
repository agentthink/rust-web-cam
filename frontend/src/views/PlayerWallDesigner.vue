<template>
  <div class="page-container">
    <div class="layout-designer">
      <div class="toolbar">
      <div style="display:flex;align-items:center;gap:8px">
        <el-button :icon="ArrowLeft" @click="$router.push('/video-wall')">返回</el-button>
        <el-select v-model="selectedLayoutId" placeholder="选择布局" style="width:200px" @change="onLayoutSelect">
          <el-option value="" label="-- 新建布局 --" />
          <el-option v-for="l in layoutStore.sortedLayouts" :key="l.id" :label="`${l.name} (${l.rows}×${l.cols})${l.is_default ? ' ★' : ''}`" :value="l.id" />
        </el-select>
      </div>
      <div style="display:flex;align-items:center;gap:8px">
        <el-input v-model="layoutName" placeholder="布局名称" style="width:160px" />
        <el-input-number v-model="rows" :min="1" :max="20" label="行" style="width:110px" @change="resizeCanvas" />
        <span style="color:var(--text-secondary);font-size:13px">×</span>
        <el-input-number v-model="cols" :min="1" :max="20" label="列" style="width:110px" @change="resizeCanvas" />
        <el-button type="primary" :loading="saving" @click="onSave">{{ saving ? '保存中...' : '保存' }}</el-button>
        <el-button v-if="selectedLayoutId" :loading="settingDefault" @click="onSetDefault">{{ currentIsDefault ? '★ 默认' : '设为默认' }}</el-button>
        <el-button @click="clearAll">清空</el-button>
      </div>
    </div>

    <div class="canvas-wrapper" ref="wrapperRef">
      <canvas
        ref="canvasRef"
        class="designer-canvas"
        @mousedown="onMouseDown"
        @mousemove="onMouseMove"
        @mouseup="onMouseUp"
        @contextmenu.prevent="onRightClick"
      />
    </div>

    <div class="designer-info">
      <span>已创建 {{ layoutItems.length }} 个区域</span>
      <span v-if="drawing">拖拽中: ({{ Math.min(dragStartRow, dragEndRow) }},{{ Math.min(dragStartCol, dragEndCol) }}) → ({{ Math.max(dragStartRow, dragEndRow) }},{{ Math.max(dragStartCol, dragEndCol) }})</span>
    </div>
  </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, nextTick, onUnmounted, onMounted } from 'vue'
import { ArrowLeft } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { usePlayerLayoutStore } from '../stores/playerLayoutStore'

const layoutStore = usePlayerLayoutStore()

const COLORS = ['#409EFF', '#67C23A', '#E6A23C', '#F56C6C', '#909399', '#C71585', '#00CED1', '#9370DB']

const rows = ref(3)
const cols = ref(3)
const layoutItems = ref([])
const layoutName = ref('')
const saving = ref(false)
const settingDefault = ref(false)
const drawing = ref(false)
const dragStartRow = ref(0)
const dragStartCol = ref(0)
const dragEndRow = ref(0)
const dragEndCol = ref(0)
const selectedLayoutId = ref('')

const wrapperRef = ref(null)
const canvasRef = ref(null)
let cellWidth = 0
let cellHeight = 0
let colorIndex = 0
let dpr = window.devicePixelRatio || 1

watch([rows, cols], () => nextTick(resizeCanvas))

const themeObserver = new MutationObserver(() => draw())
onMounted(() => {
  themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
})
onUnmounted(() => themeObserver.disconnect())

function resizeCanvas() {
  if (!canvasRef.value || !wrapperRef.value) return
  const wrapper = wrapperRef.value
  const canvas = canvasRef.value
  dpr = window.devicePixelRatio || 1
  const cssW = wrapper.clientWidth
  const cssH = wrapper.clientHeight
  canvas.width = cssW * dpr
  canvas.height = cssH * dpr
  canvas.style.width = cssW + 'px'
  canvas.style.height = cssH + 'px'
  cellWidth = cssW / cols.value
  cellHeight = cssH / rows.value
  draw()
}

function draw() {
  if (!canvasRef.value) return
  const ctx = canvasRef.value.getContext('2d')
  const canvas = canvasRef.value
  const isDark = document.documentElement.classList.contains('dark')
  const theme = {
    bg: isDark ? '#090c12' : '#f5f7fa',
    grid: isDark ? '#1e2330' : '#dcdfe6',
    text: isDark ? '#8892a4' : '#909399',
    textBright: isDark ? '#e2e8f0' : '#303133',
  }
  ctx.save()
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

  ctx.clearRect(0, 0, canvas.width, canvas.height)
  ctx.fillStyle = theme.bg
  ctx.fillRect(0, 0, canvas.width, canvas.height)

  for (let r = 0; r < rows.value; r++) {
    for (let c = 0; c < cols.value; c++) {
      ctx.strokeStyle = theme.grid
      ctx.lineWidth = 1
      ctx.strokeRect(c * cellWidth, r * cellHeight, cellWidth, cellHeight)
    }
  }

  layoutItems.value.forEach((item, idx) => {
    const x = item.col * cellWidth
    const y = item.row * cellHeight
    const w = item.col_span * cellWidth
    const h = item.row_span * cellHeight
    ctx.fillStyle = (item.color || COLORS[0]) + '99'
    ctx.strokeStyle = item.color || COLORS[0]
    ctx.lineWidth = 2
    ctx.fillRect(x, y, w, h)
    ctx.strokeRect(x, y, w, h)
    ctx.fillStyle = theme.textBright
    ctx.font = '12px sans-serif'
    ctx.fillText(`P${idx + 1}`, x + 4, y + 16)
  })

  if (drawing.value) {
    const x1 = Math.min(dragStartCol.value, dragEndCol.value) * cellWidth
    const y1 = Math.min(dragStartRow.value, dragEndRow.value) * cellHeight
    const x2 = (Math.max(dragStartCol.value, dragEndCol.value) + 1) * cellWidth
    const y2 = (Math.max(dragStartRow.value, dragEndRow.value) + 1) * cellHeight
    ctx.strokeStyle = isDark ? '#e2e8f0' : '#409EFF'
    ctx.lineWidth = 2
    ctx.setLineDash([5, 3])
    ctx.strokeRect(x1, y1, x2 - x1, y2 - y1)
    ctx.setLineDash([])
  }
  ctx.restore()
}

function getCell(e) {
  const rect = canvasRef.value.getBoundingClientRect()
  const x = e.clientX - rect.left
  const y = e.clientY - rect.top
  return {
    row: Math.floor(y / cellHeight),
    col: Math.floor(x / cellWidth)
  }
}

function onMouseDown(e) {
  if (e.button !== 0) return
  const cell = getCell(e)
  if (cell.row < 0 || cell.row >= rows.value || cell.col < 0 || cell.col >= cols.value) return
  drawing.value = true
  dragStartRow.value = cell.row
  dragStartCol.value = cell.col
  dragEndRow.value = cell.row
  dragEndCol.value = cell.col
  draw()
}

function onMouseMove(e) {
  if (!drawing.value) return
  const cell = getCell(e)
  dragEndRow.value = Math.max(0, Math.min(cell.row, rows.value - 1))
  dragEndCol.value = Math.max(0, Math.min(cell.col, cols.value - 1))
  draw()
}

function onMouseUp() {
  if (!drawing.value) return
  drawing.value = false
  const r1 = Math.min(dragStartRow.value, dragEndRow.value)
  const c1 = Math.min(dragStartCol.value, dragEndCol.value)
  const r2 = Math.max(dragStartRow.value, dragEndRow.value)
  const c2 = Math.max(dragStartCol.value, dragEndCol.value)
  const rect = { row: r1, col: c1, row_span: r2 - r1 + 1, col_span: c2 - c1 + 1 }
  if (!isOverlapping(rect)) {
    layoutItems.value.push({
      id: generateId(),
      ...rect,
      label: '',
      color: COLORS[colorIndex++ % COLORS.length]
    })
  }
  draw()
}

function isOverlapping(rect) {
  for (const item of layoutItems.value) {
    const aLeft = item.col, aRight = item.col + item.col_span - 1
    const aTop = item.row, aBottom = item.row + item.row_span - 1
    const bLeft = rect.col, bRight = rect.col + rect.col_span - 1
    const bTop = rect.row, bBottom = rect.row + rect.row_span - 1
    if (!(aLeft > bRight || aRight < bLeft || aTop > bBottom || aBottom < bTop)) {
      return true
    }
  }
  return false
}

function onRightClick(e) {
  const cell = getCell(e)
  const idx = layoutItems.value.findIndex(item =>
    cell.row >= item.row && cell.row < item.row + item.row_span &&
    cell.col >= item.col && cell.col < item.col + item.col_span
  )
  if (idx !== -1) {
    layoutItems.value.splice(idx, 1)
    draw()
  }
}

function clearAll() {
  layoutItems.value = []
  colorIndex = 0
  draw()
}

function generateId() {
  return 'item_' + Math.random().toString(36).substring(2, 9)
}

async function onSave() {
  if (!layoutName.value.trim()) {
    ElMessage.warning('请输入布局名称')
    return
  }
  saving.value = true
  try {
    const data = {
      name: layoutName.value.trim(),
      rows: rows.value,
      cols: cols.value,
      layout_json: layoutItems.value.map(({ id, row, col, row_span, col_span, label }) => ({ id, row, col, row_span, col_span, label })),
    }
    if (selectedLayoutId.value) {
      await layoutStore.updateLayout(Number(selectedLayoutId.value), data)
      ElMessage.success('布局已更新')
    } else {
      const created = await layoutStore.createLayout(data)
      if (created) selectedLayoutId.value = String(created.id)
      ElMessage.success('布局已创建')
    }
    await layoutStore.fetchLayouts()
  } catch (e) {
    ElMessage.error('保存失败: ' + (e.message || e))
  } finally {
    saving.value = false
  }
}

async function onSetDefault() {
  if (!selectedLayoutId.value) return
  settingDefault.value = true
  try {
    await layoutStore.setDefault(Number(selectedLayoutId.value))
    await layoutStore.fetchLayouts()
    ElMessage.success('已设为默认布局')
  } catch (e) {
    ElMessage.error('设置失败')
  } finally {
    settingDefault.value = false
  }
}

const currentIsDefault = computed(() => {
  if (!selectedLayoutId.value) return false
  const layout = layoutStore.layouts.find(l => l.id === Number(selectedLayoutId.value))
  return layout?.is_default || false
})

async function onLayoutSelect(id) {
  if (!id) {
    layoutName.value = ''
    rows.value = 3
    cols.value = 3
    layoutItems.value = []
    colorIndex = 0
    await nextTick()
    resizeCanvas()
    return
  }
  const layout = layoutStore.layouts.find(l => l.id === Number(id))
  if (layout) {
    rows.value = layout.rows || 3
    cols.value = layout.cols || 3
    layoutName.value = layout.name || ''
    layoutItems.value = (layout.layout_json || []).map(it => ({
      id: it.id || generateId(),
      row: it.row ?? 0,
      col: it.col ?? 0,
      row_span: it.row_span ?? 1,
      col_span: it.col_span ?? 1,
      label: it.label || '',
      color: COLORS[colorIndex++ % COLORS.length]
    }))
  }
}

const ro = new ResizeObserver(() => resizeCanvas())
onUnmounted(() => ro.disconnect())

onMounted(async () => {
  await layoutStore.fetchLayouts()
  await nextTick()
  if (wrapperRef.value) ro.observe(wrapperRef.value)
  resizeCanvas()
})
</script>

<style scoped>
.layout-designer {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 12px;
  gap: 12px;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
}
.canvas-wrapper {
  flex: 1;
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
  min-height: 400px;
  background: var(--bg-surface);
}
.designer-canvas {
  display: block;
  cursor: crosshair;
  width: 100%;
  height: 100%;
}
.designer-info {
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  gap: 16px;
}
</style>
