import { createVirid } from "@virid/core";
import { VuePlugin } from "@virid/vue";
import { bindCesiumBundles } from "./cesium";
import { bindPagesControllers } from "@/pages/controllers";
import { bindComponentControllers } from "@/components/";

const app = createVirid();
const vuePlugin = new VuePlugin();

app.use(vuePlugin, { disableBorrowChecker: false });

export function bootstrapVirid() {
  bindCesiumBundles(app);
  bindPagesControllers(app);
  bindComponentControllers(app);
  return app;
}
