import { describe, expect, it, vi } from 'vitest'
import { reactive } from 'vue'
vi.mock('vue', async original => ({ ...(await original()), useSSRContext: () => ({ modules: new Set() }) }))
import MergeFileNameField from './MergeFileNameField.vue'

describe('merge file naming', () => {
  it('offers original Word and image names rather than temporary converted PDFs', () => {
    const props = reactive({ modelValue: '', fallback: '合并文件.pdf', files: [
      { path: '/tmp/converted.pdf', sourcePath: '/材料/合同.docx' },
      { path: '/tmp/images.pdf', sourcePaths: ['/材料/照片.jpg', '/材料/凭证.png'] },
      '/材料/附件.pdf',
    ] })
    const emit = vi.fn()
    const state = MergeFileNameField.setup(props, { emit, expose: vi.fn() })
    expect(state.choices.value).toEqual(['/材料/合同.docx', '/材料/照片.jpg', '/材料/凭证.png', '/材料/附件.pdf'])
    state.choose('/材料/合同.docx')
    expect(emit).toHaveBeenLastCalledWith('update:modelValue', '合同.pdf')
    state.choose('/材料/照片.jpg')
    expect(emit).toHaveBeenLastCalledWith('update:modelValue', '照片.pdf')
    state.choose('/tmp/converted.pdf')
    expect(emit).toHaveBeenCalledTimes(2)
  })
  it('reports invalid paths inline and accepts a corrected filename', () => {
    const props = reactive({ modelValue: '../original.pdf', fallback: '合并文件.pdf', files: [] })
    const state = MergeFileNameField.setup(props, { emit: vi.fn(), expose: vi.fn() })
    expect(state.error.value).toContain('文件名不能包含路径')
    props.modelValue = '汇总材料'
    expect(state.error.value).toBe('')
  })
})
