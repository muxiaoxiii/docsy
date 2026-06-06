/**
 * 模板编辑器 Session 管理
 *
 * 负责 session 生命周期：
 * - loadFromFile: 从本地 docx 创建 session（create 模式）
 * - loadFromTemplateId: 从已有模板创建 session（edit 模式）
 * - resetSession: 重置为空状态
 * - markDirty: 标记为已修改
 * - clearDirty: 清除修改标记
 *
 * 约束：
 * - create/edit 初始化后必须得到同一 session 结构
 * - 所有 session mutation 通过明确 action 完成
 * - 不允许组件直接改深层字段
 */

import { ref, shallowRef } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";
import {
  readFileBytes,
  extractDocxText,
  readTemplateForEdit,
} from "../services/templateEditorApi.js";
import {
  createEmptySession,
  mapLocalDocxToSession,
  mapTemplateEditDataToSession,
  arrayBufferToBase64,
} from "../services/templateEditorMappers.js";
import {
  renderDocxHtmlFromBase64,
  renderDocxHtmlFromArrayBuffer,
} from "../services/docxPreviewService.js";
import { logError } from "../../../services/appLogger.js";

export function useTemplateEditorSession() {
  const session = shallowRef(createEmptySession());

  /**
   * 断言 session 处于 ready 状态
   */
  function assertReadySession() {
    if (!session.value || session.value.status === "idle") {
      throw new Error("Session 未初始化");
    }
  }

  /**
   * 替换整个 session
   */
  function replaceSession(nextSession) {
    session.value = nextSession;
  }

  /**
   * 浅层更新 session 字段
   */
  function updateSession(patch) {
    session.value = { ...session.value, ...patch };
  }

  /**
   * 标记为已修改
   */
  function markDirty() {
    updateSession({ dirty: true });
  }

  /**
   * 清除修改标记
   */
  function clearDirty() {
    updateSession({ dirty: false, lastSavedAt: new Date().toISOString() });
  }

  /**
   * 重置为空 session
   */
  function resetSession() {
    session.value = createEmptySession();
  }

  /**
   * 从本地 docx 文件加载（create 模式）
   */
  async function loadFromFile() {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: "Word", extensions: ["docx"] }],
    });
    if (!path) return false;

    try {
      updateSession({ status: "loadingSource" });

      const filename = path.split(/[\\/]/).pop();
      const arr = await readFileBytes(path);
      const arrayBuffer = new Uint8Array(arr).buffer;
      const base64 = arrayBufferToBase64(arrayBuffer);

      const { html } = await renderDocxHtmlFromArrayBuffer(arrayBuffer);
      const ext = await extractDocxText(path);

      const newSession = await mapLocalDocxToSession({
        path,
        filename,
        docxBase64: base64,
        plainText: ext.plain_text,
        html,
      });

      replaceSession(newSession);
      return true;
    } catch (err) {
      ElMessage.error(`加载失败：${err}`);
      updateSession({ status: "idle" });
      return false;
    }
  }

  /**
   * 从已有模板加载（edit 模式）
   */
  async function loadFromTemplateId(templateId) {
    try {
      updateSession({ status: "loadingSource" });

      const data = await readTemplateForEdit(templateId);
      const newSession = await mapTemplateEditDataToSession(templateId, data);

      // mammoth 预览
      const { html } = await renderDocxHtmlFromBase64(newSession.sourceDocxBase64);
      newSession.preview = {
        mode: "marked",
        html,
        originalHtml: html,
      };

      replaceSession(newSession);
      ElMessage.success(`已加载模板「${newSession.manifest.name}」进行编辑`);
      return true;
    } catch (err) {
      ElMessage.error(`加载模板失败：${err}`);
      updateSession({ status: "idle" });
      return false;
    }
  }

  /**
   * 刷新预览（从当前 sourceDocxBase64 重新渲染）
   */
  async function refreshPreviewFromDocx() {
    try {
      assertReadySession();
      const { html } = await renderDocxHtmlFromBase64(session.value.sourceDocxBase64);
      updateSession({
        preview: {
          ...session.value.preview,
          html,
          originalHtml: html,
        },
      });
    } catch (err) {
      // 预览刷新失败不破坏 session
      logError("template.editor.session", "refresh_preview.failed", {
        templateId: session.value?.templateId,
        mode: session.value?.mode,
        error: err,
      });
    }
  }

  return {
    session,
    assertReadySession,
    replaceSession,
    updateSession,
    markDirty,
    clearDirty,
    resetSession,
    loadFromFile,
    loadFromTemplateId,
    refreshPreviewFromDocx,
  };
}
