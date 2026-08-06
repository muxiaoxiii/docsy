<template>
  <div class="about-view">
    <div class="about-content">
      <div class="about-brand">
        <img src="../../../assets/docsy-logo.png" alt="Docsy" class="about-logo" />
        <h1 class="about-title">Docsy</h1>
        <span class="about-version">v{{ version }}</span>
      </div>

      <p class="about-desc">轻量本地文档处理工具箱</p>

      <button type="button" class="about-github" @click="openGitHub">
        <svg viewBox="0 0 16 16" width="18" height="18" fill="currentColor">
          <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
        </svg>
        github.com/muxiaoxiii/docsy
      </button>

      <div class="about-qr">
        <span class="about-qr-label">小红书</span>
        <img src="../../../assets/docsy-qrcode.png" alt="小红书二维码" class="about-qr-img" />
        <span class="about-qr-label">扫码关注</span>
      </div>

      <el-divider />

      <div class="about-legal">
        <p class="about-license">License: MIT</p>
        <p class="about-copyright">© 2026 木小樨 (muxiaoxi). All rights reserved.</p>
        <p class="about-note">本软件为开源项目，欢迎贡献，但请勿移除版权声明</p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { open } from '@tauri-apps/plugin-shell'

const version = import.meta.env.PACKAGE_VERSION || '0.9.6'

function openGitHub() {
  open('https://github.com/muxiaoxiii/docsy')
}
</script>

<style scoped>
.about-view {
  display: flex;
  justify-content: center;
  padding: 60px 24px;
  min-height: 100%;
}

.about-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  max-width: 400px;
  width: 100%;
}

.about-brand {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.about-logo {
  width: 72px;
  height: 72px;
  object-fit: contain;
}

.about-title {
  font-size: 28px;
  font-weight: 700;
  color: var(--docsy-text-strong);
  margin: 0;
}

.about-version {
  font-size: 13px;
  color: var(--docsy-text-muted);
}

.about-desc {
  font-size: 15px;
  color: var(--docsy-text);
  margin: 0 0 28px;
}

.about-github {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 8px;
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text-strong);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.about-github:hover {
  background: var(--docsy-surface-muted);
  border-color: var(--docsy-primary);
}

.about-qr {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  margin-top: 28px;
}

.about-qr-img {
  width: 160px;
  height: 160px;
  object-fit: contain;
  border-radius: 10px;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.06);
}

.about-qr-label {
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.about-legal {
  text-align: center;
}

.about-license {
  font-size: 13px;
  color: var(--docsy-text-muted);
  margin: 0 0 4px;
}

.about-copyright {
  font-size: 13px;
  color: var(--docsy-text-muted);
  margin: 0 0 12px;
}

.about-note {
  font-size: 12px;
  color: var(--docsy-text-muted);
  margin: 0;
  opacity: 0.8;
}
</style>
