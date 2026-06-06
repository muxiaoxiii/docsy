/**
 * docx 预览服务
 *
 * 封装 mammoth 调用，不直接在 composable 中 import mammoth。
 * 后续如果替换 mammoth 或补 source map，只改这一处。
 */

import mammoth from "mammoth/mammoth.browser.js";
import { base64ToArrayBuffer } from "./templateEditorMappers.js";

/**
 * 从 base64 渲染 docx HTML
 */
export async function renderDocxHtmlFromBase64(base64) {
  const arrayBuffer = base64ToArrayBuffer(base64);
  return renderDocxHtmlFromArrayBuffer(arrayBuffer);
}

/**
 * 从 ArrayBuffer 渲染 docx HTML
 */
export async function renderDocxHtmlFromArrayBuffer(arrayBuffer) {
  const result = await mammoth.convertToHtml(
    { arrayBuffer },
    { includeDefaultStyleMap: true }
  );
  return {
    html: result.value,
    messages: result.messages,
  };
}
