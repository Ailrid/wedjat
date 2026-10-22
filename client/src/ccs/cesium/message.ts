import { EventMessage } from "@virid/core";
import * as Cesium from "cesium";
type FlyToOptions = Parameters<Cesium.Camera["flyTo"]>[0];

export class InitCesiumMessage extends EventMessage {}
export class ResizeCesiumMessage extends EventMessage {}
export class FlyToMessage extends EventMessage {
  constructor(public options: FlyToOptions) {
    super();
  }
}
