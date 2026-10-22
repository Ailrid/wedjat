import type { ViridApp } from "@virid/core";
import { StateController } from "./statebar";

export function bindStatebarControllers(app: ViridApp) {
  app.bind(StateController);
}

export * from "./statebar";
