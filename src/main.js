import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'

import App from './App.vue'
import router from './router'
import './styles.css'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(ElementPlus, { locale: zhCn })

// 禁用默认右键上下文菜单（避免唤起 webview 默认的浏览器右键行为）
document.addEventListener('contextmenu', (e) => {
  e.preventDefault()
})

app.mount('#app')
