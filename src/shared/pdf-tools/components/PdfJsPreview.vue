<template>
  <div class="pdfjs-preview" :class="{ 'is-compact': compact }">
    <div v-if="errorText" class="preview-error">{{ errorText }}</div>
    <div v-else-if="!filePath" class="preview-empty">请选择 PDF 文件后生成预览</div>
    <div v-else class="preview-stage">
      <div class="page-preview" :style="frameStyle" :aria-busy="loading">
        <img v-if="imageDataUrl" :src="imageDataUrl" class="pdf-image" alt="PDF 页面预览" />
        <canvas v-else ref="canvasRef" class="pdf-canvas" />
        <div v-show="!loading" class="docsy-overlay">
          <slot :page-info="pageInfo" />
        </div>
        <div v-if="loading" class="preview-loading" role="status">正在加载 PDF 第 {{ page }} 页…</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { readFile } from '@tauri-apps/plugin-fs'
import pdfWorkerUrl from 'pdfjs-dist/build/pdf.worker.mjs?url'
import { pdfPreviewCache } from '../pdfPreviewCache.js'
import { DESKTOP_ONLY_MESSAGE, hasDesktopRuntime } from '../../../core/desktopRuntime.js'

const props = defineProps({
  filePath: { type: String, default: '' },
  page: { type: Number, default: 1 },
  scale: { type: Number, default: 1.6 },
  reloadKey: { type: Number, default: 0 },
  engine: { type: String, default: 'auto' },
  compact: { type: Boolean, default: false },
})

const emit = defineEmits(['loaded', 'error'])

const canvasRef = ref(null)
const loading = ref(false)
const errorText = ref('')
const imageDataUrl = ref('')
const pdfDoc = shallowRef(null)
const renderTask = shallowRef(null)
let pdfjsLibPromise = null
let renderSeq = 0
let renderTimer = null
const pageInfo = ref({
  page: 1,
  pages: 0,
  widthPt: 595.28,
  heightPt: 841.89,
  widthPx: 0,
  heightPx: 0,
})

const frameStyle = computed(() => ({
  aspectRatio:
    pageInfo.value.widthPx && pageInfo.value.heightPx
      ? `${pageInfo.value.widthPx} / ${pageInfo.value.heightPx}`
      : `${pageInfo.value.widthPt} / ${pageInfo.value.heightPt}`,
}))

watch(
  () => [props.filePath, props.page, props.scale, props.reloadKey, props.engine],
  (current, previous) => {
    renderSeq += 1
    cancelRender()
    if (renderTimer) clearTimeout(renderTimer)
    const force = Boolean(previous && current[0] === previous[0] && current[3] !== previous[3])
    // Keep rapid keyboard/input navigation from launching many native processes.
    // A loaded image stays covered until the requested page is actually ready.
    if (!previous || force || current[0] !== previous[0]) void renderCurrentPage(force)
    else {
      loading.value = true
      renderTimer = setTimeout(() => { renderTimer = null; void renderCurrentPage(false) }, 80)
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  renderSeq += 1
  if (renderTimer) clearTimeout(renderTimer)
  cancelRender()
  if (pdfDoc.value) {
    void pdfDoc.value.loadingTask.destroy()
    pdfDoc.value = null
  }
})

async function renderCurrentPage(force = false) {
  const requestId = ++renderSeq
  cancelRender()
  errorText.value = ''
  if (!props.filePath) { imageDataUrl.value = ''; loading.value = false; return }
  if (!hasDesktopRuntime()) {
    errorText.value = DESKTOP_ONLY_MESSAGE
    emit('error', errorText.value)
    return
  }

  loading.value = true
  try {
    if (props.engine === 'backend') {
      await renderWithBackend(requestId, force)
      return
    }
    if (props.engine === 'auto') {
      const rendered = await tryRenderWithBackend(requestId, force)
      if (rendered) return
    }
    await renderWithPdfJs(requestId)
  } catch (err) {
    if (!isCurrent(requestId)) return
    if (String(err?.name || err).includes('RenderingCancelledException')) return
    errorText.value = `预览渲染失败：${String(err)}`
    emit('error', errorText.value)
  } finally {
    if (isCurrent(requestId)) {
      loading.value = false
    }
  }
}

async function renderWithBackend(requestId, force = false) {
  const data = await pdfPreviewCache.load({ inputPath: props.filePath, page: props.page, dpi: previewDpi(), forceRefresh: force })
  if (!isCurrent(requestId)) return
  imageDataUrl.value = data.imageDataUrl
  pageInfo.value = {
    page: data.page,
    pages: data.pages,
    widthPt: data.widthPt,
    heightPt: data.heightPt,
    widthPx: data.widthPx,
    heightPx: data.heightPx,
  }
  emit('loaded', { ...pageInfo.value })
}

async function tryRenderWithBackend(requestId, force = false) {
  try {
    await renderWithBackend(requestId, force)
    return true
  } catch {
    if (!isCurrent(requestId)) return true
    imageDataUrl.value = ''
    return false
  }
}

async function renderWithPdfJs(requestId) {
  await nextTick()
  if (!canvasRef.value) return
  const doc = await loadDocument(props.filePath, props.reloadKey)
  if (!isCurrent(requestId)) return
  const pageNumber = Math.min(Math.max(1, props.page), doc.numPages)
  const page = await doc.getPage(pageNumber)
  if (!isCurrent(requestId)) return
  const viewport = page.getViewport({ scale: props.scale })
  const unitViewport = page.getViewport({ scale: 1 })
  const canvas = canvasRef.value
  const context = canvas.getContext('2d')

  canvas.width = Math.floor(viewport.width)
  canvas.height = Math.floor(viewport.height)

  renderTask.value = page.render({
    canvasContext: context,
    viewport,
  })
  await renderTask.value.promise
  renderTask.value = null
  if (!isCurrent(requestId)) return

  pageInfo.value = {
    page: pageNumber,
    pages: doc.numPages,
    widthPt: unitViewport.width,
    heightPt: unitViewport.height,
    widthPx: canvas.width,
    heightPx: canvas.height,
  }
  emit('loaded', { ...pageInfo.value })
}

function isCurrent(requestId) {
  return requestId === renderSeq
}

function previewDpi() {
  return Math.min(180, Math.max(72, Math.round(Number(props.scale || 1) * 72)))
}

async function loadDocument(path, reloadKey) {
  const version = await pdfPreviewCache.fingerprint(path)
  const cacheKey = `${path}:${reloadKey}:${version ?? renderSeq}`
  if (pdfDoc.value && pdfDoc.value.__docsyKey === cacheKey) {
    return pdfDoc.value
  }
  if (pdfDoc.value) {
    await pdfDoc.value.loadingTask.destroy()
    pdfDoc.value = null
  }
  const data = await readFile(path)
  if (version !== null && await pdfPreviewCache.fingerprint(path) !== version) throw new Error('文件在预览期间发生变化，请重新预览')
  const pdfjsLib = await loadPdfJs()
  const loadingTask = pdfjsLib.getDocument({
    data,
    enableXfa: false,
  })
  const doc = await loadingTask.promise
  doc.__docsyKey = cacheKey
  pdfDoc.value = doc
  return doc
}

async function loadPdfJs() {
  if (!pdfjsLibPromise) {
    pdfjsLibPromise = import('pdfjs-dist/build/pdf.mjs').then((pdfjsLib) => {
      pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl
      return pdfjsLib
    })
  }
  return pdfjsLibPromise
}

function cancelRender() {
  if (renderTask.value) {
    renderTask.value.cancel()
    renderTask.value = null
  }
}
</script>

<style scoped>
.pdfjs-preview {
  width: 100%;
}

.preview-stage {
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding: 12px;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  min-height: 520px;
}

.page-preview {
  position: relative;
  flex: 0 0 auto;
  width: 100%;
  max-width: 100%;
  background: #fff;
  box-shadow: 0 2px 14px rgba(0, 0, 0, 0.16);
  overflow: hidden;
}

.pdfjs-preview.is-compact .preview-stage {
  min-height: 0;
  padding: 3px;
}

.pdf-canvas,
.pdf-image {
  display: block;
  width: 100%;
  height: auto;
  object-fit: contain;
}

.docsy-overlay {
  position: absolute;
  inset: 0;
  pointer-events: none;
  /* overlay 与页面元素同尺寸（inset: 0 于 .page-preview 内）；
     声明为内联尺寸容器，使页眉/页脚 overlay 字号可用 cqw 随页宽缩放，
     与后端按绝对 pt 绘制保持一致（见 pdfPreviewCoordinates.textOverlayStyle）。 */
  container-type: inline-size;
}

.preview-loading {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--docsy-text);
  background: rgba(255, 255, 255, 0.94);
  font-size: 13px;
}

.preview-empty,
.preview-error {
  padding: 12px;
  border-radius: var(--docsy-radius);
  font-size: 13px;
}

.preview-empty {
  color: var(--docsy-text);
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
}

.preview-error {
  color: var(--el-color-danger);
  background: #fff2f0;
  border: 1px solid #ffccc7;
}
</style>
