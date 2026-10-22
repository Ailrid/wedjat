import type { ViridApp } from "@virid/core";
import { CesiumController } from "./cesium";
import { ContainerController } from "./container";

export function bindCesiumControllers(app: ViridApp) {
  app.bind(CesiumController);
  app.bind(ContainerController);
}

export * from "./cesium";
export * from "./container";
