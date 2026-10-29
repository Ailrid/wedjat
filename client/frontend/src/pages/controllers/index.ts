import { type ViridApp } from "@virid/core";
import { HomeController } from "./home";
export function bindPagesControllers(app: ViridApp) {
  app.bind(HomeController);

}

export * from "./home";