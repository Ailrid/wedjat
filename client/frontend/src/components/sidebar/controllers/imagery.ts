import { Controller, MessageWriter } from "@virid/core"; // 假设你的发送机制是 MessageWriter
import { Project, Env, Responsive, OnHook, Watch } from "@virid/vue";
import {
  type Resource,
  ShowResourceMessage,
  OperateResourceMessage,
} from "@/ccs/cesium";
import type { ImageryLayer } from "cesium";

@Controller()
export class ImageryController {
  @Env()
  public resource!: Resource;

  @Responsive()
  public resourceCloned!: Resource;

  @Responsive()
  public alpha: number = 1;

  @OnHook("onSetup")
  public onSetup() {
    this.alpha = this.resource.alpha;
    this.resourceCloned = JSON.parse(JSON.stringify(this.resource));
  }

  @Watch<ImageryController>((i) => i.alpha)
  public handleAlphaChange() {
    this.resourceCloned.alpha = this.alpha;
    OperateResourceMessage.send((resources, data) => {
      const layer = data.get(this.resource.name);

      if (layer) {
        resources.find((r) => r.name === this.resource.name)!.alpha =
          this.alpha;
        (layer as ImageryLayer).alpha = this.alpha;
      }
    });
  }

  public handleShowToggle() {
    this.resourceCloned.show = !this.resource.show;
    ShowResourceMessage.send(this.resource.name, !this.resource.show);
  }
}
