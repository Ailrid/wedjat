import { Controller } from "@virid/core";
import { Env, OnHook, Responsive } from "@virid/vue";
import {
  type Resource,
  ShowResourceMessage,
  RemoveResourceMessage,
  LoadTilesMessage,
} from "@/ccs/resource";
import type { OsgbConfig, PointCloudConfig } from "@/ccs/resource/interface";
import { SaveSettingsMessage } from "@/ccs/settings";

@Controller()
export class TilesController {
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

  // ---- 类型收窄的便捷 getter，供模板使用 ----
  public get pcd(): PointCloudConfig | undefined {
    const t = this.resourceCloned.extra.tiles;
    return t?.type === "pcd" ? t : undefined;
  }
  public get osgb(): OsgbConfig | undefined {
    const t = this.resourceCloned.extra.tiles;
    return t?.type === "osgb" ? t : undefined;
  }

  public handleShowToggle() {
    ShowResourceMessage.send(this.resource.id, !this.resource.show);
  }

  public saveChange() {
    RemoveResourceMessage.send(this.resourceCloned.id);
    const resourceCloned = JSON.parse(JSON.stringify(this.resourceCloned));
    LoadTilesMessage.send(resourceCloned);
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

  // ---- PCD 分类规则列表 ----
  public addClassificationRule() {
    this.pcd?.classificationRules.push({
      classification: 0,
      color: "#ffffff",
      visible: true,
    });
  }
  public removeClassificationRule(i: number) {
    this.pcd?.classificationRules.splice(i, 1);
  }

  // ---- PCD 高程分段列表 ----
  public addElevationStop() {
    this.pcd?.elevationStops.push({ elevation: 0, color: "#ffffff" });
  }
  public removeElevationStop(i: number) {
    this.pcd?.elevationStops.splice(i, 1);
  }
}
