<template>
  <div class="home-view">
    <section class="home-hero">
      <div class="home-hero-copy">
        <h2 class="home-greeting">
          <img class="home-title-logo" src="../assets/docsy-logo.png" alt="" />
          <span>欢迎使用 Docsy</span>
        </h2>
        <p class="home-subtitle">轻量、高效的本地文档处理工具箱</p>
      </div>
      <img class="home-mascot" src="../assets/doclet-mascot.png" alt="" />
    </section>

    <el-row :gutter="16" class="home-quick">
      <el-col :span="8">
        <el-card shadow="hover" class="home-card" @click="emit('navigate', 'pdf:unlock')">
          <div class="home-card-icon">PDF</div>
          <div class="home-card-title">PDF 工具</div>
          <div class="home-card-desc">解锁、整理证据、添加页眉页脚</div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover" class="home-card" @click="emit('navigate', 'imagePaddler')">
          <div class="home-card-icon">IMG</div>
          <div class="home-card-title">图片排版</div>
          <div class="home-card-desc">图片批量排版为 A4 文档</div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover" class="home-card" @click="emit('navigate', 'videoExtract')">
          <div class="home-card-icon">VID</div>
          <div class="home-card-title">视频抽帧</div>
          <div class="home-card-desc">按时间或频率导出视频帧</div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
const emit = defineEmits(["navigate"]);
</script>

<style scoped>
.home-view {
  max-width: 960px;
  margin: 0 auto;
}
.home-hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  min-height: 132px;
  margin-bottom: 20px;
}
.home-hero-copy {
  min-width: 0;
}
.home-greeting {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 24px;
  margin: 0 0 4px;
}
.home-title-logo {
  width: 38px;
  height: 38px;
  object-fit: contain;
  flex: 0 0 auto;
}
.home-subtitle {
  color: #6b7280;
  margin: 0;
}
.home-mascot {
  width: clamp(92px, 14vw, 132px);
  height: clamp(100px, 15vw, 148px);
  object-fit: contain;
  flex: 0 0 auto;
}
.home-card {
  text-align: center;
  cursor: pointer;
  min-height: 142px;
  transition: transform 0.15s ease;
}
.home-card:hover {
  transform: translateY(-2px);
}
.home-card-icon {
  font-size: 20px;
  font-weight: 700;
  color: #2563eb;
  margin-bottom: 12px;
}
.home-card-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 6px;
}
.home-card-desc {
  font-size: 12px;
  color: #6b7280;
}
</style>
