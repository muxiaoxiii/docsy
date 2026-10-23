import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  dialogTransition,
  finishAllMotion,
  installUiMotion,
  motionSelection,
  motionNavigation,
  motionTabs,
  playMotion,
} from './uiMotion.js'

let reduced, hidden, documentListeners, preferenceListeners
const surface = () => {
  const element = {
    style: { opacity: '1', translate: '0px 0px', scale: '1' },
    getClientRects: () => [{}],
    animate: vi.fn(() => ({ cancel: vi.fn(), onfinish: null })),
  }
  return element
}
beforeEach(() => {
  vi.useFakeTimers()
  reduced = false
  hidden = false
  documentListeners = new Map()
  preferenceListeners = new Map()
  vi.stubGlobal('document', {
    get hidden() {
      return hidden
    },
    addEventListener: (event, fn) => documentListeners.set(event, fn),
    removeEventListener: (event) => documentListeners.delete(event),
    createElement: () => ({ ...surface(), setAttribute: vi.fn(), remove: vi.fn() }),
  })
  vi.stubGlobal('window', {
    matchMedia: () => ({
      get matches() {
        return reduced
      },
      addEventListener: (event, fn) => preferenceListeners.set(event, fn),
      removeEventListener: (event) => preferenceListeners.delete(event),
    }),
    getComputedStyle: (element) => element.style,
  })
})
afterEach(() => {
  finishAllMotion()
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('bounded interface motion', () => {
  it('interrupts from the rendered pose and discards the old completion callback', () => {
    const element = surface(),
      oldDone = vi.fn(),
      latestDone = vi.fn()
    playMotion(element, [{ opacity: 0 }, { opacity: 1 }], {}, oldDone)
    const first = element.animate.mock.results[0].value
    element.style.opacity = '.6'
    playMotion(element, [{ opacity: 1 }, { opacity: 0 }], { duration: 140 }, latestDone)
    expect(first.cancel).toHaveBeenCalledOnce()
    expect(element.animate.mock.calls[1][0][0]).toEqual({ opacity: '.6' })
    element.animate.mock.results[1].value.onfinish()
    vi.runAllTimers()
    expect(oldDone).not.toHaveBeenCalled()
    expect(latestDone).toHaveBeenCalledOnce()
    expect(vi.getTimerCount()).toBe(0)
  })
  it('finishes a dialog even when a webview never sends its animation finish event', () => {
    const element = surface(),
      done = vi.fn()
    dialogTransition.onLeave({ querySelector: () => element }, done)
    vi.advanceTimersByTime(241)
    expect(done).toHaveBeenCalledOnce()
    expect(element.animate.mock.results[0].value.cancel).toHaveBeenCalledOnce()
    expect(vi.getTimerCount()).toBe(0)
  })
  it('skips motion for hidden windows and reduced motion without blocking actions', () => {
    for (const reason of ['hidden', 'reduced']) {
      hidden = reason === 'hidden'
      reduced = reason === 'reduced'
      const element = surface(),
        done = vi.fn()
      dialogTransition.onEnter({ querySelector: () => element }, done)
      expect(element.animate).not.toHaveBeenCalled()
      expect(done).toHaveBeenCalledOnce()
    }
    expect(vi.getTimerCount()).toBe(0)
  })
  it('settles active motion when the system preference changes and removes global listeners', () => {
    const app = { directive: vi.fn(), onUnmount: vi.fn() }
    const dispose = installUiMotion(app),
      done = vi.fn()
    playMotion(surface(), [{ opacity: 0 }, { opacity: 1 }], {}, done)
    reduced = true
    preferenceListeners.get('change')()
    expect(done).toHaveBeenCalledOnce()
    expect(vi.getTimerCount()).toBe(0)
    dispose()
    expect(preferenceListeners.size).toBe(0)
    expect(documentListeners.size).toBe(0)
  })
  it('only animates the new tab, preserving panels and ignoring repeated selection', () => {
    const a = { ...surface(), hidden: false },
      b = { ...surface(), hidden: false }
    a.style.display = ''
    b.style.display = 'none'
    const root = { querySelector: () => null, querySelectorAll: () => [a, b] }
    motionTabs.mounted(root)
    a.style.display = 'none'
    b.style.display = ''
    motionTabs.updated(root, { value: 'b', oldValue: 'a' })
    expect(b.animate).toHaveBeenCalledOnce()
    expect(a.animate).not.toHaveBeenCalled()
    motionTabs.updated(root, { value: 'b', oldValue: 'b' })
    expect(b.animate).toHaveBeenCalledOnce()
    motionTabs.beforeUnmount(root)
    expect(vi.getTimerCount()).toBe(0)
  })
  it('moves a decorative radio highlight without intercepting input, and cleans up resizing', () => {
    const disconnect = vi.fn(),
      observe = vi.fn()
    window.ResizeObserver = class {
      observe = observe
      disconnect = disconnect
    }
    let left = 10
    const root = {
      style: { width: '200px', height: '24px' },
      offsetWidth: 200,
      offsetHeight: 24,
      getBoundingClientRect: () => ({ left: 10, top: 20, width: 200, height: 24 }),
      querySelector: () => ({
        style: { borderRadius: left === 10 ? '8px 0px 0px 8px' : '0px 8px 8px 0px' },
        getBoundingClientRect: () => ({ left, top: 20, width: 80, height: 24 }),
      }),
      prepend: vi.fn(),
      classList: { add: vi.fn(), remove: vi.fn() },
    }
    motionSelection.mounted(root)
    const indicator = root.prepend.mock.calls[0][0]
    expect(indicator.setAttribute).toHaveBeenCalledWith('aria-hidden', 'true')
    expect(indicator.style.transform).toBe('translate(0px, 0px)')
    expect(indicator.style.borderRadius).toBe('8px 0px 0px 8px')
    left = 90
    motionSelection.updated(root, { value: 'pdf', oldValue: 'docx' })
    expect(indicator.style.transform).toBe('translate(80px, 0px)')
    expect(indicator.style.borderRadius).toBe('0px 8px 8px 0px')
    expect(indicator.animate).toHaveBeenCalledOnce()
    motionSelection.beforeUnmount(root)
    expect(disconnect).toHaveBeenCalledOnce()
    expect(indicator.remove).toHaveBeenCalledOnce()
    expect(vi.getTimerCount()).toBe(0)
  })
  it('keeps navigation highlights aligned at fractional widths and hides them outside the selected route', () => {
    let selected = true
    const root = {
      style: { width: '229.390625px', height: '49px', borderTopWidth: '1px' },
      offsetWidth: 229, offsetHeight: 49, scrollTop: 0, scrollLeft: 0,
      getBoundingClientRect: () => ({ left: 0, top: 671, width: 229.390625, height: 49 }),
      querySelector: () => selected ? {
        style: { borderRadius: '8px' },
        getBoundingClientRect: () => ({ left: 117.6875, top: 680, width: 101.703125, height: 32 }),
      } : null,
      prepend: vi.fn(), classList: { add: vi.fn(), remove: vi.fn() },
    }
    motionNavigation.mounted(root, { arg: 'footer' })
    const indicator = root.prepend.mock.calls[0][0]
    expect(indicator.style.transform).toBe('translate(117.6875px, 8px)')
    expect(indicator.style.width).toBe('101.703125px')
    expect(indicator.style.height).toBe('32px')
    selected = false
    motionNavigation.updated(root, { value: 'evidence', oldValue: 'settings' })
    expect(indicator.style.visibility).toBe('hidden')
    selected = true
    motionNavigation.updated(root, { value: 'settings', oldValue: 'evidence' })
    expect(indicator.style.visibility).toBe('visible')
    expect(indicator.animate).not.toHaveBeenCalled()
    motionNavigation.beforeUnmount(root)
  })

})
