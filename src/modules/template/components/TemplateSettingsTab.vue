<template>
  <section class="workspace">
    <!-- 模块设置 -->
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>模块设置</h3>
          <p>这些规则会应用到所有模板的填写与生成。</p>
        </div>
      </div>
      <div class="settings-form">
        <div class="settings-row-vertical">
          <div class="settings-label-vertical">
            <strong>
              多项字段连接符
              <el-tooltip content="列表字段（当事人、诉讼请求等）填入多个值时，项与项之间使用的分隔符。留空时默认使用顿号&#34;、&#34;。" placement="top">
                <el-icon class="info-icon"><InfoFilled /></el-icon>
              </el-tooltip>
            </strong>
          </div>
          <el-input
            :model-value="itemSeparatorSetting"
            size="small"
            class="settings-separator-input"
            placeholder="、"
            @update:model-value="$emit('update:itemSeparatorSetting', $event)"
            @change="$emit('save-separator')"
          />
        </div>
      </div>
    </div>

    <!-- 模板管理 -->
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>模板管理</h3>
          <p>导入或导出 .docsytpl 模板文件。</p>
        </div>
      </div>
      <div class="template-management-actions">
        <el-button @click="$emit('import-template')">导入模板</el-button>
        <el-button @click="$emit('open-export-dialog')">导出模板</el-button>
      </div>
    </div>

    <!-- 模板回收站 -->
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>模板回收站</h3>
          <p>删除的模板先进入回收站；彻底删除会同时删除该模板的内部填写数据。</p>
        </div>
        <el-button size="small" :loading="templateTrashLoading" @click="$emit('refresh-trash')">刷新</el-button>
      </div>
      <div class="trash-list-container">
        <el-table v-if="templateTrash.length" :data="templateTrash" size="small" border>
          <el-table-column prop="name" label="模板" min-width="180" />
          <el-table-column label="字段" width="80">
            <template #default="{ row }">{{ row.fieldCount }}</template>
          </el-table-column>
          <el-table-column prop="updated" label="更新时间" min-width="140">
            <template #default="{ row }">{{ shortDateTime(row.updated) }}</template>
          </el-table-column>
          <el-table-column label="操作" width="180" fixed="right">
            <template #default="{ row }">
              <el-button size="small" link type="primary" @click="$emit('restore-template', row)">恢复</el-button>
              <el-button size="small" link type="danger" @click="$emit('permanently-delete-template', row)"
                >彻底删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-empty v-else description="回收站为空" />
      </div>
    </div>

    <!-- 模板数据库 -->
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>模板数据库</h3>
          <p>模板填写历史保存在本地数据库中（含字段值和引用建议来源）。清理后无法恢复。</p>
        </div>
        <el-button size="small" :loading="templateDatabaseLoading" @click="$emit('refresh-template-database')">刷新</el-button>
      </div>
      <div class="trash-list-container">
        <el-table v-if="templateDatabase.length" :data="templateDatabase" size="small" border>
          <el-table-column prop="name" label="模板" min-width="180" />
          <el-table-column label="字段" width="80">
            <template #default="{ row }">{{ row.fieldCount }}</template>
          </el-table-column>
          <el-table-column prop="updatedAt" label="更新时间" min-width="140">
            <template #default="{ row }">{{ shortDateTime(row.updatedAt) }}</template>
          </el-table-column>
          <el-table-column label="操作" width="200" fixed="right">
            <template #default="{ row }">
              <el-button size="small" link type="primary" :disabled="templateDatabase.length < 2" @click="openMergeDialog(row)"
                >合并到…</el-button
              >
              <el-button size="small" link type="danger" @click="$emit('delete-template-database-entry', row)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-empty v-else description="暂无模板数据" />
      </div>
    </div>

    <!-- 模板数据合并对话框 -->
    <el-dialog v-model="mergeDialog.visible" title="合并模板字段数据" width="480px">
      <p class="dialog-tip">
        把源模板中与目标模板字段名相同的历史数据复制到目标模板（目标已存在的相同值会跳过）。
        源模板独有的字段数据保留在源模板；填表时字段名相同的历史本来就可以跨模板检索到。
      </p>
      <el-form label-width="80px" size="small">
        <el-form-item label="源模板">
          <el-input :model-value="mergeDialog.source?.name || ''" disabled />
        </el-form-item>
        <el-form-item label="目标模板">
          <el-select v-model="mergeDialog.targetId" filterable placeholder="选择目标模板" style="width: 100%">
            <el-option
              v-for="item in mergeTargets"
              :key="item.templateId"
              :label="`${item.name}（${item.fieldCount} 个字段）`"
              :value="item.templateId"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="mergeDialog.visible = false">取消</el-button>
        <el-button type="primary" :disabled="!mergeDialog.targetId" @click="executeMerge">合并</el-button>
      </template>
    </el-dialog>

    <!-- 导出模板对话框 -->
    <el-dialog
      :model-value="exportDialogVisible"
      title="导出模板"
      width="520px"
      @update:model-value="$emit('update:exportDialogVisible', $event)"
    >
      <el-table
        :data="exportTemplateList"
        size="small"
        border
        @selection-change="handleExportSelectionChange"
      >
        <el-table-column type="selection" width="42" />
        <el-table-column prop="name" label="模板名称" min-width="200" />
        <el-table-column prop="fieldCount" label="字段数" width="80" />
      </el-table>
      <div v-if="exportResult" class="export-result">
        <el-alert :title="exportResult" type="success" show-icon :closable="false" />
        <el-button size="small" type="primary" @click="$emit('open-export-folder')">打开文件夹</el-button>
      </div>
      <template #footer>
        <el-button @click="$emit('update:exportDialogVisible', false)">取消</el-button>
        <el-button
          type="primary"
          :disabled="!exportSelectedPaths.length"
          @click="$emit('execute-export')"
        >
          选择目录并导出
        </el-button>
      </template>
    </el-dialog>
  </section>
</template>

<script setup>
import { InfoFilled } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { shortDateTime } from '../composables/fieldRowUtils.js'
import { computed, ref } from 'vue'

const props = defineProps({
  itemSeparatorSetting: { type: String, default: '、' },
  templateTrash: { type: Array, default: () => [] },
  templateTrashLoading: { type: Boolean, default: false },
  templateDatabase: { type: Array, default: () => [] },
  templateDatabaseLoading: { type: Boolean, default: false },
  clearingHistory: { type: Boolean, default: false },
  // Export dialog
  exportDialogVisible: { type: Boolean, default: false },
  exportTemplateList: { type: Array, default: () => [] },
  exportSelectedPaths: { type: Array, default: () => [] },
  exportResult: { type: String, default: '' },
})

const emit = defineEmits([
  'update:itemSeparatorSetting',
  'save-separator',
  'restore-template',
  'permanently-delete-template',
  'clear-all-history',
  'refresh-trash',
  'refresh-template-database',
  'delete-template-database-entry',
  // Import/Export
  'import-template',
  'open-export-dialog',
  'execute-export',
  'open-export-folder',
  'update:exportDialogVisible',
  'update:exportSelectedPaths',
])

function handleExportSelectionChange(selection) {
  emit('update:exportSelectedPaths', selection.map((item) => item.path))
}

// ── Field-history merge ──────────────────────────────────────────────────────

const mergeDialog = ref({ visible: false, source: null, targetId: '' })
const mergeTargets = computed(() =>
  props.templateDatabase.filter((item) => item.templateId !== mergeDialog.value.source?.templateId),
)

function openMergeDialog(row) {
  mergeDialog.value = { visible: true, source: row, targetId: '' }
}

async function executeMerge() {
  const { source, targetId } = mergeDialog.value
  if (!source || !targetId) return
  try {
    await ElMessageBox.confirm(
      `把"${source.name}"的同名字段历史数据合并到目标模板？源模板数据不会被删除。`,
      '确认合并',
      { confirmButtonText: '合并', cancelButtonText: '取消', type: 'warning' },
    )
  } catch {
    return
  }
  const result = await tauriCallSafe('merge_template_field_history', {
    sourceTemplateId: source.templateId,
    targetTemplateId: targetId,
  })
  if (!result.ok) {
    ElMessage.error(result.error || '合并失败')
    return
  }
  ElMessage.success(`已合并 ${result.data ?? 0} 条同名字段数据`)
  mergeDialog.value.visible = false
  emit('refresh-template-database')
}
</script>

<style scoped>
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

.settings-form {
  display: grid;
  gap: 14px;
}

.settings-row-vertical {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.settings-label-vertical {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.settings-label-vertical strong {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 14px;
  color: var(--docsy-text-strong);
}

.info-icon {
  font-size: 14px;
  color: var(--docsy-text-muted);
  cursor: pointer;
}

.info-icon:hover {
  color: var(--docsy-primary);
}

.settings-separator-input {
  max-width: 200px;
}

.trash-list-container {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  overflow: hidden;
}

.template-management-actions {
  display: flex;
  gap: 8px;
}

.export-result {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 12px;
}
</style>
