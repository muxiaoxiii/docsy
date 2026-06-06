import { createApp } from "vue";
import ElementPlus from "element-plus";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import "element-plus/dist/index.css";
import App from "./App.vue";
import "./styles.css";
import { installAppLogger } from "./services/appLogger.js";

const app = createApp(App);
installAppLogger(app);
app.use(ElementPlus, { locale: zhCn });
app.mount("#app");
