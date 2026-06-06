/**
 * 模板编辑器 API 层
 *
 * 封装所有 Tauri invoke 调用，组件和 composables 不直接 import invoke。
 * 错误统一抛出 TemplateEditorError，不弹 UI 消息。
 */

import { invoke } from "@tauri-apps/api/core";
import { logDebug, logError, sanitizeLogContext } from "../../../services/appLogger.js";

export class TemplateEditorError extends Error {
  constructor(code, message, meta = {}) {
    super(message);
    this.code = code;
    this.meta = meta;
  }
}

function wrapError(command, err) {
  return new TemplateEditorError(
    "BACKEND_COMMAND_FAILED",
    String(err),
    { command }
  );
}

function summarizeArgs(args) {
  return sanitizeLogContext(args);
}

async function invokeEditor(command, args = {}) {
  const startedAt = performance.now();
  logDebug("template.editor.api", `${command}.start`, {
    command,
    args: summarizeArgs(args),
  });
  try {
    const result = await invoke(command, args);
    logDebug("template.editor.api", `${command}.success`, {
      command,
      durationMs: Math.round(performance.now() - startedAt),
    });
    return result;
  } catch (err) {
    logError("template.editor.api", `${command}.failed`, {
      command,
      args: summarizeArgs(args),
      durationMs: Math.round(performance.now() - startedAt),
      error: err,
    });
    throw wrapError(command, err);
  }
}

/**
 * 读取文件字节
 */
export async function readFileBytes(path) {
  return await invokeEditor("read_file_bytes", { path });
}

/**
 * 从文件路径提取 docx 纯文本
 */
export async function extractDocxText(path) {
  return await invokeEditor("extract_docx_text", { path });
}

/**
 * 从 base64 提取 docx 纯文本
 */
export async function extractDocxTextFromBase64(base64) {
  return await invokeEditor("extract_docx_text_from_base64", { base64 });
}

/**
 * 读取模板编辑数据
 */
export async function readTemplateForEdit(templateId) {
  return await invokeEditor("read_template_for_edit", { id: templateId });
}

/**
 * 编辑 docx 中的固有字
 */
export async function editDocxTextRange(args) {
  return await invokeEditor("edit_docx_text_range", { args });
}

/**
 * 保存用户模板
 */
export async function saveUserTemplate(args) {
  return await invokeEditor("save_user_template", { args });
}
