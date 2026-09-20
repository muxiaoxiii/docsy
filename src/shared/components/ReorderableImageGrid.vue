<template>
  <div class="reorder-image-list">
    <div class="reorder-image-toolbar">
      <div class="reorder-image-filter">
        <el-radio-group v-model="filterMode" size="small">
          <el-radio-button value="all">全部 ({{ items.length }})</el-radio-button>
          <el-radio-button value="conflict">有冲突 ({{ conflictCount }})</el-radio-button>
          <el-radio-button value="excluded">已排除 ({{ excludedCount }})</el-radio-button>
        </el-radio-group>
      </div>
      <span class="reorder-image-hint">{{ filterMode === 'all' ? '拖动手柄调整顺序' : '筛选模式下已锁定顺序' }}</span>
      <span>{{ rangeLabel }}</span>
      <el-select v-model="selectedFraction" size="small" class="reorder-image-page-size" aria-label="每页图片数量">
        <el-option
          v-for="option in pageSizeOptions"
          :key="option.fraction"
          :label="option.label"
          :value="option.fraction"
        />
      </el-select>
      <el-button size="small" text @click="adjustZoom(-10)">-</el-button>
      <el-slider v-model="zoom" :min="minZoom" :max="maxZoom" :step="5" class="reorder-image-zoom" />
      <el-button size="small" text @click="adjustZoom(10)">+</el-button>
      <span class="reorder-image-zoom-value">{{ zoom }}%</span>
    </div>

    <div v-if="filteredItems.length" ref="scrollContainer" class="reorder-image-scroll">
      <div class="reorder-image-grid">
        <article
          v-for="(item, localIndex) in pagedItems"
          :key="itemKey(item, localIndex)"
          class="reorder-image-card"
          :class="[reorder.itemClasses(globalIndex(localIndex)), cardConflictClass(item)]"
          :style="cardStyle(item)"
          :data-reorder-index="globalIndex(localIndex)"
        >
          <div class="reorder-image-card-header">
            <button
              type="button"
              class="reorder-image-handle"
              :class="{ 'is-disabled': filterMode !== 'all' }"
              :title="filterMode !== 'all' ? '筛选模式下不可调整顺序，请切换至「全部」' : '拖动调整顺序'"
              :disabled="filterMode !== 'all'"
              @pointerdown.stop="filterMode === 'all' && reorder.start(globalIndex(localIndex), $event)"
              @pointermove.stop="reorder.move"
              @pointerup.stop="reorder.finish"
              @pointercancel.stop="reorder.reset"
            >
              <el-icon><Rank /></el-icon>
            </button>
            <button
              v-if="pageBadge(item)"
              type="button"
              class="reorder-image-page-badge"
              :class="[`badge-color-${pageBadge(item).color}`, { 'is-modified': pageBadge(item).isModified }]"
              :title="`第 ${pageBadge(item).pageNumber} 页 · 点击在上方预览`"
              @click.stop="onJumpPage(pageBadge(item).pageIndex)"
            >
              P.{{ pageBadge(item).pageNumber }}
              <span v-if="pageBadge(item).isModified" class="badge-dot" title="已微调">●</span>
            </button>
            <button
              v-if="rotationResolver"
              type="button"
              class="reorder-image-action-btn reorder-image-rotate"
              :disabled="disabled"
              aria-label="顺时针旋转90°"
              :title="`顺时针旋转90°（当前 ${itemRotation(item)}°）`"
              @click.stop="rotateItem(item)"
            >
              <el-icon><RefreshRight /></el-icon>
            </button>
            <div class="reorder-image-top-actions">
              <button
                type="button"
                class="reorder-image-action-btn"
                title="查看大图与设置说明"
                @click.stop="openPreview(item)"
              >
                <el-icon><ZoomIn /></el-icon>
              </button>
              <button
                v-if="excludedResolver"
                type="button"
                class="reorder-image-decision"
                :class="{ excluded: itemExcluded(item) }"
                :title="itemExcluded(item) ? '恢复到排版结果' : '从排版结果中排除'"
                @click.stop="toggleExcluded(item, globalIndex(localIndex))"
              >
                <el-icon v-if="itemExcluded(item)"><RefreshLeft /></el-icon>
                <el-icon v-else><Delete /></el-icon>
              </button>
            </div>
          </div>
          <button
            type="button"
            class="reorder-image-preview"
            @click="handleCardClick(item, $event)"
            @dblclick="openPreview(item)"
          >
            <span class="reorder-image-thumb-wrap" :style="thumbWrapStyle(item)">
              <img
                v-if="imageSrc(item)"
                :src="imageSrc(item)"
                :alt="itemName(item)"
                class="reorder-image-thumb"
                loading="lazy"
                decoding="async"
                @load="recordDimensions(item, $event)"
              />
              <span v-else class="reorder-image-placeholder">
                {{ previewErrors[imageKey(item)] ? '预览失败' : '预览中' }}
              </span>
            </span>
            <span class="reorder-image-name" :title="itemName(item)">{{ itemName(item) }}</span>
            <span v-if="itemExcluded(item)" class="reorder-image-status">已排除，不参与排版</span>
            <span v-if="itemMeta(item)" class="reorder-image-meta">{{ itemMeta(item) }}</span>
          </button>
        </article>
      </div>
    </div>

    <el-empty v-else :description="filterMode === 'all' ? emptyDescription : '没有符合条件的图片'" :image-size="80" />

    <div v-if="filteredItems.length" class="reorder-image-pager">
      <el-button size="small" :disabled="page <= 1" @click="page -= 1">上一页</el-button>
      <span>第 {{ page }} / {{ pageCount }} 页</span>
      <el-button size="small" :disabled="page >= pageCount" @click="page += 1">下一页</el-button>
    </div>

    <el-dialog
      v-model="previewVisible"
      :title="previewItem ? itemName(previewItem) || '图片详情与批注' : '图片预览'"
      width="96vw"
      top="3vh"
      class="reorder-image-detail-dialog"
      destroy-on-close
    >
      <div class="reorder-image-detail-toolbar">
        <el-button
          v-if="rotationResolver && previewItem"
          :disabled="disabled"
          size="small"
          @click="rotateItem(previewItem)"
        >
          <el-icon><RefreshRight /></el-icon><span>旋转90°</span>
        </el-button>
        <span v-if="rotationResolver && previewItem" class="reorder-image-detail-angle"
          >{{ itemRotation(previewItem) }}°</span
        >
        <el-button
          size="small"
          :disabled="!previewSrc"
          @click="setPreviewZoom(previewZoomPercent - 25)"
          aria-label="缩小图片"
          >−</el-button
        >
        <el-slider
          class="reorder-image-detail-zoom"
          :model-value="previewZoomPercent"
          :min="5"
          :max="400"
          :step="5"
          :disabled="!previewSrc"
          @input="setPreviewZoom"
          aria-label="大图缩放比例"
        />
        <el-button
          size="small"
          :disabled="!previewSrc"
          @click="setPreviewZoom(previewZoomPercent + 25)"
          aria-label="放大图片"
          >+</el-button
        >
        <span class="reorder-image-detail-percent">{{ previewZoomPercent }}%</span>
        <el-button size="small" @click="previewFit = true">适合窗口</el-button>
        <el-button size="small" @click="setPreviewZoom(100)">100%</el-button>
        <el-button
          v-if="annotationResolver"
          size="small"
          class="reorder-image-detail-toggle"
          :aria-expanded="detailsOpen"
          @click="detailsOpen = !detailsOpen"
        >
          {{ detailsOpen ? '收起标题与说明' : '标题与说明' }}
        </el-button>
      </div>
      <div class="reorder-image-dialog-layout">
        <div
          ref="previewViewport"
          class="reorder-image-dialog-preview"
          :class="{ 'is-panning': isPanning }"
          @pointerdown="startPan"
          @pointermove="movePan"
          @pointerup="stopPan"
          @pointercancel="stopPan"
          @lostpointercapture="stopPan"
          @wheel.ctrl.prevent="zoomPreviewWheel"
          @wheel.meta.prevent="zoomPreviewWheel"
        >
          <div class="reorder-image-dialog-stage" :style="previewStageStyle">
            <img
              v-if="previewSrc"
              :src="previewSrc"
              :alt="previewItem ? itemName(previewItem) : ''"
              :style="previewImageSize"
              class="reorder-image-dialog-img"
              :draggable="false"
              @load="recordPreviewDimensions"
              @dblclick="previewFit ? setPreviewZoom(100) : (previewFit = true)"
            />
            <span v-else class="reorder-image-placeholder" role="status">{{
              previewLoadError ? '高清预览失败，请关闭后重试' : '正在载入高清图片'
            }}</span>
          </div>
        </div>
        <div v-if="previewItem && annotationResolver && detailsOpen" class="reorder-image-dialog-form">
          <div class="form-section-title">图注与说明设置</div>
          <el-form label-position="top" size="small" :disabled="disabled">
            <el-form-item label="图片标题">
              <el-input v-model="editTitle" placeholder="留空则使用默认文件名" @input="saveAnnotation" clearable />
            </el-form-item>
            <el-form-item label="详细说明 / 见证记录">
              <el-input
                v-model="editDescription"
                type="textarea"
                :rows="4"
                placeholder="如：拍摄时间、见证人、事实说明...（若留空且开启了预留说明栏，将自动生成说明占位符）"
                @input="saveAnnotation"
              />
            </el-form-item>
          </el-form>
          <div class="dialog-meta-info">
            <div><strong>文件：</strong>{{ itemPath(previewItem) }}</div>
            <div v-if="itemMeta(previewItem)"><strong>尺寸：</strong>{{ itemMeta(previewItem) }}</div>
            <div v-if="pageBadge(previewItem)">
              <strong>所在页：</strong>第 {{ pageBadge(previewItem).pageNumber }} 页
            </div>
          </div>
        </div>
      </div>
      <template v-if="previewItem" #footer>
        <div class="reorder-image-dialog-actions">
          <span v-if="excludedResolver">{{ itemExcluded(previewItem) ? '当前不参与排版' : '当前参与排版' }}</span>
          <div class="reorder-image-dialog-buttons">
            <el-button
              v-if="excludedResolver"
              :type="itemExcluded(previewItem) ? 'success' : 'danger'"
              @click="toggleExcluded(previewItem, previewIndex)"
            >
              {{ itemExcluded(previewItem) ? '恢复保留' : '排除图片' }}
            </el-button>
            <el-button type="primary" @click="previewVisible = false">确定</el-button>
          </div>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { Delete, Rank, RefreshLeft, RefreshRight, ZoomIn } from '@element-plus/icons-vue'
import { tauriCallQuiet } from '../../core/tauriBridge.js'
import { fileName } from '../../core/filePath.js'
import { usePointerReorder } from '../../core/composables/usePointerReorder.js'
import { pageRangeForSize, pageSizeForFraction, percentagePageSizeOptions } from './imageGridPagination.js'

const props = defineProps({
  items: { type: Array, default: () => [] },
  nameResolver: { type: Function, default: null },
  metaResolver: { type: Function, default: null },
  pathResolver: { type: Function, default: null },
  pageBadgeResolver: { type: Function, default: null },
  emptyDescription: { type: String, default: '暂无图片' },
  pageFraction: { type: Number, default: 0.25 },
  initialZoom: { type: Number, default: 100 },
  minZoom: { type: Number, default: 60 },
  maxZoom: { type: Number, default: 320 },
  preserveAspectRatio: { type: Boolean, default: false },
  excludedResolver: { type: Function, default: null },
  annotationResolver: { type: Function, default: null },
  rotationResolver: { type: Function, default: null },
  disabled: { type: Boolean, default: false },
})

const emit = defineEmits([
  'reorder',
  'toggle-excluded',
  'update:page-fraction',
  'select-page',
  'select-item',
  'update-annotation',
  'rotate',
])

function pageBadge(item) {
  return props.pageBadgeResolver ? props.pageBadgeResolver(item) : null
}

function cardConflictClass(item) {
  const badge = pageBadge(item)
  if (!badge || !badge.color || badge.color === 'ok') return ''
  return `card-conflict-${badge.color}`
}

function onJumpPage(pageIndex) {
  if (pageIndex !== undefined && pageIndex !== null) {
    emit('select-page', pageIndex)
  }
}

function handleCardClick(item) {
  const badge = pageBadge(item)
  if (badge && badge.pageIndex !== undefined) {
    emit('select-page', badge.pageIndex)
    emit('select-item', { item, pageIndex: badge.pageIndex })
  } else {
    openPreview(item)
  }
}
const filterMode = ref('all')
const page = ref(1)
const zoom = ref(props.initialZoom)
const scrollContainer = ref(null)
const sources = reactive({})
const largeSources = reactive({})
const dimensions = reactive({})
const previewErrors = reactive({})
const loadingPaths = new Set()
const loadingLargePaths = new Set()
const previewVisible = ref(false)
const previewSrc = ref('')
const previewPath = ref('')
const previewItem = computed(() => props.items.find((item) => itemPath(item) === previewPath.value) || null)
const previewIndex = computed(() => props.items.indexOf(previewItem.value))
const detailsOpen = ref(false)
const previewLoadError = ref(false)
const previewViewport = ref(null)
const previewNatural = reactive({ width: 0, height: 0 })
const viewportSize = reactive({ width: 1, height: 1 })
const previewFit = ref(true)
const detailZoom = ref(100)
const fitScale = computed(() =>
  Math.min(
    Math.max(1, viewportSize.width - 24) / Math.max(1, previewNatural.width),
    Math.max(1, viewportSize.height - 24) / Math.max(1, previewNatural.height),
    1,
  ),
)
const previewScale = computed(() => (previewFit.value ? fitScale.value : detailZoom.value / 100))
const previewZoomPercent = computed(() => Math.round(previewScale.value * 100))
const previewImageSize = computed(() => ({
  width: `${previewNatural.width * previewScale.value}px`,
  height: `${previewNatural.height * previewScale.value}px`,
}))
const previewStageStyle = computed(() => ({
  width: `${Math.max(viewportSize.width, previewNatural.width * previewScale.value + 24)}px`,
  height: `${Math.max(viewportSize.height, previewNatural.height * previewScale.value + 24)}px`,
}))
const isPanning = ref(false)
let panStart = null
let viewportObserver
let zoomRevision = 0

function recordPreviewDimensions(event) {
  previewNatural.width = event.target.naturalWidth
  previewNatural.height = event.target.naturalHeight
}

async function setPreviewZoom(value) {
  const revision = ++zoomRevision
  const viewport = previewViewport.value
  const oldWidth = Math.max(viewportSize.width, previewNatural.width * previewScale.value + 24)
  const oldHeight = Math.max(viewportSize.height, previewNatural.height * previewScale.value + 24)
  const centerX = viewport ? (viewport.scrollLeft + viewport.clientWidth / 2) / oldWidth : 0.5
  const centerY = viewport ? (viewport.scrollTop + viewport.clientHeight / 2) / oldHeight : 0.5
  detailZoom.value = Math.min(400, Math.max(5, Number(value) || 100))
  previewFit.value = false
  await nextTick()
  if (!viewport || revision !== zoomRevision) return
  viewport.scrollLeft = centerX * viewport.scrollWidth - viewport.clientWidth / 2
  viewport.scrollTop = centerY * viewport.scrollHeight - viewport.clientHeight / 2
}

function zoomPreviewWheel(event) {
  setPreviewZoom(previewZoomPercent.value + (event.deltaY < 0 ? 10 : -10))
}
function startPan(event) {
  if (event.button !== 0 || !previewSrc.value) return
  const viewport = previewViewport.value
  panStart = {
    x: event.clientX,
    y: event.clientY,
    left: viewport.scrollLeft,
    top: viewport.scrollTop,
    id: event.pointerId,
  }
  isPanning.value = true
  viewport.setPointerCapture(event.pointerId)
  event.preventDefault()
}
function movePan(event) {
  if (!panStart || panStart.id !== event.pointerId) return
  previewViewport.value.scrollLeft = panStart.left - (event.clientX - panStart.x)
  previewViewport.value.scrollTop = panStart.top - (event.clientY - panStart.y)
}
function stopPan() {
  panStart = null
  isPanning.value = false
}

watch(
  previewViewport,
  (viewport) => {
    viewportObserver?.disconnect()
    if (!viewport) return
    const measure = () => {
      viewportSize.width = viewport.clientWidth
      viewportSize.height = viewport.clientHeight
    }
    measure()
    viewportObserver = new window.ResizeObserver(measure)
    viewportObserver.observe(viewport)
  },
  { flush: 'post' },
)
onBeforeUnmount(() => viewportObserver?.disconnect())
const editTitle = ref('')
const editDescription = ref('')

function itemHasConflict(item) {
  const badge = pageBadge(item)
  return Boolean(badge && ['red', 'yellow', 'green'].includes(badge.color))
}

const conflictCount = computed(() => {
  return props.items.filter(itemHasConflict).length
})

const excludedCount = computed(() => {
  return props.items.filter(itemExcluded).length
})

const filteredItems = computed(() => {
  if (filterMode.value === 'conflict') {
    return props.items.filter(itemHasConflict)
  }
  if (filterMode.value === 'excluded') {
    return props.items.filter(itemExcluded)
  }
  return props.items
})

const reorder = usePointerReorder({
  itemCount: () => props.items.length,
  onReorder: (payload) => emit('reorder', payload),
})

const thumbSize = computed(() => Math.round((112 * zoom.value) / 100))
const pageSizeOptions = computed(() => percentagePageSizeOptions(filteredItems.value.length))
const selectedFraction = computed({
  get: () => props.pageFraction,
  set: (value) => emit('update:page-fraction', value),
})
const pageSize = computed(() => pageSizeForFraction(filteredItems.value.length, selectedFraction.value))
const pageRange = computed(() => pageRangeForSize(filteredItems.value.length, pageSize.value, page.value))
const pageCount = computed(() => pageRange.value.pageCount)
const pagedItems = computed(() => filteredItems.value.slice(pageRange.value.start, pageRange.value.end))
const rangeLabel = computed(() =>
  filteredItems.value.length
    ? `${pageRange.value.start + 1}-${pageRange.value.end} / ${filteredItems.value.length}`
    : '0 / 0',
)

function itemGeometry(item) {
  const measured = dimensions[imageKey(item)]
  const width = Number(item?.width || measured?.width || 0)
  const height = Number(item?.height || measured?.height || 0)
  if (!props.preserveAspectRatio || !width || !height) {
    return { width: thumbSize.value, height: thumbSize.value }
  }
  const ratio = Math.min(12, Math.max(0.08, width / height))
  if (ratio >= 1) return { width: thumbSize.value, height: Math.max(18, Math.round(thumbSize.value / ratio)) }
  return { width: Math.max(18, Math.round(thumbSize.value * ratio)), height: thumbSize.value }
}

function cardStyle(item) {
  const geometry = itemGeometry(item)
  return { width: `${Math.max(146, geometry.width) + 18}px` }
}

function thumbWrapStyle(item) {
  const geometry = itemGeometry(item)
  return { width: `${geometry.width}px`, height: `${geometry.height}px` }
}

function globalIndex(localIndex) {
  if (filterMode.value === 'all') {
    return pageRange.value.start + localIndex
  }
  const item = pagedItems.value[localIndex]
  const idx = props.items.indexOf(item)
  return idx >= 0 ? idx : pageRange.value.start + localIndex
}

function itemPath(item) {
  if (props.pathResolver) return props.pathResolver(item)
  if (typeof item === 'string') return item
  return item?.path || ''
}

function itemName(item) {
  return props.nameResolver ? props.nameResolver(item) : fileName(itemPath(item))
}

function itemMeta(item) {
  if (props.metaResolver) return props.metaResolver(item)
  return item?.width && item?.height ? `${item.width}×${item.height}` : ''
}

function itemKey(item, localIndex) {
  return itemPath(item) || String(globalIndex(localIndex))
}

function imageSrc(item) {
  return sources[imageKey(item)] || ''
}

function itemExcluded(item) {
  return Boolean(props.excludedResolver?.(item))
}

function toggleExcluded(item, index) {
  emit('toggle-excluded', { item, index, excluded: !itemExcluded(item) })
}

function recordDimensions(item, event) {
  const path = itemPath(item)
  const target = event?.target
  if (!path || !target?.naturalWidth || !target?.naturalHeight) return
  dimensions[imageKey(item)] = { width: target.naturalWidth, height: target.naturalHeight }
}

function adjustZoom(delta) {
  zoom.value = Math.min(props.maxZoom, Math.max(props.minZoom, Math.round(zoom.value + delta)))
}

function itemRotation(item) {
  return Number(props.rotationResolver?.(itemPath(item)) || 0)
}
function imageKey(item) {
  return JSON.stringify([itemPath(item), itemRotation(item)])
}
function rotateItem(item) {
  if (!props.disabled && item) emit('rotate', { item, path: itemPath(item) })
}

async function preloadVisibleImages() {
  const keep = new Set(pagedItems.value.map(imageKey))
  for (const key of Object.keys(sources)) if (!keep.has(key)) delete sources[key]
  const queue = pagedItems.value
    .map((item) => ({ key: imageKey(item), path: itemPath(item), rotation: itemRotation(item) }))
    .filter(({ key, path }) => path && !sources[key] && !loadingPaths.has(key))
  await Promise.all(
    Array.from({ length: Math.min(3, queue.length) }, async () => {
      while (queue.length) {
        const { key, path, rotation } = queue.shift()
        loadingPaths.add(key)
        delete previewErrors[key]
        const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 640, rotationDegrees: rotation })
        loadingPaths.delete(key)
        if (!pagedItems.value.some((item) => imageKey(item) === key)) continue
        if (result.ok) sources[key] = result.data
        else previewErrors[key] = true
      }
    }),
  )
}

async function openPreview(item) {
  const path = itemPath(item)
  if (!path) return
  previewPath.value = path
  const annotation = props.annotationResolver?.(path)
  editTitle.value = annotation?.title || ''
  editDescription.value = annotation?.description || ''
  previewFit.value = true
  detailsOpen.value = false
  previewVisible.value = true
}

async function loadLargePreview(key) {
  previewLoadError.value = false
  previewSrc.value = largeSources[key] || ''
  previewNatural.width = 0
  previewNatural.height = 0
  if (!key || previewSrc.value || loadingLargePaths.has(key)) return
  loadingLargePaths.add(key)
  const [path, rotation] = JSON.parse(key)
  const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 4096, rotationDegrees: rotation })
  loadingLargePaths.delete(key)
  if (!previewVisible.value || !previewItem.value || imageKey(previewItem.value) !== key) return
  if (!result.ok) {
    previewLoadError.value = true
    return
  }
  for (const cachedKey of Object.keys(largeSources)) delete largeSources[cachedKey]
  largeSources[key] = result.data
  previewSrc.value = result.data
}

watch(() => (previewVisible.value && previewItem.value ? imageKey(previewItem.value) : ''), loadLargePreview)

function saveAnnotation() {
  if (!previewItem.value || props.disabled) return
  const path = itemPath(previewItem.value)
  if (!path) return
  emit('update-annotation', {
    item: previewItem.value,
    path,
    title: editTitle.value,
    description: editDescription.value,
  })
}

watch(pagedItems, preloadVisibleImages, { immediate: true })
watch([pageSize, () => props.items.length], () => {
  page.value = Math.min(pageCount.value, Math.max(1, page.value))
})
watch(selectedFraction, () => {
  page.value = 1
})
watch(filterMode, () => {
  page.value = 1
})
watch(page, async () => {
  await nextTick()
  if (scrollContainer.value) scrollContainer.value.scrollTop = 0
})
</script>

<style scoped>
.reorder-image-list {
  min-height: 0;
}
.reorder-image-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  margin-bottom: 10px;
  color: var(--docsy-text);
  font-size: 12px;
}
.reorder-image-hint {
  margin-right: auto;
  color: var(--docsy-text-muted);
}
.reorder-image-page-size {
  width: 88px;
}
.reorder-image-zoom {
  width: 110px;
}
.reorder-image-zoom-value {
  min-width: 42px;
}
.reorder-image-scroll {
  max-height: 360px;
  overflow: auto;
  padding: 8px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
}
.reorder-image-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: start;
}
.reorder-image-card {
  position: relative;
  flex: 0 0 auto;
  padding: 6px 8px;
  box-sizing: border-box;
  text-align: center;
  background: var(--docsy-surface-elevated);
  border: 1px solid transparent;
  border-radius: var(--docsy-radius);
}
.reorder-image-card.is-reorder-dragging {
  opacity: 0.55;
}
.reorder-image-card.is-reorder-before {
  box-shadow: inset 3px 0 0 var(--docsy-primary);
}
.reorder-image-card.is-reorder-after {
  box-shadow: inset -3px 0 0 var(--docsy-primary);
}
.reorder-image-handle {
  position: absolute;
  top: 3px;
  left: 6px;
  display: inline-grid;
  width: 28px;
  height: 24px;
  padding: 0;
  place-items: center;
  color: var(--docsy-text-muted);
  cursor: grab;
  touch-action: none;
  border: 0;
  background: transparent;
}
.reorder-image-handle:active {
  cursor: grabbing;
}
.reorder-image-preview {
  display: flex;
  width: 100%;
  padding: 0;
  align-items: center;
  flex-direction: column;
  color: inherit;
  font: inherit;
  border: 0;
  background: transparent;
  cursor: pointer;
}
.reorder-image-thumb-wrap {
  display: grid;
  margin: 0 auto 4px;
  place-items: center;
  overflow: hidden;
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
  transition:
    width 0.16s ease,
    height 0.16s ease;
}
.reorder-image-thumb {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.reorder-image-placeholder {
  color: var(--docsy-text-muted);
  font-size: 11px;
}
.reorder-image-name {
  display: block;
  width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
}
.reorder-image-meta {
  display: block;
  margin-top: 2px;
  color: var(--docsy-text-muted);
  font-size: 11px;
}
.reorder-image-top-actions {
  position: absolute;
  top: 3px;
  right: 6px;
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.reorder-image-action-btn {
  display: inline-grid;
  width: 24px;
  height: 24px;
  padding: 0;
  place-items: center;
  border: 0;
  border-radius: 6px;
  color: var(--docsy-text-muted);
  background: transparent;
  cursor: pointer;
}
.reorder-image-action-btn:hover {
  color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}
.reorder-image-decision {
  display: inline-grid;
  width: 24px;
  height: 24px;
  padding: 0;
  place-items: center;
  border: 0;
  border-radius: 6px;
  color: var(--docsy-text-muted);
  background: transparent;
  cursor: pointer;
}
.reorder-image-page-badge {
  position: absolute;
  top: 4px;
  left: 36px;
  height: 20px;
  padding: 0 6px;
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  font-weight: 600;
  border-radius: 4px;
  border: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
  cursor: pointer;
  transition: all 0.15s ease;
  z-index: 2;
}
.reorder-image-page-badge:hover {
  background: var(--docsy-primary-soft);
  color: var(--docsy-primary);
  border-color: var(--docsy-primary);
}
.badge-dot {
  font-size: 8px;
  color: var(--docsy-primary);
}
.badge-color-yellow {
  border-color: rgba(183, 121, 52, 0.45);
  color: #8a541c;
  background: rgba(253, 246, 236, 0.95);
}
.badge-color-green {
  border-color: rgba(79, 125, 90, 0.45);
  color: #395c41;
  background: rgba(234, 243, 222, 0.95);
}
.badge-color-red {
  border-color: rgba(181, 82, 75, 0.45);
  color: #943b35;
  background: rgba(252, 235, 235, 0.95);
}
.badge-color-blue {
  border-color: rgba(82, 121, 153, 0.45);
  color: #2d5873;
  background: rgba(237, 244, 248, 0.95);
}

.card-conflict-yellow {
  border-color: rgba(183, 121, 52, 0.65) !important;
  box-shadow: 0 0 0 1px rgba(183, 121, 52, 0.16);
}
.card-conflict-green {
  border-color: rgba(79, 125, 90, 0.65) !important;
  box-shadow: 0 0 0 1px rgba(79, 125, 90, 0.16);
}
.card-conflict-red {
  border-color: rgba(181, 82, 75, 0.7) !important;
  box-shadow: 0 0 0 1px rgba(181, 82, 75, 0.2);
}
.card-conflict-blue {
  border-color: rgba(82, 121, 153, 0.55) !important;
  box-shadow: 0 0 0 1px rgba(82, 121, 153, 0.15);
}

.reorder-image-handle.is-disabled {
  cursor: not-allowed;
  opacity: 0.35;
}

.reorder-image-card-header {
  display: flex;
  align-items: center;
  gap: 2px;
  min-height: 26px;
  margin-bottom: 4px;
}
.reorder-image-card-header .reorder-image-handle,
.reorder-image-card-header .reorder-image-page-badge,
.reorder-image-card-header .reorder-image-top-actions {
  position: static;
  flex-shrink: 0;
}
.reorder-image-card-header .reorder-image-top-actions {
  margin-left: auto;
}
.reorder-image-action-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.reorder-image-detail-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}
.reorder-image-detail-toolbar .el-button + .el-button {
  margin-left: 0;
}
.reorder-image-detail-zoom {
  width: 140px;
  margin: 0 4px;
}
.reorder-image-detail-percent {
  min-width: 42px;
  font-variant-numeric: tabular-nums;
}
.reorder-image-detail-angle {
  color: var(--docsy-text-muted);
  font-size: 12px;
}
.reorder-image-detail-toggle {
  margin-left: auto;
}
.reorder-image-dialog-layout {
  display: flex;
  gap: 16px;
  min-height: 0;
  flex: 1;
}
.reorder-image-dialog-preview {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: auto;
  touch-action: none;
  user-select: none;
  cursor: grab;
  background: var(--docsy-surface-muted);
  border-radius: var(--docsy-radius);
  border: 1px solid var(--docsy-border-subtle);
}
.reorder-image-dialog-preview.is-panning {
  cursor: grabbing;
}
.reorder-image-dialog-stage {
  display: grid;
  place-items: center;
  box-sizing: border-box;
  padding: 12px;
}
.reorder-image-dialog-img {
  display: block;
  max-width: none;
  max-height: none;
  object-fit: contain;
}
.reorder-image-dialog-form {
  flex: 0 0 250px;
  min-width: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
@media (max-width: 800px) {
  .reorder-image-dialog-layout {
    flex-direction: column;
  }
  .reorder-image-dialog-form {
    flex-basis: auto;
    max-height: 28vh;
  }
  .reorder-image-detail-zoom {
    width: 90px;
  }
}

.form-section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--docsy-text);
  border-bottom: 1px solid var(--docsy-border-subtle);
  padding-bottom: 6px;
  margin-bottom: 2px;
}

.dialog-meta-info {
  margin-top: 8px;
  padding: 10px 12px;
  background: var(--docsy-surface-muted);
  border-radius: var(--docsy-radius);
  font-size: 12px;
  color: var(--docsy-text-muted);
  display: flex;
  flex-direction: column;
  gap: 4px;
  word-break: break-all;
}

.reorder-image-dialog-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.reorder-image-dialog-buttons {
  display: flex;
  gap: 8px;
}
</style>

<style>
/* The dialog is teleported outside the grid; keep these rules specific to its root. */
.el-dialog.reorder-image-detail-dialog {
  height: 94dvh;
  max-width: 1800px;
  margin-bottom: 0;
  display: flex;
  flex-direction: column;
}
.reorder-image-detail-dialog .el-dialog__body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.reorder-image-detail-dialog .el-dialog__header {
  flex-shrink: 0;
}
.reorder-image-detail-dialog .el-dialog__title {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.reorder-image-detail-dialog .el-dialog__footer {
  flex-shrink: 0;
}
</style>
