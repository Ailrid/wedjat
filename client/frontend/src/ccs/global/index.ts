import type { ViridApp } from "@virid/core";
import { bindGlobalComponent } from "./component";
import { registerGlobalSystems } from "./system";

export function bindGlobalBundles(app: ViridApp) {
  bindGlobalComponent(app);
  registerGlobalSystems(app);
}

export * from "./component";
export * from "./message";
