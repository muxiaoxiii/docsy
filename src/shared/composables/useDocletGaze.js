import { onBeforeUnmount, onMounted, watch } from 'vue'
import atlasUrl from '../../assets/doclet-video/gaze-atlas.webp'
import { createGazeDriver } from './docletGazeDriver.js'
import { gazeTarget, GAZE_CELL, GAZE_LAYOUT } from './docletGazePoses.js'
import { createDwellTarget } from './docletDwellTarget.js'
import {
  diagnosticBuild,
  gazeDiagnosticMode,
  gazeRenderOptions,
  recordGazeDiagnostic,
} from './docletGazeDiagnostics.js'
let sharedAtlas
function loadAtlas() {
  if (!sharedAtlas)
    sharedAtlas = new Promise((resolve, reject) => {
      const image = new window.Image()
      image.onload = () => resolve(image)
      image.onerror = () => {
        sharedAtlas = null
        reject(new Error('Doclet atlas unavailable'))
      }
      image.src = atlasUrl
    })
  return sharedAtlas
}
export function useDocletGaze(root, canvas, enabled, onError) {
  let stop
  onMounted(() => {
    stop = watch(
      [enabled, canvas, gazeDiagnosticMode],
      ([active, node], _, cleanup) => {
        if (!active || !node) return
        let cancelled = false
        let driver
        let lastFocus
        let bounds = root.value?.getBoundingClientRect()
        const measure = () => {
          bounds = root.value?.getBoundingClientRect()
        }
        const attention = createDwellTarget({
          schedule: (callback, delay) => window.setTimeout(callback, delay),
          unschedule: (timer) => window.clearTimeout(timer),
          emit: (point) => {
            lastFocus = point
            if (!bounds) return
            recordGazeDiagnostic('target', {
              direction: gazeTarget(
                (point.x - bounds.left - bounds.width / 2) / Math.max(bounds.width, 160),
                (point.y - bounds.top - bounds.height / 2) / Math.max(bounds.height, 160),
              ),
            })
            driver?.aim(
              gazeTarget(
                (point.x - bounds.left - bounds.width / 2) / Math.max(bounds.width, 160),
                (point.y - bounds.top - bounds.height / 2) / Math.max(bounds.height, 160),
              ),
            )
          },
        })
        const move = (event) => {
          if (event.pointerType !== 'touch') attention.move(event.clientX, event.clientY)
        }
        const reset = () => {
          attention.reset()
          lastFocus = null
          recordGazeDiagnostic('return-center')
          driver?.aim(null)
        }
        loadAtlas()
          .then((image) => {
            if (cancelled) return
            const rect = node.getBoundingClientRect()
            const ratio = Math.min(window.devicePixelRatio || 1, 1.5)
            node.width = Math.round(rect.width * ratio)
            node.height = Math.round(rect.height * ratio)
            // Windows WebView2 can show black behind transparent low-latency
            // canvases. Keep normal compositing and the existing frame blending.
            const options = gazeRenderOptions(diagnosticBuild ? gazeDiagnosticMode.value : 'synchronized')
            const context = node.getContext('2d', { alpha: true, desynchronized: options.desynchronized })
            recordGazeDiagnostic('canvas-created', {
              requested: { alpha: true, desynchronized: options.desynchronized },
              actual: context?.getContextAttributes?.(),
              size: [node.width, node.height],
            })
            if (!context) {
              onError()
              return
            }
            driver = createGazeDriver({
              request: (callback) => window.requestAnimationFrame(callback),
              cancel: (id) => window.cancelAnimationFrame(id),
              draw: (frame, from, mix) => {
                recordGazeDiagnostic('draw', { frame, from, mix })
                context.clearRect(0, 0, node.width, node.height)
                const { columns, width, height } = GAZE_LAYOUT
                const paint = (pose, opacity) => {
                  if (opacity <= 0) return
                  const cell = GAZE_CELL.get(pose) ?? 0
                  context.globalAlpha = opacity
                  context.drawImage(
                    image,
                    (cell % columns) * width,
                    Math.floor(cell / columns) * height,
                    width,
                    height,
                    0,
                    0,
                    node.width,
                    node.height,
                  )
                }
                paint(from, 1 - mix)
                // Add premultiplied pixels: a source-over crossfade lowers the
                // overlapping white body's opacity and causes a visible flash.
                context.globalCompositeOperation = options.additive || mix < 1 ? 'lighter' : 'source-over'
                paint(frame, mix)
                context.globalCompositeOperation = 'source-over'
                context.globalAlpha = 1
              },
            })
            driver.start()
            if (lastFocus && bounds)
              driver.aim(
                gazeTarget(
                  (lastFocus.x - bounds.left - bounds.width / 2) / Math.max(bounds.width, 160),
                  (lastFocus.y - bounds.top - bounds.height / 2) / Math.max(bounds.height, 160),
                ),
              )
          })
          .catch(() => {
            if (!cancelled) onError()
          })
        window.addEventListener('pointermove', move, { passive: true })
        window.addEventListener('resize', measure, { passive: true })
        window.addEventListener('scroll', measure, { passive: true, capture: true })
        document.documentElement.addEventListener('pointerleave', reset)
        window.addEventListener('blur', reset)
        cleanup(() => {
          cancelled = true
          attention.reset()
          driver?.stop()
          recordGazeDiagnostic('canvas-disposed')
          window.removeEventListener('pointermove', move)
          window.removeEventListener('resize', measure)
          window.removeEventListener('scroll', measure, true)
          document.documentElement.removeEventListener('pointerleave', reset)
          window.removeEventListener('blur', reset)
        })
      },
      { immediate: true, flush: 'post' },
    )
  })
  onBeforeUnmount(() => stop?.())
}
