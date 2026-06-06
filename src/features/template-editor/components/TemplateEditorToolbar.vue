<template>
  <div class="builder-toolbar">
    <span class="filename">{{ filename }}</span>
    <el-input
      :model-value="manifestName"
      @update:model-value="$emit('update:manifestName', $event)"
      placeholder="模板名称"
      size="small"
      style="width: 240px"
    />
    <el-segmented
      :model-value="previewMode"
      :options="previewModeOptions"
      size="small"
      @update:model-value="$emit('switch-preview-mode', $event)"
    />
    <el-button size="small" @click="$emit('insert-field')">插入字段</el-button>
    <el-button size="small" @click="$emit('edit-fixed-text')">编辑固有字</el-button>
    <el-button @click="$emit('reset')">重新选择</el-button>
    <el-button type="primary" :disabled="!markCount" @click="$emit('save')">
      保存模板（{{ markCount }} 个字段）
    </el-button>
  </div>
</template>

<script setup>
defineProps({
  filename: { type: String, default: "" },
  manifestName: { type: String, default: "" },
  previewMode: { type: String, default: "marked" },
  markCount: { type: Number, default: 0 },
});

defineEmits([
  "update:manifestName",
  "switch-preview-mode",
  "insert-field",
  "edit-fixed-text",
  "reset",
  "save",
]);

const previewModeOptions = [
  { label: "原文标记", value: "marked" },
  { label: "标签预览", value: "labels" },
  { label: "正文编辑", value: "edit" },
];
</script>

<style scoped>
.builder-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid #e5e7eb;
}
.filename {
  color: #6b7280;
  font-size: 13px;
  margin-right: 8px;
}
</style>
