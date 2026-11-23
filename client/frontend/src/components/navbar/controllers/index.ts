import type { ViridApp } from "@virid/core";
import { NavbarController } from "./navbar";
import { bindImportExternalControllers } from "../importExternal/controllers";
import { bindImportInternalControllers } from "../importInternal/controllers";

export function bindNavbarControllers(app: ViridApp) {
  app.bind(NavbarController);
  bindImportExternalControllers(app);
  bindImportInternalControllers(app);
}

export * from "./navbar";
export * from "../importExternal/controllers";
export * from "../importInternal/controllers";
