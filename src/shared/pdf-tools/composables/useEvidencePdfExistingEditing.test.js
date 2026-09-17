import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useEvidencePdfExistingEditing } from './useEvidencePdfExistingEditing.js'

vi.mock('element-plus', () => ({
  ElMessage: {
    info: vi.fn(),
    warning: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  },
}))

describe('useEvidencePdfExistingEditing', () => {
  function setup() {
    const editingExistingPageNumberPath = ref('')
    const editingExistingFooterPath = ref('')
    const editingExistingHeaderPath = ref('')
    const refreshPreview = vi.fn()
    const fileExistingStatus = vi.fn().mockReturnValue({ text: '待处理', type: 'info' })

    const composable = useEvidencePdfExistingEditing({
      _selectedOverlayFile: ref(null),
      editingHeaderPath: ref(''),
      editingFooterPath: ref(''),
      editingExistingHeaderPath,
      editingExistingFooterPath,
      editingExistingPageNumberPath,
      insertHeaderFooterEnabled: ref(false),
      headerMode: ref('none'),
      footerEnabled: ref(false),
      footerContinuous: ref(false),
      totalOverlayPages: ref(0),
      currentRules: ref({}),
      _workflowMode: ref(''),
      footerText: ref(''),
      hasExistingHeader: () => true,
      hasExistingFooter: () => true,
      hasExistingPageNumber: () => true,
      fileExistingStatus,
      refreshPreview,
    })

    return { composable, editingExistingPageNumberPath, refreshPreview }
  }

  it('correctly updates page number properties without modifying footer fields', () => {
    const { composable } = setup()

    const row = {
      path: '/test/doc.pdf',
      existingFooterText: '原页脚内容',
      existingFooterTargetText: '原页脚内容',
      existingPageNumberText: '1/10',
      existingPageNumberTargetText: '1/10',
      existingPageNumberArtifact: true,
      existingFooterArtifact: true,
    }

    // 用户把页码编辑成 1/20
    row.existingPageNumberText = '1/20'
    composable.finishExistingPageNumberEdit(row)

    // 页码字段应被更新，且标记为 edited
    expect(row.existingPageNumberText).toBe('1/20')
    expect(row.existingPageNumberEdited).toBe(true)

    // 页脚字段绝对不能被篡改！
    expect(row.existingFooterText).toBe('原页脚内容')
    expect(row.existingFooterTargetText).toBe('原页脚内容')
    expect(row.existingFooterEdited).toBeUndefined()
  })

  it('handles plain text page number modifications with convertPlainPageNumber', () => {
    const { composable } = setup()

    const row = {
      path: '/test/doc2.pdf',
      existingPageNumberText: '第 1 页',
      existingPageNumberTargetText: '第 1 页',
      existingPageNumberArtifact: false, // 普通文本旧页码
    }

    row.existingPageNumberText = '第 2 页'
    composable.finishExistingPageNumberEdit(row)

    expect(row.existingPageNumberText).toBe('第 2 页')
    expect(row.existingPageNumberEdited).toBe(true)
    expect(row.convertPlainPageNumber).toBe(true)
  })
})
