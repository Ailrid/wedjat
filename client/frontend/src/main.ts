import "reflect-metadata";
// main.ts
import "cesium/Build/Cesium/Widgets/widgets.css";

import "@/assets/main.css";
import { createApp } from "vue";
import App from "./App.vue";
import router from "@/router";
import { bootstrapVirid } from "./ccs";
bootstrapVirid();

const app = createApp(App);
app.use(router);
app.mount("#app");
