<template>
  <div>
    <div class="rule-item section-label"><strong>页眉文字</strong></div>
    <div class="rule-item">
      <label>页眉来源</label>
      <el-select v-model="headerModeModel">
        <el-option label="不插入页眉" value="none" />
        <el-option label="文件名" value="filename" />
        <el-option label="按证据列表名称" value="per_file" />
        <el-option label="固定文本" value="custom" />
      </el-select>
    </div>
    <div v-if="headerMode === 'custom'" class="rule-item">
      <label>页眉文本</label>
      <el-input v-model="headerTextModel" placeholder="可用 [##]、[序号]、[文件名]、[YYYYMMDD]" />
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

    <div class="rule-item section-label"><strong>页脚文字</strong></div>
    <div class="rule-item">
      <label>插入页脚文字</label>
      <el-switch v-model="footerTextEnabledModel" active-text="启用" inactive-text="关闭" />
    </div>
    <div class="rule-item">
      <label>页脚文本</label>
      <el-input v-model="footerTextContentModel" :disabled="!footerTextEnabled" placeholder="固定文字，不用于页码" />
    </div>
    <TextPlacementFields
      prefix="页脚"
      :disabled="!footerTextEnabled"
      v-model:align="footerTextAlignModel"
      v-model:font-size="footerTextFontSizeModel"
      v-model:font-family="footerTextFontFamilyModel"
      v-model:margin-mm="footerTextMarginMmModel"
      v-model:offset-x-mm="footerTextOffsetXMmModel"
      v-model:color="footerTextColorModel"
      :offset-limit-mm="offsetLimitMm"
      margin-label="距底"
    />

    <div class="rule-item section-label"><strong>页码</strong></div>
    <div class="rule-item">
      <label>插入页码</label>
      <el-switch v-model="pageNumberEnabledModel" active-text="启用" inactive-text="关闭" />
    </div>
    <div class="rule-item">
      <label>连续方式</label>
      <el-select v-model="pageNumberSequenceModel" :disabled="!pageNumberEnabled">
        <el-option label="全部文件连续" value="continuous" />
        <el-option label="每个文件单独编号" value="per-file" />
      </el-select>
    </div>
    <div class="rule-item">
      <label>数字样式</label>
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
      <label>页码格式</label>
      <el-input
        v-model="pageNumberTemplateModel"
        :disabled="!pageNumberEnabled"
        placeholder="例如 {page}/{total}、-{page}-"
      />
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
      <span class="field-hint">可排除首页，或让指定页段使用不同样式、位置和起始编号</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import TextPlacementFields from './TextPlacementFields.vue'
import { PAGE_NUMBER_STYLES } from '../composables/pdfPageNumberRules.js'

const props = defineProps({
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
  footerTextEnabled: { type: Boolean, required: true },
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
  offsetLimitMm: { type: Number, default: 120 },
})

const emit = defineEmits([
  'editPageNumberRules',
  ...[
    'headerMode',
    'headerText',
    'headerPrefix',
    'headerSuffix',
    'headerAlign',
    'headerFontSize',
    'headerFontFamily',
    'headerMarginMm',
    'headerOffsetXMm',
    'headerColor',
    'footerTextEnabled',
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
</script>

<style scoped>
.section-label {
  grid-column: 1 / -1;
  margin-top: 4px;
  padding-top: 8px;
  border-top: 1px solid var(--docsy-border-subtle);
}
</style>
