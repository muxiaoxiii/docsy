import { computed, ref, watch } from 'vue'
import {
  createDefaultFooterTextGroup,
  createDefaultHeaderGroup,
  createDefaultPageNumberGroup,
  groupsFor,
  selectedGroupFor,
  setSelectedGroup,
} from '../../pdf-tools/composables/useEvidencePdfSession.js'

/**
 * Manages header/footer/page-number rule state for the evidence PDF workbench.
 *
 * Extracted from EvidencePdfWorkbench.vue to reduce component size.
 * Handles group management, per-file vs global mode, legacy field proxies,
 * and workflow defaults.
 */
export function useHeaderFooterRules({
  overlayFiles,
  selectedOverlayFile,
  workflowMode,
  hasExistingEditRule,
  hasExistingConvertRule,
  hasExistingRemovalRule,
}) {
  // --- Constants ---
  const DEFAULT_RASTER_DPI = 200
  const HORIZONTAL_OFFSET_LIMIT_MM = 120

  // --- Core rule state ---
  const cleanupHeaderHeightMm = ref(18)
  const cleanupFooterHeightMm = ref(18)
  const insertHeaderFooterEnabled = ref(true)
  const globalApplyEnabled = ref(true)
  const globalHeaderGroup = ref(createDefaultHeaderGroup())
  const globalFooterTextGroup = ref(createDefaultFooterTextGroup())
  const globalPageNumberGroup = ref(createDefaultPageNumberGroup())
  const headerInsertEnabled = ref(false)
  const footerInsertEnabled = ref(false)
  const pageNumberShowTotal = ref(true)

  const normalizeA4 = ref(false)
  const a4Orientation = ref('preserve')
  const rasterDpi = ref(DEFAULT_RASTER_DPI)
  const removeAnnotations = ref(false)
  const bookmarkEnabled = ref(false)
  const bookmarkRemoveExisting = ref(false)
  const bookmarkLabelSource = ref('header')
  const annotationKinds = ref([
    'Text', 'FreeText', 'Highlight', 'Underline', 'StrikeOut',
    'Squiggly', 'Ink', 'Stamp', 'Square', 'Circle', 'Line', 'Polygon', 'PolyLine',
  ])

  const footerEnabled = ref(false)
  const footerText = ref('{page}/{total}')
  const footerContinuous = ref(true)
  const pageNumberOverrides = ref([])
  const pageNumberRulesVisible = ref(false)

  const outputMode = ref('files_and_merge')
  const mergeFileName = ref('merged_evidence.pdf')
  const fileSuffixEnabled = ref(true)
  const fileSuffixText = ref('processed')

  // --- Group computeds ---
  const headerGroups = computed(() =>
    globalApplyEnabled.value ? [globalHeaderGroup.value] : groupsFor(selectedOverlayFile.value, 'header'),
  )
  const footerTextGroups = computed(() =>
    globalApplyEnabled.value ? [globalFooterTextGroup.value] : groupsFor(selectedOverlayFile.value, 'footerText'),
  )
  const pageNumberGroups = computed(() =>
    globalApplyEnabled.value ? [globalPageNumberGroup.value] : groupsFor(selectedOverlayFile.value, 'pageNumber'),
  )

  // Writable models so HeaderFooterRuleFields can add/remove groups per file
  const headerGroupsModel = computed({
    get: () => globalApplyEnabled.value
      ? [globalHeaderGroup.value]
      : selectedOverlayFile.value?.headerGroups || [],
    set: (v) => {
      if (globalApplyEnabled.value) {
        globalHeaderGroup.value = v[0] || createDefaultHeaderGroup()
      } else if (selectedOverlayFile.value) {
        selectedOverlayFile.value.headerGroups = v
      }
    },
  })
  const footerTextGroupsModel = computed({
    get: () => globalApplyEnabled.value
      ? [globalFooterTextGroup.value]
      : selectedOverlayFile.value?.footerTextGroups || [],
    set: (v) => {
      if (globalApplyEnabled.value) {
        globalFooterTextGroup.value = v[0] || createDefaultFooterTextGroup()
      } else if (selectedOverlayFile.value) {
        selectedOverlayFile.value.footerTextGroups = v
      }
    },
  })
  const pageNumberGroupsModel = computed({
    get: () => globalApplyEnabled.value
      ? [globalPageNumberGroup.value]
      : selectedOverlayFile.value?.pageNumberGroups || [],
    set: (v) => {
      if (globalApplyEnabled.value) {
        globalPageNumberGroup.value = v[0] || createDefaultPageNumberGroup()
      } else if (selectedOverlayFile.value) {
        selectedOverlayFile.value.pageNumberGroups = v
      }
    },
  })
  const pageNumberTemplate = computed({
    get: () => selectedPageNumberGroup.value.template || '{page}/{total}',
    set: (v) => { selectedPageNumberGroup.value.template = v },
  })

  // --- Selected group computeds ---
  const selectedHeaderGroup = computed(() =>
    globalApplyEnabled.value
      ? globalHeaderGroup.value
      : selectedGroupFor(selectedOverlayFile.value, 'header') || createDefaultHeaderGroup(),
  )
  const selectedFooterTextGroup = computed(() =>
    globalApplyEnabled.value
      ? globalFooterTextGroup.value
      : selectedGroupFor(selectedOverlayFile.value, 'footerText') || createDefaultFooterTextGroup(),
  )
  const selectedPageNumberGroup = computed(() =>
    globalApplyEnabled.value
      ? globalPageNumberGroup.value
      : selectedGroupFor(selectedOverlayFile.value, 'pageNumber') || createDefaultPageNumberGroup(),
  )

  // Per-file selected group ids (bind to HeaderFooterRuleFields v-model)
  const selectedHeaderGroupId = computed({
    get: () => globalApplyEnabled.value
      ? globalHeaderGroup.value.id
      : selectedOverlayFile.value?.selectedHeaderGroupId || 'h1',
    set: (v) => {
      if (!globalApplyEnabled.value) setSelectedGroup(selectedOverlayFile.value, 'header', v)
    },
  })
  const selectedFooterTextGroupId = computed({
    get: () => globalApplyEnabled.value
      ? globalFooterTextGroup.value.id
      : selectedOverlayFile.value?.selectedFooterTextGroupId || 'ft1',
    set: (v) => {
      if (!globalApplyEnabled.value) setSelectedGroup(selectedOverlayFile.value, 'footerText', v)
    },
  })
  const selectedPageNumberGroupId = computed({
    get: () => globalApplyEnabled.value
      ? globalPageNumberGroup.value.id
      : selectedOverlayFile.value?.selectedPageNumberGroupId || 'pn1',
    set: (v) => {
      if (!globalApplyEnabled.value) setSelectedGroup(selectedOverlayFile.value, 'pageNumber', v)
    },
  })

  // --- Legacy field proxies (individual property accessors) ---
  const headerMode = computed({
    get: () => selectedHeaderGroup.value.mode,
    set: (v) => { selectedHeaderGroup.value.mode = v },
  })
  const headerText = computed({
    get: () => selectedHeaderGroup.value.text,
    set: (v) => { selectedHeaderGroup.value.text = v },
  })
  const headerPrefix = computed({
    get: () => selectedHeaderGroup.value.prefix,
    set: (v) => { selectedHeaderGroup.value.prefix = v },
  })
  const headerSuffix = computed({
    get: () => selectedHeaderGroup.value.suffix,
    set: (v) => { selectedHeaderGroup.value.suffix = v },
  })
  const headerAlign = computed({
    get: () => selectedHeaderGroup.value.align,
    set: (v) => { selectedHeaderGroup.value.align = v },
  })
  const headerFontSize = computed({
    get: () => selectedHeaderGroup.value.fontSize,
    set: (v) => { selectedHeaderGroup.value.fontSize = v },
  })
  const headerFontFamily = computed({
    get: () => selectedHeaderGroup.value.fontFamily,
    set: (v) => { selectedHeaderGroup.value.fontFamily = v },
  })
  const headerMarginMm = computed({
    get: () => selectedHeaderGroup.value.marginMm,
    set: (v) => { selectedHeaderGroup.value.marginMm = v },
  })
  const headerOffsetXMm = computed({
    get: () => selectedHeaderGroup.value.offsetXMm,
    set: (v) => { selectedHeaderGroup.value.offsetXMm = v },
  })
  const headerColor = computed({
    get: () => selectedHeaderGroup.value.color,
    set: (v) => { selectedHeaderGroup.value.color = v },
  })

  const footerTextContent = computed({
    get: () => selectedFooterTextGroup.value.text,
    set: (v) => { selectedFooterTextGroup.value.text = v },
  })
  const footerTextAlign = computed({
    get: () => selectedFooterTextGroup.value.align,
    set: (v) => { selectedFooterTextGroup.value.align = v },
  })
  const footerTextFontSize = computed({
    get: () => selectedFooterTextGroup.value.fontSize,
    set: (v) => { selectedFooterTextGroup.value.fontSize = v },
  })
  const footerTextFontFamily = computed({
    get: () => selectedFooterTextGroup.value.fontFamily,
    set: (v) => { selectedFooterTextGroup.value.fontFamily = v },
  })
  const footerTextMarginMm = computed({
    get: () => selectedFooterTextGroup.value.marginMm,
    set: (v) => { selectedFooterTextGroup.value.marginMm = v },
  })
  const footerTextOffsetXMm = computed({
    get: () => selectedFooterTextGroup.value.offsetXMm,
    set: (v) => { selectedFooterTextGroup.value.offsetXMm = v },
  })
  const footerTextColor = computed({
    get: () => selectedFooterTextGroup.value.color,
    set: (v) => { selectedFooterTextGroup.value.color = v },
  })

  const pageNumberSequence = computed({
    get: () => selectedPageNumberGroup.value.sequence,
    set: (v) => { selectedPageNumberGroup.value.sequence = v },
  })
  const pageNumberStyle = computed({
    get: () => selectedPageNumberGroup.value.style,
    set: (v) => { selectedPageNumberGroup.value.style = v },
  })
  const pageNumberRegion = computed({
    get: () => selectedPageNumberGroup.value.region,
    set: (v) => { selectedPageNumberGroup.value.region = v },
  })

  // footerAlign..footerColor are page number placement, backed by selectedPageNumberGroup
  const footerAlign = computed({
    get: () => selectedPageNumberGroup.value.align,
    set: (v) => { selectedPageNumberGroup.value.align = v },
  })
  const footerFontSize = computed({
    get: () => selectedPageNumberGroup.value.fontSize,
    set: (v) => { selectedPageNumberGroup.value.fontSize = v },
  })
  const footerFontFamily = computed({
    get: () => selectedPageNumberGroup.value.fontFamily,
    set: (v) => { selectedPageNumberGroup.value.fontFamily = v },
  })
  const footerMarginMm = computed({
    get: () => selectedPageNumberGroup.value.marginMm,
    set: (v) => { selectedPageNumberGroup.value.marginMm = v },
  })
  const footerOffsetXMm = computed({
    get: () => selectedPageNumberGroup.value.offsetXMm,
    set: (v) => { selectedPageNumberGroup.value.offsetXMm = v },
  })
  const footerColor = computed({
    get: () => selectedPageNumberGroup.value.color,
    set: (v) => { selectedPageNumberGroup.value.color = v },
  })

  // --- Auto-cleanup computeds ---
  const autoCleanupHeaderEnabled = computed(() =>
    overlayFiles.value.some(
      (file) => file.existingHeaderArtifact && file.existingHeaderEdited && !file.removeExistingHeader,
    ),
  )
  const autoCleanupFooterEnabled = computed(() =>
    overlayFiles.value.some(
      (file) => file.existingFooterArtifact && file.existingFooterEdited && !file.removeExistingFooter,
    ),
  )

  // --- Processing notes ---
  const processingNotes = computed(() => {
    const notes = []
    if (normalizeA4.value) {
      notes.push('A4 规范化会把小页面居中补白到 A4，超过 A4 的页面才等比缩小；会尽量保留原 PDF 内容层')
    }
    if (removeAnnotations.value) {
      notes.push('删除批注只处理评论、高亮等批注对象，已扁平化到正文的标记不会被对象删除')
    }
    if (outputMode.value === 'merge_only') {
      notes.push('只生成合并 PDF 时，中间单文件副本会在合并成功后清理')
    }
    if (hasExistingRemovalRule.value) {
      notes.push('删除现有页眉页脚不会使用白色遮盖；只能删除标准结构或已确认匹配的普通文本')
    }
    if (hasExistingEditRule.value) {
      notes.push('原页眉、原页脚、原页码列中的标准结构编辑会尽量原位处理；普通文本型旧内容会先删除匹配文本再按原位置重建')
    }
    if (hasExistingConvertRule.value) {
      notes.push('普通文本型旧页眉页脚页码会先删除匹配文本，再按检测到的位置重建')
    }
    return notes
  })

  // --- Current rules aggregate ---
  const currentRules = computed(() => ({
    normalizeA4: normalizeA4.value,
    a4Orientation: a4Orientation.value,
    rasterDpi: rasterDpi.value,
    removeAnnotations: removeAnnotations.value,
    bookmarkEnabled: bookmarkEnabled.value,
    bookmarkRemoveExisting: bookmarkRemoveExisting.value,
    bookmarkLabelSource: bookmarkLabelSource.value,
    annotationKinds: annotationKinds.value,
    cleanupHeaderEnabled: autoCleanupHeaderEnabled.value,
    cleanupFooterEnabled: autoCleanupFooterEnabled.value,
    cleanupHeaderHeightMm: cleanupHeaderHeightMm.value,
    cleanupFooterHeightMm: cleanupFooterHeightMm.value,
    headerMode: insertHeaderFooterEnabled.value ? headerMode.value : 'none',
    headerInsertEnabled: insertHeaderFooterEnabled.value && headerInsertEnabled.value,
    headerText: headerText.value,
    headerPrefix: headerPrefix.value,
    headerSuffix: headerSuffix.value,
    headerDateValue: '', // set externally
    headerAlign: headerAlign.value,
    headerFontSize: headerFontSize.value,
    headerFontFamily: headerFontFamily.value,
    headerMarginMm: headerMarginMm.value,
    headerOffsetXMm: headerOffsetXMm.value,
    headerColor: headerColor.value,
    headerGroups: insertHeaderFooterEnabled.value ? headerGroups.value : [],
    footerTextGroups: insertHeaderFooterEnabled.value && footerInsertEnabled.value ? footerTextGroups.value : [],
    pageNumberGroups: insertHeaderFooterEnabled.value && footerEnabled.value ? pageNumberGroups.value : [],
    _globalApply: globalApplyEnabled.value,
    _globalHeaderGroup: globalApplyEnabled.value ? globalHeaderGroup.value : null,
    _globalFooterTextGroup: globalApplyEnabled.value ? globalFooterTextGroup.value : null,
    _globalPageNumberGroup: globalApplyEnabled.value ? globalPageNumberGroup.value : null,
    footerEnabled: insertHeaderFooterEnabled.value && footerEnabled.value,
    footerText: footerText.value,
    footerContinuous: footerContinuous.value,
    footerAlign: footerAlign.value,
    footerFontSize: footerFontSize.value,
    footerFontFamily: footerFontFamily.value,
    footerMarginMm: footerMarginMm.value,
    footerOffsetXMm: footerOffsetXMm.value,
    footerColor: footerColor.value,
    footerInsertEnabled: insertHeaderFooterEnabled.value && footerInsertEnabled.value,
    footerTextContent: footerTextContent.value,
    footerTextAlign: footerTextAlign.value,
    footerTextFontSize: footerTextFontSize.value,
    footerTextFontFamily: footerTextFontFamily.value,
    footerTextMarginMm: footerTextMarginMm.value,
    footerTextOffsetXMm: footerTextOffsetXMm.value,
    footerTextColor: footerTextColor.value,
    pageNumberEnabled: insertHeaderFooterEnabled.value && footerEnabled.value,
    pageNumberSequence: pageNumberSequence.value,
    pageNumberStyle: pageNumberStyle.value,
    pageNumberTemplate: pageNumberTemplate.value,
    pageNumberRegion: pageNumberRegion.value,
    pageNumberAlign: footerAlign.value,
    pageNumberFontSize: footerFontSize.value,
    pageNumberFontFamily: footerFontFamily.value,
    pageNumberMarginMm: footerMarginMm.value,
    pageNumberOffsetXMm: footerOffsetXMm.value,
    pageNumberColor: footerColor.value,
    pageNumberOverrides: pageNumberOverrides.value,
    pageNumberShowTotal: pageNumberShowTotal.value,
    selectedHeaderGroupId: selectedHeaderGroupId.value,
    selectedFooterTextGroupId: selectedFooterTextGroupId.value,
    selectedPageNumberGroupId: selectedPageNumberGroupId.value,
    outputMode: outputMode.value,
    mergeAfterProcessing: outputMode.value !== 'files_only',
    mergeFileName: mergeFileName.value,
    fileSuffixEnabled: fileSuffixEnabled.value,
    fileSuffixText: fileSuffixText.value,
  }))

  // --- Processing rule applicability ---
  const hasApplicableProcessingRule = computed(
    () =>
      normalizeA4.value ||
      removeAnnotations.value ||
      bookmarkEnabled.value ||
      bookmarkRemoveExisting.value ||
      hasExistingEditRule.value ||
      hasExistingConvertRule.value ||
      hasExistingRemovalRule.value ||
      (insertHeaderFooterEnabled.value &&
        ((headerInsertEnabled.value && headerMode.value !== 'none') || footerInsertEnabled.value || footerEnabled.value)),
  )

  // --- Helper functions ---
  // Parameter following: copy style params from source file's group to target,
  // only if target still has default values (never been explicitly set).
  const STYLE_KEYS = ['align', 'fontSize', 'fontFamily', 'marginMm', 'offsetXMm', 'color']
  const DEFAULTS = {
    header: { align: 'center', fontSize: 12, fontFamily: 'auto', marginMm: 15, offsetXMm: 0, color: '#000000' },
    footerText: { align: 'left', fontSize: 9, fontFamily: 'auto', marginMm: 10, offsetXMm: 0, color: '#000000' },
    pageNumber: { align: 'center', fontSize: 9, fontFamily: 'auto', marginMm: 10, offsetXMm: 0, color: '#000000' },
  }

  function syncGroupParamsFromSource(source, target) {
    for (const kind of ['header', 'footerText', 'pageNumber']) {
      const srcGroup = selectedGroupFor(source, kind)
      const tgtGroup = selectedGroupFor(target, kind)
      if (!srcGroup || !tgtGroup) continue
      const defaults = DEFAULTS[kind]
      const isDefault = STYLE_KEYS.every(k => tgtGroup[k] === defaults[k] || tgtGroup[k] == null)
      if (!isDefault) continue
      for (const k of STYLE_KEYS) {
        if (srcGroup[k] != null) tgtGroup[k] = srcGroup[k]
      }
    }
    overlayFiles.value = [...overlayFiles.value]
  }

  function applyWorkflowDefaults() {
    if (workflowMode.value === 'split') {
      insertHeaderFooterEnabled.value = false
      headerMode.value = 'none'
      footerEnabled.value = false
      footerContinuous.value = true
      outputMode.value = 'files_only'
      mergeFileName.value = 'split_evidence.pdf'
      return
    }
    insertHeaderFooterEnabled.value = true
    headerMode.value = 'filename'
    footerEnabled.value = false
    footerContinuous.value = true
    outputMode.value = 'files_and_merge'
    mergeFileName.value = 'merged_evidence.pdf'
  }

  function hasReplacementRule() {
    return (
      (insertHeaderFooterEnabled.value &&
        (headerMode.value !== 'none' || footerInsertEnabled.value || footerEnabled.value)) ||
      hasExistingEditRule.value ||
      hasExistingConvertRule.value ||
      hasExistingRemovalRule.value
    )
  }

  function applyReplacementPreset(refreshPreview) {
    insertHeaderFooterEnabled.value = true
    headerInsertEnabled.value = true
    footerInsertEnabled.value = true
    pageNumberShowTotal.value = true
    normalizeA4.value = false
    removeAnnotations.value = false
    cleanupHeaderHeightMm.value = 18
    cleanupFooterHeightMm.value = 18
    const file = selectedOverlayFile.value
    if (file) {
      const headerGroup = createDefaultHeaderGroup()
      headerGroup.mode = workflowMode.value === 'split' ? 'per_file' : 'filename'
      file.headerGroups = [headerGroup]
      file.footerTextGroups = [createDefaultFooterTextGroup()]
      file.pageNumberGroups = [createDefaultPageNumberGroup()]
      file.selectedHeaderGroupId = headerGroup.id
      file.selectedFooterTextGroupId = 'ft1'
      file.selectedPageNumberGroupId = 'pn1'
    }
    footerEnabled.value = true
    footerContinuous.value = true
    footerText.value = '{page}/{total}'
    outputMode.value = 'files_and_merge'
    if (refreshPreview) refreshPreview()
  }

  function ensureReplacementPreset(refreshPreview) {
    if (!hasReplacementRule()) {
      applyReplacementPreset(refreshPreview)
    }
  }

  // --- Watches ---
  watch(
    () => selectedHeaderGroup.value.mode,
    (mode) => {
      const group = selectedHeaderGroup.value
      if (mode === 'template') {
        group.mode = 'custom'
      } else if (mode === 'seq') {
        group.text = '证据[序号]'
        group.mode = 'custom'
      } else if (mode === 'seq_cn') {
        group.text = '证据[中文序号]'
        group.mode = 'custom'
      } else if (mode === 'prefix_seq') {
        group.text = `${group.text || ''}证据[序号]`
        group.mode = 'custom'
      }
    },
    { immediate: true },
  )

  watch(pageNumberSequence, (value) => {
    footerContinuous.value = value !== 'per-file'
  })

  watch(outputMode, (mode) => {
    if (mode === 'files_only') bookmarkEnabled.value = false
  })

  return {
    // Constants
    DEFAULT_RASTER_DPI,
    HORIZONTAL_OFFSET_LIMIT_MM,
    STYLE_KEYS,
    DEFAULTS,
    // Core state
    cleanupHeaderHeightMm,
    cleanupFooterHeightMm,
    insertHeaderFooterEnabled,
    globalApplyEnabled,
    globalHeaderGroup,
    globalFooterTextGroup,
    globalPageNumberGroup,
    headerInsertEnabled,
    footerInsertEnabled,
    pageNumberShowTotal,
    normalizeA4,
    a4Orientation,
    rasterDpi,
    removeAnnotations,
    bookmarkEnabled,
    bookmarkRemoveExisting,
    bookmarkLabelSource,
    annotationKinds,
    footerEnabled,
    footerText,
    footerContinuous,
    pageNumberOverrides,
    pageNumberRulesVisible,
    outputMode,
    mergeFileName,
    fileSuffixEnabled,
    fileSuffixText,
    // Group computeds
    headerGroups,
    footerTextGroups,
    pageNumberGroups,
    headerGroupsModel,
    footerTextGroupsModel,
    pageNumberGroupsModel,
    pageNumberTemplate,
    selectedHeaderGroup,
    selectedFooterTextGroup,
    selectedPageNumberGroup,
    selectedHeaderGroupId,
    selectedFooterTextGroupId,
    selectedPageNumberGroupId,
    // Legacy field proxies
    headerMode,
    headerText,
    headerPrefix,
    headerSuffix,
    headerAlign,
    headerFontSize,
    headerFontFamily,
    headerMarginMm,
    headerOffsetXMm,
    headerColor,
    footerTextContent,
    footerTextAlign,
    footerTextFontSize,
    footerTextFontFamily,
    footerTextMarginMm,
    footerTextOffsetXMm,
    footerTextColor,
    pageNumberSequence,
    pageNumberStyle,
    pageNumberRegion,
    footerAlign,
    footerFontSize,
    footerFontFamily,
    footerMarginMm,
    footerOffsetXMm,
    footerColor,
    // Computed aggregates
    autoCleanupHeaderEnabled,
    autoCleanupFooterEnabled,
    processingNotes,
    currentRules,
    hasApplicableProcessingRule,
    // Functions
    syncGroupParamsFromSource,
    applyWorkflowDefaults,
    hasReplacementRule,
    applyReplacementPreset,
    ensureReplacementPreset,
  }
}
