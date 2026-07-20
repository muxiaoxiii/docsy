<template>
  <div>
    <div class="rule-item">
      <label>页眉来源</label>
      <el-select v-model="headerModeModel">
        <el-option label="不插入页眉" value="none" />
        <el-option label="文件名" value="filename" />
        <el-option label="按证据列表名称" value="per_file" />
        <el-option label="固定文本" value="custom" />
      </el-select>
    </div>
    <div class="rule-item" v-if="headerMode === 'custom'">
      <label>页眉文本</label>
      <el-input v-model="headerTextModel" placeholder="可写固定文字，也可用 [##]、[序号]、[文件名]、[YYYYMMDD]" />
    </div>
    <div class="rule-item">
      <label>页眉前缀</label>
      <el-input v-model="headerPrefixModel" placeholder="例如 证据[##]、[YYYYMMDD]" :disabled="headerMode === 'none'" />
    </div>
    <div class="rule-item">
      <label>页眉后缀</label>
      <el-input v-model="headerSuffixModel" placeholder="例如 -[##]、[YYYYMMDD]" :disabled="headerMode === 'none'" />
    </div>
    <div class="rule-item">
      <label>页眉位置</label>
      <el-select v-model="headerAlignModel" :disabled="headerMode === 'none'">
        <el-option label="居中" value="center" />
        <el-option label="左侧" value="left" />
        <el-option label="右侧" value="right" />
      </el-select>
    </div>
    <div class="rule-item">
      <label>页眉字号</label>
      <el-input-number v-model="headerFontSizeModel" :min="6" :max="24" :step="1" :disabled="headerMode === 'none'" />
    </div>
    <div class="rule-item">
      <label>页眉字体</label>
      <el-select v-model="headerFontFamilyModel" :disabled="headerMode === 'none'">
        <el-option label="自动" value="auto" />
        <el-option label="宋体" value="songti" />
        <el-option label="黑体" value="heiti" />
        <el-option label="楷体" value="kaiti" />
        <el-option label="仿宋" value="fangsong" />
        <el-option label="Helvetica" value="helvetica" />
        <el-option label="Times" value="times" />
        <el-option label="Courier" value="courier" />
      </el-select>
    </div>
    <div class="rule-item">
      <label>页眉距顶 mm</label>
      <el-input-number v-model="headerMarginMmModel" :min="3" :max="60" :step="1" :disabled="headerMode === 'none'" />
    </div>
    <div class="rule-item">
      <label>页眉水平偏移 mm</label>
      <el-input-number
        v-model="headerOffsetXMmModel"
        :min="-offsetLimitMm"
        :max="offsetLimitMm"
        :step="1"
        :disabled="headerMode === 'none'"
      />
    </div>
    <div class="rule-item">
      <label>页眉颜色</label>
      <el-color-picker v-model="headerColorModel" :disabled="headerMode === 'none'" />
    </div>
    <div class="rule-item">
      <label>页脚页码</label>
      <el-switch v-model="footerEnabledModel" active-text="启用" inactive-text="关闭" />
    </div>
    <div class="rule-item" v-if="showFooterContinuous">
      <label>页码方式</label>
      <el-select v-model="footerContinuousModel" :disabled="!footerEnabled">
        <el-option :value="true" label="拼接连续页码" />
        <el-option :value="false" label="每个文件单独页码" />
      </el-select>
    </div>
    <div class="rule-item">
      <label>页脚格式</label>
      <el-input v-model="footerTextModel" :disabled="!footerEnabled" />
    </div>
    <div class="rule-item">
      <label>页脚位置</label>
      <el-select v-model="footerAlignModel" :disabled="!footerEnabled">
        <el-option label="居中" value="center" />
        <el-option label="左侧" value="left" />
        <el-option label="右侧" value="right" />
      </el-select>
    </div>
    <div class="rule-item">
      <label>页脚字号</label>
      <el-input-number v-model="footerFontSizeModel" :min="6" :max="24" :step="1" :disabled="!footerEnabled" />
    </div>
    <div class="rule-item">
      <label>页脚字体</label>
      <el-select v-model="footerFontFamilyModel" :disabled="!footerEnabled">
        <el-option label="自动" value="auto" />
        <el-option label="宋体" value="songti" />
        <el-option label="黑体" value="heiti" />
        <el-option label="楷体" value="kaiti" />
        <el-option label="仿宋" value="fangsong" />
        <el-option label="Helvetica" value="helvetica" />
        <el-option label="Times" value="times" />
        <el-option label="Courier" value="courier" />
      </el-select>
    </div>
    <div class="rule-item">
      <label>页脚距底 mm</label>
      <el-input-number v-model="footerMarginMmModel" :min="3" :max="60" :step="1" :disabled="!footerEnabled" />
    </div>
    <div class="rule-item">
      <label>页脚水平偏移 mm</label>
      <el-input-number
        v-model="footerOffsetXMmModel"
        :min="-offsetLimitMm"
        :max="offsetLimitMm"
        :step="1"
        :disabled="!footerEnabled"
      />
    </div>
    <div class="rule-item">
      <label>页脚颜色</label>
      <el-color-picker v-model="footerColorModel" :disabled="!footerEnabled" />
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

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
  footerEnabled: { type: Boolean, required: true },
  footerContinuous: { type: Boolean, required: true },
  footerText: { type: String, required: true },
  footerAlign: { type: String, required: true },
  footerFontSize: { type: Number, required: true },
  footerFontFamily: { type: String, required: true },
  footerMarginMm: { type: Number, required: true },
  footerOffsetXMm: { type: Number, required: true },
  footerColor: { type: String, required: true },
  showFooterContinuous: { type: Boolean, default: false },
  offsetLimitMm: { type: Number, default: 120 },
})

const emit = defineEmits([
  'update:headerMode',
  'update:headerText',
  'update:headerPrefix',
  'update:headerSuffix',
  'update:headerAlign',
  'update:headerFontSize',
  'update:headerFontFamily',
  'update:headerMarginMm',
  'update:headerOffsetXMm',
  'update:headerColor',
  'update:footerEnabled',
  'update:footerContinuous',
  'update:footerText',
  'update:footerAlign',
  'update:footerFontSize',
  'update:footerFontFamily',
  'update:footerMarginMm',
  'update:footerOffsetXMm',
  'update:footerColor',
])

function model(key) {
  return computed({
    get: () => props[key],
    set: (value) => emit(`update:${key}`, value),
  })
}

const headerModeModel = model('headerMode')
const headerTextModel = model('headerText')
const headerPrefixModel = model('headerPrefix')
const headerSuffixModel = model('headerSuffix')
const headerAlignModel = model('headerAlign')
const headerFontSizeModel = model('headerFontSize')
const headerFontFamilyModel = model('headerFontFamily')
const headerMarginMmModel = model('headerMarginMm')
const headerOffsetXMmModel = model('headerOffsetXMm')
const headerColorModel = model('headerColor')
const footerEnabledModel = model('footerEnabled')
const footerContinuousModel = model('footerContinuous')
const footerTextModel = model('footerText')
const footerAlignModel = model('footerAlign')
const footerFontSizeModel = model('footerFontSize')
const footerFontFamilyModel = model('footerFontFamily')
const footerMarginMmModel = model('footerMarginMm')
const footerOffsetXMmModel = model('footerOffsetXMm')
const footerColorModel = model('footerColor')
</script>
