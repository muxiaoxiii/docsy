/**
 * 模板编辑器固有字编辑
 *
 * 负责固有字的纯文字替换。
 *
 * 约束：
 * - 替换范围不得与任何 mark 重叠
 * - 替换必须调用后端 edit_docx_text_range
 * - 替换成功后重新提取 plainText、重新 mammoth 渲染、调整 marks
 * - 后续 marks 偏移必须按 replacement 长度差调整
 */

import {
  extractDocxTextFromBase64,
  editDocxTextRange,
} from "../services/templateEditorApi.js";
import { renderDocxHtmlFromBase64 } from "../services/docxPreviewService.js";
import { codePointLength, findMatchingRangesInPlain } from "../utils/textRange.js";

/**
 * 解析可编辑的文本范围
 *
 * 返回在 plainText 中匹配的所有位置候选。
 * 如果没有候选，说明选区可能包含字段字或无法定位。
 */
export function resolveEditableTextRange(plainText, selectionText, marks) {
  return findMatchingRangesInPlain(plainText, selectionText, marks);
}

/**
 * 检查范围是否与任何 mark 重叠
 *
 * 如果重叠，说明该范围是字段字，不能直接编辑。
 */
export function assertRangeIsFixedText(range, marks) {
  return !marks.some((m) => range.start < m.end && range.end > m.start);
}

/**
 * 调整 marks 偏移
 *
 * 替换范围后的 mark 需要按 delta 调整位置。
 *
 * | mark 位置 | 处理 |
 * |---|---|
 * | mark.end <= range.start | 不变 |
 * | mark.start >= range.end | start/end += delta |
 * | 与 range 重叠 | 理论上不应发生（已提前检查） |
 */
export function shiftMarksAfterTextEdit(marks, range, delta) {
  if (delta === 0) return marks;
  return marks.map((m) => {
    if (m.start >= range.end) {
      return { ...m, start: m.start + delta, end: m.end + delta };
    }
    return m;
  });
}

/**
 * 执行固有字替换
 *
 * 完整流程：
 * 1. 调用后端 edit_docx_text_range
 * 2. 计算 delta
 * 3. 调整 marks 偏移
 * 4. 重新提取 plainText
 * 5. 重新渲染 mammoth 预览
 *
 * 返回更新后的 session patch，调用方负责 updateSession。
 */
export async function applyTextReplacement(docxBase64, range, replacement) {
  const nextBase64 = await editDocxTextRange({
    docxBase64,
    start: range.start,
    end: range.end,
    replacement,
  });

  // range 使用 docx 纯文本字符偏移；这里也按 Unicode code point 计算。
  const delta = codePointLength(replacement) - (range.end - range.start);

  // 重新提取 plainText 和预览
  const ext = await extractDocxTextFromBase64(nextBase64);
  const { html } = await renderDocxHtmlFromBase64(nextBase64);

  return {
    sourceDocxBase64: nextBase64,
    plainText: ext.plain_text,
    previewHtml: html,
    originalHtml: html,
    delta,
  };
}
