/**
 * 模板编辑器字段标记管理
 *
 * 负责字段标记的创建、编辑、删除。
 *
 * 约束：
 * - removeMark 只取消字段标记，不改变 docx 源文字
 * - 取消标记后，该文字自然回到固有字
 * - 不允许直接编辑字段字文字
 * - 新增 mark 前必须检查与已有 mark 是否重叠
 * - 同一个 key 可以有多个 mark，但生成页只出现一个字段
 * - mark 排序统一按 start ASC, end ASC
 */

import {
  codePointSlice,
  findMatchingRangesInPlain,
  findDuplicateChoices,
} from "../utils/textRange.js";

/**
 * 从选区创建 pending mark
 */
export function buildPendingMark(selectionText, plainText, existingMarks) {
  const locationChoices = findMatchingRangesInPlain(plainText, selectionText, existingMarks);
  const firstChoice = locationChoices[0];
  const range = firstChoice ? [firstChoice.start, firstChoice.end] : null;
  if (!range) {
    return null;
  }
  const duplicateChoices = findDuplicateChoices(existingMarks, selectionText, range);

  return {
    text: selectionText,
    start: range[0],
    end: range[1],
    key: "",
    label: "",
    type: guessType(selectionText),
    visibility: "value_only",
    required: false,
    row_repeat: false,
    auto_number: false,
    reuseKey: "",
    locationKey: firstChoice?.key || "",
    locationChoices,
    duplicateChoices,
  };
}

/**
 * 确认 pending mark，返回更新后的 marks 数组
 */
export function confirmPendingMark(pending, marks, editingMarkIndex) {
  if (!pending) return marks;

  // 处理 key 分配
  if (pending.auto_number) {
    pending.key = "row";
    pending.row_repeat = false;
  } else if (pending.reuseKey) {
    applyReuseField(pending, marks);
  } else {
    if (!pending.label.trim()) {
      return null; // 需要标签
    }
    if (editingMarkIndex < 0) {
      pending.key = nextFieldKey(marks);
    }
  }

  const newMarks = [...marks];
  if (editingMarkIndex >= 0) {
    newMarks.splice(editingMarkIndex, 1, { ...pending });
  } else {
    newMarks.push({ ...pending });
  }
  return newMarks;
}

/**
 * 应用复用字段属性
 */
function applyReuseField(pending, marks) {
  const src = marks.find((m) => m.key === pending.reuseKey);
  if (!src) return;
  pending.key = src.key;
  pending.label = src.label;
  pending.type = src.type;
  pending.visibility = src.visibility;
  pending.required = src.required;
  pending.row_repeat = src.row_repeat || false;
  pending.auto_number = src.auto_number || false;
}

/**
 * 应用位置选择
 */
export function applyLocationChoice(pending, key, plainText, marks) {
  if (!pending || !key) return pending;
  const choice = pending.locationChoices?.find((c) => c.key === key);
  if (!choice) return pending;

  pending.start = choice.start;
  pending.end = choice.end;
  pending.text = codePointSlice(plainText, choice.start, choice.end);
  pending.duplicateChoices = findDuplicateChoices(marks, pending.text, [
    choice.start,
    choice.end,
  ]);
  if (
    pending.reuseKey &&
    !pending.duplicateChoices.some((c) => c.key === pending.reuseKey)
  ) {
    pending.reuseKey = "";
  }
  return pending;
}

/**
 * 取消标记，返回更新后的 marks 数组
 */
export function removeMark(index, marks) {
  const newMarks = [...marks];
  newMarks.splice(index, 1);
  return newMarks;
}

/**
 * 生成下一个字段 key
 */
export function nextFieldKey(marks) {
  let i = marks.length + 1;
  while (marks.some((m) => m.key === `field_${i}`)) i += 1;
  return `field_${i}`;
}

/**
 * 根据选中文本猜测字段类型
 */
export function guessType(text) {
  if (!text) return "text";
  const t = text.trim();
  if (/年\s*月\s*日/.test(t) || /\d{4}\s*年/.test(t)) return "date";
  if (/^\d+$/.test(t)) return "text";
  return "text";
}

/**
 * 根据选中文本推荐标签
 */
export function suggestLabel(text) {
  if (!text) return "如 法院";
  const t = text.trim();
  if (t.length <= 6) return `如 ${t}`;
  return `如 ${t.slice(0, 6)}…`;
}
