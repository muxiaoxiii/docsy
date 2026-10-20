import { onBeforeUnmount, onMounted, watch } from 'vue'
import atlasUrl from '../../assets/doclet-video/gaze-atlas.webp'
import { createGazeDriver } from './docletGazeDriver.js'
export { gazeDistance } from './docletGazeDriver.js'
const circle = [8, 12, 17, 22, 26, 31, 36, 41, 56]
export function gazePosition(x, y) {
  if (Math.hypot(x, y) < 0.18) return 0
  const angle = (Math.atan2(x, -y) + Math.PI * 2) % (Math.PI * 2)
  const sector = angle / (Math.PI / 4)
  const start = Math.floor(sector)
  return (circle[start] + (circle[start + 1] - circle[start]) * (sector - start)) % 48
}
export function gazeFrame(x, y) { return Math.round(gazePosition(x, y)) % 48 }
let sharedAtlas
function loadAtlas() {
  if (!sharedAtlas) sharedAtlas = new Promise((resolve, reject) => {
    const image = new window.Image()
    image.onload = () => resolve(image)
    image.onerror = () => { sharedAtlas = null; reject(new Error('Doclet atlas unavailable')) }
    image.src = atlasUrl
  })
  return sharedAtlas
}
export function useDocletGaze(root, canvas, enabled, onError) {
  let stop
  onMounted(() => {
    stop = watch([enabled, canvas], ([active, node], _, cleanup) => {
      if (!active || !node) return
      let cancelled = false
      let driver
      let settleTimer
      let lastPointer
      let bounds = root.value?.getBoundingClientRect()
      const measure = () => { bounds = root.value?.getBoundingClientRect() }
      const move = event => {
        if (!bounds) return
        const now = event.timeStamp
        const speed = lastPointer ? Math.hypot(event.clientX - lastPointer.x, event.clientY - lastPointer.y) / Math.max(1, now - lastPointer.time) : 0
        lastPointer = { x: event.clientX, y: event.clientY, time: now }
        const target = gazePosition(
          (event.clientX - bounds.left - bounds.width / 2) / Math.max(bounds.width, 160),
          (event.clientY - bounds.top - bounds.height / 2) / Math.max(bounds.height, 160),
        )
        window.clearTimeout(settleTimer)
        if (speed > 1.2) {
          // Drop fast intermediate positions. Follow only the final settled direction.
          settleTimer = window.setTimeout(() => driver?.aim(target), 120)
        } else driver?.aim(target)
      }
      const reset = () => { window.clearTimeout(settleTimer); lastPointer = null; driver?.aim(0) }
      loadAtlas().then(image => {
        if (cancelled) return
        const rect = node.getBoundingClientRect()
        const ratio = Math.min(window.devicePixelRatio || 1, 1.5)
        node.width = Math.round(rect.width * ratio)
        node.height = Math.round(rect.height * ratio)
        const context = node.getContext('2d', { alpha: true, desynchronized: true })
        if (!context) { onError(); return }
        driver = createGazeDriver({
          request: callback => window.requestAnimationFrame(callback),
          cancel: id => window.cancelAnimationFrame(id),
          draw: frame => {
            context.clearRect(0, 0, node.width, node.height)
            context.drawImage(image, (frame % 10) * 192, Math.floor(frame / 10) * 240, 192, 240, 0, 0, node.width, node.height)
          },
        })
        driver.start()
      }).catch(() => { if (!cancelled) onError() })
      window.addEventListener('pointermove', move, { passive: true })
      window.addEventListener('resize', measure, { passive: true })
      window.addEventListener('scroll', measure, { passive: true, capture: true })
      document.documentElement.addEventListener('pointerleave', reset)
      window.addEventListener('blur', reset)
      cleanup(() => {
        cancelled = true
        window.clearTimeout(settleTimer)
        driver?.stop()
        window.removeEventListener('pointermove', move)
        window.removeEventListener('resize', measure)
        window.removeEventListener('scroll', measure, true)
        document.documentElement.removeEventListener('pointerleave', reset)
        window.removeEventListener('blur', reset)
      })
    }, { immediate: true, flush: 'post' })
  })
  onBeforeUnmount(() => stop?.())
}
