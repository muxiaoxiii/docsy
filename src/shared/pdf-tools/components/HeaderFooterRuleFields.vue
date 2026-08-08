<template>
  <div>
    <div class="rule-item section-label">
      <strong>页眉文字</strong>
      <div class="section-actions">
        <el-switch v-model="headerInsertEnabledModel" size="small" />
        <el-button size="small" circle :disabled="!headerInsertEnabled" @click="addGroup">
          <el-icon><Plus /></el-icon>
        </el-button>
        <UndoRedoButtons
          :can-undo="headerHistory.canUndo"
          :can-redo="headerHistory.canRedo"
          @undo="headerHistory.undo()"
          @redo="headerHistory.redo()"
          compact
        />
      </div>
    </div>
    <template v-if="headerInsertEnabled">
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
        <el-select v-model="headerModeModel">
          <el-option label="不插入页眉" value="none" />
          <el-option label="文件名" value="filename" />
          <el-option label="按证据列表名称" value="per_file" />
          <el-option label="固定文本" value="custom" />
          <el-option label="序号（证据1, 证据2）" value="seq" />
          <el-option label="中文序号（证据一、证据二）" value="seq_cn" />
          <el-option label="前缀+序号" value="prefix_seq" />
        </el-select>
      </div>
      <div v-if="headerMode === 'custom' || headerMode === 'seq' || headerMode === 'prefix_seq'" class="rule-item">
        <label>{{ headerMode === 'prefix_seq' ? '前缀' : (headerMode === 'seq' ? '前缀（可选）' : '页眉文本') }}</label>
        <div class="text-input-with-info">
          <el-input v-model="headerTextModel" placeholder="输入文本或模板标记" />
          <el-tooltip placement="top" :show-after="200" class="template-info-tip">
            <template #content>
              <div class="template-help">
                <p><strong>模板标记（自动替换）：</strong></p>
                <p><code>[文件名]</code> — 去掉扩展名的文件名</p>
                <p><code>[序号]</code> — 文件序号（1, 2, 3...）</p>
                <p><code>[中文序号]</code> — 中文序号（一、二、三...）</p>
                <p><code>[#]</code> / <code>[##]</code> / <code>[###]</code> — 序号，位数=#个数（01, 001...）</p>
                <p><code>[日期]</code> — 当前日期（YYYYMMDD）</p>
                <p><code>[YYYY-MM-DD]</code> — 自定义日期格式</p>
                <p style="margin-top:6px;color:var(--el-text-color-placeholder);">可与固定文字混合使用，如"证据[#]-[文件名]"</p>
              </div>
            </template>
            <el-icon class="info-icon"><InfoFilled /></el-icon>
          </el-tooltip>
        </div>
      </div>
      <div class="rule-item">
        <label>页眉前缀</label><el-input v-model="headerPrefixModel" :disabled="headerMode === 'none'" />
      </div>
      <div class="rule-item">
        <label>页眉后缀</label><el-input v-model="headerSuffixModel" :disabled="headerMode === 'none'" />
      </div>
      <TextPlacementFields
        prefix="页眉"
        :disabled="headerMode === 'none'"
        v-model:align="headerAlignModel"
        v-model:font-size="headerFontSizeModel"
        v-model:font-family="headerFontFamilyModel"
        v-model:margin-mm="headerMarginMmModel"
        v-model:offset-x-mm="headerOffsetXMmModel"
        v-model:color="headerColorModel"
        :offset-limit-mm="offsetLimitMm"
        margin-label="距顶"
      />
      <div class="rule-item page-range-row">
        <label>显示范围</label>
        <div class="page-range-inputs">
          <el-input-number
            v-model="headerPageStartModel"
            :min="1"
            :max="9999"
            size="small"
            controls-position="right"
            placeholder="起始页"
          />
          <span class="range-sep">–</span>
          <el-input-number
            v-model="headerPageEndModel"
            :min="0"
            :max="9999"
            size="small"
            controls-position="right"
            placeholder="0=全部"
          />
        </div>
      </div>
    </template>

    <div class="rule-item section-label">
      <strong>页脚文字</strong>
      <div class="section-actions">
        <el-switch v-model="footerInsertEnabledModel" size="small" />
        <el-button size="small" circle :disabled="!footerInsertEnabled" @click="addFooterTextGroup">
          <el-icon><Plus /></el-icon>
        </el-button>
        <UndoRedoButtons
          :can-undo="footerTextHistory.canUndo"
          :can-redo="footerTextHistory.canRedo"
          @undo="footerTextHistory.undo()"
          @redo="footerTextHistory.redo()"
          compact
        />
      </div>
    </div>
    <template v-if="footerInsertEnabled">
      <!-- Footer text group list (only when >1 group) -->
      <div v-if="footerTextGroups.length > 1" class="header-group-list">
        <div
          v-for="group in footerTextGroups"
          :key="group.id"
          class="header-group-item"
          :class="{ active: group.id === selectedFooterTextGroupId }"
          @click="$emit('update:selectedFooterTextGroupId', group.id)"
        >
          <span class="group-label">
            <el-icon v-if="group.id === selectedFooterTextGroupId"><i-ep-arrow-right /></el-icon>
            {{ group.label || '页脚文字' }} · {{ group.text || '（空）' }}
          </span>
          <el-button
            size="small"
            link
            type="danger"
            :disabled="footerTextGroups.length <= 1"
            @click.stop="removeFooterTextGroup(group.id)"
          >
            删除
          </el-button>
        </div>
      </div>
      <div class="rule-item">
        <label>页脚文本</label>
        <div class="text-input-with-info">
          <el-input v-model="footerTextContentModel" placeholder="输入文本或模板标记" />
          <el-tooltip placement="top" :show-after="200" class="template-info-tip">
            <template #content>
              <div class="template-help">
                <p><strong>模板标记（自动替换）：</strong></p>
                <p><code>[文件名]</code> — 去掉扩展名的文件名</p>
                <p><code>[序号]</code> — 文件序号（1, 2, 3...）</p>
                <p><code>[中文序号]</code> — 中文序号（一、二、三...）</p>
                <p><code>[#]</code> / <code>[##]</code> / <code>[###]</code> — 序号，位数=#个数（01, 001...）</p>
                <p><code>[日期]</code> — 当前日期（YYYYMMDD）</p>
                <p><code>[YYYY-MM-DD]</code> — 自定义日期格式</p>
                <p style="margin-top:6px;color:var(--el-text-color-placeholder);">可与固定文字混合使用，如"证据[#]-[文件名]"</p>
              </div>
            </template>
            <el-icon class="info-icon"><InfoFilled /></el-icon>
          </el-tooltip>
        </div>
      </div>
      <TextPlacementFields
        prefix="页脚"
        v-model:align="footerTextAlignModel"
        v-model:font-size="footerTextFontSizeModel"
        v-model:font-family="footerTextFontFamilyModel"
        v-model:margin-mm="footerTextMarginMmModel"
        v-model:offset-x-mm="footerTextOffsetXMmModel"
        v-model:color="footerTextColorModel"
        :offset-limit-mm="offsetLimitMm"
        margin-label="距底"
      />
      <div class="rule-item page-range-row">
        <label>显示范围</label>
        <div class="page-range-inputs">
          <el-input-number
            v-model="footerTextPageStartModel"
            :min="1"
            :max="9999"
            size="small"
            controls-position="right"
            placeholder="起始页"
          />
          <span class="range-sep">–</span>
          <el-input-number
            v-model="footerTextPageEndModel"
            :min="0"
            :max="9999"
            size="small"
            controls-position="right"
            placeholder="0=全部"
          />
        </div>
      </div>
    </template>

    <div class="rule-item section-label">
      <strong>页码</strong>
      <div class="section-actions">
        <el-switch v-model="pageNumberEnabledModel" size="small" />
        <el-button size="small" circle :disabled="!pageNumberEnabled" @click="addPageNumberGroup">
          <el-icon><Plus /></el-icon>
        </el-button>
        <UndoRedoButtons
          :can-undo="pageNumberHistory.canUndo"
          :can-redo="pageNumberHistory.canRedo"
          @undo="pageNumberHistory.undo()"
          @redo="pageNumberHistory.redo()"
          compact
        />
      </div>
    </div>
    <template v-if="pageNumberEnabled">
      <!-- Page number group list (only when >1 group) -->
      <div v-if="pageNumberGroups.length > 1" class="header-group-list">
        <div
          v-for="group in pageNumberGroups"
          :key="group.id"
          class="header-group-item"
          :class="{ active: group.id === selectedPageNumberGroupId }"
          @click="$emit('update:selectedPageNumberGroupId', group.id)"
        >
          <span class="group-label">
            <el-icon v-if="group.id === selectedPageNumberGroupId"><i-ep-arrow-right /></el-icon>
            {{ group.label || '页码' }} · {{ group.template || '{page}/{total}' }}
          </span>
          <el-button
            size="small"
            link
            type="danger"
            :disabled="pageNumberGroups.length <= 1"
            @click.stop="removePageNumberGroup(group.id)"
          >
            删除
          </el-button>
        </div>
      </div>
      <div class="rule-item">
        <label>连续方式</label>
        <el-select v-model="pageNumberSequenceModel">
          <el-option label="全部文件连续" value="continuous" />
          <el-option label="每个文件单独编号" value="per-file" />
        </el-select>
      </div>
      <div class="rule-item">
        <label>页码样式</label>
        <el-select v-model="pageNumberStyleModel">
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
        <el-switch v-model="pageNumberShowTotalModel" active-text="开" inactive-text="关" />
      </div>
      <div class="rule-item">
        <label>页码格式</label>
        <div class="template-presets">
          <el-radio-group
            v-model="pageNumberPresetModel"
            size="small"
            @change="applyPresetTemplate"
          >
            <el-radio-button v-for="p in filteredPageNumberPresets" :key="p.value" :value="p.value">
              {{ p.label }}
            </el-radio-button>
          </el-radio-group>
        </div>
        <el-input
          v-model="pageNumberTemplateModel"
          placeholder="例如 {page}/{total}、-{page}-"
          size="small"
          class="template-custom-input"
        />
      </div>
      <div class="rule-item">
        <label>预览</label>
        <span class="page-number-preview">{{ pageNumberPreviewText }}</span>
      </div>
      <div class="rule-item">
        <label>页码区域</label>
        <el-select v-model="pageNumberRegionModel">
          <el-option label="页脚区域" value="footer" />
          <el-option label="页眉区域" value="header" />
        </el-select>
      </div>
      <TextPlacementFields
        prefix="页码"
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
        <el-button @click="$emit('editPageNumberRules')">
          设置规则{{ pageNumberOverrideCount ? `（${pageNumberOverrideCount} 条）` : '' }}
        </el-button>
      </div>
    </template>
  </div>
</template>

<script setup>
import { computed, watch } from 'vue'
import { Plus, InfoFilled } from '@element-plus/icons-vue'
import TextPlacementFields from './TextPlacementFields.vue'
import { PAGE_NUMBER_STYLES, renderPageNumberTemplate } from '../composables/pdfPageNumberRules.js'
import { useHistory } from '../../../core/composables/useHistory.js'
import UndoRedoButtons from '../../../components/UndoRedoButtons.vue'

function debounce(fn, ms) {
  let timer
  return (...args) => {
    clearTimeout(timer)
    timer = setTimeout(() => fn(...args), ms)
  }
}

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
  headerPageStart: { type: Number, default: 1 },
  headerPageEnd: { type: Number, default: 0 },
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
  footerTextPageStart: { type: Number, default: 1 },
  footerTextPageEnd: { type: Number, default: 0 },
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
  pageHeightMm: { type: Number, default: 297 },
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
    'headerPageStart',
    'headerPageEnd',
    'footerInsertEnabled',
    'footerTextContent',
    'footerTextAlign',
    'footerTextFontSize',
    'footerTextFontFamily',
    'footerTextMarginMm',
    'footerTextOffsetXMm',
    'footerTextColor',
    'footerTextPageStart',
    'footerTextPageEnd',
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

// --- Undo/Redo history for each section ---
const headerHistory = useHistory({
  snapshot: () => ({
    align: props.headerAlign,
    fontSize: props.headerFontSize,
    fontFamily: props.headerFontFamily,
    marginMm: props.headerMarginMm,
    offsetXMm: props.headerOffsetXMm,
    color: props.headerColor,
  }),
  restore: (s) => {
    emit('update:headerAlign', s.align)
    emit('update:headerFontSize', s.fontSize)
    emit('update:headerFontFamily', s.fontFamily)
    emit('update:headerMarginMm', s.marginMm)
    emit('update:headerOffsetXMm', s.offsetXMm)
    emit('update:headerColor', s.color)
  },
  onChange: () => emit('change'),
})

const footerTextHistory = useHistory({
  snapshot: () => ({
    align: props.footerTextAlign,
    fontSize: props.footerTextFontSize,
    fontFamily: props.footerTextFontFamily,
    marginMm: props.footerTextMarginMm,
    offsetXMm: props.footerTextOffsetXMm,
    color: props.footerTextColor,
  }),
  restore: (s) => {
    emit('update:footerTextAlign', s.align)
    emit('update:footerTextFontSize', s.fontSize)
    emit('update:footerTextFontFamily', s.fontFamily)
    emit('update:footerTextMarginMm', s.marginMm)
    emit('update:footerTextOffsetXMm', s.offsetXMm)
    emit('update:footerTextColor', s.color)
  },
  onChange: () => emit('change'),
})

const pageNumberHistory = useHistory({
  snapshot: () => ({
    align: props.pageNumberAlign,
    fontSize: props.pageNumberFontSize,
    fontFamily: props.pageNumberFontFamily,
    marginMm: props.pageNumberMarginMm,
    offsetXMm: props.pageNumberOffsetXMm,
    color: props.pageNumberColor,
  }),
  restore: (s) => {
    emit('update:pageNumberAlign', s.align)
    emit('update:pageNumberFontSize', s.fontSize)
    emit('update:pageNumberFontFamily', s.fontFamily)
    emit('update:pageNumberMarginMm', s.marginMm)
    emit('update:pageNumberOffsetXMm', s.offsetXMm)
    emit('update:pageNumberColor', s.color)
  },
  onChange: () => emit('change'),
})

// Debounced watches: push history when placement params change
watch(
  () => [props.headerAlign, props.headerFontSize, props.headerFontFamily, props.headerMarginMm, props.headerOffsetXMm, props.headerColor],
  debounce(() => headerHistory.push(), 500),
  { deep: true },
)
watch(
  () => [props.footerTextAlign, props.footerTextFontSize, props.footerTextFontFamily, props.footerTextMarginMm, props.footerTextOffsetXMm, props.footerTextColor],
  debounce(() => footerTextHistory.push(), 500),
  { deep: true },
)
watch(
  () => [props.pageNumberAlign, props.pageNumberFontSize, props.pageNumberFontFamily, props.pageNumberMarginMm, props.pageNumberOffsetXMm, props.pageNumberColor],
  debounce(() => pageNumberHistory.push(), 500),
  { deep: true },
)

// Expose undo/redo for parent-level keyboard shortcut dispatch
function undo() {
  if (headerHistory.canUndo.value) { headerHistory.undo(); return true }
  if (footerTextHistory.canUndo.value) { footerTextHistory.undo(); return true }
  if (pageNumberHistory.canUndo.value) { pageNumberHistory.undo(); return true }
  return false
}
function redo() {
  if (headerHistory.canRedo.value) { headerHistory.redo(); return true }
  if (footerTextHistory.canRedo.value) { footerTextHistory.redo(); return true }
  if (pageNumberHistory.canRedo.value) { pageNumberHistory.redo(); return true }
  return false
}
defineExpose({ undo, redo })

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
const headerPageStartModel = model('headerPageStart')
const headerPageEndModel = model('headerPageEnd')
const footerInsertEnabledModel = model('footerInsertEnabled'),
  footerTextContentModel = model('footerTextContent'),
  footerTextAlignModel = model('footerTextAlign'),
  footerTextFontSizeModel = model('footerTextFontSize'),
  footerTextFontFamilyModel = model('footerTextFontFamily'),
  footerTextMarginMmModel = model('footerTextMarginMm'),
  footerTextOffsetXMmModel = model('footerTextOffsetXMm'),
  footerTextColorModel = model('footerTextColor')
const footerTextPageStartModel = model('footerTextPageStart')
const footerTextPageEndModel = model('footerTextPageEnd')
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
// Sync template when showTotal changes
let lastTemplateWithTotal = ''
watch(() => props.pageNumberShowTotal, (showTotal) => {
  const tpl = props.pageNumberTemplate || '{page}/{total}'
  if (!showTotal && tpl.includes('{total}')) {
    // Save template before stripping
    lastTemplateWithTotal = tpl
    const cleaned = tpl
      .replaceAll('{total}', '')
      .replaceAll('//', '/')
      .replace(/\/+$/, '')
      .replace(/^\//, '')
    pageNumberTemplateModel.value = cleaned || '{page}'
  } else if (showTotal && !tpl.includes('{total}')) {
    // Restore saved template or use default
    pageNumberTemplateModel.value = lastTemplateWithTotal || PRESETS_WITH_TOTAL[0]?.value || '{page}/{total}'
  }
})
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
  const headerGroups = props.headerGroups || []
  const footerTextGroups = props.footerTextGroups || []
  const pageNumberGroups = props.pageNumberGroups || []
  const pageHeight = Number(props.pageHeightMm || 297)
  const enabledHeader = headerGroups.filter((g) => g.enabled)
  const enabledFooterText = footerTextGroups.filter((g) => g.enabled)
  const enabledPnFooter = pageNumberGroups.filter((g) => g.enabled && g.region !== 'header')
  const enabledPnHeader = pageNumberGroups.filter((g) => g.enabled && g.region === 'header')
  const label = (g, fallback) => g.label || fallback
  // Header-to-header overlap (±2mm tolerance)
  for (let i = 0; i < enabledHeader.length; i++) {
    for (let j = i + 1; j < enabledHeader.length; j++) {
      if (Math.abs(enabledHeader[i].marginMm - enabledHeader[j].marginMm) <= 2) {
        warnings.push(`"${label(enabledHeader[i], '页眉 ' + (i + 1))}" 与 "${label(enabledHeader[j], '页眉 ' + (j + 1))}" 距顶距离接近，可能重叠`)
      }
    }
  }
  // Header bottom vs page number area (footer-region page numbers) and header-region page numbers
  for (const h of enabledHeader) {
    const headerBottom = (h.marginMm || 10) + (h.fontSize || 10) * 0.4
    for (const pn of enabledPnFooter) {
      const pnTop = pageHeight - (pn.marginMm || 10) - (pn.fontSize || 9) * 0.4
      if (headerBottom > pnTop - 5) {
        warnings.push(`"${label(h, '页眉')}" 底部与页码区域可能碰撞`)
        break
      }
    }
    for (const pn of enabledPnHeader) {
      const pnBottom = (pn.marginMm || 10) + (pn.fontSize || 9) * 0.4
      if (Math.abs(headerBottom - pnBottom) <= 5) {
        warnings.push(`"${label(h, '页眉')}" 与页眉区页码"${label(pn, '页码')}"可能重叠`)
      }
    }
  }
  // Footer text vs page number (±3mm), all group combinations
  for (const ft of enabledFooterText) {
    for (const pn of enabledPnFooter) {
      if (Math.abs((ft.marginMm || 10) - (pn.marginMm || 10)) <= 3) {
        warnings.push(`"${label(ft, '页脚文字')}" 与 "${label(pn, '页码')}" 距底距离接近，可能重叠`)
      }
    }
  }
  return warnings
})

function groupModeLabel(group) {
  return MODE_LABELS[group.mode] || group.mode
}

function addGroup() {
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
    pageStart: 1,
    pageEnd: 0,
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
    pageStart: 1,
    pageEnd: 0,
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
.page-range-row {
  grid-column: 1 / -1;
}
.page-range-inputs {
  display: flex;
  align-items: center;
  gap: 4px;
}
.page-range-inputs .el-input-number {
  width: 100px;
}
.range-sep {
  color: var(--docsy-text-muted);
  font-size: 13px;
}
.text-input-with-info {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: nowrap;
}
.text-input-with-info .el-input {
  flex: 1;
}
.text-input-with-info .info-icon {
  flex-shrink: 0;
  font-size: 16px;
  color: var(--docsy-text-muted);
  cursor: pointer;
  transition: color 0.15s;
}
.text-input-with-info .info-icon:hover {
  color: var(--docsy-text-strong);
}
.template-help {
  max-width: 280px;
  font-size: 12px;
  line-height: 1.6;
}
.template-help p {
  margin: 2px 0;
}
.template-help code {
  background: rgba(255, 255, 255, 0.1);
  padding: 0 3px;
  border-radius: 3px;
  font-size: 11px;
}
</style>
