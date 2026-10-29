import { Component, type ViridApp, Safe } from "@virid/core";
import { Responsive } from "@virid/vue";
import {
  type Viewer,
  type ScreenSpaceEventHandler,
  ImageryLayer,
  DataSource,
  Cesium3DTileset,
} from "cesium";
export interface CameraPosition {
  lat: number;
  lon: number;
  height: number;
  pitch: number;
  heading: number;
  roll: number;
}

export type ResourceType =
  | "imagery" // DOM、专题底图（XYZ / WMS / WMTS）
  | "terrain" // DEM 切片后的 Cesium Terrain
  | "tiles" // 城市三维模型 / 激光雷达点云（入口 tileset.json）
  | "vector"; // GeoJSON / KML / CZML（未切片矢量）

export interface Resource {
  type: ResourceType;
  name: string;
  url: string;
  show: boolean;
  alpha: number;
}

@Component()
export class CesiumComponent {
  public viewer!: Viewer;
  @Responsive()
  public cameraPosition: CameraPosition = {
    lat: 0,
    lon: 0,
    height: 0,
    pitch: 0,
    heading: 0,
    roll: 0,
  };

  public screenSpaceEventHandler!: ScreenSpaceEventHandler;

  @Safe()
  public getViewer() {
    return this.viewer;
  }
}

@Component()
export class ResourceComponent {
  @Responsive()
  public resources: Resource[] = [];
  public resourcesData: Map<
    string,
    ImageryLayer | Cesium3DTileset | DataSource
  > = new Map();
}

export function bindCesiumComponent(app: ViridApp) {
  app.bind(CesiumComponent);
  app.bind(ResourceComponent);
}
