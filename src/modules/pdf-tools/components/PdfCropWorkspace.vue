<template>
  <ToolWorkspaceShell
    title="裁剪 PDF 页面"
    description="拖动边框选择保留区域，按当前页、全部页或指定页裁剪。生成新文件，原文件保持不变。"
    :hide-toolbar="!file"
  >
    <template #toolbar>
      <el-button type="primary" plain :disabled="busy" @click="selectFile">选择 PDF 文件</el-button>
      <el-button :disabled="!file || busy" @click="selectDirectory">{{
        outputDir ? '更改输出目录' : '选择输出目录'
      }}</el-button>
    </template>
    <WorkspaceEmptyState
      v-if="!file"
      class="crop-empty"
      scene="pdf-crop"
      title="选择需要裁剪的 PDF"
      description="拖动边框选择保留区域，可应用到当前页或指定页面。"
      ><template #actions
        ><el-button type="primary" plain @click="selectFile">选择 PDF 文件</el-button></template
      ></WorkspaceEmptyState
    >
    <div v-else class="crop-workspace">
      <div class="crop-options">
        <div class="crop-path" :title="file">{{ fileName(file) }}</div>
        <div class="crop-controls">
          <el-button-group
            ><el-button :disabled="busy || page <= 1" @click="page--">上一页</el-button
            ><el-button :disabled="busy || page >= total" @click="page++">下一页</el-button></el-button-group
          >
          <el-input-number v-model="page" :disabled="busy" :min="1" :max="total || 1" controls-position="right" /><span
            >/ {{ total }} 页</span
          >
          <el-button :disabled="busy" @click="box = [0, 0, 1, 1]">恢复整页</el-button>
        </div>
        <div class="crop-controls">
          <span>应用到</span
          ><el-select v-model="scope" :disabled="busy" style="width: 150px"
            ><el-option label="当前页" value="current" /><el-option label="全部页" value="all" /><el-option
              label="奇数页"
              value="odd" /><el-option label="偶数页" value="even" /><el-option
              label="指定页码"
              value="custom" /></el-select
          ><el-input
            v-if="scope === 'custom'"
            v-model="pageText"
            :disabled="busy"
            placeholder="例如 1,3,5-8"
            style="width: 200px"
          />
        </div>
        <p>多页按各自页面比例裁剪，请翻页核对。裁剪仅隐藏框外内容，不用于删除敏感信息；修改后数字签名可能失效。</p>
      </div>
      <div class="crop-preview">
        <PdfJsPreview :file-path="file" :page="page" compact
          ><template #default
            ><div ref="surface" class="crop-surface" @pointermove="move" @pointerup="end" @pointercancel="end">
              <div class="selection" :style="selectionStyle" @pointerdown.stop.prevent="start($event, 'move')">
                <button
                  v-for="h in handles"
                  :key="h"
                  type="button"
                  :aria-label="`调整裁剪${labels[h]}`"
                  :class="['handle', h]"
                  @pointerdown.stop.prevent="start($event, h)"
                  @keydown="nudge($event, h)"
                />
              </div></div></template
        ></PdfJsPreview>
      </div>
    </div>
    <template #actions>
      <el-button type="success" :loading="busy" :disabled="!file || !total" @click="exportPdf">导出裁剪 PDF</el-button>
      <el-button v-if="output" @click="openPath(output)">打开文件</el-button>
      <FileContinueMenu v-if="output" :paths="[output]" source="PDF 页面裁剪" exclude="pdf-crop" />
    </template>
  </ToolWorkspaceShell>
</template>
<script setup>
import { computed, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import FileContinueMenu from '../../../shared/components/FileContinueMenu.vue'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import PdfJsPreview from '../../../shared/pdf-tools/components/PdfJsPreview.vue'
import { dragCrop } from '../../../shared/pdf-tools/imageCrop.js'
import { parsePageSelection } from '../../../core/pdfUtils.js'
import { fileName, parentDir } from '../../../core/filePath.js'
import { getPdfPageCount, openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'

const file = ref(''),
  outputDir = ref(''),
  output = ref(''),
  total = ref(0),
  page = ref(1),
  busy = ref(false)
const scope = ref('current'),
  pageText = ref(''),
  box = ref([0, 0, 1, 1]),
  surface = ref(null)
const handles = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w']
const labels = {
  nw: '左上角',
  n: '上边缘',
  ne: '右上角',
  e: '右边缘',
  se: '右下角',
  s: '下边缘',
  sw: '左下角',
  w: '左边缘',
}
const selectionStyle = computed(() => ({
  left: `${box.value[0] * 100}%`,
  top: `${box.value[1] * 100}%`,
  width: `${(box.value[2] - box.value[0]) * 100}%`,
  height: `${(box.value[3] - box.value[1]) * 100}%`,
}))
let drag = null
watch([box, scope, pageText, page], () => {
  output.value = ''
})
let loadRequest = 0
async function loadFile(path) {
  if (!path || busy.value) return false
  const request = ++loadRequest
  const result = await getPdfPageCount(path)
  if (request !== loadRequest) return false
  if (!result.ok || Number(result.data) < 1) {
    ElMessage.error(userFacingError(result.error, '读取页数失败，已保留当前裁剪设置'))
    return false
  }
  file.value = path
  output.value = ''
  outputDir.value = parentDir(path)
  total.value = Number(result.data)
  page.value = 1
  box.value = [0, 0, 1, 1]
  return true
}
async function selectFile() {
  const p = await open({ filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (p) await loadFile(Array.isArray(p) ? p[0] : p)
}
async function selectDirectory() {
  const p = await open({ directory: true })
  if (p) outputDir.value = Array.isArray(p) ? p[0] : p
}
function start(e, handle) {
  if (busy.value || e.button !== 0 || !surface.value) return
  surface.value.setPointerCapture(e.pointerId)
  drag = {
    id: e.pointerId,
    handle,
    x: e.clientX,
    y: e.clientY,
    bounds: surface.value.getBoundingClientRect(),
    box: [...box.value],
  }
}
function move(e) {
  if (!drag || drag.id !== e.pointerId) return
  box.value = dragCrop(
    drag.box,
    drag.handle,
    (e.clientX - drag.x) / drag.bounds.width,
    (e.clientY - drag.y) / drag.bounds.height,
  )
}
function end() {
  drag = null
}
function nudge(e, handle) {
  const steps = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }
  if (busy.value || !steps[e.key]) return
  e.preventDefault()
  const [x, y] = steps[e.key]
  const n = e.shiftKey ? 0.01 : 0.001
  box.value = dragCrop(box.value, handle, x * n, y * n)
}
async function exportPdf() {
  let pages
  if (scope.value === 'current') pages = [page.value]
  else if (scope.value === 'custom') pages = parsePageSelection(pageText.value, total.value)
  else
    pages = Array.from({ length: total.value }, (_, i) => i + 1).filter(
      (p) => scope.value === 'all' || (scope.value === 'odd' ? p % 2 === 1 : p % 2 === 0),
    )
  if (!pages.length) {
    ElMessage.warning('请选择有效的页面范围')
    return
  }
  busy.value = true
  try {
    const [left, top, right, bottom] = box.value
    const r = await tauriCallSafe('crop_pdf_pages', {
      args: { inputPath: file.value, outputDir: outputDir.value, pages, left, top, right, bottom },
    })
    if (r.ok) {
      output.value = r.data
      ElMessage.success('已生成裁剪文件')
    } else ElMessage.error(userFacingError(r.error, '裁剪失败'))
  } finally {
    busy.value = false
  }
}
defineExpose({ loadFile, getPath: () => file.value, isBusy: () => busy.value })
</script>
<style scoped>
.crop-empty {
  flex: 1;
  padding: 40px;
  text-align: center;
  color: var(--docsy-text-muted);
}
.crop-workspace {
  display: flex;
  gap: 16px;
  height: 100%;
  min-height: 0;
  padding-top: 14px;
}
.crop-options {
  flex: 0 1 310px;
  min-width: 220px;
}
.crop-path {
  overflow-wrap: anywhere;
  font-weight: 600;
}
.crop-controls {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}
.crop-controls :deep(.el-button + .el-button) {
  margin-left: 0;
}
.crop-options p {
  font-size: 12px;
  line-height: 1.6;
  color: var(--docsy-text-muted);
}
.crop-preview {
  flex: 1;
  min-width: 0;
  overflow: auto;
}
.crop-preview :deep(.preview-stage) {
  min-height: 0;
}
.crop-preview :deep(.page-preview) {
  max-width: 680px;
  margin: auto;
}
.crop-surface {
  position: absolute;
  inset: 0;
  pointer-events: auto;
  touch-action: none;
}
.selection {
  position: absolute;
  border: 2px solid var(--docsy-primary);
  box-shadow: 0 0 0 9999px #0005;
  cursor: move;
  box-sizing: border-box;
}
.handle {
  position: absolute;
  width: 14px;
  height: 14px;
  background: white;
  border: 2px solid var(--docsy-primary);
  border-radius: 50%;
  padding: 0;
  touch-action: none;
}
.nw {
  top: -8px;
  left: -8px;
  cursor: nwse-resize;
}
.n {
  top: -8px;
  left: 50%;
  cursor: ns-resize;
}
.ne {
  top: -8px;
  right: -8px;
  cursor: nesw-resize;
}
.e {
  right: -8px;
  top: 50%;
  cursor: ew-resize;
}
.se {
  right: -8px;
  bottom: -8px;
  cursor: nwse-resize;
}
.s {
  bottom: -8px;
  left: 50%;
  cursor: ns-resize;
}
.sw {
  bottom: -8px;
  left: -8px;
  cursor: nesw-resize;
}
.w {
  left: -8px;
  top: 50%;
  cursor: ew-resize;
}
@media (max-width: 850px) {
  .crop-workspace {
    flex-direction: column;
    overflow: auto;
  }
  .crop-options {
    flex: auto;
  }
  .crop-preview {
    overflow: visible;
  }
}
</style>
