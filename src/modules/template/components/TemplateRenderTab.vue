<template>
  <section class="workspace">
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>模板库</h3>
          <p>保存到 Docsy 的模板会显示在这里，选择后直接填写。</p>
        </div>
        <div class="actions inline">
          <el-button :loading="templateLibraryLoading" @click="$emit('load-template-library')">刷新</el-button>
        </div>
      </div>

      <div v-if="templateLibrary.length" class="template-library-grid">
        <div
          v-for="item in templateLibrary"
          :key="item.path"
          class="template-library-card"
          :class="{ active: item.path === templatePath }"
          @click="$emit('open-template-from-library', item)"
        >
          <strong>{{ item.name }}</strong>
          <span>{{ item.fieldCount }} 个字段</span>
          <small>{{ shortDateTime(item.updated) }}</small>
          <div class="template-card-actions">
            <el-button size="small" text @click.stop="$emit('edit-template', item)">编辑</el-button>
            <el-button size="small" text type="danger" @click.stop="$emit('delete-template', item)">删除</el-button>
          </div>
        </div>
      </div>
      <el-empty v-else description="还没有保存到软件内部的模板" />
    </div>

    <div v-if="templateManifest" class="panel form-panel">
      <div class="panel-header compact">
        <div>
          <h3>{{ templateManifest.template.name }}</h3>
          <p>{{ renderableTemplateFields.length }} 个字段。输入时会从历史和通用字段里即时检索。</p>
        </div>
        <div class="actions inline field-filter-actions">
          <el-input
            :model-value="fieldSearch"
            size="small"
            clearable
            placeholder="搜索字段"
            class="field-search-input"
            @update:model-value="$emit('update:fieldSearch', $event)"
          />
        </div>
        <el-button type="success" :loading="rendering" @click="$emit('render-template')">
          生成 Word
        </el-button>
        <span v-if="filenamePreview" class="filename-preview" :title="filenamePreview">{{ filenamePreview }}</span>
        <el-button :disabled="!templateManifest" @click="$emit('toggle-fill-preview')">
          {{ fillPreviewVisible ? '收起预览' : '预览' }}
        </el-button>
        <el-dropdown @command="$emit('batch-command', $event)" trigger="click">
          <el-button :loading="batchProcessing"
            >批量填写 <el-icon class="el-icon--right"><arrow-down /></el-icon
          ></el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="export">导出字段表</el-dropdown-item>
              <el-dropdown-item command="import">导入并生成</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>

      <div class="template-form-grid">
        <section
          v-for="field in fillPositionEntries"
          :key="`${field.id}#${field.posIndex ?? 0}`"
          class="fill-field-card"
          :class="{ 'duplicate-field': field._isDuplicate, 'follow-field': field.posIndex > 0 || (!field.editable && !field.isReference) }"
        >
          <div class="fill-field-header">
            <el-tooltip :content="fillFieldLabel(field)" placement="top" :show-after="500" :disabled="fillFieldLabel(field).length < 12">
              <strong class="field-name">{{ fillFieldLabel(field) }}</strong>
            </el-tooltip>
            <span v-if="field.posIndex > 0" class="position-label fill-all-tag">位置{{ field.posIndex + 1 }}</span>

            <em v-if="field.required" class="fill-all-tag">必填</em>
            <el-tag v-if="field.isDuplicate" size="small" effect="plain" type="info" class="fill-all-tag">
              同名字段，自动同步
            </el-tag>
            <el-tag v-else-if="field.fillAllPositions && field.posIndex > 0 && !hasSlotTypeOverride(field) && effectiveFieldType(field) === 'reference'" size="small" effect="plain" class="fill-all-tag">
              {{ followerReferenceLabel(field) }}
            </el-tag>
          </div>
          <div class="fill-field-body">
            <div v-if="fieldStructureHints(field).length" class="fill-structure-hints">
              <span v-for="hint in fieldStructureHints(field)" :key="hint.key" class="fill-structure-hint">
                {{ hint.label }}：<code :class="{ empty: hint.empty }">{{ hint.text }}</code>
                <em>空值时删除</em>
              </span>
            </div>
            <template v-if="field.editable || field.isReference || hasSlotTypeOverride(field) || effectiveFieldType(field) === 'reference'">
            <div v-if="effectiveFieldType(field) === 'date'" class="date-fill-row">
              <el-date-picker
                v-if="getEntryValue(field) !== '留空'"
                :model-value="getEntryValue(field)"
                type="date"
                value-format="YYYY-MM-DD"
                @update:model-value="setEntryValue(field, $event)"
              />
              <span v-else class="date-blank-text">留空</span>
              <button type="button" class="date-blank-btn" @click="toggleDateBlank(field)">
                {{ getEntryValue(field) === '留空' ? '输入日期' : '留空' }}
              </button>
            </div>
            <el-checkbox
              v-else-if="effectiveFieldType(field) === 'checkbox'"
              :model-value="getEntryValue(field)"
              @update:model-value="setEntryValue(field, $event)"
            >
              {{ firstOptionLabel(field) || '选中' }}
            </el-checkbox>
            <el-radio-group
              v-else-if="effectiveFieldType(field) === 'radio_group'"
              :model-value="getEntryValue(field)"
              @update:model-value="setEntryValue(field, $event)"
            >
              <el-radio v-for="option in field.options" :key="option.id" :label="option.id">
                {{ option.label }}
              </el-radio>
            </el-radio-group>
            <el-checkbox-group
              v-else-if="effectiveFieldType(field) === 'checkbox_group'"
              :model-value="getEntryValue(field)"
              @update:model-value="setEntryValue(field, $event)"
            >
              <el-checkbox v-for="option in field.options" :key="option.id" :label="option.id">
                {{ option.label }}
              </el-checkbox>
            </el-checkbox-group>
            <div v-else-if="effectiveFieldType(field) === 'party_list'" class="party-list-editor">
              <div
                v-for="(item, index) in getPartyListRows(field)"
                :key="index"
                class="party-list-row compact"
                :class="{ 'no-suffix': !partyFieldUsesSuffix(field) }"
              >
                <span class="party-order">{{ index + 1 }}</span>
                <el-autocomplete
                  v-model="item.text"
                  size="small"
                  placeholder="名称"
                  :fetch-suggestions="(query, cb) => $emit('complete-field', field, query, cb)"
                  @input="$emit('schedule-history-refresh')"
                />
                <el-select
                  v-if="partyFieldUsesSuffix(field)"
                  v-model="item.suffix"
                  size="small"
                  filterable
                  allow-create
                  default-first-option
                  placeholder="后缀"
                  @change="$emit('schedule-history-refresh')"
                >
                  <el-option
                    v-for="suffix in partySuffixOptions(field)"
                    :key="suffix"
                    :label="suffix"
                    :value="suffix"
                  />
                </el-select>
                <div class="party-row-actions">
                  <el-button
                    size="small"
                    text
                    :disabled="index === 0"
                    @click="$emit('move-party-item', field, index, -1)"
                  >
                    上移
                  </el-button>
                  <el-button
                    size="small"
                    text
                    :disabled="index === getPartyListRows(field).length - 1"
                    @click="$emit('move-party-item', field, index, 1)"
                  >
                    下移
                  </el-button>
                  <el-button size="small" text type="danger" @click="$emit('remove-party-item', field, index)">
                    删除
                  </el-button>
                </div>
              </div>
              <div v-if="partyFieldStructureHint(field)" class="field-structure-hint">
                {{ partyFieldStructureHint(field) }}
              </div>
              <div class="party-list-add-row">
                <el-button size="small" @click="$emit('add-party-item', field)">添加一项</el-button>
              </div>
            </div>
            <div v-else-if="effectiveFieldType(field) === 'reference'" class="reference-fill-editor">
              <div v-if="isReferenceSingleCandidate(field)" class="reference-fixed-value">
                {{ referenceFixedDisplayLabel(field) }}
              </div>
              <el-select
                v-else
                :model-value="getReferenceSelection(field)"
                filterable
                clearable
                class="reference-select-muted"
                :placeholder="followerReferenceLabel(field)"
                @update:model-value="$emit('reference-selection-change', field, $event)"
              >
                <el-option
                  v-for="item in referenceFillOptions(field)"
                  :key="item.key"
                  :label="item.label"
                  :value="item.key"
                />
              </el-select>
              <p v-if="effectiveFieldType(field) === 'reference' && !hasSlotTypeOverride(field)" class="setting-caption">引用字段从已填字段取值，不能手动输入；前后缀在"…"菜单里设置。</p>
            </div>
            <el-select
              v-else-if="effectiveFieldType(field) === 'select'"
              :model-value="getEntryValue(field)"
              filterable
              allow-create
              default-first-option
              clearable
              placeholder="选择或输入"
              @update:model-value="setEntryValue(field, $event)"
            >
              <el-option
                v-for="opt in selectFieldOptions(field)"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
            <el-autocomplete
              v-else
              :model-value="getEntryValue(field)"
              size="small"
              :fetch-suggestions="(query, cb) => $emit('complete-field', field, query, cb)"
              @input="(val) => { setEntryValue(field, val); $emit('schedule-history-refresh') }"
              @keyup.enter="$event.target.blur()"
            />
            <div v-if="templateStoredSuggestionItems(field).length" class="suggestion-row">
              <el-tag
                v-for="item in templateStoredSuggestionItems(field)"
                :key="`${field.name}-${item.source}-${item.display}`"
                size="small"
                effect="plain"
                class="suggestion-tag"
                @click="$emit('apply-suggestion', field, item.value)"
              >
                {{ item.display }}
                <span v-if="item.count">×{{ item.count }}</span>
              </el-tag>
            </div>
            </template>
            <template v-else>
              <div class="follow-value">{{ getEntryValue(field) || '（空）' }}</div>

            </template>
            <el-popover placement="bottom-end" trigger="click" width="280">
              <template #reference>
                <button class="field-more-button" type="button">…</button>
              </template>
              <div class="fill-structure-editor">
                <strong>{{ structureEditorTitle(field) }}</strong>
                <div class="setting-row">
                  <span class="setting-label">字段类型</span>
                  <el-select
                    :model-value="typeGroupOf(effectiveFieldType(field))"
                    size="small"
                    class="type-group-select"
                    teleported
                    @update:model-value="(group) => onFieldTypeGroupChange(field, group)"
                  >
                    <el-option
                      v-for="item in fieldTypeOverrideGroups"
                      :key="item.value"
                      :value="item.value"
                      :label="item.label"
                    />
                  </el-select>
                  <el-select
                    v-if="typeGroupSubOptions(effectiveFieldType(field))"
                    :model-value="effectiveFieldType(field)"
                    size="small"
                    class="type-sub-select"
                    teleported
                    @update:model-value="(type) => $emit('set-field-type-override', field, type)"
                  >
                    <el-option
                      v-for="sub in typeGroupSubOptions(effectiveFieldType(field))"
                      :key="sub.value"
                      :value="sub.value"
                      :label="sub.label"
                    />
                  </el-select>
                </div>
                <el-input
                  :model-value="getStructureOverride(field).prefix"
                  size="small"
                  placeholder="前缀"
                  @input="(val) => setStructureOverride(field, 'prefix', val)"
                >
                  <template #prepend>前缀</template>
                </el-input>
                <el-input
                  v-if="!fieldUsesRepeatableSuffix(field)"
                  :model-value="getStructureOverride(field).suffix"
                  size="small"
                  placeholder="后缀"
                  @input="(val) => setStructureOverride(field, 'suffix', val)"
                >
                  <template #prepend>后缀</template>
                </el-input>
                <p v-else class="setting-caption">这是列表项后缀，每一项单独设置；字段整体后缀不在这里修改。</p>
                <div v-if="isReferenceablePosition(field)" class="setting-row">
                  <span class="setting-label">引用来源</span>
                  <el-select
                    :model-value="getSavedReferenceKey(field)"
                    filterable
                    clearable
                    placeholder="同字段首个位置"
                    size="small"
                    teleported
                    @update:model-value="(key) => $emit('save-field-reference', field, key || '')"
                  >
                    <el-option
                      v-for="item in referenceFillOptions(field)"
                      :key="item.key"
                      :label="item.label"
                      :value="item.key"
                    />
                  </el-select>
                  <p class="setting-caption">选择后此位置引用所选字段的值，并保存到模板。</p>
                </div>
                <div v-if="effectiveFieldType(field) === 'date'" class="setting-row">
                  <span class="setting-label">日期格式</span>
                  <el-select
                    :model-value="field.dateFormat || 'iso'"
                    size="small"
                    teleported
                    @update:model-value="(fmt) => $emit('save-field-date-format', field, fmt)"
                  >
                    <el-option v-for="item in dateFormatOptions" :key="item.value" :label="item.label" :value="item.value" />
                  </el-select>
                </div>
              </div>
            </el-popover>
          </div>
        </section>
      </div>

      <div v-if="fillPreviewVisible && fillPreviewText" class="fill-preview-panel">
        <div class="fill-preview-header">
          <h3>填写预览</h3>
          <p>未填入的字段用方括号标注，以最终生成效果为准。</p>
        </div>
        <DocumentPreview
          v-if="fillDocumentRuns.length && fillPreviewOverlays.length"
          :runs="fillDocumentRuns"
          :overlays="fillPreviewOverlays"
          mode="fill"
        />
        <pre v-else class="fill-preview-text">{{ fillPreviewText }}</pre>
      </div>
    </div>
  </section>
</template>

<script setup>
import { ArrowDown } from '@element-plus/icons-vue'
import DocumentPreview from '@/shared/components/DocumentPreview.vue'
import {
  shortDateTime,
  fillFieldLabel,
  FIELD_TYPE_GROUPS,
  typeGroupOf,
  typeGroupSubOptions,
  typeActualOf,
  partyFieldUsesSuffix,
  partySuffixOptions,
  partyFieldStructureHint,
  fieldUsesRepeatableSuffix,
  isEmptyValue,
  displayValue,
  parseReferenceSourceKey,
  partyItemsToValues,
  fieldFormKey,
} from '../composables/fieldRowUtils.js'

const props = defineProps({
  // Template state
  templatePath: { type: String, default: '' },
  templateManifest: { type: Object, default: null },
  templateLibrary: { type: Array, default: () => [] },
  templateLibraryLoading: { type: Boolean, default: false },
  filenameTokens: { type: Array, default: () => [] },
  // Form state (parent owns these, child reads via getters)
  formValues: { type: Object, default: () => ({}) },
  referenceSelections: { type: Object, default: () => ({}) },
  structureOverrides: { type: Object, default: () => ({}) },
  typeOverrides: { type: Object, default: () => ({}) },
  historyContext: {
    type: Object,
    default: () => ({ lastValues: {}, fieldSuggestions: {}, semanticSuggestions: {}, associationSuggestions: {} }),
  },
  // UI state
  rendering: { type: Boolean, default: false },
  batchProcessing: { type: Boolean, default: false },
  fieldSearch: { type: String, default: '' },
  // Fill preview
  fillPreviewVisible: { type: Boolean, default: false },
  fillPreviewText: { type: String, default: '' },
  fillPreviewOverlays: { type: Array, default: () => [] },
  fillDocumentRuns: { type: Array, default: () => [] },
  // Computed from parent
  renderableTemplateFields: { type: Array, default: () => [] },
  // Position entries (one card per document position; followers are read-only
  // copies of the primary field). Ordering handled by the parent.
  fillPositionEntries: { type: Array, default: () => [] },
  filteredRenderableFields: { type: Array, default: () => [] },
})

const emit = defineEmits([
  'load-template-library',
  'select-template-package',
  'open-template-from-library',
  'edit-template',
  'delete-template',
  'update:fieldSearch',
  'render-template',
  'batch-command',
  'schedule-history-refresh',
  'complete-field',
  'move-party-item',
  'remove-party-item',
  'add-party-item',
  'reference-selection-change',
  'apply-suggestion',
  'set-field-type-override',
  'update-form-value',
  'update-structure-override',
  'save-field-reference',
  'save-field-date-format',
  'toggle-fill-preview',
])

// ── Helpers ──────────────────────────────────────────────────────────────────

function effectiveFieldType(field) {
  const slotKey = slotKeyFor(field)
  if (props.typeOverrides[slotKey]) return props.typeOverrides[slotKey]
  if (props.typeOverrides[field.id]) return props.typeOverrides[field.id]
  // Follower positions without an explicit override display as "reference"
  // (following the primary position is the fillAllPositions behavior, not a type).
  if ((field.posIndex ?? 0) > 0 && field.fillAllPositions) return 'reference'
  return field.type
}

function slotKeyFor(field) {
  const base = field?.id || field?.name || ''
  const pos = field?.posIndex ?? 0
  return pos > 0 ? `${base}#${pos}` : base
}

// A follower made independent by a type change or a custom reference source
// stores its own slot value.
function hasSlotTypeOverride(field) {
  return Boolean(props.typeOverrides[slotKeyFor(field)])
}

// Filename preview
const filenamePreview = computed(() => {
  if (!props.filenameTokens.length) return ''
  const parts = props.filenameTokens.map((t) => {
    if (t.type === 'literal') return t.value
    if (t.type === 'field') {
      const val = props.formValues[t.value]
      return val || `[${t.value}]`
    }
    if (t.type === 'preset') {
      if (t.value === '日期') {
        const d = new Date()
        return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
      }
      if (t.value === '模板名') return props.templateManifest?.name || '模板'
      if (t.value === '序号') return '1'
      return t.value
    }
    return t.value || ''
  })
  return parts.join('') + '.docx'
})

// Types offered in the fill-page "…" menu (fillable types only; the structural
// link/action types belong to the build page).
const fieldTypeOverrideGroups = FIELD_TYPE_GROUPS.filter((g) => !['link', 'action'].includes(g.value))

function onFieldTypeGroupChange(field, group) {
  const actual = typeActualOf(group, null)
  emit('set-field-type-override', field, actual)
}

function getEntryValue(field) {
  if (field.isDuplicate) return getFormValueByPrimary(field)
  const slotKey = slotKeyFor(field)
  const base = fieldFormKey(field)
  // A follower made independent by a type change or a custom reference source
  // stores its own slot value.
  if ((field.posIndex ?? 0) > 0) {
    const slotValue = props.formValues[slotKey]
    if (props.typeOverrides[slotKey] || slotValue !== undefined) return slotValue
  }
  return props.formValues[base]
}

function setEntryValue(field, value) {
  if (field.isDuplicate) return
  const slotKey = slotKeyFor(field)
  emit('update-form-value', props.typeOverrides[slotKey] ? slotKey : fieldFormKey(field), value)
}

// Toggle a date field between its picked value and the "留空" blank marker
// (rendered as "    年  月  日" so the doc can be filled in by hand).
function toggleDateBlank(field) {
  const current = getEntryValue(field)
  setEntryValue(field, current === '留空' ? '' : '留空')
}

function firstOptionLabel(field) {
  return field.options?.[0]?.label || ''
}

const dateFormatOptions = [
  { value: 'iso', label: '数字 2026-08-05' },
  { value: 'cn', label: '中文 2026年8月5日' },
  { value: 'cn_full', label: '中文大写 二零二六年八月五日' },
  { value: 'en_long', label: '英文 August 5, 2026' },
  { value: 'en_short', label: '英文缩写 Aug. 5, 2026' },
  { value: 'en_dmy', label: '英文日优先 5 August 2026' },
  { value: 'en_ordinal', label: '英文序数 2026 August 5th' },
  { value: 'blank', label: '留空 年月日手写' },
]

// Follower positions (fillAllPositions slot > 0) and reference fields can
// repoint their data source from the "…" menu.
function isReferenceablePosition(field) {
  if (field.isReference) return true
  // Followers show the reference source dropdown only when in reference mode
  // (no type override, or override is 'reference'). Once changed to an
  // independent type (e.g. 'text'), the dropdown is hidden.
  if (field.fillAllPositions && (field.posIndex ?? 0) > 0) {
    return effectiveFieldType(field) === 'reference'
  }
  return false
}

function getSavedReferenceKey(field) {
  const ref = props.referenceSelections[`${field.id}#${field.posIndex ?? 0}`]
  return ref || ''
}

function followerReferenceLabel(field) {
  const savedKey = getSavedReferenceKey(field)
  if (savedKey) {
    const parsed = parseReferenceSourceKey(savedKey)
    if (parsed.sourceField) return `引用：${parsed.sourceField}`
  }
  if (field.reference?.sourceField) return `引用：${field.reference.sourceField}`
  // fillAllPositions follower: reference is the primary position (same field name)
  if (field.fillAllPositions && (field.posIndex ?? 0) > 0) return `引用：${field.name}`
  return '引用'
}

function selectFieldOptions(field) {
  return (field.options || [])
    .map((opt) => ({
      label: opt.label || '',
      value: opt.checkedText || opt.label || '',
    }))
    .filter((opt) => opt.label)
}

// ── Form value accessors (emit events instead of mutating props) ────────────

function getFormValueByPrimary(field) {
  // For duplicate fields, get the value from the first field with the same name
  const primary = props.renderableTemplateFields.find(
    (f) => f.name === field.name && !f._isDuplicate
  )
  if (primary) return props.formValues[fieldFormKey(primary)]
  return props.formValues[fieldFormKey(field)]
}

function getReferenceSelection(field) {
  const slotKey = slotKeyFor(field)
  return props.referenceSelections[slotKey] || props.referenceSelections[fieldFormKey(field)]
}

// Determine the reference source for a field: checks slot-level saved reference,
// field-level reference selection, and template-level reference config.
function resolveReferenceSource(field) {
  const slotKey = slotKeyFor(field)
  const slotRef = props.referenceSelections[slotKey]
  if (slotRef) {
    const parsed = parseReferenceSourceKey(slotRef)
    if (parsed.mode !== 'auto' && (parsed.sourceField || parsed.sourceSemanticKey)) {
      return { fixed: true, ...parsed }
    }
  }
  const fieldRef = props.referenceSelections[fieldFormKey(field)]
  if (fieldRef) {
    const parsed = parseReferenceSourceKey(fieldRef)
    if (parsed.mode !== 'auto' && (parsed.sourceField || parsed.sourceSemanticKey)) {
      return { fixed: true, ...parsed }
    }
  }
  if (field.reference) {
    const mode = field.reference.sourceMode || 'auto'
    if (mode !== 'auto' && (field.reference.sourceField || field.reference.sourceSemanticKey)) {
      return {
        fixed: true,
        mode,
        sourceField: field.reference.sourceField || '',
        sourceSemanticKey: field.reference.sourceSemanticKey || '',
        sourceIndex: field.reference.sourceIndex ?? null,
      }
    }
  }
  return { fixed: false, mode: 'auto', sourceField: '', sourceSemanticKey: '', sourceIndex: null }
}

// Single candidate: follower following first position, or fixed source reference.
function isReferenceSingleCandidate(field) {
  if ((field.posIndex ?? 0) > 0 && field.fillAllPositions && !hasSlotTypeOverride(field)) {
    return true
  }
  return resolveReferenceSource(field).fixed
}

// Display label for single-candidate reference: "引用：X 第 N 项" etc.
function referenceFixedDisplayLabel(field) {
  if ((field.posIndex ?? 0) > 0 && field.fillAllPositions && !hasSlotTypeOverride(field)) {
    return followerReferenceLabel(field)
  }
  const source = resolveReferenceSource(field)
  if (source.fixed) {
    if (source.sourceField) {
      return source.sourceIndex != null
        ? `引用：${source.sourceField} 第 ${source.sourceIndex + 1} 项`
        : `引用：${source.sourceField}`
    }
    if (source.sourceSemanticKey) return `引用：${source.sourceSemanticKey}`
  }
  return '引用'
}

function getPartyListRows(field) {
  const key = fieldFormKey(field)
  const current = props.formValues[key]
  if (!Array.isArray(current)) {
    return []
  }
  if (!current.length) {
    return [{ text: '', suffix: '' }]
  }
  return current
}

function getStructureOverride(field) {
  const key = field?.id || field?.name || ''
  if (!key) return { prefix: '', suffix: '' }
  if (!props.structureOverrides[key]) {
    return { prefix: '', suffix: '' }
  }
  return props.structureOverrides[key]
}

function setStructureOverride(field, prop, value) {
  const key = field?.id || field?.name || ''
  if (!key) return
  emit('update-structure-override', key, prop, value)
}

// ── Reference fill options ──────────────────────────────────────────────────

function referenceFillOptions(field) {
  const options = []
  const seen = new Set()
  for (const item of props.renderableTemplateFields) {
    if (item.name === field.name || item.type === 'reference') continue
    if (item.type === 'party_list') {
      const values = partyItemsToValues(props.formValues[fieldFormKey(item)] || [])
      if (values.length) {
        const allKey = `field::${item.name}::`
        options.push({ key: allKey, label: `${fillFieldLabel(item)}：全部` })
      }
      values.forEach((value, index) => {
        const key = `field::${item.name}::${index}`
        if (seen.has(key)) return
        seen.add(key)
        options.push({ key, label: `${fillFieldLabel(item)}第 ${index + 1} 项：${value}` })
      })
    } else {
      const value = props.formValues[fieldFormKey(item)]
      const key = `field::${item.name}::`
      if (seen.has(key)) continue
      seen.add(key)
      const label = isEmptyValue(value) ? fillFieldLabel(item) : `${fillFieldLabel(item)}：${displayValue(value)}`
      options.push({ key, label })
    }
  }
  return options
}

// partyItemsToValues is now imported from fieldRowUtils.js

// ── Suggestions ─────────────────────────────────────────────────────────────

function templateStoredSuggestionItems(field) {
  return (props.historyContext.fieldSuggestions?.[field.id] || []).slice(0, 6)
}

// ── Structure hints ─────────────────────────────────────────────────────────

function fieldStructureHints(field) {
  const override = existingStructureOverrideForField(field)
  const prefixes = override ? [override.prefix ?? ''] : []
  const suffixes = fieldUsesRepeatableSuffix(field)
    ? []
    : override
      ? [override.suffix ?? '']
      : []
  return [
    ...prefixes.filter(Boolean).map((text, index) => ({
      key: `prefix-${index}-${text}`,
      label: '前缀',
      text: displayStructureText(text),
      empty: !cleanStructureText(text),
    })),
    ...suffixes.filter(Boolean).map((text, index) => ({
      key: `suffix-${index}-${text}`,
      label: '后缀',
      text: displayStructureText(text),
      empty: !cleanStructureText(text),
    })),
  ]
}

function existingStructureOverrideForField(field) {
  const key = field?.id || field?.name || ''
  return key ? props.structureOverrides[key] : null
}

function displayStructureText(text) {
  const value = cleanStructureText(text)
  return value || '无'
}

function cleanStructureText(text) {
  const value = String(text || '').trim()
  if (!value) return ''
  const withoutConnector = value.replace(/^(?:以及|或者|[，,、;；和与及\s])+/u, '')
  if (!withoutConnector) return ''
  return withoutConnector
}

function structureEditorTitle(field) {
  return fillFieldLabel(field)
}
</script>

<style scoped>
.filename-preview {
  font-size: 12px;
  color: var(--docsy-text-muted, #909399);
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  vertical-align: middle;
  margin-left: 4px;
}
.date-fill-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.date-blank-text {
  color: var(--docsy-text-muted, #909399);
  font-size: 12px;
}

.date-blank-btn {
  border: none;
  background: transparent;
  color: var(--docsy-text-muted, #909399);
  font-size: 12px;
  cursor: pointer;
  padding: 2px 4px;
}

.date-blank-btn:hover {
  color: var(--docsy-accent, #409eff);
}

.workspace {
  display: grid;
  gap: 14px;
  padding-top: 8px;
}

.panel {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  padding: 14px;
  background: var(--docsy-surface-elevated);
  box-shadow: 0 3px 14px rgba(54, 45, 36, 0.035);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  margin-bottom: 12px;
}

.panel-header.compact {
  align-items: center;
}

h3 {
  margin: 0 0 4px;
  font-size: 16px;
  color: var(--docsy-text-strong);
}

p {
  margin: 0;
  color: var(--docsy-text-muted);
  font-size: 13px;
}

.actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.actions.inline {
  margin-top: 0;
}

.template-library-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  gap: 10px;
}

.template-library-card {
  display: grid;
  gap: 5px;
  padding: 12px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text-strong);
  text-align: left;
  cursor: pointer;
}

.template-library-card:hover,
.template-library-card.active {
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}

.template-library-card span,
.template-library-card small {
  color: var(--docsy-text-muted);
}

.form-panel {
  min-height: 0;
}

.template-form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 12px;
}

.fill-field-card {
  position: relative;
  display: grid;
  gap: 8px;
  min-width: 0;
  padding: 12px 34px 12px 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-left: 3px solid transparent;
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
  overflow-wrap: break-word;
  word-break: break-word;
}

.fill-field-body {
  display: contents;
}

.field-more-button {
  position: absolute;
  right: 8px;
  bottom: 8px;
  width: 22px;
  height: 22px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: 50%;
  background: var(--docsy-surface);
  color: var(--docsy-text);
  cursor: pointer;
  line-height: 18px;
}

.field-more-button:hover {
  border-color: var(--docsy-primary);
  color: var(--docsy-primary);
}

.fill-structure-editor {
  display: grid;
  gap: 8px;
}

.fill-field-header {
  display: flex;
  gap: 4px;
  align-items: center;
  overflow: hidden;
}

.fill-field-header strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
  flex-shrink: 1;
}

.fill-field-header .fill-all-tag {
  flex-shrink: 0;
  white-space: nowrap;
}

.fill-field-header span,
.fill-field-header em {
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  font-size: 12px;
  font-style: normal;
}

.fill-field-header em {
  background: #fef0f0;
  color: #f56c6c;
}



.fill-structure-hints {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.fill-structure-hint {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.fill-structure-hint code {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
  font-family: inherit;
}

.fill-structure-hint code.empty {
  color: #c0c4cc;
}

.fill-structure-hint em {
  margin-left: 4px;
  color: #c0c4cc;
  font-style: normal;
}

.party-list-editor {
  display: grid;
  gap: 8px;
  width: 100%;
}

.reference-fill-editor {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.reference-fixed-value {
  padding: 5px 11px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  font-size: 13px;
  line-height: 1.5;
}

.reference-select-muted :deep(.el-input__wrapper) {
  background-color: var(--docsy-surface-muted);
}

.party-list-row {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) minmax(90px, 130px) minmax(106px, auto);
  gap: 6px;
  align-items: center;
}

.party-list-row.compact {
  grid-template-columns: 24px minmax(0, 1fr) minmax(72px, 90px) minmax(96px, auto);
}

.party-list-row.compact.no-suffix {
  grid-template-columns: 24px minmax(0, 1fr) minmax(96px, auto);
}

.party-list-row.compact .el-button {
  padding-left: 4px;
  padding-right: 4px;
}

.party-row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 4px;
  justify-content: flex-end;
  min-width: 0;
}

.party-row-actions .el-button + .el-button {
  margin-left: 0;
}

.party-list-add-row {
  display: flex;
  justify-content: flex-start;
}

.field-structure-hint {
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.party-order {
  color: var(--docsy-text-muted);
  text-align: center;
}

.party-suffix {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 0 8px;
  border-radius: 4px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
  font-size: 12px;
}

.suggestion-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
  max-height: 80px;
  overflow-y: auto;
}

.suggestion-tag {
  cursor: pointer;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Duplicate field styling */
.duplicate-field {
  opacity: 0.7;
  border-style: dashed;
}

.duplicate-field .el-textarea.is-disabled .el-textarea__inner {
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  cursor: not-allowed;
}

/* Fill preview */
.fill-preview-panel {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--docsy-border-subtle);
}

.fill-preview-header {
  margin-bottom: 8px;
}

.fill-preview-header h3 {
  margin: 0 0 4px;
  font-size: 14px;
  color: var(--docsy-text-strong);
}

.fill-preview-header p {
  margin: 0;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.fill-preview-text {
  max-height: 60vh;
  overflow-y: auto;
  margin: 0;
  padding: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-strong);
  font-family: inherit;
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
}

@media (max-width: 1180px) {
  .template-form-grid {
    grid-template-columns: 1fr;
  }

  .panel-header,
  .panel-header.compact {
    align-items: flex-start;
    flex-direction: column;
  }
}

@media (max-width: 760px) {
  .party-list-row,
  .party-list-row.compact,
  .party-list-row.compact.no-suffix {
    grid-template-columns: 24px minmax(0, 1fr);
  }

  .party-list-row > :not(.party-order):not(.el-autocomplete):not(.el-input) {
    grid-column: 2;
  }

  .party-row-actions {
    justify-content: flex-start;
  }
}
</style>
