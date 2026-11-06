<template>
  <div v-if="batch" class="incoming-files-notice" role="status">
    <div>
      <strong :title="batch.paths.join('\n')">{{ batch.paths.length }} 个文件 → {{ targetLabel }}{{ batch.source ? ` · 来自${batch.source}` : '' }}</strong>
      <p>{{ receiving ? '正在导入…' : error || '等待目标工具接收，已有列表将保留。' }}</p>
    </div>
    <div class="incoming-actions">
      <el-button size="small" :disabled="disabled || receiving" @click="$emit('retry')">继续导入</el-button
      ><el-button size="small" text :disabled="receiving" @click="$emit('discard')">取消导入</el-button>
    </div>
  </div>
</template>
<script setup>
import { computed } from 'vue'
import { FILE_TRANSFER_TARGETS } from '../../core/fileTransfers.js'
const props = defineProps({
  batch: { type: Object, default: null },
  error: { type: String, default: '' },
  receiving: Boolean,
  disabled: Boolean,
})
const targetLabel = computed(() => FILE_TRANSFER_TARGETS.find(item => item.id === props.batch?.target)?.label || '目标工具')
defineEmits(['retry', 'discard'])
</script>
<style scoped>
.incoming-files-notice {
  flex-shrink: 0;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  padding: 10px 14px;
  background: var(--docsy-surface-active);
  font-size: 13px;
}
.incoming-files-notice p {
  margin: 3px 0 0;
  color: var(--docsy-text-muted);
}
.incoming-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.incoming-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
</style>
