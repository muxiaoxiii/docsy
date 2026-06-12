import { createRouter, createWebHistory } from 'vue-router'
import { getRoutes } from '../core/moduleRegistry.js'

const routes = [
  {
    path: '/',
    name: 'home',
    component: () => import('../modules/home/HomeView.vue'),
  },
  ...getRoutes(),
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router
