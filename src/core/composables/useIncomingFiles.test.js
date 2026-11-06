import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createRenderer, defineComponent, h, nextTick, ref } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { createMemoryHistory, createRouter, RouterView } from 'vue-router'
import { useWorkspaceStore } from '../../stores/workspace.js'
import { useIncomingFiles } from './useIncomingFiles.js'

vi.mock('../tauriBridge.js', () => ({ userFacingError: (e) => e.message || String(e) }))
const renderer = createRenderer({
  createElement: () => ({}),
  createText: () => ({}),
  createComment: () => ({}),
  setText() {},
  setElementText() {},
  patchProp() {},
  insert() {},
  remove() {},
  parentNode: () => null,
  nextSibling: () => null,
})
const settle = async () => {
  for (let i = 0; i < 8; i++) {
    await Promise.resolve()
    await nextTick()
  }
}
let app
beforeEach(() => {
  const data = new Map()
  vi.stubGlobal('window', {
    sessionStorage: {
      getItem: (k) => data.get(k) || null,
      setItem: (k, v) => data.set(k, v),
      removeItem: (k) => data.delete(k),
    },
  })
})
afterEach(() => {
  app?.unmount()
  vi.unstubAllGlobals()
})
async function fixture(receive, busy = ref(false)) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const store = useWorkspaceStore(),
    state = {}
  const Receiver = defineComponent({
    setup() {
      state.incoming = useIncomingFiles('pdf-tools', receive, { busy: () => busy.value })
      return () => h('div')
    },
  })
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: { render: () => h('div') } },
      { path: '/pdf', name: 'pdf-tools', component: Receiver },
    ],
  })
  app = renderer.createApp({ render: () => h(RouterView) })
  app.use(pinia)
  app.use(router)
  await router.push('/pdf')
  await router.isReady()
  app.mount({})
  await settle()
  return { store, state }
}
describe('receiving generated files', () => {
  it('defers while busy, then acknowledges exactly once after acceptance', async () => {
    const busy = ref(true),
      receive = vi.fn(async () => true),
      { store, state } = await fixture(receive, busy)
    store.sendFiles(['/a.pdf'], 'pdf-merge', '裁剪')
    await settle()
    expect(receive).not.toHaveBeenCalled()
    expect(state.incoming.blocked.value).toBe(true)
    busy.value = false
    await settle()
    expect(receive).toHaveBeenCalledTimes(1)
    expect(store.fileTransfer).toBeNull()
  })
  it('retains cancelled or failed imports and supports explicit retry', async () => {
    const receive = vi
      .fn()
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error('missing file'))
      .mockResolvedValue(true)
    const { store, state } = await fixture(receive)
    store.sendFiles(['/a.pdf'], 'pdf-merge')
    await settle()
    expect(store.fileTransfer).not.toBeNull()
    await state.incoming.retry()
    expect(state.incoming.error.value).toBe('missing file')
    expect(store.fileTransfer).not.toBeNull()
    await state.incoming.retry()
    await settle()
    expect(store.fileTransfer).toBeNull()
    expect(receive).toHaveBeenCalledTimes(3)
  })
  it('does not acknowledge a newer batch while an older one finishes', async () => {
    let finish
    const receive = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve
          }),
      )
      .mockResolvedValue(true)
    const { store } = await fixture(receive)
    store.sendFiles(['/old.pdf'], 'pdf-merge')
    await settle()
    const batch = store.sendFiles(['/new.pdf'], 'pdf-merge')
    await settle()
    finish(true)
    await settle()
    expect(receive).toHaveBeenCalledTimes(2)
    expect(receive.mock.calls[1][0].id).toBe(batch.id)
    expect(store.fileTransfer).toBeNull()
  })
})
