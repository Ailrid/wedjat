import type { ViridApp } from "@virid/core";
import { PanelController } from "./panel";
import { LocateController } from "./locate";
import { VisionController } from "./vision";
import { InertialController } from "./inertial";
import { SettingController } from "./setting";

export function bindPanelControllers(app: ViridApp) {
  app.bind(PanelController);
  app.bind(LocateController);
  app.bind(VisionController);
  app.bind(InertialController);
  app.bind(SettingController);
}

export * from "./panel";
export * from "./locate";
export * from "./vision";
export * from "./inertial";
export * from "./setting";
