import type { ViridApp } from "@virid/core";
import { SidebarController } from "./sidebar";
import {ImageryController} from "./imagery"
import { ResourceController } from "./resource";
import { TerrainController } from "./terrain";
import { VectorController } from "./vector";
import { TilesController } from "./tiles";
export function bindSidebarControllers(app: ViridApp) {
  app.bind(SidebarController);
  app.bind(ImageryController);
  app.bind(ResourceController);
  app.bind(TerrainController);
  app.bind(VectorController);
  app.bind(TilesController);
}

export * from "./sidebar";
export * from "./imagery";
export * from "./resource";
export * from "./terrain";
export * from "./vector";
export * from "./tiles";
