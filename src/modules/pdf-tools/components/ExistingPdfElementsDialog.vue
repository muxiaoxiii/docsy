<template>
  <el-dialog v-model="visibleModel" title="确认原有页眉、页脚和页码" width="min(1080px, 94vw)" append-to-body>
    <div class="decision-toolbar">
      <el-select v-model="fileFilter" size="small" placeholder="全部文件" clearable style="width: 200px">
        <el-option v-for="f in fileNames" :key="f" :label="f" :value="f" />
      </el-select>
      <el-button size="small" @click="selectAll">全选</el-button>
      <el-button size="small" @click="invertSelection">反选</el-button>
      <el-button size="small" :disabled="!selectedKeys.length" @click="clearSelection">取消选择</el-button>
      <el-button size="small" @click="selectByKind('pageNumber')">选中全部页码</el-button>
      <el-button size="small" @click="selectByKind('header')">选中全部页眉</el-button>
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
      @row-contextmenu="handleRowRightClick"
      @row-click="handleRowClick"
    >
      <el-table-column width="44">
        <template #header>
          <el-checkbox :model-value="allSelected" @change="toggleAll" />
        </template>
        <template #default="{ row }">
          <el-checkbox v-model="selectedKeys" :value="row.key" />
        </template>
      </el-table-column>
      <el-table-column prop="fileName" label="文件" min-width="180" show-overflow-tooltip />
      <el-table-column label="类型" width="86">
        <template #default="{ row }">{{ elementKindText(row.element.kind) }}</template>
      </el-table-column>
      <el-table-column label="检测文字" min-width="180" show-overflow-tooltip>
        <template #default="{ row }">{{ row.element.detectedText || '-' }}</template>
      </el-table-column>
      <el-table-column label="页段" width="90">
        <template #default="{ row }">{{ row.element.pageStart }}-{{ row.element.pageEnd }}</template>
      </el-table-column>
      <el-table-column label="来源" width="105">
        <template #default="{ row }">{{ row.element.source === 'artifact' ? '标准结构' : '页面文本' }}</template>
      </el-table-column>
      <el-table-column label="处理" width="100">
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
          <el-button link size="small" type="primary" @click="previewRow(row)">预览</el-button>
          <el-button link size="small" @click="selectBySequence(row)">同序列</el-button>
          <el-button v-if="row.element.decision !== 'keep'" link size="small" @click="setDecision(row, 'keep')">
            取消
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <p class="hint-text">右键点击行可切换勾选状态</p>
    <template #footer>
      <el-button @click="visibleModel = false">完成</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, nextTick, ref, watch } from 'vue'
import { elementDecisionText, elementKindText } from '../composables/existingPdfElements.js'

const props = defineProps({
  visible: { type: Boolean, required: true },
  rows: { type: Array, default: () => [] },
  filter: { type: String, default: 'all' },
})
const emit = defineEmits(['update:visible', 'change', 'preview', 'jump-to-settings'])
const selectedKeys = ref([])
const visibleModel = computed({ get: () => props.visible, set: (value) => emit('update:visible', value) })
const fileFilter = ref('')
const fileNames = computed(() => [...new Set(props.rows.map(r => r.fileName))].sort())
const KIND_ORDER = { header: 0, footerText: 1, pageNumber: 2 }
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
  // Sort: by fileName → by kind (header < footer < pageNumber) → by pageStart
  return [...rows].sort((a, b) => {
    const fa = a.fileName || '', fb = b.fileName || ''
    if (fa !== fb) return fa.localeCompare(fb)
    const ka = KIND_ORDER[a.element.kind] ?? 9, kb = KIND_ORDER[b.element.kind] ?? 9
    if (ka !== kb) return ka - kb
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
  },
)

function toggleAll(value) {
  selectedKeys.value = value ? filteredRows.value.map((row) => row.key) : []
}
function selectAll() {
  toggleAll(true)
}
function clearSelection() {
  selectedKeys.value = []
}
function selectBySequence(row) {
  const { kind, pageStart, pageEnd, detectedText, fileName } = row.element
  if (kind === 'pageNumber') {
    // Select all page numbers from same file whose page ranges form a continuous sequence
    const filePageNumbers = filteredRows.value
      .filter(r => r.element.kind === 'pageNumber' && r.element.fileName === fileName)
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
      .filter(r => r.element.kind === kind && r.element.fileName === fileName && r.element.detectedText === detectedText)
      .map(r => r.key)
  }
}
function selectByKind(kind) {
  selectedKeys.value = filteredRows.value.filter(row => row.element.kind === kind).map(row => row.key)
}
function invertSelection() {
  const selected = new Set(selectedKeys.value)
  selectedKeys.value = filteredRows.value.filter((row) => !selected.has(row.key)).map((row) => row.key)
}
function handleRowRightClick(_row, _column, event) {
  event.preventDefault()
  const key = _row.key
  const idx = selectedKeys.value.indexOf(key)
  if (idx >= 0) {
    selectedKeys.value = selectedKeys.value.filter(k => k !== key)
  } else {
    selectedKeys.value = [...selectedKeys.value, key]
  }
}
function applyDecision(decision) {
  const selected = new Set(selectedKeys.value)
  filteredRows.value.filter((row) => selected.has(row.key)).forEach((row) => setDecision(row, decision))
  selectedKeys.value = []
}
function setDecision(row, decision) {
  row.element.decision = decision
  if (decision !== 'edit') row.element.editedText = row.element.detectedText
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
  return { delete: 'danger', edit: 'warning', ignore: 'info', keep: 'success' }[decision] || 'info'
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
</style>
