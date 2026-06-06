/**
 * 文本范围工具函数
 */

export function codePointLength(text) {
  return Array.from(String(text || "")).length;
}

export function codePointSlice(text, start, end) {
  return Array.from(String(text || "")).slice(start, end).join("");
}

export function codeUnitIndexToCodePointIndex(text, unitIndex) {
  return Array.from(String(text || "").slice(0, unitIndex)).length;
}

/**
 * 标准化 mark 文本（折叠空白）
 */
export function normalizeMarkText(text) {
  return String(text || "").replace(/\s+/g, " ").trim();
}

/**
 * 检查两个范围是否重叠
 * rangeA overlaps rangeB <=> A.start < B.end && A.end > B.start
 */
export function rangesOverlap(a, b) {
  return a.start < b.end && a.end > b.start;
}

/**
 * 检查范围是否与任何已有 mark 重叠
 */
export function overlapsAnyMark(range, marks, exceptIndex = -1) {
  return marks.some((m, i) => {
    if (i === exceptIndex) return false;
    return rangesOverlap(range, m);
  });
}

/**
 * 在 plainText 中查找所有匹配的文本位置
 *
 * 处理 mammoth HTML 选中文本与 docx 纯文本之间的对齐：
 * - 字符相等：前进
 * - 两边都是空白：跳过双方的所有连续空白
 * - 原文里是空白、选中文本不是：跳过原文的空白
 * - 选中文本是空白、原文不是：失败
 */
export function findMatchingRangesInPlain(plainText, text, existingMarks = []) {
  const target = Array.from(text);
  const arr = Array.from(plainText);
  const isWS = (c) => c === " " || c === "\t" || c === "　" || c === "\n";
  const ranges = [];

  outer: for (let i = 0; i <= arr.length; i++) {
    let p = i;
    let q = 0;
    if (p < arr.length && isWS(arr[p]) && q < target.length && !isWS(target[q]))
      continue;
    while (q < target.length) {
      const a = arr[p];
      const b = target[q];
      if (a === undefined) continue outer;
      if (a === b) {
        p += 1;
        q += 1;
        continue;
      }
      if (isWS(a) && isWS(b)) {
        while (p < arr.length && isWS(arr[p])) p += 1;
        while (q < target.length && isWS(target[q])) q += 1;
        continue;
      }
      if (isWS(a)) {
        while (p < arr.length && isWS(arr[p])) p += 1;
        continue;
      }
      if (isWS(b)) {
        while (q < target.length && isWS(target[q])) q += 1;
        continue;
      }
      continue outer;
    }
    if (existingMarks.some((m) => i < m.end && p > m.start)) {
      continue;
    }
    ranges.push({
      key: `${i}-${p}`,
      start: i,
      end: p,
      label: buildLocationLabel(plainText, i, p, ranges.length + 1),
    });
  }
  return ranges;
}

/**
 * 构建位置候选标签
 */
function buildLocationLabel(plainText, start, end, index) {
  const before = codePointSlice(plainText, Math.max(0, start - 14), start).replace(/\s+/g, " ");
  const body = codePointSlice(plainText, start, end).replace(/\s+/g, " ");
  const after = codePointSlice(plainText, end, end + 14).replace(/\s+/g, " ");
  return `${index}. ${before}【${body}】${after}`.trim();
}

/**
 * 查找重复字段候选
 */
export function findDuplicateChoices(marks, text, range) {
  const normalized = normalizeMarkText(text);
  if (!normalized) return [];
  const seen = new Set();
  return marks
    .filter((m) => {
      if (range && range[0] === m.start && range[1] === m.end) return false;
      return normalizeMarkText(m.text) === normalized || m.text === text;
    })
    .filter((m) => {
      if (seen.has(m.key)) return false;
      seen.add(m.key);
      return true;
    })
    .map((m) => ({
      key: m.key,
      label: m.label,
      type: m.type,
      visibility: m.visibility,
      required: m.required,
      row_repeat: m.row_repeat,
      auto_number: m.auto_number,
      text: m.text,
    }));
}
