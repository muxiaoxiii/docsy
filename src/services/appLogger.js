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
        .map(([childKey, childValue]) => [
          childKey,
          sanitize(childValue, depth + 1, childKey),
        ])
    );
  }
  return String(value);
}

async function write(level, target, message, context = {}) {
  const entry = {
    level,
    target,
    message,
    context: sanitize(context),
  };
  try {
    await invoke("write_frontend_log", { entry });
  } catch (err) {
    const method = level === "error" ? "error" : level === "warn" ? "warn" : "debug";
    console[method]?.("[DocsyLogFallback]", entry, err);
  }
}

export function logDebug(target, message, context) {
  void write("debug", target, message, context);
}

export function logInfo(target, message, context) {
  void write("info", target, message, context);
}

export function logWarn(target, message, context) {
  void write("warn", target, message, context);
}

export function logError(target, message, context) {
  void write("error", target, message, context);
}

export function sanitizeLogContext(context) {
  return sanitize(context);
}

export function installAppLogger(app) {
  logInfo("app.lifecycle", "frontend.start", {
    userAgent: navigator.userAgent,
    language: navigator.language,
    location: window.location.href,
  });

  app.config.errorHandler = (err, instance, info) => {
    logError("frontend.vue", "vue.error", {
      info,
      component: instance?.$?.type?.name,
      error: err,
    });
  };

  window.addEventListener("error", (event) => {
    logError("frontend.window", "window.error", {
      message: event.message,
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno,
      error: event.error,
    });
  });

  window.addEventListener("unhandledrejection", (event) => {
    logError("frontend.promise", "unhandled_rejection", {
      reason: event.reason,
    });
  });
}
