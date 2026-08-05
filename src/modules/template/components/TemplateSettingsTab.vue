<template>
  <section class="workspace">
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>模板模块设置</h3>
          <p>这些规则会应用到所有模板的填写与生成。</p>
        </div>
      </div>
      <div class="settings-form">
        <div class="settings-row">
          <div class="settings-label">
            <strong>多项字段连接符</strong>
            <span>列表字段（当事人、诉讼请求等）填入多个值时，项与项之间使用的分隔符。留空时默认使用顿号"、"。</span>
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

    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>模板回收站</h3>
          <p>删除的模板先进入回收站；彻底删除会同时删除该模板的内部填写数据。</p>
        </div>
        <el-button size="small" :loading="templateTrashLoading" @click="$emit('refresh-trash')">刷新</el-button>
      </div>
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

    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>填写历史数据库</h3>
          <p>模板填写历史保存在本地数据库中（含字段值和引用建议来源）。清理后无法恢复。</p>
        </div>
      </div>
      <div class="settings-form">
        <div class="settings-row">
          <div class="settings-label">
            <strong>清空全部填写历史</strong>
            <span>删除所有模板的生成记录和字段值。模板库文件不受影响。</span>
          </div>
          <el-button type="danger" plain size="small" :loading="clearingHistory" @click="$emit('clear-all-history')">
            清空历史
          </el-button>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup>
import { shortDateTime } from '../composables/fieldRowUtils.js'

defineProps({
  itemSeparatorSetting: { type: String, default: '、' },
  templateTrash: { type: Array, default: () => [] },
  templateTrashLoading: { type: Boolean, default: false },
  clearingHistory: { type: Boolean, default: false },
})

defineEmits([
  'update:itemSeparatorSetting',
  'save-separator',
  'restore-template',
  'permanently-delete-template',
  'clear-all-history',
  'refresh-trash',
])
</script>
