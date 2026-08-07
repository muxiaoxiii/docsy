<template>
  <div class="fn-layout">
    <!-- Left: Preview blocks (can wrap) -->
    <div class="fn-preview-strip">
      <span
        v-for="(token, idx) in modelValue"
        :key="token.id"
        class="fn-block"
        :class="`fn-${token.type}`"
        title="点击删除"
        @click="removeToken(idx)"
      >{{ previewToken(token) }}</span>
      <span v-if="!modelValue.length" class="fn-hint">点击右侧按钮或在中间输入文件名规则</span>
    </div>

    <!-- Middle: Text input -->
    <div class="fn-input-col">
      <input
        ref="inputRef"
        v-model="textValue"
        class="fn-input"
        placeholder="[[字段名]]-[日期]"
        @keydown.enter.prevent="commitText"
        @blur="commitText"
        @focus="inputFocused = true"
      />
      <div v-if="showAc && acFields.length" class="fn-autocomplete">
        <div v-for="f in acFields" :key="f.name" class="fn-ac-item" @mousedown.prevent="insertField(f)">
          {{ f.label || f.name }}
        </div>
      </div>
    </div>

    <!-- Right: Buttons (fixed width) -->
    <div class="fn-btn-col">
      <span class="fn-btn" :title="templateName || '模板名'" @click="addPreset('preset', '模板名')">模板名</span>

      <span class="fn-btn fn-btn-split" @click="addPreset('preset', '日期')">
        <span class="fn-btn-main">📅</span>
        <span class="fn-btn-arrow" @click.stop.prevent="toggleDatePopover">▾</span>
      </span>
      <div v-if="datePopover" class="fn-popover fn-pop-right" @mousedown.prevent>
        <div class="fn-pop-item" @click="addAndClose('preset', '日期', 'date')">20260806</div>
        <div class="fn-pop-item" @click="addAndClose('preset', '日期-', 'date')">2026-08-06</div>
        <div class="fn-pop-item" @click="addAndClose('preset', '日期短', 'date')">0806</div>
      </div>

      <span class="fn-btn fn-btn-split" @click="addPreset('preset', '序号')">
        <span class="fn-btn-main">🔢</span>
        <span class="fn-btn-arrow" @click.stop.prevent="toggleSeqPopover">▾</span>
      </span>
      <div v-if="seqPopover" class="fn-popover fn-pop-right" @mousedown.prevent>
        <div class="fn-pop-item" @click="addAndClose('preset', '序号', 'seq')">1, 2, 3…</div>
        <div class="fn-pop-item" @click="addAndClose('preset', '序号01', 'seq')">01, 02, 03…</div>
        <div class="fn-pop-item" @click="addAndClose('preset', '序号001', 'seq')">001, 002, 003…</div>
        <div class="fn-pop-item" @click="addAndClose('preset', '中文序号', 'seq')">一, 二, 三…</div>
      </div>

      <span class="fn-btn" title="连字符" @click="addPreset('literal', '-')">-</span>
      <span class="fn-btn" title="下划线" @click="addPreset('literal', '_')">_</span>
      <span class="fn-btn" title="竖线" @click="addPreset('literal', '丨')">丨</span>

      <el-dropdown trigger="click" @command="addField" :teleported="false">
        <span class="fn-btn fn-field-btn">字段▾</span>
        <template #dropdown>
          <el-dropdown-menu class="fn-field-menu">
            <el-dropdown-item v-for="f in availableFields" :key="f.name" :command="f.name">
              {{ f.label || f.name }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
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
const seqPopover = ref(false)
const datePopover = ref(false)

// ── Text ↔ Tokens ───────────────────────────────────────

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

// ── Actions ──────────────────────────────────────────────

function removeToken(idx) {
  emit('update:modelValue', props.modelValue.filter((_, i) => i !== idx))
}

function addPreset(type, value) {
  commitText()
  emit('update:modelValue', [...props.modelValue, { id: crypto.randomUUID(), type, value }])
}

function addField(name) {
  commitText()
  emit('update:modelValue', [...props.modelValue, { id: crypto.randomUUID(), type: 'field', value: name }])
}

function toggleSeqPopover() { seqPopover.value = !seqPopover.value; datePopover.value = false }
function toggleDatePopover() { datePopover.value = !datePopover.value; seqPopover.value = false }

function addAndClose(type, value, which) {
  addPreset(type, value)
  if (which === 'seq') seqPopover.value = false
  if (which === 'date') datePopover.value = false
}

// ── Preview token rendering ──────────────────────────────

function previewToken(token) {
  if (token.type === 'field') {
    const v = props.sampleValues[token.value]
    if (v != null && v !== '' && v !== false) {
      if (Array.isArray(v)) return v.map((i) => (typeof i === 'object' ? i.text : i)).filter(Boolean).join('、')
      return String(v)
    }
    return '___'
  }
  if (token.type === 'preset') {
    if (token.value === '模板名') return props.templateName || '模板'
    if (token.value === '日期') return todayStr()
    if (token.value === '日期-') return todayDash()
    if (token.value === '日期短') return todayShort()
    if (token.value === '序号') return String((props.index || 0) + 1)
    if (token.value === '序号01') return String((props.index || 0) + 1).padStart(2, '0')
    if (token.value === '序号001') return String((props.index || 0) + 1).padStart(3, '0')
    if (token.value === '中文序号') return toChinese((props.index || 0) + 1)
    return token.value
  }
  return token.value
}

function todayStr() {
  const d = new Date()
  return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
}
function todayDash() {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}
function todayShort() {
  const d = new Date()
  return `${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
}
function toChinese(n) {
  const c = ['零','一','二','三','四','五','六','七','八','九','十']
  if (n <= 10) return c[n]
  if (n < 20) return '十' + c[n - 10]
  if (n < 100) return c[Math.floor(n / 10)] + '十' + (n % 10 ? c[n % 10] : '')
  return String(n)
}
</script>

<style scoped>
.fn-layout {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  padding: 6px 8px;
  min-height: 30px;
  position: relative;
}

/* ── Left: preview blocks ──────────────────────────────── */

.fn-preview-strip {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 1px;
  align-items: center;
  min-height: 22px;
}

.fn-block {
  display: inline-block;
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fn-block:hover { opacity: 0.7; }

.fn-field { background: var(--docsy-primary-soft); color: var(--docsy-primary-hover); }
.fn-preset { background: #fef3c7; color: #92400e; }
.fn-literal { color: var(--docsy-text-muted); }

.fn-hint {
  font-size: 11px;
  color: var(--docsy-text-muted);
}

/* ── Middle: input ─────────────────────────────────────── */

.fn-input-col {
  flex: 1;
  min-width: 80px;
  position: relative;
}

.fn-input {
  width: 100%;
  height: 22px;
  padding: 0 5px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 3px;
  font-size: 11px;
  background: var(--docsy-surface-base);
  color: var(--docsy-text-strong);
  outline: none;
  box-sizing: border-box;
}

.fn-input:focus { border-color: var(--docsy-primary); }
.fn-input::placeholder { color: var(--docsy-text-muted); font-size: 11px; }

.fn-autocomplete {
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  z-index: 10;
  background: var(--docsy-surface-base);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 4px;
  max-height: 200px;
  overflow-y: auto;
  box-shadow: 0 2px 8px rgba(0,0,0,0.12);
}

.fn-ac-item { padding: 3px 8px; font-size: 12px; cursor: pointer; }
.fn-ac-item:hover { background: var(--docsy-primary-soft); }

/* ── Right: buttons (fixed) ────────────────────────────── */

.fn-btn-col {
  display: flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
  flex-wrap: wrap;
  position: relative;
}

.fn-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 20px;
  border-radius: 3px;
  font-size: 11px;
  cursor: pointer;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  flex-shrink: 0;
  padding: 0 5px;
  user-select: none;
  white-space: nowrap;
}

.fn-btn:hover { border-color: var(--docsy-primary); }

.fn-btn-split { padding: 0; display: inline-flex; gap: 0; }
.fn-btn-main { padding: 0 4px; display: inline-flex; align-items: center; }
.fn-btn-arrow {
  font-size: 9px;
  padding: 0 2px;
  display: inline-flex;
  align-items: center;
  border-left: 1px solid var(--docsy-border-subtle);
  color: var(--docsy-text-muted);
  line-height: 1;
}
.fn-btn-arrow:hover { color: var(--docsy-primary); }
.fn-field-btn { color: var(--docsy-text-muted); }

/* ── Popover ───────────────────────────────────────────── */

.fn-popover {
  position: absolute;
  top: 100%;
  right: 0;
  z-index: 20;
  background: var(--docsy-surface-base);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.12);
  min-width: 100px;
}

.fn-pop-item {
  padding: 4px 10px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.fn-pop-item:hover { background: var(--docsy-primary-soft); }

/* ── Field dropdown ────────────────────────────────────── */

:deep(.fn-field-menu) { max-height: 280px; overflow-y: auto; }
</style>
