import { invoke } from "@tauri-apps/api/core";

const MAX_STRING_LENGTH = 600;
const MAX_ARRAY_ITEMS = 8;
const MAX_OBJECT_KEYS = 20;

function looksSensitiveKey(key) {
  return /base64|docx|bytes|content|html|xml|text/i.test(key);
}

function sanitize(value, depth = 0, key = "") {
  if (value == null) return value;
  if (value instanceof Error) {
    return {
      name: value.name,
      message: value.message,
      code: value.code,
      meta: sanitize(value.meta, depth + 1),
      stack: typeof value.stack === "string"
        ? value.stack.split("\n").slice(0, 4).join("\n")
        : undefined,
    };
  }
  if (typeof value === "string") {
    if (looksSensitiveKey(key)) return `[string length=${value.length}]`;
    if (value.length > MAX_STRING_LENGTH) {
      return `${value.slice(0, MAX_STRING_LENGTH)}...[length=${value.length}]`;
    }
    return value;
  }
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (Array.isArray(value)) {
    return {
      length: value.length,
      sample: value.slice(0, MAX_ARRAY_ITEMS).map((item) => sanitize(item, depth + 1)),
    };
  }
  if (typeof value === "object") {
    if (depth >= 3) return "[object]";
    return Object.fromEntries(
      Object.entries(value)
        .slice(0, MAX_OBJECT_KEYS)
        .map(([k, v]) => [k, sanitize(v, depth + 1, k)])
    );
  }
  return String(value);
}

async function send(level, target, message, context) {
  try {
    await invoke("write_frontend_log", {
      level,
      target,
      message,
      context: context ? JSON.stringify(sanitize(context)) : null,
    });
  } catch {
    // log write failure must not affect main flow
  }
}

export function logDebug(target, message, context) {
  return send("debug", target, message, context);
}

export function logInfo(target, message, context) {
  return send("info", target, message, context);
}

export function logWarn(target, message, context) {
  return send("warn", target, message, context);
}

export function logError(target, message, context) {
  return send("error", target, message, context);
}
