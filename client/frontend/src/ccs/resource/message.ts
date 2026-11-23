import { EventMessage } from "@virid/core";
import { type Resource } from "./interface";
import { type ResourceComponent } from "./component";
import * as Cesium from "cesium";

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
export class LoadTilesMessage extends EventMessage {
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
    public id: string,
    public show: boolean,
  ) {
    super();
  }
}

export class RemoveResourceMessage extends EventMessage {
  constructor(public id: string) {
    super();
  }
}
export class SortResourceMessage extends EventMessage {
  constructor(public ids: string[]) {
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
export class LoadSavedResourcesMessage extends EventMessage {}

export class LocateResourceMessage extends EventMessage {
  constructor(public id: string) {
    super();
  }
}
