<template>
  <div>
    <div class="rule-item section-label">
      <strong>页眉文字</strong>
      <div class="section-actions">
        <el-switch v-model="headerInsertEnabledModel" size="small" />
        <el-button size="small" circle :disabled="!headerInsertEnabled" @click="addGroup">
          <el-icon><i-ep-plus /></el-icon>
        </el-button>
      </div>
    </div>
    <!-- Overlap warnings -->
    <el-alert
      v-if="overlapWarnings.length"
      type="warning"
      :closable="false"
      show-icon
      class="overlap-warnings"
    >
      <template #title>{{ overlapWarnings.join('；') }}</template>
    </el-alert>
    <!-- Group list (only when >1 group) -->
    <div v-if="headerGroups.length > 1" class="header-group-list">
      <div
        v-for="group in headerGroups"
        :key="group.id"
        class="header-group-item"
        :class="{ active: group.id === selectedHeaderGroupId }"
        @click="$emit('update:selectedHeaderGroupId', group.id)"
      >
        <span class="group-label">
          <el-icon v-if="group.id === selectedHeaderGroupId"><i-ep-arrow-right /></el-icon>
          {{ group.label || '页眉' }} · {{ groupModeLabel(group) }}
        </span>
        <el-button
          size="small"
          link
          type="danger"
          :disabled="headerGroups.length <= 1"
          @click.stop="removeGroup(group.id)"
        >
          删除
        </el-button>
      </div>
    </div>
    <!-- Settings for selected group -->
    <div class="rule-item">
      <label>页眉来源</label>
      <el-select v-model="headerModeModel" :disabled="!headerInsertEnabled">
        <el-option label="不插入页眉" value="none" />
        <el-option label="文件名" value="filename" />
        <el-option label="按证据列表名称" value="per_file" />
        <el-option label="固定文本" value="custom" />
      </el-select>
    </div>
    <div v-if="headerMode === 'custom'" class="rule-item">
      <label>页眉文本</label>
      <el-input v-model="headerTextModel" :disabled="!headerInsertEnabled" placeholder="可用 [##]、[序号]、[文件名]、[YYYYMMDD]" />
    </div>
    <div class="rule-item">
      <label>页眉前缀</label><el-input v-model="headerPrefixModel" :disabled="!headerInsertEnabled || headerMode === 'none'" />
    </div>
    <div class="rule-item">
      <label>页眉后缀</label><el-input v-model="headerSuffixModel" :disabled="!headerInsertEnabled || headerMode === 'none'" />
    </div>
    <TextPlacementFields
      prefix="页眉"
      :disabled="!headerInsertEnabled || headerMode === 'none'"
      v-model:align="headerAlignModel"
      v-model:font-size="headerFontSizeModel"
      v-model:font-family="headerFontFamilyModel"
      v-model:margin-mm="headerMarginMmModel"
      v-model:offset-x-mm="headerOffsetXMmModel"
      v-model:color="headerColorModel"
      :offset-limit-mm="offsetLimitMm"
      margin-label="距顶"
    />

    <div class="rule-item section-label">
      <strong>页脚文字</strong>
      <div class="section-actions">
        <el-switch v-model="footerInsertEnabledModel" size="small" />
        <el-button size="small" circle :disabled="!footerInsertEnabled" @click="addFooterTextGroup">
          <el-icon><i-ep-plus /></el-icon>
        </el-button>
      </div>
    </div>
    <div class="rule-item">
      <label>页脚文本</label>
      <el-input v-model="footerTextContentModel" :disabled="!footerInsertEnabled" placeholder="固定文字，不用于页码" />
    </div>
    <TextPlacementFields
      prefix="页脚"
      :disabled="!footerInsertEnabled"
      v-model:align="footerTextAlignModel"
      v-model:font-size="footerTextFontSizeModel"
      v-model:font-family="footerTextFontFamilyModel"
      v-model:margin-mm="footerTextMarginMmModel"
      v-model:offset-x-mm="footerTextOffsetXMmModel"
      v-model:color="footerTextColorModel"
      :offset-limit-mm="offsetLimitMm"
      margin-label="距底"
    />

    <div class="rule-item section-label">
      <strong>页码</strong>
      <div class="section-actions">
        <el-switch v-model="pageNumberEnabledModel" size="small" />
        <el-button size="small" circle :disabled="!pageNumberEnabled" @click="addPageNumberGroup">
          <el-icon><i-ep-plus /></el-icon>
        </el-button>
      </div>
    </div>
    <div class="rule-item">
      <label>连续方式</label>
      <el-select v-model="pageNumberSequenceModel" :disabled="!pageNumberEnabled">
        <el-option label="全部文件连续" value="continuous" />
        <el-option label="每个文件单独编号" value="per-file" />
      </el-select>
    </div>
    <div class="rule-item">
      <label>页码样式</label>
      <el-select v-model="pageNumberStyleModel" :disabled="!pageNumberEnabled">
        <el-option
          v-for="style in PAGE_NUMBER_STYLES"
          :key="style.value"
          :value="style.value"
          :label="`${style.label} · ${style.sample}`"
        />
      </el-select>
    </div>
    <div class="rule-item">
      <label>显示总页数</label>
      <el-switch v-model="pageNumberShowTotalModel" active-text="开" inactive-text="关" :disabled="!pageNumberEnabled" />
    </div>
    <div class="rule-item">
      <label>页码格式</label>
      <div class="template-presets">
        <el-radio-group
          v-model="pageNumberPresetModel"
          size="small"
          :disabled="!pageNumberEnabled"
          @change="applyPresetTemplate"
        >
          <el-radio-button v-for="p in filteredPageNumberPresets" :key="p.value" :value="p.value">
            {{ p.label }}
          </el-radio-button>
        </el-radio-group>
      </div>
      <el-input
        v-model="pageNumberTemplateModel"
        :disabled="!pageNumberEnabled"
        placeholder="例如 {page}/{total}、-{page}-"
        size="small"
        class="template-custom-input"
      />
    </div>
    <div class="rule-item" v-if="pageNumberEnabled">
      <label>预览</label>
      <span class="page-number-preview">{{ pageNumberPreviewText }}</span>
    </div>
    <div class="rule-item">
      <label>页码区域</label>
      <el-select v-model="pageNumberRegionModel" :disabled="!pageNumberEnabled">
        <el-option label="页脚区域" value="footer" />
        <el-option label="页眉区域" value="header" />
      </el-select>
    </div>
    <TextPlacementFields
      prefix="页码"
      :disabled="!pageNumberEnabled"
      v-model:align="pageNumberAlignModel"
      v-model:font-size="pageNumberFontSizeModel"
      v-model:font-family="pageNumberFontFamilyModel"
      v-model:margin-mm="pageNumberMarginMmModel"
      v-model:offset-x-mm="pageNumberOffsetXMmModel"
      v-model:color="pageNumberColorModel"
      :offset-limit-mm="offsetLimitMm"
      :margin-label="pageNumberRegion === 'header' ? '距顶' : '距底'"
    />
    <div class="rule-item">
      <label>分段与例外</label>
      <el-button :disabled="!pageNumberEnabled" @click="$emit('editPageNumberRules')">
        设置规则{{ pageNumberOverrideCount ? `（${pageNumberOverrideCount} 条）` : '' }}
      </el-button>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import TextPlacementFields from './TextPlacementFields.vue'
import { PAGE_NUMBER_STYLES, renderPageNumberTemplate } from '../composables/pdfPageNumberRules.js'

const PRESETS_WITH_TOTAL = [
  { value: '{page}/{total}', label: '1/35' },
  { value: '第{page}页，共{total}页', label: '第1页，共35页' },
  { value: '{page} of {total}', label: '1 of 35' },
]
const PRESETS_NO_TOTAL = [
  { value: '{page}', label: '1' },
  { value: '-{page}-', label: '-1-' },
  { value: '— {page} —', label: '— 1 —' },
  { value: '第{page}页', label: '第1页' },
  { value: '（{page}）', label: '（1）' },
]

const MODE_LABELS = {
  none: '不插入',
  filename: '文件名',
  per_file: '按列表名称',
  custom: '固定文本',
}

const props = defineProps({
  headerGroups: { type: Array, required: true },
  selectedHeaderGroupId: { type: String, required: true },
  headerInsertEnabled: { type: Boolean, default: true },
  // Legacy single-group props (still used for selected group via parent computed)
  headerMode: { type: String, required: true },
  headerText: { type: String, required: true },
  headerPrefix: { type: String, required: true },
  headerSuffix: { type: String, required: true },
  headerAlign: { type: String, required: true },
  headerFontSize: { type: Number, required: true },
  headerFontFamily: { type: String, required: true },
  headerMarginMm: { type: Number, required: true },
  headerOffsetXMm: { type: Number, required: true },
  headerColor: { type: String, required: true },
  footerTextGroups: { type: Array, default: () => [] },
  selectedFooterTextGroupId: { type: String, default: '' },
  footerInsertEnabled: { type: Boolean, default: true },
  footerTextContent: { type: String, required: true },
  footerTextAlign: { type: String, required: true },
  footerTextFontSize: { type: Number, required: true },
  footerTextFontFamily: { type: String, required: true },
  footerTextMarginMm: { type: Number, required: true },
  footerTextOffsetXMm: { type: Number, required: true },
  footerTextColor: { type: String, required: true },
  pageNumberGroups: { type: Array, default: () => [] },
  selectedPageNumberGroupId: { type: String, default: '' },
  pageNumberEnabled: { type: Boolean, required: true },
  pageNumberSequence: { type: String, required: true },
  pageNumberStyle: { type: String, required: true },
  pageNumberTemplate: { type: String, required: true },
  pageNumberRegion: { type: String, required: true },
  pageNumberAlign: { type: String, required: true },
  pageNumberFontSize: { type: Number, required: true },
  pageNumberFontFamily: { type: String, required: true },
  pageNumberMarginMm: { type: Number, required: true },
  pageNumberOffsetXMm: { type: Number, required: true },
  pageNumberColor: { type: String, required: true },
  pageNumberOverrideCount: { type: Number, default: 0 },
  pageNumberShowTotal: { type: Boolean, default: true },
  pageNumberSamplePage: { type: Number, default: 1 },
  pageNumberSampleTotal: { type: Number, default: 35 },
  offsetLimitMm: { type: Number, default: 120 },
})

const emit = defineEmits([
  'editPageNumberRules',
  'update:pageNumberShowTotal',
  'update:headerGroups',
  'update:selectedHeaderGroupId',
  'update:footerTextGroups',
  'update:selectedFooterTextGroupId',
  'update:pageNumberGroups',
  'update:selectedPageNumberGroupId',
  ...[
    'headerMode',
    'headerInsertEnabled',
    'headerText',
    'headerPrefix',
    'headerSuffix',
    'headerAlign',
    'headerFontSize',
    'headerFontFamily',
    'headerMarginMm',
    'headerOffsetXMm',
    'headerColor',
    'footerInsertEnabled',
    'footerTextContent',
    'footerTextAlign',
    'footerTextFontSize',
    'footerTextFontFamily',
    'footerTextMarginMm',
    'footerTextOffsetXMm',
    'footerTextColor',
    'pageNumberEnabled',
    'pageNumberSequence',
    'pageNumberStyle',
    'pageNumberTemplate',
    'pageNumberRegion',
    'pageNumberAlign',
    'pageNumberFontSize',
    'pageNumberFontFamily',
    'pageNumberMarginMm',
    'pageNumberOffsetXMm',
    'pageNumberColor',
  ].map((key) => `update:${key}`),
])

function model(key) {
  return computed({ get: () => props[key], set: (value) => emit(`update:${key}`, value) })
}
const headerModeModel = model('headerMode'),
  headerInsertEnabledModel = model('headerInsertEnabled'),
  headerTextModel = model('headerText'),
  headerPrefixModel = model('headerPrefix'),
  headerSuffixModel = model('headerSuffix')
const headerAlignModel = model('headerAlign'),
  headerFontSizeModel = model('headerFontSize'),
  headerFontFamilyModel = model('headerFontFamily'),
  headerMarginMmModel = model('headerMarginMm'),
  headerOffsetXMmModel = model('headerOffsetXMm'),
  headerColorModel = model('headerColor')
const footerTextEnabledModel = model('footerTextEnabled'),
  footerInsertEnabledModel = model('footerInsertEnabled'),
  footerTextContentModel = model('footerTextContent'),
  footerTextAlignModel = model('footerTextAlign'),
  footerTextFontSizeModel = model('footerTextFontSize'),
  footerTextFontFamilyModel = model('footerTextFontFamily'),
  footerTextMarginMmModel = model('footerTextMarginMm'),
  footerTextOffsetXMmModel = model('footerTextOffsetXMm'),
  footerTextColorModel = model('footerTextColor')
const pageNumberEnabledModel = model('pageNumberEnabled'),
  pageNumberSequenceModel = model('pageNumberSequence'),
  pageNumberStyleModel = model('pageNumberStyle'),
  pageNumberTemplateModel = model('pageNumberTemplate'),
  pageNumberRegionModel = model('pageNumberRegion'),
  pageNumberAlignModel = model('pageNumberAlign'),
  pageNumberFontSizeModel = model('pageNumberFontSize'),
  pageNumberFontFamilyModel = model('pageNumberFontFamily'),
  pageNumberMarginMmModel = model('pageNumberMarginMm'),
  pageNumberOffsetXMmModel = model('pageNumberOffsetXMm'),
  pageNumberColorModel = model('pageNumberColor')
const pageNumberShowTotalModel = model('pageNumberShowTotal')
const pageNumberPresetModel = computed({
  get() {
    const tpl = props.pageNumberTemplate
    const presets = props.pageNumberShowTotal ? PRESETS_WITH_TOTAL : PRESETS_NO_TOTAL
    return presets.some(p => p.value === tpl) ? tpl : 'custom'
  },
  set() {},
})
const filteredPageNumberPresets = computed(() =>
  props.pageNumberShowTotal ? PRESETS_WITH_TOTAL : PRESETS_NO_TOTAL
)
function applyPresetTemplate(val) {
  if (val && val !== 'custom') pageNumberTemplateModel.value = val
}
const pageNumberPreviewText = computed(() => {
  let tpl = props.pageNumberTemplate || '{page}'
  if (!props.pageNumberShowTotal) tpl = tpl.replaceAll('{total}', '').replaceAll('//', '/').replace(/\/+$/, '')
  return renderPageNumberTemplate(tpl, props.pageNumberSamplePage, props.pageNumberSampleTotal, props.pageNumberStyle)
})

// Overlap detection
const overlapWarnings = computed(() => {
  const warnings = []
  const groups = props.headerGroups || []
  // Check header-to-header overlap (±2mm tolerance)
  for (let i = 0; i < groups.length; i++) {
    for (let j = i + 1; j < groups.length; j++) {
      if (groups[i].enabled && groups[j].enabled && Math.abs(groups[i].marginMm - groups[j].marginMm) <= 2) {
        warnings.push(`"${groups[i].label || '页眉 ' + (i + 1)}" 与 "${groups[j].label || '页眉 ' + (j + 1)}" 距顶距离接近，可能重叠`)
      }
    }
  }
  // Check header vs footer collision (page height ~297mm for A4)
  const pageHeight = 297
  const headerBottom = groups.filter(g => g.enabled).map(g => g.marginMm + (g.fontSize || 10) * 0.4)
  const footerTop = pageHeight - (props.pageNumberMarginMm || 10) - (props.pageNumberFontSize || 9) * 0.4
  for (const hb of headerBottom) {
    if (hb > footerTop - 5) {
      warnings.push(`页眉底部（${hb.toFixed(1)}mm）与页码区域可能碰撞`)
    }
  }
  // Check footer text vs page number overlap
  const ftMargin = props.footerTextMarginMm || 10
  const pnMargin = props.pageNumberMarginMm || 10
  if (props.footerInsertEnabled && props.pageNumberEnabled && Math.abs(ftMargin - pnMargin) <= 3) {
    warnings.push('页脚文字与页码的距底距离接近，可能重叠')
  }
  return warnings
})

function groupModeLabel(group) {
  return MODE_LABELS[group.mode] || group.mode
}

let groupCounter = 1
function addGroup() {
  groupCounter++
  const newGroup = {
    id: `h${Date.now()}`,
    label: `页眉 ${props.headerGroups.length + 1}`,
    enabled: true,
    mode: 'filename',
    text: '',
    prefix: '',
    suffix: '',
    align: 'right',
    fontSize: 10,
    fontFamily: 'auto',
    marginMm: 10,
    offsetXMm: 0,
    color: '#000000',
  }
  const updated = [...props.headerGroups, newGroup]
  emit('update:headerGroups', updated)
  emit('update:selectedHeaderGroupId', newGroup.id)
}

function removeGroup(id) {
  if (props.headerGroups.length <= 1) return
  const updated = props.headerGroups.filter((g) => g.id !== id)
  emit('update:headerGroups', updated)
  if (props.selectedHeaderGroupId === id) {
    emit('update:selectedHeaderGroupId', updated[0].id)
  }
}

function addFooterTextGroup() {
  const newGroup = {
    id: `ft${Date.now()}`,
    label: `页脚文字 ${props.footerTextGroups.length + 1}`,
    enabled: true,
    text: '',
    align: 'left',
    fontSize: 9,
    fontFamily: 'auto',
    marginMm: 10,
    offsetXMm: 0,
    color: '#000000',
  }
  const updated = [...props.footerTextGroups, newGroup]
  emit('update:footerTextGroups', updated)
  emit('update:selectedFooterTextGroupId', newGroup.id)
}

function removeFooterTextGroup(id) {
  if (props.footerTextGroups.length <= 1) return
  const updated = props.footerTextGroups.filter((g) => g.id !== id)
  emit('update:footerTextGroups', updated)
  if (props.selectedFooterTextGroupId === id) {
    emit('update:selectedFooterTextGroupId', updated[0].id)
  }
}

function addPageNumberGroup() {
  const newGroup = {
    id: `pn${Date.now()}`,
    label: `页码 ${props.pageNumberGroups.length + 1}`,
    enabled: true,
    sequence: 'continuous',
    style: 'arabic',
    template: '{page}/{total}',
    region: 'footer',
    align: 'center',
    fontSize: 9,
    fontFamily: 'auto',
    marginMm: 10,
    offsetXMm: 0,
    color: '#000000',
  }
  const updated = [...props.pageNumberGroups, newGroup]
  emit('update:pageNumberGroups', updated)
  emit('update:selectedPageNumberGroupId', newGroup.id)
}

function removePageNumberGroup(id) {
  if (props.pageNumberGroups.length <= 1) return
  const updated = props.pageNumberGroups.filter((g) => g.id !== id)
  emit('update:pageNumberGroups', updated)
  if (props.selectedPageNumberGroupId === id) {
    emit('update:selectedPageNumberGroupId', updated[0].id)
  }
}
</script>

<style scoped>
.section-label {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 4px;
  padding-top: 8px;
  border-top: 1px solid var(--docsy-border-subtle);
}
.section-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.overlap-warnings {
  grid-column: 1 / -1;
  margin-bottom: 4px;
}
.header-group-list {
  grid-column: 1 / -1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-bottom: 4px;
  padding: 4px 0;
  border-bottom: 1px solid var(--docsy-border-subtle);
}
.header-group-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  color: var(--docsy-text-muted);
  transition: background 0.15s;
}
.header-group-item:hover {
  background: var(--docsy-surface-hover);
}
.header-group-item.active {
  background: var(--docsy-surface-active);
  color: var(--docsy-text-strong);
  font-weight: 500;
}
.group-label {
  display: flex;
  align-items: center;
  gap: 4px;
}
.page-number-preview {
  display: inline-block;
  padding: 2px 8px;
  font-size: 13px;
  color: var(--docsy-text-strong);
  background: var(--docsy-surface-muted);
  border-radius: 4px;
}
.template-presets { margin-bottom: 4px; }
.template-custom-input { margin-top: 4px; }
</style>
