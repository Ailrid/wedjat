import type { ViridApp } from "@virid/core";
import { SidebarController } from "./sidebar";

export function bindSidebarControllers(app: ViridApp) {
  app.bind(SidebarController);
}

export * from "./sidebar";
