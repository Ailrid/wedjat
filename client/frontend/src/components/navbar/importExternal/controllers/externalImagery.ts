import { Controller } from "@virid/core";
import { Responsive } from "@virid/vue";
import { LoadImageryMessage, type Resource } from "@/ccs/resource";
import { v4 as uuidv4 } from "uuid";
import { CloseExternalDialogMessage } from "./importExternal";
import { SaveSettingsMessage } from "@/ccs/settings";
export interface XyzConfig {
  subdomains: string;
}

export interface WmsConfig {
  layers: string;
  version: "1.1.1" | "1.3.0";
  format: string;
  enablePickFeatures: boolean;
}

export interface WmtsConfig {
  layer: string;
  style: string;
  matrixSet: string;
  format: string;
  subdomains: string;
  matrixLabels: string;
  projection: "w" | "c";
}

@Controller()
export class ExternalImageryController {
  @Responsive()
  public tab: "xyz" | "wms" | "wmts" = "xyz";

  @Responsive()
  public minLevel: number = 1;
  @Responsive()
  public maxLevel: number = 18;

  @Responsive()
  public name: string = "";
  @Responsive()
  public url: string = "";

  @Responsive()
  public tilingScheme: "WebMercator" | "Geographic" = "WebMercator";

  @Responsive()
  public credit: string = "";
  @Responsive()
  public autoLoad: boolean = false;
  @Responsive()
  public xyz: XyzConfig = {
    subdomains: "",
  };

  @Responsive()
  public wms: WmsConfig = {
    layers: "",
    version: "1.1.1",
    format: "image/png",
    enablePickFeatures: false,
  };

  @Responsive()
  public wmts: WmtsConfig = {
    layer: "img",
    style: "default",
    matrixSet: "",
    format: "tiles",
    subdomains: "0,1,2,3,4,5,6,7",
    matrixLabels: "",
    projection: "w",
  };

  public import() {
    if (!this.name || !this.url) return;

    const base = {
      minimumLevel: this.minLevel,
      maximumLevel: this.maxLevel,
      credit: this.credit || "",
    };
    const resource: Resource = {
      id: uuidv4(),
      name: this.name,
      url: this.url,
      type: "imagery",
      show: true,
      alpha: 1,
      extra: {},
    };

    switch (this.tab) {
      case "xyz":
        resource.extra.imagery = {
          type: "xyz",
          ...base,
          subdomains: this.xyz.subdomains,
          tilingScheme: this.tilingScheme,
        };
        break;

      case "wms":
        resource.extra.imagery = {
          type: "wms",
          ...base,
          layers: this.wms.layers,
          version: this.wms.version,
          format: this.wms.format,
          enablePickFeatures: this.wms.enablePickFeatures,
          tilingScheme: this.tilingScheme,
        };
        break;

      case "wmts":
        {
          // projection 优先决定切片方案，避免 c + WebMercator 这种冲突组合
          const wmtsScheme =
            this.wmts.projection === "c"
              ? "Geographic"
              : this.wmts.projection === "w"
                ? "WebMercator"
                : this.tilingScheme;

          resource.extra.imagery = {
            type: "wmts",
            ...base,
            layer: this.wmts.layer,
            style: this.wmts.style,
            // matrixSet 手填优先，留空则跟随 projection
            tileMatrixSetID: this.wmts.matrixSet || this.wmts.projection,
            format: this.wmts.format,
            subdomains: this.wmts.subdomains,
            tileMatrixLabels: this.wmts.matrixLabels,
            projection: this.wmts.projection,
            tilingScheme: wmtsScheme,
          };
        }
        break;
    }

    if (this.autoLoad) {
      SaveSettingsMessage.send((settings) => {
        settings.resources.push(resource);
      });
    }

    LoadImageryMessage.send(resource);
    CloseExternalDialogMessage.send();
  }
}

// [
//   {
//     id: "fa2e3694-9473-4a00-be67-572d34eeb282",
//     name: "天底图影像注记",
//     url: "https://t{s}.tianditu.gov.cn/cia_w/wmts?tk=16ea42d08457b4de2ea8228e1a65fe63",
//     type: "imagery",
//     show: true,
//     alpha: 1,
//     extra: {
//       imagery: {
//         type: "wmts",
//         minimumLevel: 1,
//         maximumLevel: 18,
//         credit: "",
//         layer: "cia",
//         style: "default",
//         tileMatrixSetID: "w",
//         format: "tiles",
//         subdomains: "0,1,2,3,4,5,6,7",
//         tileMatrixLabels: "",
//         projection: "w",
//         tilingScheme: "WebMercator",
//       },
//     },
//   },
//   {
//     id: "b3f2aa62-86bc-44a9-ba0d-41651f8b0ee3",
//     name: "天地图影像底图",
//     url: "https://t{s}.tianditu.gov.cn/img_w/wmts?tk=16ea42d08457b4de2ea8228e1a65fe63",
//     type: "imagery",
//     show: false,
//     alpha: 1,
//     extra: {
//       imagery: {
//         type: "wmts",
//         minimumLevel: 1,
//         maximumLevel: 18,
//         credit: "",
//         layer: "img",
//         style: "default",
//         tileMatrixSetID: "w",
//         format: "tiles",
//         subdomains: "0,1,2,3,4,5,6,7",
//         tileMatrixLabels: "",
//         projection: "w",
//         tilingScheme: "WebMercator",
//       },
//     },
//   },
//   {
//     id: "3caa8a5e-3b34-4f08-b097-caa450572700",
//     name: "天地图矢量底图",
//     url: "http://t{s}.tianditu.gov.cn/vec_w/wmts?tk=16ea42d08457b4de2ea8228e1a65fe63",
//     type: "imagery",
//     show: false,
//     alpha: 1,
//     extra: {
//       imagery: {
//         type: "wmts",
//         minimumLevel: 1,
//         maximumLevel: 18,
//         credit: "",
//         layer: "vec",
//         style: "default",
//         tileMatrixSetID: "w",
//         format: "tiles",
//         subdomains: "0,1,2,3,4,5,6,7",
//         tileMatrixLabels: "",
//         projection: "w",
//         tilingScheme: "WebMercator",
//       },
//     },
//   },
//   {
//     id: "42e8c25e-d2a3-4171-89eb-def0891fdee5",
//     name: "天地图矢量注记",
//     url: "http://t{s}.tianditu.gov.cn/cva_w/wmts?tk=16ea42d08457b4de2ea8228e1a65fe63",
//     type: "imagery",
//     show: false,
//     alpha: 1,
//     extra: {
//       imagery: {
//         type: "wmts",
//         minimumLevel: 1,
//         maximumLevel: 18,
//         credit: "",
//         layer: "cva",
//         style: "default",
//         tileMatrixSetID: "w",
//         format: "tiles",
//         subdomains: "0,1,2,3,4,5,6,7",
//         tileMatrixLabels: "",
//         projection: "w",
//         tilingScheme: "WebMercator",
//       },
//     },
//   },
//   {
//     id: "868f445f-eda6-4e9a-90e5-a158c5e89b88",
//     name: "江苏Terrain",
//     url: "/public/dem",
//     type: "terrain",
//     show: true,
//     alpha: 1,
//     extra: {
//       terrain: {
//         requestVertexNormals: true,
//         requestWaterMask: false,
//         credit: "",
//       },
//     },
//   },
// ];
