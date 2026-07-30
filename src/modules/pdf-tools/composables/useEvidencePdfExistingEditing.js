import { ElMessage } from 'element-plus'
import { stripPdf } from '../../../core/filePath.js'
import { buildHeaderText, expandPlaceholders } from './useEvidencePdfSession.js'

export function useEvidencePdfExistingEditing({
  _selectedOverlayFile,
  editingHeaderPath,
  editingFooterPath,
  editingExistingHeaderPath,
  editingExistingFooterPath,
  editingExistingPageNumberPath,
  insertHeaderFooterEnabled,
  headerMode,
  footerEnabled,
  footerContinuous,
  totalOverlayPages,
  currentRules,
  workflowMode,
  footerText,
  hasExistingHeader,
  hasExistingFooter,
  hasExistingPageNumber,
  fileExistingStatus,
  refreshPreview,
}) {
  function isEditingHeader(row) {
    return editingHeaderPath.value && editingHeaderPath.value === row.path
  }

  async function startHeaderEdit(row, index) {
    if (!row) return
    insertHeaderFooterEnabled.value = true
    headerMode.value = 'per_file'
    if (row.header === null || row.header === undefined) {
      row.header = rowHeaderPreview(row, index) || stripPdf(row.name)
    }
    editingHeaderPath.value = row.path
  }

  function finishHeaderEdit(row) {
    if (row) {
      row.header = String(row.header ?? '').trim()
      row.headerEdited = true
    }
    editingHeaderPath.value = ''
  }

  function isEditingFooter(row) {
    return editingFooterPath.value && editingFooterPath.value === row.path
  }

  async function startFooterEdit(row, index) {
    if (!row) return
    insertHeaderFooterEnabled.value = true
    footerEnabled.value = true
    if (row.footer === null || row.footer === undefined) {
      row.footer = rowFooterPreview(row, index)
    }
    editingFooterPath.value = row.path
  }

  function finishFooterEdit(row) {
    if (row) {
      row.footer = String(row.footer ?? '').trim()
      row.footerEdited = true
    }
    editingFooterPath.value = ''
  }

  function isEditingExistingHeader(row) {
    return editingExistingHeaderPath.value && editingExistingHeaderPath.value === row.path
  }

  function startExistingHeaderEdit(row) {
    if (!hasExistingHeader(row)) return
    if (row.removeExistingHeader) {
      row.removeExistingHeader = false
    }
    if (!row.existingHeaderText) {
      row.existingHeaderText = row.existingHeaderTargetText || ''
    }
    editingExistingHeaderPath.value = row.path
  }

  function finishExistingHeaderEdit(row) {
    if (!row) {
      editingExistingHeaderPath.value = ''
      return
    }
    const next = String(row.existingHeaderText ?? '').trim()
    const original = row.existingHeaderTargetText || row.existingHeaderText || ''
    if (!next) {
      row.existingHeaderText = original
      row.existingHeaderEdited = false
      row.convertPlainHeader = false
      row.removeExistingHeader = false
    } else if (row.existingHeaderArtifact) {
      row.existingHeaderText = next
      row.existingHeaderEdited = next !== original
      row.removeExistingHeader = false
    } else if (next !== original) {
      row.existingHeaderText = next
      row.convertPlainHeader = true
      row.removeExistingHeader = false
      ElMessage.info('普通文本型旧页眉会删除匹配旧文本，并在检测到的位置重建')
    }
    const status = fileExistingStatus(row)
    row.statusText = status.text
    row.statusType = status.type
    editingExistingHeaderPath.value = ''
    refreshPreview()
  }

  function isEditingExistingFooter(row) {
    return editingExistingFooterPath.value && editingExistingFooterPath.value === row.path
  }

  function startExistingFooterEdit(row) {
    if (!hasExistingFooter(row)) return
    if (row.removeExistingFooter) {
      row.removeExistingFooter = false
    }
    if (!row.existingFooterText) {
      row.existingFooterText = row.existingFooterTargetText || ''
    }
    editingExistingFooterPath.value = row.path
  }

  function finishExistingFooterEdit(row) {
    if (!row) {
      editingExistingFooterPath.value = ''
      return
    }
    const next = String(row.existingFooterText ?? '').trim()
    const original = row.existingFooterTargetText || row.existingFooterText || ''
    if (!next) {
      row.existingFooterText = original
      row.existingFooterEdited = false
      row.convertPlainFooter = false
      row.removeExistingFooter = false
    } else if (row.existingFooterArtifact) {
      row.existingFooterText = next
      row.existingFooterEdited = next !== original
      row.removeExistingFooter = false
    } else if (next !== original) {
      row.existingFooterText = next
      row.convertPlainFooter = true
      row.removeExistingFooter = false
      ElMessage.info('普通文本型旧页脚会删除匹配旧文本，并在检测到的位置重建')
    }
    const status = fileExistingStatus(row)
    row.statusText = status.text
    row.statusType = status.type
    editingExistingFooterPath.value = ''
    refreshPreview()
  }

  function isEditingExistingPageNumber(row) {
    return editingExistingPageNumberPath.value && editingExistingPageNumberPath.value === row.path
  }

  function startExistingPageNumberEdit(row) {
    if (!hasExistingPageNumber(row)) return
    if (row.removeExistingPageNumber) {
      row.removeExistingPageNumber = false
    }
    if (!row.existingPageNumberText) {
      row.existingPageNumberText = row.existingPageNumberTargetText || ''
    }
    editingExistingPageNumberPath.value = row.path
  }

  function finishExistingPageNumberEdit(row) {
    if (!row) {
      editingExistingPageNumberPath.value = ''
      return
    }
    const next = String(row.existingPageNumberText ?? '').trim()
    const original = row.existingPageNumberTargetText || row.existingPageNumberText || ''
    if (!next) {
      row.existingPageNumberText = original
      row.existingPageNumberEdited = false
      row.convertPlainPageNumber = false
      row.removeExistingPageNumber = false
    } else if (row.existingFooterArtifact && !row.existingFooterText) {
      row.existingFooterText = next
      row.existingFooterTargetText = original
      row.existingFooterEdited = next !== original
      row.existingPageNumberText = next
      row.existingPageNumberEdited = false
      row.convertPlainPageNumber = false
      row.removeExistingFooter = false
      row.removeExistingPageNumber = false
    } else if (next !== original) {
      row.existingPageNumberText = next
      row.existingPageNumberEdited = true
      row.convertPlainPageNumber = true
      row.removeExistingPageNumber = false
      ElMessage.info('旧页码会删除匹配旧文本，并在检测到的位置重建')
    } else {
      row.existingPageNumberEdited = false
      row.convertPlainPageNumber = false
      row.removeExistingPageNumber = false
    }
    const status = fileExistingStatus(row)
    row.statusText = status.text
    row.statusType = status.type
    editingExistingPageNumberPath.value = ''
    refreshPreview()
  }

  function rowHeaderPreview(row, index) {
    return buildHeaderText(row, index, currentRules.value)
  }

  function displayRowHeader(row, index) {
    if (!insertHeaderFooterEnabled.value) return ''
    if (workflowMode.value === 'split' && headerMode.value === 'per_file') {
      return row?.header ?? rowHeaderPreview(row, index)
    }
    return rowHeaderPreview(row, index)
  }

  function rowFooterPreview(row) {
    const footerTemplate = row?.footer ?? footerText.value
    if (!insertHeaderFooterEnabled.value || !footerEnabled.value || !footerTemplate || !row) return ''
    const page = footerContinuous.value ? row.pageStart || 1 : 1
    const total = footerContinuous.value ? totalOverlayPages.value || row.pages || 1 : row.pages || 1
    return expandPlaceholders(footerTemplate, page, total)
  }

  function displayRowFooter(row, index) {
    if (row?.footer !== null && row?.footer !== undefined) return row.footer
    return rowFooterPreview(row, index)
  }

  function displayExistingHeader(row) {
    return row?.existingHeaderText || row?.existingHeaderTargetText || ''
  }

  function displayExistingFooter(row) {
    return row?.existingFooterText || row?.existingFooterTargetText || ''
  }

  function displayExistingPageNumber(row) {
    return row?.existingPageNumberText || row?.existingPageNumberTargetText || ''
  }

  return {
    isEditingHeader,
    startHeaderEdit,
    finishHeaderEdit,
    isEditingFooter,
    startFooterEdit,
    finishFooterEdit,
    isEditingExistingHeader,
    startExistingHeaderEdit,
    finishExistingHeaderEdit,
    isEditingExistingFooter,
    startExistingFooterEdit,
    finishExistingFooterEdit,
    isEditingExistingPageNumber,
    startExistingPageNumberEdit,
    finishExistingPageNumberEdit,
    rowHeaderPreview,
    displayRowHeader,
    rowFooterPreview,
    displayRowFooter,
    displayExistingHeader,
    displayExistingFooter,
    displayExistingPageNumber,
  }
}
