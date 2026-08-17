import { computed, ref, reactive, watch, onBeforeUnmount } from 'vue'
import { openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { moveItem } from '../../../shared/components/reorderableItems.js'
import { fileName as baseFileName } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'

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

export function useImagePaddlerState() {
  const folder = ref('')
  const folders = ref([])
  const analyzing = ref(false)
  const generating = ref(false)
  const analysis = ref(null)
  const generatedResult = ref(null)
  const previewSources = reactive({})
  const pageZoom = ref(100)
  let analyzeTimer = null
  let analysisRequestId = 0

  const settings = reactive({
    output_format: 'pdf',
    output_mode: 'merged',
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
  const orderedImages = computed(() =>
    reorderImages(analysis.value?.images || [], layoutGrid.value, settings.order_mode),
  )
  const previewImages = computed(() => orderedImages.value.slice(0, layoutGrid.value.rows * layoutGrid.value.cols))
  const previewSlots = computed(() => {
    const slots = [...previewImages.value]
    while (slots.length < layoutGrid.value.rows * layoutGrid.value.cols) slots.push(null)
    return slots
  })
  const generatedOutputPaths = computed(() => {
    const paths = generatedResult.value?.output_paths || []
    return paths.length ? paths : generatedResult.value?.output_path ? [generatedResult.value.output_path] : []
  })
  const previewPageStyle = computed(() => ({
    aspectRatio: resolvedOrientation.value === 'landscape' ? '297 / 210' : '210 / 297',
    padding: `${(Math.max(0, settings.margin_mm) / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}%`,
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
    const requestId = ++analysisRequestId
    analyzing.value = true
    const result = await tauriCallSafe('analyze_image_paddler_folder', { folder: folder.value, folders: folders.value })
    if (requestId !== analysisRequestId) return
    if (result.ok) {
      analysis.value = result.data
      await preloadVisibleImages()
    } else {
      ElMessage.error(userFacingError(result.error, '图片文件夹分析失败，请确认文件夹路径正确'))
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
        image_paths: settings.order_mode === 'custom' ? orderedImages.value.map((image) => image.path) : undefined,
        ...settings,
        orientation: resolvedOrientation.value,
      },
    })
    if (result.ok) {
      generatedResult.value = result.data
      const count = result.data.output_paths?.length || 1
      ElMessage.success(
        count > 1
          ? `已生成 ${count} 个文档，共 ${result.data.images} 张图片、${result.data.pages} 页`
          : `已生成 ${result.data.images} 张图片，${result.data.pages} 页`,
      )
    } else {
      ElMessage.error(userFacingError(result.error, '图片排版文档生成失败，请确认图片文件未被占用'))
    }
    generating.value = false
  }

  function reorderLayoutImages({ from, to }) {
    if (!analysis.value) return
    analysis.value = {
      ...analysis.value,
      images: moveItem(orderedImages.value, from, to),
    }
    settings.order_mode = 'custom'
    generatedResult.value = null
  }

  async function openGeneratedOutput() {
    const path = generatedResult.value?.output_path
    if (!path) return
    const result = await openPath(path)
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '无法打开生成文件'))
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

  useWindowFileDrop({
    onDrop: async (paths) => {
      if (!paths.length) return
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
    },
  })

  onBeforeUnmount(() => {
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

  return {
    folder,
    folders,
    analyzing,
    generating,
    analysis,
    generatedResult,
    previewSources,
    pageZoom,
    settings,
    layoutGrid,
    resolvedOrientation,
    resolvedOrientationLabel,
    orderedImages,
    previewImages,
    previewSlots,
    generatedOutputPaths,
    previewPageStyle,
    previewGridStyle,
    previewCellStyle,
    layoutMetrics,
    previewImageAreaStyle,
    previewNameStyle,
    selectFolder,
    analyze,
    run,
    reorderLayoutImages,
    openGeneratedOutput,
    imageSrc,
    fileName,
    fileNameLines,
    previewImageStyle,
    imageItemName,
    imageItemMeta,
    addFilenameRule,
    removeFilenameRule,
    rulePlaceholder,
    applyRecommendedSettings,
    adjustPageZoom,
    orientationLabel,
    layoutLabel,
    scaleModeLabel,
  }
}
