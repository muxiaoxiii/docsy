import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { headerFooterDetectionZoneMm, isFirstPageEvidenceCandidate, useEvidencePdfDetection } from './useEvidencePdfDetection.js'
import { createEvidenceFile } from './useEvidencePdfSession.js'
import { tauriCallQuiet } from '../../../core/tauriBridge.js'

vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallQuiet: vi.fn(), emitOperationEvent: vi.fn(), emitOperationUpdate: vi.fn(),
}))
vi.mock('../../../shared/diagnostics.js', () => ({
  log: { info: vi.fn(), warn: vi.fn() }, registerSnapshotProvider: vi.fn(),
}))

describe('batch detection isolation', () => {
  it('limits concurrent requests and maps out-of-order successes and failures to their source files', async () => {
    const files = Array.from({ length: 6 }, (_, index) => ({
      ...createEvidenceFile(`/case-${index}/report.pdf`), pages: 10,
    }))
    const pending = new Map()
    let active = 0
    let peak = 0
    tauriCallQuiet.mockImplementation((_command, { args }) => new Promise(resolve => {
      active += 1
      peak = Math.max(peak, active)
      pending.set(args.inputPath, result => { active -= 1; resolve(result) })
    }))
    const detecting = ref(false)
    const progress = ref('')
    const { detectAllHeaderFooter } = useEvidencePdfDetection({
      overlayRows: ref(files), detectingAllHeaderFooter: detecting, detectionProgressText: progress,
      cleanupHeaderHeightMm: ref(25), cleanupFooterHeightMm: ref(25),
    })
    const run = detectAllHeaderFooter({ silent: true })
    expect(pending.size).toBe(4)
    await detectAllHeaderFooter({ silent: true })
    expect(pending.size).toBe(4)
    const success = index => ({ ok: true, data: {
      pagesAnalyzed: 10,
      headerCandidates: [{
        text: `证据${index + 1}`, normalizedText: `证据${index + 1}`, region: 'header',
        source: 'artifact', confidence: 1, count: 10, pageRange: { start: 1, end: 10 },
      }],
    } })
    pending.get(files[3].path)(success(3))
    pending.get(files[1].path)({ ok: false, error: '文件损坏' })
    await vi.waitFor(() => expect(pending.size).toBe(6))
    for (const index of [5, 0, 4, 2]) pending.get(files[index].path)(success(index))
    await run
    expect(peak).toBe(4)
    expect(files[1].statusText).toBe('检测失败')
    expect(files[1].statusDetail).toBe('文件损坏')
    for (const index of [0, 2, 3, 4, 5]) {
      expect(files[index].existingElements.map(element => element.detectedText)).toEqual([`证据${index + 1}`])
    }
    expect(detecting.value).toBe(false)
    expect(progress.value).toBe('')
  })
})

describe('headerFooterDetectionZoneMm', () => {
  it('keeps detection wider than the cleanup band', () => {
    expect(headerFooterDetectionZoneMm(18)).toBe(25)
    expect(headerFooterDetectionZoneMm(32)).toBe(32)
  })

  it('bounds invalid and excessive values', () => {
    expect(headerFooterDetectionZoneMm(0)).toBe(25)
    expect(headerFooterDetectionZoneMm(100)).toBe(60)
  })
})

describe('isFirstPageEvidenceCandidate', () => {
  it('accepts backend-tagged evidence labels on page 1', () => {
    expect(
      isFirstPageEvidenceCandidate({
        labels: ['evidence-label'],
        text: '证据１',
        normalizedText: '证据1',
        pageRange: { start: 1, end: 1 },
      }),
    ).toBe(true)
  })

  it('matches 证据/对比文件 patterns without a backend label', () => {
    expect(
      isFirstPageEvidenceCandidate({ text: '证据２', normalizedText: '证据2', pageRange: { start: 1, end: 1 } }),
    ).toBe(true)
    expect(isFirstPageEvidenceCandidate({ text: '对比文件3', pageRange: { start: 1, end: 1 } })).toBe(true)
    expect(isFirstPageEvidenceCandidate({ text: '证据一', pageRange: { start: 1, end: 1 } })).toBe(true)
  })

  it('rejects labels not starting on page 1', () => {
    expect(
      isFirstPageEvidenceCandidate({
        labels: ['evidence-label'],
        text: '证据2',
        pageRange: { start: 2, end: 2 },
      }),
    ).toBe(false)
  })

  it('rejects non-label text', () => {
    expect(isFirstPageEvidenceCandidate({ text: '证据目录', pageRange: { start: 1, end: 1 } })).toBe(false)
    expect(isFirstPageEvidenceCandidate({ text: '附 页', pageRange: { start: 1, end: 1 } })).toBe(false)
    expect(isFirstPageEvidenceCandidate(null)).toBe(false)
  })
})
