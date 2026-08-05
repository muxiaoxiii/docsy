<template>
  <div class="template-view">
    <el-tabs v-model="activeTab" class="template-tabs">
      <el-tab-pane label="制作模板" name="build">
        <TemplateBuildTab
          ref="buildTabRef"
          :source-docx="sourceDocx"
          :scanning="scanning"
          :marks="marks"
          :document-text="documentText"
          v-model:template-name="templateName"
          :field-rows="fieldRows"
          :selected-rows="selectedRows"
          :saving="saving"
          v-model:show-document-text="showDocumentText"
          v-model:show-template-preview="showTemplatePreview"
          v-model:group-name="groupName"
          v-model:group-label="groupLabel"
          v-model:group-type="groupType"
          :undo-stack="undoStack"
          :preview-sample-values="previewSampleValues"
          :source-preview-selection-payload="sourcePreviewSelectionPayload"
          :preview-focused-row-id="previewFocusedRowId"
          :template-preview="templatePreview"
          @select-source-docx="selectSourceDocx"
          @group-selected-rows="groupSelectedRows"
          @set-selected-rows-usage="setSelectedRowsUsage"
          @clear-selected-rows="clearSelectedRows"
          @selection-change="handleSelectionChange"
          @field-table-wheel="handleFieldTableWheel"
          @open-split-dialog="openSplitDialog"
          @row-type-change="onRowTypeChange"
          @field-name-input="onFieldNameInput"
          @structure-target-name-change="onStructureTargetNameChange"
          @apply-reference-suggestion="applyReferenceSuggestion"
          @apply-all-reference-suggestions="applyAllReferenceSuggestions"
          @sync-reference-source-from-key="syncReferenceSourceFromKey"
          @sync-marker-symbols="syncMarkerSymbols"
          @apply-marker-group-members="applyMarkerGroupMembers"
          @add-select-option="addSelectOption"
          @undo-last-action="undoLastAction"
          @save-template="saveTemplate"
          @remember-source-preview-selection="rememberSourcePreviewSelection"
          @focus-preview-row="focusPreviewRow"
          @trigger-preview-selection-add="triggerPreviewSelectionAdd"
          @set-preview-sample-value="setPreviewSampleValue"
        />
      </el-tab-pane>

      <el-tab-pane label="填写模板" name="render">
        <TemplateRenderTab
          :template-path="templatePath"
          :template-manifest="templateManifest"
          :template-library="templateLibrary"
          :template-library-loading="templateLibraryLoading"
          :form-values="formValues"
          :reference-selections="referenceSelections"
          :structure-overrides="structureOverrides"
          :type-overrides="typeOverrides"
          :history-context="historyContext"
          :rendering="rendering"
          :batch-processing="batchProcessing"
          v-model:field-search="fieldSearch"
          :collapsed-fields="collapsedFields"
          :renderable-template-fields="renderableTemplateFields"
          :filtered-renderable-fields="filteredRenderableFields"
          @load-template-library="loadTemplateLibrary"
          @select-template-package="selectTemplatePackage"
          @open-template-from-library="openTemplateFromLibrary"
          @delete-template="deleteTemplate"
          @collapse-all-fields="collapseAllFields"
          @expand-all-fields="expandAllFields"
          @render-template="renderTemplate"
          @batch-command="handleBatchCommand"
          @toggle-field-collapse="toggleFieldCollapse"
          @schedule-history-refresh="scheduleHistoryRefresh"
          @complete-field="completeField"
          @move-party-item="movePartyItem"
          @remove-party-item="removePartyItem"
          @add-party-item="addPartyItem"
          @reference-selection-change="onReferenceSelectionChange"
          @apply-suggestion="applySuggestion"
          @set-field-type-override="setFieldTypeOverride"
          @update-form-value="handleUpdateFormValue"
          @update-structure-override="handleUpdateStructureOverride"
        />
      </el-tab-pane>

      <el-tab-pane label="填写历史" name="history">
        <TemplateHistoryTab
          :history-runs-loading="historyRunsLoading"
          :grouped-history-runs="groupedHistoryRuns"
          :expanded-history-groups="expandedHistoryGroups"
          @refresh-history="loadTemplateHistoryRuns"
          @apply-history-run="applyHistoryRun"
          @open-history-template="openHistoryTemplate"
          @open-path="openPath"
          @expand-history-group="expandHistoryGroup"
          @collapse-history-group="collapseHistoryGroup"
        />
      </el-tab-pane>
      <el-tab-pane label="设置" name="settings">
        <TemplateSettingsTab
          v-model:item-separator-setting="itemSeparatorSetting"
          :template-trash="templateTrash"
          :template-trash-loading="templateTrashLoading"
          :clearing-history="clearingHistory"
          :template-database="templateDatabase"
          :template-database-loading="templateDatabaseLoading"
          @save-separator="saveItemSeparatorSetting"
          @restore-template="restoreTemplate"
          @permanently-delete-template="permanentlyDeleteTemplate"
          @clear-all-history="clearAllHistory"
          @refresh-trash="loadTemplateTrash"
          @refresh-template-database="loadTemplateDatabase"
          @delete-template-database-entry="deleteTemplateDatabaseEntry"
        />
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="splitDialog.visible" title="拆分标黄片段" width="520px">
      <p class="dialog-tip">用竖线或换行分隔，例如：张三|李四。保存模板时会按字符范围拆开。</p>
      <el-input v-model="splitDialog.partsText" type="textarea" :rows="5" />
      <template #footer>
        <el-button @click="splitDialog.visible = false">取消</el-button>
        <el-button type="primary" @click="applySplitDialog">应用拆分</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="batchSaveVisible" title="保存批量填写记录到模板历史" width="min(820px, 94vw)" append-to-body>
      <div class="batch-save-toolbar">
        <el-button size="small" @click="toggleBatchSaveAll(true)">全选</el-button>
        <el-button size="small" @click="invertBatchSaveSelection">反选</el-button>
        <span class="batch-save-count">已选 {{ batchSaveSelected.length }} / {{ batchSaveRows.length }} 行</span>
      </div>
      <el-table :data="batchSaveRows" size="small" border max-height="52vh" row-key="key" @row-click="(row) => toggleBatchSaveRow(row.key)">
        <el-table-column width="44">
          <template #default="{ row }">
            <el-checkbox :model-value="batchSaveSelected.includes(row.key)" @click.stop @change="() => toggleBatchSaveRow(row.key)" />
          </template>
        </el-table-column>
        <el-table-column type="index" label="#" width="44" />
        <el-table-column label="填写内容" min-width="240" show-overflow-tooltip>
          <template #default="{ row }">{{ batchSaveRowSummary(row) }}</template>
        </el-table-column>
        <el-table-column label="输出文件" prop="outputPath" min-width="200" show-overflow-tooltip />
      </el-table>
      <p class="hint-text">勾选需要保存到模板填写历史的行，点击"保存数据"录入；之后可在填写页看到这些历史建议。</p>
      <template #footer>
        <el-button @click="batchSaveVisible = false">取消</el-button>
        <el-button type="primary" :disabled="!batchSaveSelected.length" @click="submitBatchSave">保存数据</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { open, save } from '@tauri-apps/plugin-dialog'
import { fileName, parentDir, stripExtension } from '../../../core/filePath.js'
import { openPath, tauriCallSafe } from '../../../core/tauriBridge.js'
import {
  normalizeSuggestionSearchText,
  PUBLIC_CAUSE_ACTIONS,
  PUBLIC_COURT_NAMES,
  PUBLIC_LITIGATION_STAGES,
} from '../rules/publicRules.js'

// Tab components
import TemplateBuildTab from '../components/TemplateBuildTab.vue'
import TemplateRenderTab from '../components/TemplateRenderTab.vue'
import TemplateHistoryTab from '../components/TemplateHistoryTab.vue'
import TemplateSettingsTab from '../components/TemplateSettingsTab.vue'
import { useTemplateSettings } from '../composables/useTemplateSettings.js'
import { useBatchFill } from '../composables/useBatchFill.js'
import { markToRow, normalizeFieldRows, autoMergeMarks, inferFieldFromText, validateFieldRowsBeforeSave, buildFields } from '../composables/useFieldNormalization.js'
import { usePreviewSelection } from '../composables/usePreviewSelection.js'
import {
  sliceChars,
  referenceSourceKey,
  parseReferenceSourceKey,
  syncReferenceSourceFromKey,
  normalizedReferenceSource,
} from '../composables/fieldRowUtils.js'

const activeTab = ref('build')

// ── Settings: template trash + history database (composable) ────────────────
const {
  itemSeparatorSetting,
  templateTrash,
  templateTrashLoading,
  clearingHistory,
  templateDatabase,
  templateDatabaseLoading,
  saveItemSeparatorSetting,
  loadTemplateTrash,
  restoreTemplate,
  permanentlyDeleteTemplate,
  clearAllHistory,
  loadTemplateDatabase,
  deleteTemplateDatabaseEntry,
} = useTemplateSettings(loadHistoryContext, loadTemplateHistoryRuns)

const typeHelpItems = [
  { value: 'text', label: '文本', description: '普通可替换文字，如法院、案号、律所名称。' },
  { value: 'date', label: '日期', description: '日期字段，填写时用日期选择器，生成时输出中文日期格式。' },
  { value: 'select', label: '下拉选择', description: '从预设选项中选择或手动输入，如案由、诉讼阶段。' },
  { value: 'party_list', label: '列表', description: '适合当事人、律师等多项内容；多个名称按顺序用顿号连接。' },
  { value: 'reference', label: '引用', description: '复用前面字段的值；来源由填写时选择或在设置里指定。' },
  { value: 'checkbox', label: '单个勾选', description: '一个独立方框，只控制是否勾选。' },
  { value: 'radio_group', label: '互斥勾选组', description: '多个方框只能选一个，如一般授权/特别授权。' },
  { value: 'checkbox_group', label: '多选勾选组', description: '多个方框可同时选中，如多个保全事项。' },
  {
    value: 'prefix',
    label: '前缀',
    description: '字段为空时随字段一起删除的前缀文字，如"原告""，第三人""（案号："。',
  },
  { value: 'suffix', label: '后缀', description: '字段为空时随字段一起删除的后置文字，如"律师""）"。' },
  { value: 'delete_text', label: '删除文本', description: '保存模板时从 Word 原文中删除这段文字。' },
  { value: 'ignore', label: '保留原文', description: '不作为字段或规则保存；保存模板时只清除黄色高亮，正文仍保留。' },
]

const sourceDocx = ref('')
const templateName = ref('')
const marks = ref([])
const documentText = ref('')
const documentRuns = ref([])
const fieldRows = ref([])
const previewSampleValues = reactive({})
const selectedRows = ref([])
const buildTabRef = ref(null)

// ── Preview selection (composable) ──────────────────────────────────────────
const {
  sourcePreviewRef,
  documentPreviewRef,
  sourcePreviewSelection,
  sourcePreviewSelectionPayload,
  previewFocusedRowId,
  buildTemplatePreview,
  rememberSourcePreviewSelection,
  collectSourcePreviewSelection,
  nextReferenceFieldName,
} = usePreviewSelection(documentRuns, documentText, fieldRows, previewSampleValues)

// Sync composable's DOM refs with child component's exposed refs
watch(buildTabRef, (ref) => {
  if (ref) {
    sourcePreviewRef.value = ref.sourcePreviewRef
    documentPreviewRef.value = ref.documentPreviewRef
  }
}, { immediate: true })

const scanning = ref(false)
const saving = ref(false)
let lastPreviewAddKey = ''
let lastPreviewAddAt = 0
const groupName = ref('')
const groupLabel = ref('')
const groupType = ref('text')
const splitDialog = reactive({
  visible: false,
  rowId: '',
  partsText: '',
})
const showDocumentText = ref(false)
const showTemplatePreview = ref(false)
const undoStack = ref([])

const templatePath = ref('')
const templateManifest = ref(null)
const templateLibrary = ref([])
const templateLibraryLoading = ref(false)
const historyRuns = ref([])
const historyRunsLoading = ref(false)
const renderableTemplateFields = computed(() => (templateManifest.value?.fields || []).filter(isRenderableField))
const fieldSearch = ref('')
const collapsedFields = reactive(new Set())
const filteredRenderableFields = computed(() => {
  const query = fieldSearch.value.trim().toLowerCase()
  if (!query) return renderableTemplateFields.value
  return renderableTemplateFields.value.filter(
    (field) =>
      field.label?.toLowerCase().includes(query) ||
      field.name?.toLowerCase().includes(query) ||
      (field.semanticKey || '').toLowerCase().includes(query),
  )
})
function toggleFieldCollapse(field) {
  if (collapsedFields.has(field.id)) {
    collapsedFields.delete(field.id)
  } else {
    collapsedFields.add(field.id)
  }
}
function collapseAllFields() {
  collapsedFields.clear()
  for (const field of renderableTemplateFields.value) {
    collapsedFields.add(field.id)
  }
}
function expandAllFields() {
  collapsedFields.clear()
}
// Searching should surface matches even if they were collapsed.
watch(fieldSearch, (value) => {
  if (String(value || '').trim()) {
    collapsedFields.clear()
  }
})
const formValues = reactive({})
const referenceSelections = reactive({})
const structureOverrides = reactive({})
// Per-field temporary type override editable from the fill page's "…" menu
const typeOverrides = reactive({})

function effectiveFieldType(field) {
  return typeOverrides[field.id] || field.type
}

function setFieldTypeOverride(field, type) {
  if (!type || type === field.type) {
    delete typeOverrides[field.id]
  } else {
    typeOverrides[field.id] = type
  }
}
const rendering = ref(false)
const historyContext = ref({
  lastValues: {},
  fieldSuggestions: {},
  semanticSuggestions: {},
  associationSuggestions: {},
})
let historyRefreshTimer = null
let cachedFieldSuggestions = null
let cachedSemanticSuggestions = null
let templateOpenRequestSeq = 0
let historyContextRequestSeq = 0

const groupedHistoryRuns = computed(() => groupHistoryRuns(historyRuns.value))
const templatePreview = computed(() =>
  buildTemplatePreview(documentRuns.value, documentText.value, fieldRows.value, previewSampleValues),
)

onMounted(() => {
  document.addEventListener('selectionchange', rememberSourcePreviewSelection)
  document.addEventListener('pointerup', rememberSourcePreviewSelection)
  window.addEventListener('docsy-template-library-changed', refreshLibraryAndHistory)
  void loadTemplateLibrary()
  void loadTemplateHistoryRuns()
  void loadTemplateDatabase()
})

onUnmounted(() => {
  document.removeEventListener('selectionchange', rememberSourcePreviewSelection)
  document.removeEventListener('pointerup', rememberSourcePreviewSelection)
  window.removeEventListener('docsy-template-library-changed', refreshLibraryAndHistory)
})

function refreshLibraryAndHistory() {
  void loadTemplateLibrary()
  void loadTemplateHistoryRuns()
}

function setPreviewSampleValue(name, value) {
  previewSampleValues[name] = value
}

function clearPreviewSampleValues() {
  for (const key of Object.keys(previewSampleValues)) delete previewSampleValues[key]
}

function isMarkerType(type) {
  return ['checkbox', 'radio_group', 'checkbox_group'].includes(type)
}

function rowUsage(row) {
  if (['prefix', 'suffix', 'ignore', 'delete_text'].includes(row?.type)) return row.type
  return 'field'
}

function typeLabel(type) {
  return typeHelpItems.find((item) => item.value === type)?.label || type
}

function structureTargetRow(structureRow) {
  const usage = rowUsage(structureRow)
  if (usage !== 'prefix' && usage !== 'suffix') return null
  const boundTarget = fieldRows.value.find(
    (row) => row.enabled && rowUsage(row) === 'field' && row.rowId === structureRow.structureTargetRowId,
  )
  if (boundTarget) return boundTarget
  const sameNameFields = fieldRows.value.filter(
    (row) => row.enabled && rowUsage(row) === 'field' && row.name.trim() === structureRow.name.trim(),
  )
  if (sameNameFields.length === 1) return sameNameFields[0]
  const positionalTarget = positionalStructureTargetRow(structureRow)
  if (positionalTarget) return positionalTarget
  return null
}

function positionalStructureTargetRow(structureRow) {
  const usage = rowUsage(structureRow)
  if (usage !== 'prefix' && usage !== 'suffix') return null
  const direction = usage === 'prefix' ? 1 : -1
  return findNeighborFieldRow(fieldRows.value, fieldRows.value.indexOf(structureRow), direction)
}

function bindStructureRowToTarget(row, target) {
  if (!row || !target) return
  row.structureTargetRowId = target.rowId || ''
  row.name = target.name || row.name || ''
}

function onFieldNameInput(row) {
  if (!row || rowUsage(row) !== 'field') return
  for (const item of fieldRows.value) {
    if (rowUsage(item) !== 'prefix' && rowUsage(item) !== 'suffix') continue
    const targetsThisRow = item.structureTargetRowId
      ? item.structureTargetRowId === row.rowId
      : positionalStructureTargetRow(item) === row || structureRowTargetsField(item, row)
    if (!targetsThisRow) continue
    bindStructureRowToTarget(item, row)
  }
}

function onStructureTargetNameChange(row) {
  if (!row || (rowUsage(row) !== 'prefix' && rowUsage(row) !== 'suffix')) return
  const target = uniqueFieldRowByName(row.name)
  if (target) bindStructureRowToTarget(row, target)
  else row.structureTargetRowId = ''
}

function uniqueFieldRowByName(name) {
  const normalized = String(name || '').trim()
  if (!normalized) return null
  const matches = fieldRows.value.filter(
    (item) => item.enabled && rowUsage(item) === 'field' && item.name.trim() === normalized,
  )
  return matches.length === 1 ? matches[0] : null
}

function sameFieldRows(row) {
  return fieldRows.value.filter(
    (item) =>
      item.enabled &&
      rowUsage(item) === 'field' &&
      item.name.trim() &&
      item.name.trim() === row.name.trim() &&
      item.type === row.type,
  )
}

function handleFieldTableWheel(event) {
  const wrap = buildTabRef.value?.fieldTableRef?.$el?.querySelector('.el-table__body-wrapper .el-scrollbar__wrap')
  if (!wrap) return
  const maxScrollLeft = wrap.scrollWidth - wrap.clientWidth
  if (maxScrollLeft <= 1) return
  const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY
  if (!delta) return
  const before = wrap.scrollLeft
  const next = Math.max(0, Math.min(maxScrollLeft, before + delta))
  if (next === before) return
  wrap.scrollLeft = next
  event.preventDefault()
}

function referenceSuggestion(row) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  const text = normalizeComparableText(row.text)
  if (!text) return null
  const rowIndex = fieldRows.value.indexOf(row)
  if (rowIndex <= 0) return null
  const target = findReferenceTarget(row, text, rowIndex)
  if (!target) return null
  return {
    target: target.row,
    targetLabel: target.label,
    targetKind: target.kind,
    sourceIndex: target.sourceIndex,
    prefixRows: adjacentStructureRows(row, 'prefix'),
    suffixRows: adjacentStructureRows(row, 'suffix'),
  }
}

function findReferenceTarget(row, text, rowIndex) {
  const previousRows = fieldRows.value.slice(0, rowIndex).reverse()
  const partyTotalsBeforeRow = partySourceTotalsBefore(rowIndex)
  for (const item of previousRows) {
    if (!item.enabled || rowUsage(item) !== 'field' || isMarkerType(item.type)) continue
    if (normalizeComparableText(item.text) === text) {
      return {
        row: item,
        kind: 'field',
        label: item.name || item.label || item.text,
      }
    }
    if (item.type === 'party_list') {
      const itemIndex = (item.partyItems || []).findIndex((party) => normalizeComparableText(party) === text)
      if (itemIndex >= 0) {
        const sourceIndex = partySourceIndexForRow(item, rowIndex, partyTotalsBeforeRow) + itemIndex
        return {
          row: item,
          kind: 'party_item',
          sourceIndex,
          label: `${item.name || item.label || '当事人列表'} · 第 ${sourceIndex + 1} 项`,
        }
      }
    }
  }
  return null
}

function partySourceTotalsBefore(rowIndex) {
  const totals = new Map()
  for (const item of fieldRows.value.slice(0, Math.max(0, rowIndex))) {
    if (!item.enabled || rowUsage(item) !== 'field' || item.type !== 'party_list') continue
    const name = item.name?.trim()
    if (!name) continue
    totals.set(name, (totals.get(name) || 0) + Math.max(1, item.partyItems?.length || 0))
  }
  return totals
}

function partySourceIndexForRow(row, rowIndex, totalsBeforeRow = partySourceTotalsBefore(rowIndex)) {
  const name = row?.name?.trim()
  if (!name) return 0
  let cursor = totalsBeforeRow.get(name) || 0
  for (let index = Math.max(0, rowIndex) - 1; index >= 0; index -= 1) {
    const item = fieldRows.value[index]
    if (item === row) return cursor - Math.max(1, item.partyItems?.length || 0)
    if (!item.enabled || rowUsage(item) !== 'field' || item.type !== 'party_list' || item.name?.trim() !== name)
      continue
    cursor -= Math.max(1, item.partyItems?.length || 0)
  }
  return 0
}

function normalizeComparableText(text) {
  return String(text || '')
    .replace(/\s+/g, '')
    .trim()
}

function adjacentStructureRows(row, usage) {
  const index = fieldRows.value.indexOf(row)
  if (index < 0) return []
  const direction = usage === 'prefix' ? -1 : 1
  const rows = []
  for (let cursor = index + direction; cursor >= 0 && cursor < fieldRows.value.length; cursor += direction) {
    const candidate = fieldRows.value[cursor]
    if (rowUsage(candidate) !== usage) break
    rows.push(candidate)
  }
  return usage === 'prefix' ? rows.reverse() : rows
}

function applyReferenceSuggestion(row) {
  const suggestion = referenceSuggestion(row)
  if (!suggestion) return
  pushUndoSnapshot('改成引用')
  applyReferenceSuggestionToRow(row, suggestion)
  ElMessage.success(`已改为引用"${suggestion.target.name}"`)
}

function applyReferenceSuggestionToRow(row, suggestion) {
  const { target } = suggestion
  row.type = 'reference'
  row.name = referenceFieldNameForRow(row)
  row.label = row.name
  row.semanticKey = target.semanticKey || target.name
  row.required = false
  row.partyItems = []
  if (suggestion.targetKind === 'party_item') {
    row.referenceSourceMode = 'field'
    row.referenceSourceField = target.name
    row.referenceSourceSemanticKey = ''
    row.referenceSourceIndex = suggestion.sourceIndex ?? 0
  } else {
    row.referenceSourceMode = 'field'
    row.referenceSourceField = target.name
    row.referenceSourceSemanticKey = ''
    row.referenceSourceIndex = null
  }
  row.referenceSourceKey = referenceSourceKey(
    row.referenceSourceMode,
    row.referenceSourceField,
    row.referenceSourceIndex,
  )
  const structureTargetName = row.name
  if (row.referenceIncludePrefix) {
    for (const item of suggestion.prefixRows) {
      bindStructureRowToTarget(item, row)
      item.name = structureTargetName
      item.label = '前缀'
    }
  }
  if (row.referenceIncludeSuffix) {
    for (const item of suggestion.suffixRows) {
      bindStructureRowToTarget(item, row)
      item.name = structureTargetName
      item.label = '后缀'
    }
  }
  row.referenceHintSeen = true
}

function referenceFieldNameForRow(row) {
  return row?.name && !isGeneratedFieldName(row.name) ? row.name : nextReferenceFieldName()
}

function onReferenceSelectionChange(field, key) {
  const source = parseReferenceFillKey(key)
  const values = normalizeValuesForReferenceSources()
  formValues[fieldFormKey(field)] = resolveReferenceValueFromSource(source, values)
  scheduleHistoryRefresh()
}

function allReferenceSuggestions() {
  return fieldRows.value.filter((row) => referenceSuggestion(row))
}

function applyAllReferenceSuggestions() {
  const rows = allReferenceSuggestions()
  if (!rows.length) return
  pushUndoSnapshot('全部应用引用建议')
  for (const row of rows) {
    const suggestion = referenceSuggestion(row)
    if (!suggestion) continue
    applyReferenceSuggestionToRow(row, suggestion)
  }
  ElMessage.success(`已应用 ${rows.length} 条引用建议`)
}

function applyMarkerGroupMembers(row, members) {
  pushUndoSnapshot('调整勾选组成员')
  const selected = new Set(members)
  for (const item of fieldRows.value) {
    if (rowUsage(item) !== 'field' || !isMarkerType(item.type)) continue
    if (selected.has(item.rowId)) {
      item.name = row.name
      item.label = row.label || row.name
      item.type = row.type
    } else if (item !== row && item.name === row.name && item.type === row.type) {
      item.name = `${item.name}_${fieldRows.value.indexOf(item) + 1}`
    }
  }
}

function syncMarkerSymbols(row) {
  if (!isMarkerType(row.type) || !row.name) return
  pushUndoSnapshot('同步勾选符号')
  for (const item of sameFieldRows(row)) {
    if (!isMarkerType(item.type)) continue
    item.checkedText = row.checkedText
    item.uncheckedText = row.uncheckedText
  }
  ElMessage.success('已同步同组符号')
}

async function selectSourceDocx() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Word 文档', extensions: ['docx', 'docm', 'doc'] }],
  })
  if (!selected) return
  // .doc (old binary format) cannot preserve yellow highlights during conversion
  if (selected.toLowerCase().endsWith('.doc') && !selected.toLowerCase().endsWith('.docx')) {
    await ElMessageBox.confirm(
      '旧版 .doc 格式暂不支持自动导入（标黄信息会丢失）。请先用 Word 或 WPS 打开文件，另存为 .docx 格式后再导入。',
      '需要另存为 .docx',
      { confirmButtonText: '我知道了', showCancelButton: false, type: 'warning' },
    )
    return
  }
  sourceDocx.value = selected
  templateName.value = cleanTemplateName(stripExtension(fileName(selected), /\.(docx|docm|doc)$/i))
  await inspectSourceDocx()
}

async function inspectSourceDocx() {
  if (!sourceDocx.value) return
  scanning.value = true
  const result = await tauriCallSafe('inspect_docx_template', { path: sourceDocx.value })
  scanning.value = false
  if (!result.ok) {
    ElMessage.error(result.error || 'Word 文档读取失败，请确认文件未损坏且不是加密文件')
    return
  }
  marks.value = result.data.marks || []
  documentText.value = result.data.documentText || ''
  documentRuns.value = result.data.documentRuns || []
  sourcePreviewSelection.value = null
  sourcePreviewSelectionPayload.value = null
  clearPreviewSampleValues()
  fieldRows.value = normalizeFieldRows(autoMergeMarks(marks.value).map((mark, index) => markToRow(mark, index)))
  if (!fieldRows.value.length) {
    ElMessage.warning('没有找到黄色高亮标记')
  }
}

function refsForRowTextRange(row, start, end) {
  const refs = row.markRefs?.length
    ? row.markRefs
    : [{ markId: row.markId, start: row.charStart || 0, end: row.charEnd }]
  const result = []
  let cursor = 0
  for (const ref of refs) {
    const refStart = ref.start ?? 0
    const refEnd = ref.end ?? refStart
    const length = Math.max(0, refEnd - refStart)
    const segmentStart = cursor
    const segmentEnd = cursor + length
    const overlapStart = Math.max(start, segmentStart)
    const overlapEnd = Math.min(end, segmentEnd)
    if (overlapEnd > overlapStart && ref.markId) {
      result.push({
        markId: ref.markId,
        start: refStart + overlapStart - segmentStart,
        end: refStart + overlapEnd - segmentStart,
      })
    }
    cursor = segmentEnd
  }
  if (!result.length && row.markId) result.push({ markId: row.markId, start, end })
  return result
}

function findNeighborFieldRow(rows, startIndex, direction) {
  for (let index = startIndex + direction; index >= 0 && index < rows.length; index += direction) {
    const row = rows[index]
    if (row.enabled && rowUsage(row) === 'field') return row
    if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') continue
    break
  }
  return null
}

function isGeneratedFieldName(name) {
  return /^field_\d+$/.test(String(name || '')) || /^字段\d+$/.test(String(name || ''))
}

function cleanTemplateName(name) {
  return String(name || '')
    .replace(/[-_ ]?(标黄|模板|可替换|待填|字段版)$/i, '')
    .trim()
}

function groupSelectedRows() {
  const orderedRows = fieldRows.value.filter((row) => selectedRows.value.includes(row))
  const name = groupName.value.trim() || orderedRows[0]?.name?.trim()
  if (!name || !orderedRows.length) return
  pushUndoSnapshot('合并所选')
  const label = groupLabel.value.trim() || orderedRows.map((row) => row.text).join('')

  if (!isMarkerType(groupType.value)) {
    const firstIndex = fieldRows.value.findIndex((row) => row === orderedRows[0])
    const mergedRow = {
      ...orderedRows[0],
      rowId: `merged:${orderedRows.map((row) => row.rowId).join('+')}`,
      markId: orderedRows[0].markId,
      markRefs: orderedRows.flatMap(
        (row) => row.markRefs || [{ markId: row.markId, start: row.charStart, end: row.charEnd }],
      ),
      text: orderedRows.map((row) => row.text).join(''),
      context: orderedRows[0].context,
      type: groupType.value,
      name,
      label: label || name,
      semanticKey: orderedRows.find((row) => row.semanticKey)?.semanticKey || '',
      required: orderedRows.some((row) => row.required),
      optionalWhenEmpty: orderedRows.some((row) => row.optionalWhenEmpty),
      optionalScope: orderedRows.find((row) => row.optionalScope)?.optionalScope || 'position',
      optionalPrefix: orderedRows.find((row) => row.optionalPrefix)?.optionalPrefix || '',
      optionalSuffix: orderedRows.find((row) => row.optionalSuffix)?.optionalSuffix || '',
    }
    fieldRows.value = fieldRows.value.filter((row) => !selectedRows.value.includes(row))
    fieldRows.value.splice(firstIndex, 0, mergedRow)
    selectedRows.value = []
    buildTabRef.value?.fieldTableRef?.clearSelection?.()
    ElMessage.success('已合并为一个字段')
    return
  }

  for (const row of orderedRows) {
    row.name = name
    row.label = label || name
    row.type = groupType.value
    if (isMarkerType(groupType.value)) {
      row.optionId = row.optionId || `option_${fieldRows.value.indexOf(row) + 1}`
      row.optionLabel = row.optionLabel || row.text
    }
  }
  clearSelectedRows()
  ElMessage.success('已合并所选字段')
}

function setSelectedRowsUsage(usage) {
  if (!selectedRows.value.length) return
  pushUndoSnapshot(usage === 'prefix' ? '设为前缀' : '设为后缀')
  for (const row of selectedRows.value) {
    row.type = usage
    onRowTypeChange(row)
  }
  clearSelectedRows()
}

function clearSelectedRows() {
  selectedRows.value = []
  buildTabRef.value?.fieldTableRef?.clearSelection?.()
}

function rowsSnapshot() {
  return JSON.parse(JSON.stringify(fieldRows.value))
}

function restoreRows(snapshot) {
  fieldRows.value = JSON.parse(JSON.stringify(snapshot))
  selectedRows.value = []
  buildTabRef.value?.fieldTableRef?.clearSelection?.()
}

function pushUndoSnapshot(label) {
  const snapshot = rowsSnapshot()
  undoStack.value = [...undoStack.value.slice(-9), { label, snapshot }]
}

function undoLastAction() {
  const item = undoStack.value.pop()
  if (!item) {
    ElMessage.info('没有可撤销的操作')
    return
  }
  restoreRows(item.snapshot)
  ElMessage.success(`已撤销：${item.label}`)
}

function focusPreviewRow(row) {
  previewFocusedRowId.value = row.rowId
  row.referenceHintSeen = true
  buildTabRef.value?.fieldTableRef?.setCurrentRow?.(row)
}

function triggerPreviewSelectionAdd(type) {
  try {
    const payload = sourcePreviewSelectionPayload.value
    const key = `${type}:${payload?.text || ''}:${payload?.refs?.map((ref) => `${ref.markId}:${ref.start}:${ref.end}`).join('|') || ''}`
    const now = Date.now()
    if (key && key === lastPreviewAddKey && now - lastPreviewAddAt < 350) return
    lastPreviewAddKey = key
    lastPreviewAddAt = now
    addPreviewSelection(type)
  } catch (err) {
    ElMessage.error(`操作失败：${err?.message || err}`)
  }
}

function addPreviewSelection(type) {
  try {
    const selection = collectSourcePreviewSelection()
    if (!selection.refs.length) {
      ElMessage.warning(
        sourcePreviewSelectionPayload.value?.text ? '选区没有对应到 Word 文本，请换到模板全文中选择' : '请先选中文字',
      )
      return
    }
    pushUndoSnapshot('从预览新增标记')
    const addedRows = rowsFromPreviewSelection(selection, type)
    fieldRows.value = normalizeFieldRows([...fieldRows.value, ...addedRows])
    sourcePreviewSelection.value = null
    sourcePreviewSelectionPayload.value = null
    window.getSelection()?.removeAllRanges()
    if (addedRows.some((row) => (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') && !row.name)) {
      ElMessage.warning('已加入字段列表，请在设置里确认归属字段名')
    } else {
      ElMessage.success(type === 'delete_text' ? '已标记为删除文本' : '已加入字段列表')
    }
  } catch (err) {
    ElMessage.error(`添加失败：${err?.message || err}`)
    sourcePreviewSelection.value = null
    sourcePreviewSelectionPayload.value = null
  }
}

function rowsFromPreviewSelection(selection, type) {
  const inferred = inferFieldFromText(selection.text, selection.context, false, fieldRows.value.length)
  return [
    createPreviewRow(selection, {
      type,
      text: selection.text,
      refs: selection.refs,
      inferred,
      rowKey: 'single',
    }),
  ]
}

function createPreviewRow(selection, options) {
  const usageType = options.type
  const inferred = inferFieldFromText(selection.text, selection.context, false, fieldRows.value.length)
  const inferredField = options.inferred || inferred
  const isStructure = ['prefix', 'suffix', 'delete_text', 'ignore'].includes(usageType)
  const effectiveType = isStructure ? usageType : usageType || inferredField.type || 'text'
  const manualMeta = manualFieldMeta(selection.text, effectiveType, inferredField)
  const refs = options.refs || selection.refs
  const text = options.text ?? selection.text
  const row = {
    rowId: `preview:${Date.now()}:${fieldRows.value.length}:${options.rowKey || usageType}`,
    displayId: refs.map((ref) => ref.markId).join('+'),
    markId: refs[0]?.markId,
    markRefs: refs,
    charStart: refs[0]?.start,
    charEnd: refs[0]?.end,
    text,
    context: selection.context,
    enabled: true,
    type: effectiveType,
    name: isStructure ? options.structureName || '' : manualMeta.name,
    label: isStructure ? options.structureLabel || typeLabel(usageType) : manualMeta.label,
    semanticKey: isStructure ? '' : manualMeta.semanticKey,
    userSelectedType: isStructure ? '' : effectiveType,
    markSegments: markSegmentsFromRefs(refs, text),
    required: false,
    optionalWhenEmpty: false,
    optionalScope: 'position',
    optionalPrefix: '',
    optionalSuffix: '',
    optionId: '',
    optionLabel: '',
    checkedText: defaultCheckedText(text),
    uncheckedText: defaultUncheckedText(text),
    partyItems: effectiveType === 'party_list' ? splitPartyLabelText(text) : [],
    referenceHintSeen: true,
    referenceIncludePrefix: true,
    referenceIncludeSuffix: true,
    referenceSourceMode: 'auto',
    referenceSourceField: '',
    referenceSourceSemanticKey: '',
    referenceSourceIndex: null,
    referenceSourceKey: referenceSourceKey('auto', '', null),
  }
  return row
}

function manualFieldMeta(text, effectiveType, inferredField) {
  if (inferredField?.type === effectiveType && !['prefix', 'suffix', 'ignore', 'delete_text'].includes(effectiveType)) {
    return {
      name: inferredField.name,
      label: inferredField.label,
      semanticKey: inferredField.semanticKey || inferredField.name,
    }
  }
  const fallbackName = String(text || '').trim() || `字段${fieldRows.value.length + 1}`
  if (effectiveType === 'party_list') {
    return { name: fallbackName, label: fallbackName, semanticKey: '当事人' }
  }
  if (effectiveType === 'date') {
    return { name: '日期', label: '日期', semanticKey: '日期' }
  }
  if (effectiveType === 'reference') {
    const name = nextReferenceFieldName()
    return { name, label: name, semanticKey: '' }
  }
  return { name: fallbackName, label: fallbackName, semanticKey: fallbackName }
}

function markSegmentsFromRefs(refs, text) {
  const segments = []
  let cursor = 0
  for (const ref of refs) {
    const length = Math.max(0, (ref.end ?? ref.start ?? 0) - (ref.start ?? 0))
    segments.push({
      markId: ref.markId,
      text: sliceChars(text, cursor, cursor + length),
    })
    cursor += length
  }
  return segments
}

function onRowTypeChange(row) {
  const usage = rowUsage(row)
  if (usage === 'prefix' || usage === 'suffix') {
    row.required = false
    row.optionalWhenEmpty = false
    row.label = usage === 'prefix' ? '前缀' : '后缀'
    if (!row.structureTargetRowId) {
      const index = fieldRows.value.indexOf(row)
      const target =
        uniqueFieldRowByName(row.name) || findNeighborFieldRow(fieldRows.value, index, usage === 'prefix' ? 1 : -1)
      if (target) bindStructureRowToTarget(row, target)
    }
  } else if (usage === 'ignore') {
    row.required = false
    row.optionalWhenEmpty = false
    row.name = ''
    row.label = '保留原文'
  } else if (usage === 'delete_text') {
    row.required = false
    row.optionalWhenEmpty = false
    row.name = ''
    row.label = '删除文本'
  } else if (row.type === 'reference') {
    row.required = false
    row.optionalWhenEmpty = false
    row.name = row.name && !isGeneratedFieldName(row.name) ? row.name : referenceFieldNameForRow(row)
    row.label = row.name
    if (!row.referenceSourceKey) row.referenceSourceKey = referenceSourceKey('auto', '', null)
    syncReferenceSourceFromKey(row)
  } else if (row.type === 'select') {
    if (!row.selectOptions) row.selectOptions = []
  }
  row.partyItems = row.type === 'party_list' ? splitPartyLabelText(row.text) : []
}

function splitPartyLabelText(text) {
  return splitPartyLabelSegments(text).map((item) => item.text)
}

function splitPartyLabelSegments(text) {
  const chars = [...String(text || '')]
  const segments = []
  let start = 0
  const flush = (end) => {
    let trimmedStart = start
    let trimmedEnd = end
    while (trimmedStart < trimmedEnd && /\s/.test(chars[trimmedStart])) trimmedStart += 1
    while (trimmedEnd > trimmedStart && /\s/.test(chars[trimmedEnd - 1])) trimmedEnd -= 1
    if (trimmedEnd > trimmedStart) {
      segments.push({
        text: chars.slice(trimmedStart, trimmedEnd).join(''),
        start: trimmedStart,
        end: trimmedEnd,
      })
    }
  }
  for (let index = 0; index < chars.length; index += 1) {
    if (/[、，,；;\n]/.test(chars[index])) {
      flush(index)
      start = index + 1
    }
  }
  flush(chars.length)
  return segments
}

function openSplitDialog(row) {
  splitDialog.visible = true
  splitDialog.rowId = row.rowId
  splitDialog.partsText = row.text
}

function applySplitDialog() {
  const rowIndex = fieldRows.value.findIndex((row) => row.rowId === splitDialog.rowId)
  if (rowIndex < 0) return
  const row = fieldRows.value[rowIndex]
  const parts = splitDialog.partsText
    .split(/[\n|]/)
    .map((item) => item.trim())
    .filter(Boolean)
  if (parts.length < 2) {
    ElMessage.warning('至少拆成两个片段')
    return
  }

  pushUndoSnapshot('拆分字段')
  let cursor = 0
  const splitRows = []
  for (const [index, part] of parts.entries()) {
    const range = findPartCharRange(row.text, part, cursor)
    if (!range) {
      ElMessage.error(`找不到片段：${part}`)
      return
    }
    const { start, end } = range
    cursor = end
    const refs = refsForRowTextRange(row, start, end)
    splitRows.push({
      ...row,
      rowId: `${row.markId}:${start}:${end}:${index}`,
      text: part,
      label: part,
      name: `${row.name}_${index + 1}`,
      markRefs: refs,
      charStart: start,
      charEnd: end,
      markSegments: markSegmentsFromRefs(refs, part),
      optionId: `${row.optionId}_${index + 1}`,
    })
  }
  fieldRows.value.splice(rowIndex, 1, ...splitRows)
  splitDialog.visible = false
  ElMessage.success('已拆分字段')
}

function findPartCharRange(text, part, cursor = 0) {
  const source = [...String(text || '')]
  const target = [...String(part || '')]
  if (!target.length) return null
  for (let start = Math.max(0, cursor); start <= source.length - target.length; start += 1) {
    let matched = true
    for (let offset = 0; offset < target.length; offset += 1) {
      if (source[start + offset] !== target[offset]) {
        matched = false
        break
      }
    }
    if (matched) return { start, end: start + target.length }
  }
  return null
}

function defaultCheckedText(text) {
  const trimmed = String(text || '').trim()
  if (trimmed.includes('(') || trimmed.includes('（')) return '(√)'
  return '☑'
}

function defaultUncheckedText(text) {
  const trimmed = String(text || '').trim()
  if (trimmed.includes('(') || trimmed.includes('（')) return '( )'
  if (trimmed === '□') return '□'
  return '☐'
}

async function saveTemplate() {
  const validationError = validateFieldRowsBeforeSave()
  if (validationError) {
    ElMessage.warning(validationError)
    return
  }
  const fields = buildFields()
  if (!fields.length) {
    ElMessage.warning('请至少确认一个字段')
    return
  }
  let confirmedName = templateName.value || '未命名模板'
  try {
    const result = await ElMessageBox.prompt('保存到 Docsy 模板库，之后可在填写页直接选择。', '确认模板名称', {
      confirmButtonText: '保存',
      cancelButtonText: '取消',
      inputValue: confirmedName,
      inputPattern: /\S+/,
      inputErrorMessage: '请输入模板名称',
    })
    confirmedName = result.value.trim()
  } catch {
    return
  }

  saving.value = true
  const result = await tauriCallSafe('save_docx_template_to_library', {
    args: {
      sourceDocx: sourceDocx.value,
      outputPath: '',
      templateName: confirmedName,
      fields,
    },
  })
  saving.value = false
  if (!result.ok) {
    ElMessage.error(result.error || '保存模板失败，请检查文件是否被其他程序占用')
    return
  }
  const actualOutputPath = result.data?.outputPath || ''
  templateName.value = confirmedName
  ElMessage.success('模板已保存到 Docsy 模板库')
  templatePath.value = actualOutputPath
  await maybeSeedTemplateHistory(actualOutputPath)
  await loadTemplateLibrary()
  if (actualOutputPath) {
    await openTemplatePackage(actualOutputPath, result.data?.manifest || null)
    activeTab.value = 'render'
  }
}

async function maybeSeedTemplateHistory(path) {
  if (!path) return
  const values = templateSeedValues()
  if (!Object.keys(values).length) return
  try {
    await ElMessageBox.confirm(
      '是否把当前模板中的标黄示例值存入这个模板的内部数据库？选择"否"则该模板的数据从空开始。',
      '保存模板数据',
      {
        confirmButtonText: '存入',
        cancelButtonText: '不存',
        type: 'info',
      },
    )
  } catch {
    return
  }
  const result = await tauriCallSafe('seed_template_history', {
    templatePath: path,
    values,
  })
  if (!result.ok) {
    ElMessage.warning(result.error || '模板数据保存失败')
  }
}

function templateSeedValues() {
  const values = {}
  const partyValues = new Map()
  for (const row of fieldRows.value) {
    if (!row.enabled || rowUsage(row) !== 'field' || isMarkerType(row.type) || isGeneratedFieldName(row.name)) continue
    const name = row.name.trim()
    const text = String(row.text || '').trim()
    if (!name || !text) continue
    if (row.type === 'party_list') {
      if (!partyValues.has(name)) partyValues.set(name, [])
      const items = row.partyItems?.length ? row.partyItems : splitPartyLabelText(text)
      for (const item of items.length ? items : [text]) {
        const value = String(item || '').trim()
        if (value && !partyValues.get(name).includes(value)) partyValues.get(name).push(value)
      }
    } else if (!(name in values)) {
      values[name] = text
    }
  }
  for (const [name, items] of partyValues.entries()) {
    if (items.length) values[name] = items
  }
  return values
}

function structureRowTargetsField(structureRow, fieldRow) {
  return structureTargetRow(structureRow) === fieldRow
}

async function selectTemplatePackage() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Docsy 模板', extensions: ['docsytpl'] }],
  })
  if (!selected) return
  await openTemplatePackage(selected)
}

async function loadTemplateLibrary() {
  templateLibraryLoading.value = true
  const result = await tauriCallSafe('list_template_library')
  templateLibraryLoading.value = false
  if (!result.ok) {
    ElMessage.error(result.error || '读取模板库失败')
    return
  }
  templateLibrary.value = result.data || []
}

async function loadTemplateHistoryRuns() {
  historyRunsLoading.value = true
  const result = await tauriCallSafe('list_template_generation_runs', { limit: 300 })
  historyRunsLoading.value = false
  if (!result.ok) {
    ElMessage.error(result.error || '读取填写历史失败')
    return
  }
  historyRuns.value = result.data || []
}

function groupHistoryRuns(runs) {
  const groups = new Map()
  for (const run of runs || []) {
    const key = run.templateId || run.templatePath || run.templateName || 'unknown'
    if (!groups.has(key)) {
      groups.set(key, {
        templateId: key,
        templateName: run.templateName || '未命名模板',
        templatePath: run.templatePath || '',
        runs: [],
      })
    }
    groups.get(key).runs.push(run)
  }
  return Array.from(groups.values())
}

async function applyHistoryRun(run) {
  if (!run?.templatePath) return
  const opened = await openTemplatePackage(run.templatePath)
  if (!opened) return
  applyValuesToForm(run.fieldValues || {}, true)
  activeTab.value = 'render'
  ElMessage.success('已填入该次历史表单')
}

async function deleteTemplate(item) {
  if (!item?.path) return
  try {
    await ElMessageBox.confirm(`删除"${item.name}"？模板会先放入回收站，可在设置里恢复。`, '删除模板', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    })
  } catch {
    return
  }
  const result = await tauriCallSafe('move_template_to_trash', { args: { path: item.path } })
  if (!result.ok) {
    ElMessage.error(result.error || '删除模板失败')
    return
  }
  if (templatePath.value === item.path) {
    templatePath.value = ''
    templateManifest.value = null
    resetFormValues([])
  }
  await loadTemplateLibrary()
  await loadTemplateHistoryRuns()
  ElMessage.success('模板已移入回收站')
}

async function openTemplateFromLibrary(item) {
  if (!item?.path) return
  await openTemplatePackage(item.path, item.manifest)
}

async function openTemplatePackage(path, knownManifest = null) {
  const requestSeq = ++templateOpenRequestSeq
  cachedFieldSuggestions = null
  cachedSemanticSuggestions = null
  const result = knownManifest ? { ok: true, data: knownManifest } : await tauriCallSafe('inspect_docsytpl', { path })
  if (requestSeq !== templateOpenRequestSeq) return false
  if (!result.ok) {
    ElMessage.error(result.error || '读取模板失败，文件可能已被移动或删除')
    return false
  }
  templatePath.value = path
  templateManifest.value = result.data
  clearStructureOverrides()
  resetFormValues((result.data.fields || []).filter(isRenderableField))
  await loadHistoryContext(false)
  return true
}

// Opening a template from the history tab should land on the fill form.
async function openHistoryTemplate(path) {
  const ok = await openTemplatePackage(path)
  if (ok) activeTab.value = 'render'
}

function clearStructureOverrides() {
  for (const key of Object.keys(structureOverrides)) delete structureOverrides[key]
}

function clearReferenceSelections() {
  for (const key of Object.keys(referenceSelections)) delete referenceSelections[key]
}

function fieldFormKey(field) {
  if (!field) return ''
  return field.id || field.name
}

function resetFormValues(fields) {
  for (const key of Object.keys(formValues)) delete formValues[key]
  clearReferenceSelections()
  for (const field of fields) {
    const key = fieldFormKey(field)
    if (field.type === 'checkbox') {
      formValues[key] = false
    } else if (field.type === 'checkbox_group') {
      formValues[key] = []
    } else if (field.type === 'date') {
      formValues[key] = todayText()
    } else if (field.type === 'party_list') {
      formValues[key] = [{ text: '', suffix: '' }]
    } else if (field.type === 'reference') {
      const source = fixedReferenceSource(field.reference)
      referenceSelections[key] =
        source.mode === 'field' ? referenceSourceKey('field', source.sourceField, source.sourceIndex) : ''
      formValues[key] = ''
    } else {
      formValues[key] = ''
    }
  }
}

async function loadHistoryContext(applyLastValues = false) {
  if (!templatePath.value || !templateManifest.value) return
  const requestSeq = ++historyContextRequestSeq
  const requestPath = templatePath.value
  const fullRefresh = applyLastValues || cachedFieldSuggestions == null
  // Only send non-empty values: empty fields produce no suggestions, so this
  // shrinks the payload and the backend's per-field association queries.
  const filledValues = {}
  for (const [key, value] of Object.entries(normalizeValues())) {
    const empty =
      value === undefined || value === null || value === '' ||
      (Array.isArray(value) && value.length === 0)
    if (!empty) filledValues[key] = value
  }
  const result = await tauriCallSafe('get_template_history_context', {
    templatePath: requestPath,
    values: filledValues,
    fullRefresh,
  })
  if (requestSeq !== historyContextRequestSeq || requestPath !== templatePath.value) return
  if (!result.ok) return

  let merged = result.data
  if (cachedFieldSuggestions != null && !fullRefresh) {
    merged = {
      ...merged,
      fieldSuggestions: cachedFieldSuggestions,
      semanticSuggestions: cachedSemanticSuggestions || {},
    }
  } else {
    cachedFieldSuggestions = result.data.fieldSuggestions || {}
    cachedSemanticSuggestions = result.data.semanticSuggestions || {}
  }
  historyContext.value = merged

  if (applyLastValues) {
    applyLastValuesToForm(result.data.lastValues || {})
  }
}

function applyLastValuesToForm(lastValues) {
  applyValuesToForm(lastValues, false)
}

function applyValuesToForm(values, overwrite = false) {
  for (const field of renderableTemplateFields.value) {
    const key = fieldFormKey(field)
    const value = values[field.id] ?? values[field.name]
    if (value === undefined) continue
    if (overwrite || isEmptyValue(formValues[key])) {
      formValues[key] = inputValueForField(field, value)
    }
  }
  scheduleHistoryRefresh()
}

function isEmptyValue(value) {
  if (Array.isArray(value)) {
    return !value.length || !partyItemsToValues(value).length
  }
  return value === '' || value == null
}

function inputValueForField(field, value) {
  if (field.type === 'party_list' && Array.isArray(value)) {
    return value.map((item) => parsePartyItem(displayValue(item)))
  }
  return value
}

function scheduleHistoryRefresh() {
  if (historyRefreshTimer) window.clearTimeout(historyRefreshTimer)
  historyRefreshTimer = window.setTimeout(() => {
    void loadHistoryContext(false)
  }, 350)
}

async function renderTemplate() {
  if (!templatePath.value || !templateManifest.value) return
  const missing = requiredMissingFields()
  if (missing.length) {
    ElMessage.warning(`请先填写必填字段：${missing.join('、')}`)
    return
  }
  const defaultName = `${stripExtension(fileName(templatePath.value), /\.docsytpl$/i)}-output.docx`
  const outputPath = await save({
    defaultPath: `${parentDir(templatePath.value)}/${defaultName}`,
    filters: [{ name: 'Word 文档', extensions: ['docx'] }],
  })
  if (!outputPath) return
  const finalOutputPath = ensureExtension(outputPath, 'docx')

  rendering.value = true
  const result = await tauriCallSafe('render_docx_template', {
    args: {
      templatePath: templatePath.value,
      outputPath: finalOutputPath,
      values: normalizeValues(),
      structureOverrides: normalizeStructureOverrides(),
      itemSeparator: itemSeparatorSetting.value || '、',
    },
  })
  rendering.value = false
  if (!result.ok) {
    ElMessage.error(result.error || 'Word 文书生成失败，请检查模板字段是否完整')
    return
  }
  ElMessage.success('Word 文书已生成')
  await loadHistoryContext(false)
  await loadTemplateHistoryRuns()
  const openResult = await openPath(result.data)
  if (!openResult.ok) {
    ElMessage.warning('文书已生成但无法自动打开，请到保存目录查看')
  }
}

// ── Batch Fill (composable) ─────────────────────────────────────────────────
const {
  batchProcessing,
  batchSaveVisible,
  batchSaveRows,
  batchSaveSelected,
  handleBatchCommand,
  toggleBatchSaveAll,
  invertBatchSaveSelection,
  toggleBatchSaveRow,
  batchSaveRowSummary,
  submitBatchSave,
} = useBatchFill(templatePath, templateManifest, normalizeValues, normalizeStructureOverrides, itemSeparatorSetting, loadTemplateHistoryRuns)

function normalizeValues() {
  const values = {}
  for (const field of renderableTemplateFields.value) {
    const key = fieldFormKey(field)
    const value = formValues[key]
    let normalizedValue
    if (effectiveFieldType(field) === 'party_list') {
      normalizedValue = partyItemsToValues(value)
    } else {
      normalizedValue = value
    }
    values[field.id] = normalizedValue
    if (!(field.name in values)) {
      values[field.name] = normalizedValue
    }
    if (effectiveFieldType(field) !== 'reference') {
      addSemanticAliasValue(values, field, normalizedValue)
    }
  }
  return values
}

function normalizeStructureOverrides() {
  const result = {}
  for (const [name, override] of Object.entries(structureOverrides)) {
    const field = fieldByStructureOverrideKey(name)
    result[name] = {
      prefix: override.prefix ?? '',
      suffix: fieldUsesRepeatableSuffix(field) ? '' : (override.suffix ?? ''),
    }
  }
  return result
}

function fieldByStructureOverrideKey(key) {
  return (templateManifest.value?.fields || []).find((field) => structureOverrideKey(field) === key)
}

function addSemanticAliasValue(values, field, value) {
  const key = String(field.semanticKey || '').trim()
  if (!key || key === field.name || isEmptyValue(value)) return
  if (Array.isArray(value)) {
    const existing = Array.isArray(values[key]) ? values[key] : values[key] ? [values[key]] : []
    values[key] = [...existing, ...value].filter(Boolean)
  } else if (values[key] == null || values[key] === '') {
    values[key] = value
  }
}

function fixedReferenceSource(reference) {
  if (!reference) return { mode: 'auto', sourceField: '', sourceSemanticKey: '', sourceIndex: null }
  const mode =
    reference.sourceMode || (reference.sourceSemanticKey ? 'semantic' : reference.sourceField ? 'field' : 'auto')
  return {
    mode,
    sourceField: reference.sourceField || '',
    sourceSemanticKey: reference.sourceSemanticKey || '',
    sourceIndex: reference.sourceIndex == null ? null : reference.sourceIndex,
  }
}

function normalizeValuesForReferenceSources() {
  const values = {}
  for (const field of renderableTemplateFields.value) {
    if (field.type === 'reference') continue
    const value =
      field.type === 'party_list'
        ? partyItemsToValues(formValues[fieldFormKey(field)] || [])
        : formValues[fieldFormKey(field)]
    values[field.id] = value
    if (!(field.name in values)) values[field.name] = value
    addSemanticAliasValue(values, field, value)
  }
  return values
}

function parseReferenceFillKey(key) {
  const parsed = parseReferenceSourceKey(key)
  return parsed.sourceField || parsed.sourceSemanticKey
    ? parsed
    : { mode: 'auto', sourceField: '', sourceSemanticKey: '', sourceIndex: null }
}

function resolveReferenceValueFromSource(source, values) {
  if (!source || source.mode === 'auto') return ''
  const raw = source.mode === 'semantic' ? values?.[source.sourceSemanticKey] : values?.[source.sourceField]
  if (Array.isArray(raw)) {
    return source.sourceIndex == null
      ? raw.map(displayValue).filter(Boolean).join('、')
      : displayValue(raw[source.sourceIndex] || '')
  }
  return source.sourceIndex == null && raw != null ? String(raw) : ''
}

function structureOverrideKey(field) {
  return field?.id || field?.name || ''
}

function splitPartyInput(value) {
  return value
    .split(/[、\n]/)
    .map((item) => item.trim())
    .filter(Boolean)
}

function partyListRows(field) {
  const key = fieldFormKey(field)
  const current = formValues[key]
  if (!Array.isArray(current)) {
    formValues[key] = splitPartyInput(String(current || '')).map(parsePartyItem)
  }
  if (!formValues[key].length) {
    formValues[key].push({ text: '', suffix: defaultPartySuffix(field, 0) })
  }
  formValues[key].forEach((item, index) => {
    if (!item.suffix && partyFieldUsesSuffix(field)) item.suffix = defaultPartySuffix(field, index)
  })
  return formValues[key]
}

function partyFieldUsesSuffix(field) {
  return fieldUsesRepeatableSuffix(field)
}

function fieldUsesRepeatableSuffix(field) {
  return (
    field?.type === 'party_list' && (field.markRefs || []).some((markRef) => markRef.optionalRule?.removeEmptySuffix)
  )
}

function partySuffixOptions(field) {
  return Array.from(
    new Set((field.markRefs || []).map((markRef) => markRef.optionalRule?.removeEmptySuffix).filter(Boolean)),
  )
}

function defaultPartySuffix(field, index) {
  const options = partySuffixOptions(field)
  if (!options.length) return ''
  return options[index] || options[0]
}

function parsePartyItem(value) {
  if (value && typeof value === 'object') {
    return {
      text: String(value.name || value.label || value.text || '').trim(),
      suffix: String(value.suffix || '').trim(),
    }
  }
  return {
    text: String(value || '').trim(),
    suffix: '',
  }
}

function addPartyItem(field) {
  const rows = partyListRows(field)
  rows.push({ text: '', suffix: defaultPartySuffix(field, rows.length) })
  scheduleHistoryRefresh()
}

function addSelectOption(row) {
  if (!row.selectOptions) row.selectOptions = []
  row.selectOptions.push({ label: '', checkedText: '' })
}

function removePartyItem(field, index) {
  const rows = partyListRows(field)
  rows.splice(index, 1)
  if (!rows.length) rows.push({ text: '', suffix: '' })
  scheduleHistoryRefresh()
}

function movePartyItem(field, index, delta) {
  const rows = partyListRows(field)
  const next = index + delta
  if (next < 0 || next >= rows.length) return
  const [item] = rows.splice(index, 1)
  rows.splice(next, 0, item)
  scheduleHistoryRefresh()
}

function partyItemsToValues(value) {
  if (typeof value === 'string') return splitPartyInput(value)
  if (!Array.isArray(value)) return []
  return value
    .map((item) => {
      if (typeof item === 'string') return item.trim()
      const suffix = String(item?.suffix || '').trim()
      let text = String(item?.text || '').trim()
      if (suffix && text.endsWith(suffix)) {
        text = text.slice(0, -suffix.length).trim()
      }
      return suffix ? { name: text, suffix } : text
    })
    .filter((item) => (typeof item === 'string' ? Boolean(item) : Boolean(item.name)))
}

function requiredMissingFields() {
  return renderableTemplateFields.value
    .filter((field) => field.required && isEmptyValue(formValues[fieldFormKey(field)]))
    .map((field) => field.label || field.name)
}

function isRenderableField(field) {
  return !['delete_text', 'prefix', 'suffix', 'ignore'].includes(field?.type)
}

function ensureExtension(path, extension) {
  return String(path || '')
    .toLowerCase()
    .endsWith(`.${extension}`)
    ? path
    : `${path}.${extension}`
}

function allFieldSuggestionItems(field) {
  const combined = [
    ...(historyContext.value.associationSuggestions?.[field.id] || []).map((item) => ({
      ...item,
      source: '关联',
    })),
    ...(historyContext.value.fieldSuggestions?.[field.id] || []),
    ...(historyContext.value.semanticSuggestions?.[field.id] || []),
    ...publicSuggestionItems(field),
  ]
  const seen = new Set()
  return combined.filter((item) => {
    const key = item.display || JSON.stringify(item.value)
    if (!key || seen.has(key)) return false
    seen.add(key)
    return true
  })
}

function completeField(field, query, callback) {
  const rawTokens = String(query || '')
    .split(/\s+/)
    .filter(Boolean)
  const tokens = rawTokens.map(normalizeSuggestionSearchText).filter(Boolean)
  const items = allFieldSuggestionItems(field)
    .filter((item) => tokens.length === 0 || multiTokenMatches(item, tokens))
    .sort((a, b) => {
      if (tokens.length === 0) return (b.count || 0) - (a.count || 0)
      const aScore = matchScore(a, tokens)
      const bScore = matchScore(b, tokens)
      if (aScore !== bScore) return bScore - aScore
      return (b.count || 0) - (a.count || 0)
    })
    .slice(0, 20)
    .map((item) => ({ value: item.display, rawValue: item.value }))
  callback(items)
}

function multiTokenMatches(item, tokens) {
  return tokens.every((token) => suggestionMatches(item, token))
}

function suggestionMatches(item, normalizedKeyword) {
  const parts = [item.display, item.value, item.source].filter(Boolean)
  return parts.some((part) => normalizeSuggestionSearchText(part).includes(normalizedKeyword))
}

function matchScore(item, tokens) {
  const text = normalizeSuggestionSearchText(item.display || '')
  if (tokens.length === 0) return 0
  let score = 0
  for (const token of tokens) {
    if (text.includes(token)) {
      score += 1
      if (text.startsWith(token)) score += 2
      const allTokensMatch = tokens.every((t) => text.includes(t))
      if (allTokensMatch && tokens.length > 1) score += 1
    }
  }
  return score
}

function publicSuggestionItems(field) {
  const key = (field.semanticKey || field.name || '').trim()
  if (!key) return []
  const registry = publicDataRegistry()
  if (registry.has(key)) return mapPublicSuggestions(registry.get(key)())
  const name = `${field.name || ''}${field.label || ''}`
  if (/法院/.test(name)) return mapPublicSuggestions(PUBLIC_COURT_NAMES)
  if (/案由|纠纷/.test(name)) return mapPublicSuggestions(PUBLIC_CAUSE_ACTIONS)
  if (/诉讼阶段|阶段|程序/.test(name) && !/代理人/.test(name)) return mapPublicSuggestions(PUBLIC_LITIGATION_STAGES)
  return []
}

function publicDataRegistry() {
  return new Map([
    ['法院', () => PUBLIC_COURT_NAMES],
    ['案号', () => []],
    ['案由', () => PUBLIC_CAUSE_ACTIONS],
    ['诉讼阶段', () => PUBLIC_LITIGATION_STAGES],
    ['律所名称', () => []],
    ['地址', () => []],
    ['当事人', () => []],
    ['律师', () => []],
    ['法定代表人', () => []],
    ['负责人', () => []],
    ['统一社会信用代码', () => []],
    ['身份证号', () => []],
    ['护照号', () => []],
  ])
}

function mapPublicSuggestions(values) {
  return values.map((value) => ({
    source: '公共',
    display: value,
    value,
    count: 0,
  }))
}

function applySuggestion(field, value) {
  formValues[fieldFormKey(field)] = inputValueForField(field, value)
  scheduleHistoryRefresh()
}

function displayValue(value) {
  if (value == null) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (Array.isArray(value)) return value.map(displayValue).join('、')
  return value.name || value.label || JSON.stringify(value)
}

const expandedHistoryGroups = reactive(new Set())
function expandHistoryGroup(templateId) {
  expandedHistoryGroups.add(templateId)
}
function collapseHistoryGroup(templateId) {
  expandedHistoryGroups.delete(templateId)
}

function todayText() {
  const now = new Date()
  const year = now.getFullYear()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  const day = String(now.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

// ── Wrapper functions for component emits ────────────────────────────────────

function handleSelectionChange(filteredRows) {
  selectedRows.value = filteredRows
}

function handleUpdateFormValue(key, value) {
  formValues[key] = value
}

function handleUpdateStructureOverride(key, prop, value) {
  if (!structureOverrides[key]) {
    structureOverrides[key] = { prefix: '', suffix: '' }
  }
  structureOverrides[key][prop] = value
}
</script>

<style scoped>
.template-view,
.template-tabs {
  min-height: 100%;
}

.template-view {
  padding: 20px 24px 32px;
  background: var(--docsy-canvas);
}

:deep(.template-tabs > .el-tabs__content),
:deep(.template-tabs > .el-tabs__content > .el-tab-pane) {
  min-width: 0;
  overflow: visible;
}

.dialog-tip {
  margin-bottom: 10px;
}

@media (max-width: 760px) {
  .template-view {
    padding: 16px;
  }
}
</style>
