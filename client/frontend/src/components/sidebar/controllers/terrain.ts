import { Controller } from "@virid/core";
import { Env, Responsive, OnHook } from "@virid/vue";
import {
  type Resource,
  ShowResourceMessage,
  RemoveResourceMessage,
  LoadTerrainMessage,
} from "@/ccs/resource";
import { SaveSettingsMessage } from "@/ccs/settings";

@Controller()
export class TerrainController {
  @Env()
  public resource!: Resource;

  @Responsive()
  public resourceCloned!: Resource;

  @Responsive()
  public isShowing: boolean = false;

  @OnHook("onSetup")
  public onSetup() {
    this.resourceCloned = JSON.parse(JSON.stringify(this.resource));
  }

  public handleToggle() {
    ShowResourceMessage.send(this.resource.id, !this.resource.show);
  }

  public saveChange() {
    // 地形修改 url / vertexNormals / waterMask 都必须重新创建 provider 才生效
    RemoveResourceMessage.send(this.resourceCloned.id);

    const resourceCloned = JSON.parse(JSON.stringify(this.resourceCloned));
    LoadTerrainMessage.send(resourceCloned);

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
