<template>
  <div class="filename-token-input">
    <div class="fn-row">
      <div class="fn-token-strip">
        <span
          v-for="(token, idx) in modelValue"
          :key="token.id"
          class="fn-bubble"
          :class="`fn-${token.type}`"
          :title="`点击删除`"
          @click="removeToken(idx)"
        >{{ tokenLabel(token) }}</span>
      </div>
      <span class="fn-sep">│</span>
      <span class="fn-preset-btn" title="日期（YYYYMMDD）" @click="addPreset('preset', '日期')">📅</span>
      <span class="fn-preset-btn" title="序号（批量自增）" @click="addPreset('preset', '序号')">🔢</span>
      <span class="fn-preset-btn" title="连字符" @click="addPreset('literal', '-')">-</span>
      <span class="fn-preset-btn" title="下划线" @click="addPreset('literal', '_')">_</span>
      <el-dropdown trigger="click" @command="addField" :teleported="false">
        <span class="fn-preset-btn fn-field-btn" title="插入字段">字段▾</span>
        <template #dropdown>
          <el-dropdown-menu class="fn-field-menu">
            <el-dropdown-item v-for="f in availableFields" :key="f.name" :command="f.name">
              {{ f.label || f.name }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <span class="fn-sep">│</span>
      <input
        ref="inputRef"
        v-model="textValue"
        class="fn-input"
        placeholder="或直接输入 [[字段名]]-[日期]"
        @keydown.enter.prevent="commitText"
        @blur="commitText"
        @focus="onFocus"
      />
      <div v-if="showAc && acFields.length" class="fn-autocomplete">
        <div v-for="f in acFields" :key="f.name" class="fn-ac-item" @mousedown.prevent="insertField(f)">
          {{ f.label || f.name }}
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
const showAc = ref(false)
const inputFocused = ref(false)

// ── Text value (two-way with input) ──────────────────────

const textValue = ref('')

// Sync tokens → text only when input is NOT focused
// (so user typing is never interrupted)
watch(() => props.modelValue, (tokens) => {
  if (inputFocused.value) return
  textValue.value = tokensToString(tokens)
}, { immediate: true })

function tokensToString(tokens) {
  return tokens.map((t) => {
    if (t.type === 'field') return `[[${t.value}]]`
    if (t.type === 'preset') return `[${t.value}]`
    return t.value
  }).join('')
}

function commitText() {
  const tokens = []
  const re = /\[\[([^\]]+)\]\]|\[([^\]]+)\]|([^[\]]+)/g
  let m
  while ((m = re.exec(textValue.value))) {
    if (m[1]) tokens.push({ id: crypto.randomUUID(), type: 'field', value: m[1] })
    else if (m[2]) tokens.push({ id: crypto.randomUUID(), type: 'preset', value: m[2] })
    else if (m[3]) tokens.push({ id: crypto.randomUUID(), type: 'literal', value: m[3] })
  }
  emit('update:modelValue', tokens)
  showAc.value = false
}

function onFocus() {
  inputFocused.value = true
  // When focusing, if tokens exist, show their text representation
  // so user can edit it directly
  if (props.modelValue.length && !textValue.value) {
    textValue.value = tokensToString(props.modelValue)
  }
}

// On blur: commit text → tokens, then mark unfocused
watch(inputFocused, (focused) => {
  if (!focused) commitText()
})

// ── Autocomplete ─────────────────────────────────────────

const acFields = computed(() => {
  if (!showAc.value) return []
  const val = textValue.value
  const lastBracket = val.lastIndexOf('[[')
  if (lastBracket < 0) return []
  const query = val.slice(lastBracket + 2).toLowerCase()
  return props.availableFields.filter((f) =>
    !query || f.name.toLowerCase().includes(query) || (f.label || '').toLowerCase().includes(query)
  ).slice(0, 8)
})

// Watch for [[ input to trigger autocomplete
watch(textValue, (val) => {
  if (!inputFocused.value) { showAc.value = false; return }
  const cursor = inputRef.value?.selectionStart || val.length
  showAc.value = val.slice(Math.max(0, cursor - 2), cursor) === '[['
})

function insertField(f) {
  const val = textValue.value
  const lastBracket = val.lastIndexOf('[[')
  textValue.value = val.slice(0, lastBracket) + `[[${f.name}]]`
  showAc.value = false
}

// ── Token strip ──────────────────────────────────────────

function tokenLabel(token) {
  if (token.type === 'field') return token.value
  if (token.type === 'preset') return token.value
  return token.value
}

function removeToken(idx) {
  emit('update:modelValue', props.modelValue.filter((_, i) => i !== idx))
}

function addPreset(type, value) {
  // Commit current input text first, then append
  commitText()
  emit('update:modelValue', [...props.modelValue, { id: crypto.randomUUID(), type, value }])
}

function addField(name) {
  commitText()
  emit('update:modelValue', [...props.modelValue, { id: crypto.randomUUID(), type: 'field', value: name }])
}

// ── Preview ──────────────────────────────────────────────

const previewText = computed(() => {
  const tokens = props.modelValue
  if (!tokens.length) return ''
  const name = tokens.map((t) => {
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
  }).join('')
  return name + '.docx'
})
</script>

<style scoped>
.fn-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  min-height: 30px;
  position: relative;
}

.fn-token-strip {
  display: flex;
  flex-wrap: nowrap;
  gap: 2px;
  overflow-x: auto;
  flex-shrink: 1;
  min-width: 0;
}

.fn-bubble {
  display: inline-block;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
}

.fn-field { background: var(--docsy-primary-soft); color: var(--docsy-primary-hover); }
.fn-preset { background: #fef3c7; color: #92400e; }
.fn-literal { color: var(--docsy-text-muted); }

.fn-bubble:hover { opacity: 0.7; }

.fn-sep {
  color: var(--docsy-border-subtle);
  font-size: 12px;
  flex-shrink: 0;
}

.fn-preset-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  border-radius: 3px;
  font-size: 11px;
  cursor: pointer;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  flex-shrink: 0;
  padding: 0 4px;
}

.fn-preset-btn:hover { border-color: var(--docsy-primary); }

.fn-field-btn {
  font-size: 11px;
  color: var(--docsy-text-muted);
}

/* Field dropdown: max 10 items (~280px), scroll the rest */
:deep(.fn-field-menu) {
  max-height: 280px;
  overflow-y: auto;
}

.fn-input {
  flex: 1;
  min-width: 120px;
  height: 22px;
  padding: 0 5px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 3px;
  font-size: 11px;
  background: var(--docsy-surface-base);
  color: var(--docsy-text-strong);
  outline: none;
}

.fn-input:focus { border-color: var(--docsy-primary); }
.fn-input::placeholder { color: var(--docsy-text-muted); font-size: 11px; }

.fn-preview {
  font-size: 10px;
  color: var(--docsy-text-muted);
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

.fn-autocomplete {
  position: absolute;
  bottom: 100%;
  left: 50%;
  z-index: 10;
  background: var(--docsy-surface-base);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 4px;
  max-height: 200px;
  overflow-y: auto;
  min-width: 140px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.12);
}

.fn-ac-item {
  padding: 3px 8px;
  font-size: 12px;
  cursor: pointer;
}

.fn-ac-item:hover { background: var(--docsy-primary-soft); }
</style>
