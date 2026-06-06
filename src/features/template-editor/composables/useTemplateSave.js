/**
 * 模板编辑器保存逻辑
 *
 * 负责从 session 派生保存 payload，并调用后端保存命令。
 * View 组件不直接组装 fields/dictionaries/builder_state。
 */

import { saveUserTemplate } from "../services/templateEditorApi.js";
import { mapSessionToSaveArgs } from "../services/templateEditorMappers.js";

export async function saveTemplateSession(session) {
  const normalized = normalizeSessionForSave(session);
  const result = await saveUserTemplate(mapSessionToSaveArgs(normalized));
  return {
    result,
    marks: normalized.marks,
    dictionaries: normalized.dictionaries,
    manifest: normalized.manifest,
  };
}

export function normalizeSessionForSave(session) {
  const manifest = { ...session.manifest };
  if (!manifest.id) {
    manifest.id = `tpl_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  }

  const marks = [...session.marks]
    .map((m) => ({ ...m }))
    .sort((a, b) => (a.start || 0) - (b.start || 0));

  let fieldCounter = 1;
  for (const mark of marks) {
    if (mark.auto_number) {
      mark.key = "row";
    } else if (!mark.key) {
      while (marks.some((x) => x.key === `field_${fieldCounter}`)) {
        fieldCounter += 1;
      }
      mark.key = `field_${fieldCounter}`;
      fieldCounter += 1;
    }
  }

  const dictionaries = sanitizeDictionaries(session.dictionaries, marks);
  const fields = buildFields(marks);
  for (const field of fields) {
    if (!field.dict_source && dictionaries[field.key]?.length) {
      field.dict_source = field.key;
    }
  }

  return {
    ...session,
    manifest,
    marks,
    dictionaries,
  };
}

function buildFields(marks) {
  const fieldMap = new Map();
  for (const mark of marks.filter((m) => !m.auto_number)) {
    if (fieldMap.has(mark.key)) continue;
    fieldMap.set(mark.key, {
      key: mark.key,
      label: mark.label || mark.key,
      type: mark.row_repeat ? "party" : mark.type,
      visibility: mark.visibility,
      required: mark.required,
      value_suffix: mark.value_suffix || "",
      dict_source: mark.dict_source || undefined,
    });
  }
  return [...fieldMap.values()];
}

function sanitizeDictionaries(existingDictionaries = {}, marks = []) {
  const fieldKeys = new Set(marks.map((m) => m.key).filter(Boolean));
  const dictionaries = {};
  for (const [key, values] of Object.entries(existingDictionaries || {})) {
    const cleaned = uniqueCleanDictValues(values, fieldKeys);
    if (cleaned.length) dictionaries[key] = cleaned;
  }
  return dictionaries;
}

function uniqueCleanDictValues(values, fieldKeys) {
  const out = [];
  const seen = new Set();
  for (const value of values || []) {
    const normalized = normalizeDictValue(value);
    if (!normalized || isGeneratedPlaceholderValue(normalized, fieldKeys)) continue;
    if (seen.has(normalized)) continue;
    seen.add(normalized);
    out.push(value);
  }
  return out;
}

function normalizeDictValue(value) {
  if (value && typeof value === "object" && value.name) {
    return String(value.name).trim();
  }
  return String(value ?? "").trim();
}

function isGeneratedPlaceholderValue(value, fieldKeys) {
  if (/^\{\{[#*]?\w+\}\}$/.test(value)) return true;
  if (/^field_\d+$/.test(value)) return true;
  return fieldKeys.has(value);
}
