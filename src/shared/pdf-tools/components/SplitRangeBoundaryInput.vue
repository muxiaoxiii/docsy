<template>
  <span class="split-boundary-input" @click.stop @keydown.enter.stop.prevent="commit()">
    <el-input
      ref="input"
      v-model="draft"
      size="small"
      inputmode="numeric"
      placeholder="待设置"
      :aria-label="label"
      @focus="emit('navigate')"
      @blur="commit()"
      @keydown.up.stop.prevent="step(1)"
      @keydown.down.stop.prevent="step(-1)"
    />
    <span class="split-boundary-steps">
      <button
        type="button"
        :aria-label="label + '增加'"
        :disabled="Number(draft) >= max"
        @mousedown.prevent
        @click.stop="step(1)"
      >
        ⌃
      </button>
      <button
        type="button"
        :aria-label="label + '减少'"
        :disabled="Number(draft) <= min"
        @mousedown.prevent
        @click.stop="step(-1)"
      >
        ⌄
      </button>
    </span>
  </span>
</template>

<script setup>
import { ref, watch } from 'vue'
const props = defineProps({
  value: { type: Number, default: null },
  max: { type: Number, required: true },
  min: { type: Number, default: 1 },
  label: { type: String, required: true },
})
const emit = defineEmits(['navigate', 'commit'])
const draft = ref(props.value == null ? '' : String(props.value))
const lastSubmitted = ref(props.value)
watch(
  () => props.value,
  (value) => {
    draft.value = value == null ? '' : String(value)
    lastSubmitted.value = value
  },
)
function commit(value = draft.value) {
  if (String(value ?? '').trim() === '' || !Number.isFinite(Number(value))) {
    draft.value = props.value == null ? '' : String(props.value)
    return
  }
  const page = Math.max(props.min ?? 1, Math.min(props.max, Math.round(Number(value))))
  draft.value = String(page)
  if (page !== lastSubmitted.value) {
    lastSubmitted.value = page
    emit('commit', page)
  }
}
function step(delta) {
  if (String(draft.value).trim() === '') return commit(props.min ?? 1)
  const value =
    String(draft.value).trim() !== '' && Number.isFinite(Number(draft.value))
      ? Number(draft.value)
      : (props.value ?? props.min ?? 1)
  commit(value + delta)
}
</script>

<style scoped>
.split-boundary-input {
  position: relative;
  display: inline-flex;
  align-items: center;
  flex: 0 0 72px;
  width: 72px;
}
.split-boundary-input :deep(.el-input__wrapper) {
  padding-right: 22px;
}
.split-boundary-input :deep(.el-input__inner) {
  text-align: center;
}
.split-boundary-steps {
  position: absolute;
  right: 1px;
  top: 1px;
  bottom: 1px;
  width: 19px;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--el-border-color);
  border-radius: 0 var(--el-border-radius-base) var(--el-border-radius-base) 0;
  overflow: hidden;
}
.split-boundary-steps button {
  flex: 1;
  min-height: 0;
  padding: 0;
  border: 0;
  background: var(--el-fill-color-light);
  color: var(--el-text-color-regular);
  font: inherit;
  font-size: 12px;
  line-height: 10px;
  cursor: pointer;
}
.split-boundary-steps button + button {
  border-top: 1px solid var(--el-border-color);
}
.split-boundary-steps button:hover {
  color: var(--el-color-primary);
}
.split-boundary-steps button:disabled {
  color: var(--el-text-color-disabled);
  cursor: default;
}
</style>
