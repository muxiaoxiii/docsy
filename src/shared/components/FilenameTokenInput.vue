<template>
  <div class="filename-token-input">
    <div class="fn-row">
      <div class="fn-token-strip">
        <span
          v-for="(token, idx) in modelValue"
          :key="token.id"
          class="fn-bubble"
          :class="`fn-${token.type}`"
          title="点击删除"
          @click="removeToken(idx)"
        >{{ tokenLabel(token) }}</span>
      </div>
      <span class="fn-sep">│</span>
      <span class="fn-btn" title="模板名称" @click="addPreset('preset', '模板名')">模板</span>
      <span class="fn-btn" title="日期 YYYYMMDD" @click="addPreset('preset', '日期')">📅</span>
      <el-popover trigger="click" :width="140" popper-class="fn-seq-popover">
        <template #reference>
          <span class="fn-btn" title="序号">🔢</span>
        </template>
        <div class="fn-seq-options">
          <div class="fn-seq-item" @click="addSeq('序号')">1, 2, 3…</div>
          <div class="fn-seq-item" @click="addSeq('序号01')">01, 02, 03…</div>
          <div class="fn-seq-item" @click="addSeq('序号001')">001, 002, 003…</div>
          <div class="fn-seq-item" @click="addSeq('中文序号')">一, 二, 三…</div>
        </div>
      </el-popover>
      <span class="fn-btn" title="连字符" @click="addPreset('literal', '-')">-</span>
      <span class="fn-btn" title="下划线" @click="addPreset('literal', '_')">_</span>
      <el-dropdown trigger="click" @command="addField" :teleported="false">
        <span class="fn-btn fn-field-btn" title="插入字段">字段▾</span>
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
        @focus="inputFocused = true"
      />
      <div v-if="showAc && acFields.length" class="fn-autocomplete">
        <div v-for="f in acFields" :key="f.name" class="fn-ac-item" @mousedown.prevent="insertField(f)">
          {{ f.label || f.name }}
        </div>
      </div>
      <span
        v-if="previewText"
        class="fn-preview"
        :class="{ 'fn-preview-over': previewTooLong }"
        :title="previewTooLong ? `文件名过长（${previewLen}/255）` : previewText"
      >{{ previewText }}</span>
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
  templateName: { type: String, default: '' },
})
const emit = defineEmits(['update:modelValue'])

const inputRef = ref(null)
const showAc = ref(false)
const inputFocused = ref(false)

// ── Text ↔ Tokens sync ──────────────────────────────────

const textValue = ref('')

watch(() => props.modelValue, (tokens) => {
  if (inputFocused.value) return
  textValue.value = tokens.map((t) => {
    if (t.type === 'field') return `[[${t.value}]]`
    if (t.type === 'preset') return `[${t.value}]`
    return t.value
  }).join('')
}, { immediate: true })

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
  inputFocused.value = false
}

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
  commitText()
  emit('update:modelValue', [...props.modelValue, { id: crypto.randomUUID(), type, value }])
}

function addSeq(format) {
  commitText()
  emit('update:modelValue', [...props.modelValue, { id: crypto.randomUUID(), type: 'preset', value: format }])
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
      if (t.value === '模板名') return props.templateName || '模板'
      if (t.value === '日期') return todayStr()
      if (t.value === '序号') return String((props.index || 0) + 1)
      if (t.value === '序号01') return String((props.index || 0) + 1).padStart(2, '0')
      if (t.value === '序号001') return String((props.index || 0) + 1).padStart(3, '0')
      if (t.value === '中文序号') return toChinese((props.index || 0) + 1)
      return `[${t.value}]`
    }
    return t.value
  }).join('')
  return sanitize(name) + '.docx'
})

const previewLen = computed(() => (previewText.value || '').length)
const previewTooLong = computed(() => previewLen.value > 255)

function todayStr() {
  const d = new Date()
  return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
}

function toChinese(n) {
  const chars = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九', '十']
  if (n <= 10) return chars[n]
  if (n < 20) return '十' + chars[n - 10]
  if (n < 100) return chars[Math.floor(n / 10)] + '十' + (n % 10 ? chars[n % 10] : '')
  return String(n)
}

function sanitize(name) {
  return String(name || '').replace(/[/\\:*?"<>|]/g, '_').trim()
}
</script>

<style scoped>
.fn-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  min-height: 30px;
  position: relative;
  flex-wrap: wrap;
}

.fn-token-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.fn-bubble {
  display: inline-block;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
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

.fn-btn {
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

.fn-btn:hover { border-color: var(--docsy-primary); }
.fn-field-btn { color: var(--docsy-text-muted); }

:deep(.fn-field-menu) {
  max-height: 280px;
  overflow-y: auto;
}

.fn-input {
  flex: 1;
  min-width: 100px;
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
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

.fn-preview-over {
  color: #dc2626;
  font-weight: 600;
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

.fn-seq-options {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.fn-seq-item {
  padding: 4px 8px;
  font-size: 12px;
  cursor: strip;
  border-radius: 3px;
}

.fn-seq-item:hover { background: var(--docsy-primary-soft); cursor: pointer; }
</style>
