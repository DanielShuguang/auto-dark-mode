import { createApp } from "vue";
// base.css 必须排在 uno.css 之前：它里面有元素选择器（input/button 重置），
// 顺序反了会把 UnoCSS 的原子类覆盖掉。
import "./styles/base.css";
import "virtual:uno.css";
import App from "./App.vue";

createApp(App).mount("#app");
