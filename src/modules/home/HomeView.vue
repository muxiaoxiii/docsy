<template>
  <div class="home-view">
    <div class="hero">
      <img src="../../assets/docsy-logo.png" alt="Docsy" class="hero-logo" />
      <h1>欢迎使用 Docsy</h1>
      <p class="subtitle">轻量、高效的本地文档处理工具箱</p>
    </div>
    <div class="cards">
      <el-card
        v-for="card in homeCards"
        :key="card.route"
        class="home-card"
        shadow="hover"
        @click="router.push({ name: card.route })"
      >
        <el-icon v-if="card.icon" :size="32"><component :is="card.icon" /></el-icon>
        <h3>{{ card.title }}</h3>
        <p>{{ card.description }}</p>
      </el-card>
    </div>
  </div>
</template>

<script setup>
import { useRouter } from 'vue-router'
import { getHomeCards } from '../../core/moduleRegistry.js'

const router = useRouter()
const homeCards = getHomeCards()
</script>

<style scoped>
.home-view {
  max-width: 800px;
  margin: 0 auto;
  padding: 40px 20px;
}

.hero {
  text-align: center;
  margin-bottom: 40px;
}

.hero-logo {
  width: 64px;
  height: 64px;
}

.hero h1 {
  margin: 16px 0 8px;
  color: #303133;
}

.subtitle {
  color: #909399;
  font-size: 14px;
}

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
}

.home-card {
  cursor: pointer;
  text-align: center;
  transition: transform 0.2s;
}

.home-card:hover {
  transform: translateY(-2px);
}

.home-card h3 {
  margin: 12px 0 4px;
  color: #303133;
}

.home-card p {
  color: #909399;
  font-size: 13px;
}
</style>
