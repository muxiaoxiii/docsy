<template>
  <div class="image-preview-list">
    <div class="image-preview-toolbar">
      <span>{{ rangeLabel }}</span>
      <el-select v-model="pageSize" size="small" class="image-preview-page-size">
        <el-option v-for="size in pageSizeOptions" :key="size" :label="`${size} 张`" :value="size" />
      </el-select>
      <el-button size="small" text @click="adjustZoom(-10)">-</el-button>
      <el-slider v-model="zoom" :min="minZoom" :max="maxZoom" :step="5" class="image-preview-zoom" />
      <el-button size="small" text @click="adjustZoom(10)">+</el-button>
      <span class="image-preview-zoom-value">{{ zoom }}%</span>
    </div>

    <div v-if="items.length" class="image-preview-scroll">
      <div class="image-preview-grid" :style="gridStyle">
        <button
          v-for="(item, idx) in pagedItems"
          :key="itemKey(item, idx)"
          class="image-preview-card"
          :style="cardStyle"
          type="button"
          @click="openPreview(item)"
        >
          <span class="image-preview-thumb-wrap" :style="thumbWrapStyle">
            <img
              v-if="imageSrc(item)"
              :src="imageSrc(item)"
              :alt="itemName(item)"
              class="image-preview-thumb"
            />
            <span v-else class="image-preview-placeholder">预览中</span>
          </span>
          <span class="image-preview-name" :title="itemName(item)">{{ itemName(item) }}</span>
          <span v-if="itemMeta(item)" class="image-preview-meta">{{ itemMeta(item) }}</span>
        </button>
      </div>
    </div>

    <el-empty v-else :description="emptyDescription" :image-size="80" />

    <div v-if="items.length" class="image-preview-pager">
      <el-button size="small" :disabled="page <= 1" @click="page -= 1">上一页</el-button>
      <span>第 {{ page }} / {{ pageCount }} 页</span>
      <el-button size="small" :disabled="page >= pageCount" @click="page += 1">下一页</el-button>
    </div>

    <el-dialog v-model="previewVisible" title="图片预览" width="80%" destroy-on-close>
      <div class="image-preview-dialog-body">
        <img v-if="previewSrc" :src="previewSrc" class="image-preview-dialog-img" />
      </div>
    </el-dialog>
  </div>
</template>

<script setup>
import { computed, reactive, ref, watch } from 'vue'
import { tauriCallSafe } from '../../core/tauriBridge.js'

const props = defineProps({
  items: {
    type: Array,
    default: () => [],
  },
  nameResolver: {
    type: Function,
    default: null,
  },
  metaResolver: {
    type: Function,
    default: null,
  },
  pathResolver: {
    type: Function,
    default: null,
  },
  emptyDescription: {
    type: String,
    default: '暂无图片',
  },
  pageSizeOptions: {
    type: Array,
    default: () => [24, 48, 96],
  },
  initialPageSize: {
    type: Number,
    default: 24,
  },
  initialZoom: {
    type: Number,
    default: 100,
  },
  minZoom: {
    type: Number,
    default: 60,
  },
  maxZoom: {
    type: Number,
    default: 180,
  },
})

const page = ref(1)
const pageSize = ref(props.initialPageSize)
const zoom = ref(props.initialZoom)
const sources = reactive({})
const previewVisible = ref(false)
const previewSrc = ref('')

const pageCount = computed(() => Math.max(1, Math.ceil(props.items.length / pageSize.value)))
const pageStart = computed(() => Math.min((page.value - 1) * pageSize.value, props.items.length))
const pageEnd = computed(() => Math.min(pageStart.value + pageSize.value, props.items.length))
const pagedItems = computed(() => props.items.slice(pageStart.value, pageEnd.value))
const rangeLabel = computed(() => {
  if (!props.items.length) return '0 / 0'
  return `${pageStart.value + 1}-${pageEnd.value} / ${props.items.length}`
})
const cardSize = computed(() => Math.round(112 * zoom.value / 100))
const thumbSize = computed(() => Math.round(72 * zoom.value / 100))
const gridStyle = computed(() => ({
  gridTemplateColumns: `repeat(auto-fill, minmax(${cardSize.value}px, 1fr))`,
}))
const cardStyle = computed(() => ({
  minHeight: `${cardSize.value + 44}px`,
}))
const thumbWrapStyle = computed(() => ({
  width: `${thumbSize.value}px`,
  height: `${thumbSize.value}px`,
}))

function itemPath(item) {
  if (props.pathResolver) return props.pathResolver(item)
  if (typeof item === 'string') return item
  return item?.path || ''
}

function itemName(item) {
  if (props.nameResolver) return props.nameResolver(item)
  const path = itemPath(item)
  return String(path || '').split(/[\\/]/).pop() || path
}

function itemMeta(item) {
  if (props.metaResolver) return props.metaResolver(item)
  if (item?.width && item?.height) return `${item.width}×${item.height}`
  return ''
}

function itemKey(item, idx) {
  return itemPath(item) || `${idx}`
}

function imageSrc(item) {
  return sources[itemPath(item)] || ''
}

function adjustZoom(delta) {
  zoom.value = Math.min(props.maxZoom, Math.max(props.minZoom, Math.round(zoom.value + delta)))
}

async function preloadVisibleImages() {
  await Promise.all(pagedItems.value.map(async (item) => {
    const path = itemPath(item)
    if (!path || sources[path]) return
    const result = await tauriCallSafe('read_image_data_url', { path })
    if (result.ok) {
      sources[path] = result.data
    }
  }))
}

function openPreview(item) {
  const src = imageSrc(item)
  if (!src) return
  previewSrc.value = src
  previewVisible.value = true
}

watch([pagedItems, pageSize], () => {
  preloadVisibleImages()
}, { immediate: true })

watch([pageSize, () => props.items.length], () => {
  if (page.value > pageCount.value) page.value = pageCount.value
  if (page.value < 1) page.value = 1
})
</script>

<style scoped>
.image-preview-list {
  min-height: 0;
}

.image-preview-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  margin-bottom: 10px;
  color: #606266;
  font-size: 12px;
}

.image-preview-page-size {
  width: 88px;
}

.image-preview-zoom {
  width: 110px;
}

.image-preview-zoom-value {
  min-width: 42px;
}

.image-preview-scroll {
  max-height: 360px;
  overflow: auto;
  padding: 8px;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  background: #fafafa;
}

.image-preview-grid {
  display: grid;
  gap: 8px;
  align-items: start;
}

.image-preview-card {
  min-width: 0;
  text-align: center;
  padding: 8px;
  background: #f5f7fa;
  border: 1px solid transparent;
  border-radius: 4px;
  cursor: pointer;
  color: inherit;
  font: inherit;
}

.image-preview-card:hover {
  border-color: #c6e2ff;
  background: #ecf5ff;
}

.image-preview-thumb-wrap {
  margin: 0 auto 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #e4e7ed;
  overflow: hidden;
}

.image-preview-thumb {
  width: 100%;
  height: 100%;
  display: block;
  object-fit: contain;
}

.image-preview-placeholder {
  font-size: 11px;
  color: #909399;
}

.image-preview-name {
  display: block;
  font-size: 11px;
  color: #606266;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.image-preview-meta {
  font-size: 10px;
  color: #c0c4cc;
}

.image-preview-pager {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 10px;
  color: #606266;
  font-size: 12px;
}

.image-preview-dialog-body {
  display: flex;
  justify-content: center;
  align-items: center;
  max-height: 70vh;
}

.image-preview-dialog-img {
  max-width: 100%;
  max-height: 70vh;
  object-fit: contain;
}
</style>
