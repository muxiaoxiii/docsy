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
  pairOffsets,
  pairAlignToXY,
  detectPageConflicts,
  computeOptimalPageScale,
  layoutOptionLabel,
} from './layoutPreview.js'
import {
  PER_PAGE_CHOICES,
  arrangeOptionsFor,
  controlsFromLayout,
  importRecommendationDrifts,
  isFlowLayoutMode,
  layoutFromControls,
  parseLayoutString,
} from './layoutControls.js'

import { buildOutputPlan, migratePaddlerPreferences } from './outputPlan.js'

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
  const pageScalePlanKey = ref('')
  const activePageScale = ref(100)
  const imageAnnotations = ref({})
  let analyzeTimer = null
  let analysisRequestId = 0
  let preferencesReady = false

  const settings = reactive({
    output_format: 'pdf',
    output_dir: '',
    output_mode: 'merged',
    layout: '2x1',
    custom_rows: 2,
    custom_cols: 2,
    // 布局控件：每页张数 + 排列方式（与 layout 双向同步）
    images_per_page: 2,
    arrange_mode: 'stack',
    last_page_mode: 'keep', // keep=末页保留原网格；reflow=末页重新铺满
    // smart=跟随当前布局智能统一宽度；manual=手动宽度；fit/original=缩放模式
    size_mode: 'smart',
    scale_mode: 'fixed_width',
    orientation: 'auto',
    dpi: 300,
    margin_mm: 12,
    show_filename: true,
    caption_position: 'below',
    caption_gap_mm: 2,
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
  // 全局缩放独立存储；pageScales 仅保存「显式局部覆盖」
  const globalScalePercent = ref(100)
  // 设置区宽度（px），与证据模块一样可拖拽；写入工作区偏好
  const settingsPanelWidth = ref(340)
  const preference = useWorkspacePreferences('image-paddler.workspace', {
    settings,
    pageZoom,
    previewPageFraction,
    exclusionByPath,
    preferenceRevision,
    pageScales,
    pageScalePlanKey,
    imageAnnotations,
    globalScalePercent,
    settingsPanelWidth,
  }, { migrate: migratePaddlerPreferences })

  const isFlowLayout = computed(() => isFlowLayoutMode(settings.output_format, settings.use_table))
  const arrangeOptions = computed(() => arrangeOptionsFor(settings.images_per_page, isFlowLayout.value))
  const layoutGrid = computed(() => {
    const grid = parseLayoutString(settings.layout, settings.custom_rows, settings.custom_cols)
    return isFlowLayout.value ? { rows: grid.rows * grid.cols, cols: 1 } : grid
  })
  const isFrameSequence = computed(() => inputContext.value.sourceKind === 'video-frames')
  const resolvedOrientation = computed(() => {
    if (settings.orientation !== 'auto') return settings.orientation
    return analysis.value?.recommended?.orientation || 'portrait'
  })
  const resolvedOrientationLabel = computed(() => (resolvedOrientation.value === 'landscape' ? '横向' : '竖向'))
  const orderedImages = computed(() => {
    const images = (analysis.value?.images || []).map(image => {
      const rotation = imageRotation(image.path)
      const swapped = rotation === 90 || rotation === 270
      return { ...image, width: swapped ? image.height : image.width,
        height: swapped ? image.width : image.height, rotation_degrees: rotation }
    })
    if (settings.output_mode === 'per_folder') {
      return buildOutputPlan(images, Math.max(1, images.length), 'per_folder')
        .flatMap(file => reorderImages(file.pages[0].images, layoutGrid.value, settings.order_mode))
    }
    return reorderImages(images, layoutGrid.value, settings.order_mode)
  })
  const includedImages = computed(() => orderedImages.value.filter((image) => !isImageExcluded(image)))
  const excludedCount = computed(() => orderedImages.value.length - includedImages.value.length)
  const perPage = computed(() => Math.max(1, layoutGrid.value.rows * layoutGrid.value.cols))
  const isPairLayout = computed(() => perPage.value === 2 && !isFlowLayout.value)
  const captionControlsEnabled = computed(() => settings.show_filename || settings.reserve_note_placeholder || includedImages.value.some(image => imageDescription(image.path)))
  const captionGapMm = computed(() =>
    captionControlsEnabled.value
      ? Math.max(0, Math.min(20, Number(settings.caption_gap_mm) || 0))
      : 0,
  )

  /** 布局感知推荐：跟随每页张数与页面方向，不是导入时那一套。 */
  const layoutAwareRecommendation = computed(() => {
    let width = safeColumnWidthValue.value
    let feasible = true
    for (const entry of outputPages.value) {
      const grid = settings.last_page_mode === 'reflow'
        ? compactGridForCount(layoutGrid.value, entry.images.length) : layoutGrid.value
      const metrics = computeMetricsForGrid(grid, entry.images)
      width = Math.min(width, metrics.cellWidth - Math.max(0, Number(settings.print_safety_pad_mm) || 0))
      if (metrics.filenameReserve >= metrics.cellHeight) feasible = false
      for (const image of entry.images) {
        width = Math.min(width, metrics.imageCellHeight * image.width / Math.max(1, image.height),
          image.width * 25.4 / clampNumber(settings.dpi, 72, 1200, 300))
      }
    }
    feasible = feasible && width >= 0.1
    const recommendedWidth = Math.max(0.1, Math.floor(width * 10) / 10)
    return {
      recommended_width_mm: recommendedWidth,
      safe_column_width_mm: safeColumnWidthValue.value,
      feasible,
      reason: feasible ? `${layoutGrid.value.rows}×${layoutGrid.value.cols} · 按实际标题和说明计算 ${recommendedWidth} mm`
        : '标题或说明已占满单元格，请减少每页张数或缩小字号',
    }
  })

  const importRecommendationDrifted = computed(() =>
    importRecommendationDrifts(analysis.value?.recommended, layoutGrid.value, perPage.value),
  )

  function syncLayoutFromControls() {
    const flow = isFlowLayout.value
    const next = layoutFromControls({
      imagesPerPage: settings.images_per_page,
      arrangeMode: settings.arrange_mode,
      customRows: settings.custom_rows,
      customCols: settings.custom_cols,
      flow,
    })
    settings.layout = next.layout
    if (next.customRows != null) settings.custom_rows = next.customRows
    if (next.customCols != null) settings.custom_cols = next.customCols
    if (flow && settings.arrange_mode !== 'stack') settings.arrange_mode = 'stack'
  }

  function setImagesPerPage(value) {
    settings.images_per_page = value
    if (value !== 'custom') {
      const options = arrangeOptionsFor(value, isFlowLayout.value)
      if (!options.some((option) => option.value === settings.arrange_mode)) {
        settings.arrange_mode = options[0]?.value || 'stack'
      }
    }
    syncLayoutFromControls()
    if (settings.size_mode === 'smart') applyLayoutAwareSize({ silent: true })
  }

  function setArrangeMode(value) {
    settings.arrange_mode = value
    syncLayoutFromControls()
    if (settings.size_mode === 'smart') applyLayoutAwareSize({ silent: true })
  }

  function setSizeMode(mode) {
    settings.size_mode = mode
    if (mode === 'smart') {
      settings.scale_mode = 'fixed_width'
      applyLayoutAwareSize({ silent: false })
    } else if (mode === 'fit' || mode === 'original') {
      settings.scale_mode = mode
    } else if (mode === 'manual') {
      settings.scale_mode = 'fixed_width'
    }
  }

  function applyLayoutAwareSize({ silent = true } = {}) {
    const rec = layoutAwareRecommendation.value
    if (!rec) return
    settings.size_mode = 'smart'
    settings.scale_mode = 'fixed_width'
    settings.fixed_width_mm = rec.recommended_width_mm
    if (!silent) ElMessage.success(`智能宽度 ${rec.recommended_width_mm} mm`)
  }

  function clampSettingsPanelWidth(value) {
    return clampNumber(Number(value), 280, 520, 340)
  }

  function setSettingsPanelWidth(value) {
    settingsPanelWidth.value = clampSettingsPanelWidth(value)
  }

  function startSettingsPanelResize(event, layoutEl) {
    if (event.button !== 0 || !layoutEl) return
    event.preventDefault()
    const rect = layoutEl.getBoundingClientRect()
    const update = (pointerEvent) => {
      setSettingsPanelWidth(pointerEvent.clientX - rect.left)
    }
    const finish = () => {
      window.removeEventListener('pointermove', update)
      window.removeEventListener('pointerup', finish)
      window.removeEventListener('pointercancel', finish)
    }
    window.addEventListener('pointermove', update)
    window.addEventListener('pointerup', finish, { once: true })
    window.addEventListener('pointercancel', finish, { once: true })
    update(event)
  }

  function applyLayoutAwareRecommendation(showMessage = true) {
    applyLayoutAwareSize({ silent: !showMessage })
  }

  // 控件 → layout 字符串
  watch(
    [
      () => settings.images_per_page,
      () => settings.arrange_mode,
      () => settings.use_table,
      () => settings.output_format,
      () => settings.custom_rows,
      () => settings.custom_cols,
    ],
    () => syncLayoutFromControls(),
  )

  const outputPlan = computed(() => buildOutputPlan(includedImages.value, perPage.value, settings.output_mode))
  const outputPages = computed(() => outputPlan.value.flatMap(file => file.pages))
  const totalPages = computed(() => Math.max(1, outputPages.value.length))
  const outputPlanKey = computed(() => JSON.stringify({
    grid: layoutGrid.value, orientation: resolvedOrientation.value,
    format: settings.output_format, flow: isFlowLayout.value, last: settings.last_page_mode,
    files: outputPlan.value,
  }))
  // A local page override belongs only to the exact saved content and pagination.
  watch(outputPlanKey, key => {
    if (!includedImages.value.length) return
    if (pageScalePlanKey.value !== key) {
      pageScales.value = {}
      activePageScale.value = globalScalePercent.value
    }
    pageScalePlanKey.value = key
  }, { flush: 'sync' })


  watch([totalPages, includedImages], () => {
    if (currentPageIndex.value >= totalPages.value) {
      currentPageIndex.value = Math.max(0, totalPages.value - 1)
    }
  })

  const previewImages = computed(() => outputPages.value[currentPageIndex.value]?.images || [])

  const previewSlots = computed(() => {
    const slots = [...previewImages.value]
    if (settings.border_enabled && !isFlowLayout.value && slots.length) {
      const capacity = previewLayoutGrid.value.rows * previewLayoutGrid.value.cols
      while (slots.length < capacity) slots.push(null)
    }
    return slots
  })
  const previewLayoutGrid = computed(() => {
    const base = layoutGrid.value
    if (settings.last_page_mode !== 'reflow') return base
    return compactGridForCount(base, previewImages.value.length)
  })

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
    if (!settings.border_enabled || isFlowLayout.value) {
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
    // 固定基础预留区，用户间距只影响文字坐标。
    const gapMm = titleReserve > 0 || noteReserve > 0 ? captionGapMm.value : 0
    const filenameReserve =
      titleReserve > 0 || noteReserve > 0 ? titleReserve + noteReserve + filenameSafetyMm + 2 : 0

    return {
      cellWidth,
      cellHeight,
      filenameReserve,
      titleReserve,
      noteReserve,
      captionGapMm: gapMm,
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
    // 图片区固定在单元格内：图上标题时下移 reserve，图下标题时贴顶；与 caption_gap 无关
    return {
      height,
      position: 'absolute',
      left: 0,
      width: '100%',
      top: settings.caption_position === 'above' ? `${(metrics.filenameReserve / metrics.cellHeight) * 100}%` : '0%',
    }
  })

  function captionTextHeightMm(image, metrics) {
    const titleLines = settings.show_filename
      ? requiredFilenameLines(imageTitle(image.path), metrics.cellWidth, metrics.filenameFontSizePt, FILENAME_MAX_LINES)
      : 0
    const note = imageDescription(image.path) || (settings.reserve_note_placeholder ? settings.note_placeholder_text : '')
    const noteLineCount = note
      ? requiredFilenameLines(note, metrics.cellWidth, metrics.noteFontSizePt, NOTE_MAX_LINES)
      : 0
    return titleLines * metrics.filenameLineHeightMm + noteLineCount * metrics.noteLineHeightMm
  }

  // Reuse conflict geometry so preview text, images and export follow the same positions.
  function previewCaptionStyle(image, index) {
    const metrics = layoutMetrics.value
    const box = currentPageConflictReport.value.imageBoxes?.[index]
    if (!box || !image || captionTextHeightMm(image, metrics) <= 0) return { display: 'none' }
    const row = Math.floor(index / previewLayoutGrid.value.cols)
    const imageTop = box.y - settings.margin_mm - row * metrics.cellHeight
    const textHeight = captionTextHeightMm(image, metrics)
    const top = settings.caption_position === 'above'
      ? imageTop - textHeight - captionGapMm.value
      : imageTop + box.h + captionGapMm.value
    return { position: 'absolute', left: 0, width: '100%', top: `${top / metrics.cellHeight * 100}%` }
  }
  const previewNameStyle = computed(() => {
    const metrics = layoutMetrics.value
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
  /** 布局感知推荐宽度（同时受栏宽与格高约束），不再是“只按列数”的旧推荐。 */
  const currentRecommendedWidth = computed(
    () => layoutAwareRecommendation.value?.recommended_width_mm ?? Math.floor(safeColumnWidthValue.value * 10) / 10,
  )
  const actualImageWidth = computed(() => Math.min(Math.max(0.1, Number(settings.fixed_width_mm) || 160), 500))
  const widthIsLimited = computed(() => Number(settings.fixed_width_mm) > safeColumnWidthValue.value + 0.5)

  watch(layoutAwareRecommendation, rec => {
    if (settings.size_mode === 'smart') {
      settings.scale_mode = 'fixed_width'
      settings.fixed_width_mm = rec.recommended_width_mm
    }
  })

  function effectiveScaleForPage(pageIdx) {
    const idx = Number(pageIdx)
    if (idx === currentPageIndex.value) {
      return (Number(activePageScale.value) || 100) / 100
    }
    const local = pageScales.value[idx]
    if (local !== undefined) return Number(local) || 1
    return (Number(globalScalePercent.value) || 100) / 100
  }

  watch(
    [currentPageIndex, pageScales, globalScalePercent],
    () => {
      const local = pageScales.value[currentPageIndex.value]
      if (local !== undefined) {
        activePageScale.value = Math.round(local * 100)
      } else {
        activePageScale.value = Math.round(Number(globalScalePercent.value) || 100)
      }
    },
    { flush: 'sync', immediate: true, deep: true },
  )

  const hasSavedScale = computed(() => pageScales.value[currentPageIndex.value] !== undefined)
  /** 相对全局是否已局部偏离（即时生效；取消=回到全局） */
  const isCurrentPageDirty = computed(() => {
    return activePageScale.value !== Math.round(Number(globalScalePercent.value) || 100)
  })

  /** 即时生效：预览改动直接写入局部覆盖，导出与预览/冲突检测同一状态。 */
  function commitActivePageScaleToState() {
    const percent = Math.round(Number(activePageScale.value) || 100)
    const globalPercent = Math.round(Number(globalScalePercent.value) || 100)
    const next = { ...pageScales.value }
    if (percent === globalPercent) {
      delete next[currentPageIndex.value]
    } else {
      next[currentPageIndex.value] = percent / 100
    }
    pageScales.value = next
  }

  watch(activePageScale, () => {
    commitActivePageScaleToState()
  }, { flush: 'sync' })

  function saveCurrentPageScale() {
    commitActivePageScaleToState()
    ElMessage.success(`第 ${currentPageIndex.value + 1} 页比例已生效 (${activePageScale.value}%)`)
  }

  function cancelCurrentPageScale() {
    // 即时生效模型下，取消 = 清除本页局部覆盖，回到全局比例
    const next = { ...pageScales.value }
    delete next[currentPageIndex.value]
    pageScales.value = next
    activePageScale.value = Math.round(Number(globalScalePercent.value) || 100)
  }

  function resetCurrentPageScale() {
    const next = { ...pageScales.value }
    delete next[currentPageIndex.value]
    pageScales.value = next
    activePageScale.value = Math.round(Number(globalScalePercent.value) || 100)
    ElMessage.success(`第 ${currentPageIndex.value + 1} 页已恢复继承全局 ${globalScalePercent.value}%`)
  }

  const hasAnySavedScales = computed(() => Object.keys(pageScales.value).length > 0)

  function pageGeometryFor(pageIdx) {
    const pageImgs = outputPages.value[pageIdx]?.images || []
    const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
    const pageGrid =
      settings.last_page_mode === 'reflow'
        ? compactGridForCount(layoutGrid.value, pageImgs.length)
        : layoutGrid.value
    const metrics = computeMetricsForGrid(pageGrid, pageImgs)
    return { pageImgs, page, pageGrid, metrics }
  }

  function computePageOptimalScale(pageIdx) {
    const { pageImgs, page, pageGrid, metrics } = pageGeometryFor(pageIdx)
    if (!pageImgs.length) return { ok: false, scale: null, reason: '本页没有图片' }
    const pageImgsWithAnnotations = pageImgs.map((img) => ({
      ...img,
      title: imageTitle(img.path),
      captionHeightMm: captionTextHeightMm(img, metrics),
      description:
        imageDescription(img.path) || (settings.reserve_note_placeholder ? settings.note_placeholder_text : ''),
    }))
    return computeOptimalPageScale({
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
      captionGapMm: captionGapMm.value,
      fontSizePt: clampNumber(settings.filename_font_size_pt, 6, 24, 8),
      noteFontSizePt: clampNumber(settings.note_font_size_pt, 6, 24, 8),
      pairMode: settings.pair_mode,
    })
  }

  function autoFitCurrentPageScale() {
    // 智能尺寸模式下，优先重算统一宽度，而不是叠加一层百分比
    if (settings.size_mode === 'smart') {
      applyLayoutAwareSize({ silent: true })
    }
    const result = computePageOptimalScale(currentPageIndex.value)
    if (result.ok) {
      activePageScale.value = result.scale
      commitActivePageScaleToState()
      ElMessage.success(`本页比例已设为 ${result.scale}%（与导出一致）`)
    } else {
      ElMessage.warning(result.reason || '无法在 30%~140% 内消除冲突，请减少每页张数、切换横向或改用适应页面')
    }
  }

  const scaleScope = ref('all')

  function setGlobalScale(val) {
    const percent = Math.round(Number(val) || 100)
    globalScalePercent.value = percent
    // 全局缩放独立存储：不把旧页码写死；无局部覆盖的页（含新增页）自动继承
    pageScales.value = {}
    activePageScale.value = percent
  }

  function autoFitAllPagesScale() {
    // 与「当前布局智能推荐」同一套几何：优先修正统一宽度
    if (settings.size_mode === 'smart' || settings.scale_mode === 'fixed_width') {
      applyLayoutAwareSize({ silent: true })
    }
    const total = totalPages.value
    let minOptimal = 140
    let hasAnyValid = false
    const failures = []
    for (let pageIdx = 0; pageIdx < total; pageIdx += 1) {
      const result = computePageOptimalScale(pageIdx)
      if (result.ok) {
        hasAnyValid = true
        if (result.scale < minOptimal) minOptimal = result.scale
      } else {
        failures.push(pageIdx + 1)
      }
    }
    if (!hasAnyValid) {
      ElMessage.warning(
        failures.length
          ? `智能尺寸后第 ${failures.join('、')} 页仍有冲突：请减少每页张数、切换横向，或改用「适应页面」`
          : '无法计算全局比例，请检查每页张数与图片尺寸',
      )
      return
    }
    setGlobalScale(minOptimal)
    if (failures.length) {
      ElMessage.warning(
        `全局比例已设为 ${minOptimal}%，但第 ${failures.join('、')} 页仍可能冲突，建议减少每页张数或改用适应页面`,
      )
    } else {
      ElMessage.success(`已按当前布局智能计算全局比例 ${minOptimal}%（新页面将自动继承）`)
    }
  }

  function applyScaleToAllPages(scaleOverride) {
    const targetScale = scaleOverride !== undefined ? scaleOverride : activePageScale.value / 100
    setGlobalScale(Math.round(targetScale * 100))
    ElMessage.success(`已将全局比例设为 ${globalScalePercent.value}%（全部页与后续新页继承）`)
  }

  function applyScaleToSubsequentPages(fromPageIndex, scaleOverride) {
    const startPage = fromPageIndex !== undefined ? fromPageIndex : currentPageIndex.value
    const targetScale = scaleOverride !== undefined ? scaleOverride : activePageScale.value / 100
    const targetPercent = Math.round(targetScale * 100)
    const globalPercent = Math.round(Number(globalScalePercent.value) || 100)
    const newScales = { ...pageScales.value }
    for (let i = startPage; i < totalPages.value; i += 1) {
      if (targetPercent === globalPercent) {
        delete newScales[i]
      } else {
        newScales[i] = targetScale
      }
    }
    pageScales.value = newScales
    ElMessage.success(`已将第 ${startPage + 1} 页及后续页设为 ${targetPercent}%（其余页仍继承全局）`)
  }

  function resetAllPageScales() {
    pageScales.value = {}
    globalScalePercent.value = 100
    activePageScale.value = 100
    ElMessage.success('已恢复全局 100%，并清除全部页面局部比例')
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
    if (!folders.value.length) {
      analyzing.value = false
      return
    }
    const requestId = ++analysisRequestId
    analyzing.value = true
    try {
      const result = await tauriCallSafe('analyze_image_paddler_folder', {
        folder: folder.value,
        folders: folders.value,
        imagePaths: explicitPaths.value.length ? explicitPaths.value : undefined,
      })
      if (requestId !== analysisRequestId) return
      if (result.ok) {
        analysis.value = result.data
        await preloadVisibleImages()
      } else {
        ElMessage.error(userFacingError(result.error, '图片文件夹分析失败，请确认文件夹路径正确'))
      }
    } catch (error) {
      if (requestId === analysisRequestId) {
        ElMessage.error(userFacingError(error, '图片分析失败，请重试或检查文件是否可读'))
      }
    } finally {
      if (requestId === analysisRequestId) analyzing.value = false
    }
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
    // 预览/冲突/导出共用 effectiveScaleForPage：当前页未点保存的调整也会进入导出
    const pageScalesArray = Array.from({ length: totalP }, (_, i) => effectiveScaleForPage(i))
    const result = await tauriCallSafe('run_image_paddler', {
      args: {
        folder: folder.value,
        folders: folders.value,
        image_paths: outputPages.value.flatMap(page => page.images.map(image => image.path)),
        output_stem: inputContext.value.sourceStem || undefined,
        ...settings,
        // 路径已按前端 order_mode 预排，后端保持 custom 避免二次重排
        order_mode: 'custom',
        border_enabled: settings.border_enabled && !isFlowLayout.value,
        fixed_width_mm: actualImageWidth.value,
        orientation: resolvedOrientation.value,
        page_scales: pageScalesArray,
        pair_mode: settings.pair_mode,
        caption_position: settings.caption_position,
        caption_gap_mm: captionGapMm.value,
        last_page_mode: settings.last_page_mode,
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
    const sourceByPath = new Map(analysis.value.images.map(image => [image.path, image]))
    const reordered = moveItem(orderedImages.value, from, to)
      .map(image => sourceByPath.get(image?.path))
      .filter(Boolean)
    if (!reordered.length) return
    analysis.value = {
      ...analysis.value,
      // Keep analysis dimensions in source orientation; rotation is applied only by orderedImages.
      images: reordered,
    }
    settings.order_mode = 'custom'
    generatedResult.value = null
  }

  async function chooseOutputDirectory() {
    if (generating.value) return
    const selected = await open({ directory: true, multiple: false, title: '选择输出文件夹', defaultPath: settings.output_dir || undefined })
    if (typeof selected === 'string') settings.output_dir = selected
  }

  const generatedOutputDirectories = computed(() => [...new Set(generatedOutputPaths.value.map(parentDir).filter(Boolean))])
  async function openGeneratedDirectory(path) {
    if (!path) return
    const result = await openPath(path)
    if (!result.ok) ElMessage.error(userFacingError(result.error, '无法打开输出文件夹'))
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
    return parseLayoutString(layout, customRows, customCols)
  }

  function compactGridForCount(baseGrid, count) {
    if (settings.last_page_mode !== 'reflow') return baseGrid
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

  function imageRotation(path) {
    const value = Number(imageAnnotations.value[path]?.rotation_degrees || 0)
    return [0, 90, 180, 270].includes(value) ? value : 0
  }

  function rotateImage({ path }) {
    if (!path || generating.value) return
    setImageAnnotation(path, { rotation_degrees: (imageRotation(path) + 90) % 360 })
  }

  function previewSourceKey(path) { return JSON.stringify([path, imageRotation(path)]) }

  function imageSrc(path) {
    return previewSources[previewSourceKey(path)] || ''
  }

  function fileName(path) {
    let name = baseFileName(path)
    if (settings.filename_without_ext) {
      name = name.replace(/\.[^.]+$/, '')
    }
    if (settings.filename_remove_text) name = name.split(settings.filename_remove_text).join('')
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
    return wrapFilenameLines(desc, layoutMetrics.value.cellWidth, 3, settings.note_font_size_pt)
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
    return wrapFilenameLines(imageTitle(path), layoutMetrics.value.cellWidth, layoutMetrics.value.filenameMaxLines)
  }

  function requiredFilenameLines(name, cellWidthMm, fontSizePt, maxLines) {
    return wrapFilenameLines(name, cellWidthMm, maxLines, fontSizePt).length
  }

  function wrapFilenameLines(name, cellWidthMm, maxLines, font = settings.filename_font_size_pt) {
    const rawLines = String(name || '').split(/\r?\n/).map(line => line.trim()).filter(Boolean)
    if (rawLines.length > 1) {
      const lines = []
      for (const line of rawLines) {
        if (lines.length >= maxLines) break
        lines.push(...wrapFilenameLines(line, cellWidthMm, maxLines - lines.length, font))
      }
      return lines
    }
    name = rawLines[0] || ''
    const fontSize = clampNumber(font, 6, 24, 8)
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

  function previewImageStyle(img, index = previewImages.value.findIndex(image => image.path === img.path)) {
    const metrics = layoutMetrics.value
    const box = currentPageConflictReport.value.imageBoxes?.[index]
    if (!box) return { display: 'none' }
    return {
      width: `${(box.w / metrics.cellWidth) * 100}%`,
      height: `${(box.h / metrics.imageCellHeight) * 100}%`,
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
        const key = previewSourceKey(path)
        if (previewSources[key]) return
        const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 900, rotationDegrees: imageRotation(path) })
        if (result.ok) previewSources[key] = result.data
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

  /** 导入推荐：根据素材整体给出方向/张数/网格/初始尺寸，仅在用户明确应用时整体采用。 */
  function applyImportRecommendation(showMessage = true) {
    const recommended = analysis.value?.recommended
    if (!recommended) return
    settings.orientation = recommended.orientation || 'auto'
    const controls = controlsFromLayout(recommended.layout || '2x1', settings.custom_rows, settings.custom_cols, isFlowLayout.value)
    settings.images_per_page = controls.imagesPerPage
    settings.arrange_mode = controls.arrangeMode
    if (controls.customRows) settings.custom_rows = controls.customRows
    if (controls.customCols) settings.custom_cols = controls.customCols
    syncLayoutFromControls()
    settings.margin_mm = Number(recommended.margin_mm || 12)
    settings.show_filename = recommended.show_filename !== false
    if (recommended.recommended_width_mm) {
      settings.size_mode = 'manual'
      settings.scale_mode = 'fixed_width'
      settings.fixed_width_mm = Number(recommended.recommended_width_mm)
    } else {
      settings.size_mode = 'manual'
      settings.scale_mode = recommended.scale_mode || 'fit'
    }
    // 应用导入方案后，按当前（刚被导入推荐改过的）布局再算一遍智能宽度提示
    if (showMessage) {
      ElMessage.success('已应用导入推荐方案（方向/每页张数/网格）。可再点「应用当前布局智能尺寸」按当前布局精调宽度')
    }
  }

  /** 兼容旧名：默认指向导入推荐。 */
  function applyRecommendedSettings(showMessage = true) {
    applyImportRecommendation(showMessage)
  }

  /** 当前布局智能推荐：尊重已选张数/方向/网格，只重算安全尺寸，不改回导入布局。 */
  function applyCurrentLayoutRecommendation(showMessage = true) {
    applyLayoutAwareRecommendation(showMessage)
  }

  function restoreSmartSize() {
    settings.size_mode = 'smart'
    applyLayoutAwareSize({ silent: false })
  }

  function markManualWidth() {
    if (settings.scale_mode === 'fixed_width') settings.size_mode = 'manual'
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
    if (preferenceRevision.value < 5) {
      // 1.0.7 布局/缩放状态迁移：解析旧 layout → 每页张数+排列；全局缩放独立；智能宽度覆盖失效固定宽度
      const controls = controlsFromLayout(
        settings.layout || '2x1',
        settings.custom_rows,
        settings.custom_cols,
        isFlowLayoutMode(settings.output_format, settings.use_table),
      )
      settings.images_per_page = controls.imagesPerPage ?? 2
      settings.arrange_mode = controls.arrangeMode || 'stack'
      if (controls.customRows) settings.custom_rows = controls.customRows
      if (controls.customCols) settings.custom_cols = controls.customCols
      if (settings.caption_gap_mm === undefined) settings.caption_gap_mm = 2
      if (settings.last_page_mode === undefined) settings.last_page_mode = 'keep'
      if (!settings.size_mode) {
        settings.size_mode = settings.scale_mode === 'fixed_width' ? 'manual' : settings.scale_mode === 'fit' || settings.scale_mode === 'original' ? settings.scale_mode : 'smart'
      }
      if (globalScalePercent.value === undefined || globalScalePercent.value === null) globalScalePercent.value = 100
      preferenceRevision.value = 5
    }
    preferenceRevision.value = 6
    settingsPanelWidth.value = clampNumber(settingsPanelWidth.value, 280, 520, 340)
    syncLayoutFromControls()
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
    const pageImgs = outputPages.value[pageIdx]?.images || []
    if (!pageImgs.length) return { items: [], hasOverflow: false, hasOverlap: false, hasCaptionOverlap: false }
    const page = resolvedOrientation.value === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
    const currentScale = effectiveScaleForPage(pageIdx)

    const pageGrid =
      settings.last_page_mode === 'reflow'
        ? compactGridForCount(layoutGrid.value, pageImgs.length)
        : layoutGrid.value
    const metrics = computeMetricsForGrid(pageGrid, pageImgs)
    const pageImgsWithAnnotations = pageImgs.map((img) => ({
      ...img,
      title: imageTitle(img.path),
      captionHeightMm: captionTextHeightMm(img, metrics),
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
      captionGapMm: captionGapMm.value,
      fontSizePt: clampNumber(settings.filename_font_size_pt, 6, 24, 8),
      noteFontSizePt: clampNumber(settings.note_font_size_pt, 6, 24, 8),
      pairMode: settings.pair_mode,
    })
  }

  const currentPageConflictReport = computed(() => getPageConflicts(currentPageIndex.value))
  const currentPageConflicts = computed(() => currentPageConflictReport.value.items || [])

  // Vue tracks the actual geometry inputs; do not maintain a second, incomplete cache key.
  const allPageConflictReports = computed(() =>
    outputPages.value.map((_, index) => getPageConflicts(index)),
  )
  function getConflictReportForPage(pageIdx) {
    return allPageConflictReports.value[pageIdx]
  }

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
    if (report.hasCaptionOverlap) {
      tip = '图片与标题或说明重叠；缩小后仍有冲突时，可切换「本页」并点击「智能」，或使用「适应」尺寸。'
    } else if (worstColor === 'green') {
      const isHighDensity = layoutGrid.value.rows * layoutGrid.value.cols >= 6
      if (conflicts.some(item => item.captionOverflow)) {
        tip = '标题或说明超出单元格，可减小间距或图片比例；图片位置不会随间距改变。'
      } else if (isHighDensity && settings.show_filename) {
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
    const pageIdx = outputPages.value.findIndex(page => page.images.some(image => image.path === item.path))
    if (pageIdx < 0) return null
    const slotIdx = outputPages.value[pageIdx].images.findIndex(image => image.path === item.path)
    const report = getConflictReportForPage(pageIdx)
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
          } else if (item.captionOverflow) {
            reason = '标题或说明超出单元格，请减小间距或图片比例'
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
    // Export preflight always scans the current snapshot, independently of display caching.
    if (outputPages.value.some((_, i) => {
      const report = getPageConflicts(i)
      return report.hasOverflow || report.hasOverlap || report.hasCaptionOverlap
    })) {
      conflictDialogVisible.value = true
      return
    }
    await run()
  }

  return {
    outputPlan,
    outputPages,
    conflictDialogVisible,
    conflictSummaryList,
    handleStartGenerate,
    isFlowLayout,
    arrangeOptions,
    PER_PAGE_CHOICES,
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
    layoutAwareRecommendation,
    importRecommendationDrifted,
    applyImportRecommendation,
    applyCurrentLayoutRecommendation,
    restoreSmartSize,
    markManualWidth,
    setImagesPerPage,
    setArrangeMode,
    setSizeMode,
    settingsPanelWidth,
    setSettingsPanelWidth,
    startSettingsPanelResize,
    captionGapMm,
    captionControlsEnabled,
    nextPage,
    prevPage,
    goToPage,
    previewImages,
    previewSlots,
    previewLayoutGrid,
    generatedOutputPaths,
    generatedOutputDirectories,
    chooseOutputDirectory,
    openGeneratedDirectory,
    previewPageStyle,
    previewGridStyle,
    previewCellStyle,
    layoutMetrics,
    previewImageAreaStyle,
    previewImageAreaContainerStyle,
    previewCaptionStyle,
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
    imageRotation,
    rotateImage,
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
