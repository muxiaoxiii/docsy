/**
 * 模板编辑器数据映射层
 *
 * 负责后端数据和前端 session 之间的转换。
 * 旧模板没有 builder_state.json 时，只允许 mapper 做降级反推。
 */

import { extractDocxTextFromBase64 } from "./templateEditorApi.js";

/**
 * 创建空 session
 */
export function createEmptySession() {
  return {
    status: "idle",
    mode: null,
    sourceKind: null,
    templateId: null,
    versionId: null,
    sourceFilename: "",
    manifest: {
      id: "",
      name: "",
      type: "custom",
      version: "1.0.0",
      created_at: "",
    },
    sourceDocxBase64: "",
    plainText: "",
    marks: [],
    dictionaries: {},
    preview: {
      mode: "marked",
      html: "",
      originalHtml: "",
    },
    selection: null,
    pendingMark: null,
    dirty: false,
    lastSavedAt: null,
    diagnostics: [],
  };
}

/**
 * 从本地 docx 文件创建 session
 */
export async function mapLocalDocxToSession({ path, filename, docxBase64, plainText, html }) {
  const session = createEmptySession();
  session.status = "ready";
  session.mode = "create";
  session.sourceKind = "localDocx";
  session.templateId = null;
  session.sourceFilename = filename;
  session.manifest = {
    id: "",
    name: filename.replace(/\.docx$/i, ""),
    type: "custom",
    version: "1.0.0",
    created_at: new Date().toISOString(),
  };
  session.sourceDocxBase64 = docxBase64;
  session.plainText = plainText;
  session.preview = {
    mode: "marked",
    html: html,
    originalHtml: html,
  };
  return session;
}

/**
 * 从后端 read_template_for_edit 数据创建 session
 */
export async function mapTemplateEditDataToSession(templateId, data) {
  const session = createEmptySession();
  session.status = "ready";
  session.mode = "edit";
  session.templateId = templateId;

  // manifest
  session.manifest = data.manifest || {};
  if (!session.manifest.id) {
    session.manifest.id = templateId;
  }
  if (!session.manifest.name) {
    session.manifest.name = templateId;
  }
  session.sourceFilename = session.manifest.name || templateId;

  // sourceDocxBase64：优先使用 builder_state 中的原始 docx
  session.sourceDocxBase64 = data.docxBase64 || "";

  // 判断 sourceKind
  if (templateId === "letter") {
    session.sourceKind = "builtinTemplate";
  } else {
    session.sourceKind = "currentTemplate";
  }

  // dictionaries
  session.dictionaries = data.dictionaries || {};

  // 提取 plainText
  try {
    const ext = await extractDocxTextFromBase64(session.sourceDocxBase64);
    session.plainText = ext.plain_text;
  } catch {
    session.plainText = "";
  }

  // marks
  const savedMarks = data.builderState?.marks;
  const fieldByKey = new Map(
    (data.fields?.fields || []).map((field) => [field.key, field])
  );
  if (Array.isArray(savedMarks) && savedMarks.length) {
    // 有 builder_state，使用保存的 marks
    session.marks = savedMarks.map((m) => ({
      ...m,
      ...fieldPatchForMark(m, fieldByKey),
      text: m.text || session.plainText.slice(m.start, m.end),
    }));
  } else {
    // 旧模板，从占位符反推 marks
    session.marks = mapLegacyPlaceholdersToMarks(data, session.plainText);
  }

  return session;
}

function fieldPatchForMark(mark, fieldByKey) {
  const key = mark.auto_number ? "row" : String(mark.key || "").replace(/^[*#]/, "");
  const field = fieldByKey.get(key);
  if (!field) return {};
  return {
    label: field.label || mark.label || key,
    type: field.type || mark.type || "text",
    visibility: field.visibility || mark.visibility || "value_only",
    required: !!field.required,
    dict_source: field.dict_source || mark.dict_source || undefined,
    value_suffix: field.value_suffix || mark.value_suffix || "",
  };
}

/**
 * 旧模板占位符反推 marks
 */
function mapLegacyPlaceholdersToMarks(data, plainText) {
  return (data.marks || []).map((m) => {
    const rawKey = String(m.key || "");
    const key = rawKey.replace(/^[*#]/, "");
    const field = (data.fields?.fields || []).find((f) => f.key === key);
    return {
      start: m.start,
      end: m.end,
      key,
      label: field?.label || key,
      type: field?.type || "text",
      visibility: field?.visibility || "value_only",
      required: field?.required || false,
      row_repeat: rawKey.startsWith("*"),
      auto_number: rawKey === "#row" || rawKey === "row",
      text: plainText.slice(m.start, m.end),
    };
  });
}

/**
 * 从 marks 派生 fields
 */
export function mapMarksToFields(marks) {
  const fieldMap = new Map();
  for (const m of marks.filter((m) => !m.auto_number)) {
    if (fieldMap.has(m.key)) continue;
    fieldMap.set(m.key, {
      key: m.key,
      label: m.label || m.key,
      type: m.row_repeat ? "party" : m.type,
      visibility: m.visibility,
      required: m.required,
      value_suffix: m.value_suffix || "",
      dict_source: m.dict_source || undefined,
    });
  }
  return [...fieldMap.values()];
}

/**
 * 从 marks 派生或合并 dictionaries
 */
export function buildDictionariesFromMarks(marks, existingDictionaries = {}) {
  return { ...existingDictionaries };
}

/**
 * 构造 builder_state
 */
export function mapMarksToBuilderState(session) {
  return {
    version: 1,
    sourceDocxBase64: session.sourceDocxBase64,
    sourceFilename: session.sourceFilename,
    marks: session.marks.map((m) => ({ ...m })),
    savedAt: new Date().toISOString(),
  };
}

/**
 * 将 session 转换为 save_user_template 的参数
 */
export function mapSessionToSaveArgs(session) {
  const fields = mapMarksToFields(session.marks);

  // 关联字典源
  for (const f of fields) {
    if (!f.dict_source && session.dictionaries[f.key]) {
      f.dict_source = f.key;
    }
  }

  return {
    manifest: session.manifest,
    fields: { fields },
    dictionaries: session.dictionaries,
    original_docx_base64: session.sourceDocxBase64,
    builder_state: mapMarksToBuilderState(session),
    marks: session.marks.map((m) => ({
      start: m.start,
      end: m.end,
      key: m.auto_number ? "#row" : m.row_repeat ? `*${m.key}` : m.key,
    })),
  };
}

/**
 * base64 转 ArrayBuffer
 */
export function base64ToArrayBuffer(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes.buffer;
}

/**
 * ArrayBuffer 转 base64
 */
export function arrayBufferToBase64(buf) {
  const u8 = new Uint8Array(buf);
  let bin = "";
  for (let i = 0; i < u8.byteLength; i++) bin += String.fromCharCode(u8[i]);
  return btoa(bin);
}
