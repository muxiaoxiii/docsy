<template>
  <el-dialog v-model="visibleModel" title="分段与例外" width="min(1080px, 96vw)" append-to-body>
    <div class="dialog-head">
      <span>例外按勾选类型生效，按列表顺序应用、后面的覆盖前面的；编号起始在主面板各规则内设置。</span>
      <el-button type="primary" size="small" @click="addException">添加例外</el-button>
    </div>

    <!-- 规则基础范围：下拉选择规则，编辑它的指定文件/页段/证据序号 -->
    <div class="section-caption">规则基础范围</div>
    <div v-if="selectedCard" class="base-scope">
      <div class="rule-item">
        <label>规则</label>
        <el-select :model-value="selectedCardId" @change="selectedCardId = $event">
          <el-option v-for="card in cards" :key="card.group.id" :value="card.group.id" :label="cardOptionLabel(card)" />
        </el-select>
      </div>
      <div class="rule-item">
        <label>指定文件</label>
        <el-select v-model="selectedCard.group.fileIds" multiple collapse-tags clearable placeholder="全部文件">
          <el-option v-for="file in files" :key="fileId(file)" :label="file.name" :value="fileId(file)" />
        </el-select>
      </div>
      <template v-if="selectedCard.kind !== 'pageNumber'">
        <div class="rule-item">
          <label>起始页</label>
          <el-input-number v-model="selectedCard.group.pageStart" :min="1" controls-position="right" />
        </div>
        <div class="rule-item">
          <label>结束页</label>
          <el-input-number v-model="selectedCard.group.pageEnd" :min="0" controls-position="right" />
          <span class="inherit-text">0 表示到文件末页</span>
        </div>
      </template>
      <div v-if="selectedCard.kind === 'header' && selectedCard.group.mode === 'per_file'" class="rule-item">
        <label>证据序号</label>
        <el-select v-model="selectedCard.group.numbering.source" @change="ensureCustomNumbering(selectedCard.group)">
          <el-option label="跟随全局" value="default" />
          <el-option label="单独设置" value="custom" />
        </el-select>
        <el-input-number
          v-if="selectedCard.group.numbering.source === 'custom'"
          v-model="selectedCard.group.numbering.evidenceStart"
          :min="0"
          controls-position="right"
        />
      </div>
    </div>

    <div class="section-caption">例外</div>

    <!-- 统一例外列表：每行两条，第一行定范围与动作，第二行按勾选类型给覆盖字段 -->
    <div v-if="localExceptions.length" class="exception-list">
      <div v-for="row in localExceptions" :key="row.id" class="exception-row">
        <div class="exception-row-main">
          <el-checkbox-group :model-value="row.kinds" @change="setKinds(row, $event)">
            <el-checkbox value="header">页眉</el-checkbox>
            <el-checkbox value="footerText">页脚文字</el-checkbox>
            <el-checkbox value="pageNumber">页码</el-checkbox>
          </el-checkbox-group>
          <el-select v-model="row.scope.type" class="scope-type">
            <el-option label="合并后页码" value="global" />
            <el-option label="文件内页码" value="file" />
          </el-select>
          <el-select
            v-model="row.scope.fileIds"
            multiple
            collapse-tags
            clearable
            placeholder="全部文件"
            class="scope-files"
          >
            <el-option v-for="file in files" :key="fileId(file)" :label="file.name" :value="fileId(file)" />
          </el-select>
          <span class="num-field"
            ><el-input-number v-model="row.scope.start" :min="1" controls-position="right"
          /></span>
          <span class="range-sep">–</span>
          <span class="num-field"
            ><el-input-number v-model="row.scope.end" :min="row.scope.start || 1" controls-position="right"
          /></span>
          <el-select
            :model-value="exceptionAction(row)"
            class="action-select"
            @change="setExceptionAction(row, $event)"
          >
            <el-option label="不显示" value="hide" />
            <el-option label="覆盖主规则" value="override" />
          </el-select>
          <el-button link type="danger" @click="removeException(row)">删除</el-button>
        </div>
        <div class="exception-row-detail">
          <template v-if="exceptionAction(row) === 'hide'">
            <span class="inherit-text">所选类型在范围内不显示</span>
            <el-checkbox
              v-if="row.kinds.includes('pageNumber')"
              :model-value="row.overrides.count !== false"
              @change="row.overrides.count = $event"
              >隐藏页仍计数</el-checkbox
            >
          </template>
          <template v-else>
            <el-input
              v-if="row.kinds.includes('header')"
              v-model="row.overrides.headerText"
              clearable
              placeholder="页眉文本：跟随主规则"
              class="text-field"
            />
            <el-input
              v-if="row.kinds.includes('footerText')"
              v-model="row.overrides.footerText"
              clearable
              placeholder="页脚文本：跟随主规则"
              class="text-field"
            />
            <template v-if="row.kinds.includes('pageNumber')">
              <el-select v-model="row.overrides.style" clearable placeholder="页码样式跟随" class="mini-field">
                <el-option
                  v-for="style in PAGE_NUMBER_STYLES"
                  :key="style.value"
                  :label="style.label"
                  :value="style.value"
                />
              </el-select>
              <el-input v-model="row.overrides.template" clearable placeholder="页码格式跟随" class="text-field" />
              <span class="num-field slim"
                ><el-input-number v-model="row.overrides.startOffset" placeholder="偏移" controls-position="right"
              /></span>
            </template>
            <el-select v-model="row.overrides.align" clearable placeholder="位置跟随" class="mini-field">
              <el-option label="左" value="left" />
              <el-option label="中" value="center" />
              <el-option label="右" value="right" />
            </el-select>
            <span class="num-field slim"
              ><el-input-number
                v-model="row.overrides.fontSize"
                :min="5"
                :max="72"
                placeholder="字号跟随"
                controls-position="right"
            /></span>
            <span class="color-field">
              <el-color-picker v-model="row.overrides.color" size="small" />
              <el-button v-if="row.overrides.color" link size="small" @click="row.overrides.color = ''"
                >颜色跟随</el-button
              >
            </span>
          </template>
        </div>
      </div>
    </div>
    <el-empty v-else description="没有例外，全部页面按各主规则处理" :image-size="54" />

    <template #footer><el-button @click="visibleModel = false">完成</el-button></template>
  </el-dialog>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { PAGE_NUMBER_STYLES, normalizeInsertException } from '../composables/pdfPageNumberRules.js'

const KIND_LABELS = { header: '页眉', footerText: '页脚文字', pageNumber: '页码' }

const props = defineProps({
  visible: { type: Boolean, required: true },
  exceptions: { type: Array, default: () => [] },
  groups: { type: Array, required: true },
  selectedGroupId: { type: String, default: '' },
  headerGroups: { type: Array, default: () => [] },
  footerGroups: { type: Array, default: () => [] },
  selectedHeaderGroupId: { type: String, default: '' },
  selectedFooterGroupId: { type: String, default: '' },
  numberingDefaults: { type: Object, default: () => ({ evidenceStart: 1, pageStart: 1 }) },
  files: { type: Array, default: () => [] },
})
const emit = defineEmits([
  'update:visible',
  'update:exceptions',
  'update:groups',
  'update:selectedGroupId',
  'update:headerGroups',
  'update:footerGroups',
  'update:selectedHeaderGroupId',
  'update:selectedFooterGroupId',
])

const localExceptions = ref([])
const localGroups = ref([])
const localHeaderGroups = ref([])
const localFooterGroups = ref([])
const selectedCardId = ref('')

const visibleModel = computed({ get: () => props.visible, set: (value) => emit('update:visible', value) })
const cards = computed(() => [
  ...localHeaderGroups.value.map((group) => ({ kind: 'header', group })),
  ...localFooterGroups.value.map((group) => ({ kind: 'footerText', group })),
  ...localGroups.value.map((group) => ({ kind: 'pageNumber', group })),
])
const selectedCard = computed(
  () => cards.value.find((card) => card.group.id === selectedCardId.value) || cards.value[0] || null,
)

function fileId(file) {
  return String(file?.id || file?.path || '')
}
function cardKindIndex(card) {
  const list =
    card.kind === 'header'
      ? localHeaderGroups.value
      : card.kind === 'footerText'
        ? localFooterGroups.value
        : localGroups.value
  return list.indexOf(card.group) + 1
}
function cardSummary(card) {
  const group = card.group
  if (card.kind === 'pageNumber') return group.template || '{page}/{total}'
  const scope = group.fileIds?.length
    ? `${group.fileIds.length} 个文件`
    : group.pageEnd > 0
      ? `第 ${group.pageStart || 1}-${group.pageEnd} 页`
      : '全部页面'
  return `${group.text || group.mode || '规则'} · ${scope}`
}
function cardOptionLabel(card) {
  const name = card.group.label || `${KIND_LABELS[card.kind]} ${cardKindIndex(card)}`
  return `${name} · ${cardSummary(card)}`
}
function ensureCustomNumbering(group) {
  if (group.numbering.source !== 'custom') return
  group.numbering.evidenceStart ??= Number(props.numberingDefaults.evidenceStart || 1)
}

function clone(value) {
  return JSON.parse(JSON.stringify(value ?? []))
}
function cleanOverrides(value) {
  return Object.fromEntries(
    Object.entries(value || {}).filter(([, item]) => item !== '' && item !== null && item !== undefined),
  )
}
function normalizeGroups(groups) {
  return clone(groups).map((group) => ({
    ...group,
    fileIds: [...(group.fileIds || [])],
    numbering: { source: 'default', ...(group.numbering || {}) },
    exceptions: [],
    overrides: undefined,
  }))
}
function serializeExceptions() {
  return clone(localExceptions.value).map((entry, index) => {
    const normalized = normalizeInsertException(entry, index)
    normalized.overrides = cleanOverrides(normalized.overrides)
    return normalized
  })
}

function addException() {
  localExceptions.value.push(
    normalizeInsertException({
      id: `exception-${Date.now()}`,
      kinds: ['pageNumber'],
      scope: { type: 'global', start: 1, end: 1, fileIds: [] },
      overrides: { enabled: false, count: true },
    }),
  )
}
function removeException(row) {
  localExceptions.value = localExceptions.value.filter((entry) => entry.id !== row.id)
}
function setKinds(row, value) {
  // 至少保留一个类型：取消最后一个勾选时忽略本次操作
  if (value.length) row.kinds = value
}
function exceptionAction(row) {
  return row.overrides.enabled === false ? 'hide' : 'override'
}
function setExceptionAction(row, value) {
  if (value === 'hide') {
    row.overrides = { enabled: false, count: row.overrides.count !== false }
  } else {
    row.overrides = { enabled: true }
  }
}

watch(
  () => props.visible,
  (visible) => {
    if (!visible) return
    localGroups.value = normalizeGroups(props.groups)
    localHeaderGroups.value = normalizeGroups(props.headerGroups)
    localFooterGroups.value = normalizeGroups(props.footerGroups)
    // 就地迁移：页码规则上残留的旧 exceptions/overrides 搬进共享例外列表
    const migrated = clone(props.exceptions)
    for (const group of props.groups) {
      const legacy = group.exceptions?.length ? group.exceptions : group.overrides || []
      migrated.push(...clone(legacy))
    }
    localExceptions.value = migrated.map((entry, index) => normalizeInsertException(entry, index))
    const preferred = props.selectedGroupId || props.selectedHeaderGroupId || props.selectedFooterGroupId
    selectedCardId.value = cards.value.some((card) => card.group.id === preferred)
      ? preferred
      : cards.value[0]?.group.id || ''
  },
  { immediate: true },
)

watch(selectedCardId, (value) => {
  const card = cards.value.find((entry) => entry.group.id === value)
  if (!card) return
  if (card.kind === 'header') emit('update:selectedHeaderGroupId', value)
  else if (card.kind === 'footerText') emit('update:selectedFooterGroupId', value)
  else emit('update:selectedGroupId', value)
})
watch(localExceptions, () => emit('update:exceptions', serializeExceptions()), { deep: true })
watch(localGroups, (value) => emit('update:groups', normalizeGroups(value)), { deep: true })
watch(localHeaderGroups, (value) => emit('update:headerGroups', normalizeGroups(value)), { deep: true })
watch(localFooterGroups, (value) => emit('update:footerGroups', normalizeGroups(value)), { deep: true })
</script>

<style scoped>
.dialog-head,
.exception-row-main,
.exception-row-detail,
.color-field {
  display: flex;
  align-items: center;
  gap: 10px;
}
.dialog-head {
  justify-content: space-between;
  margin-bottom: 12px;
  color: var(--docsy-text-muted);
  font-size: 13px;
}
.section-caption {
  margin: 4px 0 8px;
  color: var(--docsy-text-muted);
  font-size: 12px;
  font-weight: 600;
}
.inherit-text {
  color: var(--docsy-text-muted);
  font-size: 12px;
}
.base-scope {
  display: flex;
  align-items: end;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 12px;
  padding: 12px;
  border: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-soft);
}
.base-scope .rule-item {
  min-width: 160px;
}
.base-scope .rule-item:first-child {
  min-width: 220px;
}
.rule-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.rule-item label {
  color: var(--docsy-text-muted);
  font-size: 12px;
}
.exception-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 52vh;
  overflow-y: auto;
}
.exception-row {
  padding: 8px 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
}
.exception-row-main {
  flex-wrap: wrap;
}
.exception-row-detail {
  flex-wrap: wrap;
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px dashed var(--docsy-border-subtle);
}
.scope-type {
  width: 120px;
}
.scope-files {
  min-width: 160px;
  flex: 1;
}
.action-select {
  width: 118px;
}
.num-field {
  width: 130px;
  flex: none;
}
.num-field.slim {
  width: 104px;
}
.num-field .el-input-number {
  width: 100%;
}
.range-sep {
  color: var(--docsy-text-muted);
}
.mini-field {
  width: 110px;
}
.text-field {
  width: 180px;
}
.color-field {
  gap: 4px;
}
</style>
