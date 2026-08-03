<template>
  <div class="reorder-image-list">
    <div class="reorder-image-toolbar">
      <span>{{ rangeLabel }}</span>
      <span class="reorder-image-hint">拖动手柄调整顺序</span>
      <el-select v-model="pageSize" size="small" class="reorder-image-page-size">
        <el-option v-for="size in pageSizeOptions" :key="size" :label="`${size} 张`" :value="size" />
      </el-select>
      <el-button size="small" text @click="adjustZoom(-10)">-</el-button>
      <el-slider v-model="zoom" :min="minZoom" :max="maxZoom" :step="5" class="reorder-image-zoom" />
      <el-button size="small" text @click="adjustZoom(10)">+</el-button>
      <span class="reorder-image-zoom-value">{{ zoom }}%</span>
    </div>

    <div v-if="items.length" class="reorder-image-scroll">
      <div class="reorder-image-grid" :style="gridStyle">
        <article
          v-for="(item, localIndex) in pagedItems"
          :key="itemKey(item, localIndex)"
          class="reorder-image-card"
          :class="reorder.itemClasses(globalIndex(localIndex))"
          :style="cardStyle"
          :data-reorder-index="globalIndex(localIndex)"
        >
          <button
            type="button"
            class="reorder-image-handle"
            title="拖动调整顺序"
            aria-label="拖动调整顺序"
            @pointerdown.stop="reorder.start(globalIndex(localIndex), $event)"
            @pointermove.stop="reorder.move"
            @pointerup.stop="reorder.finish"
            @pointercancel.stop="reorder.reset"
          >
            <el-icon><Rank /></el-icon>
          </button>
          <button type="button" class="reorder-image-preview" @click="openPreview(item)">
            <span class="reorder-image-thumb-wrap" :style="thumbWrapStyle">
              <img v-if="imageSrc(item)" :src="imageSrc(item)" :alt="itemName(item)" class="reorder-image-thumb" />
              <span v-else class="reorder-image-placeholder">预览中</span>
            </span>
            <span class="reorder-image-name" :title="itemName(item)">{{ itemName(item) }}</span>
            <span v-if="itemMeta(item)" class="reorder-image-meta">{{ itemMeta(item) }}</span>
          </button>
          <div class="reorder-image-actions" aria-label="顺序调整">
            <el-button
              text
              size="small"
              :disabled="globalIndex(localIndex) === 0"
              aria-label="上移"
              @click="moveBy(globalIndex(localIndex), -1)"
            >
              <el-icon><ArrowUp /></el-icon>
            </el-button>
            <el-button
              text
              size="small"
              :disabled="globalIndex(localIndex) === items.length - 1"
              aria-label="下移"
              @click="moveBy(globalIndex(localIndex), 1)"
            >
              <el-icon><ArrowDown /></el-icon>
            </el-button>
          </div>
        </article>
      </div>
    </div>

    <el-empty v-else :description="emptyDescription" :image-size="80" />

    <div v-if="items.length" class="reorder-image-pager">
      <el-button size="small" :disabled="page <= 1" @click="page -= 1">上一页</el-button>
      <span>第 {{ page }} / {{ pageCount }} 页</span>
      <el-button size="small" :disabled="page >= pageCount" @click="page += 1">下一页</el-button>
    </div>

    <el-dialog v-model="previewVisible" title="图片预览" width="80%" destroy-on-close>
      <div class="reorder-image-dialog-body">
        <img v-if="previewSrc" :src="previewSrc" class="reorder-image-dialog-img" />
      </div>
    </el-dialog>
  </div>
</template>

<script setup>
import { computed, reactive, ref, watch } from 'vue'
import { ArrowDown, ArrowUp, Rank } from '@element-plus/icons-vue'
import { tauriCallSafe } from '../../core/tauriBridge.js'
import { fileName } from '../../core/filePath.js'
import { usePointerReorder } from '../../core/composables/usePointerReorder.js'

const props = defineProps({
  items: { type: Array, default: () => [] },
  nameResolver: { type: Function, default: null },
  metaResolver: { type: Function, default: null },
  pathResolver: { type: Function, default: null },
  emptyDescription: { type: String, default: '暂无图片' },
  pageSizeOptions: { type: Array, default: () => [24, 48, 96] },
  initialPageSize: { type: Number, default: 24 },
  initialZoom: { type: Number, default: 100 },
  minZoom: { type: Number, default: 60 },
  maxZoom: { type: Number, default: 180 },
})

const emit = defineEmits(['reorder'])
const page = ref(1)
const pageSize = ref(props.initialPageSize)
const zoom = ref(props.initialZoom)
const sources = reactive({})
const loadingPaths = new Set()
const previewVisible = ref(false)
const previewSrc = ref('')
const reorder = usePointerReorder({
  itemCount: () => props.items.length,
  onReorder: (payload) => emit('reorder', payload),
})

const pageCount = computed(() => Math.max(1, Math.ceil(props.items.length / pageSize.value)))
const pageStart = computed(() => Math.min((page.value - 1) * pageSize.value, props.items.length))
const pageEnd = computed(() => Math.min(pageStart.value + pageSize.value, props.items.length))
const pagedItems = computed(() => props.items.slice(pageStart.value, pageEnd.value))
const rangeLabel = computed(() =>
  props.items.length ? `${pageStart.value + 1}-${pageEnd.value} / ${props.items.length}` : '0 / 0',
)
const cardSize = computed(() => Math.round((112 * zoom.value) / 100))
const thumbSize = computed(() => Math.round((72 * zoom.value) / 100))
const gridStyle = computed(() => ({ gridTemplateColumns: `repeat(auto-fill, minmax(${cardSize.value}px, 1fr))` }))
const cardStyle = computed(() => ({ minHeight: `${cardSize.value + 54}px` }))
const thumbWrapStyle = computed(() => ({ width: `${thumbSize.value}px`, height: `${thumbSize.value}px` }))

function globalIndex(localIndex) {
  return pageStart.value + localIndex
}

function moveBy(index, delta) {
  const to = index + delta
  if (to < 0 || to >= props.items.length) return
  emit('reorder', { from: index, to })
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
      const result = await tauriCallSafe('read_image_data_url', { path })
      loadingPaths.delete(path)
      if (result.ok) sources[path] = result.data
    }
  })
  await Promise.all(workers)
}

function openPreview(item) {
  const src = imageSrc(item)
  if (!src) return
  previewSrc.value = src
  previewVisible.value = true
}

watch([pagedItems, pageSize], preloadVisibleImages, { immediate: true })
watch([pageSize, () => props.items.length], () => {
  page.value = Math.min(pageCount.value, Math.max(1, page.value))
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
  border-radius: 6px;
  background: var(--docsy-surface-muted);
}
.reorder-image-grid {
  display: grid;
  gap: 8px;
  align-items: start;
}
.reorder-image-card {
  position: relative;
  min-width: 0;
  padding: 30px 8px 6px;
  text-align: center;
  background: var(--docsy-surface-elevated);
  border: 1px solid transparent;
  border-radius: 4px;
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
  display: block;
  width: 100%;
  padding: 0;
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
  border-radius: 4px;
  background: var(--docsy-surface-muted);
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
.reorder-image-actions {
  position: absolute;
  top: 1px;
  right: 2px;
  display: flex;
}
.reorder-image-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
.reorder-image-pager {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 10px;
  font-size: 12px;
  color: var(--docsy-text-muted);
}
.reorder-image-dialog-body {
  display: grid;
  min-height: 320px;
  place-items: center;
  overflow: auto;
}
.reorder-image-dialog-img {
  max-width: 100%;
  max-height: 72vh;
  object-fit: contain;
}
</style>
