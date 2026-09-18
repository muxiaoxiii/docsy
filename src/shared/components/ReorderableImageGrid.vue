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
          :class="[
            reorder.itemClasses(globalIndex(localIndex)),
            cardConflictClass(item),
          ]"
          :style="cardStyle(item)"
          :data-reorder-index="globalIndex(localIndex)"
        >
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
                {{ previewErrors[itemPath(item)] ? '预览失败' : '预览中' }}
              </span>
            </span>
            <span class="reorder-image-name" :title="itemName(item)">{{ itemName(item) }}</span>
            <span v-if="itemExcluded(item)" class="reorder-image-status">已排除，不参与排版</span>
            <span v-if="itemMeta(item)" class="reorder-image-meta">{{ itemMeta(item) }}</span>
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
      :title="previewItem ? (itemName(previewItem) || '图片详情与批注') : '图片预览'"
      width="860px"
      destroy-on-close
    >
      <div class="reorder-image-dialog-layout">
        <div class="reorder-image-dialog-preview">
          <img v-if="previewSrc" :src="previewSrc" class="reorder-image-dialog-img" />
          <span v-else class="reorder-image-placeholder">正在载入高清图片</span>
        </div>
        <div v-if="previewItem && annotationResolver" class="reorder-image-dialog-form">
          <div class="form-section-title">图注与说明设置</div>
          <el-form label-position="top" size="small">
            <el-form-item label="图片标题">
              <el-input
                v-model="editTitle"
                placeholder="留空则使用默认文件名"
                @input="saveAnnotation"
                clearable
              />
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
            <div v-if="pageBadge(previewItem)"><strong>所在页：</strong>第 {{ pageBadge(previewItem).pageNumber }} 页</div>
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
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { Delete, Rank, RefreshLeft, ZoomIn } from '@element-plus/icons-vue'
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
})

const emit = defineEmits(['reorder', 'toggle-excluded', 'update:page-fraction', 'select-page', 'select-item', 'update-annotation'])

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

function handleCardClick(item, event) {
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
const previewItem = ref(null)
const previewIndex = ref(-1)
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
  filteredItems.value.length ? `${pageRange.value.start + 1}-${pageRange.value.end} / ${filteredItems.value.length}` : '0 / 0',
)

function itemGeometry(item) {
  const path = itemPath(item)
  const measured = dimensions[path]
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
  return { width: `${Math.max(76, geometry.width) + 18}px` }
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
  return sources[itemPath(item)] || ''
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
  dimensions[path] = { width: target.naturalWidth, height: target.naturalHeight }
}

function adjustZoom(delta) {
  zoom.value = Math.min(props.maxZoom, Math.max(props.minZoom, Math.round(zoom.value + delta)))
}

async function preloadVisibleImages() {
  const paths = pagedItems.value.map(itemPath).filter(Boolean)
  const keep = new Set(paths)
  for (const path of Object.keys(sources)) {
    if (!keep.has(path)) delete sources[path]
  }
  const queue = paths.filter((path) => !sources[path] && !loadingPaths.has(path))
  const workers = Array.from({ length: Math.min(3, queue.length) }, async () => {
    while (queue.length) {
      const path = queue.shift()
      if (!path) return
      loadingPaths.add(path)
      delete previewErrors[path]
      const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 640 })
      loadingPaths.delete(path)

      const currentPaths = pagedItems.value.map(itemPath).filter(Boolean)
      if (!currentPaths.includes(path)) continue

      if (result.ok) sources[path] = result.data
      else previewErrors[path] = true
    }
  })
  await Promise.all(workers)
}

async function openPreview(item) {
  const path = itemPath(item)
  if (!path) return
  previewItem.value = item
  previewIndex.value = props.items.indexOf(item)
  if (props.annotationResolver) {
    const ann = props.annotationResolver(path)
    editTitle.value = ann?.title || ''
    editDescription.value = ann?.description || ''
  } else {
    editTitle.value = ''
    editDescription.value = ''
  }
  previewVisible.value = true
  previewSrc.value = largeSources[path] || ''
  if (previewSrc.value || loadingLargePaths.has(path)) return
  loadingLargePaths.add(path)
  const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 3840 })
  loadingLargePaths.delete(path)
  if (!result.ok || previewItem.value !== item) return
  for (const cachedPath of Object.keys(largeSources)) delete largeSources[cachedPath]
  largeSources[path] = result.data
  previewSrc.value = result.data
}

function saveAnnotation() {
  if (!previewItem.value) return
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
  padding: 30px 8px 6px;
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

.reorder-image-dialog-layout {
  display: flex;
  gap: 20px;
  align-items: flex-start;
  min-height: 280px;
}

.reorder-image-dialog-preview {
  flex: 1 1 55%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--docsy-surface-muted);
  border-radius: var(--docsy-radius);
  border: 1px solid var(--docsy-border-subtle);
  min-height: 260px;
  max-height: 480px;
  overflow: hidden;
  padding: 8px;
}

.reorder-image-dialog-img {
  max-width: 100%;
  max-height: 460px;
  object-fit: contain;
  border-radius: 4px;
}

.reorder-image-dialog-form {
  flex: 1 1 45%;
  display: flex;
  flex-direction: column;
  gap: 12px;
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
