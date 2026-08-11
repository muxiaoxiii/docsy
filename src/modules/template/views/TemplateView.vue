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
          :editing-library-template-path="editingLibraryTemplatePath"
          v-model:filename-tokens="filenameTokens"
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
          :renderable-template-fields="renderableTemplateFields"
          :fill-position-entries="filteredFillPositionEntries"
          :filtered-renderable-fields="filteredRenderableFields"
          :fill-preview-visible="fillPreviewVisible"
          :fill-preview-text="fillPreviewText"
          :fill-preview-overlays="fillPreviewOverlays"
          :fill-document-runs="fillDocumentRuns"
          :filename-tokens="filenameTokens"
          @load-template-library="loadTemplateLibrary"
          @select-template-package="selectTemplatePackage"
          @open-template-from-library="openTemplateFromLibrary"
          @edit-template="editTemplateFromLibrary"
          @delete-template="deleteTemplate"
          @render-template="renderTemplate"
          @batch-command="handleBatchCommand"
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
          @save-field-reference="onSaveFieldReference"
          @save-field-date-format="onSaveFieldDateFormat"
          @toggle-fill-preview="toggleFillPreview"
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
          v-model:export-dialog-visible="exportDialogVisible"
          :export-template-list="exportTemplateList"
          v-model:export-selected-paths="exportSelectedPaths"
          :export-result="exportResult"
          @save-separator="saveItemSeparatorSetting"
          @restore-template="restoreTemplate"
          @permanently-delete-template="permanentlyDeleteTemplate"
          @clear-all-history="clearAllHistory"
          @refresh-trash="loadTemplateTrash"
          @refresh-template-database="loadTemplateDatabase"
          @delete-template-database-entry="deleteTemplateDatabaseEntry"
          @import-template="importTemplateToLibrary"
          @open-export-dialog="openExportDialog"
          @execute-export="executeExportTemplates"
          @open-export-folder="openExportFolder"
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
      <el-table
        :data="batchSaveRows"
        size="small"
        border
        max-height="52vh"
        row-key="key"
        @row-click="(row) => toggleBatchSaveRow(row.key)"
      >
        <el-table-column width="44">
          <template #default="{ row }">
            <el-checkbox
              :model-value="batchSaveSelected.includes(row.key)"
              @click.stop
              @change="() => toggleBatchSaveRow(row.key)"
            />
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
import TemplateBuildTab from '../components/TemplateBuildTab.vue'
import TemplateRenderTab from '../components/TemplateRenderTab.vue'
import TemplateHistoryTab from '../components/TemplateHistoryTab.vue'
import TemplateSettingsTab from '../components/TemplateSettingsTab.vue'
import { useTemplateState } from '../composables/useTemplateState.js'

const {
  // Core state
  activeTab,
  buildTabRef,

  // Build tab state
  sourceDocx,
  scanning,
  marks,
  documentText,
  templateName,
  fieldRows,
  selectedRows,
  saving,
  showDocumentText,
  showTemplatePreview,
  groupName,
  groupLabel,
  groupType,
  undoStack,
  previewSampleValues,
  sourcePreviewSelectionPayload,
  previewFocusedRowId,
  templatePreview,
  editingLibraryTemplatePath,
  filenameTokens,
  splitDialog,

  // Fill/Render tab state
  templatePath,
  templateManifest,
  templateLibrary,
  templateLibraryLoading,
  formValues,
  referenceSelections,
  structureOverrides,
  typeOverrides,
  historyContext,
  rendering,
  fieldSearch,
  renderableTemplateFields,
  filteredFillPositionEntries,
  filteredRenderableFields,
  fillPreviewVisible,
  fillPreviewText,
  fillPreviewOverlays,
  fillDocumentRuns,

  // History tab state
  historyRunsLoading,
  groupedHistoryRuns,
  expandedHistoryGroups,

  // Settings state
  itemSeparatorSetting,
  templateTrash,
  templateTrashLoading,
  clearingHistory,
  templateDatabase,
  templateDatabaseLoading,
  exportDialogVisible,
  exportTemplateList,
  exportSelectedPaths,
  exportResult,

  // Batch state
  batchProcessing,
  batchSaveVisible,
  batchSaveRows,
  batchSaveSelected,

  // Build tab events
  selectSourceDocx,
  groupSelectedRows,
  setSelectedRowsUsage,
  clearSelectedRows,
  handleSelectionChange,
  handleFieldTableWheel,
  openSplitDialog,
  onRowTypeChange,
  onFieldNameInput,
  onStructureTargetNameChange,
  applyReferenceSuggestion,
  applyAllReferenceSuggestions,
  syncMarkerSymbols,
  applyMarkerGroupMembers,
  addSelectOption,
  undoLastAction,
  saveTemplate,
  rememberSourcePreviewSelection,
  focusPreviewRow,
  triggerPreviewSelectionAdd,
  setPreviewSampleValue,

  // Fill/Render tab events
  loadTemplateLibrary,
  selectTemplatePackage,
  openTemplateFromLibrary,
  editTemplateFromLibrary,
  deleteTemplate,
  renderTemplate,
  handleBatchCommand,
  scheduleHistoryRefresh,
  completeField,
  movePartyItem,
  removePartyItem,
  addPartyItem,
  onReferenceSelectionChange,
  applySuggestion,
  setFieldTypeOverride,
  handleUpdateFormValue,
  handleUpdateStructureOverride,
  onSaveFieldReference,
  onSaveFieldDateFormat,
  toggleFillPreview,

  // History tab events
  loadTemplateHistoryRuns,
  applyHistoryRun,
  openHistoryTemplate,
  expandHistoryGroup,
  collapseHistoryGroup,

  // Settings tab events
  saveItemSeparatorSetting,
  restoreTemplate,
  permanentlyDeleteTemplate,
  clearAllHistory,
  loadTemplateTrash,
  loadTemplateDatabase,
  deleteTemplateDatabaseEntry,
  importTemplateToLibrary,
  openExportDialog,
  executeExportTemplates,
  openExportFolder,

  // Dialog events
  applySplitDialog,
  toggleBatchSaveAll,
  invertBatchSaveSelection,
  toggleBatchSaveRow,
  batchSaveRowSummary,
  submitBatchSave,

  // Re-exports
  openPath,
  syncReferenceSourceFromKey,
} = useTemplateState()
</script>

<style scoped>
.template-view,
.template-tabs {
  min-height: 100%;
}

.template-view {
  padding: 20px 24px 36px;
  background: var(--docsy-canvas);
}

:deep(.template-tabs > .el-tabs__header) {
  position: sticky;
  top: 0;
  z-index: 8;
  margin: 0 0 18px;
  padding: 6px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: rgba(255, 253, 248, 0.94);
  box-shadow: var(--docsy-shadow-panel);
  backdrop-filter: blur(14px);
}

:deep(.template-tabs > .el-tabs__header .el-tabs__nav-wrap::after),
:deep(.template-tabs > .el-tabs__header .el-tabs__active-bar) {
  display: none;
}

:deep(.template-tabs > .el-tabs__header .el-tabs__item) {
  height: 38px;
  padding: 0 18px;
  border-radius: var(--docsy-radius);
  font-size: 13px;
}

:deep(.template-tabs > .el-tabs__header .el-tabs__item.is-active) {
  color: var(--docsy-primary-hover);
  background: var(--docsy-primary-soft);
  font-weight: 650;
}

:deep(.template-tabs > .el-tabs__content),
:deep(.template-tabs > .el-tabs__content > .el-tab-pane) {
  min-width: 0;
  overflow: visible;
}

.template-view :deep(.workspace) {
  gap: 16px;
}

.template-view :deep(.panel) {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: var(--docsy-shadow-panel);
}

.template-view :deep(.panel-header) {
  padding-bottom: 14px;
  border-bottom-color: var(--docsy-border-subtle);
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
