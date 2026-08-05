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
              <el-tooltip content='列表字段（当事人、诉讼请求等）填入多个值时，项与项之间使用的分隔符。留空时默认使用顿号"、"。' placement="top">
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
          <el-table-column label="操作" width="120" fixed="right">
            <template #default="{ row }">
              <el-button size="small" link type="danger" @click="$emit('delete-template-database-entry', row)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-empty v-else description="暂无模板数据" />
      </div>
    </div>
  </section>
</template>

<script setup>
import { InfoFilled } from '@element-plus/icons-vue'
import { shortDateTime } from '../composables/fieldRowUtils.js'

defineProps({
  itemSeparatorSetting: { type: String, default: '、' },
  templateTrash: { type: Array, default: () => [] },
  templateTrashLoading: { type: Boolean, default: false },
  templateDatabase: { type: Array, default: () => [] },
  templateDatabaseLoading: { type: Boolean, default: false },
  clearingHistory: { type: Boolean, default: false },
})

defineEmits([
  'update:itemSeparatorSetting',
  'save-separator',
  'restore-template',
  'permanently-delete-template',
  'clear-all-history',
  'refresh-trash',
  'refresh-template-database',
  'delete-template-database-entry',
])
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
</style>
