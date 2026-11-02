<template>
  <div class="merge-file-name-field">
    <div class="merge-file-name-controls">
      <el-input
        :model-value="modelValue"
        :disabled="disabled"
        aria-label="合并文件名"
        :aria-invalid="Boolean(error)"
        :class="{ 'invalid-name': error }"
        :placeholder="fallback"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <el-dropdown trigger="click" :disabled="disabled || !choices.length" @command="choose">
        <el-button :disabled="disabled || !choices.length" aria-label="选用输入文件名">选用文件名</el-button>
        <template #dropdown
          ><el-dropdown-menu>
            <el-dropdown-item v-for="path in choices" :key="path" :command="path" :title="path">{{
              fileName(path)
            }}</el-dropdown-item>
          </el-dropdown-menu></template
        >
      </el-dropdown>
      <HelpTooltip
        label="合并文件名"
        content="可自定义或选用一个输入文件名；自动补齐 .pdf，同名文件自动加序号，不覆盖原文件。"
      />
    </div>
    <span v-if="error" class="name-error" role="alert">{{ error }}</span>
  </div>
</template>
<script setup>
import { computed } from 'vue'
import { fileName } from '../../../core/filePath.js'
import { pdfNameFromSource, pdfOutputFileName } from '../../../core/outputFiles.js'
import HelpTooltip from '../../components/HelpTooltip.vue'
const props = defineProps({
  modelValue: { type: String, default: '' },
  files: { type: Array, default: () => [] },
  disabled: Boolean,
  fallback: { type: String, default: '合并文件.pdf' },
})
const emit = defineEmits(['update:modelValue'])
const choices = computed(() => [
  ...new Set(
    props.files
      .flatMap((file) =>
        typeof file === 'string'
          ? [file]
          : file.sourcePaths?.length
            ? file.sourcePaths
            : [file.sourcePath || file.path],
      )
      .filter(Boolean),
  ),
])
const error = computed(() => {
  try {
    pdfOutputFileName(props.modelValue, props.fallback)
    return ''
  } catch (error) {
    return error.message
  }
})
function choose(path) {
  if (choices.value.includes(path)) emit('update:modelValue', pdfNameFromSource(path))
}
</script>
<style scoped>
.merge-file-name-field {
  min-width: 0;
  width: 100%;
}
.merge-file-name-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.merge-file-name-controls .el-input {
  min-width: 100px;
  flex: 1;
}
.invalid-name :deep(.el-input__wrapper) {
  box-shadow: 0 0 0 1px var(--el-color-danger) inset;
}
.name-error {
  display: block;
  color: var(--el-color-danger);
  font-size: 12px;
  margin-top: 4px;
}
</style>
