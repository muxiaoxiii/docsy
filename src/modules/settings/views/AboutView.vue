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
          <path
            d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"
          />
        </svg>
        github.com/muxiaoxiii/docsy
      </button>

      <button type="button" class="about-site" @click="openSite">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M12 6h-6a2 2 0 0 0 -2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-6" />
          <path d="M11 13l9 -9" />
          <path d="M15 4h5v5" />
        </svg>
        访问官网 docsy.muxiaoxi.top
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
import { openExternalUrl } from '../../../core/tauriBridge.js'
import { DOCSY_SITE_URL } from '../../../core/siteConfig.js'

const version = import.meta.env.PACKAGE_VERSION || '开发版'

function openGitHub() {
  openExternalUrl('https://github.com/muxiaoxiii/docsy')
}

function openSite() {
  openExternalUrl(DOCSY_SITE_URL)
}
</script>

<style scoped>
.about-view {
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding-block: clamp(28px, 6.2dvh, 56px);
  padding-inline: 32px;
  min-height: 100%;
}

.about-content {
  display: grid;
  grid-template-columns: minmax(260px, 1fr) 220px;
  grid-template-areas:
    'brand qr'
    'desc qr'
    'github qr'
    'site qr'
    'divider divider'
    'legal legal';
  align-items: center;
  max-width: 720px;
  width: 100%;
  padding-block: clamp(30px, 4.7dvh, 42px) clamp(24px, 3.4dvh, 30px);
  padding-inline: 46px;
  column-gap: 54px;
  background: var(--docsy-surface-elevated);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  box-shadow: var(--docsy-shadow-panel);
}

.about-brand {
  grid-area: brand;
  display: flex;
  align-items: center;
  justify-self: start;
  gap: 14px;
  margin: 0 0 clamp(8px, 1.4dvh, 10px);
}

.about-logo {
  width: 72px;
  height: 72px;
  object-fit: contain;
}

.about-title {
  font-family:
    ui-rounded,
    'SF Pro Rounded',
    -apple-system,
    'PingFang SC',
    sans-serif;
  font-size: 34px;
  font-weight: 760;
  color: var(--docsy-text-strong);
  margin: 0;
}

.about-version {
  align-self: flex-end;
  margin-bottom: 6px;
  padding: 4px 7px;
  font-size: 13px;
  color: var(--docsy-text-muted);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 999px;
}

.about-desc {
  grid-area: desc;
  font-size: 15px;
  color: var(--docsy-text);
  margin: 0 0 clamp(18px, 2.7dvh, 24px);
}

.about-github {
  grid-area: github;
  justify-self: start;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text-strong);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition:
    background 0.15s,
    border-color 0.15s;
}

.about-github:hover {
  background: var(--docsy-surface-muted);
  border-color: var(--docsy-primary);
}

.about-site {
  grid-area: site;
  justify-self: start;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: 1px solid var(--docsy-primary);
  border-radius: var(--docsy-radius);
  background: var(--docsy-primary-soft);
  color: var(--docsy-primary-hover);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition:
    background 0.15s,
    border-color 0.15s;
}

.about-site:hover {
  background: color-mix(in srgb, var(--docsy-primary-soft) 70%, var(--docsy-primary));
  color: var(--docsy-surface-elevated);
}

.about-site svg {
  flex-shrink: 0;
}

.about-qr {
  grid-area: qr;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  margin: 0;
}

.about-qr-img {
  width: 160px;
  height: 160px;
  object-fit: contain;
  border-radius: var(--docsy-radius);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.06);
}

.about-qr-label {
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.about-legal {
  grid-area: legal;
  text-align: center;
}

.about-content :deep(.el-divider) {
  grid-area: divider;
  margin-block: clamp(22px, 3.4dvh, 30px) clamp(16px, 2.5dvh, 22px);
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

@media (max-width: 700px) {
  .about-content {
    grid-template-columns: 1fr;
    grid-template-areas:
      'brand'
      'desc'
      'github'
      'site'
      'qr'
      'divider'
      'legal';
    justify-items: center;
    padding: 34px 28px 26px;
  }

  .about-brand,
  .about-github,
  .about-site {
    justify-self: center;
  }

  .about-desc {
    text-align: center;
  }

  .about-qr {
    margin-top: 28px;
  }
}
</style>
