import type { ViridApp } from "@virid/core";
import { bindResourceComponent } from "./component";
import { registerResourceSystems } from "./system";

export function bindResourceBundles(app: ViridApp) {
  bindResourceComponent(app);
  registerResourceSystems(app);
}

export * from "./component";
export * from "./message";
export * from "./interface";
