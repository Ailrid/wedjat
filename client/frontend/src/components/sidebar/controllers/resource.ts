import { Controller, EventMessage } from "@virid/core";
import { Listener, Project } from "@virid/vue";
import {
  type Resource,
  ResourceComponent,
  SortResourceMessage,
} from "@/ccs/resource";

export class SortItemMessage extends EventMessage {
  constructor(
    public up: boolean,
    public id: string,
  ) {
    super();
  }
}

@Controller()
export class ResourceController {
  @Project(ResourceComponent, (i) => i.resources)
  public resources!: Resource[];

  @Listener()
  sortItem(mag: SortItemMessage) {
    const { id, up } = mag;

    const rawIndex = this.resources.findIndex((i) => i.id == id);
    let newIndex = rawIndex;
    if (up) {
      newIndex = rawIndex - 1 < 0 ? 0 : rawIndex - 1;
    } else {
      newIndex =
        rawIndex + 1 >= this.resources.length
          ? this.resources.length - 1
          : rawIndex + 1;
    }
    let ids = JSON.parse(JSON.stringify(this.resources.map((i) => i.id)));
    // 交换两个索引的位置
    [ids[rawIndex], ids[newIndex]] = [ids[newIndex], ids[rawIndex]];
    SortResourceMessage.send(ids);
  }
}
