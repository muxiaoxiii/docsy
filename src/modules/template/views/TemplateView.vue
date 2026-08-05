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
          :history-runs="historyRuns"
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
          @save-separator="saveItemSeparatorSetting"
          @restore-template="restoreTemplate"
          @permanently-delete-template="permanentlyDeleteTemplate"
          @clear-all-history="clearAllHistory"
          @refresh-trash="loadTemplateTrash"
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
  inferTemplateField,
  looksLikeDatePrefix,
  looksLikePrefixMark,
  looksLikeSuffixMark,
  normalizeSuggestionSearchText,
  PUBLIC_CAUSE_ACTIONS,
  PUBLIC_COURT_NAMES,
  PUBLIC_LITIGATION_STAGES,
  prefixStructureInfo,
  prefixTargetRule,
  suffixTargetRule,
} from '../rules/publicRules.js'

// Tab components
import TemplateBuildTab from '../components/TemplateBuildTab.vue'
import TemplateRenderTab from '../components/TemplateRenderTab.vue'
import TemplateHistoryTab from '../components/TemplateHistoryTab.vue'
import TemplateSettingsTab from '../components/TemplateSettingsTab.vue'

const activeTab = ref('build')
const itemSeparatorSetting = ref(window.localStorage.getItem('docsy.template.itemSeparator') || '、')
function saveItemSeparatorSetting() {
  window.localStorage.setItem('docsy.template.itemSeparator', itemSeparatorSetting.value || '、')
  ElMessage.success('已保存多项字段连接符设置')
}

// ── Settings: template trash + history database ─────────────────────────────
const templateTrash = ref([])
const templateTrashLoading = ref(false)
const clearingHistory = ref(false)
async function loadTemplateTrash() {
  templateTrashLoading.value = true
  const result = await tauriCallSafe('list_template_trash')
  templateTrashLoading.value = false
  if (result.ok) {
    templateTrash.value = result.data || []
  } else {
    ElMessage.error(result.error || '读取模板回收站失败')
  }
}
async function restoreTemplate(row) {
  const result = await tauriCallSafe('restore_template_from_trash', { args: { path: row.path } })
  if (!result.ok) {
    ElMessage.error(result.error || '恢复失败')
    return
  }
  ElMessage.success('模板已恢复')
  await loadTemplateTrash()
  window.dispatchEvent(new CustomEvent('docsy-template-library-changed'))
}
async function permanentlyDeleteTemplate(row) {
  let migrateToCommon = false
  try {
    await ElMessageBox.confirm(
      `彻底删除"${row.name}"？可以先把该模板的内部填写数据迁移为模板通用数据，供其他模板按通用字段名继续检索。`,
      '彻底删除模板',
      {
        confirmButtonText: '迁移数据并删除',
        cancelButtonText: '直接删除数据',
        distinguishCancelAndClose: true,
        type: 'warning',
      },
    )
    migrateToCommon = true
  } catch (action) {
    if (action !== 'cancel') return
  }
  const result = await tauriCallSafe('permanently_delete_template', {
    args: { path: row.path, migrateToCommon },
  })
  if (!result.ok) {
    ElMessage.error(result.error || '彻底删除失败')
    return
  }
  ElMessage.success(migrateToCommon ? '模板已删除，数据已迁移为模板通用数据' : '模板和内部数据已删除')
  await loadTemplateTrash()
  window.dispatchEvent(new CustomEvent('docsy-template-library-changed'))
}
async function clearAllHistory() {
  try {
    await ElMessageBox.confirm(
      '确定清空全部填写历史？删除后无法恢复，模板库文件不受影响。',
      '清空填写历史',
      { confirmButtonText: '清空', type: 'warning' },
    )
  } catch {
    return
  }
  clearingHistory.value = true
  const result = await tauriCallSafe('clear_template_history')
  clearingHistory.value = false
  if (!result.ok) {
    ElMessage.error(result.error || '清空历史失败')
    return
  }
  ElMessage.success(`已清空 ${result.data ?? 0} 条填写记录`)
  await loadHistoryContext(true)
  await loadTemplateHistoryRuns()
}

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
const sourcePreviewSelection = ref(null)
const sourcePreviewSelectionPayload = ref(null)
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
const previewFocusedRowId = ref('')
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
const batchProcessing = ref(false)
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

function structureTargetDisplayName(row) {
  const target = structureTargetRow(row)
  if (!target) return row.name || '未指定'
  return displayNameForFieldRow(target) || row.name || '未指定'
}

function displayNameForFieldRow(row) {
  if (!row) return ''
  const label = String(row.label || '').trim()
  const rawText = String(row.text || '').trim()
  if (label && label !== rawText && !isGeneratedFieldName(label)) return label
  return String(row.name || '').trim()
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

function isConnectorRow(row) {
  if (rowUsage(row) !== 'prefix') return false
  const info = prefixStructureInfo(row.text)
  return Boolean(info.connectorText && !info.roleText)
}

function connectorTargetName(row) {
  const target = connectorTargetRow(row)
  return target?.name || row.name || ''
}

function connectorTargetRow(row) {
  if (!isConnectorRow(row)) return null
  const index = fieldRows.value.indexOf(row)
  if (index < 0) return null
  return findNeighborFieldRow(fieldRows.value, index, 1)
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

function referenceSourceKey(mode = 'auto', source = '', sourceIndex = null) {
  return `${mode || 'auto'}::${source || ''}::${sourceIndex == null ? '' : sourceIndex}`
}

function parseReferenceSourceKey(key) {
  const parts = String(key || '').split('::')
  const [mode = 'auto', source = '', sourceIndexText = ''] =
    parts.length >= 3 ? parts : ['field', parts[0] || '', parts[1] || '']
  return {
    mode: mode || 'auto',
    sourceField: mode === 'field' ? source : '',
    sourceSemanticKey: mode === 'semantic' ? source : '',
    sourceIndex: sourceIndexText === '' ? null : Number(sourceIndexText),
  }
}

function onReferenceSelectionChange(field, key) {
  const source = parseReferenceFillKey(key)
  const values = normalizeValuesForReferenceSources()
  formValues[fieldFormKey(field)] = resolveReferenceValueFromSource(source, values)
  scheduleHistoryRefresh()
}

function syncReferenceSourceFromKey(row) {
  const parsed = parseReferenceSourceKey(row.referenceSourceKey)
  row.referenceSourceMode = parsed.mode
  row.referenceSourceField = parsed.sourceField
  row.referenceSourceSemanticKey = parsed.sourceSemanticKey
  row.referenceSourceIndex = parsed.sourceIndex
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

function markToRow(mark, index) {
  const inferred = inferFieldFromText(mark.text, mark.context, mark.checkboxLike, index)
  const partyItems = inferred.type === 'party_list' ? splitPartyLabelText(mark.text) : []
  return {
    rowId: `${mark.id}:${index}`,
    displayId: mark.displayId || mark.id,
    markId: mark.id,
    markRefs: mark.markRefs || [{ markId: mark.id, start: null, end: null }],
    markSegments: mark.markSegments || [{ markId: mark.id, text: mark.text }],
    charStart: null,
    charEnd: null,
    text: mark.text,
    context: mark.context,
    enabled: true,
    type: inferred.type,
    name: inferred.name,
    label: inferred.label,
    semanticKey: inferred.semanticKey,
    required: false,
    optionalWhenEmpty: inferred.optionalWhenEmpty,
    optionalScope: inferred.optionalScope,
    optionalPrefix: inferred.optionalPrefix,
    optionalSuffix: inferred.optionalSuffix,
    optionId: `option_${index + 1}`,
    optionLabel: mark.optionLabel || mark.text,
    checkedText: defaultCheckedText(mark.text),
    uncheckedText: defaultUncheckedText(mark.text),
    partyItems,
    referenceHintSeen: false,
    referenceIncludePrefix: true,
    referenceIncludeSuffix: true,
    referenceSourceMode: 'auto',
    referenceSourceField: '',
    referenceSourceSemanticKey: '',
    referenceSourceIndex: null,
    referenceSourceKey: referenceSourceKey('auto', '', null),
    options: inferred.options || [],
    selectOptions: (inferred.options || []).map((opt) => ({
      label: opt.label || '',
      checkedText: opt.checkedText || '',
    })),
  }
}

function autoAssignStructureTargets(rows) {
  for (const [index, row] of rows.entries()) {
    if (rowUsage(row) === 'prefix') {
      const target = findNeighborFieldRow(rows, index, 1)
      if (target) {
        const rule = prefixTargetRule(row.text)
        if (rule && isGeneratedFieldName(target.name)) {
          target.type = rule.targetType
          target.name = rule.targetName
          target.label = rule.targetLabel
          target.semanticKey = rule.targetSemanticKey || rule.targetName
        }
        bindStructureRowToTarget(row, target)
        row.name = rule?.targetName || target.name
        row.label = prefixStructureLabel(row, target)
      }
    } else if (rowUsage(row) === 'suffix') {
      const target = findNeighborFieldRow(rows, index, -1)
      if (target) {
        const rule = suffixTargetRule(row.text)
        if (rule && isGeneratedFieldName(target.name)) {
          target.type = rule.targetType
          target.name = rule.targetName
          target.label = rule.targetLabel
          target.semanticKey = rule.targetSemanticKey || rule.targetName
        }
        bindStructureRowToTarget(row, target)
        row.name = rule?.targetName || target.name
        row.label = `${target.label || target.name}后缀`
      }
    }
  }
  return rows
}

function refreshPartyItemsForRows(rows) {
  for (const row of rows) {
    row.partyItems = row.type === 'party_list' ? splitPartyLabelText(row.text) : []
  }
  return rows
}

function normalizeFieldRows(rows) {
  return reorderRowsByDocumentPosition(
    refreshPartyItemsForRows(
      autoSplitPartyListRows(
        autoAssignStructureTargets(
          dropGeneratedConnectorRows(
            autoSplitLegalCompoundRows(autoSplitKnownSuffixRows(autoSplitLeadingConnectorRows(rows))),
          ),
        ),
      ),
    ),
  )
}

function autoSplitPartyListRows(rows) {
  const result = []
  for (const row of rows) {
    if (rowUsage(row) !== 'field' || row.type !== 'party_list') {
      result.push(row)
      continue
    }
    const segments = splitPartyLabelSegments(row.text)
    if (segments.length <= 1) {
      result.push(row)
      continue
    }
    for (const [index, segment] of segments.entries()) {
      const refs = markRefsForTextRange(row, segment.start, segment.end)
      result.push({
        ...row,
        rowId: `${row.rowId}:party-item:${index}`,
        text: segment.text,
        label: segment.text,
        markRefs: refs,
        charStart: segment.start,
        charEnd: segment.end,
        markSegments: markSegmentsFromRefs(refs, segment.text),
        partyItems: [segment.text],
      })
    }
  }
  return result
}

function dropGeneratedConnectorRows(rows) {
  return rows.filter((row) => !(isGeneratedConnectorRow(row) && isPureConnectorText(row.text)))
}

function isGeneratedConnectorRow(row) {
  return rowUsage(row) === 'prefix' && String(row.rowId || '').includes(':auto-connector')
}

function isPureConnectorText(text) {
  const value = String(text || '').trim()
  return Boolean(value) && /^(?:以及|或者|[，,、;；和与及\s])+$/.test(value)
}

function autoSplitLeadingConnectorRows(rows) {
  const result = []
  for (const row of rows) {
    const split = splitLeadingConnectorRow(row)
    if (split) result.push(...split)
    else result.push(row)
  }
  return result
}

function splitLeadingConnectorRow(row) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  const info = leadingConnectorInfo(row.text)
  if (!info.connectorText || !info.restText) return null
  const connectorLength = charLength(info.connectorText)
  const textLength = charLength(row.text)
  const inferred = inferFieldFromText(info.restText, row.context, false, fieldRows.value.length)
  const fieldType = row.userSelectedType || inferred.type || row.type
  const fieldName = !row.name || isGeneratedFieldName(row.name) ? inferred.name : row.name
  const fieldLabel =
    !row.label || row.label === row.text || isGeneratedFieldName(row.label) ? inferred.label : row.label
  const fieldRefs = refsForRowTextRange(row, connectorLength, textLength)
  const fieldText = info.restText
  return [
    {
      ...row,
      rowId: `${row.rowId}:auto-field`,
      text: fieldText,
      type: fieldType,
      name: fieldName,
      label: fieldLabel,
      semanticKey: inferred.semanticKey || fieldName,
      markRefs: fieldRefs,
      charStart: connectorLength,
      charEnd: textLength,
      markSegments: markSegmentsFromRefs(fieldRefs, fieldText),
      partyItems: fieldType === 'party_list' ? splitPartyLabelText(fieldText) : [],
    },
  ]
}

function autoSplitKnownSuffixRows(rows) {
  const result = []
  for (const row of rows) {
    const split = splitKnownSuffixRow(row)
    if (split) result.push(...split)
    else result.push(row)
  }
  return result
}

function autoSplitLegalCompoundRows(rows) {
  const result = []
  for (const row of rows) {
    const split = splitLegalCompoundRow(row)
    if (split) result.push(...split)
    else result.push(row)
  }
  return result
}

function splitKnownSuffixRow(row) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  if (row.userSelectedType) return null
  if (!row.markId || !row.text) return null
  const suffixRule = knownSuffixAtEnd(row.text)
  if (!suffixRule) return null
  const suffixText = suffixRule.text
  const trimmedText = String(row.text).trimEnd()
  const suffixStartOffset = trimmedText.length - suffixText.length
  const rawFieldText = trimmedText.slice(0, suffixStartOffset)
  const fieldText = rawFieldText.trim()
  if (!fieldText || fieldText.length > 12) return null
  const fieldStartOffset = rawFieldText.length - rawFieldText.trimStart().length
  const fieldStart = charLength(trimmedText.slice(0, fieldStartOffset))
  const fieldEnd = fieldStart + charLength(fieldText)
  const suffixStart = charLength(trimmedText.slice(0, suffixStartOffset))
  const suffixEnd = suffixStart + charLength(suffixText)
  const fieldRow = {
    ...row,
    rowId: `${row.rowId}:auto-field`,
    text: fieldText,
    type: suffixRule.targetType,
    name: suffixRule.targetName,
    label: suffixRule.targetLabel,
    semanticKey: suffixRule.targetName,
    markRefs: markRefsForTextRange(row, fieldStart, fieldEnd),
    charStart: fieldStart,
    charEnd: fieldEnd,
  }
  const suffixRow = {
    ...row,
    rowId: `${row.rowId}:auto-suffix`,
    text: suffixText,
    type: 'suffix',
    name: suffixRule.targetName,
    label: `${suffixRule.targetLabel}后缀`,
    semanticKey: '',
    required: false,
    optionalWhenEmpty: false,
    markRefs: markRefsForTextRange(row, suffixStart, suffixEnd),
    charStart: suffixStart,
    charEnd: suffixEnd,
  }
  return [fieldRow, suffixRow]
}

function splitLegalCompoundRow(row) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  if (row.userSelectedType) return null
  const text = String(row.text || '')
  if (!text || charLength(text) < 8) return null

  const pieces = []
  const causeMatch = text.match(/[\u4e00-\u9fa5A-Za-z0-9、，,（）()·]{2,48}纠纷/)
  if (causeMatch) {
    const start = charLength(text.slice(0, causeMatch.index))
    const end = start + charLength(causeMatch[0])
    pieces.push(
      makeRangeRow(row, start, end, { type: 'text', name: '案由', label: '案由', semanticKey: '案由' }, 'cause'),
    )
    const afterCause = text.slice(causeMatch.index + causeMatch[0].length)
    if (afterCause.startsWith('一案')) {
      pieces.push(
        makeRangeRow(
          row,
          end,
          end + 2,
          { type: 'ignore', name: '', label: '保留原文', semanticKey: '' },
          'cause-ignore',
        ),
      )
    }
  }

  const caseMatch = text.match(
    /[（(]\s*\d{4}\s*[）)]\s*[\u4e00-\u9fa5A-Za-z0-9]{1,12}(?:民|行|知|执|赔|破|清|申|再|终|初|保|诉前|民终|民初|行初|行终)[\u4e00-\u9fa5A-Za-z0-9-]*号/,
  )
  if (caseMatch) {
    const caseStart = charLength(text.slice(0, caseMatch.index))
    const caseEnd = caseStart + charLength(caseMatch[0])
    const prefixStart = legalCaseNumberPrefixStart(text, caseMatch.index)
    if (prefixStart >= 0 && prefixStart < caseMatch.index) {
      pieces.push(
        makeRangeRow(
          row,
          charLength(text.slice(0, prefixStart)),
          caseStart,
          { type: 'prefix', name: '案号', label: '案号前缀', semanticKey: '' },
          'case-prefix',
        ),
      )
    }
    pieces.push(
      makeRangeRow(
        row,
        caseStart,
        caseEnd,
        { type: 'text', name: '案号', label: '案号', semanticKey: '案号' },
        'case-number',
      ),
    )
    const closeChar = text.slice(caseMatch.index + caseMatch[0].length, caseMatch.index + caseMatch[0].length + 1)
    if (closeChar === '）' || closeChar === ')') {
      pieces.push(
        makeRangeRow(
          row,
          caseEnd,
          caseEnd + 1,
          { type: 'suffix', name: '案号', label: '案号后缀', semanticKey: '' },
          'case-suffix',
        ),
      )
    }
  }

  if (pieces.length <= 1) return null
  return dedupeRangeRows(pieces)
}

function legalCaseNumberPrefixStart(text, caseNumberIndex) {
  const before = text.slice(0, caseNumberIndex)
  const labelIndex = Math.max(before.lastIndexOf('案号：'), before.lastIndexOf('案号:'))
  if (labelIndex < 0) return -1
  const bracketIndex = Math.max(before.lastIndexOf('（', labelIndex), before.lastIndexOf('(', labelIndex))
  return bracketIndex >= 0 && labelIndex - bracketIndex <= 2 ? bracketIndex : labelIndex
}

function makeRangeRow(row, start, end, attrs, suffix) {
  return {
    ...row,
    ...attrs,
    rowId: `${row.rowId}:auto-${suffix}`,
    text: sliceChars(row.text, start, end),
    markRefs: markRefsForTextRange(row, start, end),
    charStart: start,
    charEnd: end,
    required: false,
    optionalWhenEmpty: false,
    referenceHintSeen: false,
    partyItems: attrs.type === 'party_list' ? splitPartyLabelText(sliceChars(row.text, start, end)) : [],
  }
}

function dedupeRangeRows(rows) {
  const seen = new Set()
  return rows.filter((row) => {
    const key = `${row.charStart}:${row.charEnd}:${row.type}:${row.name}`
    if (seen.has(key)) return false
    seen.add(key)
    return row.charEnd > row.charStart
  })
}

function markRefsForTextRange(row, start, end) {
  const segments = row.markSegments?.length ? row.markSegments : [{ markId: row.markId, text: row.text }]
  const refs = []
  let cursor = 0
  for (const segment of segments) {
    const length = charLength(segment.text)
    const segmentStart = cursor
    const segmentEnd = cursor + length
    const overlapStart = Math.max(start, segmentStart)
    const overlapEnd = Math.min(end, segmentEnd)
    if (overlapEnd > overlapStart && segment.markId) {
      refs.push({
        markId: segment.markId,
        start: overlapStart - segmentStart,
        end: overlapEnd - segmentStart,
      })
    }
    cursor = segmentEnd
  }
  if (!refs.length && row.markId) refs.push({ markId: row.markId, start, end })
  return refs
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

function charLength(text) {
  return [...String(text || '')].length
}

function knownSuffixAtEnd(text) {
  const value = String(text || '').trim()
  const suffixes = ['诉讼代理人', '实习律师', '代理人', '律师']
  for (const suffix of suffixes) {
    if (value.endsWith(suffix)) {
      const rule = suffixTargetRule(suffix)
      if (rule) return { ...rule, text: suffix }
    }
  }
  return null
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

function prefixStructureLabel(row, target) {
  return `${displayNameForFieldRow(target) || target.name}前缀`
}

function autoMergeMarks(rawMarks) {
  const rows = []
  let current = null
  for (const mark of rawMarks || []) {
    const normalized = {
      ...mark,
      markRefs: [{ markId: mark.id, start: null, end: null }],
      markSegments: [{ markId: mark.id, text: mark.text }],
    }
    if (shouldMergeMark(current, normalized)) {
      current.text += normalized.text
      current.markRefs.push(...normalized.markRefs)
      current.markSegments.push(...normalized.markSegments)
      current.displayId = `${current.displayId || current.id}+${normalized.id}`
      current.checkboxLike = current.checkboxLike && normalized.checkboxLike
    } else {
      if (current) rows.push(current)
      current = normalized
    }
  }
  if (current) rows.push(current)
  return rows
}

function shouldMergeMark(current, next) {
  if (!current || !next) return false
  if (current.checkboxLike || next.checkboxLike) return false
  if (current.part !== next.part || current.context !== next.context) return false
  if (!Number.isFinite(current.runIndex) || !Number.isFinite(next.runIndex)) return false
  if (next.runIndex !== current.runIndex + current.markRefs.length) return false
  return shouldMergeText(current.text, next.text)
}

function shouldMergeText(left, right) {
  const combined = `${left}${right}`
  if (looksLikePrefixMark(left) || looksLikeSuffixMark(right)) return false
  if (isDateLikePrefix(combined)) return true
  if (isPunctuationFragment(right) || isPunctuationFragment(left)) return true
  if (/[A-Za-z0-9]$/.test(left) || /^[A-Za-z0-9]/.test(right)) return true
  if (isShortChineseFragment(left) || isShortChineseFragment(right)) return true
  return false
}

function isShortChineseFragment(text) {
  const value = String(text || '')
  return /^[\u4e00-\u9fa5]{1,4}$/.test(value)
}

function isPunctuationFragment(text) {
  return /^[（()）.,，.、:：;；\s_-]+$/.test(String(text || ''))
}

function isDateLikePrefix(text) {
  return looksLikeDatePrefix(text)
}

function inferFieldFromText(text, context, checkboxLike, index) {
  const publicInference = inferTemplateField({ text, context, checkboxLike, index })
  if (checkboxLike) {
    return publicInference
  }
  if (isLikelyPrefixMark(text)) {
    return {
      type: 'prefix',
      name: '',
      label: '前缀',
      semanticKey: '',
      optionalWhenEmpty: false,
      optionalScope: 'position',
      optionalPrefix: '',
      optionalSuffix: '',
    }
  }
  if (isLikelySuffixMark(text)) {
    return {
      type: 'suffix',
      name: '',
      label: '后缀',
      semanticKey: '',
      optionalWhenEmpty: false,
      optionalScope: 'position',
      optionalPrefix: '',
      optionalSuffix: '',
    }
  }
  if (publicInference?.name && !isGeneratedFieldName(publicInference.name)) {
    return publicInference
  }
  return {
    type: 'text',
    name: `字段${index + 1}`,
    label: text,
    semanticKey: `字段${index + 1}`,
    optionalWhenEmpty: false,
    optionalScope: 'position',
    optionalPrefix: '',
    optionalSuffix: '',
  }
}

function isGeneratedFieldName(name) {
  return /^field_\d+$/.test(String(name || '')) || /^字段\d+$/.test(String(name || ''))
}

function isLikelyPrefixMark(text) {
  return looksLikePrefixMark(text)
}

function isLikelySuffixMark(text) {
  return looksLikeSuffixMark(text)
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

function reorderRowsByDocumentPosition(rows) {
  const runOrder = new Map()
  for (const [index, run] of documentRuns.value.entries()) {
    runOrder.set(run.id, Number.isFinite(run.runIndex) ? run.runIndex : index)
  }
  return [...rows]
    .map((row, index) => ({ row, index }))
    .sort((a, b) => rowDocumentOrder(a.row, runOrder) - rowDocumentOrder(b.row, runOrder) || a.index - b.index)
    .map((item) => item.row)
}

function rowDocumentOrder(row, runOrder) {
  const refs = row.markRefs?.length ? row.markRefs : [{ markId: row.markId }]
  let min = Number.POSITIVE_INFINITY
  for (const ref of refs) {
    if (!ref?.markId || !runOrder.has(ref.markId)) continue
    min = Math.min(min, runOrder.get(ref.markId))
  }
  return Number.isFinite(min) ? min : Number.MAX_SAFE_INTEGER
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

function buildTemplatePreview(runs, fallbackText, rows, sampleValues = {}) {
  if (!runs?.length) return buildTextFallbackPreview(fallbackText)
  const original = []
  const rendered = []
  let segmentIndex = 0
  let lastParagraph = null
  const rangesByRun = previewRangesByRun(rows)
  for (const run of runs) {
    if (lastParagraph !== null && run.paragraphIndex !== lastParagraph) {
      original.push(previewPlainSegment('\n', segmentIndex++))
      rendered.push(previewPlainSegment('\n', segmentIndex++))
    }
    lastParagraph = run.paragraphIndex
    const runText = String(run.text || '')
    const ranges = rangesByRun.get(run.id) || []
    let cursor = 0
    for (const range of ranges) {
      const start = Math.max(0, Math.min(charLength(runText), range.start ?? 0))
      const end = Math.max(start, Math.min(charLength(runText), range.end ?? charLength(runText)))
      if (end <= cursor) continue
      const visibleStart = Math.max(cursor, start)
      if (cursor < visibleStart) {
        const plain = previewRunSegment(run, cursor, visibleStart, null, segmentIndex++)
        original.push(plain)
        rendered.push({ ...plain, id: `rendered-${plain.id}` })
      }
      const marked = previewRunSegment(run, visibleStart, end, range.row, segmentIndex++)
      original.push({
        ...marked,
        text: range.occurrence === 0 ? previewSourceLabel(range.row) : '',
        deleted: range.occurrence > 0,
      })
      const replacementText = range.occurrence === 0 ? previewReplacementText(range.row, sampleValues) : ''
      rendered.push({
        ...marked,
        id: `rendered-${marked.id}`,
        text: replacementText,
        deleted: rowUsage(range.row) === 'delete_text' || !replacementText || range.occurrence > 0,
      })
      cursor = end
    }
    if (cursor < charLength(runText)) {
      const tail = previewRunSegment(run, cursor, charLength(runText), null, segmentIndex++)
      original.push(tail)
      rendered.push({ ...tail, id: `rendered-${tail.id}` })
    }
  }
  return { original, rendered }
}

function buildTextFallbackPreview(text) {
  const source = String(text || '')
  if (!source) return { original: [], rendered: [] }
  return { original: [previewPlainSegment(source, 0)], rendered: [previewPlainSegment(source, 1)] }
}

function previewRangesByRun(rows) {
  const map = new Map()
  const occurrenceByRow = new Map()
  for (const row of rows) {
    if (!row.enabled) continue
    const refs = row.markRefs?.length
      ? row.markRefs
      : [
          {
            markId: row.markId,
            start: row.charStart,
            end: row.charEnd,
          },
        ]
    for (const ref of refs) {
      if (!ref?.markId) continue
      if (!map.has(ref.markId)) map.set(ref.markId, [])
      const occurrenceKey = row.rowId
      const occurrence = occurrenceByRow.get(occurrenceKey) || 0
      occurrenceByRow.set(occurrenceKey, occurrence + 1)
      map.get(ref.markId).push({ row, start: ref.start ?? 0, end: ref.end ?? undefined, occurrence })
    }
  }
  for (const ranges of map.values()) {
    ranges.sort((a, b) => (a.start ?? 0) - (b.start ?? 0))
  }
  return map
}

function previewRunSegment(run, start, end, row, index) {
  return {
    id: `run-${run.id}-${start}-${end}-${index}`,
    text: sliceChars(run.text, start, end),
    row,
    runId: run.id,
    start,
    end,
    bold: run.bold,
    italic: run.italic,
    underline: run.underline,
  }
}

function previewPlainSegment(text, index) {
  return { id: `plain-${index}`, text, row: null, runId: '', start: 0, end: charLength(text) }
}

function previewSourceLabel(row) {
  if (rowUsage(row) === 'ignore') return '【保留原文】'
  if (rowUsage(row) === 'delete_text') return `【删除：${row.text}】`
  if (rowUsage(row) === 'prefix')
    return `【${isConnectorRow(row) ? '连接符' : '前缀'}：${structureTargetDisplayName(row)}】`
  if (rowUsage(row) === 'suffix') return `【后缀：${structureTargetDisplayName(row)}】`
  if (row.type === 'reference') return `【引用：${row.name || '引用'}】`
  return `【${row.name || row.label || row.text}】`
}

function previewReplacementText(row, sampleValues = {}) {
  const usage = rowUsage(row)
  if (usage === 'ignore') return row.text
  if (usage === 'delete_text') return ''
  const hasSample = hasPreviewSampleValue(sampleValues, row.name)
  if (usage === 'prefix' || usage === 'suffix')
    return hasSample && isEmptyPreviewValue(sampleValues[row.name]) ? '' : row.text
  if (row.type === 'reference') return previewReferenceValue(row, sampleValues) || row.text
  const value = sampleValues[row.name]
  if (row.type === 'date' && !isEmptyPreviewValue(value)) return normalizePreviewDate(value)
  if (!isEmptyPreviewValue(value)) return Array.isArray(value) ? value.join('、') : String(value)
  return row.text
}

function previewReferenceValue(row, sampleValues = {}) {
  const source = normalizedReferenceSource(row)
  if (source.mode === 'auto') return sampleValues[row.name] || ''
  const value = source.mode === 'semantic' ? sampleValues[source.sourceSemanticKey] : sampleValues[source.sourceField]
  if (Array.isArray(value)) {
    return source.sourceIndex == null ? value.join('、') : String(value[source.sourceIndex] || '')
  }
  return source.sourceIndex == null && !isEmptyPreviewValue(value) ? String(value) : ''
}

function hasPreviewSampleValue(sampleValues, name) {
  return Object.prototype.hasOwnProperty.call(sampleValues || {}, name)
}

function isEmptyPreviewValue(value) {
  return value == null || value === '' || (Array.isArray(value) && value.length === 0)
}

function normalizePreviewDate(value) {
  const raw = String(value || '').trim()
  if (!raw) return ''
  if (/今天|今日/.test(raw)) return todayText()
  const compact = raw.replace(/\s+/g, '')
  const ymd = compact.match(/^(\d{4})(\d{2})(\d{2})$/)
  if (ymd) return `${Number(ymd[1])}年${Number(ymd[2])}月${Number(ymd[3])}日`
  const md = compact.match(/^(\d{2})(\d{2})$/)
  if (md) return `${new Date().getFullYear()}年${Number(md[1])}月${Number(md[2])}日`
  const cn = compact.match(/^(\d{4})年(\d{1,2})月(\d{1,2})日?$/)
  if (cn) return `${Number(cn[1])}年${Number(cn[2])}月${Number(cn[3])}日`
  const dashed = compact.match(/^(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})$/)
  if (dashed) return `${Number(dashed[1])}年${Number(dashed[2])}月${Number(dashed[3])}日`
  return raw
}

function sliceChars(text, start, end) {
  return [...String(text || '')].slice(start, end).join('')
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

function nextReferenceFieldName() {
  const base = '引用'
  const used = new Set(
    fieldRows.value
      .filter((row) => row.enabled && rowUsage(row) === 'field')
      .map((row) => String(row.name || '').trim())
      .filter(Boolean),
  )
  if (!used.has(base)) return base
  for (let index = 2; index < 1000; index += 1) {
    const candidate = `${base}${index}`
    if (!used.has(candidate)) return candidate
  }
  return `${base}${Date.now()}`
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

function leadingConnectorInfo(text) {
  const value = String(text || '')
  const match = value.match(/^(?:以及|或者|[，,、;；和与及\s])+/)
  const connectorText = match?.[0] || ''
  return {
    connectorText,
    restText: connectorText ? value.slice(connectorText.length).trim() : '',
  }
}

function rememberSourcePreviewSelection() {
  const selection = window.getSelection?.()
  if (!selection || selection.rangeCount === 0 || selection.isCollapsed) return
  const range = selection.getRangeAt(0)
  const resolved = resolvePreviewSelection(range)
  if (!resolved.text) return
  sourcePreviewSelection.value = range.cloneRange()
  sourcePreviewSelectionPayload.value = resolved
}

function collectSourcePreviewSelection() {
  const selection = window.getSelection?.()
  if (selection && selection.rangeCount > 0 && !selection.isCollapsed) {
    const liveRange = selection.getRangeAt(0)
    const resolved = resolvePreviewSelection(liveRange)
    if (resolved.refs.length) {
      sourcePreviewSelection.value = liveRange.cloneRange()
      sourcePreviewSelectionPayload.value = resolved
      return resolved
    }
  }

  if (sourcePreviewSelection.value) {
    const resolved = resolvePreviewSelection(sourcePreviewSelection.value)
    if (resolved.refs.length) {
      sourcePreviewSelectionPayload.value = resolved
      return resolved
    }
  }

  return sourcePreviewSelectionPayload.value || { text: '', refs: [], context: '' }
}

function resolvePreviewSelection(range) {
  const sourceRoot = buildTabRef.value?.sourcePreviewRef
  if (sourceRoot && rangeIntersectsRoot(range, sourceRoot)) return resolveSourcePreviewRange(sourceRoot, range)
  const documentRoot = buildTabRef.value?.documentPreviewRef
  if (documentRoot && rangeIntersectsRoot(range, documentRoot)) return resolveDocumentPreviewRange(documentRoot, range)
  return { text: '', refs: [], context: '' }
}

function resolveSourcePreviewRange(root, range) {
  const stream = sourcePreviewTextStream(root)
  const selected = sourcePreviewSelectionParts(root, range)
  const text = selected.text || range.toString()
  if (!text) return { text: '', refs: [], context: '' }
  return {
    text,
    refs: selected.refs,
    context: documentText.value || stream.text || text,
  }
}

function resolveDocumentPreviewRange(root, range) {
  const stream = documentPreviewTextStream()
  if (!stream.text) return { text: '', refs: [], context: '' }
  const start = previewRootOffset(root, range, true)
  const end = previewRootOffset(root, range, false)
  const streamLength = charLength(stream.text)
  const normalizedStart = Math.max(0, Math.min(streamLength, Math.min(start, end)))
  const normalizedEnd = Math.max(normalizedStart, Math.min(streamLength, Math.max(start, end)))
  const text = sliceChars(stream.text, normalizedStart, normalizedEnd)
  const refs = refsForPreviewTextStreamRange(stream, normalizedStart, normalizedEnd)
  const fallbackText = text || range.toString()
  if (!fallbackText) return { text: '', refs: [], context: '' }
  return {
    text: fallbackText,
    refs,
    context: documentText.value || stream.text || fallbackText,
  }
}

function documentPreviewTextStream() {
  const chunks = []
  let text = ''
  let cursor = 0
  let lastParagraph = null
  for (const run of documentRuns.value || []) {
    if (lastParagraph !== null && run.paragraphIndex !== lastParagraph) {
      text += '\n'
      cursor += 1
    }
    lastParagraph = run.paragraphIndex
    const value = String(run.text || '')
    const start = cursor
    const end = start + charLength(value)
    chunks.push({
      text: value,
      streamStart: start,
      streamEnd: end,
      runId: run.id,
      runStart: 0,
    })
    text += value
    cursor = end
  }
  return { text, chunks }
}

function previewRootOffset(root, range, useStart) {
  const container = useStart ? range.startContainer : range.endContainer
  const offset = useStart ? range.startOffset : range.endOffset
  if (!root.contains(container)) return useStart ? 0 : charLength(root.textContent || '')
  const before = document.createRange()
  before.selectNodeContents(root)
  before.setEnd(container, offset)
  return charLength(before.toString())
}

function rangeIntersectsRoot(range, root) {
  if (root.contains(range.commonAncestorContainer)) return true
  return root.contains(range.startContainer) || root.contains(range.endContainer)
}

function sourcePreviewTextStream(root) {
  const chunks = []
  let text = ''
  let cursor = 0
  for (const node of root.querySelectorAll('[data-run-id]')) {
    const value = node.textContent || ''
    const start = cursor
    const end = start + charLength(value)
    chunks.push({
      node,
      text: value,
      streamStart: start,
      streamEnd: end,
      runId: node.dataset.runId,
      runStart: Number(node.dataset.start || 0),
    })
    text += value
    cursor = end
  }
  return { text, chunks }
}

function refsForPreviewTextStreamRange(stream, start, end) {
  const refs = []
  for (const chunk of stream.chunks) {
    if (!chunk.runId) continue
    const overlapStart = Math.max(start, chunk.streamStart)
    const overlapEnd = Math.min(end, chunk.streamEnd)
    if (overlapEnd <= overlapStart) continue
    refs.push({
      markId: chunk.runId,
      start: chunk.runStart + overlapStart - chunk.streamStart,
      end: chunk.runStart + overlapEnd - chunk.streamStart,
    })
  }
  return refs
}

function sourcePreviewSelectionParts(root, range) {
  const refs = []
  const texts = []
  for (const node of root.querySelectorAll('[data-run-id]')) {
    if (!rangeIntersectsNode(range, node)) continue
    const part = selectedNodeTextPart(range, node)
    if (!part.text) continue
    const runStart = Number(node.dataset.start || 0)
    refs.push({
      markId: node.dataset.runId,
      start: runStart + part.start,
      end: runStart + part.end,
    })
    texts.push(part.text)
  }
  return { text: texts.join(''), refs }
}

function rangeIntersectsNode(range, node) {
  try {
    return range.intersectsNode(node)
  } catch {
    return false
  }
}

function selectedNodeTextPart(range, node) {
  const nodeRange = document.createRange()
  nodeRange.selectNodeContents(node)
  const overlap = range.cloneRange()
  if (overlap.compareBoundaryPoints(window.Range.START_TO_START, nodeRange) < 0) {
    overlap.setStart(nodeRange.startContainer, nodeRange.startOffset)
  }
  if (overlap.compareBoundaryPoints(window.Range.END_TO_END, nodeRange) > 0) {
    overlap.setEnd(nodeRange.endContainer, nodeRange.endOffset)
  }
  const text = overlap.toString()
  if (!text) return { text: '', start: 0, end: 0 }
  const before = document.createRange()
  before.selectNodeContents(node)
  before.setEnd(overlap.startContainer, overlap.startOffset)
  const start = charLength(before.toString())
  return { text, start, end: start + charLength(text) }
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

function validateFieldRowsBeforeSave() {
  // Every yellow run must remain represented by a confirmed row, including
  // prefixes, suffixes and "保留原文" rows. Otherwise saving clears its yellow
  // highlight but leaves its sample text in the generated template.
  const coveredMarks = new Set()
  for (const row of fieldRows.value) {
    if (!row.enabled) continue
    for (const ref of row.markRefs || []) {
      if (ref?.markId) coveredMarks.add(ref.markId)
    }
    if (row.markId) coveredMarks.add(row.markId)
  }
  const missingMarks = marks.value.filter((mark) => !coveredMarks.has(mark.id))
  if (missingMarks.length) {
    const samples = missingMarks
      .slice(0, 3)
      .map((mark) => `"${mark.text}"`)
      .join('、')
    return `仍有 ${missingMarks.length} 处标黄文本未处理：${samples}。请设为字段、前缀、后缀、保留原文或删除文本后再保存`
  }
  for (const row of fieldRows.value) {
    if (!row.enabled || rowUsage(row) === 'ignore') continue
    if (rowUsage(row) === 'delete_text') continue
    const currentName = effectiveRowName(row)
    if (!currentName.trim()) {
      return `"${row.text}"还没有填写字段名`
    }
    if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') {
      const targets = fieldRows.value.filter(
        (target) => target.enabled && rowUsage(target) === 'field' && target.name.trim() === currentName.trim(),
      )
      if (!targets.length) {
        return `"${row.text}"设为${rowUsage(row) === 'prefix' ? '前缀' : '后缀'}，但找不到同名字段`
      }
      if (targets.some((target) => isMarkerType(target.type))) {
        return `"${row.text}"不能挂到勾选字段上，请改成文本、日期、下拉或当事人列表字段`
      }
    }
  }
  return ''
}

function buildFields() {
  const rows = fieldRows.value.filter(
    (row) =>
      row.enabled &&
      ((rowUsage(row) === 'field' && effectiveRowName(row).trim()) ||
        (rowUsage(row) === 'delete_text' && row.markRefs?.length)),
  )
  const byKey = new Map()
  for (const row of rows) {
    const type = row.type
    const currentName = effectiveRowName(row)
    const referenceSource = row.type === 'reference' ? normalizedReferenceSource(row) : null
    const key =
      rowUsage(row) === 'delete_text'
        ? `${type}:${row.rowId}`
        : row.type === 'reference'
          ? `${type}:${currentName.trim()}:${referenceSource?.mode || 'auto'}:${referenceSource?.sourceField || referenceSource?.sourceSemanticKey || ''}:${referenceSource?.sourceIndex ?? ''}`
          : `${type}:${currentName.trim()}`
    if (!byKey.has(key)) {
      byKey.set(key, {
        id: stableFieldId(
          rowUsage(row) === 'delete_text'
            ? row.rowId
            : row.type === 'reference'
              ? `${currentName}:${referenceSource?.mode || 'auto'}:${referenceSource?.sourceField || referenceSource?.sourceSemanticKey || ''}:${referenceSource?.sourceIndex ?? ''}`
              : currentName,
          type,
        ),
        name: rowUsage(row) === 'delete_text' ? `delete_${row.rowId}` : currentName.trim(),
        label: manifestLabelForRow(row, currentName),
        semanticKey: rowUsage(row) === 'delete_text' ? '' : row.semanticKey.trim() || currentName.trim(),
        type,
        required: row.required,
        marks: [],
        markRefs: [],
        optionalRule: null,
        options: [],
        fillAllPositions: false,
        reference:
          row.type === 'reference'
            ? {
                sourceMode: referenceSource?.mode || 'auto',
                sourceField: referenceSource?.sourceField || '',
                sourceSemanticKey: referenceSource?.sourceSemanticKey || '',
                sourceIndex: referenceSource?.sourceIndex ?? null,
              }
            : null,
      })
    } else {
      // Several independent rows merged into one field: the value must fill
      // every position (single-row multiple refs stay single-valued).
      const existing = byKey.get(key)
      if (existing && !isMarkerType(type) && rowUsage(row) !== 'delete_text') {
        existing.fillAllPositions = true
      }
    }
    const field = byKey.get(key)
    if (row.optionalWhenEmpty && row.optionalScope === 'field' && !field.optionalRule) {
      field.optionalRule = {
        enabled: true,
        removeEmptyPrefix: row.optionalPrefix || '',
        removeEmptySuffix: row.optionalSuffix || '',
      }
    }
    if (type === 'select') {
      const src = row.selectOptions?.length ? row.selectOptions : row.options
      if (src?.length && field.options.length === 0) {
        field.options = src
          .filter((opt) => (opt.label || opt.checkedText || '').trim())
          .map((opt, i) => ({
            id: `opt_${i + 1}`,
            label: opt.label || opt.checkedText || '',
            checkedText: opt.checkedText || opt.label || '',
            uncheckedText: '',
            markerMarkId: '',
            markerTag: '',
          }))
      }
    }
    if (isMarkerType(type)) {
      field.options.push({
        id: row.optionId.trim() || `option_${field.options.length + 1}`,
        label: row.optionLabel.trim() || row.text,
        markerMarkId: row.markId,
        checkedText: row.checkedText || '☑',
        uncheckedText: row.uncheckedText || '☐',
      })
    } else {
      const refs = row.markRefs?.length
        ? row.markRefs
        : [
            {
              markId: row.markId,
              start: row.charStart,
              end: row.charEnd,
            },
          ]
      const structuralRule = structuralOptionalRuleForRow(row)
      for (const markRef of refs) {
        const normalizedRef = { ...markRef }
        if (row.optionalWhenEmpty && row.optionalScope !== 'field') {
          normalizedRef.optionalRule = {
            enabled: true,
            removeEmptyPrefix: row.optionalPrefix || '',
            removeEmptySuffix: row.optionalSuffix || '',
          }
        } else if (structuralRule.enabled) {
          normalizedRef.optionalRule = structuralRule
        }
        field.marks.push(normalizedRef.markId)
        field.markRefs.push(normalizedRef)
      }
    }
  }
  return Array.from(byKey.values())
}

function normalizedReferenceSource(row) {
  if (row.referenceSourceField || row.referenceSourceSemanticKey || row.referenceSourceMode) {
    return {
      mode:
        row.referenceSourceMode ||
        (row.referenceSourceSemanticKey ? 'semantic' : row.referenceSourceField ? 'field' : 'auto'),
      sourceField: row.referenceSourceField,
      sourceSemanticKey: row.referenceSourceSemanticKey || '',
      sourceIndex: row.referenceSourceIndex == null ? null : row.referenceSourceIndex,
    }
  }
  return parseReferenceSourceKey(row.referenceSourceKey)
}

function manifestLabelForRow(row, currentName) {
  const name = String(currentName || '').trim()
  if (isGeneratedFieldName(name)) return name
  if (rowUsage(row) === 'delete_text') return safeExplicitLabel(row, name)
  if (row.type === 'party_list') return name || row.label?.trim() || row.text
  return safeExplicitLabel(row, name)
}

function safeExplicitLabel(row, fallbackName) {
  const label = String(row?.label || '').trim()
  const rawText = String(row?.text || '').trim()
  if (!label || label === rawText) return fallbackName || ''
  return label
}

function structuralOptionalRuleForRow(fieldRow) {
  const prefix = []
  const suffix = []
  for (const row of fieldRows.value) {
    const currentName = effectiveRowName(row)
    if (!row.enabled || !currentName.trim() || currentName.trim() !== fieldRow.name.trim()) continue
    if (rowUsage(row) === 'prefix' && structureRowTargetsField(row, fieldRow)) {
      prefix.push(row.text)
    } else if (rowUsage(row) === 'suffix' && structureRowTargetsField(row, fieldRow)) {
      suffix.push(row.text)
    }
  }
  return {
    enabled: Boolean(prefix.length || suffix.length),
    removeEmptyPrefix: prefix.join(''),
    removeEmptySuffix: suffix.join(''),
  }
}

function effectiveRowName(row) {
  if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') return structureTargetRow(row)?.name || row?.name || ''
  if (isConnectorRow(row)) return connectorTargetName(row)
  return row?.name || ''
}

function structureRowTargetsField(structureRow, fieldRow) {
  return structureTargetRow(structureRow) === fieldRow
}

function stableFieldId(name, type) {
  const slug = String(name || '')
    .trim()
    .replace(/[^a-zA-Z0-9_]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .toLowerCase()
  return `fld_${type}_${slug || hashText(name)}`
}

function hashText(text) {
  let hash = 2166136261
  for (const char of String(text || 'field')) {
    hash ^= char.charCodeAt(0)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(16)
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

// ── Batch Fill ──────────────────────────────────────────────────────────────

async function handleBatchCommand(command) {
  if (command === 'export') {
    await exportBatchTemplate()
  } else if (command === 'import') {
    await importAndBatchRender()
  }
}

async function exportBatchTemplate() {
  if (!templatePath.value || !templateManifest.value) return
  const defaultName = `${stripExtension(fileName(templatePath.value), /\.docsytpl$/i)}-批量填写模板.xlsx`
  const outputPath = await save({
    defaultPath: `${parentDir(templatePath.value)}/${defaultName}`,
    filters: [{ name: 'Excel 文件', extensions: ['xlsx'] }],
  })
  if (!outputPath) return

  batchProcessing.value = true
  const result = await tauriCallSafe('export_template_fields_xlsx', {
    templatePath: templatePath.value,
    outputPath: ensureExtension(outputPath, 'xlsx'),
    defaultValues: normalizeValues(),
  })
  batchProcessing.value = false

  if (!result.ok) {
    ElMessage.error(result.error || '导出失败')
    return
  }
  ElMessage.success('字段表已导出')
  ElMessage.info('若模板有填写历史，第 3 行为最近一次填写示例（标"否"不会生成数据）；需要可复制一行后填写')
  const openResult = await openPath(result.data)
  if (!openResult.ok) {
    ElMessage.warning('字段表已导出但无法自动打开，请到保存目录查看')
  }
}

async function importAndBatchRender() {
  if (!templatePath.value || !templateManifest.value) return

  const selected = await open({
    multiple: false,
    filters: [{ name: 'Excel 文件', extensions: ['xlsx'] }],
  })
  if (!selected) return

  const xlsxPath = selected

  // Step 1: Validate
  batchProcessing.value = true
  const validation = await tauriCallSafe('validate_batch_import', {
    templatePath: templatePath.value,
    xlsxPath,
  })

  if (!validation.ok) {
    batchProcessing.value = false
    ElMessage.error(validation.error || '校验失败')
    return
  }

  const v = validation.data

  // Show validation result dialog
  const { proceed, skipRows } = await showValidationDialog(v)
  if (!proceed) {
    batchProcessing.value = false
    return
  }

  // Step 2: Choose output directory
  const outputDir = await open({
    directory: true,
    multiple: false,
    defaultPath: parentDir(templatePath.value),
  })
  if (!outputDir) {
    batchProcessing.value = false
    return
  }

  // Step 3: Batch render (pass current structureOverrides + skipRows)
  const result = await tauriCallSafe('batch_render_from_xlsx', {
    templatePath: templatePath.value,
    xlsxPath,
    outputDir,
    namePattern: '',
    skipRows,
    structureOverrides: normalizeStructureOverrides(),
    itemSeparator: itemSeparatorSetting.value || '、',
  })
  batchProcessing.value = false

  if (!result.ok) {
    ElMessage.error(result.error || '批量生成失败')
    return
  }

  const r = result.data
  const outputDirText = typeof outputDir === 'string' ? outputDir : ''
  const lines = [`批量填写完成：成功 ${r.success} 份` + (r.failed > 0 ? `，失败 ${r.failed} 份` : '')]
  if (outputDirText) lines.push(`输出目录：${outputDirText}`)
  if (r.rows?.length) lines.push(`本次生成了 ${r.rows.length} 行填写数据，可保存到模板填写历史`)

  let saveData = false
  try {
    await ElMessageBox.confirm(lines.join('\n'), '批量填写完成', {
      confirmButtonText: outputDirText ? '打开输出目录' : '完成',
      cancelButtonText: r.rows?.length ? '保存数据' : '',
      type: 'success',
      customStyle: { whiteSpace: 'pre-line' },
    })
    if (outputDirText) {
      const openResult = await openPath(outputDirText)
      if (!openResult.ok) {
        ElMessage.warning('无法打开输出目录，请手动查看')
      }
    }
  } catch {
    saveData = true
  }
  if (saveData && r.rows?.length) {
    openBatchSaveDialog(r.rows)
  }
}

const batchSaveVisible = ref(false)
const batchSaveRows = ref([])
const batchSaveSelected = ref([])

function openBatchSaveDialog(rows) {
  batchSaveRows.value = (rows || []).map((row, index) => ({
    key: `${index}`,
    outputPath: row.outputPath || '',
    values: row.values || {},
    selected: true,
  }))
  batchSaveSelected.value = []
  batchSaveVisible.value = true
}

function toggleBatchSaveAll(value) {
  if (value) {
    batchSaveSelected.value = batchSaveRows.value.map((row) => row.key)
  } else {
    batchSaveSelected.value = []
  }
}

function invertBatchSaveSelection() {
  const selected = new Set(batchSaveSelected.value)
  batchSaveSelected.value = batchSaveRows.value
    .map((row) => row.key)
    .filter((key) => !selected.has(key))
}

function toggleBatchSaveRow(key) {
  const idx = batchSaveSelected.value.indexOf(key)
  if (idx >= 0) {
    batchSaveSelected.value = batchSaveSelected.value.filter((k) => k !== key)
  } else {
    batchSaveSelected.value = [...batchSaveSelected.value, key]
  }
}

function batchSaveRowSummary(row) {
  const values = row.values || {}
  const parts = Object.values(values)
    .map((value) => {
      if (Array.isArray(value)) return value.map((item) => (item?.text ?? item?.name ?? '')).filter(Boolean).join('、')
      if (value && typeof value === 'object') return value.text ?? value.name ?? ''
      return String(value ?? '')
    })
    .filter(Boolean)
    .slice(0, 5)
  return parts.length ? parts.join(' | ') : '（空）'
}

async function submitBatchSave() {
  const rows = batchSaveRows.value.filter((row) => batchSaveSelected.value.includes(row.key))
  if (!rows.length) {
    ElMessage.warning('请至少选择一行')
    return
  }
  const payload = rows.map((row) => ({
    templatePath: templatePath.value,
    outputPath: row.outputPath,
    values: row.values,
  }))
  const result = await tauriCallSafe('save_batch_history_rows', { rows: payload })
  if (result.ok) {
    ElMessage.success(`已保存 ${result.data} 行填写记录到模板历史`)
    batchSaveVisible.value = false
    loadTemplateHistoryRuns()
  } else {
    ElMessage.error(result.error || '保存填写记录失败')
  }
}

async function showValidationDialog(validation) {
  const v = validation
  const lines = []

  if (!v.templateIdMatch) {
    lines.push('⚠️ Excel 文件的模板 ID 与当前模板不一致，将按字段名称匹配导入。')
    lines.push('')
  }

  lines.push(`共 ${v.totalRows} 行数据，${v.validRows} 行有效。`)

  if (v.warnings.length) {
    lines.push('')
    lines.push('警告：')
    for (const w of v.warnings) {
      lines.push(`  · ${w.message}`)
    }
  }

  // Collect error row indices for skip
  const errorRowSet = new Set(v.errors.map((e) => e.row))

  if (v.errors.length) {
    lines.push('')
    lines.push('错误：')
    const shown = v.errors.slice(0, 10)
    for (const e of shown) {
      lines.push(`  · 第 ${e.row + 1} 行：${e.message}`)
    }
    if (v.errors.length > 10) {
      lines.push(`  · ...还有 ${v.errors.length - 10} 个错误`)
    }
  }

  // Block if no valid rows
  if (v.validRows === 0 && v.totalRows > 0) {
    lines.push('')
    lines.push('❌ 没有有效数据行，无法生成。')
  }

  // Block if no valid rows
  if (v.validRows === 0) {
    try {
      await ElMessageBox.alert(lines.join('\n'), '无有效数据', {
        confirmButtonText: '知道了',
        type: 'warning',
        customStyle: { whiteSpace: 'pre-line' },
      })
    } catch {
      // User dismissed
    }
    return { proceed: false, skipRows: [] }
  }

  const hasErrors = v.errors.length > 0
  const title = hasErrors ? '校验结果（有错误）' : '校验结果'
  const type = hasErrors ? 'warning' : 'info'

  try {
    if (hasErrors) {
      await ElMessageBox.confirm(lines.join('\n'), title, {
        confirmButtonText: '跳过错误行继续',
        cancelButtonText: '取消',
        type,
        customStyle: { whiteSpace: 'pre-line' },
      })
    } else {
      await ElMessageBox.confirm(lines.join('\n'), title, {
        confirmButtonText: '开始生成',
        cancelButtonText: '取消',
        type,
        customStyle: { whiteSpace: 'pre-line' },
      })
    }
    return { proceed: true, skipRows: Array.from(errorRowSet) }
  } catch {
    return { proceed: false, skipRows: [] }
  }
}

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

.workspace {
  display: grid;
  gap: 14px;
  padding-top: 8px;
}

.panel {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  padding: 14px;
  background: var(--docsy-surface-elevated);
  box-shadow: 0 3px 14px rgba(54, 45, 36, 0.035);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  margin-bottom: 12px;
}

.panel-header.compact {
  align-items: center;
}

.panel-actions {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.help-button {
  color: var(--docsy-text);
}

h3 {
  margin: 0 0 4px;
  font-size: 16px;
  color: var(--docsy-text-strong);
}

p {
  margin: 0;
  color: var(--docsy-text-muted);
  font-size: 13px;
}

.actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.actions.inline {
  margin-top: 0;
}

.template-name {
  max-width: 260px;
}

.document-collapse {
  margin-top: 12px;
}

.document-preview {
  max-height: 260px;
  overflow: auto;
  margin: 0;
  padding: 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-strong);
  font-family: inherit;
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
}

.selection-tools {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-bottom: 10px;
  padding: 8px 10px;
  border: 1px solid #cddfd9;
  border-radius: 6px;
  background: var(--docsy-primary-soft);
}

.selection-count {
  color: var(--docsy-primary);
  font-size: 13px;
  font-weight: 600;
}

.suggestion-panel {
  display: grid;
  gap: 8px;
  margin-bottom: 10px;
  padding: 10px;
  border: 1px solid #d7e4d1;
  border-radius: 6px;
  background: #f2f7ef;
}

.suggestion-header,
.suggestion-item {
  display: flex;
  gap: 10px;
  align-items: center;
  justify-content: space-between;
}

.suggestion-header strong {
  color: var(--docsy-success);
  font-size: 13px;
}

.suggestion-list {
  display: grid;
  gap: 4px;
}

.suggestion-item {
  color: var(--docsy-text);
  font-size: 13px;
}

.group-input {
  max-width: 180px;
}

.type-input {
  width: 150px;
}

.type-help-item {
  display: grid;
  gap: 2px;
  color: var(--docsy-text);
  line-height: 1.45;
}

.type-help-item strong {
  color: var(--docsy-text-strong);
}

.field-rules {
  display: grid;
  gap: 12px;
  color: var(--docsy-text);
}

.field-rules h4 {
  margin: 0 0 6px;
  color: var(--docsy-text-strong);
  font-size: 13px;
}

.field-rules p {
  margin: 4px 0 0;
  color: var(--docsy-text);
  line-height: 1.55;
}

.rule-summary-collapse {
  margin-top: 10px;
}

.rule-summary-list {
  display: grid;
  gap: 8px;
}

.rule-summary-item {
  display: grid;
  grid-template-columns: minmax(120px, 180px) minmax(0, 1fr);
  gap: 10px;
  align-items: start;
  color: var(--docsy-text);
  font-size: 13px;
}

.rule-summary-item strong {
  color: var(--docsy-text-strong);
}

.template-build-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-top: 12px;
}

.preview-panel {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--docsy-border-subtle);
}

.preview-panel-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: baseline;
  margin-bottom: 8px;
}

.preview-panel-header h3,
.template-preview-grid h4 {
  margin: 0;
  color: var(--docsy-text-strong);
  font-size: 14px;
}

.template-preview-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 12px;
}

.template-preview-grid section {
  min-width: 0;
}

.template-preview-text {
  min-height: 160px;
  max-height: 360px;
  overflow: auto;
  padding: 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-strong);
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
}

.source-preview-text {
  user-select: text;
}

.source-run {
  white-space: pre-wrap;
}

.source-bold {
  font-weight: 700;
}

.source-italic {
  font-style: italic;
}

.source-underline {
  text-decoration: underline;
}

.preview-token {
  display: inline;
  margin: 0 1px;
  padding: 1px 3px;
  border: 0;
  border-radius: 3px;
  font: inherit;
  line-height: inherit;
  cursor: pointer;
}

.preview-text {
  background: var(--docsy-primary-soft);
  color: var(--docsy-primary-hover);
}

.preview-date {
  background: #f3e8ff;
  color: #6b2fa0;
}

.preview-select {
  background: #e6fffb;
  color: #0f766e;
}

.preview-party {
  background: #f0f9eb;
  color: #2f6f1f;
}

.preview-reference {
  background: #edf2ff;
  color: #364fc7;
  border-bottom: 1px dashed currentcolor;
}

.preview-checkbox {
  background: #fff7e6;
  color: #9a5b13;
}

.preview-radio {
  background: #fff1f0;
  color: #b42318;
}

.preview-checkbox-group {
  background: #eef2ff;
  color: #4338ca;
}

.preview-prefix {
  background: #fdf6ec;
  color: #9a5b13;
}

.preview-suffix {
  background: #f6f6f6;
  color: var(--docsy-text);
}

.preview-delete-text {
  background: #fef0f0;
  color: #c45656;
  text-decoration: line-through;
}

.preview-ignore {
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
}

.preview-deleted {
  color: var(--docsy-text-muted);
  text-decoration: line-through;
  opacity: 0.7;
}

.preview-focused {
  outline: 2px solid var(--docsy-primary);
}

.preview-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-top: 8px;
}

.preview-selection-status {
  max-width: 360px;
  padding: 3px 8px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: 4px;
  color: var(--docsy-text);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-action-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--docsy-border-subtle);
  color: var(--docsy-text);
  font-size: 13px;
}

.legend-token {
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.settings-badge {
  line-height: 1;
}

.reference-suggestion {
  display: grid;
  gap: 6px;
  margin-bottom: 12px;
  padding: 10px;
  border: 1px solid #fcd3d3;
  border-radius: 6px;
  background: #fef0f0;
  color: var(--docsy-text);
}

.reference-suggestion p {
  color: var(--docsy-text);
  line-height: 1.5;
}

.reference-actions {
  margin-top: 2px;
}

.dialog-tip {
  margin-bottom: 10px;
}

.mark-cell {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  max-width: 100%;
}

.mark-text {
  display: -webkit-box;
  min-width: 0;
  overflow: hidden;
  line-height: 1.35;
  word-break: break-word;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.structure-mark {
  color: var(--docsy-text-muted);
}

.relation-arrow {
  color: var(--docsy-text-muted);
  flex: 0 0 auto;
}

.party-item-arrow {
  color: #67c23a;
  flex: 0 0 auto;
  font-weight: 700;
}

.field-cell {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.field-label {
  color: var(--docsy-text-muted);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.stack-input {
  margin-top: 4px;
}

.symbol-pair {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}

.muted {
  color: #c0c4cc;
}

:deep(.ignored-field-row) {
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-muted);
  opacity: 0.72;
}

:deep(.ignored-field-row .el-input__wrapper),
:deep(.ignored-field-row .el-select__wrapper) {
  background: var(--docsy-surface-muted);
}

:deep(.delete-field-row) {
  color: #c45656;
  background: #fef0f0;
}

:deep(.party-child-row) {
  color: var(--docsy-text);
}

:deep(.party-group-row) {
  font-weight: 600;
}

:deep(.party-child-row .el-table-column--selection .cell) {
  visibility: hidden;
}

:deep(.party-group-row .el-table-column--selection .cell) {
  visibility: hidden;
}

.party-child-field > span:first-child {
  color: var(--docsy-primary);
  font-weight: 600;
}

:deep(.grouped-field-row .mark-cell) {
  padding-left: 14px;
}

:deep(.party-child-row.grouped-field-row .mark-cell) {
  padding-left: 28px;
}

:deep(.grouped-field-row td:first-child) {
  border-left-width: 4px;
  border-left-style: solid;
}

:deep(.grouped-field-row-0 td) {
  background: #f2f8ff;
}

:deep(.grouped-field-row-0 td:first-child) {
  border-left-color: var(--docsy-primary);
}

:deep(.grouped-field-row-1 td) {
  background: #f1fbf3;
}

:deep(.grouped-field-row-1 td:first-child) {
  border-left-color: #67c23a;
}

:deep(.grouped-field-row-2 td) {
  background: #f9f4ff;
}

:deep(.grouped-field-row-2 td:first-child) {
  border-left-color: #8e5cf7;
}

:deep(.grouped-field-row-3 td) {
  background: #fff8ed;
}

:deep(.grouped-field-row-3 td:first-child) {
  border-left-color: #e6a23c;
}

:deep(.grouped-field-row-4 td) {
  background: #fff2f2;
}

:deep(.grouped-field-row-4 td:first-child) {
  border-left-color: #f56c6c;
}

:deep(.grouped-field-row-5 td) {
  background: #f0fbff;
}

:deep(.grouped-field-row-5 td:first-child) {
  border-left-color: #14b8c5;
}

.template-library-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  gap: 10px;
}

.template-library-card {
  display: grid;
  gap: 5px;
  padding: 12px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text-strong);
  text-align: left;
  cursor: pointer;
}

.template-library-card:hover,
.template-library-card.active {
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}

.template-library-card span,
.template-library-card small {
  color: var(--docsy-text-muted);
}

.history-group-list,
.history-run-list {
  display: grid;
  gap: 12px;
}

.history-group {
  display: grid;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
}

.history-group-header,
.history-run-card,
.history-run-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.history-group-header {
  justify-content: space-between;
}

.history-group-header h4 {
  margin: 0 0 2px;
  color: var(--docsy-text-strong);
}

.history-group-header span {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.history-run-card {
  justify-content: space-between;
  padding: 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
  cursor: pointer;
}

.history-run-card:hover {
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}

.history-run-main {
  display: grid;
  gap: 5px;
  min-width: 0;
}

.history-run-main > span {
  overflow: hidden;
  color: var(--docsy-text-muted);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.history-run-fields {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.history-run-actions {
  flex: 0 0 auto;
}

.template-form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 12px;
}

.fill-field-card {
  position: relative;
  display: grid;
  gap: 8px;
  min-width: 0;
  padding: 12px 34px 12px 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
}

/* Keep the card's grid layout when wrapping the controls for collapse:
   display:contents makes the wrapper transparent to layout, while v-show
   still hides/shows it via inline display:none. */
.fill-field-body {
  display: contents;
}

.field-more-button {
  position: absolute;
  right: 8px;
  bottom: 8px;
  width: 22px;
  height: 22px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: 50%;
  background: var(--docsy-surface);
  color: var(--docsy-text);
  cursor: pointer;
  line-height: 18px;
}

.field-more-button:hover {
  border-color: var(--docsy-primary);
  color: var(--docsy-primary);
}

.fill-structure-editor {
  display: grid;
  gap: 8px;
}

.fill-field-header {
  display: flex;
  gap: 8px;
  align-items: center;
  min-width: 0;
}

.fill-field-header strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fill-field-header span,
.fill-field-header em {
  flex: 0 0 auto;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  font-size: 12px;
  font-style: normal;
}

.fill-field-header em {
  background: #fef0f0;
  color: #f56c6c;
}

.fill-structure-hints {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.fill-structure-hint {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.fill-structure-hint code {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
  font-family: inherit;
}

.fill-structure-hint code.empty {
  color: #c0c4cc;
}

.fill-structure-hint em {
  margin-left: 4px;
  color: #c0c4cc;
  font-style: normal;
}

.party-list-editor {
  display: grid;
  gap: 8px;
  width: 100%;
}

.reference-fill-editor {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.party-list-row {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) minmax(90px, 130px) minmax(106px, auto);
  gap: 6px;
  align-items: center;
}

.party-list-row.compact {
  grid-template-columns: 24px minmax(0, 1fr) minmax(72px, 90px) minmax(96px, auto);
}

.party-list-row.compact.no-suffix {
  grid-template-columns: 24px minmax(0, 1fr) minmax(96px, auto);
}

.party-list-row.compact .el-button {
  padding-left: 4px;
  padding-right: 4px;
}

.party-row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 4px;
  justify-content: flex-end;
  min-width: 0;
}

.party-row-actions .el-button + .el-button {
  margin-left: 0;
}

.party-list-add-row {
  display: flex;
  justify-content: flex-start;
}

.select-options-editor {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}
.select-option-row {
  display: flex;
  gap: 6px;
  align-items: center;
}
.select-option-input {
  flex: 1;
}

.field-structure-hint {
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.party-order {
  color: var(--docsy-text-muted);
  text-align: center;
}

.party-suffix {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 0 8px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
  font-size: 12px;
}

.suggestion-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}

.suggestion-tag {
  cursor: pointer;
}

.field-panel,
.form-panel {
  min-height: 0;
}

@media (max-width: 1180px) {
  .template-preview-grid,
  .template-form-grid {
    grid-template-columns: 1fr;
  }

  .panel-header,
  .panel-header.compact {
    align-items: flex-start;
    flex-direction: column;
  }

  .panel-actions {
    display: flex;
    flex-wrap: wrap;
    align-self: stretch;
  }

  .template-name {
    flex: 1 1 220px;
    max-width: none;
  }
}

@media (max-width: 760px) {
  .template-view {
    padding: 16px;
  }

  .party-list-row,
  .party-list-row.compact,
  .party-list-row.compact.no-suffix {
    grid-template-columns: 24px minmax(0, 1fr);
  }

  .party-list-row > :not(.party-order):not(.el-autocomplete):not(.el-input) {
    grid-column: 2;
  }

  .party-row-actions {
    justify-content: flex-start;
  }
}
</style>
