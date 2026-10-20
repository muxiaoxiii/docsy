<template>
  <el-tooltip
    :content="content"
    :trigger="['hover', 'focus', 'click']"
    :show-after="180"
    placement="top"
    effect="light"
    :popper-style="{ maxWidth: 'min(320px, calc(100vw - 32px))', lineHeight: '1.6' }"
  >
    <button
      type="button"
      class="help-tooltip"
      :aria-label="`${label ? label + '：' : '说明：'}${content}`"
      @click.stop.prevent
    >
      <el-icon><QuestionFilled /></el-icon>
    </button>
  </el-tooltip>
</template>

<script setup>
import { QuestionFilled } from '@element-plus/icons-vue'
defineProps({
  content: { type: String, required: true },
  label: { type: String, default: '' },
})
</script>

<style scoped>
.help-tooltip {
  display: inline-flex;
  width: 18px;
  height: 18px;
  flex: 0 0 18px;
  align-items: center;
  justify-content: center;
  margin-inline-start: 4px;
  padding: 0;
  border: 0;
  border-radius: var(--docsy-radius);
  background: transparent;
  color: var(--docsy-text-muted);
  cursor: help;
  vertical-align: middle;
  font-size: 13px;
}
.help-tooltip:hover, .help-tooltip:focus-visible { color: var(--docsy-primary); }
.help-tooltip:focus-visible { outline: 2px solid var(--docsy-primary); outline-offset: 2px; }
</style>
