import { Controller, MessageWriter } from "@virid/core"; // 假设你的发送机制是 MessageWriter
import { Project, Env, Responsive, OnHook, Watch } from "@virid/vue";
import {
  type Resource,
  ShowResourceMessage,
  OperateResourceMessage,
  RemoveResourceMessage,
  LoadImageryMessage,
} from "@/ccs/resource";
import type { ImageryLayer } from "cesium";
import { SaveSettingsMessage } from "@/ccs/settings";

@Controller()
export class ImageryController {
  @Env()
  public resource!: Resource;

  @Responsive()
  public alpha: number = 1;

  @Responsive()
  public resourceCloned!: Resource;

  @Responsive()
  public subdomains!: string;

  @Responsive()
  public tilingScheme!: "WebMercator" | "Geographic";

  @Responsive()
  public isShowing: boolean = false;

  @OnHook("onSetup")
  public onSetup() {
    this.alpha = this.resource.alpha;
    this.resourceCloned = JSON.parse(JSON.stringify(this.resource));
  }

  @Watch<ImageryController>((i) => i.alpha)
  public handleAlphaChange() {
    OperateResourceMessage.send((resources, data) => {
      const layer = data.get(this.resource.id);

      if (layer) {
        resources.find((r) => r.id === this.resource.id)!.alpha = this.alpha;
        (layer as ImageryLayer).alpha = this.alpha;
      }
    });
  }

  public handleShowToggle() {
    ShowResourceMessage.send(this.resource.id, !this.resource.show);
  }
  public saveChange() {
    RemoveResourceMessage.send(this.resourceCloned.id);
    const resourceCloned = JSON.parse(JSON.stringify(this.resourceCloned));
    LoadImageryMessage.send(resourceCloned);
    SaveSettingsMessage.send((settings) => {
      const idx = settings.resources.findIndex(
        (r) => r.id === resourceCloned.id,
      );
      if (idx >= 0) {
        settings.resources[idx] = resourceCloned;
      }
    });
  }
  public removeResource() {
    RemoveResourceMessage.send(this.resourceCloned.id);
    SaveSettingsMessage.send((settings) => {
      const idx = settings.resources.findIndex(
        (r) => r.id === this.resourceCloned.id,
      );
      if (idx >= 0) {
        settings.resources.splice(idx, 1);
      }
    });
  }

}
