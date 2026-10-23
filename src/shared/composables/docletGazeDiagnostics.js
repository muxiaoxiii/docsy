import { ref } from 'vue'

export const diagnosticBuild = import.meta.env.VITE_DOCLET_DIAGNOSTICS === '1'
export const gazeDiagnosticMode = ref('original')
export const gazeDiagnosticModes = [
  { value: 'original', label: 'A · 原始方式（对照）', desynchronized: true, additive: true },
  { value: 'synchronized', label: 'B · 关闭低延迟', desynchronized: false, additive: true },
  { value: 'plain', label: 'C · 普通帧直接绘制', desynchronized: true, additive: false },
  { value: 'combined', label: 'D · 两项同时调整', desynchronized: false, additive: false },
]
const events = []
export function gazeRenderOptions(mode) {
  return gazeDiagnosticModes.find((entry) => entry.value === mode) || gazeDiagnosticModes[0]
}
export function recordGazeDiagnostic(type, detail = {}) {
  if (!diagnosticBuild) return
  events.push({ time: new Date().toISOString(), type, mode: gazeDiagnosticMode.value, ...detail })
  if (events.length > 600) events.shift()
}
export function gazeDiagnosticReport() {
  return JSON.stringify(
    {
      diagnostic: 'Docsy 1.1.2 gaze diagnostic 1',
      mode: gazeDiagnosticMode.value,
      userAgent: navigator.userAgent,
      pixelRatio: window.devicePixelRatio,
      viewport: [window.innerWidth, window.innerHeight],
      page: window.location.pathname,
      events,
    },
    null,
    2,
  )
}
