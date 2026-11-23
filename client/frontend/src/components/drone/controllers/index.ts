import type { ViridApp } from "@virid/core";
import { CameraController } from "./camera";
export function bindDroneControllers(app: ViridApp) {
  app.bind(CameraController);
}


export * from "./camera";
