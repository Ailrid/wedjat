import type { ViridApp } from "@virid/core";
import { bindCesiumComponent } from "./component";
import { registerCesiumSystems } from "./system";

export function bindCesiumBundles(app: ViridApp) {
  bindCesiumComponent(app);
  registerCesiumSystems(app);
}

export * from "./component";
export * from "./message";
