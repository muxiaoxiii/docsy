import { ElMessage, ElMessageBox } from 'element-plus'
import { openExternalUrl, tauriCallSafe, userFacingError } from './tauriBridge.js'

export const isMac = /mac/i.test(navigator.platform || navigator.userAgent)

export const TOOL_DOWNLOAD_SPECS = {
  ffmpeg: {
    label: 'FFmpeg',
    windows: {
      url: 'https://github.com/BtbN/FFmpeg-Builds/releases',
      guide:
        '推荐下载 <code>ffmpeg-master-latest-win64-gpl.zip</code>（GPL 版包含 drawtext 时间戳水印支持）。下载后在 Docsy 点击「本地安装」选择该 zip 即可自动配置。',
    },
    macos: {
      url: 'https://evermeet.cx/ffmpeg/',
      guide:
        '官方推荐的 macOS 独立静态包构建页。请分别下载 <b>ffmpeg</b> 与 <b>ffprobe</b>（点击 7z/zip），解压后在 Docsy 点击「本地安装」导入。<br>💡 <b>更优推荐</b>：在 Docsy 中直接使用「终端安装 (Homebrew)」，全自动安装带 drawtext 的完整版。',
    },
  },
  poppler: {
    label: 'Poppler',
    windows: {
      url: 'https://github.com/oschwartz10612/poppler-windows/releases',
      guide:
        '在 Releases 列表中选择最新的 <code>Release-*.zip</code> 下载，下载后在 Docsy 点击「本地安装」选择该 zip 即可。',
    },
    macos: {
      url: 'https://poppler.freedesktop.org/',
      guide:
        'Poppler 官方主页。macOS 平台官方首选通过 Homebrew 自动化安装，<b>强烈建议直接点击「终端安装 (Homebrew)」</b>一键搞定。',
    },
  },
  qpdf: {
    label: 'qpdf',
    windows: {
      url: 'https://github.com/qpdf/qpdf/releases',
      guide:
        '在 Releases 列表中下载 <code>qpdf-*-msvc64.zip</code>（64 位版本），下载后在 Docsy 点击「本地安装」选择该 zip。',
    },
    macos: {
      url: 'https://github.com/qpdf/qpdf/releases',
      guide:
        'Qpdf 官方 Releases 页面。macOS 平台官方首选通过 Homebrew 自动化管理，<b>建议直接点击「终端安装 (Homebrew)」</b>一键完成。',
    },
  },
}

/**
 * 打开外部工具下载页面，并在跳转前提供平台专属的下载引导
 * @param {string} toolName 工具标识（如 'ffmpeg', 'qpdf', 'poppler'）
 * @param {string} [fallbackUrl] 兜底下载地址
 * @param {string} [fallbackLabel] 兜底工具名称
 */
export async function openToolDownloadWithGuide(toolName, fallbackUrl, fallbackLabel) {
  const spec = TOOL_DOWNLOAD_SPECS[toolName]
  const targetLabel = spec?.label || fallbackLabel || toolName
  let targetUrl = fallbackUrl
  let guideHtml = ''

  if (spec) {
    const platformConfig = isMac ? spec.macos : spec.windows
    targetUrl = platformConfig.url
    guideHtml = platformConfig.guide
  }

  if (!targetUrl) return

  try {
    await ElMessageBox.confirm(
      `<div style="line-height:1.8">
        <p style="font-size:14px;font-weight:600;margin-bottom:8px">即将打开 ${targetLabel} 官方下载页（${isMac ? 'macOS' : 'Windows'}）</p>
        ${
          guideHtml
            ? `<div style="background:var(--el-fill-color-light);padding:10px 12px;border-radius:6px;margin-bottom:10px;font-size:13px">
                📋 <strong>选择指引：</strong><br>${guideHtml}
              </div>`
            : ''
        }
        <p style="color:var(--el-text-color-secondary);font-size:12px;margin:0">
          ⚠️ 请从官方源下载，不要使用未经校验的第三方包。下载后可通过 Docsy 的「本地安装」按钮快速导入。
        </p>
      </div>`,
      '外部下载指引',
      {
        confirmButtonText: '打开下载页',
        cancelButtonText: '取消',
        dangerouslyUseHTMLString: true,
        type: 'info',
      },
    )
  } catch {
    return // 用户取消
  }

  const result = await openExternalUrl(targetUrl)
  if (!result.ok) {
    ElMessage.error(userFacingError(result.error, '无法打开下载页'))
  }
}

/**
 * 在 macOS 下通过系统终端与 Homebrew 安装指定外部工具
 * @param {string} toolName 工具标识（如 'ffmpeg', 'qpdf', 'poppler'）
 * @param {string} toolLabel 工具名称（如 'FFmpeg', 'qpdf', 'Poppler'）
 * @param {Function} [onFinished] 成功唤起终端后的回调
 */
export async function installToolViaTerminal(toolName, toolLabel, onFinished) {
  const targetName = toolLabel || toolName
  const res = await tauriCallSafe('open_terminal_to_install', { toolName })
  if (res.ok) {
    ElMessage.success(`已为您打开终端开始安装 ${targetName}。`)
    if (typeof onFinished === 'function') {
      onFinished()
    }
    return true
  } else {
    ElMessage.error(userFacingError(res.error, '打开终端失败'))
    return false
  }
}

/**
 * 在 macOS 下合并多个工具一次性打开终端进行批量安装
 * @param {string[]} toolNames 工具标识列表（如 ['ffmpeg', 'poppler', 'qpdf']）
 * @param {Function} [onStarted] 成功唤起终端后的回调
 */
export async function installToolsBatchViaTerminal(toolNames, onStarted) {
  if (!toolNames || toolNames.length === 0) return true
  const res = await tauriCallSafe('open_terminal_to_install', {
    toolName: toolNames.join(','),
  })
  if (res.ok) {
    ElMessage.success('已打开系统终端执行安装，安装过程中 Docsy 将实时检测状态。')
    if (typeof onStarted === 'function') {
      onStarted()
    }
    return true
  } else {
    ElMessage.error(userFacingError(res.error, '打开终端失败'))
    return false
  }
}

