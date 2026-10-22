import type { ViridApp } from "@virid/core";
import { NavbarController } from "./navbar";

export function bindNavbarControllers(app: ViridApp) {
  app.bind(NavbarController);
}

export * from "./navbar";
