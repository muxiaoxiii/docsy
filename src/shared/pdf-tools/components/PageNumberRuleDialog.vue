<template>
  <el-dialog v-model="visibleModel" title="页码分段与例外" width="min(980px, 94vw)" append-to-body>
    <div class="dialog-head">
      <span>规则按列表顺序应用，后面的规则覆盖前面的规则。</span>
      <el-button type="primary" size="small" @click="addRule">添加规则</el-button>
    </div>
    <el-table :data="localRules" border size="small" max-height="58vh">
      <el-table-column label="范围依据" width="120"
        ><template #default="{ row }"
          ><el-select v-model="row.scope"
            ><el-option label="合并后页码" value="global" /><el-option
              label="每个文件内"
              value="file" /></el-select></template
      ></el-table-column>
      <el-table-column label="起始页" width="105"
        ><template #default="{ row }"
          ><el-input-number v-model="row.start" :min="1" controls-position="right" /></template
      ></el-table-column>
      <el-table-column label="结束页" width="105"
        ><template #default="{ row }"
          ><el-input-number v-model="row.end" :min="row.start || 1" controls-position="right" /></template
      ></el-table-column>
      <el-table-column label="处理" width="115"
        ><template #default="{ row }"
          ><el-select v-model="row.action"
            ><el-option label="不插入页码" value="exclude" /><el-option
              label="覆盖全局规则"
              value="override" /></el-select></template
      ></el-table-column>
      <el-table-column label="样式" min-width="140"
        ><template #default="{ row }"
          ><el-select v-model="row.style" :disabled="row.action === 'exclude'"
            ><el-option
              v-for="style in PAGE_NUMBER_STYLES"
              :key="style.value"
              :label="style.label"
              :value="style.value" /></el-select></template
      ></el-table-column>
      <el-table-column label="格式" min-width="140"
        ><template #default="{ row }"
          ><el-input
            v-model="row.template"
            :disabled="row.action === 'exclude'"
            placeholder="{page}/{total}" /></template
      ></el-table-column>
      <el-table-column label="位置" width="105"
        ><template #default="{ row }"
          ><el-select v-model="row.align" :disabled="row.action === 'exclude'"
            ><el-option label="左" value="left" /><el-option label="中" value="center" /><el-option
              label="右"
              value="right" /></el-select></template
      ></el-table-column>
      <el-table-column label="编号偏移" width="115"
        ><template #default="{ row }"
          ><el-input-number
            v-model="row.startOffset"
            :disabled="row.action === 'exclude'"
            controls-position="right" /></template
      ></el-table-column>
      <el-table-column label="操作" width="66" fixed="right"
        ><template #default="{ $index }"
          ><el-button link type="danger" @click="localRules.splice($index, 1)">删除</el-button></template
        ></el-table-column
      >
    </el-table>
    <el-empty v-if="!localRules.length" description="没有例外规则，将按全局页码设置处理全部页面" :image-size="54" />
    <template #footer><el-button @click="visibleModel = false">完成</el-button></template>
  </el-dialog>
</template>
<script setup>
import { computed, ref, watch } from 'vue'
import { PAGE_NUMBER_STYLES } from '../composables/pdfPageNumberRules.js'
const props = defineProps({ visible: { type: Boolean, required: true }, rules: { type: Array, required: true } })
const emit = defineEmits(['update:visible', 'update:rules'])
const localRules = ref([])
const visibleModel = computed({ get: () => props.visible, set: (value) => emit('update:visible', value) })
function addRule() {
  localRules.value.push({
    scope: 'global',
    start: 1,
    end: 1,
    action: 'exclude',
    style: 'arabic',
    template: '{page}/{total}',
    align: 'center',
    startOffset: 0,
  })
}
watch(
  () => props.visible,
  () => {
    localRules.value = cloneRules(props.rules)
  },
  { immediate: true },
)
watch(localRules, (value) => emit('update:rules', cloneRules(value)), { deep: true })
function cloneRules(value) {
  return JSON.parse(JSON.stringify(value || []))
}
</script>
<style scoped>
.dialog-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  color: var(--docsy-text-muted);
  font-size: 13px;
}
</style>
