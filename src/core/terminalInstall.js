import { ElMessage, ElMessageBox } from 'element-plus'
import { tauriCallSafe, userFacingError } from './tauriBridge.js'

export const isMac = /mac/i.test(navigator.platform || navigator.userAgent)

/**
 * 在 macOS 下通过系统终端与 Homebrew 安装指定外部工具
 * @param {string} toolName 工具标识（如 'ffmpeg', 'qpdf', 'poppler'）
 * @param {string} toolLabel 工具名称（如 'FFmpeg', 'qpdf', 'Poppler'）
 * @param {Function} onFinished 成功唤起终端后的回调
 */
export async function installToolViaTerminal(toolName, toolLabel, onFinished) {
  // 1. 检查是否安装了 Homebrew
  const brewRes = await tauriCallSafe('check_homebrew_installed')
  const hasBrew = brewRes.ok && Boolean(brewRes.data)

  if (hasBrew) {
    const cmdDesc =
      toolName === 'ffmpeg'
        ? `安装 ${toolLabel} (Full 完整版，支持 drawtext 时间戳水印)`
        : `执行「brew install ${toolName}」安装 ${toolLabel}`
    try {
      await ElMessageBox.confirm(
        `即将打开系统终端${cmdDesc}。安装过程可能需要数分钟，是否继续？`,
        `终端安装 ${toolLabel}`,
        {
          confirmButtonText: '打开终端安装',
          cancelButtonText: '取消',
          type: 'info',
        },
      )
    } catch {
      return false
    }

    const res = await tauriCallSafe('open_terminal_to_install', { toolName })
    if (res.ok) {
      ElMessage.success(`已为您打开终端开始安装 ${toolLabel}。终端安装完成后，请返回 Docsy 检测确认。`)
      if (typeof onFinished === 'function') {
        onFinished()
      }
      return true
    } else {
      ElMessage.error(userFacingError(res.error, '打开终端失败'))
      return false
    }
  } else {
    // 2. 未安装 Homebrew 时的引导
    try {
      await ElMessageBox.confirm(
        `检测到您的系统尚未安装 Homebrew 包管理器。\n\n• 点击「安装 Homebrew」，将自动为您打开终端运行国内极速镜像安装脚本；\n• 或者您可以选择手动前往官网下载离线工具包。`,
        '未检测到 Homebrew',
        {
          confirmButtonText: '安装 Homebrew',
          cancelButtonText: '取消',
          type: 'warning',
        },
      )
    } catch {
      return false
    }

    const res = await tauriCallSafe('open_terminal_to_install', { toolName: 'homebrew' })
    if (res.ok) {
      ElMessage.success('已为您打开终端开始安装 Homebrew。安装成功后再回来安装外部工具。')
      return true
    } else {
      ElMessage.error(userFacingError(res.error, '打开终端失败'))
      return false
    }
  }
}
