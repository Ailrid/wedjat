import { Controller } from "@virid/core";
import { Responsive } from "@virid/vue";
import {
  LoadTilesMessage,
  type Resource,
  type PointCloudConfig,
  type OsgbConfig,
  type PointCloudStyleMode,
  type ClassificationRule,
  type ElevationStop,
} from "@/ccs/resource";
import { v4 as uuidv4 } from "uuid";
import { CloseExternalDialogMessage } from "./importExternal";
import { SaveSettingsMessage } from "@/ccs/settings";

/** ASPRS 常见类别预设 */
const DEFAULT_CLASSIFICATION_RULES: ClassificationRule[] = [
  { classification: 2, color: "#8b4513", visible: true }, // 地面
  { classification: 3, color: "#22c55e", visible: true }, // 低植被
  { classification: 4, color: "#16a34a", visible: true }, // 中植被
  { classification: 5, color: "#15803d", visible: true }, // 高植被
  { classification: 6, color: "#ffffff", visible: true }, // 建筑
  { classification: 9, color: "#3b82f6", visible: true }, // 水体
];

const DEFAULT_ELEVATION_STOPS: ElevationStop[] = [
  { elevation: 0, color: "#22c55e" },
  { elevation: 100, color: "#eab308" },
  { elevation: 200, color: "#ef4444" },
];

@Controller()
export class ExternalTilesController {
  @Responsive() public tab: "pointCloud" | "model" = "pointCloud";

  // 通用
  @Responsive() public name: string = "";
  @Responsive() public url: string = "";
  @Responsive() public credit: string = "";
  @Responsive() public autoLoad: boolean = false;

  // ---------- 点云 ----------
  @Responsive() public styleMode: PointCloudStyleMode = "uniform";
  @Responsive() public uniformColor: string = "#ffffff";
  @Responsive()
  public classificationRules: ClassificationRule[] = [
    ...DEFAULT_CLASSIFICATION_RULES,
  ];
  @Responsive()
  public elevationStops: ElevationStop[] = [...DEFAULT_ELEVATION_STOPS];

  @Responsive() public attenuation: boolean = true;
  @Responsive() public eyeDomeLighting: boolean = true;
  @Responsive() public pcMaximumScreenSpaceError: number = 16;
  @Responsive() public pcDynamicScreenSpaceError: boolean = true;

  // ---------- 三维模型 ----------
  @Responsive() public mdMaximumScreenSpaceError: number = 16;
  @Responsive() public mdDynamicScreenSpaceError: boolean = true;
  @Responsive() public mdMaximumMemoryUsage: number = 512;
  @Responsive()
  public mdShadows: OsgbConfig["shadows"] = "ENABLED";

  // ---------- 列表操作：整体赋值，符合 @virid 响应式规约 ----------
  public addClassificationRule() {
    this.classificationRules = [
      ...this.classificationRules,
      { classification: 0, color: "#ffffff", visible: true },
    ];
  }

  public removeClassificationRule(index: number) {
    this.classificationRules = this.classificationRules.filter(
      (_, i) => i !== index,
    );
  }

  public addElevationStop() {
    this.elevationStops = [
      ...this.elevationStops,
      { elevation: 0, color: "#ffffff" },
    ];
  }

  public removeElevationStop(index: number) {
    this.elevationStops = this.elevationStops.filter((_, i) => i !== index);
  }

  public import() {
    if (!this.name || !this.url) return;

    const resource: Resource = {
      id: uuidv4(),
      name: this.name,
      url: this.url,
      type: "tiles",
      show: true,
      alpha: 1,
      extra: {},
    };

    if (this.tab === "pointCloud") {
      const config: PointCloudConfig = {
        type: "pcd",
        styleMode: this.styleMode,
        uniformColor: this.uniformColor,
        classificationRules: this.classificationRules,
        elevationStops: this.elevationStops,
        attenuation: this.attenuation,
        eyeDomeLighting: this.eyeDomeLighting,
        maximumScreenSpaceError: this.pcMaximumScreenSpaceError,
        dynamicScreenSpaceError: this.pcDynamicScreenSpaceError,
        credit: this.credit,
      };
      resource.extra.tiles = config;
    } else {
      const config: OsgbConfig = {
        type: "osgb",
        maximumScreenSpaceError: this.mdMaximumScreenSpaceError,
        dynamicScreenSpaceError: this.mdDynamicScreenSpaceError,
        maximumMemoryUsage: this.mdMaximumMemoryUsage,
        shadows: this.mdShadows,
        credit: this.credit,
      };
      resource.extra.tiles = config;
    }

    if (this.autoLoad) {
      SaveSettingsMessage.send((settings) => {
        settings.resources = [...settings.resources, resource];
      });
    }

    LoadTilesMessage.send(resource);
    CloseExternalDialogMessage.send();
  }
}
