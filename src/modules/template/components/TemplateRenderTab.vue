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
          <el-button @click="$emit('select-template-package')">选择外部模板</el-button>
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
          <el-button size="small" text @click="$emit('collapse-all-fields')">折叠全部</el-button>
          <el-button size="small" text @click="$emit('expand-all-fields')">展开全部</el-button>
        </div>
        <el-button type="success" :loading="rendering" @click="$emit('render-template')">生成 Word</el-button>
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
        <section v-for="field in filteredRenderableFields" :key="field.id" class="fill-field-card">
          <div class="fill-field-header" @click="$emit('toggle-field-collapse', field)">
            <strong>{{ fillFieldLabel(field) }}</strong>
            <span v-if="field.semanticKey && field.semanticKey !== field.name">{{ field.semanticKey }}</span>
            <em v-if="field.required">必填</em>
            <el-tag v-if="field.fillAllPositions" size="small" effect="plain" class="fill-all-tag">
              填一次将自动填充到所有位置
            </el-tag>
            <span class="field-collapse-toggle">{{ collapsedFields.has(field.id) ? '▸' : '▾' }}</span>
          </div>
          <div v-show="!collapsedFields.has(field.id)" class="fill-field-body">
            <div v-if="fieldStructureHints(field).length" class="fill-structure-hints">
              <span v-for="hint in fieldStructureHints(field)" :key="hint.key" class="fill-structure-hint">
                {{ hint.label }}：<code :class="{ empty: hint.empty }">{{ hint.text }}</code>
                <em>空值时删除</em>
              </span>
            </div>
            <el-date-picker
              v-if="effectiveFieldType(field) === 'date'"
              :model-value="getFormValue(field)"
              type="date"
              value-format="YYYY-MM-DD"
              @update:model-value="setFormValue(field, $event)"
            />
            <el-checkbox
              v-else-if="effectiveFieldType(field) === 'checkbox'"
              :model-value="getFormValue(field)"
              @update:model-value="setFormValue(field, $event)"
            >
              {{ firstOptionLabel(field) || '选中' }}
            </el-checkbox>
            <el-radio-group
              v-else-if="effectiveFieldType(field) === 'radio_group'"
              :model-value="getFormValue(field)"
              @update:model-value="setFormValue(field, $event)"
            >
              <el-radio v-for="option in field.options" :key="option.id" :label="option.id">
                {{ option.label }}
              </el-radio>
            </el-radio-group>
            <el-checkbox-group
              v-else-if="effectiveFieldType(field) === 'checkbox_group'"
              :model-value="getFormValue(field)"
              @update:model-value="setFormValue(field, $event)"
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
              <el-select
                :model-value="getReferenceSelection(field)"
                filterable
                clearable
                placeholder="从已填字段取值"
                @update:model-value="$emit('reference-selection-change', field, $event)"
              >
                <el-option
                  v-for="item in referenceFillOptions(field)"
                  :key="item.key"
                  :label="item.label"
                  :value="item.key"
                />
              </el-select>
              <el-input
                :model-value="getFormValue(field)"
                type="textarea"
                :autosize="{ minRows: 1, maxRows: 6 }"
                resize="none"
                clearable
                placeholder="引用文本，可单独修改"
                @input="setFormValue(field, $event)"
              />
            </div>
            <el-select
              v-else-if="effectiveFieldType(field) === 'select'"
              :model-value="getFormValue(field)"
              filterable
              allow-create
              default-first-option
              clearable
              placeholder="选择或输入"
              @update:model-value="setFormValue(field, $event)"
            >
              <el-option
                v-for="opt in selectFieldOptions(field)"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
            <el-input
              v-else
              :model-value="getFormValue(field)"
              type="textarea"
              :autosize="{ minRows: 1, maxRows: 6 }"
              resize="none"
              clearable
              @input="setFormValue(field, $event)"
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
            <el-popover placement="bottom-end" trigger="click" width="280">
              <template #reference>
                <button class="field-more-button" type="button">…</button>
              </template>
              <div class="fill-structure-editor">
                <strong>{{ structureEditorTitle(field) }}</strong>
                <div class="setting-row">
                  <span class="setting-label">字段类型</span>
                  <el-select
                    :model-value="effectiveFieldType(field)"
                    size="small"
                    @change="(type) => $emit('set-field-type-override', field, type)"
                  >
                    <el-option
                      v-for="item in typeHelpItems.filter((t) => ['text', 'date', 'select'].includes(t.value))"
                      :key="item.value"
                      :value="item.value"
                      :label="item.label"
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
              </div>
            </el-popover>
          </div>
        </section>
      </div>
    </div>
  </section>
</template>

<script setup>
import { ArrowDown } from '@element-plus/icons-vue'
import {
  shortDateTime,
  fillFieldLabel,
  typeHelpItems,
  partyFieldUsesSuffix,
  partySuffixOptions,
  partyFieldStructureHint,
  fieldUsesRepeatableSuffix,
  isEmptyValue,
  displayValue,
} from '../composables/fieldRowUtils.js'

const props = defineProps({
  // Template state
  templatePath: { type: String, default: '' },
  templateManifest: { type: Object, default: null },
  templateLibrary: { type: Array, default: () => [] },
  templateLibraryLoading: { type: Boolean, default: false },
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
  collapsedFields: { type: Object, default: () => new Set() },
  // Computed from parent
  renderableTemplateFields: { type: Array, default: () => [] },
  filteredRenderableFields: { type: Array, default: () => [] },
})

const emit = defineEmits([
  'load-template-library',
  'select-template-package',
  'open-template-from-library',
  'delete-template',
  'update:fieldSearch',
  'collapse-all-fields',
  'expand-all-fields',
  'render-template',
  'batch-command',
  'toggle-field-collapse',
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
])

// ── Helpers ──────────────────────────────────────────────────────────────────

function fieldFormKey(field) {
  if (!field) return ''
  return field.id || field.name
}

function effectiveFieldType(field) {
  return props.typeOverrides[field.id] || field.type
}

function firstOptionLabel(field) {
  return field.options?.[0]?.label || ''
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

function getFormValue(field) {
  return props.formValues[fieldFormKey(field)]
}

function setFormValue(field, value) {
  emit('update-form-value', fieldFormKey(field), value)
}

function getReferenceSelection(field) {
  return props.referenceSelections[fieldFormKey(field)]
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
      if (isEmptyValue(value)) continue
      const key = `field::${item.name}::`
      if (seen.has(key)) continue
      seen.add(key)
      options.push({ key, label: `${fillFieldLabel(item)}：${displayValue(value)}` })
    }
  }
  return options
}

function partyItemsToValues(value) {
  if (typeof value === 'string') return value.split(/[、\n]/).map((item) => item.trim()).filter(Boolean)
  if (!Array.isArray(value)) return []
  return value
    .map((item) => {
      if (typeof item === 'string') return item.trim()
      const suffix = String(item?.suffix || '').trim()
      let text = String(item?.text || '').trim()
      if (suffix && text.endsWith(suffix)) {
        text = text.slice(0, -suffix.length).trim()
      }
      return suffix ? { name: text, suffix } : text
    })
    .filter((item) => (typeof item === 'string' ? Boolean(item) : Boolean(item.name)))
}

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
