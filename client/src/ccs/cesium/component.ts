import { Component, type ViridApp, Safe } from "@virid/core";
import { Responsive } from "@virid/vue";
import { type Viewer } from "cesium";

@Component()
export class CesiumComponent {
  public viewer!: Viewer;

  @Safe()
  public getViewer() {
    return this.viewer;
  }
} 

export function bindCesiumComponent(app: ViridApp) {
  app.bind(CesiumComponent);
}
