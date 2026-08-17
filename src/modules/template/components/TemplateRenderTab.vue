<template>
  <section class="workspace">
    <div class="panel library-panel" :class="{ 'has-active-template': templateManifest }">
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
      <el-empty v-else description="模板库为空，请先从“制作模板”保存模板" :image-size="60" />
    </div>

    <div v-if="templateManifest" class="panel form-panel">
      <div class="fill-command-bar">
        <div class="active-template-heading">
          <span class="active-template-label">正在填写</span>
          <h3>{{ templateManifest.template.name }}</h3>
          <p>
            已填写 {{ filledFieldCount }} / {{ editableFieldCount }}
            <span v-if="requiredFieldCount"> · 必填 {{ filledRequiredCount }} / {{ requiredFieldCount }}</span>
          </p>
        </div>
        <div class="fill-command-actions">
          <el-input
            :model-value="fieldSearch"
            size="small"
            clearable
            placeholder="搜索字段"
            class="field-search-input"
            @update:model-value="$emit('update:fieldSearch', $event)"
          />
          <el-button :disabled="!templateManifest" @click="$emit('toggle-fill-preview')">
            {{ fillPreviewVisible ? '收起预览' : '文档预览' }}
          </el-button>
          <el-dropdown trigger="click" @command="$emit('batch-command', $event)">
            <el-button :loading="batchProcessing">
              批量填写 <el-icon class="el-icon--right"><arrow-down /></el-icon>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="export">导出字段表</el-dropdown-item>
                <el-dropdown-item command="import">导入并生成</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <el-button type="success" :loading="rendering" @click="$emit('render-template')">生成 Word</el-button>
        </div>
      </div>
      <div class="fill-progress-track" aria-hidden="true">
        <span :style="{ width: `${fillProgressPercent}%` }"></span>
      </div>
      <div v-if="filenamePreview" class="filename-preview" :title="filenamePreview">
        <span>输出文件名</span>
        <strong>{{ filenamePreview }}</strong>
      </div>

      <div class="fill-workbench" :class="{ 'with-preview': fillPreviewVisible }">
        <div class="field-editor-column">
          <div class="field-section-heading">
            <div>
              <h3>填写字段</h3>
              <p>内容会自动保存到本次填写状态；同名字段会同步更新。</p>
            </div>
            <span>{{ fillPositionEntries.length }} 项</span>
          </div>
          <div class="template-form-grid">
            <section
              v-for="field in fillPositionEntries"
              :key="`${field.id}#${field.posIndex ?? 0}`"
              class="fill-field-card"
              :class="{
                'duplicate-field': field._isDuplicate,
                'follow-field': field.posIndex > 0 || (!field.editable && !field.isReference),
                'is-filled': !isEmptyValue(getEntryValue(field)),
              }"
            >
              <div class="fill-field-header">
                <el-tooltip
                  :content="fillFieldLabel(field)"
                  placement="top"
                  :show-after="500"
                  :disabled="fillFieldLabel(field).length < 12"
                >
                  <strong class="field-name">{{ fillFieldLabel(field) }}</strong>
                </el-tooltip>
                <span v-if="field.posIndex > 0" class="position-label fill-all-tag">位置{{ field.posIndex + 1 }}</span>

                <em v-if="field.required" class="fill-all-tag">必填</em>
                <el-tag v-if="field.isDuplicate" size="small" effect="plain" type="info" class="fill-all-tag">
                  同名字段，自动同步
                </el-tag>
                <el-tag
                  v-else-if="
                    field.fillAllPositions &&
                    field.posIndex > 0 &&
                    !hasSlotTypeOverride(field) &&
                    effectiveFieldType(field) === 'reference'
                  "
                  size="small"
                  effect="plain"
                  class="fill-all-tag"
                >
                  {{ followerReferenceLabel(field) }}
                </el-tag>
              </div>
              <div class="fill-field-body">
                <div v-if="hasOverallStructure(field)" class="fill-structure-inline">
                  <el-input
                    v-if="hasOverallPrefix(field)"
                    :model-value="getStructureOverride(field).prefix"
                    size="small"
                    placeholder="前缀"
                    @input="(val) => setStructureOverride(field, 'prefix', val)"
                  >
                    <template #prepend>前缀</template>
                  </el-input>
                  <el-input
                    v-if="hasOverallSuffix(field)"
                    :model-value="getStructureOverride(field).suffix"
                    size="small"
                    placeholder="后缀"
                    @input="(val) => setStructureOverride(field, 'suffix', val)"
                  >
                    <template #prepend>后缀</template>
                  </el-input>
                </div>
                <template
                  v-if="
                    field.editable ||
                    field.isReference ||
                    hasSlotTypeOverride(field) ||
                    effectiveFieldType(field) === 'reference'
                  "
                >
                  <div v-if="!fieldIsMultiple(field) && effectiveFieldType(field) === 'date'" class="date-fill-row">
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
                  <div v-else-if="fieldIsMultiple(field)" class="party-list-editor">
                    <div
                      v-for="(item, index) in getPartyListRows(field)"
                      :key="index"
                      class="party-list-row compact"
                      :class="{
                        'has-prefix': multiShowsPrefix(field),
                        'has-suffix': multiShowsSuffix(field),
                        'no-affix': !multiShowsPrefix(field) && !multiShowsSuffix(field),
                      }"
                    >
                      <span class="party-order">{{ index + 1 }}</span>
                      <el-input
                        v-if="multiShowsPrefix(field)"
                        v-model="item.prefix"
                        size="small"
                        placeholder="前缀"
                        @input="() => commitPartyTempRow(field)"
                      />
                      <el-select
                        v-if="effectiveFieldType(field) === 'select'"
                        v-model="item.text"
                        size="small"
                        filterable
                        allow-create
                        default-first-option
                        placeholder="选择或输入"
                        @change="() => commitPartyTempRow(field)"
                      >
                        <el-option
                          v-for="opt in selectFieldOptions(field)"
                          :key="opt.value"
                          :label="opt.label"
                          :value="opt.value"
                        />
                      </el-select>
                      <el-date-picker
                        v-else-if="effectiveFieldType(field) === 'date'"
                        v-model="item.text"
                        size="small"
                        type="date"
                        value-format="YYYY-MM-DD"
                        @change="() => commitPartyTempRow(field)"
                      />
                      <el-autocomplete
                        v-else
                        v-model="item.text"
                        size="small"
                        placeholder="名称"
                        :fetch-suggestions="(query, cb) => $emit('complete-field', field, query, cb)"
                        @input="
                          () => {
                            commitPartyTempRow(field)
                            $emit('schedule-history-refresh')
                          }
                        "
                      />
                      <el-select
                        v-if="multiShowsSuffix(field)"
                        v-model="item.suffix"
                        size="small"
                        filterable
                        allow-create
                        default-first-option
                        placeholder="后缀"
                        @change="
                          () => {
                            commitPartyTempRow(field)
                            $emit('schedule-history-refresh')
                          }
                        "
                      >
                        <el-option
                          v-for="suffix in partySuffixOptions(field)"
                          :key="suffix"
                          :label="suffix"
                          :value="suffix"
                        />
                      </el-select>
                      <div class="party-row-actions">
                        <el-popover placement="bottom-end" trigger="click" width="260">
                          <template #reference>
                            <el-button size="small" text>…</el-button>
                          </template>
                          <div class="item-structure-editor">
                            <strong>第 {{ index + 1 }} 项结构</strong>
                            <el-input
                              v-model="item.prefix"
                              size="small"
                              placeholder="无前缀"
                              @input="() => commitPartyTempRow(field)"
                            >
                              <template #prepend>前缀</template>
                            </el-input>
                            <el-input
                              v-model="item.suffix"
                              size="small"
                              placeholder="无后缀"
                              @input="() => commitPartyTempRow(field)"
                            >
                              <template #prepend>后缀</template>
                            </el-input>
                            <p class="setting-caption">这里只修改当前项目，不会影响其他项目或被引用字段。</p>
                          </div>
                        </el-popover>
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
                      <span>连接符：{{ getStructureOverride(field).itemSeparator || '、' }}</span>
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
                    <p
                      v-if="effectiveFieldType(field) === 'reference' && !hasSlotTypeOverride(field)"
                      class="setting-caption"
                    >
                      引用字段从已填字段取值，不能手动输入；前后缀在"…"菜单里设置。
                    </p>
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
                    @input="
                      (val) => {
                        setEntryValue(field, val)
                        $emit('schedule-history-refresh')
                      }
                    "
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
                    <template v-if="fieldIsMultiple(field)">
                      <el-input
                        :model-value="getStructureOverride(field).itemSeparator"
                        size="small"
                        placeholder="、"
                        @input="(val) => setStructureOverride(field, 'itemSeparator', val)"
                      >
                        <template #prepend>连接符</template>
                      </el-input>
                      <p class="setting-caption">逐项前后缀请在每一项右侧的“…”中单独设置。</p>
                    </template>
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
                        <el-option
                          v-for="item in dateFormatOptions"
                          :key="item.value"
                          :label="item.label"
                          :value="item.value"
                        />
                      </el-select>
                    </div>
                  </div>
                </el-popover>
              </div>
            </section>
          </div>
        </div>

        <aside v-if="fillPreviewVisible" class="fill-preview-panel">
          <div class="fill-preview-header">
            <div>
              <span>实时预览</span>
              <h3>文档内容</h3>
            </div>
            <p :class="{ 'preview-error-text': fillPreviewError }">
              {{ fillPreviewLoading ? '正在同步最终 Word 效果' : fillPreviewError || '与最终 Word 使用相同的渲染规则' }}
            </p>
          </div>
          <DocumentPreview
            v-if="fillDocumentRuns.length"
            :runs="fillDocumentRuns"
            :overlays="fillPreviewOverlays"
            mode="fill"
          />
          <pre v-else-if="fillPreviewText" class="fill-preview-text">{{ fillPreviewText }}</pre>
          <div v-else class="fill-preview-status">
            <span v-if="fillPreviewLoading" class="preview-loading-dot" aria-hidden="true"></span>
            <strong>{{ fillPreviewLoading ? '正在加载文档预览' : '暂时无法显示预览' }}</strong>
            <p v-if="!fillPreviewLoading">{{ fillPreviewError || '请重新加载模板正文' }}</p>
            <el-button v-if="!fillPreviewLoading" size="small" @click="$emit('reload-fill-preview')"
              >重新加载</el-button
            >
          </div>
        </aside>
        <button v-else type="button" class="preview-invitation" @click="$emit('toggle-fill-preview')">
          <span class="preview-invitation-icon">文</span>
          <strong>打开文档预览</strong>
          <small>在填写时同步核对字段所在位置</small>
        </button>
      </div>
    </div>
  </section>
</template>

<script setup>
import { computed } from 'vue'
import { ArrowDown } from '@element-plus/icons-vue'
import DocumentPreview from '@/shared/components/DocumentPreview.vue'
import {
  shortDateTime,
  fillFieldLabel,
  FIELD_TYPE_GROUPS,
  typeGroupOf,
  typeGroupSubOptions,
  typeActualOf,
  partySuffixOptions,
  partyFieldStructureHint,
  fieldIsMultiple,
  isEmptyValue,
  displayValue,
  parseReferenceSourceKey,
  partyItemsToValues,
  fieldFormKey,
  fieldSlotKey,
  referenceSelectionFor,
  createPartyTempRowStore,
  filenamePreviewText,
  optionalRulePrefix,
  optionalRuleSuffix,
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
  fillPreviewLoading: { type: Boolean, default: false },
  fillPreviewError: { type: String, default: '' },
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
  'reload-fill-preview',
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
  return fieldSlotKey(field)
}

// A follower made independent by a type change or a custom reference source
// stores its own slot value.
function hasSlotTypeOverride(field) {
  return Boolean(props.typeOverrides[slotKeyFor(field)])
}

// Filename preview: 按当前命名规则实时预览（field token 按字段名解析填写值）
const filenamePreview = computed(() =>
  filenamePreviewText(props.filenameTokens, {
    formValues: props.formValues,
    fields: props.renderableTemplateFields,
    manifestName: props.templateManifest?.template?.name || props.templateManifest?.name || '',
  }),
)

const editableEntries = computed(() =>
  props.fillPositionEntries.filter((field) => field.editable && !field.isDuplicate),
)
const requiredEntries = computed(() => editableEntries.value.filter((field) => field.required))
const editableFieldCount = computed(() => editableEntries.value.length)
const requiredFieldCount = computed(() => requiredEntries.value.length)
const filledFieldCount = computed(
  () => editableEntries.value.filter((field) => !isEmptyValue(getEntryValue(field))).length,
)
const filledRequiredCount = computed(
  () => requiredEntries.value.filter((field) => !isEmptyValue(getEntryValue(field))).length,
)
const fillProgressPercent = computed(() =>
  editableFieldCount.value ? Math.round((filledFieldCount.value / editableFieldCount.value) * 100) : 0,
)

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
  return referenceSelectionFor(props.referenceSelections, field)
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
  const primary = props.renderableTemplateFields.find((f) => f.name === field.name && !f._isDuplicate)
  if (primary) return props.formValues[fieldFormKey(primary)]
  return props.formValues[fieldFormKey(field)]
}

function getReferenceSelection(field) {
  return referenceSelectionFor(props.referenceSelections, field)
}

// Determine the reference source for a field: checks slot-level saved reference,
// field-level reference selection, and template-level reference config.
function resolveReferenceSource(field) {
  // referenceSelections 的 key 有 id 与 id#0 两种历史写法，统一走
  // referenceSelectionFor（slot 级优先于字段级）。
  const ref = referenceSelectionFor(props.referenceSelections, field)
  if (ref) {
    const parsed = parseReferenceSourceKey(ref)
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

// party_list 空数组时的临时编辑行：缓存住并在首次输入时提交为正式值，
// 否则重渲染或「添加一项」会把正在输入的临时行冲掉。
const partyTempRows = createPartyTempRowStore((key, rows) => emit('update-form-value', key, rows))

function getPartyListRows(field) {
  return partyTempRows.rowsFor(fieldFormKey(field), props.formValues[fieldFormKey(field)])
}

function commitPartyTempRow(field) {
  partyTempRows.commit(fieldFormKey(field), props.formValues[fieldFormKey(field)])
}

function getStructureOverride(field) {
  const key = field?.id || field?.name || ''
  const defaults = {
    prefix: defaultOverallPrefix(field),
    suffix: defaultOverallSuffix(field),
    itemSeparator: field?.itemSeparator || '、',
    repeatPrefix: Boolean(field?.repeatPrefix),
    repeatSuffix:
      Boolean(field?.repeatSuffix) ||
      (field?.markRefs || []).some((ref) => Boolean(optionalRuleSuffix(ref.optionalRule))),
  }
  if (!key) return defaults
  if (!props.structureOverrides[key]) {
    return defaults
  }
  return { ...defaults, ...props.structureOverrides[key] }
}

function multiUsesPrefix(field) {
  return fieldIsMultiple(field) && Boolean(getStructureOverride(field).repeatPrefix)
}

function multiUsesSuffix(field) {
  return fieldIsMultiple(field) && Boolean(getStructureOverride(field).repeatSuffix)
}

function multiShowsPrefix(field) {
  return multiUsesPrefix(field) || getPartyListRows(field).some((item) => Boolean(item.prefix))
}

function multiShowsSuffix(field) {
  return multiUsesSuffix(field) || getPartyListRows(field).some((item) => Boolean(item.suffix))
}

function structureRules(field) {
  return (field?.markRefs || []).map((ref) => ref.optionalRule).filter((rule) => rule?.enabled)
}

function defaultOverallPrefix(field) {
  if (fieldIsMultiple(field) && field.repeatPrefix) return ''
  return structureRules(field).map(optionalRulePrefix).find(Boolean) || optionalRulePrefix(field?.optionalRule)
}

function defaultOverallSuffix(field) {
  const values = structureRules(field).map(optionalRuleSuffix).filter(Boolean)
  if (fieldIsMultiple(field) && (field.repeatSuffix || values.length)) return ''
  return values.at(-1) || optionalRuleSuffix(field?.optionalRule)
}

function hasOverallPrefix(field) {
  const key = field?.id || field?.name || ''
  return Boolean(
    defaultOverallPrefix(field) || Object.prototype.hasOwnProperty.call(props.structureOverrides[key] || {}, 'prefix'),
  )
}

function hasOverallSuffix(field) {
  const key = field?.id || field?.name || ''
  return Boolean(
    defaultOverallSuffix(field) || Object.prototype.hasOwnProperty.call(props.structureOverrides[key] || {}, 'suffix'),
  )
}

function hasOverallStructure(field) {
  return hasOverallPrefix(field) || hasOverallSuffix(field)
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
  const candidates = props.renderableTemplateFields.filter(
    (item) => item.name !== field.name && item.type !== 'reference',
  )
  const groupName = String(field.groupName || '').trim()
  const groupedCandidates = groupName
    ? candidates.filter((item) => String(item.groupName || '').trim() === groupName)
    : []
  for (const item of groupedCandidates.length ? groupedCandidates : candidates) {
    if (fieldIsMultiple(item)) {
      const values = partyItemsToValues(props.formValues[fieldFormKey(item)] || [])
      if (values.length) {
        const allKey = `field::${item.name}::`
        options.push({ key: allKey, label: `${fillFieldLabel(item)}：全部` })
      }
      values.forEach((value, index) => {
        const key = `field::${item.name}::${index}`
        if (seen.has(key)) return
        seen.add(key)
        const group = item.groupName ? `${item.groupName} / ` : ''
        options.push({ key, label: `${group}${fillFieldLabel(item)}第 ${index + 1} 项：${displayValue(value)}` })
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

function structureEditorTitle(field) {
  return fillFieldLabel(field)
}
</script>

<style scoped>
.filename-preview {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  padding: 9px 18px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted, #909399);
  font-size: 12px;
}

.filename-preview strong {
  min-width: 0;
  overflow: hidden;
  color: var(--docsy-text);
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  gap: 16px;
  padding-top: 10px;
}

.panel {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
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

.library-panel.has-active-template {
  padding: 16px;
}

.library-panel.has-active-template .panel-header {
  align-items: center;
  margin-bottom: 14px;
}

.library-panel.has-active-template .template-library-grid {
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  max-height: 300px;
  overflow-y: auto;
  padding: 1px 4px 4px 1px;
}

.library-panel.has-active-template .template-library-card {
  min-width: 0;
  padding: 13px 14px;
}

.library-panel.has-active-template .template-card-actions {
  margin-top: 4px;
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
  grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 10px;
}

.template-library-card {
  display: grid;
  gap: 5px;
  padding: 12px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text-strong);
  text-align: left;
  cursor: pointer;
  transition:
    border-color 160ms ease,
    background-color 160ms ease,
    transform 160ms ease;
}

.template-library-card:hover,
.template-library-card.active {
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}

.template-library-card:hover {
  transform: translateY(-1px);
}

.template-card-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 2px;
}

.template-library-card span,
.template-library-card small {
  color: var(--docsy-text-muted);
}

.form-panel {
  min-height: 0;
  padding: 0;
  overflow: clip;
}

.fill-command-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 16px 18px 14px;
  background: color-mix(in srgb, var(--docsy-surface-elevated) 94%, transparent);
}

.active-template-heading {
  display: grid;
  min-width: 210px;
}

.active-template-heading h3 {
  margin-top: 2px;
  font-size: 18px;
  line-height: 1.3;
}

.active-template-heading p {
  margin-top: 2px;
  font-size: 12px;
}

.active-template-label {
  color: var(--docsy-primary);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.12em;
}

.fill-command-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.field-search-input {
  width: 170px;
}

.fill-progress-track {
  height: 3px;
  overflow: hidden;
  background: var(--docsy-surface-muted);
}

.fill-progress-track span {
  display: block;
  height: 100%;
  border-radius: 0 var(--docsy-radius) var(--docsy-radius) 0;
  background: var(--docsy-primary);
  transition: width 220ms ease;
}

.fill-workbench {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 0.32fr);
  min-height: 420px;
}

.fill-workbench.with-preview {
  grid-template-columns: minmax(0, 1fr) minmax(320px, 0.68fr);
}

.field-editor-column {
  min-width: 0;
  padding: 18px;
}

.field-section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 12px;
}

.field-section-heading h3 {
  font-size: 15px;
}

.field-section-heading > span {
  flex: 0 0 auto;
  padding: 3px 9px;
  border-radius: 999px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.template-form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 12px;
}

.fill-field-card {
  position: relative;
  display: grid;
  gap: 8px;
  min-width: 0;
  padding: 12px 12px 40px;
  border: 1px solid var(--docsy-border-subtle);
  border-left: 3px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface-elevated) 96%, var(--docsy-surface-muted));
  overflow-wrap: break-word;
  word-break: break-word;
  transition:
    border-color 160ms ease,
    background-color 160ms ease,
    box-shadow 160ms ease;
}

.fill-field-card:focus-within {
  border-color: color-mix(in srgb, var(--docsy-primary) 55%, var(--docsy-border-subtle));
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--docsy-primary) 10%, transparent);
}

.fill-field-card.is-filled {
  border-left-color: var(--docsy-primary);
  background: color-mix(in srgb, var(--docsy-primary-soft) 34%, var(--docsy-surface-elevated));
}

.fill-field-body {
  display: grid;
  gap: 8px;
  min-width: 0;
}

.fill-field-body :deep(.el-input),
.fill-field-body :deep(.el-autocomplete),
.fill-field-body :deep(.el-select),
.fill-field-body :deep(.el-date-editor) {
  width: 100%;
  min-width: 0;
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
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  font-size: 12px;
  font-style: normal;
}

.fill-field-header em {
  background: #fef0f0;
  color: var(--el-color-danger);
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
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
  font-family: inherit;
}

.fill-structure-hint code.empty {
  color: var(--el-text-color-disabled);
}

.fill-structure-hint em {
  margin-left: 4px;
  color: var(--el-text-color-disabled);
  font-style: normal;
}

.party-list-editor {
  display: grid;
  gap: 8px;
  width: 100%;
}

.fill-structure-inline {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 6px;
  padding: 8px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
}

.item-structure-editor {
  display: grid;
  gap: 8px;
}

.reference-fill-editor {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.reference-fixed-value {
  padding: 5px 11px;
  border-radius: var(--docsy-radius);
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
  grid-template-columns: 24px minmax(84px, 110px) minmax(0, 1fr) minmax(92px, 118px) auto;
}

.party-list-row.compact.has-prefix:not(.has-suffix),
.party-list-row.compact.has-suffix:not(.has-prefix) {
  grid-template-columns: 24px minmax(0, 1fr) minmax(92px, 118px) auto;
}

.party-list-row.compact.no-affix {
  grid-template-columns: 24px minmax(0, 1fr) auto;
}

.party-list-row.compact .el-button {
  padding-left: 4px;
  padding-right: 4px;
}

.party-row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 4px;
  grid-column: 2 / -1;
  justify-content: flex-start;
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
  border-radius: var(--docsy-radius);
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
  min-width: 0;
  padding: 18px;
  border-left: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-muted);
}

.fill-preview-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.fill-preview-header h3 {
  margin: 2px 0 0;
  font-size: 15px;
  color: var(--docsy-text-strong);
}

.fill-preview-header span {
  color: var(--docsy-primary);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.1em;
}

.fill-preview-header p {
  margin: 0;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.fill-preview-header .preview-error-text {
  color: var(--el-color-danger);
}

.fill-preview-text {
  max-height: 60vh;
  overflow-y: auto;
  margin: 0;
  padding: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-strong);
  font-family: inherit;
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
}

.fill-preview-status {
  display: grid;
  place-items: center;
  gap: 8px;
  min-height: 220px;
  padding: 24px;
  border: 1px dashed var(--docsy-border-strong);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text);
  text-align: center;
}

.fill-preview-status p {
  max-width: 280px;
  line-height: 1.6;
}

.preview-loading-dot {
  width: 24px;
  height: 24px;
  border: 3px solid var(--docsy-border-strong);
  border-top-color: var(--docsy-primary);
  border-radius: 50%;
  animation: fill-preview-spin 700ms linear infinite;
}

@keyframes fill-preview-spin {
  to {
    transform: rotate(360deg);
  }
}

.preview-invitation {
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 6px;
  min-width: 0;
  padding: 28px 18px;
  border: 0;
  border-left: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
  cursor: pointer;
  font: inherit;
  text-align: center;
}

.preview-invitation:hover {
  background: var(--docsy-primary-soft);
}

.preview-invitation-icon {
  display: grid;
  place-items: center;
  width: 52px;
  height: 64px;
  margin-bottom: 6px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: var(--docsy-radius);
  background: var(--docsy-preview-paper, #fff);
  color: var(--docsy-primary);
  font-family: 'SimSun', '宋体', serif;
  font-size: 22px;
  box-shadow: 0 8px 22px rgba(48, 41, 32, 0.08);
}

.preview-invitation small {
  max-width: 180px;
  color: var(--docsy-text-muted);
  line-height: 1.5;
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

  .fill-command-bar {
    align-items: flex-start;
    flex-direction: column;
  }

  .fill-command-actions {
    justify-content: flex-start;
    width: 100%;
  }

  .fill-workbench,
  .fill-workbench.with-preview {
    grid-template-columns: 1fr;
  }

  .fill-preview-panel,
  .preview-invitation {
    border-top: 1px solid var(--docsy-border-subtle);
    border-left: 0;
  }

  .library-panel.has-active-template .panel-header {
    align-items: center;
    flex-direction: row;
    justify-content: space-between;
  }
}

@media (max-width: 760px) {
  .library-panel.has-active-template .template-library-grid {
    grid-template-columns: 1fr;
    max-height: 360px;
  }

  .fill-command-actions > *,
  .field-search-input {
    flex: 1 1 140px;
    width: auto;
  }

  .field-editor-column,
  .fill-preview-panel {
    padding: 14px;
  }

  .party-list-row,
  .party-list-row.compact,
  .party-list-row.compact.no-affix {
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
