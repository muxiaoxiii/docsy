<template>
  <el-dialog
    v-model="visible"
    title="图片转 PDF"
    width="min(1100px, calc(100vw - 32px))"
    :close-on-click-modal="false"
    :close-on-press-escape="!exporting"
    :show-close="!exporting"
    :transition="{ name: 'dialog-fade', css: false }"
    destroy-on-close
    @closed="handleClosed"
  >
    <div class="image-pdf-dialog compact-form" :class="{ 'has-list': images.length > 1 }" v-loading="loading">
      <aside v-if="images.length > 1" class="image-list" aria-label="图片列表">
        <div class="list-heading">{{ images.length }} 张 · {{ pages.length }} 页</div>
        <FileQueuePanel
          :items="namedImages"
          :clearable="false"
          :removable="false"
          :busy="exporting"
          sortable
          selectable
          group-drop-attribute="data-image-page-drop"
          :selected-index="selected"
          max-height="440px"
          @select="select"
          @reorder="reorder"
          @group-drop="joinPage"
          @group-hover="pageDropTarget = $event"
        >
          <template #before-item="{ item, index, dragging, groupDropTarget }"
            ><div
              v-if="isPageStart(index)"
              :class="['page-heading', { 'is-drop-target': groupDropTarget === item.pageId }]"
            >
              <div class="page-headline">
                <button type="button" class="page-select" @click="select(index)">
                  第 {{ pageOf(item.pageId) + 1 }} 页 · {{ pageImages(item.pageId).length }} 张
                </button>
                <el-button
                  v-if="active?.pageId !== item.pageId"
                  link
                  size="small"
                  :disabled="exporting"
                  @click="joinPage({ from: selected, target: item.pageId })"
                  >放入选中图</el-button
                >
              </div>
              <div :data-image-page-drop="item.pageId" :class="['page-drop-zone', { 'is-dragging': dragging }]">
                拖到此处放入本页
              </div>
            </div></template
          >
          <template #item-actions="{ index }"
            ><el-button
              link
              size="small"
              type="danger"
              :disabled="exporting"
              :aria-label="`移除第 ${index + 1} 张图片`"
              @click="remove(index)"
              >移除</el-button
            ></template
          >
        </FileQueuePanel>
        <div class="list-actions">
          <el-button size="small" plain :disabled="selected === 0 || exporting" @click="move(-1)">上移</el-button
          ><el-button size="small" plain :disabled="selected === images.length - 1 || exporting" @click="move(1)"
            >下移</el-button
          >
        </div>
      </aside>
      <section class="preview-column">
        <div class="paper-toolbar">
          <el-button-group
            ><el-button size="small" :disabled="page === 0" @click="page--">上一页</el-button
            ><el-button size="small" :disabled="page + 1 >= pageCount" @click="page++"
              >下一页</el-button
            ></el-button-group
          >
          <span>第 {{ page + 1 }} / {{ pageCount }} 页</span
          ><span class="paper-size">A4 · {{ settings.landscape ? '297 × 210' : '210 × 297' }} mm</span>
        </div>
        <div
          class="paper-area"
          :class="{ 'is-drop-target': pageDropTarget === pages[page]?.id }"
          :data-image-page-drop="pages[page]?.id"
        >
          <canvas ref="canvas" class="paper" aria-label="PDF 排版预览，点击图片可选择" @click="selectFromCanvas" />
        </div>
        <p class="preview-note">低清预览 · 拖拽柄调整顺序，拖入纸张或合页区才会合为一页</p>
      </section>
      <section class="image-options">
        <el-form label-position="top" size="small" :disabled="exporting">
          <el-form-item label="纸张方向"
            ><el-radio-group v-model="settings.landscape"
              ><el-radio-button :value="false">纵向</el-radio-button
              ><el-radio-button :value="true">横向</el-radio-button></el-radio-group
            ></el-form-item
          >
          <div class="two-fields">
            <el-form-item
              ><template #label
                >本页行数
                <HelpTooltip
                  content="行列不足时自动补足另一项，所有图片仍在本页；已有缩放、旋转和裁剪保持不变。" /></template
              ><el-input-number
                v-model="currentLayout.rows"
                :min="1"
                :max="4"
                @change="layoutChanged('rows')"
                controls-position="right" /></el-form-item
            ><el-form-item label="本页列数"
              ><el-input-number
                v-model="currentLayout.cols"
                :min="1"
                :max="4"
                @change="layoutChanged('cols')"
                controls-position="right"
            /></el-form-item>
          </div>
          <el-button size="small" plain :disabled="loading || !plan" @click="fitPage">本页适配</el-button>
          <div class="two-fields">
            <el-form-item label="纸张边距 mm"
              ><el-input-number v-model="settings.margin" :min="0" :max="40" controls-position="right" /></el-form-item
            ><el-form-item label="图片间距 mm"
              ><el-input-number v-model="settings.gap" :min="0" :max="20" controls-position="right"
            /></el-form-item>
          </div>
          <div v-if="active" class="selected-options">
            <el-button
              v-if="activePage?.images.length > 1"
              class="split-page-button"
              size="small"
              plain
              @click="splitActive"
              >拆成独立页</el-button
            >
            <div class="selected-name" :title="active.path">{{ name(active.path) }}</div>
            <el-form-item
              ><template #label
                >图片缩放
                <HelpTooltip
                  content="100% 以初始适配的实际打印尺寸为基准；调整布局、旋转和裁剪均不改变这一基准。可手动缩小，或使用适应格子、本页适配。裁剪只影响 PDF，原图保持不变。" /></template
              ><el-input-number v-model="active.percent" :min="1" :max="400" :step="5" controls-position="right" /><span
                class="unit"
                >%</span
              ><el-button link size="small" @click="fitActive">适应格子</el-button></el-form-item
            >
            <el-form-item label="旋转"
              ><el-button-group
                ><el-button size="small" @click="active.rotation = (active.rotation + 270) % 360">左转 90°</el-button
                ><el-button size="small" @click="active.rotation = (active.rotation + 90) % 360"
                  >右转 90°</el-button
                ></el-button-group
              ></el-form-item
            >
            <div class="crop-title">
              <el-button size="small" plain @click="openCrop">手动裁剪</el-button
              ><el-button text size="small" @click="resetActive">重置图片</el-button>
            </div>
            <p v-if="selectedPlacement" :class="['dpi-note', { low: selectedPlacement.effective_dpi < 150 }]">
              预计 {{ Math.round(selectedPlacement.effective_dpi) }} DPI<span
                v-if="selectedPlacement.effective_dpi < 150"
              >
                · 原图分辨率偏低，建议缩小打印尺寸</span
              >
            </p>
          </div>
        </el-form>
      </section>
    </div>
    <el-alert
      v-if="error || fitError"
      :title="error || fitError"
      type="error"
      :closable="false"
      show-icon
      class="dialog-error"
    />
    <template #footer
      ><div class="dialog-footer">
        <span>保留原始像素，无额外有损压缩 · {{ pages.length }} 页 PDF</span
        ><el-button :disabled="exporting" @click="visible = false">取消</el-button
        ><el-button
          type="primary"
          :loading="exporting"
          :disabled="loading || !!error || !!fitError || !plan || !images.length"
          @click="confirm"
          >生成 PDF 并加入</el-button
        >
      </div></template
    >
  </el-dialog>
  <el-dialog
    v-model="cropVisible"
    title="裁剪图片"
    width="min(800px, calc(100vw - 32px))"
    :close-on-click-modal="false"
    append-to-body
  >
    <p class="crop-instruction">拖动边缘或角点调整范围，拖动框内可移动。按原图方向裁剪，应用后保留当前旋转。</p>
    <div class="crop-stage">
      <div
        v-if="cropSrc"
        ref="cropSurface"
        class="crop-surface"
        @pointermove="moveCrop"
        @pointerup="endCrop"
        @pointercancel="endCrop"
      >
        <img :src="cropSrc" alt="待裁剪原图" draggable="false" />
        <div class="crop-selection" :style="cropStyle" @pointerdown.stop.prevent="startCrop($event, 'move')">
          <span class="crop-grid-line crop-grid-line--v1"></span><span class="crop-grid-line crop-grid-line--v2"></span
          ><span class="crop-grid-line crop-grid-line--h1"></span
          ><span class="crop-grid-line crop-grid-line--h2"></span>
          <button
            v-for="handle in cropHandles"
            :key="handle"
            type="button"
            :class="['crop-handle', `crop-handle--${handle}`]"
            :aria-label="`调整裁剪${cropHandleLabels[handle]}`"
            @pointerdown.stop.prevent="startCrop($event, handle)"
            @keydown="nudgeCrop($event, handle)"
          ></button>
        </div>
      </div>
    </div>
    <template #footer
      ><el-button @click="cropDraft = [0, 0, 1, 1]">恢复全图</el-button
      ><el-button @click="cropVisible = false">取消</el-button
      ><el-button type="primary" @click="applyCrop">应用裁剪</el-button></template
    >
  </el-dialog>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { tauriCallQuiet, tauriCallSafe } from '../../../core/tauriBridge.js'
import HelpTooltip from '../../components/HelpTooltip.vue'
import FileQueuePanel from '../../components/FileQueuePanel.vue'
import { dragCrop } from '../imageCrop.js'
import {
  imagePages,
  autoImageGrid,
  fitImageGrid,
  joinImagePage,
  reorderPageImage,
  splitImagePage,
} from '../imagePages.js'
const visible = ref(false),
  images = ref([]),
  selected = ref(0),
  page = ref(0)
const settings = reactive({ landscape: false, margin: 10, gap: 4 })
const layouts = reactive({})
const pageDropTarget = ref('')
let pageSequence = 0
const pages = computed(() => imagePages(images.value))
const activePage = computed(() => pages.value[page.value])
const fallbackLayout = reactive({ rows: 1, cols: 1 })
const currentLayout = computed(() => layouts[activePage.value?.id] || fallbackLayout)
const canvas = ref(null),
  plan = ref(null),
  error = ref(''),
  loading = ref(false),
  exporting = ref(false)
const active = computed(() => images.value[selected.value])
const pageCount = computed(() => Math.max(1, pages.value.length))
const selectedPlacement = computed(() => plan.value?.pages.flat().find((item) => item.index === selected.value))
const namedImages = computed(() => images.value.map((image) => ({ ...image, name: name(image.path) })))
const fitError = computed(() =>
  plan.value?.pages.flat().some((item) => !item.fits)
    ? '图片已保持当前比例，但超出格子；请降低缩放或点“适应格子”。'
    : '',
)
const cropVisible = ref(false),
  cropSrc = ref(''),
  cropSurface = ref(null),
  cropDraft = ref([0, 0, 1, 1])
const cropHandles = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w']
const cropHandleLabels = {
  nw: '左上角',
  n: '上边缘',
  ne: '右上角',
  e: '右边缘',
  se: '右下角',
  s: '下边缘',
  sw: '左下角',
  w: '左边缘',
}
const cropStyle = computed(() => ({
  left: `${cropDraft.value[0] * 100}%`,
  top: `${cropDraft.value[1] * 100}%`,
  width: `${(cropDraft.value[2] - cropDraft.value[0]) * 100}%`,
  height: `${(cropDraft.value[3] - cropDraft.value[1]) * 100}%`,
}))
let cropDrag = null,
  cropPath = '',
  cropMin = [0.001, 0.001]
async function openCrop() {
  if (!active.value) return
  try {
    const image = await thumbnail(active.value.path)
    cropPath = active.value.path
    cropSrc.value = image.src
    const [left, top, right, bottom] = active.value.trim
    cropDraft.value = [left / 100, top / 100, 1 - right / 100, 1 - bottom / 100]
    const placement = selectedPlacement.value
    cropMin = [1 / (placement?.source_width || 1000), 1 / (placement?.source_height || 1000)]
    cropVisible.value = true
  } catch (err) {
    error.value = `无法打开裁剪：${err.message}`
  }
}
function startCrop(event, handle) {
  if (event.button !== 0) return
  cropSurface.value.setPointerCapture(event.pointerId)
  cropDrag = {
    pointerId: event.pointerId,
    handle,
    x: event.clientX,
    y: event.clientY,
    box: [...cropDraft.value],
    bounds: cropSurface.value.getBoundingClientRect(),
  }
}
function moveCrop(event) {
  if (!cropDrag || event.pointerId !== cropDrag.pointerId) return
  cropDraft.value = dragCrop(
    cropDrag.box,
    cropDrag.handle,
    (event.clientX - cropDrag.x) / cropDrag.bounds.width,
    (event.clientY - cropDrag.y) / cropDrag.bounds.height,
    ...cropMin,
  )
}
function endCrop() {
  cropDrag = null
}
function nudgeCrop(event, handle) {
  const steps = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }
  const step = steps[event.key]
  if (!step) return
  event.preventDefault()
  const amount = event.shiftKey ? 0.01 : 0.001
  cropDraft.value = dragCrop(cropDraft.value, handle, step[0] * amount, step[1] * amount, ...cropMin)
}
function applyCrop() {
  const image = images.value.find((image) => image.path === cropPath)
  if (image) {
    const [left, top, right, bottom] = cropDraft.value
    image.trim = [left * 100, top * 100, (1 - right) * 100, (1 - bottom) * 100]
  }
  cropVisible.value = false
}
const previews = new Map()
let pendingResult = null
let resolver,
  timer,
  revision = 0,
  drawing = 0,
  outputDir = null
const name = (path) => path.split(/[/\\]/).at(-1)
function args() {
  return {
    ...settings,
    rows: 1,
    cols: 1,
    page_layouts: pages.value.map((sheet) => ({
      count: sheet.images.length,
      rows: layouts[sheet.id]?.rows || 1,
      cols: layouts[sheet.id]?.cols || 1,
    })),
    output_dir: outputDir,
    images: images.value.map((image) => ({
      path: image.path,
      rotation: image.rotation,
      scale: image.percent / 100,
      reference_width: image.referenceWidth,
      crop: [image.trim[0] / 100, image.trim[1] / 100, 1 - image.trim[2] / 100, 1 - image.trim[3] / 100],
    })),
  }
}
function handleClosed() {
  const result = pendingResult
  pendingResult = null
  finish(result)
}
function finish(value) {
  if (!resolver) return
  const resolve = resolver
  resolver = null
  cropVisible.value = false
  cropSrc.value = ''
  cropDrag = null
  revision++
  drawing++
  clearTimeout(timer)
  previews.clear()
  images.value = []
  plan.value = null
  resolve(value)
}
function open(paths, options = {}) {
  if (resolver) return Promise.resolve(null)
  pendingResult = null
  outputDir = options.outputDir || null
  images.value = paths.map((path) => ({
    path,
    pageId: `image-page-${++pageSequence}`,
    rotation: 0,
    percent: 100,
    referenceWidth: null,
    trim: [0, 0, 0, 0],
  }))
  Object.keys(layouts).forEach((key) => delete layouts[key])
  Object.assign(settings, { landscape: false, margin: 10, gap: 4 })
  syncLayouts()
  selected.value = 0
  page.value = 0
  plan.value = null
  error.value = ''
  loading.value = true
  visible.value = true
  const result = new Promise((resolve) => {
    resolver = resolve
  })
  schedule()
  return result
}
function pageOf(id) {
  return pages.value.findIndex((sheet) => sheet.id === id)
}
function pageImages(id) {
  return pages.value.find((sheet) => sheet.id === id)?.images || []
}
function isPageStart(index) {
  return index === 0 || images.value[index - 1].pageId !== images.value[index].pageId
}
function select(index) {
  selected.value = index
  page.value = Math.max(0, pageOf(images.value[index]?.pageId))
}
function syncLayouts() {
  for (const sheet of pages.value) {
    if (layouts[sheet.id]?.count === sheet.images.length) continue
    layouts[sheet.id] = { ...autoImageGrid(sheet.images.length, settings.landscape), count: sheet.images.length }
  }
  for (const id of Object.keys(layouts)) if (!pages.value.some((sheet) => sheet.id === id)) delete layouts[id]
}
function layoutChanged(changed) {
  if (!activePage.value) return
  Object.assign(currentLayout.value, fitImageGrid(activePage.value.images.length, currentLayout.value, changed))
  schedule()
}
function move(delta) {
  reorder({ from: selected.value, to: selected.value + delta })
}
function reorder({ from, to }) {
  if (!images.value[from] || !images.value[to]) return
  const path = active.value?.path
  images.value = reorderPageImage(images.value, from, to, `image-page-${++pageSequence}`)
  syncLayouts()
  select(images.value.findIndex((image) => image.path === path))
}
function joinPage({ from, target }) {
  const image = images.value[from]
  if (!image || image.pageId === target) return
  if (pageImages(target).length >= 16) {
    error.value = '每页最多 16 张图片，请选择其他页'
    return
  }
  images.value = joinImagePage(images.value, image.path, target)
  syncLayouts()
  select(images.value.findIndex((item) => item.path === image.path))
}
function splitActive() {
  const path = active.value?.path
  images.value = splitImagePage(images.value, path, `image-page-${++pageSequence}`)
  syncLayouts()
  select(images.value.findIndex((image) => image.path === path))
}
function remove(index = selected.value) {
  const path = active.value?.path
  images.value.splice(index, 1)
  syncLayouts()
  if (!images.value.length) {
    visible.value = false
    return
  }
  const kept = images.value.findIndex((image) => image.path === path)
  select(kept < 0 ? Math.min(index, images.value.length - 1) : kept)
}
function fitActive() {
  if (selectedPlacement.value) active.value.percent = fittingPercent(selectedPlacement.value)
}
function fittingPercent(placement) {
  return Math.max(1, Math.min(400, Math.floor(placement.max_scale * 100)))
}
function fitPage() {
  for (const placement of plan.value?.pages[page.value] || []) {
    images.value[placement.index].percent = fittingPercent(placement)
  }
}
function resetActive() {
  active.value.trim = [0, 0, 0, 0]
  active.value.percent = 100
  active.value.rotation = 0
}
function schedule() {
  if (!visible.value || exporting.value) return
  clearTimeout(timer)
  loading.value = true
  revision++
  timer = setTimeout(updatePlan, 120)
}
async function updatePlan() {
  const current = revision
  const result = await tauriCallQuiet('plan_image_pdf', { args: args() })
  if (current !== revision || !visible.value) return
  if (!result.ok) {
    error.value = result.error
    plan.value = null
    loading.value = false
    return
  }
  // Keep the 100% print-size reference stable across rotation and cropping.
  let anchored = false
  for (const placement of result.data.pages.flat()) {
    const image = images.value[placement.index]
    if (image.referenceWidth == null) {
      image.referenceWidth = placement.reference_width
      anchored = true
    }
  }
  if (anchored) return
  plan.value = result.data
  page.value = Math.min(page.value, pageCount.value - 1)
  error.value = ''
  await nextTick()
  await draw()
  if (current === revision) loading.value = false
}
async function thumbnail(path) {
  if (previews.has(path)) return previews.get(path)
  const result = await tauriCallQuiet('read_image_data_url', { path, maxEdge: 640 })
  if (!result.ok) throw new Error(result.error)
  const image = new window.Image()
  image.src = result.data
  await image.decode()
  if (previews.size >= 24) previews.delete(previews.keys().next().value)
  previews.set(path, image)
  return image
}
async function draw() {
  const current = ++drawing,
    sheet = plan.value,
    target = canvas.value
  if (!target || !sheet || !visible.value) return
  target.width = Math.round(sheet.width * 2.4)
  target.height = Math.round(sheet.height * 2.4)
  const ctx = target.getContext('2d'),
    unit = target.width / sheet.width
  ctx.setTransform(unit, 0, 0, unit, 0, 0)
  ctx.fillStyle = '#fff'
  ctx.fillRect(0, 0, sheet.width, sheet.height)
  ctx.save()
  ctx.strokeStyle = '#d8dce2'
  ctx.lineWidth = 0.25
  ctx.setLineDash([1.5, 1.5])
  ctx.strokeRect(
    settings.margin,
    settings.margin,
    sheet.width - settings.margin * 2,
    sheet.height - settings.margin * 2,
  )
  ctx.restore()
  try {
    for (const item of sheet.pages[page.value] || []) {
      const source = await thumbnail(images.value[item.index].path)
      if (current !== drawing || !visible.value) return
      const [x, y, w, h] = item.crop_px,
        sx = source.naturalWidth / item.source_width,
        sy = source.naturalHeight / item.source_height
      const turned = item.rotation % 180 === 90,
        width = turned ? item.height : item.width,
        height = turned ? item.width : item.height
      ctx.save()
      ctx.translate(item.x + item.width / 2, item.y + item.height / 2)
      ctx.rotate((item.rotation * Math.PI) / 180)
      ctx.drawImage(source, x * sx, y * sy, w * sx, h * sy, -width / 2, -height / 2, width, height)
      ctx.restore()
      if (item.index === selected.value && images.value.length > 1) {
        ctx.strokeStyle = '#5687af'
        ctx.lineWidth = 0.4
        ctx.strokeRect(item.x, item.y, item.width, item.height)
      }
    }
  } catch (err) {
    if (current === drawing) error.value = `预览失败：${err.message}`
  }
}
function selectFromCanvas(event) {
  if (!plan.value || loading.value) return
  const bounds = canvas.value.getBoundingClientRect(),
    x = ((event.clientX - bounds.left) / bounds.width) * plan.value.width,
    y = ((event.clientY - bounds.top) / bounds.height) * plan.value.height
  const found = plan.value.pages[page.value].find(
    (item) => x >= item.x && x <= item.x + item.width && y >= item.y && y <= item.y + item.height,
  )
  if (found) selected.value = found.index
}
async function confirm() {
  exporting.value = true
  const result = await tauriCallSafe('export_image_pdf', { args: args() })
  exporting.value = false
  if (!result.ok) {
    error.value = result.error
    return
  }
  pendingResult = result.data
  visible.value = false
}
watch(
  images,
  () => {
    syncLayouts()
    schedule()
  },
  { deep: true },
)
watch(
  settings,
  () => {
    schedule()
  },
  { deep: true },
)
watch([page, selected, canvas], () => {
  if (!loading.value) void draw()
})
onBeforeUnmount(() => finish(null))
defineExpose({ open })
</script>

<style scoped>
.image-pdf-dialog {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 230px;
  gap: 18px;
  height: min(570px, 66vh);
  min-height: 330px;
}
.image-pdf-dialog.has-list {
  grid-template-columns: 205px minmax(0, 1fr) 230px;
}
.image-list {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: auto;
  border-right: 1px solid var(--el-border-color-lighter);
  padding-right: 12px;
}
.image-list :deep(.queue-summary) {
  display: none;
}
.image-list :deep(.queue-item) {
  gap: 5px;
  padding: 8px 6px;
  flex-wrap: wrap;
}
.image-list :deep(.queue-item-main) {
  flex-basis: 85px;
}
.image-list :deep(.queue-item-actions) {
  flex-shrink: 0;
}
.image-list :deep(.queue-drag-handle) {
  flex-basis: 22px;
  width: 22px;
}
.page-heading {
  flex: 0 0 100%;
  width: 100%;
  margin-bottom: 3px;
}
.page-headline {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
}
.page-select {
  border: 0;
  border-radius: 4px;
  padding: 3px 0;
  background: transparent;
  color: var(--docsy-text-strong);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.page-drop-zone {
  border: 1px dashed var(--docsy-border-strong);
  border-radius: 6px;
  padding: 6px 4px;
  color: var(--docsy-text-muted);
  font-size: 11px;
  text-align: center;
  margin-top: 3px;
}
.page-drop-zone.is-dragging {
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}
.page-heading.is-drop-target .page-drop-zone {
  background: var(--docsy-primary);
  color: white;
}
.split-page-button {
  margin-bottom: 10px;
}
.list-heading {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 10px;
}
.image-row {
  display: flex;
  gap: 8px;
  flex: 0 0 auto;
  width: 100%;
  padding: 9px 8px;
  border: 0;
  border-radius: var(--docsy-radius, 8px);
  text-align: left;
  background: transparent;
  color: var(--el-text-color-regular);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
}
.image-row span:last-child {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.image-row.active {
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}
.list-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: auto;
  padding-top: 12px;
  position: static;
  background: transparent;
}
.preview-column {
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.paper-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 12px;
  padding-bottom: 10px;
}
.paper-size {
  margin-left: auto;
  color: var(--el-text-color-secondary);
}
.paper-area {
  display: flex;
  flex: 1;
  min-height: 0;
  align-items: center;
  justify-content: center;
  background: var(--el-fill-color-light);
  border-radius: var(--docsy-radius, 8px);
  padding: 12px;
  overflow: hidden;
}
.paper-area.is-drop-target {
  outline: 2px dashed var(--docsy-primary);
  outline-offset: -4px;
  background: var(--docsy-primary-soft);
}
.paper {
  display: block;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  box-shadow: 0 2px 12px #00000013;
}
.preview-note {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin: 8px 0 0;
}
.image-options {
  overflow: auto;
  min-width: 0;
  padding-right: 2px;
}
.image-options :deep(.el-form-item) {
  margin-bottom: 10px;
}
.image-options :deep(.el-form-item__label) {
  line-height: 20px;
  height: auto;
  margin-bottom: 3px;
  padding: 0;
}
.two-fields,
.crop-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0 10px;
}
.image-options :deep(.el-input-number) {
  width: 100%;
}
.selected-options {
  border-top: 1px solid var(--el-border-color-lighter);
  padding-top: 10px;
}
.selected-name {
  font-size: 12px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-bottom: 10px;
}
.unit {
  margin: 0 8px 0 6px;
}
.selected-options :deep(.el-form-item__content > .el-input-number) {
  width: 100px;
}
.crop-grid :deep(.el-form-item__content > .el-input-number) {
  width: 100%;
}
.crop-title {
  display: flex;
  align-items: center;
  font-size: 12px;
  margin-bottom: 4px;
}
.crop-title {
  gap: 8px;
}
.crop-title :deep(.el-button + .el-button) {
  margin-left: 0;
}
.dpi-note {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin: 0;
}
.dpi-note.low {
  color: var(--el-color-warning-dark-2);
}
.dialog-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}
.dialog-footer > span {
  margin-right: auto;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.dialog-footer :deep(.el-button + .el-button),
.list-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
.dialog-error {
  margin-top: 10px;
}
.crop-instruction {
  margin: 0 0 12px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}
.crop-stage {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 150px;
  padding: 14px;
  background: var(--el-fill-color-light);
  border-radius: var(--docsy-radius, 8px);
  overflow: hidden;
}
.crop-surface {
  display: inline-block;
  position: relative;
  line-height: 0;
  max-width: 100%;
  overflow: visible;
  touch-action: none;
  user-select: none;
}
.crop-surface img {
  display: block;
  width: auto;
  height: auto;
  max-width: 100%;
  max-height: 58vh;
  pointer-events: none;
}
.crop-selection {
  position: absolute;
  border: 2px solid #fff;
  box-sizing: border-box;
  box-shadow: 0 0 0 1000px #0009;
  cursor: move;
  touch-action: none;
}
.crop-grid-line {
  position: absolute;
  background: #ffffff90;
  pointer-events: none;
}
.crop-grid-line--v1,
.crop-grid-line--v2 {
  top: 0;
  bottom: 0;
  width: 1px;
}
.crop-grid-line--v1 {
  left: 33.333%;
}
.crop-grid-line--v2 {
  left: 66.667%;
}
.crop-grid-line--h1,
.crop-grid-line--h2 {
  left: 0;
  right: 0;
  height: 1px;
}
.crop-grid-line--h1 {
  top: 33.333%;
}
.crop-grid-line--h2 {
  top: 66.667%;
}
.crop-handle {
  position: absolute;
  width: 14px;
  height: 14px;
  padding: 0;
  border: 2px solid var(--docsy-primary);
  border-radius: 3px;
  background: #fff;
  transform: translate(-50%, -50%);
  touch-action: none;
}
.crop-handle--nw {
  left: 0;
  top: 0;
  cursor: nwse-resize;
}
.crop-handle--n {
  left: 50%;
  top: 0;
  cursor: ns-resize;
}
.crop-handle--ne {
  left: 100%;
  top: 0;
  cursor: nesw-resize;
}
.crop-handle--e {
  left: 100%;
  top: 50%;
  cursor: ew-resize;
}
.crop-handle--se {
  left: 100%;
  top: 100%;
  cursor: nwse-resize;
}
.crop-handle--s {
  left: 50%;
  top: 100%;
  cursor: ns-resize;
}
.crop-handle--sw {
  left: 0;
  top: 100%;
  cursor: nesw-resize;
}
.crop-handle--w {
  left: 0;
  top: 50%;
  cursor: ew-resize;
}
@media (max-width: 850px) {
  .image-pdf-dialog.has-list {
    grid-template-columns: 125px minmax(0, 1fr) 190px;
    gap: 10px;
  }
  .image-pdf-dialog {
    grid-template-columns: minmax(0, 1fr) 190px;
    gap: 10px;
  }
}
@media (max-width: 640px) {
  .image-pdf-dialog,
  .image-pdf-dialog.has-list {
    display: flex;
    flex-direction: column;
    height: 68vh;
    overflow: auto;
  }
  .image-list {
    max-height: 110px;
    flex: 0 0 auto;
  }
  .preview-column {
    min-height: 300px;
    flex: 1 0 300px;
  }
  .image-options {
    overflow: visible;
  }
}
</style>
