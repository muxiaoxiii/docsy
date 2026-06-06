<template>
  <div v-if="marks.length" class="marks-aside">
    <div class="marks-aside-title">已标记字段（{{ marks.length }}）</div>
    <div
      v-for="(m, i) in marks"
      :key="i"
      class="mark-item"
      :class="{ 'mark-item-active': editingIndex === i }"
      @click="$emit('edit-mark', i)"
    >
      <div class="mark-item-text">{{ m.text }}</div>
      <div class="mark-item-meta">
        <el-tag size="small" type="warning" effect="plain">
          {{ m.label || m.key }}
        </el-tag>
        <el-tag v-if="m.row_repeat" size="small" type="success" effect="plain">
          行重复
        </el-tag>
        <el-tag v-if="m.auto_number" size="small" type="info" effect="plain">
          自动编号
        </el-tag>
        <el-button
          size="small"
          link
          type="danger"
          @click.stop="$emit('remove-mark', i)"
        >
          取消标记
        </el-button>
      </div>
    </div>
  </div>
</template>

<script setup>
defineProps({
  marks: { type: Array, default: () => [] },
  editingIndex: { type: Number, default: -1 },
});

defineEmits(["edit-mark", "remove-mark"]);
</script>

<style scoped>
.marks-aside {
  width: 240px;
  flex-shrink: 0;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  padding: 12px;
  height: fit-content;
  position: sticky;
  top: 0;
}
.marks-aside-title {
  font-size: 13px;
  font-weight: 600;
  color: #374151;
  margin-bottom: 8px;
}
.mark-item {
  background: #ffffff;
  border: 1px solid #e5e7eb;
  border-radius: 4px;
  padding: 6px 8px;
  margin-bottom: 6px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.mark-item:hover {
  border-color: #93c5fd;
}
.mark-item-active {
  border-color: #3b82f6;
  box-shadow: 0 0 0 1px #3b82f6;
}
.mark-item-text {
  font-size: 12px;
  color: #6b7280;
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.mark-item-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}
</style>
