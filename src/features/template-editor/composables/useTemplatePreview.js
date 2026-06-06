/**
 * 模板编辑器预览渲染
 *
 * 负责预览 HTML 的生成和更新。
 * 输入 session，输出 html。不直接 mutate session（除非函数名明确是 action）。
 *
 * 三种预览模式：
 * - marked：原文 + 高亮标记
 * - labels：plainText + <<标签>>
 * - edit：plainText + 不可编辑字段 token，固有字可直接编辑
 * - placeholder：旧模板占位符 + 高亮 label
 */

import { escapeHtml, escapeAttr } from "../utils/htmlEscape.js";
import {
  codePointLength,
  codePointSlice,
  codeUnitIndexToCodePointIndex,
} from "../utils/textRange.js";

const MARK_COLORS = [
  { bg: "#fff3cd", border: "#f59e0b" }, // 黄
  { bg: "#dbeafe", border: "#3b82f6" }, // 蓝
  { bg: "#dcfce7", border: "#22c55e" }, // 绿
  { bg: "#fce7f3", border: "#ec4899" }, // 粉
  { bg: "#e0e7ff", border: "#6366f1" }, // 靛
  { bg: "#fed7aa", border: "#f97316" }, // 橙
  { bg: "#e5e7eb", border: "#6b7280" }, // 灰
];

/**
 * 根据当前 session 状态计算预览 HTML
 */
export function computePreviewHtml(session) {
  const hasPlaceholders = /\{\{(?:[#*]?\w+|\?\w+:[^}]*)\}\}/.test(session.plainText);
  if (hasPlaceholders) {
    return computePlaceholderFallbackHtml(session.plainText, session.marks);
  }
  return computeMarkedHtml(session.preview.originalHtml, session.marks);
}

/**
 * 标签预览模式：plainText + <<标签>>
 */
export function computeLabelHtml(plainText, marks) {
  if (!plainText) return "";
  const sorted = [...marks].sort((a, b) => (a.start || 0) - (b.start || 0));
  const chunks = [];
  let cursor = 0;
  for (const m of sorted) {
    if (m.start < cursor) continue;
    let end = resolveMarkEnd(plainText, m);
    chunks.push(renderTextSegment(codePointSlice(plainText, cursor, m.start)));
    chunks.push(
      `<span class="docsy-label-preview">${escapeHtml(`<<${m.label || m.key}>>`)}</span>`
    );
    cursor = end;
  }
  chunks.push(renderTextSegment(codePointSlice(plainText, cursor)));
  return `<pre class="docsy-plain-label-preview">${chunks.join("")}</pre>`;
}

/**
 * 正文编辑模式：plainText + 不可编辑字段 token。
 *
 * 这个视图用于直接编辑固有字。它不是保存源，用户输入会被转换为
 * docx 文本替换 patch；字段 token 通过 data-start/data-end 映射回原文。
 */
export function computeEditableTextHtml(plainText, marks) {
  if (!plainText) return "";
  const sorted = [...marks].sort((a, b) => (a.start || 0) - (b.start || 0));
  const chunks = [];
  let cursor = 0;
  for (const m of sorted) {
    if (m.start < cursor) continue;
    const end = resolveMarkEnd(plainText, m);
    chunks.push(renderEditableFixedText(plainText, cursor, m.start));
    chunks.push(
      `<span class="docsy-field-token" contenteditable="false" data-key="${escapeAttr(m.key)}" data-start="${m.start}" data-end="${end}">${escapeHtml(`<<${m.label || m.key}>>`)}</span>`
    );
    cursor = end;
  }
  chunks.push(renderEditableFixedText(plainText, cursor, codePointLength(plainText)));
  return `<div class="docsy-edit-doc">${chunks.join("")}</div>`;
}

/**
 * 原文标记模式：originalHtml + 高亮
 */
export function computeMarkedHtml(originalHtml, marks) {
  if (!originalHtml) return "";
  let html = originalHtml;
  const sorted = [...marks].sort((a, b) => (a.start || 0) - (b.start || 0));

  // 统计每段文本出现了几种不同的 key
  const textKeys = {};
  for (const m of sorted) {
    const t = m.text.trim();
    if (!textKeys[t]) textKeys[t] = new Set();
    textKeys[t].add(m.key);
  }

  // 为需要区分颜色的文本分配颜色
  const keyColors = {};
  let colorIdx = 0;
  for (const m of sorted) {
    const t = m.text.trim();
    if (textKeys[t].size > 1) {
      if (!keyColors[m.key]) {
        keyColors[m.key] = MARK_COLORS[colorIdx % MARK_COLORS.length];
        colorIdx++;
      }
    }
  }

  for (const m of sorted) {
    if (!m.text) continue;
    const c = keyColors[m.key] || MARK_COLORS[0];
    html = replaceFirstEquivalentText(
      html,
      m.text,
      `<span class="docsy-marked" data-key="${escapeAttr(m.key)}" style="background:${c.bg};border-bottom-color:${c.border}">${escapeHtml(m.text)}</span>`
    );
  }
  return html;
}

/**
 * 旧模板占位符预览：将 {{field_N}} 替换为高亮的 <<label>>。
 * 条件前缀 {{?key:text}} 是实现语法，不作为正文显示；预览只显示 text。
 */
export function computePlaceholderFallbackHtml(plainText, marks) {
  if (!plainText) return "";
  const sorted = [...marks].sort((a, b) => (a.start || 0) - (b.start || 0));
  const chunks = [];
  let cursor = 0;

  // 条件前缀 {{?key:text}} 只显示 text；不能显示实现语法或 <<条件:key>>。
  let text = plainText;
  const conditionalRe = /\{\{\?(\w+):([^}]*)\}\}/g;
  let condMatch;
  const condReplacements = [];
  while ((condMatch = conditionalRe.exec(text)) !== null) {
    condReplacements.push({
      start: condMatch.index,
      end: condMatch.index + condMatch[0].length,
      key: condMatch[1],
      text: condMatch[2],
    });
  }

  // 合并 marks 和条件前缀，按位置排序
  const allItems = [
    ...sorted.map((m) => ({ type: "mark", ...m })),
    ...condReplacements.map((c) => ({ type: "conditional", ...c })),
  ].sort((a, b) => a.start - b.start);

  for (const item of allItems) {
    if (item.start < cursor) continue;

    chunks.push(renderTextSegment(codePointSlice(plainText, cursor, item.start)));

    if (item.type === "conditional") {
      chunks.push(escapeHtml(item.text));
      cursor = item.end;
    } else {
      // 普通 mark
      let end = resolveMarkEnd(plainText, item);
      chunks.push(
        `<span class="docsy-marked" data-key="${escapeAttr(item.key)}" style="background:#fff3cd;border-bottom-color:#f59e0b">${escapeHtml(`<<${item.label || item.key}>>`)}</span>`
      );
      cursor = end;
    }
  }
  chunks.push(renderTextSegment(codePointSlice(plainText, cursor)));
  return `<pre class="docsy-plain-label-preview">${chunks.join("")}</pre>`;
}

function renderTextSegment(text) {
  return escapeHtml(
    String(text || "").replace(/\{\{\?\w+:([^}]*)\}\}/g, "$1")
  );
}

function renderEditableFixedText(plainText, start, end) {
  if (end <= start) return "";
  const text = codePointSlice(plainText, start, end);
  return `<span class="docsy-fixed-text" data-start="${start}" data-end="${end}">${renderTextSegment(text)}</span>`;
}

/**
 * 解析 mark 的实际 end 位置（处理偏移不匹配的情况）
 */
function resolveMarkEnd(plainText, mark) {
  let end = mark.end;
  const slice = codePointSlice(plainText, mark.start, mark.end);
  if (mark.text && slice !== mark.text) {
    const unitStart = codePointSlice(plainText, 0, Math.max(0, mark.start - 10)).length;
    const idx = plainText.indexOf(mark.text, unitStart);
    const cpIdx = idx >= 0 ? codeUnitIndexToCodePointIndex(plainText, idx) : -1;
    if (cpIdx >= 0 && cpIdx < mark.start + 10) {
      end = cpIdx + codePointLength(mark.text);
    } else {
      end = mark.start + codePointLength(mark.text);
    }
  }
  return end;
}

/**
 * 替换 HTML 中第一个匹配的文本（支持空白归一）
 */
function replaceFirstEquivalentText(html, text, replacement) {
  const escaped = text
    .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
    .replace(/\s+/g, "\\s+");
  try {
    return html.replace(new RegExp(escaped), replacement);
  } catch {
    return html;
  }
}
