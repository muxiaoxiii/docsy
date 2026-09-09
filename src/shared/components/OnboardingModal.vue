<template>
  <el-dialog
    v-model="visible"
    :show-close="step === 'select'"
    :close-on-click-modal="false"
    :close-on-press-escape="step === 'select'"
    :before-close="handleSkip"
    width="680px"
    class="onboarding-dialog"
    destroy-on-close
    append-to-body
  >
    <!-- Header -->
    <template #header>
      <div class="onboarding-header">
        <div class="header-badge">首次配置向导</div>
        <h2 class="header-title">
          {{ step === 'select' ? '欢迎使用 Docsy 文档工具箱' : (isAllDone ? 'Docsy 已准备就绪！' : 'Doclet 正在为您准备组件') }}
        </h2>
        <p class="header-subtitle">
          {{
            step === 'select'
              ? '下面这些核心功能需要外部工具组件支持。您可以选择需要的功能自动配置，也可以直接跳过稍后在设置中安装。'
              : (isAllDone ? '所选组件已配置完毕，现在可以开始畅享完整的文档处理体验了。' : '小狗 Doclet 正在后台加紧处理，请稍候…')
          }}
        </p>
      </div>
    </template>

    <!-- Step 1: Feature Selection -->
    <div v-if="step === 'select'" class="onboarding-step-select">
      <div class="selection-toolbar">
        <span class="selection-count">已勾选 {{ selectedCount }} / {{ featureList.length }} 项功能</span>
        <el-button link type="primary" size="small" @click="toggleSelectAll">
          {{ selectedCount === featureList.length ? '取消全选' : '全选全部功能' }}
        </el-button>
      </div>

      <div class="feature-cards">
        <div
          v-for="item in featureList"
          :key="item.tool"
          class="feature-card"
          :class="{ 'is-selected': selectedTools.includes(item.tool), 'is-ready': item.ready }"
          @click="toggleTool(item.tool)"
        >
          <div class="card-checkbox" @click.stop>
            <el-checkbox
              :model-value="selectedTools.includes(item.tool)"
              @change="() => toggleTool(item.tool)"
            />
          </div>
          <div class="card-icon" :style="{ backgroundColor: item.color + '15', color: item.color }">
            <el-icon :size="22">
              <component :is="item.icon" />
            </el-icon>
          </div>
          <div class="card-content">
            <div class="card-title-row">
              <span class="card-title">{{ item.title }}</span>
              <el-tag v-if="item.ready" size="small" type="success" effect="plain">系统已具备</el-tag>
              <el-tag v-else size="small" type="info" effect="plain">{{ item.toolLabel }}</el-tag>
            </div>
            <p class="card-desc">{{ item.desc }}</p>
            <div class="card-detail">对应依赖：<code>{{ item.toolName }}</code>（{{ item.detail }}）</div>
          </div>
        </div>
      </div>

      <div class="onboarding-footer">
        <el-button size="large" class="skip-btn" @click="handleSkip">
          全部跳过，稍后自行安装
        </el-button>
        <el-button
          size="large"
          type="primary"
          class="submit-btn"
          :disabled="selectedCount === 0"
          @click="startSetup"
        >
          开始配置所选组件 ({{ selectedCount }})
        </el-button>
      </div>
    </div>

    <!-- Step 2: Doclet Working & Installation Progress -->
    <div v-else class="onboarding-step-progress">
      <div class="doclet-mascot-box">
        <div
          class="doclet-animated-sprite"
          :class="{ 'is-cheering': isAllDone }"
          :style="{ backgroundImage: `url(${spritesheet})` }"
        />
        <div class="doclet-current-status">
          {{ currentStatusText }}
        </div>
      </div>

      <el-progress
        :percentage="overallPercentage"
        :status="isAllDone ? 'success' : ''"
        :stroke-width="8"
        class="progress-bar"
      />

      <div class="progress-task-list">
        <div
          v-for="item in activeSetupItems"
          :key="item.tool"
          class="progress-task-item"
        >
          <div class="task-info">
            <span class="task-title">{{ item.title }}</span>
            <span class="task-tool">({{ item.toolLabel }})</span>
          </div>
          <div class="task-state">
            <el-tag v-if="item.status === 'ready'" size="small" type="success">✓ 已就绪</el-tag>
            <el-tag v-else-if="item.status === 'installing'" size="small" type="warning" effect="dark">
              正在安装…
            </el-tag>
            <el-tag v-else-if="item.status === 'error'" size="small" type="danger">需要手动安装</el-tag>
            <el-tag v-else size="small" type="info">等待中</el-tag>
          </div>
        </div>
      </div>

      <div v-if="!isAllDone && isMac" class="progress-notice">
        💡 系统终端正在合并安装所选组件。当组件安装完成，上方状态将自动点亮为「✓ 已就绪」。您可以随时点击下方按钮进入 Docsy，后台将持续进行。
      </div>

      <div class="onboarding-footer progress-footer">
        <el-button
          v-if="!isAllDone"
          size="large"
          class="skip-btn"
          @click="handleSkip"
        >
          进入 Docsy，后台继续
        </el-button>
        <el-button
          v-if="isAllDone"
          size="large"
          type="primary"
          class="finish-btn"
          @click="finishOnboarding"
        >
          🚀 开启 Docsy 之旅
        </el-button>
      </div>
    </div>
  </el-dialog>
</template>

<script setup>
import { ref, computed, reactive, onMounted, onUnmounted } from 'vue'
import { VideoCamera, Document, Files } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useAppStore } from '../../stores/app.js'
import { tauriCallSafe } from '../../core/tauriBridge.js'
import { isMac, installToolsBatchViaTerminal } from '../../core/terminalInstall.js'
import spritesheet from '../../assets/doclet-v2-spritesheet.webp'

const appStore = useAppStore()

const visible = ref(false)
const step = ref('select') // 'select' | 'progress'
const isAllDone = ref(false)
const currentStatusText = ref('Doclet 正在准备…')
let pollTimer = null

function clearPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

const featureList = reactive([
  {
    tool: 'ffmpeg',
    toolName: 'ffmpeg',
    toolLabel: 'FFmpeg 完整版',
    title: '视频智能抽帧与时间戳水印',
    desc: '高保真视频解析、按间隔或关键帧提取画面，并在导出图片时添加拍摄时间戳。',
    detail: '视频处理与文字滤镜',
    color: '#0284c7',
    icon: VideoCamera,
    ready: false,
    status: 'pending', // 'pending' | 'installing' | 'ready' | 'error'
  },
  {
    tool: 'poppler',
    toolName: 'poppler',
    toolLabel: 'Poppler 引擎',
    title: 'PDF 预览渲染与页眉页脚检测',
    desc: '提供 PDF 高精度图片级渲染预览，以及证据扫描件的页眉页脚范围智能文本定位。',
    detail: 'PDF 渲染与 OCR 定位支持',
    color: '#10b981',
    icon: Document,
    ready: false,
    status: 'pending',
  },
  {
    tool: 'qpdf',
    toolName: 'qpdf',
    toolLabel: 'Qpdf 结构工具',
    title: 'PDF 拆分合并与无损结构处理',
    desc: '极速无损合并/拆分多页文档、自动移除空白页，并保障输出 PDF 结构符合归档标准。',
    detail: 'PDF 线性化与结构处理',
    color: '#8b5cf6',
    icon: Files,
    ready: false,
    status: 'pending',
  },
])

const selectedTools = ref(['ffmpeg', 'poppler', 'qpdf'])

const selectedCount = computed(() => selectedTools.value.length)

const activeSetupItems = computed(() =>
  featureList.filter((f) => selectedTools.value.includes(f.tool)),
)

const overallPercentage = computed(() => {
  if (activeSetupItems.value.length === 0) return 100
  const completed = activeSetupItems.value.filter(
    (item) => item.status === 'ready' || item.status === 'error',
  ).length
  return Math.round((completed / activeSetupItems.value.length) * 100)
})

function toggleTool(tool) {
  const index = selectedTools.value.indexOf(tool)
  if (index >= 0) {
    selectedTools.value.splice(index, 1)
  } else {
    selectedTools.value.push(tool)
  }
}

function toggleSelectAll() {
  if (selectedTools.value.length === featureList.length) {
    selectedTools.value = []
  } else {
    selectedTools.value = featureList.map((f) => f.tool)
  }
}

async function probeInitialTools() {
  for (const item of featureList) {
    const res = await tauriCallSafe('check_external_tool', { toolName: item.tool })
    if (res.ok && res.data?.available) {
      item.ready = true
      item.status = 'ready'
    } else {
      item.ready = false
      item.status = 'pending'
    }
  }
}

async function startSetup() {
  step.value = 'progress'
  currentStatusText.value = 'Doclet 正在准备配置…'

  const toInstall = activeSetupItems.value.filter((item) => !item.ready)

  if (toInstall.length === 0) {
    isAllDone.value = true
    currentStatusText.value = '所有勾选功能已全部就绪！'
    return
  }

  for (const item of toInstall) {
    item.status = 'installing'
  }

  if (isMac) {
    currentStatusText.value = '已打开系统终端合并安装所选组件，Doclet 正在实时检测就绪状态…'
    await installToolsBatchViaTerminal(toInstall.map((item) => item.tool))

    clearPolling()
    pollTimer = setInterval(async () => {
      let pendingCount = 0
      for (const item of toInstall) {
        if (item.status === 'ready') continue
        const res = await tauriCallSafe('check_external_tool', { toolName: item.tool })
        if (res.ok && res.data?.available) {
          item.ready = true
          item.status = 'ready'
        } else {
          pendingCount++
        }
      }
      if (pendingCount === 0) {
        clearPolling()
        isAllDone.value = true
        currentStatusText.value = '🎉 所有组件已成功安装并验证就绪！'
      }
    }, 2000)
  } else {
    // Windows / other: 走托管包下载或后台安装
    for (const item of toInstall) {
      currentStatusText.value = `Doclet 正在为您配置 ${item.title}…`
      const res = await tauriCallSafe('install_external_tool', { toolName: item.tool })
      if (res.ok) {
        item.ready = true
        item.status = 'ready'
      } else {
        item.status = 'error'
      }
    }
    const allReady = toInstall.every((i) => i.status === 'ready')
    if (allReady) {
      isAllDone.value = true
      currentStatusText.value = '🎉 外部组件配置流程已完成！'
    } else {
      currentStatusText.value = '部分组件安装遇到问题，可进入 Docsy 在设置中重试'
    }
  }
}

async function handleSkip() {
  clearPolling()
  await appStore.completeOnboarding()
  visible.value = false
  ElMessage.info('已跳过引导，您稍后可随时在「设置」中安装所需组件')
}

async function finishOnboarding() {
  clearPolling()
  await appStore.completeOnboarding()
  visible.value = false
  ElMessage.success('欢迎开启 Docsy，尽情探索吧！')
}

function showModal() {
  step.value = 'select'
  isAllDone.value = false
  visible.value = true
  probeInitialTools()
}

onUnmounted(() => {
  clearPolling()
})

defineExpose({
  show: showModal,
})

onMounted(async () => {
  await appStore.loadSettings()
  if (!appStore.settings.onboarding_completed) {
    showModal()
  }
})
</script>

<style scoped>
.onboarding-dialog :deep(.el-dialog__header) {
  padding: 24px 28px 12px;
  margin-right: 0;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.onboarding-dialog :deep(.el-dialog__body) {
  padding: 20px 28px 24px;
}

.onboarding-header {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.header-badge {
  display: inline-block;
  align-self: flex-start;
  padding: 2px 10px;
  font-size: 11px;
  font-weight: 600;
  color: #0284c7;
  background: #e0f2fe;
  border-radius: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.header-title {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
  color: var(--el-text-color-primary);
}

.header-subtitle {
  margin: 0;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  line-height: 1.5;
}

.selection-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;
}

.selection-count {
  font-size: 13px;
  color: var(--el-text-color-regular);
  font-weight: 500;
}

.feature-cards {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 24px;
}

.feature-card {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 14px 16px;
  border: 1.5px solid var(--el-border-color-light);
  border-radius: 10px;
  background: var(--el-fill-color-blank);
  cursor: pointer;
  transition: all 0.2s ease;
}

.feature-card:hover {
  border-color: #93c5fd;
  background: #f8fbff;
}

.feature-card.is-selected {
  border-color: #3b82f6;
  background: #f0f7ff;
}

.feature-card.is-ready {
  border-color: #a7f3d0;
  background: #f0fdf4;
}

.card-checkbox {
  padding-top: 2px;
}

.card-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.card-icon :deep(.el-icon) {
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-icon :deep(svg),
.card-icon svg {
  width: 22px;
  height: 22px;
}

.card-content {
  flex: 1;
  min-width: 0;
}

.progress-notice {
  margin-top: 14px;
  padding: 10px 14px;
  border-radius: 8px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
}

.card-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.card-desc {
  margin: 0 0 6px 0;
  font-size: 12.5px;
  color: var(--el-text-color-regular);
  line-height: 1.45;
}

.card-detail {
  font-size: 11.5px;
  color: var(--el-text-color-secondary);
}

.card-detail code {
  background: var(--el-fill-color-light);
  padding: 1px 4px;
  border-radius: 4px;
  font-family: ui-monospace, monospace;
}

.onboarding-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--el-border-color-lighter);
}

.skip-btn {
  color: var(--el-text-color-secondary);
}

.submit-btn,
.finish-btn {
  padding: 10px 24px;
  font-weight: 600;
}

/* Step 2 styles */
.onboarding-step-progress {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px 0 0;
}

.doclet-mascot-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 18px;
}

.doclet-animated-sprite {
  width: 144px;
  height: 156px;
  background-repeat: no-repeat;
  background-size: 1152px 1716px;
  background-position: 0 -1404px;
  animation: doclet-look-around 5.6s steps(1, end) infinite;
  transform: scale(1.1);
  margin-bottom: 8px;
}

.doclet-current-status {
  font-size: 15px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-top: 6px;
  text-align: center;
}

.progress-bar {
  width: 100%;
  margin-bottom: 20px;
}

.progress-task-list {
  width: 100%;
  background: var(--el-fill-color-light);
  border-radius: 8px;
  padding: 8px 16px;
  margin-bottom: 24px;
}

.progress-task-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 0;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.progress-task-item:last-child {
  border-bottom: none;
}

.task-info {
  display: flex;
  align-items: center;
  gap: 6px;
}

.task-title {
  font-size: 13.5px;
  font-weight: 500;
  color: var(--el-text-color-primary);
}

.task-tool {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.progress-footer {
  width: 100%;
}

@keyframes doclet-look-around {
  0%,
  6.24% {
    background-position: 0 -1404px;
  }
  6.25%,
  12.49% {
    background-position: -144px -1404px;
  }
  12.5%,
  18.74% {
    background-position: -288px -1404px;
  }
  18.75%,
  24.99% {
    background-position: -432px -1404px;
  }
  25%,
  31.24% {
    background-position: -576px -1404px;
  }
  31.25%,
  37.49% {
    background-position: -720px -1404px;
  }
  37.5%,
  43.74% {
    background-position: -864px -1404px;
  }
  43.75%,
  49.99% {
    background-position: -1008px -1404px;
  }
  50%,
  56.24% {
    background-position: -864px -1404px;
  }
  56.25%,
  62.49% {
    background-position: -720px -1404px;
  }
  62.5%,
  68.74% {
    background-position: -576px -1404px;
  }
  68.75%,
  74.99% {
    background-position: -432px -1404px;
  }
  75%,
  81.24% {
    background-position: -288px -1404px;
  }
  81.25%,
  87.49% {
    background-position: -144px -1404px;
  }
  87.5%,
  100% {
    background-position: 0 -1404px;
  }
}
</style>
