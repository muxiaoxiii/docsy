<template>
  <ToolWorkspaceShell title="视频抽帧" description="从视频中按频率或间隔提取帧图片">
    <div class="extract-layout" :class="{ 'settings-collapsed': settingsCollapsed }">
      <!-- Left: Settings -->
      <div class="extract-settings">
        <!-- FFmpeg Status -->
        <div class="section-block">
          <div class="section-title">FFmpeg 状态</div>
          <div
            v-if="ffmpegLoading"
            v-loading="true"
            element-loading-text="检测中..."
            class="status-row status-loading"
          ></div>
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
              {{ isMac ? '终端安装 (Homebrew)' : '下载安装到 Docsy' }}
            </el-button>
            <el-button size="small" @click="openFfmpegDownload"> 下载页 </el-button>
          </div>
        </div>

        <!-- File Selection -->
        <div class="section-block">
          <div class="section-title">选择视频</div>
          <div class="drop-zone" :class="{ 'drop-zone-active': dragging }" @click="selectFile">
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

        <div class="section-block">
          <div class="section-title">智能筛选</div>
          <el-form label-width="90px" size="default">
            <el-form-item label="完成后分析">
              <el-switch v-model="analysisSettings.autoAnalyze" />
            </el-form-item>
            <el-form-item label="目标重合">
              <div class="overlap-targets">
                <el-input-number
                  v-model="analysisSettings.overlapMinPercent"
                  :min="2"
                  :max="30"
                  :step="1"
                  controls-position="right"
                />
                <span>至</span>
                <el-input-number
                  v-model="analysisSettings.overlapMaxPercent"
                  :min="2"
                  :max="30"
                  :step="1"
                  controls-position="right"
                />
                <span>%</span>
              </div>
            </el-form-item>
            <el-form-item label="重复判定">
              <el-slider v-model="analysisSettings.duplicatePercent" :min="90" :max="99" :step="1" show-input />
            </el-form-item>
          </el-form>
          <div class="engine-note">
            内置轻量引擎综合颜色、结构、位移、局部变化与清晰度，不依赖 OCR；原图片不会被删除。
          </div>
        </div>

        <!-- Extract Button -->
        <div class="section-block actions-block">
          <el-button
            type="success"
            size="large"
            class="primary-workspace-action"
            @click="extractFrames"
            :loading="extracting"
            :disabled="
              !videoPath || !ffmpegStatus.available || (settings.timestamp.enabled && !ffmpegStatus.has_drawtext)
            "
          >
            开始抽帧
          </el-button>
        </div>
      </div>

      <!-- Right: Results -->
      <div class="extract-results">
        <div class="results-header">
          <span>抽帧结果</span>
          <span v-if="resultImages.length" class="results-count">
            共 {{ resultImages.length }} 帧
            <template v-if="extractResult"> · {{ formatDuration(extractResult.elapsed / 1000) }}</template>
          </span>
          <span class="results-header-spacer"></span>
          <el-tag v-if="algorithmVersion" size="small" type="info">
            {{ algorithmVersion.includes('opencv') ? '本地多信号 + OpenCV 增强' : '本地多信号引擎' }}
          </el-tag>
          <el-button v-if="resultImages.length" size="small" @click="settingsCollapsed = !settingsCollapsed">
            {{ settingsCollapsed ? '显示抽帧设置' : '收起抽帧设置' }}
          </el-button>
          <el-button size="small" @click="selectExistingResultDirectory">打开已有图片目录</el-button>
        </div>

        <WorkspaceEmptyState
          v-if="extracting"
          class="result-empty-state"
          state="loading"
          :icon-url="videoFramesIconUrl"
          title="正在抽帧"
          description="Doclet 正在生成图片，完成后会在这里按顺序显示。"
        />

        <FrameSelectionWorkbench
          v-else-if="resultImages.length > 0"
          class="results-preview"
          :items="resultImages"
          :analyzing="analyzingSelection"
          :analyzed="Boolean(algorithmVersion)"
          :analysis-progress="analysisProgressText"
          :can-undo="decisionHistory.length > 0"
          @update-decision="updateFrameDecision"
          @reorder="reorderResultImages"
          @undo="undoLastAction"
          @run-analysis="runFrameAnalysis"
          @accept-suggestions="acceptSuggestions"
          @send-to-layout="sendToImageLayout"
        />

        <WorkspaceEmptyState
          v-else
          class="result-empty-state"
          :icon-url="videoFramesIconUrl"
          title="等待抽帧结果"
          description="选择视频并设置抽帧参数后，生成的图片会集中显示在这里。"
        />
      </div>
    </div>
  </ToolWorkspaceShell>
</template>

<script setup>
import { ref, reactive, onBeforeUnmount, onDeactivated, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { openExternalUrl, tauriCallQuiet, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { listen } from '@tauri-apps/api/event'
import { CircleCheckFilled, WarningFilled, VideoCamera, UploadFilled } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import FrameSelectionWorkbench from '../components/FrameSelectionWorkbench.vue'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'
import videoFramesIconUrl from '../../../assets/icons/video-frames.svg?url'
import { moveItem } from '../../../shared/components/reorderableItems.js'
import { fileName, parentDir } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'
import { useWorkspaceStore } from '../../../stores/workspace.js'
import { isMac, installToolViaTerminal, openToolDownloadWithGuide } from '../../../core/terminalInstall.js'

const router = useRouter()
const workspaceStore = useWorkspaceStore()
const EXTRACT_OPERATION_ID = 'extract_frames:video-view'
const ANALYZE_OPERATION_ID = 'analyze_frame_selection:auto'

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
const analysisSettings = reactive({
  autoAnalyze: true,
  duplicatePercent: 97,
  scenePercent: 55,
  overlapMinPercent: 5,
  overlapMaxPercent: 10,
})
const settingsCollapsed = ref(false)
const preference = useWorkspacePreferences('video-extract.workspace', {
  settings,
  analysisSettings,
})

const extracting = ref(false)
const analyzingSelection = ref(false)
const analysisProgressText = ref('')
const extractResult = ref(null)
const resultImages = ref([])
const algorithmVersion = ref('')
const sourceDirectory = ref('')
const mediaSessionId = ref(null)
const decisionHistory = ref([])
let mediaSaveTimer = null
let dynamicAnalysisTimer = null
let analysisRerunRequested = false
let analysisRequestSequence = 0
let mediaReadyToSave = false
let unlistenAnalysisProgress = null
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
  if (isMac) {
    await installToolViaTerminal('ffmpeg', 'FFmpeg')
    return
  }
  installing.value = true
  const res = await tauriCallSafe('install_external_tool', { toolName: 'ffmpeg' })
  if (res.ok) {
    ElMessage.success(res.data)
    await checkFfmpeg()
  } else {
    ElMessage.error(userFacingError(res.error, '安装失败'))
  }
  installing.value = false
}

async function openFfmpegDownload() {
  await openToolDownloadWithGuide('ffmpeg')
}

async function selectFile() {
  try {
    const file = await open({
      multiple: false,
      filters: [{ name: '视频文件', extensions: ['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'ts', 'm4v'] }],
    })
    const path = normalizeSelectedPath(file)
    if (path) await loadVideo(path)
  } catch (error) {
    ElMessage.error(userFacingError(error, '无法打开视频文件'))
  }
}

async function selectOutputDir() {
  const selected = await open({ directory: true })
  if (selected) {
    settings.outputDir = normalizeSelectedPath(selected)
  }
}

async function handleDroppedPaths(paths) {
  const candidates = Array.isArray(paths) ? paths : []
  const video = candidates.find(isVideoPath)
  if (!video) {
    if (candidates.length) ElMessage.warning('拖入的文件中没有支持的视频')
    return
  }
  await loadVideo(video)
}

async function loadVideo(path) {
  settingsCollapsed.value = false
  videoPath.value = path
  clearFrameDraftSafely()
  videoInfo.value = null
  extractResult.value = null
  resultImages.value = []
  algorithmVersion.value = ''
  analysisProgressText.value = ''
  decisionHistory.value = []
  sourceDirectory.value = ''
  mediaSessionId.value = null
  mediaReadyToSave = false

  const res = await tauriCallSafe('probe_video', { path })
  if (res.ok) {
    videoInfo.value = res.data
  } else {
    ElMessage.error(userFacingError(res.error, '无法读取视频信息，请确认文件是有效的视频格式'))
  }
}

function clearVideo() {
  settingsCollapsed.value = false
  clearFrameDraftSafely()
  videoPath.value = ''
  videoInfo.value = null
  extractResult.value = null
  resultImages.value = []
  algorithmVersion.value = ''
  analysisProgressText.value = ''
  decisionHistory.value = []
  sourceDirectory.value = ''
  mediaSessionId.value = null
  mediaReadyToSave = false
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
  algorithmVersion.value = ''
  analysisProgressText.value = ''
  decisionHistory.value = []
  mediaReadyToSave = false

  const args = {
    operation_id: EXTRACT_OPERATION_ID,
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
      await initializeResultImages(res.data.frames, res.data.output_dir)
    } else if (res.data.output_dir) {
      await loadResultImages(res.data.output_dir)
    }
  } else {
    ElMessage.error(userFacingError(res.error, '视频抽帧失败，请确认 FFmpeg 可用且磁盘空间充足'))
  }
}

async function loadResultImages(dir) {
  const res = await tauriCallSafe('list_output_frames', { dir })
  if (res.ok && Array.isArray(res.data)) {
    await initializeResultImages(res.data, dir)
  } else if (!res.ok) {
    ElMessage.error(userFacingError(res.error, '无法读取该图片目录'))
  }
}

async function selectExistingResultDirectory() {
  const selected = await open({ directory: true, multiple: false })
  if (selected) {
    extractResult.value = null
    await loadResultImages(normalizeSelectedPath(selected))
  }
}

function emptyFrame(path) {
  return {
    path,
    engine_decision: 'review',
    user_decision: null,
    relation: null,
    reason: '尚未运行智能分析',
    compared_to: null,
    confidence: 0,
    similarity: 0,
    overlap_ratio: null,
    shift_x: 0,
    shift_y: 0,
    blur_score: 0,
    change_ratio: 0,
    continuity_risk: false,
    enhanced_by: null,
    opencv_match_ratio: null,
    opencv_inlier_ratio: null,
  }
}

async function initializeResultImages(paths, dir) {
  clearFrameDraftSafely()
  resultImages.value = [...new Set(paths || [])].map(emptyFrame)
  algorithmVersion.value = ''
  analysisProgressText.value = ''
  decisionHistory.value = []
  sourceDirectory.value = dir || parentDir(resultImages.value[0]?.path || '')
  mediaSessionId.value = null
  mediaReadyToSave = true
  if (!resultImages.value.length) return
  settingsCollapsed.value = true
  syncFrameDraft()
  scheduleMediaSave()
  if (analysisSettings.autoAnalyze) await runFrameAnalysis({ saveAfter: false })
  await offerSessionRestore()
  syncFrameDraft()
  scheduleMediaSave()
}

async function runFrameAnalysis(options = {}) {
  if (!resultImages.value.length) return
  if (analyzingSelection.value) {
    analysisRerunRequested = true
    return
  }
  analyzingSelection.value = true
  const requestSequence = ++analysisRequestSequence
  const requestedPaths = resultImages.value.map((item) => item.path)
  analysisProgressText.value = options.background
    ? '正在根据人工选择更新后续建议…'
    : `正在准备分析 ${resultImages.value.length} 张图片…`
  const userDecisions = Object.fromEntries(
    resultImages.value
      .filter((item) => item.user_decision === 'keep' || item.user_decision === 'exclude')
      .map((item) => [item.path, item.user_decision]),
  )
  const caller = options.background ? tauriCallQuiet : tauriCallSafe
  const result = await caller('analyze_frame_selection', {
    args: {
      paths: requestedPaths,
      duplicate_threshold: analysisSettings.duplicatePercent / 100,
      scene_threshold: analysisSettings.scenePercent / 100,
      overlap_min: Math.min(analysisSettings.overlapMinPercent, analysisSettings.overlapMaxPercent) / 100,
      overlap_max: Math.max(analysisSettings.overlapMinPercent, analysisSettings.overlapMaxPercent) / 100,
      user_decisions: userDecisions,
    },
  })
  analyzingSelection.value = false
  if (!result.ok) {
    analysisProgressText.value = '智能分析未完成'
    if (!options.background) {
      ElMessage.error(userFacingError(result.error, '智能筛选分析失败，仍可手动选择图片'))
    }
    rerunFrameAnalysisIfNeeded()
    return
  }
  const currentPathsBeforeApply = resultImages.value.map((item) => item.path)
  if (
    requestSequence !== analysisRequestSequence ||
    requestedPaths.length !== currentPathsBeforeApply.length ||
    requestedPaths.some((path, index) => path !== currentPathsBeforeApply[index])
  ) {
    analysisRerunRequested = true
    rerunFrameAnalysisIfNeeded()
    return
  }
  analysisProgressText.value = '智能分析已完成'
  algorithmVersion.value = result.data.algorithm_version || ''
  const analyzed = result.data.items || []
  const currentDecisions = new Map(resultImages.value.map((item) => [item.path, item.user_decision]))
  const comparedPaths = analyzed.map((item) =>
    Number.isInteger(item.compared_to) ? analyzed[item.compared_to]?.path || null : null,
  )
  const currentPaths = resultImages.value.map((item) => item.path)
  const indexByPath = new Map(currentPaths.map((path, index) => [path, index]))
  const analyzedByPath = new Map(
    analyzed.map((item, index) => [
      item.path,
      {
        ...item,
        compared_to: comparedPaths[index] ? (indexByPath.get(comparedPaths[index]) ?? null) : null,
        user_decision: currentDecisions.get(item.path) || null,
      },
    ]),
  )
  resultImages.value = currentPaths.map(
    (path) =>
      analyzedByPath.get(path) || {
        ...emptyFrame(path),
        reason: '图片无法解码，已按安全原则保留供人工核对',
        continuity_risk: true,
        user_decision: currentDecisions.get(path) || null,
      },
  )
  if (result.data.warnings?.length) {
    ElMessage.warning(`有 ${result.data.warnings.length} 张图片无法分析，已保留供人工核对`)
  }
  if (options.saveAfter !== false) scheduleMediaSave()
  rerunFrameAnalysisIfNeeded()
}

function scheduleDynamicAnalysis() {
  if (!resultImages.value.length) return
  if (analyzingSelection.value) {
    analysisRerunRequested = true
    return
  }
  if (!algorithmVersion.value) return
  if (dynamicAnalysisTimer) window.clearTimeout(dynamicAnalysisTimer)
  dynamicAnalysisTimer = window.setTimeout(() => {
    dynamicAnalysisTimer = null
    void runFrameAnalysis({ background: true })
  }, 220)
}

function rerunFrameAnalysisIfNeeded() {
  if (!analysisRerunRequested) return
  analysisRerunRequested = false
  scheduleDynamicAnalysis()
}

async function offerSessionRestore() {
  const result = await tauriCallSafe('find_media_workspace_session', {
    args: {
      module_id: 'video-extract',
      source_path: sourceDirectory.value,
      paths: resultImages.value.map((item) => item.path),
    },
  })
  if (!result.ok || !result.data?.found) return
  const saved = result.data
  const exact = saved.match_kind === 'exact' || saved.match_kind === 'content_exact'
  const summary = exact
    ? `找到一份完整匹配的筛选结果，共 ${saved.matched + saved.moved_or_renamed} 张图片。`
    : `找到一份历史筛选结果：匹配 ${saved.matched} 张，移动或改名 ${saved.moved_or_renamed} 张，新增 ${saved.added} 张，内容变化 ${saved.changed} 张，缺少 ${saved.missing} 张。`
  try {
    await ElMessageBox.confirm(
      `${summary}\n仅恢复内容一致图片的人工决定；新增或已变化图片继续采用本次分析结果。`,
      '恢复上次筛选结果？',
      { confirmButtonText: '恢复结果', cancelButtonText: '使用本次结果', type: exact ? 'info' : 'warning' },
    )
  } catch {
    return
  }
  const freshByPath = new Map(resultImages.value.map((item) => [item.path, item]))
  const restored = []
  for (const item of saved.items || []) {
    const fresh = freshByPath.get(item.path)
    if (!fresh) continue
    const canRestore = item.match_status === 'matched' || item.match_status === 'moved_or_renamed'
    restored.push({
      ...fresh,
      ...(!algorithmVersion.value && canRestore ? item.metrics || {} : {}),
      engine_decision:
        !algorithmVersion.value && canRestore ? item.engine_decision || fresh.engine_decision : fresh.engine_decision,
      reason: !algorithmVersion.value && canRestore ? item.reason || fresh.reason : fresh.reason,
      user_decision: canRestore ? item.user_decision : null,
    })
    freshByPath.delete(item.path)
  }
  restored.push(...freshByPath.values())
  resultImages.value = restored
  mediaSessionId.value = saved.session_id
  ElMessage.success('已恢复上次人工筛选与图片顺序')
  scheduleDynamicAnalysis()
}

function reorderResultImages({ from, to }) {
  decisionHistory.value.push({ type: 'order', paths: resultImages.value.map((item) => item.path) })
  resultImages.value = moveItem(resultImages.value, from, to)
  scheduleDynamicAnalysis()
}

function updateFrameDecision({ index, decision }) {
  const item = resultImages.value[index]
  if (!item) return
  decisionHistory.value.push({
    type: 'decisions',
    values: [{ path: item.path, userDecision: item.user_decision || null }],
  })
  resultImages.value[index] = { ...item, user_decision: decision }
  scheduleDynamicAnalysis()
}

function acceptSuggestions() {
  let count = 0
  const values = []
  resultImages.value = resultImages.value.map((item) => {
    if (!item.user_decision && item.engine_decision === 'exclude') {
      count += 1
      values.push({ path: item.path, userDecision: null })
      return { ...item, user_decision: 'exclude' }
    }
    return item
  })
  if (values.length) decisionHistory.value.push({ type: 'decisions', values })
  if (count) ElMessage.success(`已接受 ${count} 项排除建议`)
  if (values.length) scheduleDynamicAnalysis()
}

function undoLastAction() {
  const action = decisionHistory.value.pop()
  if (!action) return
  if (action.type === 'order') {
    const byPath = new Map(resultImages.value.map((item) => [item.path, item]))
    resultImages.value = action.paths.map((path) => byPath.get(path)).filter(Boolean)
    scheduleDynamicAnalysis()
    return
  }
  const previous = new Map(action.values.map((item) => [item.path, item.userDecision]))
  resultImages.value = resultImages.value.map((item) =>
    previous.has(item.path) ? { ...item, user_decision: previous.get(item.path) } : item,
  )
  scheduleDynamicAnalysis()
}

function effectiveDecision(item) {
  return item.user_decision || item.engine_decision || 'review'
}

async function sendToImageLayout() {
  const selection = resultImages.value.map((item) => ({
    path: item.path,
    decision: effectiveDecision(item),
    reason: item.reason || '',
  }))
  const paths = selection.map((item) => item.path)
  if (!selection.some((item) => item.decision !== 'exclude')) return
  await saveMediaSession()
  workspaceStore.sendImagesToLayout(paths, mediaSessionId.value, {
    sourceKind: 'video-frames',
    sourceLabel: '视频抽帧筛选结果',
    sourceStem: fileName(videoPath.value).replace(/\.[^.]+$/, ''),
    selection,
  })
  await router.push({ name: 'image-paddler' })
}

function frameMetrics(item) {
  return {
    relation: item.relation,
    compared_to: item.compared_to,
    confidence: item.confidence,
    similarity: item.similarity,
    overlap_ratio: item.overlap_ratio,
    shift_x: item.shift_x,
    shift_y: item.shift_y,
    blur_score: item.blur_score,
    change_ratio: item.change_ratio,
    continuity_risk: item.continuity_risk,
    enhanced_by: item.enhanced_by,
    opencv_match_ratio: item.opencv_match_ratio,
    opencv_inlier_ratio: item.opencv_inlier_ratio,
    algorithm_version: algorithmVersion.value,
  }
}

function scheduleMediaSave() {
  if (!mediaReadyToSave || !sourceDirectory.value || !resultImages.value.length) return
  if (mediaSaveTimer) window.clearTimeout(mediaSaveTimer)
  mediaSaveTimer = window.setTimeout(saveMediaSession, 650)
}

function syncFrameDraft() {
  if (!mediaReadyToSave || !resultImages.value.length) return
  workspaceStore.setFrameSelectionDraft?.({
    items: resultImages.value,
    sourceDirectory: sourceDirectory.value,
    sessionId: mediaSessionId.value,
    algorithmVersion: algorithmVersion.value,
    videoPath: videoPath.value,
    extractResult: extractResult.value,
    updatedAt: new Date().toISOString(),
  })
}

function clearFrameDraftSafely() {
  try {
    workspaceStore.clearFrameSelectionDraft?.()
  } catch {
    // A stale store instance during development hot reload must never block file selection.
  }
}

function restoreFrameDraft() {
  const draft = workspaceStore.frameSelectionDraft
  if (!Array.isArray(draft?.items) || !draft.items.length) return false
  resultImages.value = draft.items
  sourceDirectory.value = draft.sourceDirectory || parentDir(draft.items[0]?.path || '')
  mediaSessionId.value = draft.sessionId || null
  algorithmVersion.value = draft.algorithmVersion || ''
  videoPath.value = draft.videoPath || ''
  extractResult.value = draft.extractResult || null
  settingsCollapsed.value = true
  decisionHistory.value = []
  mediaReadyToSave = true
  return true
}

async function saveMediaSession() {
  if (mediaSaveTimer) window.clearTimeout(mediaSaveTimer)
  mediaSaveTimer = null
  if (!mediaReadyToSave || !sourceDirectory.value || !resultImages.value.length) return false
  const result = await tauriCallSafe('save_media_workspace_session', {
    args: {
      id: mediaSessionId.value,
      module_id: 'video-extract',
      source_path: sourceDirectory.value,
      settings: {
        algorithm_version: algorithmVersion.value,
        analysis: analysisSettings,
        extraction: settings,
      },
      items: resultImages.value.map((item, orderIndex) => ({
        path: item.path,
        engine_decision: item.engine_decision,
        user_decision: item.user_decision,
        reason: item.reason,
        metrics: frameMetrics(item),
        order_index: orderIndex,
      })),
    },
  })
  if (result.ok) mediaSessionId.value = result.data.id
  return result.ok
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

watch(
  resultImages,
  () => {
    scheduleMediaSave()
    syncFrameDraft()
  },
  { deep: true },
)

onMounted(async () => {
  await preference.start()
  restoreFrameDraft()
  unlistenAnalysisProgress = await listen('docsy-operation-progress', (event) => {
    const operationId = event.payload?.operationId || ''
    if (analyzingSelection.value && operationId.startsWith('analyze_frame_selection:')) {
      analysisProgressText.value = event.payload?.label || analysisProgressText.value
    }
  })
  checkFfmpeg()
})

onBeforeUnmount(() => {
  cancelViewOperations()
  if (mediaSaveTimer) window.clearTimeout(mediaSaveTimer)
  if (dynamicAnalysisTimer) window.clearTimeout(dynamicAnalysisTimer)
  unlistenAnalysisProgress?.()
  unlistenAnalysisProgress = null
  void saveMediaSession()
  void preference.stop()
})

onDeactivated(cancelViewOperations)

function cancelViewOperations() {
  if (extracting.value) {
    void tauriCallQuiet('cancel_operation', { operationId: EXTRACT_OPERATION_ID })
  }
  if (analyzingSelection.value) {
    void tauriCallQuiet('cancel_operation', { operationId: ANALYZE_OPERATION_ID })
  }
}

useWindowFileDrop({
  onEnter: () => {
    dragging.value = true
  },
  onLeave: () => {
    dragging.value = false
  },
  onDrop: handleDroppedPaths,
  onError: () => {
    dragging.value = false
    ElMessage.error('视频拖拽监听启动失败；仍可点击选择视频。')
  },
})
</script>

<style scoped>
:deep(.workspace-content) {
  overflow: hidden;
}

.extract-layout {
  display: grid;
  grid-template-columns: 382px minmax(0, 1fr);
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-panel-radius);
  box-shadow: var(--docsy-shadow-panel);
}

.extract-layout.settings-collapsed {
  grid-template-columns: minmax(0, 1fr);
}

.extract-layout.settings-collapsed .extract-settings {
  display: none;
}

.extract-settings {
  min-width: 0;
  overflow-y: auto;
  padding-block: clamp(16px, 3.1dvh, 22px) clamp(20px, 3.9dvh, 28px);
  padding-inline: 24px;
  border-right: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-elevated);
}

.extract-results {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: color-mix(in srgb, var(--docsy-surface-muted) 82%, var(--docsy-canvas));
}

.result-empty-state {
  min-height: 0;
  flex: 1;
  margin: 18px;
}

.results-header {
  padding-block: clamp(12px, 1.8dvh, 14px);
  padding-inline: 18px;
  min-height: clamp(48px, 6.2dvh, 56px);
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: rgba(255, 253, 248, 0.9);
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

.results-header-spacer {
  flex: 1;
}

.results-preview {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: clamp(12px, 2.4dvh, 16px);
  overflow: hidden;
}

.results-preview :deep(.reorder-image-scroll) {
  flex: 1;
  min-height: 0;
  max-height: none;
}

.section-block {
  margin-bottom: clamp(14px, 2.8dvh, 20px);
  padding-bottom: clamp(14px, 2.8dvh, 20px);
  border-bottom: 1px solid var(--docsy-border-subtle);
}

.section-block:last-child {
  border-bottom: 0;
}

.section-title {
  font-size: 14px;
  font-weight: 680;
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
  color: var(--el-color-success);
}

.status-warn {
  color: var(--el-color-warning);
}

.drop-zone {
  border: 1px dashed var(--docsy-border-strong);
  border-radius: var(--docsy-radius);
  padding-block: clamp(20px, 3.5dvh, 28px);
  padding-inline: 16px;
  background: var(--docsy-surface-muted);
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
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--docsy-primary) 30%, transparent);
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
  gap: 3px;
  padding: 9px 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
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

.primary-workspace-action {
  width: 100%;
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

.overlap-targets {
  display: grid;
  width: 100%;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 7px;
}

.overlap-targets :deep(.el-input-number) {
  width: 100%;
}

.engine-note {
  padding: 9px 11px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-muted);
  font-size: 12px;
  line-height: 1.55;
}

@media (max-width: 1180px) {
  :deep(.workspace-content) {
    overflow: auto;
  }

  .extract-layout {
    display: block;
    height: auto;
    overflow: visible;
  }

  .extract-settings {
    overflow: visible;
    border-right: 0;
    border-bottom: 1px solid var(--docsy-border-subtle);
  }

  .extract-results {
    min-height: clamp(440px, 62dvh, 560px);
  }
}
</style>
