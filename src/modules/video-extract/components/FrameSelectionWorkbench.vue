<template>
  <section class="frame-workbench" tabindex="0">
    <header class="frame-summary">
      <div class="summary-count summary-total">
        <strong>{{ items.length }}</strong>
        <span>全部图片</span>
      </div>
      <div class="summary-count summary-kept">
        <strong>{{ counts.keep }}</strong>
        <span>最终保留</span>
      </div>
      <div class="summary-count summary-excluded">
        <strong>{{ counts.exclude }}</strong>
        <span>当前排除</span>
      </div>
      <div class="summary-count">
        <strong>{{ counts.review }}</strong>
        <span>待确认</span>
      </div>
      <div class="summary-count summary-risk">
        <strong>{{ counts.risk }}</strong>
        <span>连续性风险</span>
      </div>
      <div class="summary-actions">
        <el-button :disabled="!canUndo" @click="emit('undo')">撤销</el-button>
        <el-button :loading="analyzing" @click="emit('run-analysis')">重新分析</el-button>
        <el-button :disabled="!suggestionCount" @click="emit('accept-suggestions')">
          接受 {{ suggestionCount }} 项建议
        </el-button>
        <el-button type="success" :disabled="!counts.keep" @click="emit('send-to-layout')">
          发送 {{ counts.keep }} 张到图片排版
        </el-button>
      </div>
    </header>

    <div v-if="items.length" class="analysis-status" :class="{ active: analyzing, complete: analyzed }">
      <strong>{{ analysisStatusTitle }}</strong>
      <span>{{ analysisStatusDescription }}</span>
    </div>

    <div class="frame-toolbar">
      <el-segmented v-model="filter" :options="filterOptions" size="small" />
      <span class="toolbar-spacer"></span>
      <span class="toolbar-label">预览大小</span>
      <el-slider v-model="thumbWidth" :min="160" :max="600" :step="20" class="thumb-slider" />
      <span class="toolbar-zoom-value">{{ thumbWidth }} px</span>
      <el-select v-model="pageSize" size="small" class="page-size-select">
        <el-option :value="24" label="每页 24 张" />
        <el-option :value="48" label="每页 48 张" />
        <el-option :value="96" label="每页 96 张" />
      </el-select>
    </div>

    <div class="frame-review-layout">
      <div class="frame-browser">
        <div v-if="pagedItems.length" class="frame-grid">
          <article
            v-for="entry in pagedItems"
            :key="entry.item.path"
            class="frame-card"
            :class="[statusClass(entry.item), { active: entry.originalIndex === selectedIndex }]"
            :style="frameCardStyle(entry.item)"
            :data-frame-index="entry.originalIndex"
            @click="selectedIndex = entry.originalIndex"
            @dblclick="openLargePreview(entry.originalIndex)"
          >
            <div class="frame-card-image">
              <img
                v-if="imageSrc(entry.item)"
                :src="imageSrc(entry.item)"
                :alt="itemName(entry.item)"
                loading="lazy"
                decoding="async"
                @load="recordFrameDimensions(entry.item, $event)"
              />
              <span v-else class="frame-loading">
                {{ previewErrors[entry.item.path] ? '预览加载失败' : '正在载入预览' }}
              </span>
              <button
                v-if="filter === 'all'"
                type="button"
                class="frame-drag-handle"
                title="拖动调整顺序"
                @pointerdown.stop="reorder.start(entry.originalIndex, $event)"
                @pointermove.stop="reorder.move"
                @pointerup.stop="reorder.finish"
                @pointercancel.stop="reorder.reset"
              >
                <el-icon><Rank /></el-icon>
              </button>
              <button
                type="button"
                class="frame-decision-button"
                :title="effectiveDecision(entry.item) === 'exclude' ? '恢复到保留列表' : '从结果中排除'"
                @click.stop="toggleDecision(entry.originalIndex)"
              >
                <el-icon v-if="effectiveDecision(entry.item) === 'exclude'"><RefreshLeft /></el-icon>
                <el-icon v-else><Delete /></el-icon>
              </button>
              <span class="frame-status">{{ statusLabel(entry.item) }}</span>
            </div>
            <div class="frame-card-copy">
              <span class="frame-name" :title="itemName(entry.item)">{{ itemName(entry.item) }}</span>
              <span class="frame-reason" :title="entry.item.reason || ''">{{ shortReason(entry.item) }}</span>
            </div>
          </article>
        </div>
        <WorkspaceEmptyState v-else title="当前筛选条件下没有图片" description="切换筛选条件，或恢复已排除的图片。" />
        <div v-if="filteredEntries.length > pageSize" class="frame-pager">
          <el-button size="small" :disabled="page <= 1" @click="page -= 1">上一页</el-button>
          <span>第 {{ page }} / {{ pageCount }} 页</span>
          <el-button size="small" :disabled="page >= pageCount" @click="page += 1">下一页</el-button>
        </div>
      </div>

      <aside v-if="selectedItem" class="frame-inspector">
        <div class="inspector-head">
          <div>
            <span class="inspector-index">第 {{ selectedIndex + 1 }} 张</span>
            <h3>{{ itemName(selectedItem) }}</h3>
          </div>
          <el-button text @click="openLargePreview(selectedIndex)">大图</el-button>
        </div>
        <div class="compare-mode-row">
          <el-radio-group v-model="compareMode" size="small">
            <el-radio-button value="single">当前图</el-radio-button>
            <el-radio-button value="side" :disabled="!comparisonItem">前后对比</el-radio-button>
            <el-radio-button value="overlay" :disabled="!comparisonItem">透明叠加</el-radio-button>
            <el-radio-button value="difference" :disabled="!comparisonItem">差异</el-radio-button>
          </el-radio-group>
        </div>
        <div class="inspector-preview" :class="`mode-${compareMode}`">
          <img v-if="selectedSource" :src="selectedSource" alt="当前图片" class="current-image" />
          <span v-else class="inspector-loading">
            {{ previewErrors[selectedItem.path] ? '预览加载失败' : '正在载入当前图片' }}
          </span>
          <img
            v-if="comparisonSource && compareMode !== 'single'"
            :src="comparisonSource"
            alt="比较图片"
            class="comparison-image"
          />
        </div>
        <div class="decision-explanation" :class="statusClass(selectedItem)">
          <div class="decision-title">
            <strong>{{ statusLabel(selectedItem) }}</strong>
            <span>{{ confidenceLabel(selectedItem) }}</span>
          </div>
          <p>{{ decisionReason(selectedItem) }}</p>
        </div>
        <dl class="metric-list">
          <div>
            <dt>画面关系</dt>
            <dd>{{ relationLabel(selectedItem.relation) }}</dd>
          </div>
          <div>
            <dt>整体相似度</dt>
            <dd>{{ formatPercent(selectedItem.similarity) }}</dd>
          </div>
          <div>
            <dt>有效重合度</dt>
            <dd>{{ selectedItem.overlap_ratio == null ? '不适用' : formatPercent(selectedItem.overlap_ratio) }}</dd>
          </div>
          <div>
            <dt>局部变化</dt>
            <dd>{{ formatPercent(selectedItem.change_ratio) }}</dd>
          </div>
          <div>
            <dt>清晰度</dt>
            <dd>{{ formatPercent(selectedItem.blur_score) }}</dd>
          </div>
          <div>
            <dt>位移</dt>
            <dd>{{ shiftLabel(selectedItem) }}</dd>
          </div>
          <div>
            <dt>增强复核</dt>
            <dd>{{ enhancementLabel(selectedItem) }}</dd>
          </div>
        </dl>
        <div class="inspector-actions">
          <el-button
            :type="effectiveDecision(selectedItem) === 'keep' ? 'success' : 'default'"
            @click="setDecision(selectedIndex, 'keep')"
          >
            保留
          </el-button>
          <el-button
            :type="effectiveDecision(selectedItem) === 'exclude' ? 'danger' : 'default'"
            @click="setDecision(selectedIndex, 'exclude')"
          >
            排除
          </el-button>
          <el-button @click="setDecision(selectedIndex, null)">跟随建议</el-button>
        </div>
        <p class="inspector-tip">删除键排除，方向键切换，双击查看大图。所有操作只影响本次结果，不删除原文件。</p>
      </aside>
    </div>

    <el-dialog
      v-model="largePreviewVisible"
      :title="largePreviewItem ? itemName(largePreviewItem) : '图片审核'"
      width="96%"
      destroy-on-close
    >
      <div v-if="largePreviewItem" class="large-preview">
        <el-button :disabled="largePreviewIndex <= 0" @click="moveLargePreview(-1)">上一张</el-button>
        <div class="large-preview-stage">
          <img v-if="largePreviewSource" :src="largePreviewSource" :alt="itemName(largePreviewItem)" />
          <span v-else class="inspector-loading">正在载入高清图片</span>
        </div>
        <el-button :disabled="largePreviewIndex >= items.length - 1" @click="moveLargePreview(1)">下一张</el-button>
      </div>
      <template v-if="largePreviewItem" #footer>
        <div class="large-preview-actions">
          <span>{{ statusLabel(largePreviewItem) }}</span>
          <el-button
            :type="effectiveDecision(largePreviewItem) === 'keep' ? 'success' : 'default'"
            @click="setLargePreviewDecision('keep')"
          >
            保留
          </el-button>
          <el-button
            :type="effectiveDecision(largePreviewItem) === 'exclude' ? 'danger' : 'default'"
            @click="setLargePreviewDecision('exclude')"
          >
            排除
          </el-button>
        </div>
      </template>
    </el-dialog>
  </section>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { Delete, Rank, RefreshLeft } from '@element-plus/icons-vue'
import { tauriCallQuiet } from '../../../core/tauriBridge.js'
import { fileName } from '../../../core/filePath.js'
import { usePointerReorder } from '../../../core/composables/usePointerReorder.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'

const props = defineProps({
  items: { type: Array, default: () => [] },
  analyzing: { type: Boolean, default: false },
  analyzed: { type: Boolean, default: false },
  analysisProgress: { type: String, default: '' },
  canUndo: { type: Boolean, default: false },
})

const emit = defineEmits(['update-decision', 'reorder', 'undo', 'run-analysis', 'accept-suggestions', 'send-to-layout'])
const filter = ref('all')
const thumbWidth = ref(240)
const pageSize = ref(24)
const page = ref(1)
const selectedIndex = ref(0)
const compareMode = ref('single')
const sources = reactive({})
const largeSources = reactive({})
const frameDimensions = reactive({})
const previewErrors = reactive({})
const loading = new Set()
const loadingLarge = new Set()
const largePreviewVisible = ref(false)
const largePreviewIndex = ref(0)
const preference = useWorkspacePreferences('video-extract.workbench', {
  filter,
  thumbWidth,
  pageSize,
  compareMode,
})
const filterOptions = [
  { label: '全部', value: 'all' },
  { label: '保留', value: 'keep' },
  { label: '已排除', value: 'exclude' },
  { label: '待确认', value: 'review' },
  { label: '风险', value: 'risk' },
]

const reorder = usePointerReorder({
  itemCount: () => props.items.length,
  itemAttribute: 'data-frame-index',
  onReorder: (payload) => emit('reorder', payload),
})

const counts = computed(() => {
  const value = { keep: 0, exclude: 0, review: 0, risk: 0 }
  for (const item of props.items) {
    const decision = effectiveDecision(item)
    if (decision === 'exclude') value.exclude += 1
    else if (decision === 'review') value.review += 1
    else value.keep += 1
    if (item.continuity_risk) value.risk += 1
  }
  return value
})
const suggestionCount = computed(
  () => props.items.filter((item) => !item.user_decision && item.engine_decision === 'exclude').length,
)
const filteredEntries = computed(() =>
  props.items
    .map((item, originalIndex) => ({ item, originalIndex }))
    .filter(({ item }) => {
      if (filter.value === 'all') return true
      if (filter.value === 'risk') return Boolean(item.continuity_risk)
      return effectiveDecision(item) === filter.value
    }),
)
const pageCount = computed(() => Math.max(1, Math.ceil(filteredEntries.value.length / pageSize.value)))
const pagedItems = computed(() =>
  filteredEntries.value.slice((page.value - 1) * pageSize.value, page.value * pageSize.value),
)
const selectedItem = computed(() => props.items[selectedIndex.value] || null)
const comparisonIndex = computed(() => {
  if (!selectedItem.value) return -1
  if (Number.isInteger(selectedItem.value.compared_to)) return selectedItem.value.compared_to
  return selectedIndex.value > 0 ? selectedIndex.value - 1 : -1
})
const comparisonItem = computed(() => props.items[comparisonIndex.value] || null)
const selectedSource = computed(() => imageSrc(selectedItem.value))
const comparisonSource = computed(() => imageSrc(comparisonItem.value))
const largePreviewItem = computed(() => props.items[largePreviewIndex.value] || null)
const largePreviewSource = computed(() => {
  const path = largePreviewItem.value?.path
  return path ? largeSources[path] || sources[path] || '' : ''
})
const analysisStatusTitle = computed(() => {
  if (props.analyzing) return props.analysisProgress || `正在智能分析 ${props.items.length} 张图片`
  if (props.analyzed) return '智能分析已完成'
  return '智能分析尚未完成'
})
const analysisStatusDescription = computed(() => {
  if (props.analyzing) return '预览和人工筛选可以同时进行，完成后会自动补充置信度与排除原因。'
  if (props.analyzed) return '算法建议已经生成；人工确认始终优先，原图片不会被删除。'
  return '当前只显示人工决定，置信度与画面关系需要等待智能分析结果。'
})

function effectiveDecision(item) {
  return item?.user_decision || item?.engine_decision || 'review'
}

function setDecision(index, decision) {
  emit('update-decision', { index, decision })
  if (index === selectedIndex.value && index < props.items.length - 1) {
    selectedIndex.value = index + 1
  }
}

function toggleDecision(index) {
  const item = props.items[index]
  setDecision(index, effectiveDecision(item) === 'exclude' ? 'keep' : 'exclude')
}

function statusClass(item) {
  if (item?.continuity_risk) return 'status-risk'
  return `status-${effectiveDecision(item)}`
}

function statusLabel(item) {
  if (!item?.relation && item?.user_decision === 'exclude') return '用户已排除 · 待分析'
  if (!item?.relation && item?.user_decision === 'keep') return '用户已保留 · 待分析'
  if (item?.continuity_risk) return '需要核对连续性'
  if (effectiveDecision(item) === 'exclude') return item?.user_decision ? '用户已排除' : '建议排除'
  if (effectiveDecision(item) === 'keep') return item?.user_decision ? '用户已保留' : '建议保留'
  return '待确认'
}

function confidenceLabel(item) {
  return item?.relation ? `置信度 ${formatPercent(item.confidence)}` : '智能分析待完成'
}

function enhancementLabel(item) {
  if (!item?.enhanced_by) return '轻量视觉引擎'
  const response = item.opencv_match_ratio == null ? '' : ` · 位移响应 ${formatPercent(item.opencv_match_ratio)}`
  return `${item.enhanced_by}${response}`
}

function decisionReason(item) {
  if (!item?.relation && item?.user_decision) return '已记录人工决定；智能分析完成后会补充判断依据。'
  return item?.reason || '尚未运行智能分析，由用户决定是否保留。'
}

function relationLabel(value) {
  return (
    {
      first: '序列起点',
      duplicate: '近似重复',
      continuous_vertical: '纵向连续',
      continuous_horizontal: '横向连续',
      changed: '局部变化',
      scene_cut: '场景切换',
      uncertain: '无法可靠判断',
    }[value] || '尚未分析'
  )
}

function shortReason(item) {
  if (item?.continuity_risk) return '重合不足，可能存在内容缺口'
  if (item?.relation === 'duplicate') return `相似度 ${formatPercent(item.similarity)}`
  if (item?.overlap_ratio != null) return `重合 ${formatPercent(item.overlap_ratio)}`
  if (item?.relation === 'scene_cut') return '检测到场景切换'
  if (item?.reason) return item.reason
  return '尚未分析'
}

function formatPercent(value) {
  if (!Number.isFinite(Number(value))) return '未知'
  return `${Math.round(Number(value) * 100)}%`
}

function shiftLabel(item) {
  const x = Number(item?.shift_x || 0)
  const y = Number(item?.shift_y || 0)
  if (!x && !y) return '无可靠位移'
  if (Math.abs(y) >= Math.abs(x)) return `纵向 ${Math.abs(y)} 个分析像素`
  return `横向 ${Math.abs(x)} 个分析像素`
}

function itemName(item) {
  return fileName(item?.path || '')
}

function imageSrc(item) {
  return item?.path ? sources[item.path] || '' : ''
}

function recordFrameDimensions(item, event) {
  const target = event?.target
  if (!item?.path || !target?.naturalWidth || !target?.naturalHeight) return
  frameDimensions[item.path] = { width: target.naturalWidth, height: target.naturalHeight }
}

function frameCardStyle(item) {
  const measured = frameDimensions[item?.path]
  const width = Number(item?.width || measured?.width || 16)
  const height = Number(item?.height || measured?.height || 9)
  const ratio = Math.min(12, Math.max(0.08, width / height))
  const imageWidth = ratio >= 1 ? thumbWidth.value : Math.max(76, Math.round(thumbWidth.value * ratio))
  const imageHeight = ratio >= 1 ? Math.max(56, Math.round(thumbWidth.value / ratio)) : thumbWidth.value
  return {
    width: `${Math.max(80, imageWidth) + 2}px`,
    '--frame-image-height': `${imageHeight}px`,
  }
}

async function loadSources(items) {
  const paths = items.map((entry) => entry?.item?.path || entry?.path).filter(Boolean)
  for (const path of [selectedItem.value?.path, comparisonItem.value?.path, largePreviewItem.value?.path]) {
    if (path) paths.push(path)
  }
  const keep = new Set(paths)
  for (const path of Object.keys(sources)) {
    if (!keep.has(path) && !loading.has(path)) delete sources[path]
  }
  const queue = [...keep].filter((path) => !sources[path] && !loading.has(path))
  const workers = Array.from({ length: Math.min(4, queue.length) }, async () => {
    while (queue.length) {
      const path = queue.shift()
      loading.add(path)
      delete previewErrors[path]
      const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 560 })
      loading.delete(path)
      if (result.ok) sources[path] = result.data
      else previewErrors[path] = true
    }
  })
  await Promise.all(workers)
}

async function loadLargeSource(item) {
  const path = item?.path
  if (!path || largeSources[path] || loadingLarge.has(path)) return
  loadingLarge.add(path)
  const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 3840 })
  loadingLarge.delete(path)
  if (result.ok) {
    for (const cachedPath of Object.keys(largeSources)) delete largeSources[cachedPath]
    largeSources[path] = result.data
  }
}

function openLargePreview(index) {
  largePreviewIndex.value = index
  largePreviewVisible.value = true
  void loadLargeSource(props.items[index])
}

function moveLargePreview(delta) {
  largePreviewIndex.value = Math.min(props.items.length - 1, Math.max(0, largePreviewIndex.value + delta))
  selectedIndex.value = largePreviewIndex.value
  void loadLargeSource(props.items[largePreviewIndex.value])
}

function setLargePreviewDecision(decision) {
  if (!largePreviewItem.value) return
  selectedIndex.value = largePreviewIndex.value
  emit('update-decision', { index: largePreviewIndex.value, decision })
}

function handleKeydown(event) {
  const target = event.target
  if (target?.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target?.tagName)) return
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z') {
    emit('undo')
    event.preventDefault()
  } else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    const delta = event.key === 'ArrowLeft' ? -1 : 1
    selectedIndex.value = Math.min(props.items.length - 1, Math.max(0, selectedIndex.value + delta))
    event.preventDefault()
  } else if ((event.key === 'Delete' || event.key === 'Backspace') && selectedItem.value) {
    setDecision(selectedIndex.value, 'exclude')
    event.preventDefault()
  }
}

watch([pagedItems, selectedIndex, comparisonIndex], () => void loadSources(pagedItems.value), { immediate: true })
watch([filter, pageSize], () => {
  page.value = 1
})
watch(pageCount, (value) => {
  page.value = Math.min(value, Math.max(1, page.value))
})
watch(
  () => props.items.length,
  (length) => {
    selectedIndex.value = Math.min(Math.max(0, length - 1), selectedIndex.value)
  },
)

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
  void preference.start()
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
  void preference.stop()
})
</script>

<style scoped>
.frame-workbench {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  outline: none;
  container-type: inline-size;
}

.frame-summary {
  display: flex;
  align-items: stretch;
  gap: 1px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-border-subtle);
}

.summary-count {
  display: flex;
  min-width: 92px;
  padding: 10px 14px;
  flex-direction: column;
  background: var(--docsy-surface-elevated);
}

.summary-count strong {
  color: var(--docsy-text-strong);
  font-size: 20px;
  line-height: 1;
}

.summary-count span {
  margin-top: 5px;
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.summary-kept strong {
  color: var(--docsy-primary);
}

.summary-excluded strong,
.summary-risk strong {
  color: var(--el-color-warning-dark-2);
}

.summary-actions {
  display: flex;
  min-width: 0;
  padding: 10px 14px;
  flex: 1;
  align-items: center;
  justify-content: flex-end;
  background: var(--docsy-surface-elevated);
}

.analysis-status {
  display: flex;
  min-height: 40px;
  padding: 8px 14px;
  align-items: center;
  gap: 12px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-muted);
  font-size: 12px;
}

.analysis-status strong {
  flex: 0 0 auto;
  color: var(--docsy-text-strong);
}

.analysis-status.active {
  background: color-mix(in srgb, var(--el-color-warning-light-9) 58%, var(--docsy-surface-elevated));
}

.analysis-status.complete {
  background: color-mix(in srgb, var(--docsy-primary-soft) 72%, var(--docsy-surface-elevated));
}

.frame-toolbar {
  display: flex;
  min-height: 48px;
  padding: 8px 14px;
  align-items: center;
  gap: 10px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: color-mix(in srgb, var(--docsy-surface-elevated) 88%, transparent);
}

.toolbar-spacer {
  flex: 1;
}

.toolbar-label {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.thumb-slider {
  width: 120px;
}

.toolbar-zoom-value {
  min-width: 46px;
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.page-size-select {
  width: 122px;
}

.frame-review-layout {
  display: grid;
  min-height: 0;
  flex: 1;
  grid-template-columns: minmax(0, 1fr) clamp(300px, 29vw, 410px);
}

.frame-browser {
  min-width: 0;
  overflow: auto;
  padding: 14px;
  background: color-mix(in srgb, var(--docsy-surface-muted) 86%, var(--docsy-canvas));
}

.frame-grid {
  display: flex;
  flex-wrap: wrap;
  align-items: start;
  gap: 12px;
}

.frame-card {
  flex: 0 0 auto;
  overflow: hidden;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: 0 7px 18px rgba(49, 44, 37, 0.045);
  cursor: pointer;
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}

.frame-card:hover,
.frame-card.active {
  border-color: color-mix(in srgb, var(--docsy-primary) 58%, var(--docsy-border-subtle));
  box-shadow: 0 10px 24px rgba(34, 63, 50, 0.11);
}

.frame-card.active {
  box-shadow:
    inset 0 0 0 1px var(--docsy-primary),
    0 10px 24px rgba(34, 63, 50, 0.11);
}

.frame-card.status-exclude {
  opacity: 0.72;
  background: color-mix(in srgb, var(--el-color-danger-light-9) 58%, var(--docsy-surface-elevated));
}

.frame-card.status-risk {
  border-color: var(--el-color-warning-light-5);
}

.frame-card-image {
  position: relative;
  display: grid;
  width: 100%;
  height: var(--frame-image-height, 135px);
  place-items: center;
  overflow: hidden;
  background: var(--docsy-surface-muted);
}

.frame-card-image img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.frame-loading,
.inspector-loading {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.frame-drag-handle,
.frame-decision-button {
  position: absolute;
  top: 8px;
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border: 1px solid rgba(255, 255, 255, 0.62);
  border-radius: 8px;
  color: #fff;
  background: rgba(30, 34, 31, 0.64);
  backdrop-filter: blur(8px);
  cursor: pointer;
}

.frame-drag-handle {
  left: 8px;
  touch-action: none;
  cursor: grab;
}

.frame-decision-button {
  right: 8px;
}

.frame-status {
  position: absolute;
  right: 8px;
  bottom: 8px;
  max-width: calc(100% - 16px);
  padding: 4px 7px;
  overflow: hidden;
  border-radius: 6px;
  color: #fff;
  background: rgba(29, 48, 40, 0.78);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
  backdrop-filter: blur(8px);
}

.status-exclude .frame-status {
  background: rgba(126, 70, 61, 0.82);
}

.status-risk .frame-status {
  background: rgba(141, 91, 20, 0.86);
}

.frame-card-copy {
  display: flex;
  min-width: 0;
  padding: 9px 10px 10px;
  flex-direction: column;
  gap: 4px;
}

.frame-name,
.frame-reason {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.frame-name {
  color: var(--docsy-text-strong);
  font-size: 12px;
  font-weight: 600;
}

.frame-reason {
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.frame-pager {
  display: flex;
  margin-top: 14px;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.frame-inspector {
  min-width: 0;
  overflow: auto;
  padding: 16px;
  border-left: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-elevated);
}

.inspector-head,
.decision-title,
.compare-mode-row,
.inspector-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.inspector-head h3 {
  max-width: 280px;
  margin: 3px 0 0;
  overflow: hidden;
  color: var(--docsy-text-strong);
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.inspector-index {
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.compare-mode-row {
  margin: 12px 0 10px;
  justify-content: flex-start;
}

.inspector-preview {
  position: relative;
  display: grid;
  min-height: 220px;
  max-height: 44vh;
  grid-template-columns: 1fr;
  place-items: center;
  overflow: auto;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
}

.inspector-preview.mode-side {
  grid-template-columns: 1fr 1fr;
  gap: 1px;
}

.inspector-preview img {
  display: block;
  width: auto;
  max-width: 100%;
  height: auto;
  max-height: 44vh;
  object-fit: contain;
}

@container (max-width: 1100px) {
  .frame-summary {
    flex-wrap: wrap;
  }

  .summary-count {
    min-width: 82px;
    flex: 1;
  }

  .summary-actions {
    min-width: 100%;
    flex-basis: 100%;
  }
}

.inspector-preview.mode-overlay img {
  grid-area: 1 / 1;
}

.inspector-preview.mode-difference img {
  grid-area: 1 / 1;
}

.inspector-preview.mode-difference .comparison-image {
  opacity: 0.82;
  mix-blend-mode: difference;
}

.inspector-preview.mode-overlay .comparison-image {
  opacity: 0.48;
}

.decision-explanation {
  margin-top: 12px;
  padding: 11px 12px;
  border-left: 3px solid var(--docsy-primary);
  border-radius: 0 var(--docsy-radius) var(--docsy-radius) 0;
  background: var(--docsy-primary-soft);
}

.decision-explanation.status-exclude {
  border-left-color: var(--el-color-danger);
  background: var(--el-color-danger-light-9);
}

.decision-explanation.status-risk {
  border-left-color: var(--el-color-warning);
  background: var(--el-color-warning-light-9);
}

.decision-title span {
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.decision-explanation p {
  margin: 6px 0 0;
  color: var(--docsy-text);
  font-size: 12px;
  line-height: 1.6;
}

.metric-list {
  display: grid;
  margin: 12px 0;
  grid-template-columns: 1fr 1fr;
  gap: 1px;
  overflow: hidden;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-border-subtle);
}

.metric-list div {
  min-width: 0;
  padding: 8px 10px;
  background: var(--docsy-surface-elevated);
}

.metric-list dt {
  color: var(--docsy-text-muted);
  font-size: 10px;
}

.metric-list dd {
  margin: 3px 0 0;
  color: var(--docsy-text-strong);
  font-size: 12px;
}

.inspector-actions {
  justify-content: flex-start;
}

.inspector-tip {
  margin: 12px 0 0;
  color: var(--docsy-text-muted);
  font-size: 11px;
  line-height: 1.55;
}

.large-preview {
  display: grid;
  min-height: 76vh;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
}

.large-preview-stage {
  display: grid;
  min-width: 0;
  min-height: 76vh;
  place-items: center;
  overflow: auto;
  background: var(--docsy-surface-muted);
}

.large-preview img {
  display: block;
  max-width: none;
  max-height: none;
  margin: 0 auto;
  object-fit: contain;
}

.large-preview-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

.large-preview-actions span {
  margin-right: auto;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

@media (max-width: 1260px) {
  .frame-review-layout {
    grid-template-columns: minmax(0, 1fr) 320px;
  }

  .summary-count {
    min-width: 76px;
    padding-inline: 10px;
  }
}
</style>
