import { afterEach, expect, it, vi } from 'vitest'
const lifecycle = vi.hoisted(() => ({ unmount: null, draw: null }))
vi.mock('vue', async (original) => ({
  ...(await original()),
  onMounted: (callback) => callback(),
  onBeforeUnmount: (callback) => {
    lifecycle.unmount = callback
  },
  watch: (sources, callback) => {
    let dispose
    callback(
      sources.map((source) => source.value),
      undefined,
      (fn) => {
        dispose = fn
      },
    )
    return () => dispose?.()
  },
}))
vi.mock('./docletGazeDriver.js', () => ({
  createGazeDriver: ({ draw }) => {
    lifecycle.draw = draw
    return { start: () => draw(48, 48, 1), stop: vi.fn(), aim: vi.fn() }
  },
}))
afterEach(() => {
  lifecycle.unmount?.()
  lifecycle.unmount = null
  vi.unstubAllGlobals()
  vi.unstubAllEnvs()
  vi.resetModules()
})
async function setup(mode, enabled = true) {
  vi.stubEnv('VITE_DOCLET_DIAGNOSTICS', enabled ? '1' : '')
  const diagnostic = await import('./docletGazeDiagnostics.js')
  diagnostic.gazeDiagnosticMode.value = mode
  const operations = []
  const context = {
    clearRect: vi.fn(),
    globalAlpha: 1,
    globalCompositeOperation: 'source-over',
    drawImage: vi.fn(function () {
      operations.push({ alpha: this.globalAlpha, composite: this.globalCompositeOperation })
    }),
    getContextAttributes: () => ({ alpha: true }),
  }
  const canvas = { getBoundingClientRect: () => ({ width: 120, height: 150 }), getContext: vi.fn(() => context) }
  const removeEventListener = vi.fn()
  vi.stubGlobal('window', {
    Image: class {
      set src(value) {
        this.url = value
        Promise.resolve().then(() => this.onload())
      }
    },
    devicePixelRatio: 2,
    setTimeout,
    clearTimeout,
    addEventListener: vi.fn(),
    removeEventListener,
    requestAnimationFrame: vi.fn(),
    cancelAnimationFrame: vi.fn(),
  })
  vi.stubGlobal('document', { documentElement: { addEventListener: vi.fn(), removeEventListener: vi.fn() } })
  const { useDocletGaze } = await import('./useDocletGaze.js')
  const fail = vi.fn()
  useDocletGaze({ value: canvas }, { value: canvas }, { value: true }, fail)
  await vi.waitFor(() => expect(context.drawImage).toHaveBeenCalled())
  return { context, canvas, operations, fail, removeEventListener }
}
it.each([
  ['original', true, 'lighter'],
  ['synchronized', false, 'lighter'],
  ['plain', true, 'source-over'],
  ['combined', false, 'source-over'],
])('diagnostic mode %s isolates context and ordinary frame blending', async (mode, desynchronized, composite) => {
  const h = await setup(mode)
  expect(h.canvas.getContext).toHaveBeenCalledWith('2d', { alpha: true, desynchronized })
  expect([h.canvas.width, h.canvas.height]).toEqual([180, 225])
  expect(h.operations.at(-1)).toEqual({ alpha: 1, composite })
  expect(h.context.clearRect).toHaveBeenCalledWith(0, 0, 180, 225)
  lifecycle.draw(1195, 65, 0.5)
  expect(h.operations.slice(-2)).toEqual([
    { alpha: 0.5, composite: 'source-over' },
    { alpha: 0.5, composite: 'lighter' },
  ])
  expect(h.context.globalAlpha).toBe(1)
  expect(h.context.globalCompositeOperation).toBe('source-over')
  expect(h.fail).not.toHaveBeenCalled()
  lifecycle.unmount()
  expect(h.removeEventListener).toHaveBeenCalledWith('pointermove', expect.any(Function))
})
it('normal builds use synchronized transparent compositing and retain additive blending', async () => {
  const h = await setup('combined', false)
  expect(h.canvas.getContext).toHaveBeenCalledWith('2d', { alpha: true, desynchronized: false })
  expect(h.operations.at(-1).composite).toBe('lighter')
})
