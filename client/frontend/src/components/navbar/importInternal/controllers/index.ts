import type { ViridApp } from "@virid/core";
import { ImportInternalController } from "./importInternal";
export function bindImportInternalControllers(app: ViridApp) {
  app.bind(ImportInternalController);
}

export * from "./importInternal";
