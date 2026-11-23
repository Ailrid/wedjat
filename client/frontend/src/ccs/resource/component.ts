import { Component, type ViridApp } from "@virid/core";
import { Responsive } from "@virid/vue";
import {
  ImageryLayer,
  DataSource,
  Cesium3DTileset,
  CesiumTerrainProvider,
} from "cesium";
import type { Resource } from "./interface";


@Component()
export class ResourceComponent {
  @Responsive()
  public resources: Resource[] = [];

  public resourcesData: Map<
    string,
    ImageryLayer | Cesium3DTileset | DataSource | CesiumTerrainProvider
  > = new Map();
}
export function bindResourceComponent(app: ViridApp) {
  app.bind(ResourceComponent);
}
