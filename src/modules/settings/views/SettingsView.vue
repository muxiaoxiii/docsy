<template>
  <div class="settings-view">
    <h2>设置</h2>

    <!-- External Tools Status -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <div class="card-header">
          <div class="card-header-title">
            <span>外部工具状态（环境检测）</span>
          </div>
          <div class="card-header-actions">
            <el-button type="primary" size="small" @click="runOnboardingWizard">
              <el-icon><Compass /></el-icon>
              启动引导 (一键安装组件)
            </el-button>
            <el-button size="small" :loading="checkingTools" @click="checkTools">
              <el-icon><Refresh /></el-icon>
              重新检测环境
            </el-button>
            <el-button size="small" @click="openToolsPage">外部工具下载页面</el-button>
            <el-button size="small" @click="openManagedToolsDir">打开 Docsy 工具目录</el-button>
          </div>
        </div>
      </template>
      <el-alert type="warning" :closable="false" show-icon class="tools-risk-alert">
        <template #title>风险提示：请从官方渠道下载外部工具</template>
        只使用官方源或官网下载页列出的镜像，不要使用第三方打包版本；下载后注意核对 SHA256 校验值（Docsy
        自动安装时会强制校验）。第三方工具的使用风险由您自行承担。
      </el-alert>
      <p class="section-desc">
        Windows 支持自动下载安装工具包到 Docsy 托管目录；macOS 推荐使用 Homebrew 安装系统工具（支持一键调起终端安装），也可手动下载后通过本地安装导入。如果某个工具出现问题，可以清除托管版本后重新安装。
      </p>
      <div v-if="managedToolsDir" class="managed-dir">{{ managedToolsDir }}</div>
      <div v-if="hasMissingTools" class="quick-setup-banner">
        <div class="quick-setup-text">
          <span class="quick-setup-title">💡 组件一键安装与环境配置引导</span>
          <span class="quick-setup-desc">检测到部分关键组件尚未就绪。您可以点击启动引导向导，按向导一键自动安装配置所需组件。</span>
        </div>
        <el-button type="primary" size="small" @click="runOnboardingWizard">
          <el-icon><Compass /></el-icon>
          启动组件安装引导
        </el-button>
      </div>
      <div class="tool-list">
        <div v-for="tool in tools" :key="tool.name" class="tool-item">
          <div class="tool-info">
            <span class="tool-name">{{ tool.label }}</span>
            <el-tag v-if="tool.checking" type="info" size="small">检测中</el-tag>
            <el-tag v-else-if="tool.probeState === 'pending'" type="info" size="small">尚未检测</el-tag>
            <el-tag v-else-if="tool.status.available" type="success" size="small">可用</el-tag>
            <el-tag v-else-if="tool.probeState === 'error'" type="danger" size="small">检测失败</el-tag>
            <el-tag v-else type="danger" size="small">未安装</el-tag>
            <el-tag v-if="tool.status.available" size="small" :type="tool.status.managed ? 'primary' : 'info'">
              {{ tool.status.managed ? 'Docsy 托管' : '系统工具' }}
            </el-tag>
          </div>
          <div class="tool-detail" v-if="tool.status.available">
            <span class="tool-path">{{ tool.status.path }}</span>
            <span class="tool-version" v-if="tool.status.version">{{ tool.status.version }}</span>
          </div>
          <div class="tool-desc">{{ tool.description }}</div>
          <div v-if="tool.probeState === 'error'" class="install-hint">{{ tool.probeError }}</div>
          <div class="tool-actions">
            <el-button size="small" :loading="tool.checking" @click="checkTool(tool)">检测此工具</el-button>
            <el-button
              v-if="tool.status.available && tool.status.managed && tool.autoInstall"
              size="small"
              type="danger"
              :loading="tool.removing"
              @click="removeManagedTool(tool)"
            >
              清除此工具
            </el-button>
          </div>
          <div class="tool-actions" v-if="!tool.status.available">
            <span class="install-hint">{{ tool.status.install_hint }}</span>
            <el-button
              v-if="tool.autoInstall"
              size="small"
              type="primary"
              @click="installTool(tool.name)"
              :loading="tool.installing"
            >
              {{ isMac ? '终端安装 (Homebrew)' : '下载安装到 Docsy' }}
            </el-button>
            <el-button
              v-if="tool.autoInstall"
              size="small"
              @click="installToolFromPackage(tool.name)"
              :loading="tool.installingLocal"
            >
              本地安装
            </el-button>
            <el-button size="small" @click="openToolDownload(tool)"> 下载页 </el-button>
            <el-button v-if="tool.runtimeUrl" size="small" @click="openExternalUrl(tool.runtimeUrl)">
              VC++ 运行库
            </el-button>
          </div>
        </div>
      </div>
    </el-card>

    <el-card class="settings-section" shadow="never">
      <template #header>
        <div class="card-header">
          <span>主菜单</span>
          <el-button size="small" @click="resetMenuOrder">恢复默认顺序</el-button>
        </div>
      </template>
      <div class="menu-order-list">
        <div v-for="(item, index) in menuSettingsItems" :key="item.id" class="menu-order-item">
          <el-checkbox :model-value="isMenuVisible(item.id)" @change="(value) => setMenuVisible(item.id, value)">
            {{ item.name }}
          </el-checkbox>
          <div class="menu-order-actions">
            <el-button size="small" :disabled="index === 0" @click="moveMenuItem(index, -1)">上移</el-button>
            <el-button size="small" :disabled="index === menuSettingsItems.length - 1" @click="moveMenuItem(index, 1)"
              >下移</el-button
            >
          </div>
        </div>
      </div>
    </el-card>

    <!-- App Settings -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <span>应用设置</span>
      </template>
      <el-form label-width="120px">
        <el-form-item label="LibreOffice 路径">
          <el-input v-model="settings.libreoffice_path" placeholder="留空则自动检测" />
          <span class="form-hint">用于 DOC/DOCX 转 PDF</span>
        </el-form-item>
        <el-form-item label="工具清单地址">
          <el-input
            v-model="settings.tool_manifest_url"
            placeholder="留空使用默认清单；国内环境可填写 Gitee、对象存储或内网清单地址"
          />
          <span class="form-hint">用于 qpdf、Poppler、FFmpeg 在线安装</span>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="saveSettings">保存设置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- GitHub Proxy & Acceleration Settings -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <div class="card-header">
          <span>GitHub 镜像与下载加速 (网络优化)</span>
          <el-button
            size="small"
            type="primary"
            plain
            :loading="testingProxies"
            @click="runProxySpeedTest"
          >
            <el-icon><Refresh /></el-icon>
            测试各节点延迟
          </el-button>
        </div>
      </template>

      <div class="proxy-explanation">
        下载外部依赖工具（FFmpeg、qpdf、Poppler）时，若直连 GitHub 较慢或超时，Docsy 将按此处配置的加速镜像下载。
      </div>

      <div class="proxy-selection-row">
        <span class="label">首选下载通道：</span>
        <el-radio-group v-model="settings.selected_gh_proxy" @change="saveSettings">
          <el-radio-button label="auto">自动选择最快 / 智能回退</el-radio-button>
          <el-radio-button label="direct">直连 GitHub (官方源)</el-radio-button>
        </el-radio-group>
      </div>

      <!-- Proxy Latency Results -->
      <div v-if="proxyResults.length > 0" class="proxy-results-grid">
        <div
          v-for="p in proxyResults"
          :key="p.name + p.proxyUrl"
          class="proxy-result-card"
          :class="{ 'is-selected': isProxySelected(p) }"
        >
          <div class="proxy-card-top">
            <span class="proxy-card-title">{{ p.name }}</span>
            <el-tag
              v-if="p.isAvailable"
              size="small"
              :type="p.latencyMs < 400 ? 'success' : p.latencyMs < 900 ? 'warning' : 'info'"
            >
              {{ p.latencyMs }} ms
            </el-tag>
            <el-tag v-else size="small" type="danger">超时 / 不可用</el-tag>
          </div>
          <div class="proxy-card-url" :title="p.proxyUrl || 'https://github.com'">
            {{ p.proxyUrl || 'https://github.com (官方直连)' }}
          </div>
          <div class="proxy-card-footer">
            <el-button
              v-if="p.isAvailable && !isProxySelected(p)"
              size="small"
              text
              type="primary"
              @click="selectSpecificProxy(p)"
            >
              设为首选
            </el-button>
            <span v-else-if="isProxySelected(p)" class="selected-badge">✓ 当前首选</span>
            <el-button
              v-if="p.isCustom"
              size="small"
              text
              type="danger"
              @click="removeCustomProxy(p.proxyUrl)"
            >
              删除
            </el-button>
          </div>
        </div>
      </div>

      <!-- Add Custom Proxy -->
      <div class="custom-proxy-box">
        <div class="custom-proxy-title">自定义 GitHub 代理 / 反代加速镜像：</div>
        <div class="custom-proxy-tip">
          💡 提示：自定义镜像由第三方服务器提供网络中转，请仅添加您了解并信任的 HTTPS 镜像地址。
        </div>
        <div class="custom-proxy-input-row">
          <el-input
            v-model="newCustomProxy"
            placeholder="例如 https://gh.example.com/ 或自建反代地址"
            clearable
            @keyup.enter="addCustomProxy"
          />
          <el-button type="primary" :disabled="!newCustomProxy.trim()" @click="addCustomProxy">
            添加镜像
          </el-button>
        </div>
        <div v-if="settings.custom_gh_proxies?.length" class="custom-proxy-tags">
          <el-tag
            v-for="url in settings.custom_gh_proxies"
            :key="url"
            closable
            type="info"
            class="custom-tag"
            @close="removeCustomProxy(url)"
          >
            {{ url }}
          </el-tag>
        </div>
      </div>
    </el-card>

    <!-- Diagnostics -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <div class="card-header">
          <span>系统与环境诊断信息</span>
          <el-button size="small" type="primary" plain @click="runOnboardingWizard">
            <el-icon><Compass /></el-icon>
            启动引导向导 (一键安装组件)
          </el-button>
        </div>
      </template>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item label="版本">{{ diagnostic.version }}</el-descriptions-item>
        <el-descriptions-item label="系统">{{ diagnostic.os }} / {{ diagnostic.arch }}</el-descriptions-item>
        <el-descriptions-item label="qpdf">{{ diagnostic.qpdf?.version || '不可用' }}</el-descriptions-item>
        <el-descriptions-item label="poppler">{{ diagnostic.poppler?.version || '不可用' }}</el-descriptions-item>
        <el-descriptions-item label="ffmpeg">{{ diagnostic.ffmpeg?.version || '不可用' }}</el-descriptions-item>
      </el-descriptions>
      <div class="diag-actions">
        <el-button type="primary" size="small" :loading="composingLogEmail" @click="sendLogEmail">
          <el-icon><Message /></el-icon>
          发送日志给作者
        </el-button>
        <el-button size="small" @click="openLogDir">打开日志目录</el-button>
        <el-button size="small" @click="openLogFile">打开当前日志</el-button>
        <el-button size="small" @click="runOnboardingWizard">启动组件安装引导</el-button>
      </div>
      <p class="diagnostic-hint">
        将创建发往 oonlyxin@outlook.com 的邮件草稿，并仅附加已隐藏本地路径和文件名的脱敏日志。
      </p>
    </el-card>
  </div>
</template>

<script setup>
import { computed, ref, reactive, onMounted, onUnmounted } from 'vue'
import { openExternalUrl, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { DOCSY_TOOLS_URL } from '../../../core/siteConfig.js'
import { defaultMenuOrder, getMenuModules } from '../../../core/moduleRegistry.js'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Message, Compass, Refresh } from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { isMac, installToolViaTerminal, openToolDownloadWithGuide } from '../../../core/terminalInstall.js'

const settings = ref({
  menu_visibility: {},
  menu_order: [],
  libreoffice_path: '',
  tool_manifest_url: '',
  custom_gh_proxies: [],
  selected_gh_proxy: 'auto',
})
const managedToolsDir = ref('')
const checkingTools = ref(false)
const composingLogEmail = ref(false)
const testingProxies = ref(false)
const proxyResults = ref([])
const newCustomProxy = ref('')
const activeInstallTimers = new Set()
const menuModules = getMenuModules()
const hasMissingTools = computed(() => tools.some((tool) => !tool.status?.available))
const menuSettingsItems = computed(() =>
  normalizedMenuOrder()
    .map((id) => menuModules.find((item) => item.id === id))
    .filter(Boolean),
)

const tools = reactive([
  {
    name: 'qpdf',
    label: 'qpdf',
    description: 'PDF 合并、拆分、叠加和结构处理',
    status: defaultToolStatus(),
    probeState: 'pending',
    probeError: '',
    checking: false,
    installing: false,
    installingLocal: false,
    removing: false,
    autoInstall: true,
    downloadUrl: 'https://github.com/qpdf/qpdf/releases',
    downloadGuide: 'Windows：选择 qpdf-*-msvc64.zip；macOS：建议直接用 brew install qpdf',
  },
  {
    name: 'poppler',
    label: 'Poppler',
    description: 'PDF 预览渲染和页眉页脚文本检测',
    status: defaultToolStatus(),
    probeState: 'pending',
    probeError: '',
    checking: false,
    installing: false,
    installingLocal: false,
    removing: false,
    autoInstall: true,
    downloadUrl: 'https://github.com/oschwartz10612/poppler-windows/releases',
    downloadGuide: 'Windows：选择最新 Release-*.zip；macOS：建议直接用 brew install poppler',
    runtimeUrl: 'https://aka.ms/vc14/vc_redist.x64.exe',
  },
  {
    name: 'ffmpeg',
    label: 'FFmpeg',
    description: '视频信息读取、抽帧和时间戳水印',
    status: defaultToolStatus(),
    probeState: 'pending',
    probeError: '',
    checking: false,
    installing: false,
    installingLocal: false,
    removing: false,
    autoInstall: true,
    downloadUrl: 'https://github.com/BtbN/FFmpeg-Builds/releases',
    downloadGuide:
      'Windows：选择 ffmpeg-master-latest-win64-gpl.zip（GPL 版，含 drawtext）；macOS：需要时间戳水印时建议 brew install ffmpeg-full',
  },
  {
    name: 'word',
    label: 'Microsoft Word',
    description: 'Word 文件转 PDF 的首选引擎；Windows 使用 COM，macOS 使用 AppleScript',
    status: defaultToolStatus(),
    probeState: 'pending',
    probeError: '',
    checking: false,
    installing: false,
    installingLocal: false,
    removing: false,
    autoInstall: false,
    downloadUrl: 'https://www.microsoft.com/microsoft-365/word',
  },
  {
    name: 'wps',
    label: 'WPS Writer',
    description: 'Windows 下 Word 不可用时的第二转换引擎，使用 WPS COM 导出 PDF',
    status: defaultToolStatus(),
    probeState: 'pending',
    probeError: '',
    checking: false,
    installing: false,
    installingLocal: false,
    removing: false,
    autoInstall: false,
    downloadUrl: 'https://www.wps.cn/',
  },
  {
    name: 'libreoffice',
    label: 'LibreOffice',
    description: 'Word 文件转 PDF 的备用引擎；没有 Word 或 Word 转换失败时使用',
    status: defaultToolStatus(),
    probeState: 'pending',
    probeError: '',
    checking: false,
    installing: false,
    installingLocal: false,
    removing: false,
    autoInstall: false,
    downloadUrl: 'https://www.libreoffice.org/download/',
  },
])

const diagnostic = ref({
  version: '',
  os: '',
  arch: '',
  qpdf: null,
  poppler: null,
  ffmpeg: null,
})

function defaultToolStatus() {
  return {
    available: false,
    path: null,
    version: null,
    install_hint: '',
    managed: false,
    source: 'missing',
  }
}

async function loadSettings() {
  const result = await tauriCallSafe('get_app_settings')
  if (result.ok) {
    settings.value = { ...settings.value, ...result.data }
    settings.value.menu_visibility = settings.value.menu_visibility || {}
    settings.value.menu_order = Array.isArray(settings.value.menu_order) ? settings.value.menu_order : []
    settings.value.libreoffice_path = settings.value.libreoffice_path || ''
    settings.value.tool_manifest_url = settings.value.tool_manifest_url || ''
    settings.value.custom_gh_proxies = Array.isArray(settings.value.custom_gh_proxies) ? settings.value.custom_gh_proxies : []
    settings.value.selected_gh_proxy = settings.value.selected_gh_proxy || 'auto'
  } else {
    ElMessage.warning('设置加载失败')
  }
}

async function saveSettings() {
  // 简单验证
  const manifestUrl = (settings.value.tool_manifest_url || '').trim()
  if (manifestUrl && !manifestUrl.startsWith('http://') && !manifestUrl.startsWith('https://')) {
    ElMessage.warning('工具清单地址必须以 http:// 或 https:// 开头')
    return
  }
  const loPath = (settings.value.libreoffice_path || '').trim()
  if (loPath && !loPath.startsWith('/') && !/^[A-Za-z]:[\\/]/.test(loPath)) {
    ElMessage.warning('LibreOffice 路径格式不正确，应以 / 或盘符（如 C:\\ 或 C:/）开头')
    return
  }

  const payload = {
    ...settings.value,
    menu_order: normalizedMenuOrder(),
    libreoffice_path: settings.value.libreoffice_path || null,
    tool_manifest_url: settings.value.tool_manifest_url || null,
    custom_gh_proxies: settings.value.custom_gh_proxies || [],
    selected_gh_proxy: settings.value.selected_gh_proxy || null,
  }
  const result = await tauriCallSafe('set_app_settings', { settings: payload })
  if (result.ok) {
    settings.value = { ...settings.value, ...payload }
    window.dispatchEvent(new CustomEvent('docsy-settings-updated', { detail: payload }))
    ElMessage.success('设置已保存')
  } else {
    ElMessage.error(userFacingError(result.error, '保存设置失败'))
  }
}

function normalizedMenuOrder() {
  const knownIds = new Set(menuModules.map((item) => item.id))
  const current = Array.isArray(settings.value.menu_order) ? settings.value.menu_order : []
  const ordered = current.filter((id) => knownIds.has(id))
  for (const id of defaultMenuOrder()) {
    if (!ordered.includes(id)) ordered.push(id)
  }
  return ordered
}

function moveMenuItem(index, delta) {
  const order = normalizedMenuOrder()
  const target = index + delta
  if (target < 0 || target >= order.length) return
  const next = [...order]
  ;[next[index], next[target]] = [next[target], next[index]]
  settings.value.menu_order = next
}

function resetMenuOrder() {
  settings.value.menu_order = defaultMenuOrder()
}

function isMenuVisible(id) {
  return (settings.value.menu_visibility || {})[id] !== false
}

function setMenuVisible(id, value) {
  settings.value.menu_visibility = {
    ...(settings.value.menu_visibility || {}),
    [id]: Boolean(value),
  }
}

async function checkTools() {
  if (checkingTools.value) return
  checkingTools.value = true
  try {
    await Promise.all(tools.map(checkTool))
    syncDiagnosticToolStatus()
  } finally {
    checkingTools.value = false
  }
}

async function checkTool(tool) {
  if (!tool || tool.checking) return
  tool.checking = true
  tool.probeState = 'checking'
  tool.probeError = ''
  try {
    const result = await tauriCallSafe('check_external_tool', { toolName: tool.name })
    if (result.ok) {
      tool.status = result.data
      tool.probeState = result.data?.available ? 'available' : 'unavailable'
    } else {
      tool.status = defaultToolStatus()
      tool.probeState = 'error'
      tool.probeError = result.error || '工具检测调用失败'
    }
  } finally {
    tool.checking = false
  }
}

async function loadManagedToolsDir() {
  const result = await tauriCallSafe('get_managed_tools_dir')
  if (result.ok) {
    managedToolsDir.value = result.data
  }
}

async function loadDiagnostic() {
  const result = await tauriCallSafe('get_diagnostic_info')
  if (result.ok) {
    diagnostic.value = { ...diagnostic.value, ...result.data }
  }
}

function syncDiagnosticToolStatus() {
  for (const name of ['qpdf', 'poppler', 'ffmpeg']) {
    const tool = tools.find((t) => t.name === name)
    if (tool) {
      diagnostic.value[name] = {
        available: tool.status.available,
        version: tool.status.version,
      }
    }
  }
}

async function installTool(name) {
  const tool = tools.find((t) => t.name === name)
  if (isMac) {
    if (tool) tool.installing = true
    const ok = await installToolViaTerminal(name, tool?.label || name)
    if (!ok) {
      if (tool) tool.installing = false
      return
    }
    let checks = 0
    const timer = setInterval(async () => {
      checks++
      if (tool) await checkTool(tool)
      if (tool?.status?.available || checks > 150) {
        clearInterval(timer)
        activeInstallTimers.delete(timer)
        if (tool) tool.installing = false
        await loadDiagnostic()
      }
    }, 2000)
    activeInstallTimers.add(timer)
    return
  }
  if (tool) tool.installing = true
  const result = await tauriCallSafe('install_external_tool', { toolName: name })
  if (result.ok) {
    ElMessage.success(result.data || '安装完成')
    await checkTools()
    await loadDiagnostic()
  } else {
    ElMessage.error(userFacingError(result.error, '安装失败'))
  }
  if (tool) tool.installing = false
}

async function installToolFromPackage(name) {
  const selected = await open({
    multiple: false,
    filters: [{ name: '工具包', extensions: ['zip', '7z'] }],
  })
  if (!selected) return

  const tool = tools.find((t) => t.name === name)
  if (tool) tool.installingLocal = true
  const result = await tauriCallSafe('install_external_tool_from_package', {
    toolName: name,
    packagePath: selected,
  })
  if (result.ok) {
    ElMessage.success(result.data || '安装完成')
    await checkTools()
    await loadDiagnostic()
  } else {
    ElMessage.error(userFacingError(result.error, '安装失败'))
  }
  if (tool) tool.installingLocal = false
}

async function openLogDir() {
  const result = await tauriCallSafe('open_log_dir')
  if (!result.ok) {
    ElMessage.error(userFacingError(result.error, '无法打开日志目录'))
  }
}

async function openLogFile() {
  const result = await tauriCallSafe('open_log_file')
  if (!result.ok) {
    ElMessage.error(userFacingError(result.error, '无法打开日志文件'))
  }
}

async function sendLogEmail() {
  if (composingLogEmail.value) return
  composingLogEmail.value = true
  try {
    const result = await tauriCallSafe('compose_log_email')
    if (result.ok) {
      if (result.data?.attached) ElMessage.success(result.data.message)
      else ElMessage.warning(result.data?.message || '已打开邮件草稿和日志位置')
    } else {
      ElMessage.error(userFacingError(result.error, '无法创建日志邮件'))
    }
  } finally {
    composingLogEmail.value = false
  }
}

async function openManagedToolsDir() {
  const result = await tauriCallSafe('open_managed_tools_dir')
  if (!result.ok) {
    ElMessage.error(userFacingError(result.error, '无法打开工具目录'))
  }
}

function openToolsPage() {
  openExternalUrl(DOCSY_TOOLS_URL)
}

function runOnboardingWizard() {
  window.dispatchEvent(new CustomEvent('open-onboarding-modal'))
}

async function removeManagedTool(tool) {
  try {
    await ElMessageBox.confirm(
      `清除 Docsy 托管的 ${tool.label}？清除后将使用系统已安装的版本（如有）。`,
      '清除托管工具',
      { confirmButtonText: '清除', cancelButtonText: '取消', type: 'warning' },
    )
  } catch {
    return
  }
  tool.removing = true
  try {
    const result = await tauriCallSafe('remove_managed_tool', { toolName: tool.name })
    if (result.ok) {
      ElMessage.success(result.data || '已清除')
      await checkTool(tool)
    } else {
      ElMessage.error(userFacingError(result.error, '清除失败'))
    }
  } finally {
    tool.removing = false
  }
}

async function openToolDownload(tool) {
  await openToolDownloadWithGuide(tool.name, tool.downloadUrl, tool.label)
}

async function runProxySpeedTest() {
  testingProxies.value = true
  try {
    const res = await tauriCallSafe('test_github_proxies')
    if (res.ok && Array.isArray(res.data)) {
      proxyResults.value = res.data
      ElMessage.success('测速完成')
    } else {
      ElMessage.warning('测速未完成或返回异常')
    }
  } catch (err) {
    ElMessage.error(userFacingError(err, '测速失败'))
  } finally {
    testingProxies.value = false
  }
}

function isProxySelected(p) {
  if (settings.value.selected_gh_proxy === 'direct' && p.isDirect) return true
  if (settings.value.selected_gh_proxy === p.proxyUrl && !p.isDirect) return true
  return false
}

function selectSpecificProxy(p) {
  if (p.isDirect) {
    settings.value.selected_gh_proxy = 'direct'
  } else {
    settings.value.selected_gh_proxy = p.proxyUrl
  }
  saveSettings()
  ElMessage.success(`已将 ${p.name} 设为首选下载通道`)
}

async function addCustomProxy() {
  const url = (newCustomProxy.value || '').trim()
  if (!url) return
  if (!url.startsWith('https://')) {
    ElMessage.warning('出于安全性考虑，自定义代理镜像必须使用以 https:// 开头的地址')
    return
  }
  const current = Array.isArray(settings.value.custom_gh_proxies) ? [...settings.value.custom_gh_proxies] : []
  if (current.includes(url)) {
    ElMessage.info('该代理地址已存在')
    return
  }
  current.push(url)
  settings.value.custom_gh_proxies = current
  newCustomProxy.value = ''
  await saveSettings()
  ElMessage.success('已添加自定义代理镜像')
  runProxySpeedTest()
}

async function removeCustomProxy(url) {
  const current = (settings.value.custom_gh_proxies || []).filter((u) => u !== url)
  settings.value.custom_gh_proxies = current
  if (settings.value.selected_gh_proxy === url) {
    settings.value.selected_gh_proxy = 'auto'
  }
  await saveSettings()
  proxyResults.value = proxyResults.value.filter((p) => p.proxyUrl !== url)
  ElMessage.success('已移除自定义代理')
}

onMounted(() => {
  loadSettings()
  loadManagedToolsDir()
  loadDiagnostic()
  checkTools()
})

onUnmounted(() => {
  activeInstallTimers.forEach((timer) => clearInterval(timer))
  activeInstallTimers.clear()
})
</script>

<style scoped>
.settings-view {
  max-width: 1120px;
  margin: 0 auto;
  padding-block: clamp(22px, 3.8dvh, 34px) clamp(32px, 5.3dvh, 48px);
  padding-inline: 32px;
}

.settings-view h2 {
  margin: 0 0 clamp(14px, 2.2dvh, 18px);
  color: var(--docsy-text-strong);
  font-family:
    ui-rounded,
    'SF Pro Rounded',
    -apple-system,
    'PingFang SC',
    sans-serif;
  font-size: 22px;
  font-weight: 720;
  letter-spacing: -0.025em;
}

.settings-section {
  margin-bottom: clamp(14px, 2dvh, 18px);
  border-color: var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: var(--docsy-shadow-panel);
}

.proxy-explanation {
  font-size: 13px;
  color: var(--docsy-text-muted);
  margin-bottom: 14px;
  line-height: 1.5;
}

.proxy-selection-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  font-size: 13px;
}

.proxy-selection-row .label {
  color: var(--docsy-text-regular);
  font-weight: 500;
}

.proxy-results-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 12px;
  margin-bottom: 18px;
}

.proxy-result-card {
  padding: 12px 14px;
  border-radius: 8px;
  border: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface);
  display: flex;
  flex-direction: column;
  gap: 6px;
  transition: all 0.2s ease;
}

.proxy-result-card.is-selected {
  border-color: var(--el-color-primary);
  background: color-mix(in srgb, var(--el-color-primary) 8%, var(--docsy-surface));
}

.proxy-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.proxy-card-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--docsy-text-strong);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proxy-card-url {
  font-size: 11px;
  color: var(--docsy-text-muted);
  font-family: ui-monospace, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proxy-card-footer {
  margin-top: 4px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.selected-badge {
  font-size: 12px;
  font-weight: 600;
  color: var(--el-color-primary);
}

.custom-proxy-box {
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px dashed var(--docsy-border-subtle);
}

.custom-proxy-title {
  font-size: 13px;
  color: var(--docsy-text-regular);
  margin-bottom: 6px;
  font-weight: 500;
}

.custom-proxy-tip {
  font-size: 12px;
  color: var(--docsy-text-muted);
  margin-bottom: 10px;
  line-height: 1.4;
}

.custom-proxy-input-row {
  display: flex;
  gap: 8px;
}

.custom-proxy-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 10px;
}

.custom-tag {
  font-family: ui-monospace, monospace;
}

.settings-section :deep(.el-card__header) {
  padding-block: clamp(12px, 1.8dvh, 16px);
  padding-inline: 18px;
  background: color-mix(in srgb, var(--docsy-surface-muted) 72%, transparent);
}

.settings-section :deep(.el-card__body) {
  padding-block: clamp(14px, 2dvh, 18px);
  padding-inline: 18px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.card-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.managed-dir {
  margin-bottom: 12px;
  padding: 8px 10px;
  color: var(--docsy-text);
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  font-size: 12px;
  word-break: break-all;
}

.tool-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.tool-item {
  min-width: 0;
  padding: 14px;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
}

.tool-info {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.tool-name {
  font-weight: 600;
  font-size: 14px;
}

.tool-detail {
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-size: 12px;
  color: var(--docsy-text-muted);
  word-break: break-all;
}

.tool-desc {
  margin: 4px 0;
  font-size: 12px;
  color: var(--docsy-text);
}

.tool-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}

.menu-order-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.menu-order-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
}

.menu-order-actions {
  display: flex;
  gap: 8px;
}

.install-hint {
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.form-hint {
  margin-left: 8px;
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.diag-actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}

.diagnostic-hint {
  margin: 8px 0 0;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.quick-setup-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  margin-bottom: 12px;
  background: var(--docsy-primary-light, rgba(59, 130, 246, 0.08));
  border: 1px solid var(--docsy-primary-border, rgba(59, 130, 246, 0.25));
  border-radius: var(--docsy-radius);
}

.quick-setup-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.quick-setup-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--docsy-text);
}

.quick-setup-desc {
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.section-desc {
  font-size: 13px;
  color: var(--docsy-text-muted);
  margin: 0 0 12px;
}

.tools-risk-alert {
  margin-bottom: 12px;
  border-radius: var(--docsy-radius);
}

.tools-risk-alert :deep(.el-alert__title) {
  font-size: 13px;
}

.tools-risk-alert :deep(.el-alert__content) {
  font-size: 12.5px;
  line-height: 1.65;
  color: var(--docsy-text);
}

@media (max-width: 760px) {
  .settings-view {
    padding: 16px;
  }

  .card-header,
  .tool-item {
    align-items: flex-start;
    flex-direction: column;
  }

  .tool-list,
  .menu-order-list {
    grid-template-columns: 1fr;
  }

  .menu-order-item {
    align-items: flex-start;
    flex-direction: column;
  }

  .menu-order-actions {
    align-self: stretch;
  }

  .menu-order-actions .el-button {
    flex: 1;
  }

  .diag-actions {
    flex-wrap: wrap;
  }

  :deep(.el-form-item) {
    align-items: stretch;
    flex-direction: column;
  }

  :deep(.el-form-item__label) {
    justify-content: flex-start;
    width: auto !important;
  }
}
</style>
