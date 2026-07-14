<template>
  <div class="pdfjs-preview">
    <div v-if="errorText" class="preview-error">{{ errorText }}</div>
    <div v-else-if="!filePath" class="preview-empty">请选择 PDF 文件后生成预览</div>
    <div v-else class="preview-stage">
      <div class="page-preview" :style="frameStyle">
        <img v-if="imageDataUrl" :src="imageDataUrl" class="pdf-image" alt="PDF 页面预览" />
        <canvas v-else ref="canvasRef" class="pdf-canvas" />
        <div class="docsy-overlay">
          <slot :page-info="pageInfo" />
        </div>
        <div v-if="loading" class="preview-loading">正在渲染</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { readFile } from '@tauri-apps/plugin-fs'
import { tauriCallSafe } from '../../../core/tauriBridge.js'

const props = defineProps({
  filePath: { type: String, default: '' },
  page: { type: Number, default: 1 },
  scale: { type: Number, default: 1.6 },
  reloadKey: { type: Number, default: 0 },
  engine: { type: String, default: 'auto' },
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
const pageInfo = ref({
  page: 1,
  pages: 0,
  widthPt: 595.28,
  heightPt: 841.89,
  widthPx: 0,
  heightPx: 0,
})

const frameStyle = computed(() => ({
  aspectRatio: pageInfo.value.widthPx && pageInfo.value.heightPx
    ? `${pageInfo.value.widthPx} / ${pageInfo.value.heightPx}`
    : `${pageInfo.value.widthPt} / ${pageInfo.value.heightPt}`,
}))

watch(
  () => [props.filePath, props.page, props.scale, props.reloadKey, props.engine],
  () => renderCurrentPage(),
  { immediate: true }
)

onBeforeUnmount(() => {
  renderSeq += 1
  cancelRender()
  if (pdfDoc.value) {
    pdfDoc.value.destroy()
    pdfDoc.value = null
  }
})

async function renderCurrentPage() {
  const requestId = ++renderSeq
  cancelRender()
  errorText.value = ''
  imageDataUrl.value = ''
  if (!props.filePath) return

  loading.value = true
  try {
    if (props.engine === 'backend') {
      await renderWithBackend(requestId)
      return
    }
    if (props.engine === 'auto') {
      const rendered = await tryRenderWithBackend(requestId)
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

async function renderWithBackend(requestId) {
  const result = await tauriCallSafe('render_pdf_preview', {
    args: {
      inputPath: props.filePath,
      page: props.page,
      dpi: previewDpi(),
    },
  })
  if (!result.ok) {
    throw new Error(result.error || '后端预览失败')
  }
  if (!isCurrent(requestId)) return
  imageDataUrl.value = result.data.imageDataUrl
  pageInfo.value = {
    page: result.data.page,
    pages: result.data.pages,
    widthPt: result.data.widthPt,
    heightPt: result.data.heightPt,
    widthPx: result.data.widthPx,
    heightPx: result.data.heightPx,
  }
  emit('loaded', { ...pageInfo.value })
}

async function tryRenderWithBackend(requestId) {
  try {
    await renderWithBackend(requestId)
    return true
  } catch (err) {
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
  const cacheKey = `${path}:${reloadKey}`
  if (pdfDoc.value && pdfDoc.value.__docsyKey === cacheKey) {
    return pdfDoc.value
  }
  if (pdfDoc.value) {
    await pdfDoc.value.destroy()
    pdfDoc.value = null
  }
  const data = await readFile(path)
  const pdfjsLib = await loadPdfJs()
  const loadingTask = pdfjsLib.getDocument({
    data,
    enableXfa: true,
  })
  const doc = await loadingTask.promise
  doc.__docsyKey = cacheKey
  pdfDoc.value = doc
  return doc
}

async function loadPdfJs() {
  if (!pdfjsLibPromise) {
    pdfjsLibPromise = Promise.all([
      import('pdfjs-dist/build/pdf.mjs'),
      import('pdfjs-dist/build/pdf.worker.mjs?url'),
    ]).then(([pdfjsLib, workerModule]) => {
      pdfjsLib.GlobalWorkerOptions.workerSrc = workerModule.default
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
  padding: 12px;
  background: #f5f7fa;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  min-height: 520px;
}

.page-preview {
  position: relative;
  width: min(100%, 620px);
  background: #fff;
  box-shadow: 0 2px 14px rgba(0, 0, 0, 0.16);
}

.pdf-canvas,
.pdf-image {
  display: block;
  width: 100%;
  height: 100%;
}

.docsy-overlay {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.preview-loading {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #606266;
  background: rgba(255, 255, 255, 0.62);
  font-size: 13px;
}

.preview-empty,
.preview-error {
  padding: 12px;
  border-radius: 6px;
  font-size: 13px;
}

.preview-empty {
  color: #606266;
  background: #f5f7fa;
  border: 1px solid #e4e7ed;
}

.preview-error {
  color: #b42318;
  background: #fff2f0;
  border: 1px solid #ffccc7;
}
</style>
