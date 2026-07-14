<template>
  <div class="video-extract-view">
    <div class="extract-layout">
      <!-- Left: Settings -->
      <div class="extract-settings">
        <!-- FFmpeg Status -->
        <div class="section-block">
          <div class="section-title">FFmpeg 状态</div>
          <div v-if="ffmpegLoading" class="status-row">
            <el-icon class="is-loading"><Loading /></el-icon>
            <span>检测中...</span>
          </div>
          <div v-else-if="ffmpegStatus.available" class="status-row status-ok">
            <el-icon><CircleCheckFilled /></el-icon>
            <span>可用</span>
            <el-tag size="small" type="info">{{ ffmpegStatus.version }}</el-tag>
            <el-tag size="small" :type="ffmpegStatus.has_drawtext ? 'success' : 'warning'">
              {{ ffmpegStatus.has_drawtext ? 'drawtext 可用' : 'drawtext 不可用' }}
            </el-tag>
          </div>
          <div v-else class="status-row status-warn">
            <el-icon><WarningFilled /></el-icon>
            <span>未找到 ffmpeg</span>
            <el-button size="small" type="primary" @click="installFfmpeg" :loading="installing">
              下载安装到 Docsy
            </el-button>
            <el-button size="small" @click="openFfmpegDownload">
              下载页
            </el-button>
          </div>
        </div>

        <!-- File Selection -->
        <div class="section-block">
          <div class="section-title">选择视频</div>
          <div
            class="drop-zone"
            :class="{ 'drop-zone-active': dragging }"
            @dragover.prevent="dragging = true"
            @dragleave="dragging = false"
            @drop.prevent="onDrop"
            @click="selectFile"
          >
            <template v-if="videoPath">
              <div class="selected-file">
                <el-icon><VideoCamera /></el-icon>
                <span class="file-name">{{ baseName(videoPath) }}</span>
                <el-button text type="danger" size="small" @click.stop="clearVideo">清除</el-button>
              </div>
            </template>
            <template v-else>
              <el-icon class="drop-icon"><UploadFilled /></el-icon>
              <p>拖放视频文件到此处，或点击选择</p>
            </template>
          </div>
        </div>

        <!-- Video Info -->
        <div v-if="videoInfo" class="section-block">
          <div class="section-title">视频信息</div>
          <div class="info-grid">
            <div class="info-item">
              <span class="info-label">时长</span>
              <span class="info-value">{{ formatDuration(videoInfo.duration) }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">分辨率</span>
              <span class="info-value">{{ videoInfo.width }} × {{ videoInfo.height }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">帧率</span>
              <span class="info-value">{{ videoInfo.fps?.toFixed(2) }} fps</span>
            </div>
            <div class="info-item">
              <span class="info-label">编码</span>
              <span class="info-value">{{ videoInfo.codec }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">大小</span>
              <span class="info-value">{{ formatSize(videoInfo.size) }}</span>
            </div>
          </div>
        </div>

        <!-- Extraction Settings -->
        <div class="section-block">
          <div class="section-title">抽帧设置</div>
          <el-form label-width="90px" size="default">
            <el-form-item label="输出目录">
              <div class="path-picker">
                <el-button @click="selectOutputDir">选择</el-button>
                <el-button v-if="settings.outputDir" text type="danger" @click="settings.outputDir = ''">清除</el-button>
              </div>
              <div class="path-hint">{{ settings.outputDir || '默认保存到视频所在文件夹' }}</div>
            </el-form-item>

            <el-form-item label="文件名前缀">
              <el-input v-model="settings.filenamePrefix" placeholder="留空则使用 视频名_时间" />
            </el-form-item>

            <el-form-item label="抽帧模式">
              <el-radio-group v-model="settings.mode">
                <el-radio value="fps">按频率</el-radio>
                <el-radio value="interval">按间隔</el-radio>
              </el-radio-group>
            </el-form-item>

            <el-form-item v-if="settings.mode === 'fps'" label="帧/秒">
              <el-input-number v-model="settings.fps" :min="0.1" :max="60" :step="0.5" :precision="1" />
            </el-form-item>

            <el-form-item v-else label="间隔(秒)">
              <el-input-number v-model="settings.interval" :min="0.1" :max="3600" :step="1" :precision="1" />
            </el-form-item>

            <el-form-item label="输出格式">
              <el-select v-model="settings.format">
                <el-option label="JPG" value="jpg" />
                <el-option label="PNG" value="png" />
              </el-select>
            </el-form-item>

            <el-form-item label="质量">
              <el-slider v-model="settings.quality" :min="1" :max="100" :step="1" show-input />
            </el-form-item>

            <el-form-item label="时间戳水印">
              <el-switch v-model="settings.timestamp.enabled" />
            </el-form-item>

            <template v-if="settings.timestamp.enabled">
              <el-form-item label="水印位置">
                <el-select v-model="settings.timestamp.position">
                  <el-option label="左上角" value="top-left" />
                  <el-option label="右上角" value="top-right" />
                  <el-option label="左下角" value="bottom-left" />
                  <el-option label="右下角" value="bottom-right" />
                </el-select>
              </el-form-item>

              <el-form-item label="水印颜色">
                <el-select v-model="settings.timestamp.color">
                  <el-option label="白色" value="white" />
                  <el-option label="红色" value="red" />
                  <el-option label="黄色" value="yellow" />
                  <el-option label="绿色" value="green" />
                  <el-option label="黑色" value="black" />
                </el-select>
              </el-form-item>
            </template>
          </el-form>
        </div>

        <!-- Extract Button -->
        <div class="section-block actions-block">
          <el-button
            type="primary"
            size="large"
            @click="extractFrames"
            :loading="extracting"
            :disabled="!videoPath || !ffmpegStatus.available"
            style="width: 100%"
          >
            开始抽帧
          </el-button>
        </div>
      </div>

      <!-- Right: Results -->
      <div class="extract-results">
        <div class="results-header">
          <span>抽帧结果</span>
          <span v-if="extractResult" class="results-count">
            共 {{ extractResult.count }} 帧 · {{ formatDuration(extractResult.elapsed / 1000) }}
          </span>
        </div>

        <div v-if="extracting" class="results-loading">
          <el-icon class="is-loading" :size="32"><Loading /></el-icon>
          <p>正在抽帧...</p>
        </div>

        <div v-else-if="resultImages.length > 0" class="results-grid">
          <div
            v-for="(img, idx) in resultImages"
            :key="idx"
            class="result-item"
            @click="previewImage(img)"
          >
            <img :src="img.src" :alt="img.name" />
            <div class="result-item-name">{{ img.name }}</div>
          </div>
        </div>

        <el-empty v-else description="选择视频并开始抽帧" :image-size="80" />
      </div>
    </div>

    <!-- Image Preview Dialog -->
    <el-dialog v-model="previewVisible" title="帧预览" width="80%" destroy-on-close>
      <div class="preview-container">
        <img v-if="previewSrc" :src="previewSrc" class="preview-img" />
      </div>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, onBeforeUnmount } from 'vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import {
  Loading,
  CircleCheckFilled,
  WarningFilled,
  VideoCamera,
  UploadFilled,
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const ffmpegLoading = ref(true)
const ffmpegStatus = reactive({ available: false, path: null, version: null, has_drawtext: false })
const installing = ref(false)

const dragging = ref(false)
const videoPath = ref('')
const videoInfo = ref(null)

const settings = reactive({
  outputDir: '',
  filenamePrefix: '',
  mode: 'fps',
  fps: 1.0,
  interval: 1.0,
  format: 'jpg',
  quality: 90,
  timestamp: {
    enabled: false,
    position: 'top-left',
    color: 'white',
  },
})

const extracting = ref(false)
const extractResult = ref(null)
const resultImages = ref([])

const previewVisible = ref(false)
const previewSrc = ref('')
let unlistenDragDrop = null

async function checkFfmpeg() {
  ffmpegLoading.value = true
  const res = await tauriCallSafe('check_ffmpeg')
  if (res.ok) {
    Object.assign(ffmpegStatus, res.data)
  }
  ffmpegLoading.value = false
}

async function installFfmpeg() {
  installing.value = true
  const res = await tauriCallSafe('install_external_tool', { toolName: 'ffmpeg' })
  if (res.ok) {
    ElMessage.success(res.data)
    await checkFfmpeg()
  } else {
    ElMessage.error('安装失败: ' + res.error)
  }
  installing.value = false
}

async function openFfmpegDownload() {
  const res = await tauriCallSafe('open_path', { path: 'https://www.gyan.dev/ffmpeg/builds/' })
  if (!res.ok) {
    ElMessage.error('无法打开 FFmpeg 下载页: ' + res.error)
  }
}

async function selectFile() {
  const file = await open({
    multiple: false,
    filters: [
      { name: '视频文件', extensions: ['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'ts', 'm4v'] },
    ],
  })
  if (file) {
    await loadVideo(normalizeSelectedPath(file))
  }
}

async function selectOutputDir() {
  const selected = await open({ directory: true })
  if (selected) {
    settings.outputDir = normalizeSelectedPath(selected)
  }
}

async function onDrop(e) {
  dragging.value = false
  const files = e.dataTransfer?.files
  if (files?.length && files[0].path) {
    await loadVideo(files[0].path)
  }
}

async function handleDroppedPaths(paths) {
  const first = Array.isArray(paths) ? paths[0] : null
  if (!first) return
  await loadVideo(first)
}

async function loadVideo(path) {
  videoPath.value = path
  videoInfo.value = null
  extractResult.value = null
  resultImages.value = []

  const res = await tauriCallSafe('probe_video', { path })
  if (res.ok) {
    videoInfo.value = res.data
  } else {
    ElMessage.error('无法读取视频信息: ' + res.error)
  }
}

function clearVideo() {
  videoPath.value = ''
  videoInfo.value = null
  extractResult.value = null
  resultImages.value = []
}

async function extractFrames() {
  if (!videoPath.value) return

  extracting.value = true
  extractResult.value = null
  resultImages.value = []

  const args = {
    input: videoPath.value,
    output_dir: settings.outputDir,
    filename_prefix: settings.filenamePrefix,
    fps: settings.mode === 'fps' ? settings.fps : 1.0 / settings.interval,
    format: settings.format,
    quality: settings.quality,
    timestamp: settings.timestamp.enabled
      ? {
          enabled: true,
          position: settings.timestamp.position,
          color: settings.timestamp.color,
        }
      : null,
  }

  const res = await tauriCallSafe('extract_frames', { args })
  extracting.value = false

  if (res.ok) {
    extractResult.value = res.data
    ElMessage.success(`抽帧完成，共 ${res.data.count} 帧`)

    if (res.data.output_dir) {
      await loadResultImages(res.data.output_dir)
    }
  } else {
    ElMessage.error('抽帧失败: ' + res.error)
  }
}

async function loadResultImages(dir) {
  const res = await tauriCallSafe('list_output_frames', { dir })
  if (res.ok && Array.isArray(res.data)) {
    const items = await Promise.all(res.data.map(async (f) => ({
      name: baseName(f),
      src: await imageDataUrl(f),
      path: f,
    })))
    resultImages.value = items
  }
}

function previewImage(img) {
  previewSrc.value = img.src
  previewVisible.value = true
}

async function imageDataUrl(path) {
  const res = await tauriCallSafe('read_image_data_url', { path })
  return res.ok ? res.data : ''
}

function normalizeSelectedPath(value) {
  return Array.isArray(value) ? value[0] : value
}

function baseName(path) {
  return String(path || '').split(/[\\/]/).pop() || path
}

function formatDuration(seconds) {
  if (!seconds && seconds !== 0) return '-'
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  if (m > 0) return `${m}:${String(s).padStart(2, '0')}`
  return `${s}s`
}

function formatSize(bytes) {
  if (!bytes) return '-'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB'
}

onMounted(async () => {
  checkFfmpeg()
  unlistenDragDrop = await getCurrentWebview().onDragDropEvent(async (event) => {
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
      dragging.value = true
    } else if (event.payload.type === 'drop') {
      dragging.value = false
      await handleDroppedPaths(event.payload.paths)
    } else {
      dragging.value = false
    }
  })
})

onBeforeUnmount(() => {
  if (unlistenDragDrop) {
    unlistenDragDrop()
  }
})
</script>

<style scoped>
.video-extract-view {
  height: 100%;
  overflow: hidden;
}

.extract-layout {
  display: flex;
  height: 100%;
  gap: 16px;
}

.extract-settings {
  width: 380px;
  flex-shrink: 0;
  overflow-y: auto;
  padding: 16px;
  border-right: 1px solid #e4e7ed;
}

.extract-results {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.results-header {
  padding: 12px 16px;
  border-bottom: 1px solid #e4e7ed;
  font-weight: 600;
  font-size: 14px;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 12px;
}

.results-count {
  font-weight: 400;
  font-size: 12px;
  color: #909399;
}

.results-loading {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #909399;
}

.results-loading p {
  margin-top: 12px;
  font-size: 13px;
}

.results-grid {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
  align-content: start;
}

.result-item {
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  overflow: hidden;
  cursor: pointer;
  transition: box-shadow 0.2s;
}

.result-item:hover {
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.result-item img {
  width: 100%;
  height: 120px;
  object-fit: cover;
  display: block;
}

.result-item-name {
  padding: 6px 8px;
  font-size: 11px;
  color: #606266;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.section-block {
  margin-bottom: 16px;
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 10px;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #606266;
}

.status-ok {
  color: #67c23a;
}

.status-warn {
  color: #e6a23c;
}

.drop-zone {
  border: 2px dashed #dcdfe6;
  border-radius: 8px;
  padding: 24px 16px;
  text-align: center;
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s;
  color: #909399;
  font-size: 13px;
}

.drop-zone:hover,
.drop-zone-active {
  border-color: #409eff;
  background: #ecf5ff;
}

.drop-icon {
  font-size: 32px;
  margin-bottom: 8px;
  color: #c0c4cc;
}

.selected-file {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #303133;
}

.file-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.info-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.info-label {
  font-size: 11px;
  color: #909399;
}

.info-value {
  font-size: 13px;
  color: #303133;
  font-weight: 500;
}

.actions-block {
  padding-top: 8px;
}

.preview-container {
  display: flex;
  justify-content: center;
  align-items: center;
  max-height: 70vh;
}

.preview-img {
  max-width: 100%;
  max-height: 70vh;
  object-fit: contain;
}

.path-picker {
  display: flex;
  gap: 8px;
  align-items: center;
}

.path-hint {
  width: 100%;
  margin-top: 4px;
  color: #909399;
  font-size: 12px;
  word-break: break-all;
}
</style>
