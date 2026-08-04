<template>
  <div>
    <div class="rule-item section-label">
      <strong>页眉文字</strong>
      <el-switch v-model="headerInsertEnabledModel" size="small" />
    </div>
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
      <el-switch v-model="footerInsertEnabledModel" size="small" />
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
      <el-switch v-model="pageNumberEnabledModel" size="small" />
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

const props = defineProps({
  headerMode: { type: String, required: true },
  headerInsertEnabled: { type: Boolean, default: true },
  headerText: { type: String, required: true },
  headerPrefix: { type: String, required: true },
  headerSuffix: { type: String, required: true },
  headerAlign: { type: String, required: true },
  headerFontSize: { type: Number, required: true },
  headerFontFamily: { type: String, required: true },
  headerMarginMm: { type: Number, required: true },
  headerOffsetXMm: { type: Number, required: true },
  headerColor: { type: String, required: true },
  footerInsertEnabled: { type: Boolean, default: true },
  footerTextContent: { type: String, required: true },
  footerTextAlign: { type: String, required: true },
  footerTextFontSize: { type: Number, required: true },
  footerTextFontFamily: { type: String, required: true },
  footerTextMarginMm: { type: Number, required: true },
  footerTextOffsetXMm: { type: Number, required: true },
  footerTextColor: { type: String, required: true },
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