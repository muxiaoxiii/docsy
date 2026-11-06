import { describe, expect, it } from 'vitest'
import { annotationRemovalSummary, formatProcessingWarningSummary } from './processingWarnings.js'

describe('annotationRemovalSummary', () => {
  it('distinguishes disabled cleanup from a completed cleanup that found no objects', () => {
    expect(annotationRemovalSummary([{ warnings: [] }])).toBe('')
    expect(annotationRemovalSummary([{ annotationsRemoved: 0 }])).toContain('未发现可删除')
  })
  it('shows actual counts and files without targets without treating them as failures', () => {
    expect(annotationRemovalSummary([
      { annotationsRemoved: 6, annotationPagesTouched: 2 },
      { annotationsRemoved: 0, annotationPagesTouched: 0 },
      { annotationsRemoved: 3, annotationPagesTouched: 1 },
    ])).toBe('已删除批注及关联对象 9 个，涉及 3 页；1 个文件未发现可删除的批注对象')
  })
})

describe('formatProcessingWarningSummary', () => {
  it('groups per-page overlay collisions by text instead of listing every page', () => {
    const lines = formatProcessingWarningSummary([
      {
        inputPath: '/case/证据13.pdf',
        warnings: [
          '第 1 页的“— {page} —”与其他页眉页脚位置重叠，将按实际位置叠加渲染',
          '第 2 页的“— {page} —”与其他页眉页脚位置重叠，将按实际位置叠加渲染',
        ],
      },
      {
        inputPath: '/case/证据14.pdf',
        warnings: ['第 1 页的“— {page} —”与其他页眉页脚位置重叠，将按实际位置叠加渲染'],
      },
    ])

    expect(lines).toEqual(['位置重叠：2 个文件、3 页的“— {page} —”。已按原位置写入。'])
  })

  it('groups matching non-collision warnings by type', () => {
    const lines = formatProcessingWarningSummary([
      { inputPath: '/case/a.pdf', warnings: ['未找到可安全删除的匹配内容'] },
      { inputPath: '/case/b.pdf', warnings: ['未找到可安全删除的匹配内容'] },
    ])
    expect(lines).toEqual(['2 个文件：未找到可安全删除的匹配内容'])
  })
})
