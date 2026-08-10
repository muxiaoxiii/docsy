<template>
  <span class="history-btns" :class="{ compact }">
    <el-tooltip content="撤销 (Ctrl+Z)" :show-after="500" placement="top">
      <el-button
        link
        :size="compact ? 'small' : 'small'"
        :disabled="!canUndo"
        @click.stop="$emit('undo')"
      >
        <el-icon :size="iconSize"><RefreshLeft /></el-icon>
      </el-button>
    </el-tooltip>
    <el-tooltip content="重做 (Ctrl+Shift+Z)" :show-after="500" placement="top">
      <el-button
        link
        :size="compact ? 'small' : 'small'"
        :disabled="!canRedo"
        @click.stop="$emit('redo')"
      >
        <el-icon :size="iconSize"><RefreshRight /></el-icon>
      </el-button>
    </el-tooltip>
  </span>
</template>

<script setup>
import { RefreshLeft, RefreshRight } from '@element-plus/icons-vue'
import { computed } from 'vue'

const props = defineProps({
  canUndo: { type: Boolean, default: false },
  canRedo: { type: Boolean, default: false },
  compact: { type: Boolean, default: false },
})

defineEmits(['undo', 'redo'])

const iconSize = computed(() => props.compact ? 12 : 14)
</script>

<style scoped>
.history-btns {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.history-btns.compact {
  margin-left: 4px;
}
</style>
