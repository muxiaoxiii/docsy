<template>
  <div class="filename-token-input">
    <div class="token-line">
      <div
        v-for="(token, idx) in modelValue"
        :key="token.id"
        class="token-bubble"
        :class="[`token-${token.type}`, { dragging: dragIndex === idx }]"
        draggable="true"
        @dragstart="onDragStart(idx, $event)"
        @dragover.prevent="onDragOver(idx)"
        @drop="onDrop(idx)"
        @dragend="dragIndex = -1"
      >
        <span class="token-label">{{ tokenLabel(token) }}</span>
        <button class="token-remove" @click="removeToken(idx)">×</button>
      </div>
      <div class="token-actions">
        <el-dropdown trigger="click" @command="addToken">
          <button class="token-add">+</button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="field" v-for="f in availableFields" :key="f.name" :command="{ type: 'field', value: f.name }">
                📝 {{ f.label || f.name }}
              </el-dropdown-item>
              <el-dropdown-item divided command="{ type: 'preset', value: '日期' }">📅 日期</el-dropdown-item>
              <el-dropdown-item command="{ type: 'preset', value: '序号' }">🔢 序号</el-dropdown-item>
              <el-dropdown-item command="{ type: 'preset', value: '中文序号' }">🔢 中文序号</el-dropdown-item>
              <el-dropdown-item divided command="{ type: 'literal', value: '-' }">➖ 连字符 -</el-dropdown-item>
              <el-dropdown-item command="{ type: 'literal', value: '_' }">➖ 下划线 _</el-dropdown-item>
              <el-dropdown-item command="{ type: 'literal', value: ' ' }">➖ 空格</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>
    <div class="token-preview" v-if="preview">
      <span class="preview-label">预览：</span>
      <code>{{ preview }}</code>
    </div>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'

const props = defineProps({
  modelValue: { type: Array, default: () => [] },
  availableFields: { type: Array, default: () => [] },
  sampleValues: { type: Object, default: () => ({}) },
  index: { type: Number, default: 0 },
})

const emit = defineEmits(['update:modelValue'])

const dragIndex = ref(-1)

function tokenLabel(token) {
  if (token.type === 'field') return `{${token.value}}`
  if (token.type === 'preset') return `[${token.value}]`
  return token.value || '·'
}

function addToken(item) {
  // Parse string command like "{ type: 'field', value: 'x' }"
  let parsed = item
  if (typeof item === 'string') {
    try { parsed = JSON.parse(item.replace(/'/g, '"')) } catch { return }
  }
  if (!parsed?.type) return
  const tokens = [...props.modelValue, { id: Date.now(), ...parsed }]
  emit('update:modelValue', tokens)
}

function removeToken(idx) {
  const tokens = props.modelValue.filter((_, i) => i !== idx)
  emit('update:modelValue', tokens)
}

function onDragStart(idx, ev) {
  dragIndex.value = idx
  ev.dataTransfer.effectAllowed = 'move'
}

function onDragOver(idx) {
  // Visual feedback handled by CSS
}

function onDrop(idx) {
  if (dragIndex.value < 0 || dragIndex.value === idx) return
  const tokens = [...props.modelValue]
  const [moved] = tokens.splice(dragIndex.value, 1)
  tokens.splice(idx, 0, moved)
  emit('update:modelValue', tokens)
  dragIndex.value = -1
}

const preview = computed(() => {
  return props.modelValue.map((token) => {
    if (token.type === 'field') {
      return props.sampleValues[token.value] || token.value
    }
    if (token.type === 'preset') {
      if (token.value === '日期') {
        const d = new Date()
        return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
      }
      if (token.value === '序号') return String(props.index + 1)
      if (token.value === '中文序号') return ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十'][props.index] || String(props.index + 1)
      return `[${token.value}]`
    }
    return token.value
  }).join('')
})
</script>

<style scoped>
.filename-token-input {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.token-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  min-height: 34px;
}

.token-bubble {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  cursor: grab;
  user-select: none;
  transition: opacity 0.15s;
}

.token-bubble.dragging {
  opacity: 0.4;
}

.token-field {
  background: var(--docsy-primary-soft);
  color: var(--docsy-primary-hover);
}

.token-preset {
  background: #fef3c7;
  color: #92400e;
}

.token-literal {
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  border: 1px dashed var(--docsy-border-subtle);
}

.token-label {
  white-space: nowrap;
}

.token-remove {
  background: none;
  border: none;
  cursor: pointer;
  color: inherit;
  opacity: 0.5;
  font-size: 14px;
  padding: 0 2px;
  line-height: 1;
}

.token-remove:hover {
  opacity: 1;
}

.token-add {
  background: none;
  border: 1px dashed var(--docsy-border-subtle);
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  padding: 2px 8px;
  color: var(--docsy-text-muted);
}

.token-add:hover {
  border-color: var(--docsy-primary);
  color: var(--docsy-primary);
}

.token-preview {
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.preview-label {
  margin-right: 4px;
}

.token-preview code {
  background: var(--docsy-surface-muted);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 12px;
}
</style>
