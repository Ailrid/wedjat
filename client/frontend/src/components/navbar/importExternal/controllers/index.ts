import type { ViridApp } from "@virid/core";

import { ImportExternalController } from "./importExternal";
import { ExternalTerrainController } from "./externalTerrain";
import { ExternalVectorController } from "./externalVector";
import { ExternalTilesController } from "./externalTiles";
import { ExternalImageryController } from "./externalImagery";

export function bindImportExternalControllers(app: ViridApp) {
  app.bind(ImportExternalController);
  app.bind(ExternalTerrainController);
  app.bind(ExternalVectorController);
  app.bind(ExternalTilesController);
  app.bind(ExternalImageryController);
}

export * from "./importExternal";
export * from "./externalTerrain";
export * from "./externalVector";
export * from "./externalTiles";
export * from "./externalImagery";
