import type { ViridApp } from "@virid/core";
import { bindCesiumControllers } from "./cesium/controllers";
import { bindNavbarControllers } from "./navbar/controllers";
import { bindPanelControllers } from "./panel/controllers";
import { bindSidebarControllers } from "./sidebar/controllers";
import { bindStatebarControllers } from "./statebar/controllers";

export function bindComponentControllers(app: ViridApp) {
  bindCesiumControllers(app);
  bindNavbarControllers(app);
  bindPanelControllers(app);
  bindSidebarControllers(app);
  bindStatebarControllers(app);
}
