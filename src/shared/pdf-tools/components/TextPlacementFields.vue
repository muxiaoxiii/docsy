<template>
  <div class="rule-item">
    <label>{{ prefix }}位置</label
    ><el-select v-model="alignModel" :disabled="disabled"
      ><el-option label="居中" value="center" /><el-option label="左侧" value="left" /><el-option
        label="右侧"
        value="right"
    /></el-select>
  </div>
  <div class="rule-item">
    <label>{{ prefix }}字号</label><el-input-number v-model="fontSizeModel" :min="6" :max="24" :disabled="disabled" />
  </div>
  <div class="rule-item">
    <label>{{ prefix }}字体</label
    ><el-select v-model="fontFamilyModel" :disabled="disabled"
      ><el-option label="自动" value="auto" /><el-option label="宋体" value="songti" /><el-option
        label="黑体"
        value="heiti" /><el-option label="楷体" value="kaiti" /><el-option label="仿宋" value="fangsong" /><el-option
        label="Helvetica"
        value="helvetica" /><el-option label="Times" value="times" /><el-option label="Courier" value="courier"
    /></el-select>
  </div>
  <div class="rule-item">
    <label>{{ prefix }}{{ marginLabel }} mm</label
    ><el-input-number v-model="marginMmModel" :min="0" :max="80" :step="0.5" :disabled="disabled" />
  </div>
  <div class="rule-item">
    <label>{{ prefix }}水平偏移 mm</label
    ><el-input-number v-model="offsetXModel" :min="-offsetLimitMm" :max="offsetLimitMm" :disabled="disabled" />
  </div>
  <div class="rule-item">
    <label>{{ prefix }}颜色</label><el-color-picker v-model="colorModel" :disabled="disabled" />
  </div>
</template>
<script setup>
import { computed } from 'vue'
const props = defineProps({
  prefix: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
  align: { type: String, default: 'center' },
  fontSize: { type: Number, default: 9 },
  fontFamily: { type: String, default: 'auto' },
  marginMm: { type: Number, default: 10 },
  offsetXMm: { type: Number, default: 0 },
  color: { type: String, default: '#000000' },
  offsetLimitMm: { type: Number, default: 120 },
  marginLabel: { type: String, default: '距边' },
})
const emit = defineEmits([
  'update:align',
  'update:fontSize',
  'update:fontFamily',
  'update:marginMm',
  'update:offsetXMm',
  'update:color',
])
const bind = (key, event) => computed({ get: () => props[key], set: (value) => emit(event, value) })
const alignModel = bind('align', 'update:align'),
  fontSizeModel = bind('fontSize', 'update:fontSize'),
  fontFamilyModel = bind('fontFamily', 'update:fontFamily'),
  marginMmModel = bind('marginMm', 'update:marginMm'),
  offsetXModel = bind('offsetXMm', 'update:offsetXMm'),
  colorModel = bind('color', 'update:color')
</script>
