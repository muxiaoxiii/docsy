<template>
  <el-dialog v-model="visibleModel" title="插入规则高级设置" width="min(1080px, 96vw)" append-to-body>
    <div class="dialog-head">
      <span>这里设置规则的文件范围、页段和页码例外；编号起始在主面板各规则内设置，未覆盖的内容跟随主规则。</span>
      <el-button v-if="activeKind === 'pageNumber'" type="primary" size="small" :disabled="!selectedGroup" @click="addException">添加页码例外</el-button>
    </div>

    <el-segmented v-model="activeKind" :options="kindOptions" class="kind-switch" />

    <div class="rule-cards">
      <button
        v-for="(group, index) in activeGroups"
        :key="group.id"
        type="button"
        class="rule-card"
        :class="{ active: group.id === activeSelectedId }"
        @click="activeSelectedId = group.id"
      >
        <strong>{{ group.label || `${activeKind === 'header' ? '页眉' : activeKind === 'footer' ? '页脚文字' : '页码'} ${index + 1}` }}</strong>
        <span>{{ activeKind === 'pageNumber' ? group.template || '{page}/{total}' : group.text || group.mode || '规则' }}</span>
        <small>{{ activeKind === 'pageNumber' ? `${(group.exceptions || []).length} 条例外` : groupRangeText(group) }}</small>
      </button>
    </div>

    <template v-if="selectedGroup && activeKind !== 'pageNumber'">
      <div class="text-rule-scope">
        <div class="rule-item"><label>指定文件</label><el-select v-model="selectedGroup.fileIds" multiple collapse-tags clearable placeholder="全部文件"><el-option v-for="file in files" :key="fileId(file)" :label="file.name" :value="fileId(file)" /></el-select></div>
        <div class="rule-item"><label>起始页</label><el-input-number v-model="selectedGroup.pageStart" :min="1" controls-position="right" /></div>
        <div class="rule-item"><label>结束页</label><el-input-number v-model="selectedGroup.pageEnd" :min="0" controls-position="right" /><span class="inherit-text">0 表示到文件末页</span></div>
        <div v-if="activeKind === 'header' && selectedGroup.mode === 'per_file'" class="rule-item"><label>证据序号</label><el-select v-model="selectedGroup.numbering.source" @change="ensureCustomNumbering(selectedGroup)"><el-option label="跟随全局" value="default" /><el-option label="单独设置" value="custom" /></el-select><el-input-number v-if="selectedGroup.numbering.source === 'custom'" v-model="selectedGroup.numbering.evidenceStart" :min="0" controls-position="right" /></div>
      </div>
    </template>
    <template v-else-if="selectedGroup">
      <div class="page-rule-base">
        <div class="rule-item"><label>指定文件</label><el-select v-model="selectedGroup.fileIds" multiple collapse-tags clearable placeholder="全部文件"><el-option v-for="file in files" :key="fileId(file)" :label="file.name" :value="fileId(file)" /></el-select></div>
      </div>
      <el-table :data="selectedExceptions" border size="small" max-height="58vh">
        <el-table-column label="范围依据" width="132">
          <template #default="{ row }">
            <el-select v-model="row.scope.type">
              <el-option label="合并后页码" value="global" />
              <el-option label="文件内页码" value="file" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="指定文件" min-width="170">
          <template #default="{ row }">
            <el-select v-model="row.scope.fileIds" multiple collapse-tags clearable placeholder="全部文件">
              <el-option v-for="file in files" :key="fileId(file)" :label="file.name" :value="fileId(file)" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="起始页" width="105">
          <template #default="{ row }">
            <el-input-number v-model="row.scope.start" :min="1" controls-position="right" />
          </template>
        </el-table-column>
        <el-table-column label="结束页" width="105">
          <template #default="{ row }">
            <el-input-number v-model="row.scope.end" :min="row.scope.start || 1" controls-position="right" />
          </template>
        </el-table-column>
        <el-table-column label="处理" width="132">
          <template #default="{ row }">
            <el-select :model-value="exceptionAction(row)" @change="setExceptionAction(row, $event)">
              <el-option label="不显示页码" value="hide" />
              <el-option label="覆盖主规则" value="override" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="隐藏页计数" width="112">
          <template #default="{ row }">
            <el-checkbox
              :model-value="row.overrides.count !== false"
              :disabled="exceptionAction(row) !== 'hide'"
              @change="row.overrides.count = $event"
            >计数</el-checkbox>
          </template>
        </el-table-column>
        <el-table-column label="覆盖内容" min-width="230">
          <template #default="{ row }">
            <div v-if="exceptionAction(row) === 'override'" class="override-fields">
              <el-select v-model="row.overrides.style" clearable placeholder="样式跟随">
                <el-option v-for="style in PAGE_NUMBER_STYLES" :key="style.value" :label="style.label" :value="style.value" />
              </el-select>
              <el-input v-model="row.overrides.template" clearable placeholder="格式跟随主规则" />
              <el-select v-model="row.overrides.align" clearable placeholder="位置跟随">
                <el-option label="左" value="left" /><el-option label="中" value="center" /><el-option label="右" value="right" />
              </el-select>
              <el-input-number v-model="row.overrides.startOffset" controls-position="right" placeholder="偏移" />
            </div>
            <span v-else class="inherit-text">其余属性跟随主规则</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="66" fixed="right">
          <template #default="{ $index }"><el-button link type="danger" @click="selectedExceptions.splice($index, 1)">删除</el-button></template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!selectedExceptions.length" description="此页码规则没有例外，将按主规则处理全部页面" :image-size="54" />
    </template>
    <el-empty v-else description="请先添加页码规则" :image-size="54" />

    <template #footer><el-button @click="visibleModel = false">完成</el-button></template>
  </el-dialog>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { PAGE_NUMBER_STYLES, normalizePageNumberException } from '../composables/pdfPageNumberRules.js'

const props = defineProps({
  visible: { type: Boolean, required: true },
  groups: { type: Array, required: true },
  selectedGroupId: { type: String, default: '' },
  headerGroups: { type: Array, default: () => [] },
  footerGroups: { type: Array, default: () => [] },
  selectedHeaderGroupId: { type: String, default: '' },
  selectedFooterGroupId: { type: String, default: '' },
  numberingDefaults: { type: Object, default: () => ({ evidenceStart: 1, pageStart: 1 }) },
  files: { type: Array, default: () => [] },
})
const emit = defineEmits(['update:visible', 'update:groups', 'update:selectedGroupId', 'update:headerGroups', 'update:footerGroups', 'update:selectedHeaderGroupId', 'update:selectedFooterGroupId'])
const localGroups = ref([])
const localHeaderGroups = ref([])
const localFooterGroups = ref([])
const localSelectedId = ref('')
const localSelectedHeaderId = ref('')
const localSelectedFooterId = ref('')
const activeKind = ref('pageNumber')
const kindOptions = [{ label: '页眉', value: 'header' }, { label: '页脚文字', value: 'footer' }, { label: '页码', value: 'pageNumber' }]
const visibleModel = computed({ get: () => props.visible, set: (value) => emit('update:visible', value) })
const activeGroups = computed(() => activeKind.value === 'header' ? localHeaderGroups.value : activeKind.value === 'footer' ? localFooterGroups.value : localGroups.value)
const activeSelectedId = computed({
  get: () => activeKind.value === 'header' ? localSelectedHeaderId.value : activeKind.value === 'footer' ? localSelectedFooterId.value : localSelectedId.value,
  set: (value) => { if (activeKind.value === 'header') localSelectedHeaderId.value = value; else if (activeKind.value === 'footer') localSelectedFooterId.value = value; else localSelectedId.value = value },
})
const selectedGroup = computed(() => activeGroups.value.find((group) => group.id === activeSelectedId.value) || activeGroups.value[0] || null)
const selectedExceptions = computed(() => selectedGroup.value?.exceptions || [])

function fileId(file) { return String(file?.id || file?.path || '') }
function groupRangeText(group) { return group.fileIds?.length ? `${group.fileIds.length} 个文件` : group.pageEnd > 0 ? `第 ${group.pageStart || 1}-${group.pageEnd} 页` : '全部页面' }
function ensureCustomNumbering(group) {
  if (group.numbering.source !== 'custom') return
  group.numbering.evidenceStart ??= Number(props.numberingDefaults.evidenceStart || 1)
}
function clone(value) { return JSON.parse(JSON.stringify(value || [])) }
function cleanOverrides(value) {
  return Object.fromEntries(Object.entries(value || {}).filter(([, item]) => item !== '' && item !== null && item !== undefined))
}
function normalizeGroups(groups) {
  return clone(groups).map((group) => ({
    ...group,
    fileIds: [...(group.fileIds || [])],
    numbering: { source: 'default', ...(group.numbering || {}) },
    exceptions: (group.exceptions || group.overrides || []).map((entry, index) => {
      const normalized = normalizePageNumberException(entry, index)
      normalized.overrides = cleanOverrides(normalized.overrides)
      return normalized
    }),
    overrides: undefined,
  }))
}
function addException() {
  if (!selectedGroup.value) return
  selectedGroup.value.exceptions.push(normalizePageNumberException({
    id: `exception-${Date.now()}`,
    scope: { type: 'global', start: 1, end: 1, fileIds: [] },
    overrides: { enabled: false, count: true },
  }))
}
function exceptionAction(row) { return row.overrides.enabled === false ? 'hide' : 'override' }
function setExceptionAction(row, value) {
  if (value === 'hide') {
    row.overrides = { enabled: false, count: row.overrides.count !== false }
  } else {
    row.overrides = { style: row.overrides.style || '', template: row.overrides.template || '', align: row.overrides.align || '', startOffset: Number(row.overrides.startOffset || 0) }
  }
}

watch(() => props.visible, (visible) => {
  if (!visible) return
  localGroups.value = normalizeGroups(props.groups)
  localHeaderGroups.value = clone(props.headerGroups).map((group) => ({ ...group, fileIds: [...(group.fileIds || [])], numbering: { source: 'default', ...(group.numbering || {}) } }))
  localFooterGroups.value = clone(props.footerGroups).map((group) => ({ ...group, fileIds: [...(group.fileIds || [])] }))
  localSelectedId.value = props.selectedGroupId || localGroups.value[0]?.id || ''
  localSelectedHeaderId.value = props.selectedHeaderGroupId || localHeaderGroups.value[0]?.id || ''
  localSelectedFooterId.value = props.selectedFooterGroupId || localFooterGroups.value[0]?.id || ''
}, { immediate: true })
watch(localSelectedId, (value) => emit('update:selectedGroupId', value))
watch(localSelectedHeaderId, (value) => emit('update:selectedHeaderGroupId', value))
watch(localSelectedFooterId, (value) => emit('update:selectedFooterGroupId', value))
watch(activeKind, () => {
  if (!activeGroups.value.some((group) => group.id === activeSelectedId.value)) {
    activeSelectedId.value = activeGroups.value[0]?.id || ''
  }
})
watch(localGroups, (value) => emit('update:groups', normalizeGroups(value)), { deep: true })
watch(localHeaderGroups, (value) => emit('update:headerGroups', clone(value)), { deep: true })
watch(localFooterGroups, (value) => emit('update:footerGroups', clone(value)), { deep: true })
</script>

<style scoped>
.dialog-head, .rule-cards, .override-fields { display: flex; align-items: center; gap: 10px; }
.kind-switch { margin-bottom: 12px; }
.dialog-head { justify-content: space-between; margin-bottom: 12px; color: var(--docsy-text-muted); font-size: 13px; }
.rule-cards { overflow-x: auto; padding: 2px 0 12px; }
.rule-card { min-width: 150px; padding: 9px 11px; border: 1px solid var(--docsy-border-subtle); background: var(--docsy-surface-soft); color: var(--docsy-text); border-radius: 6px; text-align: left; cursor: pointer; }
.rule-card.active { border-color: var(--el-color-primary); background: var(--el-color-primary-light-9); }
.rule-card strong, .rule-card span, .rule-card small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.rule-card span, .rule-card small, .inherit-text { color: var(--docsy-text-muted); font-size: 12px; }
.override-fields { flex-wrap: wrap; }
.override-fields > * { width: 104px; }
.override-fields .el-input { width: 180px; }
.text-rule-scope { display: grid; grid-template-columns: minmax(220px, 2fr) repeat(2, minmax(130px, 1fr)) minmax(180px, 1fr); gap: 12px; padding: 14px; border: 1px solid var(--docsy-border-subtle); background: var(--docsy-surface-soft); }
.page-rule-base { display: flex; align-items: end; flex-wrap: wrap; gap: 10px; margin-bottom: 12px; padding: 12px; border: 1px solid var(--docsy-border-subtle); background: var(--docsy-surface-soft); }
.page-rule-base .rule-item { min-width: 145px; }
.page-rule-base .rule-item:first-child { min-width: 220px; }
.rule-item { display: flex; flex-direction: column; gap: 6px; }
.rule-item label { color: var(--docsy-text-muted); font-size: 12px; }
</style>
