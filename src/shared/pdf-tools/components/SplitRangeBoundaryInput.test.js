import { describe, expect, it, vi } from 'vitest'
import { reactive } from 'vue'
vi.mock('vue', async (original) => ({ ...(await original()), useSSRContext: () => ({ modules: new Set() }) }))
import Boundary from './SplitRangeBoundaryInput.vue'

describe('endpoint draft input', () => {
  it('keeps typing separate from the actual range and commits once on submission', () => {
    const props = reactive({ value: 27, max: 100, label: '结束页' }),
      emit = vi.fn()
    const state = Boundary.setup(props, { emit, expose: vi.fn() })
    state.draft.value = 5
    expect(props.value).toBe(27)
    expect(emit).not.toHaveBeenCalled()
    state.commit()
    expect(emit).toHaveBeenCalledWith('commit', 5)
    state.commit()
    expect(emit).toHaveBeenCalledTimes(1)
    props.value = 5
    state.commit()
    expect(emit).toHaveBeenCalledTimes(1)
  })
  it('restores an emptied input and clamps a submitted number to document bounds', () => {
    const props = reactive({ value: 27, max: 100, label: '结束页' }),
      emit = vi.fn()
    const state = Boundary.setup(props, { emit, expose: vi.fn() })
    state.draft.value = null
    state.commit()
    expect(state.draft.value).toBe('27')
    expect(emit).not.toHaveBeenCalled()
    state.commit(999)
    expect(emit).toHaveBeenCalledWith('commit', 100)
  })
  it('steps from the current draft without emitting an old-page navigation or recreating the input', () => {
    const props = reactive({ value: 10, max: 100, min: 1, label: '结束页' }),
      emit = vi.fn()
    const state = Boundary.setup(props, { emit, expose: vi.fn() })
    state.draft.value = '25'
    state.step(1)
    state.step(1)
    expect(state.draft.value).toBe('27')
    expect(emit.mock.calls).toEqual([
      ['commit', 26],
      ['commit', 27],
    ])
    state.commit()
    expect(emit).toHaveBeenCalledTimes(2)
  })
})

it('leaves unset insertion boundaries empty until the user enters a page', () => {
  const props = reactive({ value: null, max: 30, min: 1, label: '起始页' }),
    emit = vi.fn()
  const state = Boundary.setup(props, { emit, expose: vi.fn() })
  expect(state.draft.value).toBe('')
  state.commit()
  expect(emit).not.toHaveBeenCalled()
  state.draft.value = '10'
  state.commit()
  expect(emit).toHaveBeenCalledWith('commit', 10)
})
