import type { ViridApp } from "@virid/core";
import { PanelController } from "./panel";

export function bindPanelControllers(app: ViridApp) {
  app.bind(PanelController);
}

export * from "./panel";
