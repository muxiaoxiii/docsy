import { computed, ref, reactive, watch, onActivated, onBeforeUnmount, onMounted } from 'vue'
import { openPath, tauriCallQuiet, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { moveItem } from '../../../shared/components/reorderableItems.js'
import { fileName as baseFileName, parentDir } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'
import {
  safeColumnWidth,
  effectivePageWidth,
  pairOffsets,
  pairAlignToXY,
  detectPageConflicts,
  computeOptimalPageScale,
  safeImageWidth,
  effectiveImageWidth,
  layoutOptionLabel,
} from './layoutPreview.js'

const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'webp', 'bmp', 'tif', 'tiff'])
const FILENAME_MAX_LINES = 3
const NOTE_MAX_LINES = 3
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
  const previewPageFraction = ref(0.25)
  const explicitPaths = ref([])
  const inputContext = ref({ sourceKind: 'folder', sourceLabel: '', sourceStem: '' })
  const sourceDecisions = ref({})
  const exclusionByPath = ref({})
  const preferenceRevision = ref(0)
  const currentPageIndex = ref(0)
  const pageScales = ref({})
  const activePageScale = ref(100)
  const imageAnnotations = ref({})
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
    caption_position: 'below',
    pair_mode: 'cell-center',
    print_safety_pad_mm: 6,
    doclet_layout_tips: true,
    filename_font_family: 'sans',
    filename_font_size_pt: 8,
    filename_color: 'dark_gray',
    filename_without_ext: false,
    filename_remove_text: '',
    filename_rules: [],
    reserve_note_placeholder: false,
    note_placeholder_text: '[点击输入说明]',
    note_font_family: 'kaiti',
    note_font_size_pt: 8,
    note_color: 'gray',
    use_source_exclusions: true,
    order_mode: 'z',
    border_enabled: false,
    border_color: 'black',
    use_table: true,
    fixed_width_mm: 160,
  })
  const preference = useWorkspacePreferences('image-paddler.workspace', {
    settings,
    pageZoom,
    previewPageFraction,
    exclusionByPath,
    preferenceRevision,
    pageScales,
    imageAnnotations,
  })

  const isFlowLayout = computed(() => settings.output_format === 'docx' && !settings.use_table)
  const layoutGrid = computed(() => {
    const grid = parseLayout(settings.layout, settings.custom_rows, settings.custom_cols)
    return settings.output_format === 'docx' && !settings.use_table ? { rows: grid.rows * grid.cols, cols: 1 } : grid
  })
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
  const perPage = computed(() => Math.max(1, layoutGrid.value.rows * layoutGrid.value.cols))
  const isPairLayout = computed(() => perPage.value === 2)
  const totalPages = computed(() => Math.max(1, Math.ceil(includedImages.value.length / perPage.value)))

  watch([totalPages, includedImages], () => {
    if (currentPageIndex.value >= totalPages.value) {
      currentPageIndex.value = Math.max(0, totalPages.value - 1)
    }
  })

  const previewImages = computed(() => {
    const count = perPage.value
    const start = currentPageIndex.value * count
    return includedImages.value.slice(start, start + count)
  })
  const previewSlots = computed(() => [...previewImages.value])
  const previewLayoutGrid = computed(() => compactGridForCount(layoutGrid.value, previewImages.value.length))

  function nextPage() {
    if (currentPageIndex.value < totalPages.value - 1) {
      goToPage(currentPageIndex.value + 1)
    }
  }

  function prevPage() {
    if (currentPageIndex.value > 0) {
      goToPage(currentPageIndex.value - 1)
    }
  }

  function goToPage(index) {
    const target = Math.max(0, Math.min(totalPages.value - 1, Number(index) || 0))
    currentPageIndex.value = target
  }
  const generatedOutputPaths = computed(() => {
    const paths = generatedResult.value?.output_paths || []
    return paths.length ? paths : generatedResult.value?.output_path ? [generatedResult.value.output_path] : []
  })
  const previewPageStyle = computed(() => ({
    aspectRatio: resolvedOrientation.value === 'landscape' ? '297 / 210' : '210 / 297',
    width: `${pageZoom.value}%`,
    minWidth: '220px',
    boxSizing: 'border-box',
  }))
  const previewGridStyle = computed(() => ({
    left: `${(settings.margin_mm / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}%`,
    right: `${(settings.margin_mm / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}%`,
    top: `${(settings.margin_mm / (resolvedOrientation.value === 'landscape' ? 210 : 297)) * 100}%`,
    bottom: `${((Number(settings.margin_mm) + (settings.output_format === 'docx' ? 2 : 0)) / (resolvedOrientation.value === 'landscape' ? 210 : 297)) * 100}%`,
    gridTemplateColumns: `repeat(${previewLayoutGrid.value.cols}, minmax(0, 1fr))`,
    gridTemplateRows: `repeat(${previewLayoutGrid.value.rows}, minmax(0, 1fr))`,
  }))
  const previewCellStyle = computed(() => {
    if (!settings.border_enabled) {
      return {
        boxShadow: 'none',
      }
    }
    const color = borderColorCss(settings.border_color)
    if (settings.border_color === 'white') {
      return {
        boxShadow: 'inset 0 0 0 1.5px #ffffff, inset 0 0 0 2.5px rgba(0, 0, 0, 0.25)',
      }
    }
    return {
      boxShadow: `inset 0 0 0 1.5px ${color}`,
    }
  })
  function computeMetricsForGrid(pageGrid, pageImages = []) {
    const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
    const margin = Math.max(0, Number(settings.margin_mm) || 0)
    const usableWidth = Math.max(1, page.width - margin * 2)
    const docxTrailingGap = settings.output_format === 'docx' ? 2 : 0
    const usableHeight = Math.max(1, page.height - margin * 2 - docxTrailingGap)
    const cellWidth = usableWidth / Math.max(1, pageGrid.cols)
    const cellHeight = usableHeight / Math.max(1, pageGrid.rows)
    const filenameFontSizePt = clampNumber(settings.filename_font_size_pt, 6, 24, 8)
    const filenameLineHeightMm = ((filenameFontSizePt * 25.4) / 72) * 1.32 + 0.45
    const filenameMaxLines =
      settings.show_filename && pageImages.length
        ? Math.max(
            1,
            ...pageImages.map((image) =>
              requiredFilenameLines(imageTitle(image.path), cellWidth, filenameFontSizePt, FILENAME_MAX_LINES),
            ),
          )
        : settings.show_filename
          ? 1
          : 0

    const noteFontSizePt = clampNumber(settings.note_font_size_pt, 6, 24, 8)
    const noteLineHeightMm = ((noteFontSizePt * 25.4) / 72) * 1.32 + 0.45
    const hasAnyNote =
      settings.reserve_note_placeholder || pageImages.some((image) => Boolean(imageDescription(image.path)))
    const noteMaxLines =
      hasAnyNote && pageImages.length
        ? Math.max(
            1,
            ...pageImages.map((image) => {
              const desc =
                imageDescription(image.path) || (settings.reserve_note_placeholder ? settings.note_placeholder_text : '')
              return desc ? Math.min(NOTE_MAX_LINES, requiredFilenameLines(desc, cellWidth, noteFontSizePt, NOTE_MAX_LINES)) : 0
            }),
          )
        : hasAnyNote
          ? 1
          : 0

    const titleReserve = settings.show_filename ? filenameLineHeightMm * filenameMaxLines : 0
    const noteReserve = hasAnyNote ? noteLineHeightMm * noteMaxLines : 0
    const filenameSafetyMm = settings.output_format === 'docx' ? DOCX_FILENAME_SAFETY_MM : PDF_FILENAME_SAFETY_MM
    const filenameReserve = titleReserve > 0 || noteReserve > 0 ? titleReserve + noteReserve + filenameSafetyMm : 0

    return {
      cellWidth,
      cellHeight,
      filenameReserve,
      titleReserve,
      noteReserve,
      filenameFontSizePt,
      filenameLineHeightMm,
      filenameMaxLines,
      noteFontSizePt,
      noteLineHeightMm,
      noteMaxLines,
      imageCellHeight: Math.max(1, cellHeight - filenameReserve),
    }
  }

  const layoutMetrics = computed(() => computeMetricsForGrid(previewLayoutGrid.value, previewImages.value))
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
      fontSize: `${((metrics.filenameFontSizePt * 25.4) / 72 / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}cqw`,
      fontFamily: filenameFontFamilyCss(settings.filename_font_family),
      color: { black: '#000000', gray: '#6B7280', blue: '#2563EB' }[settings.filename_color] || '#4B5563',
      lineHeight: `${(metrics.filenameLineHeightMm / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}cqw`,
    }
  })
  const previewNoteStyle = computed(() => {
    const metrics = layoutMetrics.value
    return {
      fontSize: `${((metrics.noteFontSizePt * 25.4) / 72 / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}cqw`,
      fontFamily: filenameFontFamilyCss(settings.note_font_family),
      color:
        { black: '#000000', gray: '#6B7280', dark_gray: '#4B5563', blue: '#2563EB' }[settings.note_color] || '#6B7280',
      lineHeight: `${(metrics.noteLineHeightMm / (resolvedOrientation.value === 'landscape' ? 297 : 210)) * 100}cqw`,
      fontStyle: 'normal',
      opacity: 0.9,
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

  const safeColumnWidthValue = computed(() => {
    const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
    return safeColumnWidth(page.width, settings.margin_mm, layoutGrid.value.cols, settings.print_safety_pad_mm)
  })
  const maximumImageWidth = computed(() => safeColumnWidthValue.value)
  const currentRecommendedWidth = computed(() => Math.floor(safeColumnWidthValue.value * 10) / 10)
  const actualImageWidth = computed(() => Math.min(Math.max(0.1, Number(settings.fixed_width_mm) || 160), 500))
  const widthIsLimited = computed(() => Number(settings.fixed_width_mm) > safeColumnWidthValue.value)

  const currentPageScale = computed(() => (Number(activePageScale.value) || 100) / 100)

  watch(
    [currentPageIndex, pageScales],
    () => {
      const saved = pageScales.value[currentPageIndex.value]
      activePageScale.value = saved !== undefined ? Math.round(saved * 100) : 100
    },
    { flush: 'sync', immediate: true, deep: true },
  )

  const hasSavedScale = computed(() => pageScales.value[currentPageIndex.value] !== undefined)
  const isCurrentPageDirty = computed(() => {
    const saved = pageScales.value[currentPageIndex.value]
    const savedPercent = saved !== undefined ? Math.round(saved * 100) : 100
    return activePageScale.value !== savedPercent
  })

  function saveCurrentPageScale() {
    const newScales = { ...pageScales.value }
    if (activePageScale.value === 100) {
      delete newScales[currentPageIndex.value]
    } else {
      newScales[currentPageIndex.value] = activePageScale.value / 100
    }
    pageScales.value = newScales
    ElMessage.success(`第 ${currentPageIndex.value + 1} 页缩放已保存 (${activePageScale.value}%)`)
  }

  function cancelCurrentPageScale() {
    const saved = pageScales.value[currentPageIndex.value]
    activePageScale.value = saved !== undefined ? Math.round(saved * 100) : 100
  }

  function resetCurrentPageScale() {
    const newScales = { ...pageScales.value }
    delete newScales[currentPageIndex.value]
    pageScales.value = newScales
    activePageScale.value = 100
    ElMessage.success(`第 ${currentPageIndex.value + 1} 页已恢复默认比例`)
  }

  const hasAnySavedScales = computed(() => Object.keys(pageScales.value).length > 0)

  function autoFitCurrentPageScale() {
    const count = perPage.value
    const start = currentPageIndex.value * count
    const pageImgs = includedImages.value.slice(start, start + count)
    const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
    const pageGrid = compactGridForCount(layoutGrid.value, pageImgs.length)
    const metrics = computeMetricsForGrid(pageGrid, pageImgs)
    const pageImgsWithAnnotations = pageImgs.map((img) => ({
      ...img,
      title: imageTitle(img.path),
      description:
        imageDescription(img.path) || (settings.reserve_note_placeholder ? settings.note_placeholder_text : ''),
    }))

    const optimal = computeOptimalPageScale({
      images: pageImgsWithAnnotations,
      grid: pageGrid,
      cellWidth: metrics.cellWidth,
      imageCellHeight: metrics.imageCellHeight,
      fixedWidthMm: actualImageWidth.value,
      scaleMode: settings.scale_mode,
      dpi: settings.dpi,
      pageWidth: page.width,
      pageHeight: page.height,
      marginMm: Number(settings.margin_mm) || 0,
      showFilename: settings.show_filename,
      captionPosition: settings.caption_position,
      captionReserveMm: metrics.filenameReserve,
      fontSizePt: clampNumber(settings.filename_font_size_pt, 6, 24, 8),
      noteFontSizePt: clampNumber(settings.note_font_size_pt, 6, 24, 8),
      pairMode: settings.pair_mode,
    })
    if (optimal === null) {
      activePageScale.value = 50
      ElMessage.warning('本页图片比例过大，缩小至 50% 仍有局部溢出，建议切换横向或减少每页张数')
    } else {
      activePageScale.value = optimal
      ElMessage.success(`已自适应计算本页比例为 ${optimal}%，点击保存即可生效`)
    }
  }

  const scaleScope = ref('all')
  const globalScalePercent = ref(100)

  function setGlobalScale(val) {
    const percent = Math.round(Number(val) || 100)
    globalScalePercent.value = percent
    const targetScale = percent / 100
    const newScales = {}
    if (percent !== 100) {
      for (let i = 0; i < totalPages.value; i += 1) {
        newScales[i] = targetScale
      }
    }
    pageScales.value = newScales
    activePageScale.value = percent
  }

  function autoFitAllPagesScale() {
    const total = totalPages.value
    let minOptimal = 140
    let hasAnyValid = false
    for (let pageIdx = 0; pageIdx < total; pageIdx += 1) {
      const count = perPage.value
      const start = pageIdx * count
      const pageImgs = includedImages.value.slice(start, start + count)
      if (!pageImgs.length) continue
      const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
      const pageGrid = compactGridForCount(layoutGrid.value, pageImgs.length)
      const metrics = computeMetricsForGrid(pageGrid, pageImgs)
      const pageImgsWithAnnotations = pageImgs.map((img) => ({
        ...img,
        title: imageTitle(img.path),
        description:
          imageDescription(img.path) || (settings.reserve_note_placeholder ? settings.note_placeholder_text : ''),
      }))
      const optimal = computeOptimalPageScale({
        images: pageImgsWithAnnotations,
        grid: pageGrid,
        cellWidth: metrics.cellWidth,
        imageCellHeight: metrics.imageCellHeight,
        fixedWidthMm: actualImageWidth.value,
        scaleMode: settings.scale_mode,
        dpi: settings.dpi,
        pageWidth: page.width,
        pageHeight: page.height,
        marginMm: Number(settings.margin_mm) || 0,
        showFilename: settings.show_filename,
        captionPosition: settings.caption_position,
        captionReserveMm: metrics.filenameReserve,
        fontSizePt: clampNumber(settings.filename_font_size_pt, 6, 24, 8),
        noteFontSizePt: clampNumber(settings.note_font_size_pt, 6, 24, 8),
        pairMode: settings.pair_mode,
      })
      if (optimal !== null) {
        hasAnyValid = true
        if (optimal < minOptimal) {
          minOptimal = optimal
        }
      } else {
        minOptimal = Math.min(minOptimal, 50)
      }
    }
    const finalScale = hasAnyValid ? minOptimal : 50
    setGlobalScale(finalScale)
    ElMessage.success(`已自适应推算全部页面最佳比例为 ${finalScale}%，所有页面统一生效`)
  }

  function applyScaleToAllPages(scaleOverride) {
    const targetScale = scaleOverride !== undefined ? scaleOverride : activePageScale.value / 100
    const targetPercent = Math.round(targetScale * 100)
    const newScales = {}
    if (targetPercent !== 100) {
      for (let i = 0; i < totalPages.value; i += 1) {
        newScales[i] = targetScale
      }
    }
    pageScales.value = newScales
    globalScalePercent.value = targetPercent
    ElMessage.success(`已将当前比例 (${targetPercent}%) 应用到全部 ${totalPages.value} 页`)
  }

  function applyScaleToSubsequentPages(fromPageIndex, scaleOverride) {
    const startPage = fromPageIndex !== undefined ? fromPageIndex : currentPageIndex.value
    const targetScale = scaleOverride !== undefined ? scaleOverride : activePageScale.value / 100
    const targetPercent = Math.round(targetScale * 100)
    const newScales = { ...pageScales.value }
    for (let i = startPage; i < totalPages.value; i += 1) {
      if (targetPercent === 100) {
        delete newScales[i]
      } else {
        newScales[i] = targetScale
      }
    }
    pageScales.value = newScales
    ElMessage.success(`已将当前比例 (${targetPercent}%) 应用到第 ${startPage + 1} 页及后续所有页`)
  }

  function resetAllPageScales() {
    pageScales.value = {}
    globalScalePercent.value = 100
    activePageScale.value = 100
    ElMessage.success('已恢复全部页面为 100% 默认比例')
  }
  function optionLayoutLabel(value) {
    return layoutOptionLabel(value, isFlowLayout.value)
  }

  async function selectFolder() {
    await addFolders()
  }

  async function addFolders() {
    const selected = await open({ directory: true, multiple: true })
    if (selected) {
      const newFolders = Array.isArray(selected) ? selected : [selected]
      folders.value = [...new Set([...folders.value, ...newFolders])]
      explicitPaths.value = []
      if (!folder.value) folder.value = folders.value[0] || ''
      inputContext.value = { sourceKind: 'folder', sourceLabel: '', sourceStem: '' }
      scheduleAnalyze()
    }
  }

  async function addImages() {
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'webp', 'bmp', 'tif', 'tiff'] }],
    })
    if (selected) {
      const newPaths = Array.isArray(selected) ? selected : [selected]
      explicitPaths.value = []
      folders.value = [...new Set([...folders.value, ...newPaths])]
      if (!folder.value) folder.value = folders.value[0] || ''
      inputContext.value = { sourceKind: 'images', sourceLabel: '导入图片', sourceStem: '' }
      scheduleAnalyze()
    }
  }

  function removeFolder(index) {
    explicitPaths.value = []
    folders.value.splice(index, 1)
    if (!folders.value.length) {
      clearAllSources()
    } else {
      folder.value = folders.value[0]
      scheduleAnalyze()
    }
  }

  function clearAllSources() {
    analysisRequestId += 1
    if (analyzeTimer) clearTimeout(analyzeTimer)
    analyzing.value = false
    folders.value = []
    folder.value = ''
    explicitPaths.value = []
    analysis.value = null
    generatedResult.value = null
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
    folders.value = normalized
    folder.value = parentDir(normalized[0])
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
    if (requestId === analysisRequestId) analyzing.value = false
  }

  async function run() {
    if (!folders.value.length || analyzing.value || generating.value) return
    if (!includedImages.value.length) {
      ElMessage.warning('当前没有参与排版的图片，请先恢复至少一张图片')
      return
    }
    generating.value = true
    const sourceRevision = analysisRequestId
    const totalP = totalPages.value
    const pageScalesArray = Array.from({ length: totalP }, (_, i) => pageScales.value[i] ?? 1.0)
    const result = await tauriCallSafe('run_image_paddler', {
      args: {
        folder: folder.value,
        folders: folders.value,
        image_paths: includedImages.value.map((image) => image.path),
        output_stem: inputContext.value.sourceStem || undefined,
        ...settings,
        order_mode: 'custom',
        fixed_width_mm: actualImageWidth.value,
        orientation: resolvedOrientation.value,
        page_scales: pageScalesArray,
        pair_mode: settings.pair_mode,
        caption_position: settings.caption_position,
        print_safety_pad_mm: settings.print_safety_pad_mm,
        image_annotations: imageAnnotations.value,
        reserve_note_placeholder: settings.reserve_note_placeholder,
        note_placeholder_text: settings.note_placeholder_text,
        note_font_family: settings.note_font_family,
        note_font_size_pt: settings.note_font_size_pt,
        note_color: settings.note_color,
      },
    })
    generating.value = false
    if (sourceRevision !== analysisRequestId) return
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

  function imageTitle(path) {
    const custom = imageAnnotations.value[path]?.title
    if (custom && custom.trim()) return custom.trim()
    return fileName(path)
  }

  function imageDescription(path) {
    const custom = imageAnnotations.value[path]?.description
    if (custom && custom.trim()) return custom.trim()
    if (settings.reserve_note_placeholder) {
      return (settings.note_placeholder_text || '[点击输入说明]').trim()
    }
    return ''
  }

  function getImageAnnotation(path) {
    return imageAnnotations.value[path] || {}
  }

  function setImageAnnotation(path, patch) {
    if (!path) return
    const current = imageAnnotations.value[path] || {}
    imageAnnotations.value = {
      ...imageAnnotations.value,
      [path]: { ...current, ...patch },
    }
    generatedResult.value = null
  }

  function clearImageAnnotation(path) {
    if (!path) return
    const next = { ...imageAnnotations.value }
    delete next[path]
    imageAnnotations.value = next
    generatedResult.value = null
  }

  function noteLines(path) {
    const desc = imageDescription(path)
    if (!desc) return []
    return wrapFilenameLines(desc, layoutMetrics.value.cellWidth, 3)
  }

  function imageItemName(img) {
    return imageTitle(img?.path || '')
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
      while (nameUnits(last) + 1 > maxUnits && last.length) last = Array.from(last).slice(0, -1).join('')
      lines[lines.length - 1] = `${last}…`
    }
    return lines.length ? lines : ['']
  }

  function nameUnits(value) {
    return [...value].reduce((sum, ch) => sum + (ch.charCodeAt(0) < 128 ? 1 : 2), 0)
  }

  function previewImageAreaContainerStyle(index) {
    const offsets = pairOffsets(
      previewImages.value.length,
      previewLayoutGrid.value.rows,
      previewLayoutGrid.value.cols,
      settings.pair_mode,
    )
    const align = pairAlignToXY(offsets[index])
    const justifyMap = { left: 'flex-start', center: 'center', right: 'flex-end' }
    const alignMap = { top: 'flex-start', center: 'center', bottom: 'flex-end' }
    return {
      justifyContent: justifyMap[align.x] || 'center',
      alignItems: alignMap[align.y] || 'center',
    }
  }

  function previewImageStyle(img) {
    const metrics = layoutMetrics.value
    const nativeWidth = (img.width * 25.4) / settings.dpi
    const nativeHeight = (img.height * 25.4) / settings.dpi
    let drawWidth, drawHeight
    const scaleFactor = currentPageScale.value
    if (settings.scale_mode === 'fixed_width') {
      const fixedW = actualImageWidth.value * scaleFactor
      const ratio = img.width > 0 ? img.height / img.width : 1
      drawWidth = fixedW
      drawHeight = fixedW * ratio
    } else {
      const fitScale = Math.min(metrics.cellWidth / nativeWidth, metrics.imageCellHeight / nativeHeight) * scaleFactor
      const scale = settings.scale_mode === 'original' ? Math.min(fitScale, scaleFactor) : fitScale
      drawWidth = nativeWidth * scale
      drawHeight = nativeHeight * scale
    }
    return {
      width: `${(drawWidth / metrics.cellWidth) * 100}%`,
      height: `${(drawHeight / metrics.imageCellHeight) * 100}%`,
      maxWidth: 'none',
      maxHeight: 'none',
      flexShrink: 0,
      display: 'block',
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
    analysisRequestId += 1
    analyzing.value = true
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
    if (recommended.recommended_width_mm) {
      settings.fixed_width_mm = Number(recommended.recommended_width_mm)
      settings.scale_mode = 'fixed_width'
    } else {
      settings.scale_mode = recommended.scale_mode || 'fit'
    }
    settings.margin_mm = Number(recommended.margin_mm || 12)
    settings.show_filename = recommended.show_filename !== false
    if (showMessage) ElMessage.success('已应用推荐参数')
  }

  function adjustPageZoom(delta) {
    pageZoom.value = clampNumber(pageZoom.value + delta, 20, 200, 100)
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
      folders.value = [...new Set([...folders.value, ...accepted])]
      folder.value = folders.value[0]
      explicitPaths.value = []
      inputContext.value = { sourceKind: 'drop', sourceLabel: '', sourceStem: '' }
      sourceDecisions.value = {}
      scheduleAnalyze()
    },
  })

  onBeforeUnmount(() => {
    analysisRequestId += 1
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
    if (preferenceRevision.value < 3) {
      if (!settings.caption_position) settings.caption_position = 'below'
      if (!settings.pair_mode) settings.pair_mode = 'cell-center'
      if (settings.print_safety_pad_mm === undefined) settings.print_safety_pad_mm = 6
      if (settings.doclet_layout_tips === undefined) settings.doclet_layout_tips = true
      if (pageScales.value === undefined || typeof pageScales.value !== 'object') pageScales.value = {}
      preferenceRevision.value = 3
    }
    if (preferenceRevision.value < 4) {
      if (settings.reserve_note_placeholder === undefined) settings.reserve_note_placeholder = false
      if (!settings.note_placeholder_text) settings.note_placeholder_text = '[点击输入说明]'
      if (!settings.note_font_family) settings.note_font_family = 'kaiti'
      if (!settings.note_font_size_pt) settings.note_font_size_pt = 8
      if (!settings.note_color) settings.note_color = 'gray'
      if (imageAnnotations.value === undefined || typeof imageAnnotations.value !== 'object') imageAnnotations.value = {}
      preferenceRevision.value = 4
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
    if (isFlowLayout.value) return `${grid.rows * grid.cols} 张（上下）`
    if (value === '1') return '1 张'
    if (value === '1x2') return '2 张（左右）'
    if (value === '2x1') return '2 张（上下）'
    return `${grid.rows} 行 × ${grid.cols} 列`
  }

  function scaleModeLabel(value) {
    if (value === 'fixed_width') return '统一图片宽度'
    if (value === 'original') return '不缩放'
    return '适应页面'
  }

  const DOCLET_TIPS = {
    yellow: '本页图片有重叠，可微调本页比例或调小统一宽度。',
    green: '图片超出打印安全区，可调小本页比例或页边距。',
    red: '本页重叠且超界，请优先检查该页布局。',
    blue: '本页比例差异较大，建议人工核对排版。',
    ok: '',
  }

  function getPageConflicts(pageIdx) {
    const count = perPage.value
    const start = pageIdx * count
    const pageImgs = includedImages.value.slice(start, start + count)
    if (!pageImgs.length) return { items: [], hasOverflow: false, hasOverlap: false, hasCaptionOverlap: false }
    const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
    const currentScale =
      pageIdx === currentPageIndex.value
        ? activePageScale.value / 100
        : pageScales.value[pageIdx] ?? 1.0

    const pageGrid = compactGridForCount(layoutGrid.value, pageImgs.length)
    const metrics = computeMetricsForGrid(pageGrid, pageImgs)
    const pageImgsWithAnnotations = pageImgs.map((img) => ({
      ...img,
      title: imageTitle(img.path),
      description:
        imageDescription(img.path) || (settings.reserve_note_placeholder ? settings.note_placeholder_text : ''),
    }))

    return detectPageConflicts({
      images: pageImgsWithAnnotations,
      grid: pageGrid,
      cellWidth: metrics.cellWidth,
      imageCellHeight: metrics.imageCellHeight,
      fixedWidthMm: actualImageWidth.value,
      pageScale: currentScale,
      scaleMode: settings.scale_mode,
      dpi: settings.dpi,
      pageWidth: page.width,
      pageHeight: page.height,
      marginMm: Number(settings.margin_mm) || 0,
      showFilename: settings.show_filename,
      captionPosition: settings.caption_position,
      captionReserveMm: metrics.filenameReserve,
      fontSizePt: clampNumber(settings.filename_font_size_pt, 6, 24, 8),
      noteFontSizePt: clampNumber(settings.note_font_size_pt, 6, 24, 8),
      pairMode: settings.pair_mode,
    })
  }

  const currentPageConflictReport = computed(() => getPageConflicts(currentPageIndex.value))
  const currentPageConflicts = computed(() => currentPageConflictReport.value.items || [])

  const allPageConflictReports = computed(() => {
    const total = totalPages.value
    const reports = new Array(total)
    for (let p = 0; p < total; p++) {
      reports[p] = getPageConflicts(p)
    }
    return reports
  })

  const currentPageConflictState = computed(() => {
    const report = currentPageConflictReport.value
    const conflicts = report.items || []
    const colors = conflicts.map((c) => c.color)
    const priority = ['red', 'yellow', 'green', 'blue', 'ok']
    let worstColor = 'ok'
    for (const p of priority) {
      if (colors.includes(p)) {
        worstColor = p
        break
      }
    }

    let tip = DOCLET_TIPS[worstColor] || ''
    if (worstColor === 'green') {
      const isHighDensity = layoutGrid.value.rows * layoutGrid.value.cols >= 6
      if (isHighDensity && settings.show_filename) {
        tip = '高密度排版图片易超界，可调小统一宽度、缩小字号或关闭文件名。'
      } else if (report.hasCaptionOverlap) {
        tip = '图片探入标题区域，可调小本页比例或调小统一宽度。'
      }
    }

    return {
      worstColor,
      docletTip: tip,
    }
  })

  function imageBadgeResolver(item) {
    const index = includedImages.value.findIndex((img) => img.path === item.path)
    if (index === -1) return null
    const count = perPage.value
    const pageIdx = Math.floor(index / count)
    const report = allPageConflictReports.value[pageIdx]
    const slotIdx = index % count
    const color = report?.items?.[slotIdx]?.color || 'ok'
    const isModified = pageScales.value[pageIdx] !== undefined
    return {
      pageNumber: pageIdx + 1,
      pageIndex: pageIdx,
      isModified,
      color,
    }
  }

  const conflictDialogVisible = ref(false)

  const conflictSummaryList = computed(() => {
    const list = []
    const reports = allPageConflictReports.value || []
    reports.forEach((report, pageIdx) => {
      if (!report || !report.items) return
      report.items.forEach((item, slotIdx) => {
        if (['red', 'yellow', 'green'].includes(item.color)) {
          let reason = '存在布局冲突'
          if (item.overlap && item.overflow) {
            reason = '图片重叠且超出边距'
          } else if (item.overlap) {
            reason = '与其他图片重叠'
          } else if (item.captionOverlap) {
            reason = '遮挡或压盖图注'
          } else if (item.overflow) {
            reason = '超出单元格或页边距'
          }
          const rawName = item.path ? item.path.split(/[/\\]/).pop() : `图片 ${slotIdx + 1}`
          const customTitle = imageAnnotations.value[item.path]?.title
          const displayName = customTitle ? `${customTitle} (${rawName})` : rawName
          list.push({
            pageNumber: pageIdx + 1,
            pageIndex: pageIdx,
            slotNumber: slotIdx + 1,
            path: item.path,
            displayName,
            color: item.color,
            reason,
          })
        }
      })
    })
    return list
  })

  async function handleStartGenerate() {
    if (!folders.value.length || analyzing.value || generating.value) return
    if (!includedImages.value.length) {
      ElMessage.warning('当前没有参与排版的图片，请先恢复至少一张图片')
      return
    }
    if (conflictSummaryList.value.length > 0) {
      conflictDialogVisible.value = true
      return
    }
    await run()
  }

  return {
    conflictDialogVisible,
    conflictSummaryList,
    handleStartGenerate,
    isFlowLayout,
    optionLayoutLabel,
    safeColumnWidthValue,
    maximumImageWidth,
    actualImageWidth,
    widthIsLimited,
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
    previewPageFraction,
    settings,
    layoutGrid,
    resolvedOrientation,
    resolvedOrientationLabel,
    orderedImages,
    includedImages,
    excludedCount,
    perPage,
    isPairLayout,
    totalPages,
    currentPageIndex,
    pageScales,
    activePageScale,
    scaleScope,
    globalScalePercent,
    setGlobalScale,
    hasSavedScale,
    hasAnySavedScales,
    isCurrentPageDirty,
    saveCurrentPageScale,
    cancelCurrentPageScale,
    resetCurrentPageScale,
    autoFitCurrentPageScale,
    autoFitAllPagesScale,
    applyScaleToAllPages,
    applyScaleToSubsequentPages,
    resetAllPageScales,
    nextPage,
    prevPage,
    goToPage,
    previewImages,
    previewSlots,
    previewLayoutGrid,
    generatedOutputPaths,
    previewPageStyle,
    previewGridStyle,
    previewCellStyle,
    layoutMetrics,
    previewImageAreaStyle,
    previewImageAreaContainerStyle,
    previewNameStyle,
    previewNoteStyle,
    selectFolder,
    addFolders,
    addImages,
    removeFolder,
    clearAllSources,
    currentRecommendedWidth,
    baseFileName,
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
    imageAnnotations,
    imageTitle,
    imageDescription,
    getImageAnnotation,
    setImageAnnotation,
    clearImageAnnotation,
    noteLines,
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
    currentPageConflicts,
    currentPageConflictState,
    imageBadgeResolver,
  }
}
