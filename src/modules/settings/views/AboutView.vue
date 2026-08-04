<template>
  <div class="about-view">
    <div class="about-content">
      <div class="about-brand">
        <img src="../../../assets/docsy-logo.png" alt="Docsy" class="about-logo" />
        <h1 class="about-title">Docsy</h1>
        <span class="about-version">v{{ version }}</span>
      </div>

      <p class="about-desc">轻量本地文档处理工具箱</p>

      <div class="about-links">
        <a href="https://github.com/muxiaoxiii/docsy" target="_blank" class="about-link">
          <el-icon><Link /></el-icon>
          GitHub
        </a>
        <a href="http://xhslink.com/m/A3Ibe0w27UC" target="_blank" class="about-link">
          <el-icon><Link /></el-icon>
          小红书
        </a>
      </div>

      <div class="about-qr">
        <img src="../../../assets/docsy-qrcode.png" alt="小红书二维码" class="about-qr-img" />
        <span class="about-qr-label">扫码关注小红书</span>
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
import { Link } from '@element-plus/icons-vue'

const version = '0.9.2'
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

.about-links {
  display: flex;
  gap: 24px;
  margin-bottom: 24px;
}

.about-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  color: var(--docsy-primary);
  text-decoration: none;
  transition: opacity 0.15s;
}

.about-link:hover {
  opacity: 0.7;
}

.about-qr {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
}

.about-qr-img {
  width: 180px;
  height: 180px;
  object-fit: contain;
  border-radius: 12px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
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
