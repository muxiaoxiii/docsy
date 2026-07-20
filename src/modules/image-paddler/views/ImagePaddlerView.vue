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
            <div class="filename-panel">
              <div class="filename-panel-row">
                <el-switch v-model="settings.show_filename" active-text="显示" inactive-text="隐藏" />
                <el-switch
                  v-model="settings.filename_without_ext"
                  active-text="隐藏扩展名"
                  inactive-text="保留扩展名"
                />
              </div>
              <div class="filename-rules">
                <div
                  v-for="(rule, idx) in settings.filename_rules"
                  :key="rule.id"
                  class="filename-rule"
                  :class="{ 'filename-rule-keep': rule.kind === 'keep' }"
                >
                  <el-select v-model="rule.kind" size="small" class="rule-kind">
                    <el-option label="删除" value="remove" />
                    <el-option label="替换" value="replace" />
                    <el-option label="加前缀" value="prefix" />
                    <el-option label="加后缀" value="suffix" />
                    <el-option label="保留成分" value="keep" />
                  </el-select>
                  <template v-if="rule.kind === 'replace'">
                    <el-input v-model="rule.value" size="small" placeholder="原文字" />
                    <el-input v-model="rule.replacement" size="small" placeholder="替换为" />
                  </template>
                  <template v-else-if="rule.kind === 'keep'">
                    <el-checkbox v-model="rule.keep_time" size="small">时间</el-checkbox>
                    <el-checkbox v-model="rule.keep_number" size="small">编号</el-checkbox>
                    <el-checkbox v-model="rule.keep_text" size="small">文本</el-checkbox>
                    <el-input v-model="rule.replacement" size="small" placeholder="自定义名称" />
                    <el-select v-model="rule.separator" size="small" class="separator-select">
                      <el-option label="_" value="_" />
                      <el-option label="-" value="-" />
                      <el-option label="空格" value=" " />
                    </el-select>
                  </template>
                  <template v-else>
                    <el-input v-model="rule.value" size="small" :placeholder="rulePlaceholder(rule.kind)" />
                  </template>
                  <el-button size="small" text type="danger" @click="removeFilenameRule(idx)">-</el-button>
                </div>
              </div>
              <el-button size="small" plain @click="addFilenameRule">+ 添加规则</el-button>
            </div>
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
                <el-option label="深灰" value="dark_gray" />
                <el-option label="浅灰" value="light_gray" />
                <el-option label="红色" value="red" />
                <el-option label="黄色" value="yellow" />
                <el-option label="蓝色" value="blue" />
              </el-select>
            </div>
          </el-form-item>

          <el-form-item>
            <el-button type="success" @click="run" :loading="generating" :disabled="!analysis"> 生成文档 </el-button>
            <span v-if="analyzing" class="analyze-hint">正在分析...</span>
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
              <el-descriptions-item label="推荐方向">{{
                orientationLabel(analysis.recommended.orientation)
              }}</el-descriptions-item>
              <el-descriptions-item label="推荐布局">{{
                layoutLabel(analysis.recommended.layout)
              }}</el-descriptions-item>
              <el-descriptions-item label="推荐缩放">{{
                scaleModeLabel(analysis.recommended.scale_mode)
              }}</el-descriptions-item>
              <el-descriptions-item label="推荐边距">{{ analysis.recommended.margin_mm }} mm</el-descriptions-item>
              <el-descriptions-item label="当前方向">{{ resolvedOrientationLabel }}</el-descriptions-item>
              <el-descriptions-item label="当前布局"
                >{{ layoutGrid.rows }} 行 × {{ layoutGrid.cols }} 列</el-descriptions-item
              >
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
                    :style="previewCellStyle"
                  >
                    <template v-if="img">
                      <div class="preview-image-area" :style="previewImageAreaStyle">
                        <img :src="imageSrc(img.path)" :alt="fileName(img.path)" :style="previewImageStyle(img)" />
                      </div>
                      <div v-if="settings.show_filename" class="preview-name" :style="previewNameStyle">
                        <span v-for="(line, lineIdx) in fileNameLines(img.path)" :key="`${lineIdx}-${line}`">{{
                          line
                        }}</span>
                      </div>
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
            </div>
            <ImagePreviewGrid
              :items="orderedImages"
              :name-resolver="imageItemName"
              :meta-resolver="imageItemMeta"
              empty-description="暂无图片"
            />
          </div>
        </template>
        <el-empty v-else :description="analyzing ? '正在分析图片...' : '选择文件夹后自动分析'" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, reactive, watch, onMounted, onBeforeUnmount } from 'vue'
import { openPath, tauriCallSafe } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ElMessage } from 'element-plus'
import ImagePreviewGrid from '../../../shared/components/ImagePreviewGrid.vue'
import { fileName as baseFileName } from '../../../core/filePath.js'

const folder = ref('')
const folders = ref([])
const analyzing = ref(false)
const generating = ref(false)
const analysis = ref(null)
const generatedResult = ref(null)
const previewSources = reactive({})
const pageZoom = ref(100)
let unlistenDragDrop = null
let analyzeTimer = null
const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'webp', 'bmp', 'tif', 'tiff'])
const KNOWN_NON_IMAGE_EXTENSIONS = new Set([
  'pdf',
  'doc',
  'docx',
  'docm',
  'xls',
  'xlsx',
  'ppt',
  'pptx',
  'txt',
  'csv',
  'mp4',
  'mov',
  'avi',
  'mkv',
  'zip',
  'rar',
  '7z',
])

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
  filename_rules: [createFilenameRule('remove')],
  order_mode: 'z',
  border_enabled: false,
  border_color: 'black',
})

const layoutGrid = computed(() => parseLayout(settings.layout, settings.custom_rows, settings.custom_cols))
const resolvedOrientation = computed(() => {
  if (settings.orientation !== 'auto') return settings.orientation
  return analysis.value?.recommended?.orientation || 'portrait'
})
const resolvedOrientationLabel = computed(() => (resolvedOrientation.value === 'landscape' ? '横向' : '竖向'))
const orderedImages = computed(() => reorderImages(analysis.value?.images || [], layoutGrid.value, settings.order_mode))
const previewImages = computed(() => orderedImages.value.slice(0, layoutGrid.value.rows * layoutGrid.value.cols))
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
const previewCellStyle = computed(() => {
  if (!settings.border_enabled) return { borderColor: 'transparent' }
  return {
    borderColor: borderColorCss(settings.border_color),
  }
})
const layoutMetrics = computed(() => {
  const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
  const margin = Math.max(0, Number(settings.margin_mm) || 0)
  const usableWidth = Math.max(1, page.width - margin * 2)
  const docxTrailingGap = settings.output_format === 'docx' ? 2 : 0
  const usableHeight = Math.max(1, page.height - margin * 2 - docxTrailingGap)
  const cellWidth = usableWidth / layoutGrid.value.cols
  const cellHeight = usableHeight / layoutGrid.value.rows
  const filenameReserve = settings.show_filename ? 8.4 : 0
  return {
    cellWidth,
    cellHeight,
    filenameReserve,
    imageCellHeight: Math.max(1, cellHeight - filenameReserve),
  }
})
const previewImageAreaStyle = computed(() => {
  const metrics = layoutMetrics.value
  return {
    height: `${Math.min(100, (metrics.imageCellHeight / metrics.cellHeight) * 100)}%`,
  }
})
const previewNameStyle = computed(() => {
  const metrics = layoutMetrics.value
  return {
    height: `${Math.min(100, (metrics.filenameReserve / metrics.cellHeight) * 100)}%`,
  }
})

async function selectFolder() {
  const selected = await open({ directory: true, multiple: true })
  if (selected) {
    folders.value = Array.isArray(selected) ? selected : [selected]
    folder.value = folders.value[0] || ''
    scheduleAnalyze()
  }
}

async function analyze() {
  if (!folders.value.length) return
  analyzing.value = true
  const result = await tauriCallSafe('analyze_image_paddler_folder', { folder: folder.value, folders: folders.value })
  if (result.ok) {
    analysis.value = result.data
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
      filename_remove_text: '',
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
  const result = await openPath(generatedResult.value.output_path)
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
  let name = baseFileName(path)
  if (settings.filename_without_ext) {
    name = name.replace(/\.[^.]+$/, '')
  }
  return applyFilenameRules(name)
}

function imageItemName(img) {
  return fileName(img?.path || '')
}

function imageItemMeta(img) {
  return img?.width && img?.height ? `${img.width}×${img.height}` : ''
}

function applyFilenameRules(name) {
  let value = name
  for (const rule of settings.filename_rules) {
    if (rule.kind === 'remove' && rule.value) {
      value = value.split(rule.value).join('')
    } else if (rule.kind === 'replace' && rule.value) {
      value = value.split(rule.value).join(rule.replacement || '')
    } else if (rule.kind === 'prefix' && rule.value) {
      value = `${rule.value}${value}`
    } else if (rule.kind === 'suffix' && rule.value) {
      value = `${value}${rule.value}`
    } else if (rule.kind === 'keep') {
      value = keepFilenameParts(value, rule)
    }
  }
  return value.trim()
}

function keepFilenameParts(value, rule) {
  const parts = []
  if (rule.replacement?.trim()) parts.push(rule.replacement.trim())
  if (rule.keep_time) parts.push(...extractTimeParts(value))
  if (rule.keep_number) parts.push(...extractNumberPartsWithoutTimes(value))
  if (rule.keep_text) parts.push(...extractTextParts(value))
  const seen = []
  for (const part of parts) {
    if (part && !seen.includes(part)) seen.push(part)
  }
  return seen.join(rule.separator || '_')
}

function extractTimeParts(value) {
  return value.match(/\d{1,2}[:：_-]\d{2}(?:[:：_-]\d{2})?|\d+(?:\.\d+)?s|\d+m\d+s/gi) || []
}

function extractNumberParts(value) {
  return value.match(/\d+/g) || []
}

function extractNumberPartsWithoutTimes(value) {
  return extractNumberParts(value.replace(/\d{1,2}[:：_-]\d{2}(?:[:：_-]\d{2})?|\d+(?:\.\d+)?s|\d+m\d+s/gi, ' '))
}

function extractTextParts(value) {
  return value
    .split(/[-_\s]+/)
    .filter(Boolean)
    .filter((part) => !/^\d+$/.test(part))
}

function fileNameLines(path) {
  return wrapFilenameLines(fileName(path), layoutMetrics.value.cellWidth, 2)
}

function wrapFilenameLines(name, cellWidthMm, maxLines) {
  const maxUnits = Math.max(6, Math.floor((cellWidthMm * 72) / 25.4 / (8 * 0.56)))
  const lines = []
  let current = ''
  let units = 0
  for (const ch of name) {
    const u = ch.charCodeAt(0) < 128 ? 1 : 2
    if (units + u > maxUnits && current) {
      lines.push(current)
      current = ''
      units = 0
      if (lines.length >= maxLines) break
    }
    current += ch
    units += u
  }
  if (current && lines.length < maxLines) lines.push(current)
  const used = lines.join('').length
  if (used < name.length && lines.length) {
    let last = lines[lines.length - 1]
    while (nameUnits(last) + 1 > maxUnits && last.length) last = last.slice(0, -1)
    lines[lines.length - 1] = `${last}…`
  }
  return lines.length ? lines : ['']
}

function nameUnits(value) {
  return [...value].reduce((sum, ch) => sum + (ch.charCodeAt(0) < 128 ? 1 : 2), 0)
}

function previewImageStyle(img) {
  const metrics = layoutMetrics.value
  const nativeWidth = (img.width * 25.4) / settings.dpi
  const nativeHeight = (img.height * 25.4) / settings.dpi
  const fitScale = Math.min(metrics.cellWidth / nativeWidth, metrics.imageCellHeight / nativeHeight)
  const scale = settings.scale_mode === 'original' ? Math.min(fitScale, 1) : fitScale
  const drawWidth = nativeWidth * scale
  const drawHeight = nativeHeight * scale
  return {
    width: `${Math.min(100, (drawWidth / metrics.cellWidth) * 100)}%`,
    height: `${Math.min(100, (drawHeight / metrics.imageCellHeight) * 100)}%`,
    maxWidth: '100%',
    maxHeight: '100%',
    objectFit: 'contain',
  }
}

function borderColorCss(color) {
  return (
    {
      white: '#ffffff',
      dark_gray: '#4b5563',
      light_gray: '#d1d5db',
      red: '#dc2626',
      yellow: '#d97706',
      blue: '#2563eb',
      black: '#000000',
    }[color] || '#000000'
  )
}

function createFilenameRule(kind = 'remove') {
  return {
    id: `${Date.now()}_${Math.random().toString(16).slice(2)}`,
    kind,
    value: '',
    replacement: '',
    keep_number: true,
    keep_time: true,
    keep_text: false,
    separator: '_',
  }
}

function addFilenameRule() {
  settings.filename_rules.push(createFilenameRule('remove'))
}

function removeFilenameRule(index) {
  settings.filename_rules.splice(index, 1)
}

function rulePlaceholder(kind) {
  if (kind === 'prefix') return '前缀文字'
  if (kind === 'suffix') return '后缀文字'
  return '要删除的文字'
}

function scheduleAnalyze() {
  analysis.value = null
  generatedResult.value = null
  if (analyzeTimer) clearTimeout(analyzeTimer)
  analyzeTimer = setTimeout(() => {
    analyze()
  }, 80)
}

async function preloadImages(paths) {
  await Promise.all(
    paths.map(async (path) => {
      if (previewSources[path]) return
      const result = await tauriCallSafe('read_image_data_url', { path })
      if (result.ok) {
        previewSources[path] = result.data
      }
    }),
  )
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

async function preloadVisibleImages() {
  const paths = previewImages.value.map((img) => img.path)
  await preloadImages([...new Set(paths)])
}

watch(previewImages, () => {
  preloadVisibleImages()
})

onMounted(async () => {
  unlistenDragDrop = await getCurrentWebview().onDragDropEvent(async (event) => {
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
      return
    }
    if (event.payload.type === 'drop') {
      const paths = event.payload.paths || []
      if (paths.length) {
        const accepted = paths.filter(isImageOrFolderCandidate)
        if (!accepted.length) {
          ElMessage.warning('请拖入图片文件或文件夹')
          return
        }
        if (accepted.length < paths.length) {
          ElMessage.warning('已忽略不支持的文件类型')
        }
        folders.value = accepted
        folder.value = accepted[0]
        scheduleAnalyze()
      }
    }
  })
})

onBeforeUnmount(() => {
  if (unlistenDragDrop) unlistenDragDrop()
  if (analyzeTimer) clearTimeout(analyzeTimer)
})

function orientationLabel(value) {
  if (value === 'landscape') return '横向'
  if (value === 'portrait') return '竖向'
  return '自动'
}

function isImageOrFolderCandidate(path) {
  const name = baseFileName(path)
  const dot = name.lastIndexOf('.')
  if (dot <= 0) return true
  const ext = name.slice(dot + 1).toLowerCase()
  if (IMAGE_EXTENSIONS.has(ext)) return true
  return !KNOWN_NON_IMAGE_EXTENSIONS.has(ext)
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

.filename-panel {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.filename-panel-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.filename-rules {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.filename-rule {
  display: grid;
  grid-template-columns: 78px minmax(0, 1fr) auto;
  gap: 6px;
  align-items: center;
}

.filename-rule-keep {
  grid-template-columns: 78px auto auto auto minmax(0, 1fr) 64px auto;
}

.rule-kind {
  width: 78px;
}

.separator-select {
  width: 64px;
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
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
  margin: 0 auto;
}

.preview-grid {
  width: 100%;
  height: 100%;
  display: grid;
  gap: 0;
}

.preview-cell {
  min-width: 0;
  min-height: 0;
  border: 1px solid transparent;
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

.preview-image-area {
  width: 100%;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.preview-cell img {
  display: block;
  object-fit: contain;
}

.preview-cell-no-name img {
  max-height: 100%;
}

.preview-name {
  width: 100%;
  line-height: 14px;
  padding: 2px 4px;
  color: #606266;
  font-size: 11px;
  text-align: center;
  overflow: hidden;
  overflow-wrap: anywhere;
}

.preview-name span {
  display: block;
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

.unit-label {
  margin-left: 8px;
  color: #909399;
  font-size: 12px;
}

.analyze-hint {
  margin-left: 8px;
  color: #909399;
  font-size: 12px;
}
</style>
