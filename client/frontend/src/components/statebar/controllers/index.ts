import type { ViridApp } from "@virid/core";
import { StateController } from "./statebar";
import { CameraPositionController } from "./camera-position";
import { TipController } from "./tip";

export function bindStatebarControllers(app: ViridApp) {
  app.bind(StateController);
  app.bind(CameraPositionController);
  app.bind(TipController);
}

export * from "./statebar";
export * from "./camera-position";
export * from "./tip";
