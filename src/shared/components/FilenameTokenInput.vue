<template>
  <div class="filename-token-input">
    <div class="fn-row">
      <div class="fn-token-strip">
        <span
          v-for="(token, idx) in modelValue"
          :key="token.id"
          class="fn-bubble"
          :class="`fn-${token.type}`"
          :title="bubbleTitle(token)"
          @click="removeToken(idx)"
        >{{ tokenLabel(token) }}</span>
      </div>
      <div class="fn-presets">
        <span
          v-for="p in presetBubbles"
          :key="p.label"
          class="fn-preset-btn"
          :title="p.tip"
          @click="addPreset(p)"
        >{{ p.label }}</span>
      </div>
    </div>
    <div class="fn-row">
      <div class="fn-input-wrap">
        <input
          ref="inputRef"
          :value="textValue"
          class="fn-input"
          placeholder="输入文件名，如 [[案号]]-[日期]"
          @input="onTextInput($event.target.value)"
          @keydown.enter.prevent="commitText"
          @focus="showAutocomplete = true"
          @blur="hideAutocompleteDelayed"
        />
        <div v-if="showAutocomplete && filteredFields.length" class="fn-autocomplete">
          <div
            v-for="f in filteredFields"
            :key="f.name"
            class="fn-ac-item"
            @mousedown.prevent="insertField(f)"
          >{{ f.label || f.name }}</div>
        </div>
      </div>
      <span v-if="previewText" class="fn-preview" :title="previewText">{{ previewText }}</span>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue'

const props = defineProps({
  modelValue: { type: Array, default: () => [] },
  availableFields: { type: Array, default: () => [] },
  sampleValues: { type: Object, default: () => ({}) },
  index: { type: Number, default: 0 },
  defaultName: { type: String, default: '' },
})
const emit = defineEmits(['update:modelValue'])

const inputRef = ref(null)
const showAutocomplete = ref(false)
let blurTimer = null

// ── Token ↔ Text sync ────────────────────────────────────

const textValue = ref('')
let syncing = false

// When tokens change externally, update text
watch(() => props.modelValue, (tokens) => {
  if (syncing) return
  syncing = true
  textValue.value = tokensToString(tokens)
  syncing = false
}, { immediate: true })

function tokensToString(tokens) {
  return tokens.map((t) => {
    if (t.type === 'field') return `[[${t.value}]]`
    if (t.type === 'preset') return `[${t.value}]`
    return t.value
  }).join('')
}

function parseTextToTokens(text) {
  const tokens = []
  const re = /\[\[([^\]]+)\]\]|\[([^\]]+)\]|([^[\]]+)/g
  let m
  while ((m = re.exec(text))) {
    if (m[1]) tokens.push({ id: crypto.randomUUID(), type: 'field', value: m[1] })
    else if (m[2]) tokens.push({ id: crypto.randomUUID(), type: 'preset', value: m[2] })
    else if (m[3]) tokens.push({ id: crypto.randomUUID(), type: 'literal', value: m[3] })
  }
  return tokens
}

function onTextInput(val) {
  textValue.value = val
  // Detect [[ for autocomplete
  const cursor = inputRef.value?.selectionStart || val.length
  const before = val.slice(Math.max(0, cursor - 2), cursor)
  showAutocomplete.value = before === '[['
}

function commitText() {
  syncing = true
  emit('update:modelValue', parseTextToTokens(textValue.value))
  syncing = false
  showAutocomplete.value = false
}

// ── Autocomplete ─────────────────────────────────────────

const filteredFields = computed(() => {
  if (!showAutocomplete.value) return []
  const val = textValue.value
  const lastBracket = val.lastIndexOf('[[')
  if (lastBracket < 0) return []
  const query = val.slice(lastBracket + 2).toLowerCase()
  return props.availableFields.filter((f) =>
    !query || (f.name.toLowerCase().includes(query) || (f.label || '').toLowerCase().includes(query))
  ).slice(0, 8)
})

function insertField(f) {
  const val = textValue.value
  const lastBracket = val.lastIndexOf('[[')
  textValue.value = val.slice(0, lastBracket) + `[[${f.name}]]`
  showAutocomplete.value = false
  commitText()
}

function hideAutocompleteDelayed() {
  blurTimer = setTimeout(() => { showAutocomplete.value = false }, 150)
}

// ── Token strip interactions ─────────────────────────────

function tokenLabel(token) {
  if (token.type === 'field') return token.value
  if (token.type === 'preset') return token.value
  return token.value
}

function bubbleTitle(token) {
  if (token.type === 'field') return `字段：${token.value}（点击删除）`
  if (token.type === 'preset') return `预设：${token.value}（点击删除）`
  return `${token.value}（点击删除）`
}

function removeToken(idx) {
  const tokens = props.modelValue.filter((_, i) => i !== idx)
  emit('update:modelValue', tokens)
}

// ── Preset bubbles ───────────────────────────────────────

const presetBubbles = [
  { label: '📅', type: 'preset', value: '日期', tip: '插入日期（YYYYMMDD）' },
  { label: '🔢', type: 'preset', value: '序号', tip: '插入序号（批量时自增）' },
  { label: '➖', type: 'literal', value: '-', tip: '插入连字符' },
  { label: '_', type: 'literal', value: '_', tip: '插入下划线' },
  { label: '␣', type: 'literal', value: ' ', tip: '插入空格' },
]

function addPreset(p) {
  const tokens = [...props.modelValue, { id: crypto.randomUUID(), type: p.type, value: p.value }]
  emit('update:modelValue', tokens)
}

// ── Preview ──────────────────────────────────────────────

const previewText = computed(() => {
  if (!props.modelValue.length) return ''
  return props.modelValue.map((t) => {
    if (t.type === 'field') {
      const v = props.sampleValues[t.value]
      if (v == null || v === '' || v === false) return t.value
      if (Array.isArray(v)) return v.map((i) => (typeof i === 'object' ? i.text : i)).filter(Boolean).join('、')
      return String(v)
    }
    if (t.type === 'preset') {
      if (t.value === '日期') {
        const d = new Date()
        return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
      }
      if (t.value === '序号') return String((props.index || 0) + 1)
      return `[${t.value}]`
    }
    return t.value
  }).join('') + '.docx'
})
</script>

<style scoped>
.filename-token-input {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
}

.fn-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.fn-token-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  flex: 1;
  min-height: 24px;
  padding: 2px 4px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
}

.fn-bubble {
  display: inline-block;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 11px;
  cursor: pointer;
  user-select: none;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fn-field { background: var(--docsy-primary-soft); color: var(--docsy-primary-hover); }
.fn-preset { background: #fef3c7; color: #92400e; }
.fn-literal { background: transparent; color: var(--docsy-text-muted); }

.fn-bubble:hover { opacity: 0.7; }

.fn-presets {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}

.fn-preset-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 3px;
  font-size: 12px;
  cursor: pointer;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  user-select: none;
}

.fn-preset-btn:hover { border-color: var(--docsy-primary); }

.fn-input-wrap {
  flex: 1;
  position: relative;
}

.fn-input {
  width: 100%;
  height: 24px;
  padding: 0 6px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 4px;
  font-size: 12px;
  background: var(--docsy-surface-base);
  color: var(--docsy-text-strong);
  outline: none;
}

.fn-input:focus { border-color: var(--docsy-primary); }

.fn-input::placeholder { color: var(--docsy-text-muted); font-size: 11px; }

.fn-autocomplete {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  z-index: 10;
  background: var(--docsy-surface-base);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 4px;
  max-height: 160px;
  overflow-y: auto;
  box-shadow: 0 2px 8px rgba(0,0,0,0.12);
}

.fn-ac-item {
  padding: 4px 8px;
  font-size: 12px;
  cursor: pointer;
}

.fn-ac-item:hover { background: var(--docsy-primary-soft); }

.fn-preview {
  font-size: 11px;
  color: var(--docsy-text-muted);
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}
</style>
