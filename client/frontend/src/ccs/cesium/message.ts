import { EventMessage } from "@virid/core";
import { type Resource, type ResourceComponent } from "./component";
import * as Cesium from "cesium";
type FlyToOptions = Parameters<Cesium.Camera["flyTo"]>[0];

export class InitCesiumMessage extends EventMessage {}
export class ResizeCesiumMessage extends EventMessage {}
export class FlyToMessage extends EventMessage {
  constructor(public options: FlyToOptions) {
    super();
  }
}

export class LoadImageryMessage extends EventMessage {
  constructor(public resource: Resource) {
    super();
  }
}
export class LoadTerrainMessage extends EventMessage {
  constructor(public resource: Resource) {
    super();
  }
}
export class Load3DTilesMessage extends EventMessage {
  constructor(public resource: Resource) {
    super();
  }
}
export class LoadVectorMessage extends EventMessage {
  constructor(public resource: Resource) {
    super();
  }
}

export class ShowResourceMessage extends EventMessage {
  constructor(
    public name: string,
    public show: boolean,
  ) {
    super();
  }
}

export class SortResourceMessage extends EventMessage {
  constructor(public names: string[]) {
    super();
  }
}

export class OperateResourceMessage extends EventMessage {
  constructor(
    public operate: (
      resources: Resource[],
      resourcesData: ResourceComponent["resourcesData"],
    ) => void,
  ) {
    super();
  }
}
