<template>
  <div class="doclet-working">
    <DocletSprite :motion="resolvedMotion" />
    <div class="doclet-working__info">
      <div v-if="message" class="doclet-working__message" role="status">{{ message }}</div>
      <div v-if="elapsed" class="doclet-working__elapsed">已用时 {{ elapsed }}</div>
      <div v-if="$slots.actions" class="doclet-working__actions"><slot name="actions" /></div>
    </div>
  </div>
</template>

<script setup>
import DocletSprite from './DocletSprite.vue'
import { computed } from 'vue'

const props = defineProps({
  motion: { type: String, default: '' },
  message: {
    type: String,
    default: 'Doclet 正在处理…',
  },
  elapsed: {
    type: String,
    default: '',
  },
})
const resolvedMotion = computed(() => props.motion || (/检测|分析|扫描|读取|预览/.test(props.message) ? 'review' : 'working'))
</script>

<style scoped>
.doclet-working {
  display: inline-flex;
  align-items: flex-end;
  gap: 4px;
  max-width: calc(100vw - 32px);
  color: var(--el-text-color-regular);
}

.doclet-working__info {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 220px;
  max-width: calc(100vw - 192px);
  margin-bottom: 52px;
  padding: 12px 14px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 14px 14px 14px 4px;
  background: var(--docsy-surface-elevated);
  box-shadow: var(--docsy-shadow-soft);
  overflow-wrap: anywhere;
}

.doclet-working__info::before {
  content: '';
  position: absolute;
  left: -6px;
  bottom: 14px;
  width: 10px;
  height: 10px;
  background: var(--docsy-surface-elevated);
  border-left: 1px solid var(--docsy-border-subtle);
  border-bottom: 1px solid var(--docsy-border-subtle);
  transform: rotate(45deg);
}

.doclet-working__actions {
  margin-top: 6px;
}

.doclet-working__message {
  font-size: 13px;
  line-height: 1.5;
}

.doclet-working__elapsed {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  line-height: 1.4;
}

</style>
