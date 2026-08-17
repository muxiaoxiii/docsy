<template>
  <el-dialog v-model="visibleModel" title="确认原有页眉、页脚和页码" width="min(1080px, 94vw)" append-to-body @click.self="clearSelection">
    <div class="decision-toolbar">
      <el-select v-model="fileFilter" size="small" placeholder="全部文件" clearable style="width: 200px">
        <el-option v-for="f in fileNames" :key="f" :label="f" :value="f" />
      </el-select>
      <el-button size="small" @click="selectAll">全选</el-button>
      <el-button size="small" @click="invertSelection">反选</el-button>
      <el-button size="small" @click="selectUndecided">选择未确认项</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" @click="clearSelection">取消选择</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" @click="applyDecision('keep')">保留</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" @click="applyDecision('ignore')">忽略识别</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" type="danger" @click="applyDecision('delete')">标记删除</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" type="primary" @click="applyDecision('edit')">标记编辑</el-button>
    </div>
    <el-table
      :data="filteredRows"
      border
      size="small"
      max-height="58vh"
      row-key="key"
      :row-class-name="rowClassName"
      @row-click="handleRowClick"
      @sort-change="handleSortChange"
      @mousemove="handleMouseMove"
      @cell-mouse-enter="handleRowMouseEnter"
      @cell-mouse-leave="handleRowMouseLeave"
    >
      <el-table-column width="44" align="center">
        <template #header>
          <el-checkbox :model-value="allSelected" @change="toggleAll" />
        </template>
        <template #default="{ row }">
          <el-checkbox v-model="selectedKeys" :value="row.key" />
        </template>
      </el-table-column>
      <el-table-column column-key="fileName" prop="fileName" label="文件" min-width="180" sortable show-overflow-tooltip :tooltip-props="{ placement: 'right' }" />
      <el-table-column column-key="kind" prop="kind" label="类型" width="86" sortable>
        <template #default="{ row }">{{ elementKindText(row.element.kind) }}</template>
      </el-table-column>
      <el-table-column column-key="detectedText" prop="detectedText" label="检测文字" min-width="180" sortable show-overflow-tooltip :tooltip-props="{ placement: 'right' }">
        <template #default="{ row }">{{ row.element.detectedText || '-' }}</template>
      </el-table-column>
      <el-table-column column-key="pageStart" prop="pageStart" label="页段" width="90" sortable>
        <template #default="{ row }">{{ row.element.pageStart }}-{{ row.element.pageEnd }}</template>
      </el-table-column>
      <el-table-column column-key="decision" prop="decision" label="处理" width="100" sortable>
        <template #default="{ row }">
          <el-tag :type="decisionTagType(row.element.decision)" size="small">
            {{ elementDecisionText(row.element.decision) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="编辑后" min-width="180">
        <template #default="{ row }">
          <el-input
            v-if="row.element.decision === 'edit'"
            v-model="row.element.editedText"
            size="small"
            @change="emitChange(row)"
          />
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="170" fixed="right">
        <template #default="{ row }">
          <el-button link size="small" type="primary" @click.stop="previewRow(row)">预览</el-button>
          <el-button link size="small" @click.stop="selectBySequence(row)">同序列</el-button>
          <el-button v-if="row.element.decision !== 'keep'" link size="small" @click.stop="setDecision(row, 'keep')">
            取消
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <div
      v-if="shiftHeld && hoverRowIndex >= 0"
      class="shift-range-tooltip"
      :style="{ left: tooltipPos.x + 12 + 'px', top: tooltipPos.y + 12 + 'px' }"
    >选取到这</div>
    <p class="hint-text">点击行或勾选框切换选中，点击表格外取消全部选择</p>
    <template #footer>
      <el-button @click="visibleModel = false">完成</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { compareDetectedTextRows, elementDecisionText, elementKindText } from '../composables/existingPdfElements.js'
import { naturalCompare } from '../composables/useEvidencePdfSession.js'

const props = defineProps({
  visible: { type: Boolean, required: true },
  rows: { type: Array, default: () => [] },
  filter: { type: String, default: 'all' },
})
const emit = defineEmits(['update:visible', 'change', 'preview', 'jump-to-settings'])
const selectedKeys = ref([])
const visibleModel = computed({ get: () => props.visible, set: (value) => emit('update:visible', value) })
const fileFilter = ref('')
const sortState = ref({ prop: '', order: '' })
// Shift+click range selection state
const lastClickedIndex = ref(-1)
const shiftHeld = ref(false)
const hoverRowIndex = ref(-1)
const tooltipPos = ref({ x: 0, y: 0 })

// Global shift key tracking
function onKeyDown(e) { if (e.key === 'Shift') shiftHeld.value = true }
function onKeyUp(e) { if (e.key === 'Shift') shiftHeld.value = false }
window.addEventListener('keydown', onKeyDown)
window.addEventListener('keyup', onKeyUp)
onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
})
const fileNames = computed(() => [...new Set(props.rows.map(r => r.fileName))].sort())
const KIND_ORDER = { header: 0, footerText: 1, pageNumber: 2 }
function handleSortChange({ prop, order }) {
  sortState.value = { prop: prop || '', order: order || '' }
}
function getColumnValue(row, prop) {
  switch (prop) {
    case 'fileName': return row.fileName || ''
    case 'kind': return row.element.kind || ''
    case 'detectedText': return row.element.detectedText || ''
    case 'pageStart': return row.element.pageStart || 0
    case 'source': return row.element.source || ''
    case 'decision': return row.element.decision || ''
    default: return ''
  }
}
// Sort priority within each file/kind group: undecided > lowConfidence > decided
function decisionSortPriority(row) {
  if (!row.element.decision) {
    return row.lowConfidence ? 1 : 0   // undecided → top, low confidence just below
  }
  return 2                              // already decided (keep/ignore/delete/edit) → bottom
}
const filteredRows = computed(() => {
  let rows = props.rows
  if (props.filter !== 'all') {
    if (props.filter === 'delete' || props.filter === 'edit') {
      rows = rows.filter((row) => row.element.decision === props.filter)
    } else {
      rows = rows.filter((row) => row.element.kind === props.filter)
    }
  }
  if (fileFilter.value) rows = rows.filter(row => row.fileName === fileFilter.value)
  const { prop, order } = sortState.value
  if (prop && order) {
    const direction = order === 'descending' ? -1 : 1
    return [...rows].sort((a, b) => {
      const result = prop === 'detectedText'
        ? compareDetectedTextRows(a, b)
        : naturalCompare(getColumnValue(a, prop), getColumnValue(b, prop))
      return result === 0 ? 0 : result * direction
    })
  }
  // Default sort: by fileName → by kind → by decision priority → by pageStart
  return [...rows].sort((a, b) => {
    const fa = a.fileName || '', fb = b.fileName || ''
    if (fa !== fb) return fa.localeCompare(fb)
    const ka = KIND_ORDER[a.element.kind] ?? 9, kb = KIND_ORDER[b.element.kind] ?? 9
    if (ka !== kb) return ka - kb
    const pa = decisionSortPriority(a), pb = decisionSortPriority(b)
    if (pa !== pb) return pa - pb
    return (a.element.pageStart || 0) - (b.element.pageStart || 0)
  })
})
const allSelected = computed(
  () => filteredRows.value.length > 0 && filteredRows.value.every((row) => selectedKeys.value.includes(row.key)),
)

watch(
  () => [props.visible, props.filter],
  () => {
    selectedKeys.value = []
    lastClickedIndex.value = -1
  },
)

function rowClassName({ row, rowIndex }) {
  const classes = []
  if (row.lowConfidence) classes.push('low-confidence-row')
  if (shiftHeld.value && hoverRowIndex.value === rowIndex) classes.push('shift-cursor')
  return classes.join(' ')
}
function toggleAll(value) {
  if (value) {
    // Select all except low-confidence rows (they must be explicitly chosen)
    selectedKeys.value = filteredRows.value
      .filter((row) => !row.lowConfidence)
      .map((row) => row.key)
  } else {
    selectedKeys.value = []
  }
}
function selectAll() {
  toggleAll(true)
}
function selectUndecided() {
  selectedKeys.value = filteredRows.value
    .filter((row) => !row.element.decision)
    .map((row) => row.key)
}
function clearSelection() {
  selectedKeys.value = []
  lastClickedIndex.value = -1
}
function selectBySequence(row) {
  const { kind, detectedText } = row.element
  const fileName = row.fileName || row.file?.name
  if (kind === 'pageNumber') {
    // Select all page numbers from same file whose page ranges form a continuous sequence
    const filePageNumbers = filteredRows.value
      .filter(r => r.element.kind === 'pageNumber' && r.fileName === fileName)
      .sort((a, b) => a.element.pageStart - b.element.pageStart)
    // Build connected groups: pages are "connected" if ranges touch or overlap
    const groups = []
    let current = []
    for (const r of filePageNumbers) {
      if (current.length === 0 || r.element.pageStart <= current[current.length - 1].element.pageEnd + 1) {
        current.push(r)
      } else {
        groups.push(current)
        current = [r]
      }
    }
    if (current.length) groups.push(current)
    // Find the group containing the clicked row
    const group = groups.find(g => g.some(r => r.key === row.key))
    if (group) selectedKeys.value = group.map(r => r.key)
  } else {
    // For headers/footers: select all with same text from same file
    selectedKeys.value = filteredRows.value
      .filter(r => r.element.kind === kind && r.fileName === fileName && r.element.detectedText === detectedText)
      .map(r => r.key)
  }
}
function invertSelection() {
  const selected = new Set(selectedKeys.value)
  selectedKeys.value = filteredRows.value.filter((row) => !selected.has(row.key)).map((row) => row.key)
}
function handleRowClick(row, _column, event) {
  const key = row?.key
  if (!key) return
  const target = event?.target

  const currentIndex = filteredRows.value.findIndex(r => r.key === key)
  if (currentIndex < 0) return

  // Shift+click 仅在勾选框区域触发范围选择
  if (event.shiftKey && lastClickedIndex.value >= 0) {
    if (target && (target.closest('.el-checkbox') || target.closest('.el-checkbox__input'))) {
      const start = Math.min(lastClickedIndex.value, currentIndex)
      const end = Math.max(lastClickedIndex.value, currentIndex)
      const rangeKeys = filteredRows.value.slice(start, end + 1).map(r => r.key)
      const selectedSet = new Set(selectedKeys.value)
      rangeKeys.forEach(k => selectedSet.add(k))
      selectedKeys.value = [...selectedSet]
      return
    }
  }

  // 更新锚点
  lastClickedIndex.value = currentIndex

  // 勾选框区域交给 el-checkbox 自身 v-model 切换
  if (target && (target.closest('.el-checkbox') || target.closest('.el-checkbox__input'))) return

  // 行点击：preventDefault 防文字选中 + 手动 toggle
  event.preventDefault()
  const idx = selectedKeys.value.indexOf(key)
  if (idx >= 0) {
    selectedKeys.value = selectedKeys.value.filter(k => k !== key)
  } else {
    selectedKeys.value = [...selectedKeys.value, key]
  }
}
function handleMouseMove(event) {
  tooltipPos.value = { x: event.clientX, y: event.clientY }
}
function handleRowMouseEnter(_row, _column, _cell, _event) {
  const rowIndex = filteredRows.value.findIndex(r => r.key === _row.key)
  if (rowIndex >= 0) hoverRowIndex.value = rowIndex
}
function handleRowMouseLeave() {
  hoverRowIndex.value = -1
}
function applyDecision(decision) {
  const selected = new Set(selectedKeys.value)
  filteredRows.value.filter((row) => selected.has(row.key)).forEach((row) => setDecision(row, decision))
  selectedKeys.value = []
}
function setDecision(row, decision) {
  row.element.decision = decision
  if (decision === 'edit' && row.element.kind === 'pageNumber') {
    const template = String(row.element.normalizedText || '')
    row.element.editedText = template.includes('{page}') || template.includes('{roman-page}')
      ? template
      : row.element.detectedText
  } else if (decision !== 'edit') {
    row.element.editedText = row.element.detectedText
  }
  emit('change', row)
}
function previewRow(row) {
  visibleModel.value = false
  nextTick(() => emit('preview', row))
}
function emitChange(row) {
  emit('change', row)
}
function decisionTagType(decision) {
  return { delete: 'danger', edit: 'warning', ignore: 'info', keep: 'success' }[decision] || 'warning'
}
</script>

<style scoped>
.decision-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}
.hint-text {
  margin: 8px 0 0;
  font-size: 12px;
  color: var(--docsy-text-muted, #999);
}
:deep(.low-confidence-row) {
  background: var(--docsy-surface-muted, #fafafa);
}
:deep(.low-confidence-row td:first-child) {
  border-left: 3px solid var(--docsy-text-muted, #c0c4cc);
}
:deep(.low-confidence-row:hover) {
  background: var(--docsy-surface-muted-hover, #f0f0f0);
}
:deep(.el-table td) {
  user-select: none;
}
:deep(.shift-cursor) {
  cursor: crosshair;
}
.shift-range-tooltip {
  position: fixed;
  z-index: 99999;
  background: var(--docsy-primary, #409eff);
  color: white;
  padding: 3px 10px;
  border-radius: var(--docsy-radius);
  font-size: 12px;
  pointer-events: none;
  white-space: nowrap;
}
</style>
