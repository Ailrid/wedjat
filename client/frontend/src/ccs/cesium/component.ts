import { Component, type ViridApp, Safe } from "@virid/core";
import { Responsive } from "@virid/vue";
import { type Viewer, type ScreenSpaceEventHandler } from "cesium";
export interface CameraPosition {
  lat: number;
  lon: number;
  height: number;
  pitch: number;
  heading: number;
  roll: number;
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

export function bindCesiumComponent(app: ViridApp) {
  app.bind(CesiumComponent);
}
