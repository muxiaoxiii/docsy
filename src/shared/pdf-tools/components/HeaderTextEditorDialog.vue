<template>
  <el-dialog
    v-model="visible"
    class="header-text-editor-dialog"
    title="逐文件编辑页眉"
    width="min(1080px, 95vw)"
    append-to-body
    :close-on-click-modal="false"
  >
    <div class="editor-intro">
      <div>这里显示当前规则生成的完整页眉。可直接修改，也可从文件名中选择文字作为页眉。</div>
      <div v-if="hasExceptions" class="editor-notice">
        已设置页眉分段或例外：此处修改基本页眉，例外页面仍按例外设置处理。
      </div>
    </div>
    <div class="batch-heading">
      <el-button size="small" @click="batchOpen = !batchOpen">{{ batchOpen ? '收起批量整理' : '批量整理…' }}</el-button>
      <span>批量替换文字，或按分隔符截取当前页眉／文件名。</span>
    </div>
    <section v-if="batchOpen" class="batch-panel" aria-label="批量整理页眉">
      <div class="batch-line">
        <el-radio-group v-model="batch.mode" size="small" aria-label="批量整理方式">
          <el-radio-button value="replace">替换文字</el-radio-button>
          <el-radio-button value="extract">按分隔符截取</el-radio-button>
        </el-radio-group>
        <template v-if="batch.mode === 'extract'">
          <el-select v-model="batch.source" aria-label="批量整理来源" class="batch-source">
            <el-option label="当前页眉" value="header" />
            <el-option label="文件名" value="filename" />
          </el-select>
          <label
            >分隔符
            <el-input v-model="batch.separator" aria-label="分隔符" placeholder="例如 -" class="batch-separator"
          /></label>
          <label
            >第
            <SplitRangeBoundaryInput :value="batch.start" :max="99" label="起始片段" @commit="batch.start = $event" />
            段</label
          >
          <label
            >到第
            <SplitRangeBoundaryInput
              v-if="!batch.throughEnd"
              :value="batch.end"
              :max="99"
              label="结束片段"
              @commit="batch.end = $event"
            /><span v-else>最后</span> 段</label
          >
          <el-checkbox v-model="batch.throughEnd">取到末尾</el-checkbox>
          <el-checkbox v-model="batch.removeOrder">去掉开头排序号（如 9.）</el-checkbox>
        </template>
      </div>
      <div class="batch-line">
        <label>查找 <el-input v-model="batch.find" aria-label="查找文字" placeholder="要替换的文字" /></label>
        <label>替换为 <el-input v-model="batch.replace" aria-label="替换文字" placeholder="留空则删除" /></label>
        <el-button
          size="small"
          @click="useComparisonNames"
          >证据文件 → 对比文件</el-button
        >
        <el-button
          type="primary"
          size="small"
          plain
          :disabled="batch.mode === 'replace' && !batch.find"
          @click="applyBatch"
          >应用到全部</el-button
        >
      </div>
      <div v-if="batchError" class="editor-error" role="alert">{{ batchError }}</div>
    </section>
    <div v-if="!rows.length" class="editor-empty">请先添加证据文件，再编辑页眉。</div>
    <div v-else class="header-editor-list">
      <article v-for="(row, rowIndex) in rows" :key="row.key" class="header-editor-row">
        <div class="header-editor-file">
          <span class="file-index">{{ row.index + 1 }}</span>
          <div class="file-details">
            <div class="file-name" :title="row.name">{{ row.name }}</div>
            <div class="rule-text" :title="row.original">当前页眉：{{ row.original || '（空）' }}</div>
          </div>
        </div>
        <div class="header-editor-input">
          <el-input
            type="textarea"
            :autosize="{ minRows: 1, maxRows: 4 }"
            :model-value="row.value"
            :aria-label="`${row.name} 的页眉`"
            @update:model-value="updateText(row, $event)"
          />
          <el-button size="small" @click="toggleSelection(row)">{{
            row.selectionOpen ? '收起文件名拆字' : '从文件名选字'
          }}</el-button>
          <el-button
            v-if="row.value !== row.base || (row.hadOverride && !row.resetToRule)"
            size="small"
            text
            @click="restoreRow(row)"
            >恢复规则</el-button
          >
        </div>
        <div v-if="row.value !== row.original && !row.value.trim()" class="editor-error" role="alert">
          页眉不能为空；可恢复规则或输入文字。
        </div>
        <div v-if="row.selectionOpen" class="token-picker">
          <div v-if="row.selectionError" class="editor-error" role="alert">{{ row.selectionError }}</div>
          <div class="token-toolbar">
            <span>选择文件名中的文字，选中内容将替换当前页眉</span>
            <el-tooltip content="切换拆分方式会清空选择，已编辑的页眉保持不变。" placement="top">
              <el-radio-group
              :model-value="row.granularity"
              size="small"
              :aria-label="`${row.name} 的拆分方式`"
              @update:model-value="setGranularity(row, $event)"
            >
              <el-radio-button value="word">按词</el-radio-button
              ><el-radio-button value="character">逐字</el-radio-button>
              </el-radio-group>
            </el-tooltip>
            <el-button size="small" text @click="selectAll(row)">全选</el-button>
            <el-button size="small" text @click="selectNone(row)">清空选择</el-button>
            <el-tooltip content="按当前选中片段在文件名中的位置处理其他文件，请逐行核对结果。" placement="top"
              ><el-button size="small" text :disabled="!row.selected.length" @click="applyPositions(rowIndex)"
                >按位置应用到全部</el-button
              ></el-tooltip
            >
          </div>
          <div class="token-list" :aria-label="`${row.name} 的文件名片段（不含扩展名）`">
            <button
              v-for="(token, index) in row.tokens"
              :key="index"
              type="button"
              class="header-token"
              :class="{ 'is-selected': row.selected.includes(index), 'is-space': !token.trim() }"
              :aria-pressed="row.selected.includes(index)"
              :aria-label="token.trim() ? token : '空格'"
              @click="toggleToken(row, index)"
            >
              {{ token.trim() ? token : '空格' }}
            </button>
          </div>
        </div>
      </article>
    </div>
    <template #footer
      ><span class="editor-count">{{ rows.length }} 个文件 · {{ changedCount }} 项已修改</span
      ><el-button @click="visible = false">取消</el-button
      ><el-button type="primary" :disabled="!changedCount || hasInvalid" @click="apply">应用修改</el-button></template
    >
  </el-dialog>
</template>
<script setup>
import { computed, ref, watch } from 'vue'
import SplitRangeBoundaryInput from './SplitRangeBoundaryInput.vue'
import { buildHeaderTextForGroup, effectiveHeaderNumbering } from '../composables/useEvidencePdfSession.js'
import { filenameHeaderKey, filenameHeaderSource } from '../composables/filenameHeader.js'
import { joinSelectedHeaderTokens, splitHeaderTokens } from '../composables/headerTextSelection.js'

const props = defineProps({
  files: { type: Array, default: () => [] },
  group: { type: Object, default: () => null },
  numberingDefaults: { type: Object, default: () => ({ evidenceStart: 1, pageStart: 1 }) },
  dateValue: { type: String, default: '' },
  indexOffset: { type: Number, default: 0 },
  hasExceptions: Boolean,
})
const emit = defineEmits(['apply'])
const visible = ref(false),
  rows = ref([]),
  batchOpen = ref(false),
  batchError = ref('')
const newBatch = () => ({
  mode: 'replace',
  source: 'header',
  separator: '-',
  start: 1,
  end: 1,
  throughEnd: false,
  removeOrder: false,
  find: '',
  replace: '',
})
const batch = ref(newBatch())
function useComparisonNames() {
  batch.value.find = '证据文件'
  batch.value.replace = '对比文件'
}
const rowChanged = (row) => row.value !== row.original || (row.resetToRule && row.value === row.base)
const changedCount = computed(() => rows.value.filter(rowChanged).length)
const hasInvalid = computed(() => rows.value.some((row) => row.value !== row.original && !row.value.trim()))
watch(
  batch,
  () => {
    batchError.value = ''
  },
  { deep: true },
)
function open() {
  const group = props.group
  if (!group) return
  const rules = { numberingDefaults: props.numberingDefaults, headerDateValue: props.dateValue }
  const resolved = { ...group, perFileSeqStart: effectiveHeaderNumbering(group, props.numberingDefaults).evidenceStart }
  const inherited = { ...resolved, fileHeaderOverrides: {} }
  rows.value = props.files
    .map((file, offset) => {
      const index = offset + props.indexOffset,
        key = filenameHeaderKey(file)
      const original = buildHeaderTextForGroup(file, index, resolved, rules)
      return {
        key,
        index,
        name: filenameHeaderSource(file),
        fileName: filenameHeaderSource(file).replace(/\.[^.]+$/, ''),
        original,
        base: buildHeaderTextForGroup(file, index, inherited, rules),
        value: original,
        hadOverride: Object.hasOwn(group.fileHeaderOverrides || {}, key),
        resetToRule: false,
        selectionOpen: false,
        tokens: [],
        selected: [],
        selectionSource: '',
        selectionError: '',
        granularity: 'word',
      }
    })
    .filter((row) => !group.fileIds?.length || group.fileIds.includes(row.key))
  batch.value = newBatch()
  batchOpen.value = false
  batchError.value = ''
  visible.value = true
}
function updateText(row, value) {
  row.value = String(value).replace(/[\r\n]+/g, ' ')
  row.selectionOpen = false
  batchError.value = ''
}
function resetTokens(row) {
  row.tokens = splitHeaderTokens(row.selectionSource, row.granularity)
  row.selected = []
}
function toggleSelection(row) {
  row.selectionError = ''
  if (row.selectionOpen) {
    row.selectionOpen = false
    return
  }
  row.selectionSource = row.fileName
  row.granularity = 'word'
  resetTokens(row)
  row.selectionOpen = true
}
function setGranularity(row, value) {
  row.granularity = value
  resetTokens(row)
}
function updateSelection(row) {
  row.value = joinSelectedHeaderTokens(row.tokens, row.selected)
}
function selectAll(row) {
  row.selected = row.tokens.map((_, index) => index)
  updateSelection(row)
}
function selectNone(row) {
  row.selected = []
  updateSelection(row)
}
function toggleToken(row, index) {
  row.selected = row.selected.includes(index)
    ? row.selected.filter((value) => value !== index)
    : [...row.selected, index]
  updateSelection(row)
}
function restoreRow(row) {
  row.value = row.base
  row.resetToRule = row.hadOverride
  row.selectionOpen = false
  batchError.value = ''
}
function applyPositions(rowIndex) {
  const source = rows.value[rowIndex]
  if (!source?.selected.length) return
  const selected = source.selected,
    max = Math.max(...selected)
  const proposals = rows.value.map((row) => {
    const tokens = splitHeaderTokens(row.fileName, source.granularity)
    return { row, tokens }
  })
  const invalid = proposals.find((item) => item.tokens.length <= max)
  if (invalid) {
    source.selectionError = `${invalid.row.name} 的片段数量不足，未应用。可逐项选择或修改文字。`
    return
  }
  proposals.forEach(({ row, tokens }) => {
    row.selectionError = ''
    row.value = joinSelectedHeaderTokens(tokens, selected)
    row.selectionOpen = false
  })
  batchError.value = ''
}
function applyBatch() {
  const config = batch.value
  if (config.mode === 'replace' && !config.find) return
  if (config.mode === 'extract' && !config.separator) {
    batchError.value = '请填写分隔符。'
    return
  }
  const proposals = []
  for (const row of rows.value) {
    let value = config.mode === 'extract' && config.source === 'filename' ? row.fileName : row.value
    if (config.mode === 'extract') {
      if (config.removeOrder) value = value.replace(/^\s*\d{1,4}\s*[.．、]\s*/, '')
      const parts = value.split(config.separator).map((part) => part.trim())
      const start = Number(config.start),
        end = config.throughEnd ? parts.length : Number(config.end)
      if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1 || end < start || end > parts.length) {
        batchError.value = `${row.name} 只有 ${parts.length} 段，未应用。请调整截取范围。`
        return
      }
      value = parts.slice(start - 1, end).join(config.separator)
    }
    if (config.find) value = value.replaceAll(config.find, config.replace)
    if (!value.trim()) {
      batchError.value = `${row.name} 的结果为空，未应用。请调整规则。`
      return
    }
    proposals.push({ row, value })
  }
  proposals.forEach(({ row, value }) => {
    row.value = value
    row.selectionOpen = false
  })
  batchError.value = ''
}
function apply() {
  if (!changedCount.value || hasInvalid.value) return
  const overrides = { ...(props.group?.fileHeaderOverrides || {}) }
  for (const row of rows.value) {
    if (!rowChanged(row)) continue
    if (row.value === row.base) delete overrides[row.key]
    else overrides[row.key] = row.value
  }
  emit('apply', overrides)
  visible.value = false
}
defineExpose({ open })
</script>
<style scoped>
:global(.header-text-editor-dialog.el-dialog) {
  max-height: 90vh;
  margin-top: 5vh !important;
  display: flex;
  flex-direction: column;
}
:global(.header-text-editor-dialog > .el-dialog__body) {
  overflow: auto;
  min-height: 0;
}
.editor-intro {
  font-size: 13px;
  color: var(--docsy-text-muted);
  line-height: 1.6;
}
.editor-notice {
  color: var(--docsy-warning);
  margin-top: 4px;
}
.batch-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  margin: 12px 0 8px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}
.batch-panel {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
  padding: 10px;
  margin-bottom: 10px;
}
.batch-line {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 8px;
}
.batch-line:last-child {
  margin-bottom: 0;
}
.batch-line label {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
}
.batch-line label > .el-input {
  width: 120px;
}
.batch-line label > .batch-separator {
  width: 65px;
}
.batch-source {
  width: 124px;
}
.batch-line :deep(.split-boundary-input) {
  flex: 0 0 62px;
  width: 62px;
}
.editor-empty {
  padding: 30px;
  text-align: center;
  color: var(--docsy-text-muted);
}
.header-editor-list {
  max-height: min(54vh, 510px);
  overflow: auto;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
}
.header-editor-row {
  padding: 11px 13px;
  background: var(--docsy-surface-elevated);
}
.header-editor-row + .header-editor-row {
  border-top: 1px solid var(--docsy-border-subtle);
}
.header-editor-file {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.file-index {
  color: var(--docsy-text-muted);
  min-width: 22px;
  font-size: 12px;
  padding-top: 2px;
}
.file-details {
  min-width: 0;
  flex: 1;
}
.file-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
  font-size: 13px;
}
.rule-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  margin-top: 3px;
  color: var(--docsy-text-muted);
}
.header-editor-input {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
}
.header-editor-input .el-input {
  flex: 1;
  min-width: 0;
}
.header-editor-input :deep(.el-button + .el-button),
.token-toolbar :deep(.el-button + .el-button) {
  margin-left: 0;
}
.token-picker {
  margin-top: 8px;
  padding: 9px;
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
}
.token-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 7px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}
.token-list {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 8px;
}
.header-token {
  border: 1px solid var(--docsy-border-strong);
  border-radius: var(--docsy-radius);
  padding: 3px 7px;
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.header-token.is-selected {
  color: var(--docsy-primary);
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}
.header-token.is-space {
  font-size: 11px;
  color: var(--docsy-text-muted);
}
.header-token:focus-visible {
  outline: 2px solid var(--docsy-primary);
  outline-offset: 2px;
}
.editor-error {
  color: var(--docsy-danger);
  font-size: 12px;
  margin-top: 6px;
}
.editor-count {
  font-size: 12px;
  color: var(--docsy-text-muted);
  margin-right: 12px;
}
</style>
