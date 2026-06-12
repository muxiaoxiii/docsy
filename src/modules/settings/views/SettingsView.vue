<template>
  <div class="settings-view">
    <h2>设置</h2>
    <el-form label-width="120px" class="settings-form">
      <el-form-item label="历史上限">
        <el-input-number v-model="settings.history_max" :min="10" :max="500" />
      </el-form-item>
      <el-form-item label="LibreOffice 路径">
        <el-input v-model="settings.libreoffice_path" placeholder="留空则自动检测" />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="saveSettings">保存设置</el-button>
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { tauriCall } from '../../../core/tauriBridge.js'

const settings = ref({
  history_max: 50,
  menu_visibility: {},
  libreoffice_path: '',
})

async function loadSettings() {
  try {
    const s = await tauriCall('get_app_settings')
    settings.value = { ...settings.value, ...s }
  } catch (err) {
    console.error('Failed to load settings:', err)
  }
}

async function saveSettings() {
  try {
    await tauriCall('set_app_settings', { settings: settings.value })
    console.log('Settings saved')
  } catch (err) {
    console.error('Failed to save settings:', err)
  }
}

onMounted(loadSettings)
</script>

<style scoped>
.settings-view {
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
}

.settings-view h2 {
  margin-bottom: 24px;
  color: #303133;
}
</style>
