<template>
  <ToolWorkspaceShell title="视频抽帧" description="从视频中按频率或间隔提取帧图片">
    <div class="extract-layout">
      <!-- Left: Settings -->
      <div class="extract-settings">
        <!-- FFmpeg Status -->
        <div class="section-block">
          <div class="section-title">FFmpeg 状态</div>
          <div v-if="ffmpegLoading" v-loading="true" element-loading-text="检测中..." class="status-row status-loading"></div>
          <div v-else-if="ffmpegStatus.available" class="status-row status-ok">
            <el-icon><CircleCheckFilled /></el-icon>
            <span>可用</span>
            <el-tag v-if="ffmpegStatus.version" size="small" type="info" :title="ffmpegStatus.version">
              FFmpeg {{ shortFfmpegVersion(ffmpegStatus.version) }}
            </el-tag>
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
            <el-button size="small" @click="openFfmpegDownload"> 下载页 </el-button>
          </div>
        </div>

        <!-- File Selection -->
        <div class="section-block">
          <div class="section-title">选择视频</div>
          <div
            class="drop-zone"
            :class="{ 'drop-zone-active': dragging }"
            @click="selectFile"
          >
            <template v-if="videoPath">
              <div class="selected-file">
                <el-icon><VideoCamera /></el-icon>
                <span class="file-name">{{ fileName(videoPath) }}</span>
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
                <el-button v-if="settings.outputDir" text type="danger" @click="settings.outputDir = ''"
                  >清除</el-button
                >
              </div>
              <div class="path-hint">{{ settings.outputDir || '默认保存到视频所在文件夹' }}</div>
            </el-form-item>

            <el-form-item label="文件名前缀">
              <el-input
                v-model="settings.filenamePrefix"
                placeholder="留空则使用视频名，输出为 视频名_时间_frame_序号"
              />
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

            <el-form-item label="时间范围">
              <div class="time-range-inputs">
                <el-input v-model="settings.startTime" placeholder="开始，如 01:30:00" clearable />
                <span>至</span>
                <el-input v-model="settings.endTime" placeholder="结束，如 01:35:00" clearable />
              </div>
              <div class="path-hint">留空则处理全片；也可直接输入秒数。</div>
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
              <el-alert
                v-if="!ffmpegStatus.has_drawtext"
                type="warning"
                :closable="false"
                show-icon
                title="当前 FFmpeg 不支持时间戳水印"
                description="请关闭水印，或安装支持 drawtext 的 FFmpeg 后重试。"
              />
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
            :disabled="
              !videoPath || !ffmpegStatus.available || (settings.timestamp.enabled && !ffmpegStatus.has_drawtext)
            "
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

        <ReorderableImageGrid
          v-else-if="resultImages.length > 0"
          class="results-preview"
          :items="resultImages"
          :name-resolver="frameName"
          empty-description="暂无抽帧结果"
          @reorder="reorderResultImages"
        />

        <el-empty v-else description="选择视频并开始抽帧" :image-size="80" />
      </div>
    </div>
  </ToolWorkspaceShell>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { openExternalUrl, tauriCallSafe } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { Loading, CircleCheckFilled, WarningFilled, VideoCamera, UploadFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import ReorderableImageGrid from '../../../shared/components/ReorderableImageGrid.vue'
import { moveItem } from '../../../shared/components/reorderableItems.js'
import { fileName } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'

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
  startTime: '',
  endTime: '',
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
const VIDEO_EXTENSIONS = new Set(['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'ts', 'm4v'])

function shortFfmpegVersion(value) {
  const text = String(value || '').trim()
  const version = text.match(/ffmpeg\s+version\s+(\d+(?:\.\d+){1,3})/i)
  if (version) return version[1]
  return text.replace(/^ffmpeg\s+version\s+/i, '').split(/\s+/)[0] || '可用'
}

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
  const res = await openExternalUrl('https://www.gyan.dev/ffmpeg/builds/')
  if (!res.ok) {
    ElMessage.error('无法打开 FFmpeg 下载页: ' + res.error)
  }
}

async function selectFile() {
  const file = await open({
    multiple: false,
    filters: [{ name: '视频文件', extensions: ['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'ts', 'm4v'] }],
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

async function handleDroppedPaths(paths) {
  const first = Array.isArray(paths) ? paths[0] : null
  if (!first) return
  await loadVideoIfSupported(first)
}

async function loadVideoIfSupported(path) {
  if (!isVideoPath(path)) {
    ElMessage.warning('请拖入支持的视频文件')
    return
  }
  await loadVideo(path)
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
    ElMessage.error('无法读取视频信息: ' + (res.error || '请确认文件是有效的视频格式'))
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
  if (settings.timestamp.enabled && !ffmpegStatus.has_drawtext) {
    ElMessage.error('当前 FFmpeg 不支持 drawtext。请关闭水印或安装支持 drawtext 的 FFmpeg。')
    return
  }

  extracting.value = true
  extractResult.value = null
  resultImages.value = []

  const args = {
    input: videoPath.value,
    output_dir: settings.outputDir,
    filename_prefix: settings.filenamePrefix,
    fps: settings.mode === 'fps' ? settings.fps : 1.0 / settings.interval,
    startTime: settings.startTime,
    endTime: settings.endTime,
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

    if (Array.isArray(res.data.frames)) {
      resultImages.value = res.data.frames.map((path) => ({ path }))
    } else if (res.data.output_dir) {
      await loadResultImages(res.data.output_dir)
    }
  } else {
    ElMessage.error('视频抽帧失败: ' + (res.error || '请确认 FFmpeg 可用且磁盘空间充足'))
  }
}

async function loadResultImages(dir) {
  const res = await tauriCallSafe('list_output_frames', { dir })
  if (res.ok && Array.isArray(res.data)) {
    resultImages.value = res.data.map((path) => ({ path }))
  }
}

function frameName(img) {
  return fileName(img?.path || '')
}

function reorderResultImages({ from, to }) {
  resultImages.value = moveItem(resultImages.value, from, to)
}

function normalizeSelectedPath(value) {
  return Array.isArray(value) ? value[0] : value
}

function isVideoPath(path) {
  const ext = String(path || '')
    .split(/[\\/]/)
    .pop()
    ?.split('.')
    .pop()
    ?.toLowerCase()
  return Boolean(ext && VIDEO_EXTENSIONS.has(ext))
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

onMounted(() => {
  checkFfmpeg()
})

useWindowFileDrop({
  onEnter: () => {
    dragging.value = true
  },
  onLeave: () => {
    dragging.value = false
  },
  onDrop: handleDroppedPaths,
})
</script>

<style scoped>
:deep(.workspace-content) {
  overflow: hidden;
}

.extract-layout {
  display: grid;
  grid-template-columns: 360px minmax(0, 1fr);
  height: 100%;
  min-height: 0;
}

.extract-settings {
  min-width: 0;
  overflow-y: auto;
  padding: 20px;
  border-right: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface);
}

.extract-results {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--docsy-canvas);
}

.results-header {
  padding: 12px 16px;
  min-height: 52px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface);
  font-weight: 600;
  font-size: 14px;
  color: var(--docsy-text-strong);
  display: flex;
  align-items: center;
  gap: 12px;
}

.results-count {
  font-weight: 400;
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.results-loading {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--docsy-text-muted);
}

.results-loading p {
  margin-top: 12px;
  font-size: 13px;
}

.results-preview {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 16px;
  overflow: hidden;
}

.results-preview :deep(.reorder-image-scroll) {
  flex: 1;
  min-height: 0;
  max-height: none;
}

.section-block {
  margin-bottom: 18px;
  padding-bottom: 18px;
  border-bottom: 1px solid var(--docsy-border-subtle);
}

.section-block:last-child {
  border-bottom: 0;
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--docsy-text-strong);
  margin-bottom: 10px;
}

.status-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  min-width: 0;
  font-size: 13px;
  color: var(--docsy-text);
}

.status-ok {
  color: #67c23a;
}

.status-warn {
  color: #e6a23c;
}

.drop-zone {
  border: 1px dashed var(--docsy-border-strong);
  border-radius: 6px;
  padding: 24px 16px;
  text-align: center;
  cursor: pointer;
  transition:
    border-color 0.2s,
    background 0.2s;
  color: var(--docsy-text-muted);
  font-size: 13px;
}

.drop-zone:hover,
.drop-zone-active {
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}

.drop-icon {
  font-size: 32px;
  margin-bottom: 8px;
  color: var(--docsy-text-muted);
}

.selected-file {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--docsy-text-strong);
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
  color: var(--docsy-text-muted);
}

.info-value {
  font-size: 13px;
  color: var(--docsy-text-strong);
  font-weight: 500;
}

.actions-block {
  padding-top: 8px;
}

.path-picker {
  display: flex;
  gap: 8px;
  align-items: center;
}

.time-range-inputs {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 8px;
  width: 100%;
}

.path-hint {
  width: 100%;
  margin-top: 4px;
  color: var(--docsy-text-muted);
  font-size: 12px;
  word-break: break-all;
}

@media (max-width: 1180px) {
  :deep(.workspace-content) {
    overflow: auto;
  }

  .extract-layout {
    display: block;
    height: auto;
  }

  .extract-settings {
    overflow: visible;
    border-right: 0;
    border-bottom: 1px solid var(--docsy-border-subtle);
  }

  .extract-results {
    min-height: 560px;
  }
}
</style>
