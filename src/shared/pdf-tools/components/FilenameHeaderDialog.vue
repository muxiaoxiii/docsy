<template>
  <el-dialog
    v-model="visible"
    title="从文件名提取页眉"
    width="min(900px, 94vw)"
    append-to-body
    :close-on-click-modal="false"
  >
    <div class="filename-rule-controls">
      <label>分隔符<el-input v-model="draft.separator" aria-label="文件名分隔符" placeholder="例如 - 或 _" /></label>
      <label
        >从第几段<SplitRangeBoundaryInput
          :value="draft.start"
          :min="1"
          :max="99"
          label="提取起始段"
          @commit="draft.start = $event"
      /></label>
      <label
        >到第几段<SplitRangeBoundaryInput
          v-if="!throughEnd"
          :value="draft.end"
          :min="1"
          :max="99"
          label="提取结束段"
          @commit="draft.end = $event" /><el-input v-else model-value="最后一段" disabled aria-label="提取结束段"
      /></label>
      <el-checkbox v-model="throughEnd">取到最后一段</el-checkbox>
      <el-checkbox v-model="draft.removeOrder">去掉开头排序号（如 9.）</el-checkbox>
      <label>替换文字<el-input v-model="draft.replaceFrom" aria-label="查找文字" placeholder="可留空" /></label>
      <label>替换为<el-input v-model="draft.replaceTo" aria-label="替换为" placeholder="留空表示删除" /></label>
      <el-button size="small" @click="useComparisonNames">证据文件 → 对比文件</el-button>
    </div>
    <p class="filename-rule-note">
      先按分隔符分段，再截取、替换。这里只设置页眉，不重命名原文件；页眉前缀、后缀会另行添加。
    </p>
    <el-table :data="rows" row-key="key" max-height="390" size="small" border>
      <el-table-column label="原文件名与分段" min-width="250">
        <template #default="{ row }"
          ><div class="filename-source" :title="row.source">{{ row.source }}</div>
          <div class="filename-segments">
            {{ row.parts.map((part, index) => `${index + 1}：${part}`).join(' ｜ ') }}
          </div></template
        >
      </el-table-column>
      <el-table-column label="提取结果（可单独修正）" min-width="210">
        <template #default="{ row }">
          <div class="filename-result">
            <el-input
              :model-value="row.text"
              :aria-label="`${row.source} 的页眉`"
              @update:model-value="setOverride(row.key, $event)"
            /><el-button v-if="hasOverride(row.key)" size="small" text @click="removeOverride(row.key)"
              >恢复规则</el-button
            >
          </div>
          <span v-if="row.error" class="filename-error" role="alert">{{ row.error }}</span>
        </template>
      </el-table-column>
    </el-table>
    <template #footer
      ><span class="filename-count">{{ rows.length }} 个文件{{ invalidCount ? `，${invalidCount} 项待修正` : '' }}</span
      ><el-button @click="visible = false">取消</el-button
      ><el-button type="primary" :disabled="!rows.length || Boolean(invalidCount)" @click="apply"
        >应用页眉规则</el-button
      ></template
    >
  </el-dialog>
</template>
<script setup>
import { computed, ref } from 'vue'
import SplitRangeBoundaryInput from './SplitRangeBoundaryInput.vue'
import {
  createFilenameHeaderRule,
  extractFilenameHeader,
  filenameHeaderKey,
  filenameHeaderParts,
  filenameHeaderSource,
} from '../composables/filenameHeader.js'
const props = defineProps({ files: { type: Array, default: () => [] }, rule: { type: Object, default: () => null } })
const emit = defineEmits(['apply'])
const visible = ref(false),
  draft = ref(createFilenameHeaderRule()),
  throughEnd = ref(false)
const currentRule = computed(() => ({ ...draft.value, end: throughEnd.value ? 0 : draft.value.end }))
const rows = computed(() =>
  props.files.map((file) => ({
    key: filenameHeaderKey(file),
    source: filenameHeaderSource(file),
    parts: filenameHeaderParts(file, currentRule.value),
    ...extractFilenameHeader(file, currentRule.value),
  })),
)
const invalidCount = computed(() => rows.value.filter((row) => row.error).length)
function open() {
  const rule = { ...createFilenameHeaderRule(), ...(props.rule || {}) }
  draft.value = { ...rule, overrides: { ...(rule.overrides || {}) }, end: rule.end || 1 }
  throughEnd.value = rule.end === 0
  visible.value = true
}
function hasOverride(key) {
  return Object.keys(draft.value.overrides).includes(key)
}
function setOverride(key, text) {
  draft.value.overrides[key] = text
}
function removeOverride(key) {
  delete draft.value.overrides[key]
}
function useComparisonNames() {
  draft.value.replaceFrom = '证据文件'
  draft.value.replaceTo = '对比文件'
}
function apply() {
  if (!rows.value.length || invalidCount.value) return
  emit('apply', JSON.parse(JSON.stringify(currentRule.value)))
  visible.value = false
}
defineExpose({ open })
</script>
<style scoped>
.filename-rule-controls {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  gap: 10px 14px;
}
.filename-rule-controls > label {
  display: grid;
  gap: 5px;
  width: 150px;
  font-size: 13px;
}
.filename-rule-controls :deep(.split-boundary-input) {
  width: 100%;
}
.filename-rule-controls :deep(.split-boundary-input .el-input) {
  --el-input-height: var(--el-component-size, 32px);
}
.filename-rule-note,
.filename-segments,
.filename-count {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.filename-source {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.filename-segments {
  overflow-wrap: anywhere;
  line-height: 1.6;
  margin-top: 4px;
}
.filename-result {
  display: flex;
  align-items: center;
  gap: 4px;
}
.filename-error {
  color: var(--el-color-danger);
  font-size: 12px;
}
.filename-count {
  margin-right: 12px;
}
</style>
