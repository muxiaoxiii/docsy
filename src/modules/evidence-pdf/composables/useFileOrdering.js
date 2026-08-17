import { sortByNatural } from '../../../shared/pdf-tools/composables/useEvidencePdfSession.js'

/**
 * Manages file ordering operations for the evidence PDF workbench:
 * sorting, moving up/down, drag reordering, and removal.
 *
 * Extracted from EvidencePdfWorkbench.vue to reduce component size.
 */
export function useFileOrdering({
  overlayFiles,
  _overlayRows,
  selectedOverlayFile,
  selectedOverlayIndex,
  mergedImportPlan,
  selectedMergedImportIndex,
  truePreview,
  displayRowHeader,
  displayRowFooter,
  refreshPreview,
}) {
  function sortOverlayFiles({ prop, order }) {
    if (!prop || !order) return
    const selectedPath = selectedOverlayFile.value?.path
    overlayFiles.value = sortByNatural(overlayFiles.value, (row, index) => overlaySortValue(row, prop, index), order)
    if (selectedPath) {
      selectedOverlayIndex.value = Math.max(
        0,
        overlayFiles.value.findIndex((file) => file.path === selectedPath),
      )
    } else {
      selectedOverlayIndex.value = 0
    }
    refreshPreview()
  }

  function overlaySortValue(row, prop, index) {
    if (prop === 'header') return displayRowHeader(row, index)
    if (prop === 'footer') return displayRowFooter(row, index)
    if (prop === 'pages') return Number(row?.pages || 0)
    if (prop === 'pageRange') return Number(row?.pageStart || 0)
    if (prop === 'sourceRange') return Number(row?.sourcePageStart || 0)
    return row?.[prop] ?? ''
  }

  function moveOverlayFile(index, direction) {
    const target = index + direction
    if (target < 0 || target >= overlayFiles.value.length) return
    const items = [...overlayFiles.value]
    const [item] = items.splice(index, 1)
    items.splice(target, 0, item)
    overlayFiles.value = items
    selectedOverlayIndex.value = target
    refreshPreview()
  }

  function reorderOverlayFiles(from, to) {
    if (from === to || from < 0 || to < 0 || from >= overlayFiles.value.length || to >= overlayFiles.value.length)
      return
    const selectedPath = selectedOverlayFile.value?.path
    const items = [...overlayFiles.value]
    const [item] = items.splice(from, 1)
    items.splice(to, 0, item)
    overlayFiles.value = items
    selectedOverlayIndex.value = selectedPath
      ? Math.max(
          0,
          items.findIndex((file) => file.path === selectedPath),
        )
      : to
    truePreview.value = null
    refreshPreview()
  }

  function reorderMergedImportItems(from, to) {
    const items = mergedImportPlan.value?.items
    if (!items || from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) return
    const [item] = items.splice(from, 1)
    items.splice(to, 0, item)
    selectedMergedImportIndex.value = to
  }

  function removeOverlayFile(index) {
    overlayFiles.value.splice(index, 1)
    selectedOverlayIndex.value = Math.min(selectedOverlayIndex.value, Math.max(0, overlayFiles.value.length - 1))
    refreshPreview()
  }

  return {
    sortOverlayFiles,
    overlaySortValue,
    moveOverlayFile,
    reorderOverlayFiles,
    reorderMergedImportItems,
    removeOverlayFile,
  }
}
