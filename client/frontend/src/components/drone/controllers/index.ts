import type { ViridApp } from "@virid/core";
import { CameraController } from "./camera";
import { LocateController } from "./locate";
import { VisionController } from "./vision";
import { IMUController } from "./imu";
import { SettingController } from "./setting";

export function bindDroneControllers(app: ViridApp) {
  app.bind(CameraController);
  app.bind(LocateController);
  app.bind(VisionController);
  app.bind(IMUController);
  app.bind(SettingController);
}

export * from "./camera";
export * from "./locate";
export * from "./vision";
export * from "./imu";
export * from "./setting";
