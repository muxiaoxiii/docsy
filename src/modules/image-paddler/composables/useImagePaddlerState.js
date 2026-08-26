import { computed, ref, reactive, watch, onActivated, onBeforeUnmount, onMounted } from 'vue'
import { openPath, tauriCallQuiet, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { moveItem } from '../../../shared/components/reorderableItems.js'
import { fileName as baseFileName, parentDir } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'

const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'webp', 'bmp', 'tif', 'tiff'])
const FILENAME_MAX_LINES = 3
const DOCX_FILENAME_SAFETY_MM = 2
const PDF_FILENAME_SAFETY_MM = 0.6
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

export function useImagePaddlerState(options = {}) {
  const folder = ref('')
  const folders = ref([])
  const analyzing = ref(false)
  const generating = ref(false)
  const analysis = ref(null)
  const generatedResult = ref(null)
  const previewSources = reactive({})
  const pageZoom = ref(100)
  const explicitPaths = ref([])
  const inputContext = ref({ sourceKind: 'folder', sourceLabel: '', sourceStem: '' })
  const sourceDecisions = ref({})
  const exclusionByPath = ref({})
  const preferenceRevision = ref(0)
  let analyzeTimer = null
  let analysisRequestId = 0
  let preferencesReady = false

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
    filename_font_family: 'sans',
    filename_font_size_pt: 8,
    filename_without_ext: false,
    filename_remove_text: '',
    filename_rules: [],
    use_source_exclusions: true,
    order_mode: 'z',
    border_enabled: false,
    border_color: 'black',
  })
  const preference = useWorkspacePreferences('image-paddler.workspace', {
    settings,
    pageZoom,
    exclusionByPath,
    preferenceRevision,
  })

  const layoutGrid = computed(() => parseLayout(settings.layout, settings.custom_rows, settings.custom_cols))
  const isFrameSequence = computed(() => inputContext.value.sourceKind === 'video-frames')
  const resolvedOrientation = computed(() => {
    if (settings.orientation !== 'auto') return settings.orientation
    return analysis.value?.recommended?.orientation || 'portrait'
  })
  const resolvedOrientationLabel = computed(() => (resolvedOrientation.value === 'landscape' ? '横向' : '竖向'))
  const orderedImages = computed(() =>
    reorderImages(analysis.value?.images || [], layoutGrid.value, settings.order_mode),
  )
  const includedImages = computed(() => orderedImages.value.filter((image) => !isImageExcluded(image)))
  const excludedCount = computed(() => orderedImages.value.length - includedImages.value.length)
  const previewImages = computed(() => includedImages.value.slice(0, layoutGrid.value.rows * layoutGrid.value.cols))
  const previewSlots = computed(() => [...previewImages.value])
  const previewLayoutGrid = computed(() => compactGridForCount(layoutGrid.value, previewImages.value.length))
  const generatedOutputPaths = computed(() => {
    const paths = generatedResult.value?.output_paths || []
    return paths.length ? paths : generatedResult.value?.output_path ? [generatedResult.value.output_path] : []
  })
  const previewPageStyle = computed(() => ({
    aspectRatio: resolvedOrientation.value === 'landscape' ? '297 / 210' : '210 / 297',
    padding: `${(Math.max(0, settings.margin_mm) / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}%`,
    width: `${pageZoom.value}%`,
    minWidth: '220px',
    boxSizing: 'border-box',
  }))
  const previewGridStyle = computed(() => ({
    gridTemplateColumns: `repeat(${previewLayoutGrid.value.cols}, minmax(0, 1fr))`,
    gridTemplateRows: `repeat(${previewLayoutGrid.value.rows}, minmax(0, 1fr))`,
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
    const cellWidth = usableWidth / previewLayoutGrid.value.cols
    const cellHeight = usableHeight / previewLayoutGrid.value.rows
    const filenameFontSizePt = clampNumber(settings.filename_font_size_pt, 6, 24, 8)
    const filenameLineHeightMm = ((filenameFontSizePt * 25.4) / 72) * 1.32 + 0.45
    const filenameMaxLines = settings.show_filename
      ? Math.max(
          1,
          ...previewImages.value.map((image) =>
            requiredFilenameLines(fileName(image.path), cellWidth, filenameFontSizePt, FILENAME_MAX_LINES),
          ),
        )
      : 0
    const filenameSafetyMm = settings.output_format === 'docx' ? DOCX_FILENAME_SAFETY_MM : PDF_FILENAME_SAFETY_MM
    const filenameReserve = settings.show_filename ? filenameLineHeightMm * filenameMaxLines + filenameSafetyMm : 0
    return {
      cellWidth,
      cellHeight,
      filenameReserve,
      filenameFontSizePt,
      filenameLineHeightMm,
      filenameMaxLines,
      imageCellHeight: Math.max(1, cellHeight - filenameReserve),
    }
  })
  const previewImageAreaStyle = computed(() => {
    const metrics = layoutMetrics.value
    const height = `${Math.min(100, (metrics.imageCellHeight / metrics.cellHeight) * 100)}%`
    return {
      height,
      flexBasis: height,
    }
  })
  const previewNameStyle = computed(() => {
    const metrics = layoutMetrics.value
    const height = `${Math.min(100, (metrics.filenameReserve / metrics.cellHeight) * 100)}%`
    return {
      height,
      minHeight: height,
      flexBasis: height,
      fontSize: `${(metrics.filenameFontSizePt * 96) / 72}px`,
      fontFamily: filenameFontFamilyCss(settings.filename_font_family),
      lineHeight: `${metrics.filenameLineHeightMm}mm`,
    }
  })

  function filenameFontFamilyCss(value) {
    return (
      {
        serif: '"Songti SC", "STSong", SimSun, serif',
        kaiti: '"Kaiti SC", "STKaiti", KaiTi, serif',
        fangsong: '"STFangsong", FangSong, serif',
        sans: '"PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif',
      }[value] || '"PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif'
    )
  }

  async function selectFolder() {
    const selected = await open({ directory: true, multiple: true })
    if (selected) {
      folders.value = Array.isArray(selected) ? selected : [selected]
      folder.value = folders.value[0] || ''
      explicitPaths.value = []
      inputContext.value = { sourceKind: 'folder', sourceLabel: '', sourceStem: '' }
      sourceDecisions.value = {}
      scheduleAnalyze()
    }
  }

  function loadImagePaths(paths, context = {}) {
    const normalized = [...new Set((paths || []).filter((path) => isExplicitImagePath(path)))]
    if (!normalized.length) return false
    explicitPaths.value = normalized
    const sourceKind = context.sourceKind || (normalized.every(isVideoFramePath) ? 'video-frames' : 'images')
    inputContext.value = {
      sourceKind,
      sourceLabel: context.sourceLabel || '',
      sourceStem: context.sourceStem || '',
    }
    sourceDecisions.value = Object.fromEntries(
      (context.selection || [])
        .filter((item) => item?.path)
        .map((item) => [item.path, { decision: item.decision || 'review', reason: item.reason || '' }]),
    )
    folders.value = [...new Set(normalized.map((path) => parentDir(path)))]
    folder.value = folders.value[0] || parentDir(normalized[0])
    settings.order_mode = 'custom'
    if (inputContext.value.sourceKind === 'video-frames') settings.output_mode = 'merged'
    scheduleAnalyze()
    return true
  }

  async function analyze() {
    if (!folders.value.length) return
    const requestId = ++analysisRequestId
    analyzing.value = true
    const result = await tauriCallSafe('analyze_image_paddler_folder', {
      folder: folder.value,
      folders: folders.value,
      imagePaths: explicitPaths.value.length ? explicitPaths.value : undefined,
    })
    if (requestId !== analysisRequestId) return
    if (result.ok) {
      analysis.value = result.data
      if (isFrameSequence.value) applyRecommendedSettings(false)
      await preloadVisibleImages()
    } else {
      ElMessage.error(userFacingError(result.error, '图片文件夹分析失败，请确认文件夹路径正确'))
    }
    analyzing.value = false
  }

  async function run() {
    if (!folders.value.length) return
    if (!includedImages.value.length) {
      ElMessage.warning('当前没有参与排版的图片，请先恢复至少一张图片')
      return
    }
    generating.value = true
    const result = await tauriCallSafe('run_image_paddler', {
      args: {
        folder: folder.value,
        folders: folders.value,
        image_paths: includedImages.value.map((image) => image.path),
        output_stem: inputContext.value.sourceStem || undefined,
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

  function compactGridForCount(baseGrid, count) {
    const capacity = baseGrid.rows * baseGrid.cols
    if (!count || count >= capacity) return baseGrid
    const targetRatio = baseGrid.cols / baseGrid.rows
    let best = { rows: 1, cols: count }
    let bestScore = Math.abs(Math.log(best.cols / best.rows) - Math.log(targetRatio))
    for (let rows = 1; rows <= count; rows += 1) {
      if (count % rows !== 0) continue
      const cols = count / rows
      const score = Math.abs(Math.log(cols / rows) - Math.log(targetRatio))
      if (score < bestScore) {
        best = { rows, cols }
        bestScore = score
      }
    }
    return best
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
    const dimensions = img?.width && img?.height ? `${img.width}×${img.height}` : ''
    const sourceReason = sourceDecisions.value[img?.path]?.reason
    if (isImageExcluded(img)) return sourceReason ? `来源判断：${sourceReason}` : dimensions
    return dimensions
  }

  function isImageExcluded(img) {
    const path = img?.path || ''
    if (!path) return false
    if (Object.prototype.hasOwnProperty.call(exclusionByPath.value, path)) {
      return Boolean(exclusionByPath.value[path])
    }
    return settings.use_source_exclusions && sourceDecisions.value[path]?.decision === 'exclude'
  }

  function toggleImageExclusion({ item, excluded }) {
    if (!item?.path) return
    exclusionByPath.value = { ...exclusionByPath.value, [item.path]: Boolean(excluded) }
    generatedResult.value = null
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
    return wrapFilenameLines(fileName(path), layoutMetrics.value.cellWidth, layoutMetrics.value.filenameMaxLines)
  }

  function requiredFilenameLines(name, cellWidthMm, fontSizePt, maxLines) {
    const maxUnits = Math.max(6, Math.floor((cellWidthMm * 72) / 25.4 / (fontSizePt * 0.56)))
    return Math.min(maxLines, Math.max(1, Math.ceil(nameUnits(name) / maxUnits)))
  }

  function wrapFilenameLines(name, cellWidthMm, maxLines) {
    const fontSize = clampNumber(settings.filename_font_size_pt, 6, 24, 8)
    const maxUnits = Math.max(6, Math.floor((cellWidthMm * 72) / 25.4 / (fontSize * 0.56)))
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
        const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 900 })
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

  function applyRecommendedSettings(showMessage = true) {
    const recommended = analysis.value?.recommended
    if (!recommended) return
    settings.orientation = recommended.orientation || 'auto'
    settings.layout = recommended.layout || '2x1'
    settings.scale_mode = recommended.scale_mode || 'fit'
    settings.margin_mm = Number(recommended.margin_mm || 12)
    settings.show_filename = recommended.show_filename !== false
    if (showMessage) ElMessage.success('已应用推荐参数')
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
      explicitPaths.value = accepted.every(isExplicitImagePath) ? accepted : []
      inputContext.value = { sourceKind: 'drop', sourceLabel: '', sourceStem: '' }
      sourceDecisions.value = {}
      scheduleAnalyze()
    },
  })

  onBeforeUnmount(() => {
    if (analyzeTimer) clearTimeout(analyzeTimer)
    void preference.stop()
  })

  function consumeIncomingTransfer() {
    const transfer = typeof options.initialTransfer === 'function' ? options.initialTransfer() : null
    const initialPaths =
      transfer?.paths || (typeof options.initialImagePaths === 'function' ? options.initialImagePaths() : [])
    if (loadImagePaths(initialPaths, transfer || {}) && typeof options.onInitialPathsLoaded === 'function') {
      options.onInitialPathsLoaded()
    }
  }

  onMounted(async () => {
    await preference.start()
    if (preferenceRevision.value < 2) {
      const legacyBlankRules =
        Array.isArray(settings.filename_rules) &&
        settings.filename_rules.every((rule) => !rule?.value && !rule?.replacement)
      if (legacyBlankRules && !settings.filename_remove_text) {
        settings.filename_without_ext = false
        settings.filename_rules = []
      }
      settings.filename_font_size_pt = clampNumber(settings.filename_font_size_pt, 6, 24, 8)
      if (!['sans', 'serif', 'kaiti', 'fangsong'].includes(settings.filename_font_family)) {
        settings.filename_font_family = 'sans'
      }
      preferenceRevision.value = 2
    }
    preferencesReady = true
    consumeIncomingTransfer()
  })

  onActivated(() => {
    if (preferencesReady) consumeIncomingTransfer()
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

  function isExplicitImagePath(path) {
    const name = baseFileName(path)
    const dot = name.lastIndexOf('.')
    return dot > 0 && IMAGE_EXTENSIONS.has(name.slice(dot + 1).toLowerCase())
  }

  function isVideoFramePath(path) {
    return /(?:^|[/\\])_docsy_video_frames(?:[/\\])|(?:^|[_-])frame[_-]?\d+/i.test(path)
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
    inputContext,
    isFrameSequence,
    pageZoom,
    settings,
    layoutGrid,
    resolvedOrientation,
    resolvedOrientationLabel,
    orderedImages,
    includedImages,
    excludedCount,
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
    loadImagePaths,
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
    isImageExcluded,
    toggleImageExclusion,
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
