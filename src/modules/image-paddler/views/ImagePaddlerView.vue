<template>
  <div class="image-paddler-view">
    <div class="paddler-layout">
      <!-- Settings Panel -->
      <div class="settings-panel">
        <h3>图片排版</h3>
        <p class="hint">将图片批量排版为 A4 文档</p>

        <el-form label-width="80px" size="small">
          <el-form-item label="文件夹">
            <el-button @click="selectFolder">选择文件夹</el-button>
            <span class="folder-path" v-if="folders.length">{{ folders.join('；') }}</span>
          </el-form-item>

          <el-form-item label="输出格式">
            <el-select v-model="settings.output_format">
              <el-option label="DOCX" value="docx" />
              <el-option label="PDF" value="pdf" />
            </el-select>
          </el-form-item>

          <el-form-item label="每页张数">
            <el-select v-model="settings.layout">
              <el-option label="1 张" value="1" />
              <el-option label="2 张（左右）" value="1x2" />
              <el-option label="2 张（上下）" value="2x1" />
              <el-option label="3 张（左右）" value="1x3" />
              <el-option label="4 张" value="4" />
              <el-option label="6 张" value="2x3" />
              <el-option label="9 张" value="3x3" />
              <el-option label="自定义" value="custom" />
            </el-select>
          </el-form-item>

          <el-form-item v-if="settings.layout === 'custom'" label="行列">
            <div class="inline-controls">
              <el-input-number v-model="settings.custom_rows" :min="1" :max="8" />
              <span>行</span>
              <el-input-number v-model="settings.custom_cols" :min="1" :max="8" />
              <span>列</span>
            </div>
          </el-form-item>

          <el-form-item label="缩放模式">
            <el-select v-model="settings.scale_mode">
              <el-option label="适应页面" value="fit" />
              <el-option label="不缩放" value="original" />
            </el-select>
          </el-form-item>

          <el-form-item label="方向">
            <el-select v-model="settings.orientation">
              <el-option label="自动" value="auto" />
              <el-option label="竖向" value="portrait" />
              <el-option label="横向" value="landscape" />
            </el-select>
          </el-form-item>

          <el-form-item label="页边距">
            <el-input-number v-model="settings.margin_mm" :min="0" :max="30" :step="1" />
            <span class="unit-label">mm</span>
          </el-form-item>

          <el-form-item label="文件名">
            <el-switch v-model="settings.show_filename" />
          </el-form-item>

          <el-form-item label="扩展名">
            <el-switch v-model="settings.filename_without_ext" active-text="隐藏" inactive-text="保留" />
          </el-form-item>

          <el-form-item label="删文字">
            <el-input v-model="settings.filename_remove_text" placeholder="从嵌入文件名中删除这些字" />
          </el-form-item>

          <el-form-item label="排列">
            <el-select v-model="settings.order_mode">
              <el-option label="Z 字" value="z" />
              <el-option label="N 字" value="n" />
              <el-option label="倒 N 字" value="reverse_n" />
            </el-select>
          </el-form-item>

          <el-form-item label="边框">
            <div class="inline-controls">
              <el-switch v-model="settings.border_enabled" />
              <el-select v-model="settings.border_color" :disabled="!settings.border_enabled">
                <el-option label="黑色" value="black" />
                <el-option label="白色" value="white" />
              </el-select>
            </div>
          </el-form-item>

          <el-form-item>
            <el-button type="primary" @click="analyze" :loading="analyzing" :disabled="!folders.length">
              分析
            </el-button>
            <el-button type="success" @click="run" :loading="generating" :disabled="!analysis">
              生成文档
            </el-button>
          </el-form-item>
        </el-form>
      </div>

      <!-- Analysis Result -->
      <div class="result-panel">
        <template v-if="analysis">
          <div v-if="generatedResult" class="generated-result">
            <div>
              <strong>已生成</strong>
              <div class="output-path">{{ generatedResult.output_path }}</div>
            </div>
            <el-button size="small" type="primary" @click="openGeneratedOutput">打开文件</el-button>
          </div>

          <div class="analysis-summary">
            <el-descriptions :column="2" border size="small">
              <el-descriptions-item label="图片数量">{{ analysis.images.length }}</el-descriptions-item>
              <el-descriptions-item label="分组数">{{ analysis.groups.length }}</el-descriptions-item>
              <el-descriptions-item label="推荐方向">{{ orientationLabel(analysis.recommended.orientation) }}</el-descriptions-item>
              <el-descriptions-item label="推荐布局">{{ layoutLabel(analysis.recommended.layout) }}</el-descriptions-item>
              <el-descriptions-item label="推荐缩放">{{ scaleModeLabel(analysis.recommended.scale_mode) }}</el-descriptions-item>
              <el-descriptions-item label="推荐边距">{{ analysis.recommended.margin_mm }} mm</el-descriptions-item>
              <el-descriptions-item label="当前方向">{{ resolvedOrientationLabel }}</el-descriptions-item>
              <el-descriptions-item label="当前布局">{{ layoutGrid.rows }} 行 × {{ layoutGrid.cols }} 列</el-descriptions-item>
            </el-descriptions>
            <div class="recommendation-bar">
              <span>{{ analysis.recommended.reason }}</span>
              <el-button size="small" type="primary" @click="applyRecommendedSettings">应用推荐参数</el-button>
            </div>
          </div>

          <div class="preview-section">
            <div class="section-head">
              <h4>第一页预览</h4>
              <div class="preview-toolbar">
                <span>{{ previewImages.length }} / {{ layoutGrid.rows * layoutGrid.cols }} 张</span>
                <el-button size="small" text @click="adjustPageZoom(-10)">-</el-button>
                <el-slider v-model="pageZoom" :min="50" :max="180" :step="5" class="zoom-slider" />
                <el-button size="small" text @click="adjustPageZoom(10)">+</el-button>
                <span class="zoom-value">{{ pageZoom }}%</span>
              </div>
            </div>
            <div class="page-preview-shell">
              <div class="page-preview" :class="resolvedOrientation" :style="previewPageStyle">
                <div class="preview-grid" :style="previewGridStyle">
                  <div
                    v-for="(img, idx) in previewSlots"
                    :key="idx"
                    class="preview-cell"
                    :class="{
                      'preview-cell-bordered': settings.border_enabled,
                      'preview-cell-white-border': settings.border_enabled && settings.border_color === 'white',
                      'preview-cell-no-name': !settings.show_filename,
                    }"
                  >
                    <template v-if="img">
                      <img :src="imageSrc(img.path)" :alt="fileName(img.path)" :style="previewImageStyle(img)" />
                      <div v-if="settings.show_filename" class="preview-name">{{ fileName(img.path) }}</div>
                    </template>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Groups -->
          <div v-if="analysis.groups.length > 1" class="groups-section">
            <h4>文件分组</h4>
            <div v-for="group in analysis.groups" :key="group.prefix" class="group-item">
              <span>{{ group.prefix }}</span>
              <el-tag size="small">{{ group.count }} 张</el-tag>
            </div>
          </div>

          <!-- Image List -->
          <div class="image-list">
            <div class="section-head">
              <h4>图片预览</h4>
              <div class="preview-toolbar">
                <span>{{ imagePageStart + 1 }}-{{ imagePageEnd }} / {{ orderedImages.length }}</span>
                <el-select v-model="imagePageSize" size="small" class="page-size-select">
                  <el-option label="24 张" :value="24" />
                  <el-option label="48 张" :value="48" />
                  <el-option label="96 张" :value="96" />
                </el-select>
                <el-button size="small" text @click="adjustImageZoom(-10)">-</el-button>
                <el-slider v-model="imageZoom" :min="60" :max="180" :step="5" class="zoom-slider" />
                <el-button size="small" text @click="adjustImageZoom(10)">+</el-button>
                <span class="zoom-value">{{ imageZoom }}%</span>
              </div>
            </div>
            <div class="image-grid-scroll">
              <div class="image-grid" :style="imageGridStyle">
                <div v-for="(img, idx) in pagedImages" :key="img.path || idx" class="image-thumb" :style="imageThumbStyle">
                  <img :src="imageSrc(img.path)" :alt="fileName(img.path)" class="thumb-image" :style="thumbImageStyle" />
                  <span class="thumb-name">{{ fileName(img.path) }}</span>
                  <span class="thumb-size">{{ img.width }}×{{ img.height }}</span>
                </div>
              </div>
            </div>
            <div class="pager-row">
              <el-button size="small" :disabled="imagePage <= 1" @click="imagePage -= 1">上一页</el-button>
              <span>第 {{ imagePage }} / {{ imagePageCount }} 页</span>
              <el-button size="small" :disabled="imagePage >= imagePageCount" @click="imagePage += 1">下一页</el-button>
            </div>
          </div>
        </template>
        <el-empty v-else description="选择文件夹后点击分析" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, reactive, watch, onMounted, onBeforeUnmount } from 'vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ElMessage } from 'element-plus'

const folder = ref('')
const folders = ref([])
const analyzing = ref(false)
const generating = ref(false)
const analysis = ref(null)
const generatedResult = ref(null)
const previewSources = reactive({})
const pageZoom = ref(100)
const imageZoom = ref(100)
const imagePage = ref(1)
const imagePageSize = ref(24)
let unlistenDragDrop = null

const settings = reactive({
  output_format: 'pdf',
  layout: '2x1',
  custom_rows: 2,
  custom_cols: 2,
  scale_mode: 'fit',
  orientation: 'auto',
  dpi: 300,
  margin_mm: 12,
  show_filename: true,
  filename_without_ext: true,
  filename_remove_text: '',
  order_mode: 'z',
  border_enabled: false,
  border_color: 'black',
})

const layoutGrid = computed(() => parseLayout(settings.layout, settings.custom_rows, settings.custom_cols))
const resolvedOrientation = computed(() => {
  if (settings.orientation !== 'auto') return settings.orientation
  return analysis.value?.recommended?.orientation || 'portrait'
})
const resolvedOrientationLabel = computed(() => resolvedOrientation.value === 'landscape' ? '横向' : '竖向')
const orderedImages = computed(() => reorderImages(analysis.value?.images || [], layoutGrid.value, settings.order_mode))
const previewImages = computed(() => orderedImages.value.slice(0, layoutGrid.value.rows * layoutGrid.value.cols))
const imagePageCount = computed(() => Math.max(1, Math.ceil(orderedImages.value.length / imagePageSize.value)))
const imagePageStart = computed(() => Math.min((imagePage.value - 1) * imagePageSize.value, orderedImages.value.length))
const imagePageEnd = computed(() => Math.min(imagePageStart.value + imagePageSize.value, orderedImages.value.length))
const pagedImages = computed(() => orderedImages.value.slice(imagePageStart.value, imagePageEnd.value))
const previewSlots = computed(() => {
  const slots = [...previewImages.value]
  while (slots.length < layoutGrid.value.rows * layoutGrid.value.cols) slots.push(null)
  return slots
})
const previewPageStyle = computed(() => ({
  aspectRatio: resolvedOrientation.value === 'landscape' ? '297 / 210' : '210 / 297',
  padding: `${Math.max(0, settings.margin_mm) * 1.2}px`,
  width: `${pageZoom.value}%`,
  minWidth: '220px',
}))
const previewGridStyle = computed(() => ({
  gridTemplateColumns: `repeat(${layoutGrid.value.cols}, minmax(0, 1fr))`,
  gridTemplateRows: `repeat(${layoutGrid.value.rows}, minmax(0, 1fr))`,
}))
const thumbCardSize = computed(() => Math.round(112 * imageZoom.value / 100))
const thumbImageSize = computed(() => Math.round(72 * imageZoom.value / 100))
const imageGridStyle = computed(() => ({
  gridTemplateColumns: `repeat(auto-fill, minmax(${thumbCardSize.value}px, 1fr))`,
}))
const imageThumbStyle = computed(() => ({
  minHeight: `${thumbCardSize.value + 38}px`,
}))
const thumbImageStyle = computed(() => ({
  width: `${thumbImageSize.value}px`,
  height: `${thumbImageSize.value}px`,
}))
const layoutMetrics = computed(() => {
  const page = resolvedOrientation.value === 'landscape'
    ? { width: 297, height: 210 }
    : { width: 210, height: 297 }
  const margin = Math.max(0, Number(settings.margin_mm) || 0)
  const usableWidth = Math.max(1, page.width - margin * 2)
  const usableHeight = Math.max(1, page.height - margin * 2)
  const cellWidth = usableWidth / layoutGrid.value.cols
  const cellHeight = usableHeight / layoutGrid.value.rows
  const filenameReserve = settings.show_filename ? 6 : 0
  return {
    cellWidth,
    imageCellHeight: Math.max(1, cellHeight - filenameReserve),
  }
})

async function selectFolder() {
  const selected = await open({ directory: true, multiple: true })
  if (selected) {
    folders.value = Array.isArray(selected) ? selected : [selected]
    folder.value = folders.value[0] || ''
    analysis.value = null
    generatedResult.value = null
  }
}

async function analyze() {
  if (!folders.value.length) return
  analyzing.value = true
  const result = await tauriCallSafe('analyze_image_paddler_folder', { folder: folder.value, folders: folders.value })
  if (result.ok) {
    analysis.value = result.data
    imagePage.value = 1
    await preloadVisibleImages()
  } else {
    ElMessage.error(result.error || '图片分析失败')
  }
  analyzing.value = false
}

async function run() {
  if (!folders.value.length) return
  generating.value = true
  const result = await tauriCallSafe('run_image_paddler', {
    args: {
      folder: folder.value,
      folders: folders.value,
      ...settings,
      orientation: resolvedOrientation.value,
    },
  })
  if (result.ok) {
    generatedResult.value = result.data
    ElMessage.success(`已生成 ${result.data.images} 张图片，${result.data.pages} 页`)
  } else {
    ElMessage.error(result.error || '生成失败')
  }
  generating.value = false
}

async function openGeneratedOutput() {
  if (!generatedResult.value?.output_path) return
  const result = await tauriCallSafe('open_path', { path: generatedResult.value.output_path })
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开生成文件')
  }
}

function parseLayout(layout, customRows, customCols) {
  if (layout === 'custom') {
    return {
      rows: clampNumber(customRows, 1, 8, 2),
      cols: clampNumber(customCols, 1, 8, 2),
    }
  }
  if (String(layout).includes('x')) {
    const [rows, cols] = String(layout).split('x').map(Number)
    return {
      rows: clampNumber(rows, 1, 8, 2),
      cols: clampNumber(cols, 1, 8, 2),
    }
  }
  const count = clampNumber(Number(layout), 1, 64, 4)
  if (count === 1) return { rows: 1, cols: 1 }
  if (count === 2) return { rows: 2, cols: 1 }
  if (count === 3) return { rows: 1, cols: 3 }
  if (count === 4) return { rows: 2, cols: 2 }
  const cols = Math.ceil(Math.sqrt(count))
  return { rows: Math.ceil(count / cols), cols }
}

function clampNumber(value, min, max, fallback) {
  const number = Number(value)
  if (!Number.isFinite(number)) return fallback
  return Math.min(max, Math.max(min, Math.round(number)))
}

function imageSrc(path) {
  return previewSources[path] || ''
}

function fileName(path) {
  let name = String(path || '').split(/[\\/]/).pop() || path
  if (settings.filename_without_ext) {
    name = name.replace(/\.[^.]+$/, '')
  }
  if (settings.filename_remove_text) {
    name = name.split(settings.filename_remove_text).join('')
  }
  return name
}

function previewImageStyle(img) {
  const metrics = layoutMetrics.value
  const nativeWidth = img.width * 25.4 / settings.dpi
  const nativeHeight = img.height * 25.4 / settings.dpi
  const fitScale = Math.min(metrics.cellWidth / nativeWidth, metrics.imageCellHeight / nativeHeight)
  const scale = settings.scale_mode === 'original' ? Math.min(fitScale, 1) : fitScale
  const drawWidth = nativeWidth * scale
  const drawHeight = nativeHeight * scale
  return {
    width: `${Math.min(100, drawWidth / metrics.cellWidth * 100)}%`,
    height: `${Math.min(100, drawHeight / metrics.imageCellHeight * 100)}%`,
    maxWidth: '100%',
    maxHeight: settings.show_filename ? 'calc(100% - 20px)' : '100%',
    objectFit: 'contain',
  }
}

async function preloadImages(paths) {
  await Promise.all(paths.map(async (path) => {
    if (previewSources[path]) return
    const result = await tauriCallSafe('read_image_data_url', { path })
    if (result.ok) {
      previewSources[path] = result.data
    }
  }))
}

function reorderImages(images, grid, mode) {
  const perPage = grid.rows * grid.cols
  const result = []
  for (let start = 0; start < images.length; start += perPage) {
    const chunk = images.slice(start, start + perPage)
    for (const idx of cellOrder(grid, mode)) {
      if (idx < chunk.length) result.push(chunk[idx])
    }
  }
  return result
}

function cellOrder(grid, mode) {
  const order = []
  if (mode === 'n') {
    for (let col = 0; col < grid.cols; col += 1) {
      for (let row = 0; row < grid.rows; row += 1) order.push(row * grid.cols + col)
    }
  } else if (mode === 'reverse_n') {
    for (let col = grid.cols - 1; col >= 0; col -= 1) {
      for (let row = 0; row < grid.rows; row += 1) order.push(row * grid.cols + col)
    }
  } else {
    for (let row = 0; row < grid.rows; row += 1) {
      for (let col = 0; col < grid.cols; col += 1) order.push(row * grid.cols + col)
    }
  }
  return order
}

function applyRecommendedSettings() {
  const recommended = analysis.value?.recommended
  if (!recommended) return
  settings.orientation = recommended.orientation || 'auto'
  settings.layout = recommended.layout || '2x1'
  settings.scale_mode = recommended.scale_mode || 'fit'
  settings.margin_mm = Number(recommended.margin_mm || 12)
  settings.show_filename = recommended.show_filename !== false
  ElMessage.success('已应用推荐参数')
}

function adjustPageZoom(delta) {
  pageZoom.value = clampNumber(pageZoom.value + delta, 50, 180, 100)
}

function adjustImageZoom(delta) {
  imageZoom.value = clampNumber(imageZoom.value + delta, 60, 180, 100)
}

async function preloadVisibleImages() {
  const paths = [...previewImages.value, ...pagedImages.value].map(img => img.path)
  await preloadImages([...new Set(paths)])
}

watch([pagedImages, previewImages], () => {
  preloadVisibleImages()
})

watch([imagePageSize, orderedImages], () => {
  if (imagePage.value > imagePageCount.value) imagePage.value = imagePageCount.value
  if (imagePage.value < 1) imagePage.value = 1
})

onMounted(async () => {
  unlistenDragDrop = await getCurrentWebview().onDragDropEvent(async (event) => {
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
      return
    }
    if (event.payload.type === 'drop') {
      const paths = event.payload.paths || []
      if (paths.length) {
        folders.value = paths
        folder.value = paths[0]
        analysis.value = null
        generatedResult.value = null
      }
    }
  })
})

onBeforeUnmount(() => {
  if (unlistenDragDrop) unlistenDragDrop()
})

function orientationLabel(value) {
  if (value === 'landscape') return '横向'
  if (value === 'portrait') return '竖向'
  return '自动'
}

function layoutLabel(value) {
  const grid = parseLayout(value, settings.custom_rows, settings.custom_cols)
  if (value === '1') return '1 张'
  if (value === '1x2') return '2 张（左右）'
  if (value === '2x1') return '2 张（上下）'
  return `${grid.rows} 行 × ${grid.cols} 列`
}

function scaleModeLabel(value) {
  if (value === 'original') return '不缩放'
  return '适应页面'
}
</script>

<style scoped>
.image-paddler-view {
  height: 100%;
}

.paddler-layout {
  display: flex;
  gap: 20px;
  height: 100%;
}

.settings-panel {
  width: 280px;
  flex-shrink: 0;
}

.settings-panel h3 {
  margin: 0 0 4px;
  color: #303133;
}

.hint {
  color: #909399;
  font-size: 12px;
  margin: 0 0 16px;
}

.folder-path {
  font-size: 12px;
  color: #909399;
  word-break: break-all;
  display: block;
  margin-top: 4px;
}

.inline-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.inline-controls :deep(.el-input-number) {
  width: 82px;
}

.result-panel {
  flex: 1;
  overflow-y: auto;
}

.analysis-summary {
  margin-bottom: 16px;
}

.recommendation-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 10px;
  padding: 10px 12px;
  border: 1px solid #d9ecff;
  background: #f4f9ff;
  border-radius: 4px;
  color: #606266;
  font-size: 12px;
}

.preview-section {
  margin-bottom: 18px;
}

.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-size: 12px;
  color: #909399;
}

.section-head h4 {
  margin: 0;
  font-size: 13px;
  color: #303133;
}

.preview-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.zoom-slider {
  width: 130px;
}

.zoom-value {
  width: 42px;
  text-align: right;
  color: #606266;
}

.page-size-select {
  width: 82px;
}

.page-preview-shell {
  display: block;
  padding: 12px;
  background: #f5f7fa;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  overflow: auto;
}

.page-preview {
  background: #fff;
  border: 1px solid #dcdfe6;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.10);
  margin: 0 auto;
}

.preview-grid {
  width: 100%;
  height: 100%;
  display: grid;
  gap: 8px;
}

.preview-cell {
  min-width: 0;
  min-height: 0;
  border: 1px dashed #dcdfe6;
  background: #fafafa;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  overflow: hidden;
}

.preview-cell-bordered {
  border: 2px solid #303133;
}

.preview-cell-white-border {
  border-color: #fff;
  box-shadow: inset 0 0 0 1px #dcdfe6;
}

.preview-cell img {
  display: block;
  object-fit: contain;
}

.preview-cell-no-name img,
.preview-cell-no-name img {
  max-height: 100%;
}

.preview-name {
  width: 100%;
  min-height: 20px;
  line-height: 20px;
  padding: 0 4px;
  color: #606266;
  font-size: 11px;
  text-align: center;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.generated-result {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  padding: 10px 12px;
  margin-bottom: 12px;
  border: 1px solid #dcdfe6;
  border-radius: 4px;
  background: #f5f7fa;
  font-size: 13px;
}

.output-path {
  margin-top: 4px;
  color: #606266;
  word-break: break-all;
  font-size: 12px;
}

.groups-section {
  margin-bottom: 16px;
}

.groups-section h4 {
  margin: 0 0 8px;
  font-size: 13px;
}

.group-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 13px;
}

.image-list h4 {
  margin: 0 0 8px;
  font-size: 13px;
}

.image-grid-scroll {
  max-height: 46vh;
  overflow: auto;
  padding: 10px;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  background: #fafafa;
}

.image-grid {
  display: grid;
  gap: 8px;
  align-items: start;
}

.image-thumb {
  text-align: center;
  padding: 8px;
  background: #f5f7fa;
  border-radius: 4px;
}

.thumb-image {
  margin: 0 auto 4px;
  border-radius: 4px;
  display: block;
  object-fit: cover;
  background: #e4e7ed;
}

.thumb-name {
  display: block;
  font-size: 11px;
  color: #606266;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.thumb-size {
  font-size: 10px;
  color: #c0c4cc;
}

.more-images {
  display: flex;
  align-items: center;
  justify-content: center;
  color: #909399;
  font-size: 13px;
}

.pager-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 10px;
  color: #606266;
  font-size: 12px;
}

.unit-label {
  margin-left: 8px;
  color: #909399;
  font-size: 12px;
}
</style>
