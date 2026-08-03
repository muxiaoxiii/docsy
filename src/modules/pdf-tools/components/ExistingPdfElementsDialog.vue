<template>
  <el-dialog v-model="visibleModel" title="确认原有页眉、页脚和页码" width="min(1080px, 94vw)" append-to-body>
    <div class="decision-toolbar">
      <el-button size="small" @click="selectAll">全选</el-button>
      <el-button size="small" @click="invertSelection">反选</el-button>
      <el-button size="small" @click="applyDecision('keep')">保留</el-button>
      <el-button size="small" @click="applyDecision('ignore')">忽略识别</el-button>
      <el-button size="small" type="danger" @click="applyDecision('delete')">标记删除</el-button>
      <el-button size="small" type="primary" @click="applyDecision('edit')">标记编辑</el-button>
    </div>
    <el-table :data="filteredRows" border size="small" max-height="58vh" row-key="key">
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
      <el-table-column label="操作" width="115" fixed="right">
        <template #default="{ row }">
          <el-button link size="small" type="primary" @click="$emit('preview', row)">预览</el-button>
          <el-button v-if="row.element.decision !== 'keep'" link size="small" @click="setDecision(row, 'keep')">
            取消处理
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <template #footer>
      <el-button @click="visibleModel = false">完成</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { elementDecisionText, elementKindText } from '../composables/existingPdfElements.js'

const props = defineProps({
  visible: { type: Boolean, required: true },
  rows: { type: Array, default: () => [] },
  filter: { type: String, default: 'all' },
})
const emit = defineEmits(['update:visible', 'change', 'preview'])
const selectedKeys = ref([])
const visibleModel = computed({ get: () => props.visible, set: (value) => emit('update:visible', value) })
const filteredRows = computed(() => {
  if (props.filter === 'all') return props.rows
  if (props.filter === 'delete' || props.filter === 'edit') {
    return props.rows.filter((row) => row.element.decision === props.filter)
  }
  return props.rows.filter((row) => row.element.kind === props.filter)
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
function invertSelection() {
  const selected = new Set(selectedKeys.value)
  selectedKeys.value = filteredRows.value.filter((row) => !selected.has(row.key)).map((row) => row.key)
}
function applyDecision(decision) {
  const selected = new Set(selectedKeys.value)
  filteredRows.value.filter((row) => selected.has(row.key)).forEach((row) => setDecision(row, decision))
}
function setDecision(row, decision) {
  row.element.decision = decision
  if (decision !== 'edit') row.element.editedText = row.element.detectedText
  emit('change', row)
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
</style>
