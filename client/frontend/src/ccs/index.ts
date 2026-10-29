import { createVirid } from "@virid/core";
import { VuePlugin } from "@virid/vue";
import { bindCesiumBundles } from "./cesium";
import { bindGlobalBundles } from "./global";
import { bindPagesControllers } from "@/pages/controllers";
import { bindComponentControllers } from "@/components/";

const app = createVirid();
const vuePlugin = new VuePlugin();

app.use(vuePlugin, { disableBorrowChecker: false });

export function bootstrapVirid() {
  bindCesiumBundles(app);
  bindGlobalBundles(app);
  bindPagesControllers(app);
  bindComponentControllers(app);
  return app;
}
