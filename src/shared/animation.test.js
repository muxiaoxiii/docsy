import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createCelebrationTrigger, withMinimumDuration } from './animation'

describe('animation utilities', () => {
  it('ensures minimum duration for fast async tasks', async () => {
    const start = Date.now()
    const result = await withMinimumDuration(async () => {
      return 'done'
    }, 150)
    const elapsed = Date.now() - start

    expect(result).toBe('done')
    expect(elapsed).toBeGreaterThanOrEqual(140)
  })

  it('does not delay tasks that take longer than minMs', async () => {
    const start = Date.now()
    const result = await withMinimumDuration(async () => {
      await new Promise((r) => setTimeout(r, 120))
      return 'slow done'
    }, 60)
    const elapsed = Date.now() - start

    expect(result).toBe('slow done')
    expect(elapsed).toBeGreaterThanOrEqual(110)
  })

  it('triggers celebration and resets after duration', () => {
    vi.useFakeTimers()
    const celebrating = ref(false)
    const trigger = createCelebrationTrigger(celebrating, 1000)

    expect(celebrating.value).toBe(false)
    trigger()
    expect(celebrating.value).toBe(true)

    vi.advanceTimersByTime(999)
    expect(celebrating.value).toBe(true)

    vi.advanceTimersByTime(2)
    expect(celebrating.value).toBe(false)
    vi.useRealTimers()
  })
})
