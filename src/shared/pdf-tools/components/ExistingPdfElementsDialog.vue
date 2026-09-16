<template>
  <el-dialog
    v-model="visibleModel"
    :title="cleanupOnly ? '拆分时清除页眉、页脚和页码' : '确认原有页眉、页脚和页码'"
    width="min(1080px, 94vw)"
    append-to-body
    @click.self="clearSelection"
  >
    <div class="decision-toolbar">
      <el-select v-if="fileOptions.length > 1" v-model="fileFilter" size="small" placeholder="全部文件" clearable style="width: 200px">
        <el-option v-for="file in fileOptions" :key="file.key" :label="file.label" :value="file.key" />
      </el-select>
      <el-button size="small" @click="selectAll">全选</el-button>
      <el-button size="small" @click="invertSelection">反选</el-button>
      <el-button v-if="!cleanupOnly" size="small" @click="selectUndecided">选择未确认项</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" @click="clearSelection">取消选择</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" @click="applyDecision('keep')">保留</el-button>
      <el-button v-if="!cleanupOnly" size="small" :disabled="!selectedKeys.length" @click="applyDecision('ignore')">忽略识别</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" type="danger" @click="applyDecision('delete')"
        >标记删除</el-button
      >
      <el-button v-if="!cleanupOnly" size="small" :disabled="!selectedKeys.length" type="primary" @click="applyDecision('edit')"
        >标记编辑</el-button
      >
    </div>
    <div class="element-groups">
    <section v-for="group in fileGroups" :key="group.key" class="element-file-group" :aria-label="group.duplicateName ? group.key : group.fileName">
    <div v-if="fileOptions.length > 1" class="file-group-header">
      <div class="file-group-title">
        <strong :title="group.key">{{ group.fileName }}</strong>
        <span>{{ group.rows.length }} 项</span>
        <small v-if="group.duplicateName" :title="group.key">{{ group.key }}</small>
      </div>
      <el-button size="small" @click="selectFileGroup(group)">选择本文件</el-button>
    </div>
    <el-table
      :data="group.rows"
      border
      size="small"
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
          <el-checkbox :model-value="isGroupSelected(group)" :indeterminate="isGroupPartlySelected(group)" :aria-label="'选择 ' + group.fileName + ' 中的全部元素'" @change="value => toggleFileGroup(group, value)" />
        </template>
        <template #default="{ row }">
          <el-checkbox
            :model-value="selectedKeys.includes(row.key)"
            @click.stop.prevent="handleCheckboxClick(row, $event)"
          />
        </template>
      </el-table-column>
      <el-table-column column-key="kind" prop="kind" label="类型" width="86" sortable="custom">
        <template #default="{ row }">{{ elementKindText(row.element.kind) }}</template>
      </el-table-column>
      <el-table-column
        column-key="detectedText"
        prop="detectedText"
        label="检测文字"
        min-width="220"
        sortable="custom"
        show-overflow-tooltip
        :tooltip-props="{ placement: 'right' }"
      >
        <template #default="{ row }">{{ row.element.detectedText || '-' }}</template>
      </el-table-column>
      <el-table-column column-key="pageStart" prop="pageStart" label="出现页段" width="96" sortable="custom">
        <template #default="{ row }">{{ row.element.pageStart }}-{{ row.element.pageEnd }}</template>
      </el-table-column>
      <el-table-column column-key="count" prop="count" label="出现次数" width="96" sortable="custom">
        <template #default="{ row }">{{ row.element.count > 0 ? row.element.count : '—' }}</template>
      </el-table-column>
      <el-table-column column-key="decision" prop="decision" label="处理" width="90" sortable="custom">
        <template #default="{ row }">
          <el-tag :type="decisionTagType(row.element.decision)" size="small">
            {{ elementDecisionText(row.element.decision) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column v-if="!cleanupOnly && filteredRows.some(row => row.element.decision === 'edit')" label="编辑后" min-width="180">
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
      <el-table-column v-if="fileOptions.length <= 1" column-key="fileName" prop="fileName" label="来源文件" min-width="130" sortable="custom" show-overflow-tooltip>
        <template #default="{ row }"><span :title="elementRowFileKey(row)">{{ row.fileName }}</span></template>
      </el-table-column>
      <el-table-column label="操作" width="150" fixed="right">
        <template #default="{ row }">
          <el-button link size="small" type="primary" @click.stop="previewRow(row)">预览</el-button>
          <el-button link size="small" @click.stop="selectBySequence(row)">同序列</el-button>
          <el-button v-if="row.element.decision !== 'keep'" link size="small" @click.stop="setDecision(row, 'keep')">
            取消
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    </section>
    <el-empty v-if="!fileGroups.length" description="没有匹配的元素" :image-size="64" />
    </div>
    <div
      v-if="shiftHeld && hoverRowIndex >= 0"
      class="shift-range-tooltip"
      :style="{ left: tooltipPos.x + 12 + 'px', top: tooltipPos.y + 12 + 'px' }"
    >
      选取到这
    </div>
    <p class="hint-text">已选 {{ selectedKeys.length }} 项<span v-if="cleanupOnly"> · 选择后点击“标记删除”，仅在拆分输出时生效，原件不变</span><span v-else> · 点击行或勾选框切换选中，点击表格外取消全部选择</span></p>
    <template #footer>
      <el-button @click="visibleModel = false">完成</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import {
  compareDetectedTextRows,
  elementDecisionText,
  elementKindText,
  mergeRangeSelection,
  elementRowFileKey,
  sameSequenceRowKeys,
  groupElementRowsByFile,
} from '../composables/existingPdfElements.js'
import { naturalCompare } from '../composables/useEvidencePdfSession.js'

const props = defineProps({
  visible: { type: Boolean, required: true },
  rows: { type: Array, default: () => [] },
  filter: { type: String, default: 'all' },
  cleanupOnly: { type: Boolean, default: false },
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
function onKeyDown(e) {
  if (e.key === 'Shift') shiftHeld.value = true
}
function onKeyUp(e) {
  if (e.key === 'Shift') shiftHeld.value = false
}
window.addEventListener('keydown', onKeyDown)
window.addEventListener('keyup', onKeyUp)
onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
})
const fileOptions = computed(() => {
  const unique = [...new Map(props.rows.map(row => [elementRowFileKey(row), row])).values()]
  return unique.map(row => ({
    key: elementRowFileKey(row),
    label: unique.filter(other => other.fileName === row.fileName).length > 1 ? elementRowFileKey(row) : row.fileName,
  }))
})
const KIND_ORDER = { header: 0, footerText: 1, pageNumber: 2 }
function handleSortChange({ prop, order }) {
  sortState.value = { prop: prop || '', order: order || '' }
  lastClickedIndex.value = -1
}
function getColumnValue(row, prop) {
  switch (prop) {
    case 'fileName':
      return row.fileName || ''
    case 'kind':
      return row.element.kind || ''
    case 'detectedText':
      return row.element.detectedText || ''
    case 'pageStart':
      return row.element.pageStart || 0
    case 'count':
      return row.element.count || 0
    case 'source':
      return row.element.source || ''
    case 'decision':
      return row.element.decision || ''
    default:
      return ''
  }
}
function decisionSortPriority(row) {
  if (!row.element.decision) {
    return row.lowConfidence ? 1 : 0 // undecided → top, low confidence just below
  }
  return 2 // already decided (keep/ignore/delete/edit) → bottom
}
const sortedRows = computed(() => {
  let rows = props.rows
  if (props.filter !== 'all') {
    if (props.filter === 'delete' || props.filter === 'edit') {
      rows = rows.filter((row) => row.element.decision === props.filter)
    } else {
      rows = rows.filter((row) => row.element.kind === props.filter)
    }
  }
  if (fileFilter.value) rows = rows.filter((row) => elementRowFileKey(row) === fileFilter.value)
  const { prop, order } = sortState.value
  if (prop && order) {
    const direction = order === 'descending' ? -1 : 1
    return [...rows].sort((a, b) => {
      const result =
        prop === 'detectedText'
          ? compareDetectedTextRows(a, b)
          : naturalCompare(getColumnValue(a, prop), getColumnValue(b, prop))
      return result === 0 ? 0 : result * direction
    })
  }
  return [...rows].sort((a, b) => {
    const ka = KIND_ORDER[a.element.kind] ?? 9,
      kb = KIND_ORDER[b.element.kind] ?? 9
    if (ka !== kb) return ka - kb
    const pa = decisionSortPriority(a),
      pb = decisionSortPriority(b)
    if (pa !== pb) return pa - pb
    return compareDetectedTextRows(a, b) || elementRowFileKey(a).localeCompare(elementRowFileKey(b)) || (a.element.pageStart || 0) - (b.element.pageStart || 0)
  })
})
const fileGroups = computed(() => {
  const groups = groupElementRowsByFile(sortedRows.value)
  return groups.map(group => ({
    ...group,
    duplicateName: fileOptions.value.some(file => file.key === group.key && file.label !== group.fileName),
  }))
})
const filteredRows = computed(() => fileGroups.value.flatMap(group => group.rows))

function isGroupSelected(group) {
  return group.rows.every(row => selectedKeys.value.includes(row.key))
}
function isGroupPartlySelected(group) {
  return !isGroupSelected(group) && group.rows.some(row => selectedKeys.value.includes(row.key))
}
function toggleFileGroup(group, value) {
  const selected = new Set(selectedKeys.value)
  for (const row of group.rows) {
    if (!value) selected.delete(row.key)
    else if (!row.lowConfidence) selected.add(row.key)
  }
  selectedKeys.value = [...selected]
  lastClickedIndex.value = -1
}
function selectFileGroup(group) {
  selectedKeys.value = group.rows.filter(row => !row.lowConfidence).map(row => row.key)
  lastClickedIndex.value = -1
}

watch(
  () => [props.visible, props.filter, fileFilter.value],
  () => {
    selectedKeys.value = []
    lastClickedIndex.value = -1
  },
)

watch(fileOptions, options => {
  if (!options.some(file => file.key === fileFilter.value)) fileFilter.value = ''
})

function rowClassName({ row }) {
  const classes = []
  if (row.lowConfidence) classes.push('low-confidence-row')
  if (shiftHeld.value && filteredRows.value[hoverRowIndex.value]?.key === row.key) classes.push('shift-cursor')
  return classes.join(' ')
}
function toggleAll(value) {
  if (value) {
    // Select all except low-confidence rows (they must be explicitly chosen)
    selectedKeys.value = filteredRows.value.filter((row) => !row.lowConfidence).map((row) => row.key)
  } else {
    selectedKeys.value = []
  }
}
function selectAll() {
  toggleAll(true)
}
function selectUndecided() {
  selectedKeys.value = filteredRows.value.filter((row) => !row.element.decision).map((row) => row.key)
}
function clearSelection() {
  selectedKeys.value = []
  lastClickedIndex.value = -1
}
function selectBySequence(row) {
  selectedKeys.value = sameSequenceRowKeys(filteredRows.value, row)
}
function invertSelection() {
  const selected = new Set(selectedKeys.value)
  selectedKeys.value = filteredRows.value.filter((row) => !selected.has(row.key)).map((row) => row.key)
}
function handleRowClick(row, _column, event) {
  const key = row?.key
  if (!key) return
  const target = event?.target

  const currentIndex = filteredRows.value.findIndex((r) => r.key === key)
  if (currentIndex < 0) return

  // Shift+点击行的任意位置都扩展选区，且始终包含终点。
  if (event?.shiftKey && lastClickedIndex.value >= 0) {
    event.preventDefault()
    selectedKeys.value = mergeRangeSelection(
      selectedKeys.value,
      filteredRows.value,
      lastClickedIndex.value,
      currentIndex,
    )
    return
  }

  // 更新锚点
  lastClickedIndex.value = currentIndex

  // 勾选框区域交给 el-checkbox 自身 v-model 切换
  if (target && (target.closest('.el-checkbox') || target.closest('.el-checkbox__input'))) return

  // 行点击：preventDefault 防文字选中 + 手动 toggle
  event.preventDefault()
  const idx = selectedKeys.value.indexOf(key)
  if (idx >= 0) {
    selectedKeys.value = selectedKeys.value.filter((k) => k !== key)
  } else {
    selectedKeys.value = [...selectedKeys.value, key]
  }
}

function handleCheckboxClick(row, event) {
  const currentIndex = filteredRows.value.findIndex((item) => item.key === row?.key)
  if (currentIndex < 0) return

  if ((event?.shiftKey || shiftHeld.value) && lastClickedIndex.value >= 0) {
    selectedKeys.value = mergeRangeSelection(
      selectedKeys.value,
      filteredRows.value,
      lastClickedIndex.value,
      currentIndex,
    )
    return
  }

  const selected = new Set(selectedKeys.value)
  if (selected.has(row.key)) selected.delete(row.key)
  else selected.add(row.key)
  selectedKeys.value = [...selected]
  lastClickedIndex.value = currentIndex
}
function handleMouseMove(event) {
  tooltipPos.value = { x: event.clientX, y: event.clientY }
}
function handleRowMouseEnter(_row, _column, _cell, _event) {
  const rowIndex = filteredRows.value.findIndex((r) => r.key === _row.key)
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
  if (props.cleanupOnly && !['keep', 'delete'].includes(decision)) return
  row.element.decision = decision
  if (decision === 'edit' && row.element.kind === 'pageNumber') {
    const template = String(row.element.normalizedText || '')
    row.element.editedText =
      template.includes('{page}') || template.includes('{roman-page}') ? template : row.element.detectedText
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
.element-groups {
  max-height: 58vh;
  overflow: auto;
}
.element-file-group + .element-file-group {
  margin-top: 20px;
}
.file-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-bottom: 0;
  background: var(--docsy-surface-muted);
}
.file-group-title {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 4px 10px;
  min-width: 0;
}
.file-group-title strong,
.file-group-title small {
  overflow-wrap: anywhere;
}
.file-group-title span,
.file-group-title small {
  color: var(--docsy-text-muted);
}
.file-group-title small {
  flex-basis: 100%;
}
.decision-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}
.decision-toolbar :deep(.el-button) {
  margin-left: 0;
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
