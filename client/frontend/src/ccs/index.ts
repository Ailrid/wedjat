import { createVirid } from "@virid/core";
import { VuePlugin } from "@virid/vue";
import { bindCesiumBundles } from "./cesium";
import { bindSettingBunch } from "./settings";
import { bindResourceBundles } from "./resource";
import { bindGlobalBundles } from "./global";
import { bindPagesControllers } from "@/pages/controllers";
import { bindComponentControllers } from "@/components/";


const app = createVirid();
const vuePlugin = new VuePlugin();

app.use(vuePlugin, { disableBorrowChecker: false });

export function bootstrapVirid() {
  bindCesiumBundles(app);
  bindSettingBunch(app);
  bindGlobalBundles(app);
  bindResourceBundles(app);
  bindPagesControllers(app);
  bindComponentControllers(app);
  return app;
}
