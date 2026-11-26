import { System, type ViridApp, MessageWriter } from "@virid/core";
import { ResourceComponent } from "./component";
import { CesiumComponent } from "@/ccs/cesium";
import { match } from "ts-pattern";
import * as Cesium from "cesium";
import {
  LoadImageryMessage,
  LoadTerrainMessage,
  LoadVectorMessage,
  LoadTilesMessage,
  ShowResourceMessage,
  SortResourceMessage,
  RemoveResourceMessage,
  OperateResourceMessage,
  LoadSavedResourcesMessage,
  LocateResourceMessage,
} from "./message";
import { SaveSettingsMessage, SettingComponent } from "../settings";
import type { PointCloudConfig, Resource, VectorConfig } from "./interface";
import { FlyToMessage } from "../cesium"; // 确保路径正确，跟你之前的引入方式一致

function toTilingScheme(
  type?: "WebMercator" | "Geographic",
): Cesium.TilingScheme | undefined {
  if (type === "WebMercator") return new Cesium.WebMercatorTilingScheme();
  if (type === "Geographic") return new Cesium.GeographicTilingScheme();
  return undefined; // 交给 Cesium 各 Provider 的默认值
}

function applyVectorStyle(dataSource: Cesium.DataSource, config: VectorConfig) {
  const pointColor = Cesium.Color.fromCssColorString(config.point.color);
  const pointOutlineColor = Cesium.Color.fromCssColorString(
    config.point.outlineColor,
  );
  const polylineColor = Cesium.Color.fromCssColorString(config.polyline.color);
  const polygonColor = Cesium.Color.fromCssColorString(config.polygon.color);
  const polygonOutlineColor = Cesium.Color.fromCssColorString(
    config.polygon.outlineColor,
  );

  for (const entity of dataSource.entities.values) {
    // GeoJSON 的点
    if (entity.point) {
      entity.point.color = new Cesium.ConstantProperty(pointColor);
      entity.point.pixelSize = new Cesium.ConstantProperty(config.point.size);
      entity.point.outlineColor = new Cesium.ConstantProperty(
        pointOutlineColor,
      );
      entity.point.outlineWidth = new Cesium.ConstantProperty(
        config.point.outlineWidth,
      );
    }

    // KML 的图标（Placemark 在 Cesium 里通常是 billboard，不是 point）
    if (entity.billboard) {
      entity.billboard.color = new Cesium.ConstantProperty(pointColor);
      // billboard 没有 pixelSize，用 scale 近似；假设原图约 32px
      entity.billboard.scale = new Cesium.ConstantProperty(
        config.point.size / 32,
      );
    }

    if (entity.polyline) {
      entity.polyline.material = new Cesium.ColorMaterialProperty(
        polylineColor,
      );
      entity.polyline.width = new Cesium.ConstantProperty(
        config.polyline.width,
      );
    }

    if (entity.polygon) {
      entity.polygon.material = new Cesium.ColorMaterialProperty(polygonColor);
      entity.polygon.outline = new Cesium.ConstantProperty(
        config.polygon.outline,
      );
      entity.polygon.outlineColor = new Cesium.ConstantProperty(
        polygonOutlineColor,
      );
      entity.polygon.outlineWidth = new Cesium.ConstantProperty(
        config.polygon.outlineWidth,
      );
    }
  }
}

/** 根据点云配置生成 Cesium3DTileStyle；original 模式返回 undefined，不覆盖原始样式 */
function buildPointCloudStyle(
  config: PointCloudConfig,
): Cesium.Cesium3DTileStyle | undefined {
  return match(config.styleMode)
    .with("original", () => undefined)
    .with(
      "uniform",
      () =>
        new Cesium.Cesium3DTileStyle({
          color: `color('${config.uniformColor}')`,
        }),
    )
    .with("classification", () => {
      const colorConds: [string, string][] = [];
      const showConds: [string, string][] = [];

      for (const r of config.classificationRules) {
        // 模板里的 ${...} 是 Cesium 表达式，需要转义成 \${...}
        const cond = `\${Classification} === ${r.classification}`;
        colorConds.push([cond, `color('${r.color}')`]);
        showConds.push([cond, String(r.visible)]);
      }

      // 兜底：未匹配到的类别用白色、显示
      colorConds.push(["true", "color('white')"]);
      showConds.push(["true", "true"]);

      return new Cesium.Cesium3DTileStyle({
        color: { conditions: colorConds },
        show: { conditions: showConds },
      });
    })
    .with("elevation", () => {
      // 按高程升序，避免用户乱序填写导致的分层错乱
      const stops = config.elevationStops.sort(
        (a, b) => a.elevation - b.elevation,
      );

      if (stops.length === 0) return undefined;
      if (stops.length === 1) {
        return new Cesium.Cesium3DTileStyle({
          color: `color('${stops[0]!.color}')`,
        });
      }

      const conds: [string, string][] = [];
      for (let i = 1; i < stops.length; i++) {
        conds.push([
          `\${POSITION}[2] < ${stops[i]!.elevation}`,
          `color('${stops[i - 1]!.color}')`,
        ]);
      }
      // 最高一段以上用最后一段颜色兜底
      conds.push(["true", `color('${stops[stops.length - 1]!.color}')`]);

      return new Cesium.Cesium3DTileStyle({ color: { conditions: conds } });
    })
    .exhaustive();
}

export class ResourceSystem {
  @System()
  static imagery(
    message: LoadImageryMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const resource: Resource = JSON.parse(JSON.stringify(message.resource));
    resourceComponent.resources.unshift(resource);
    const viewer = cesiumComponent.viewer;
    const config = resource.extra.imagery;

    if (!config) {
      MessageWriter.error(
        new Error(`Resource ${resource.name} miss imagery config`),
      );
      return;
    }

    const credit = config.credit ? new Cesium.Credit(config.credit) : undefined;

    const provider = match(config)
      .with({ type: "xyz" }, (c) => {
        return new Cesium.UrlTemplateImageryProvider({
          url: resource.url,
          subdomains: undefined,
          // minimumLevel: c.minimumLevel,
          // maximumLevel: c.maximumLevel,
          tilingScheme: toTilingScheme(c.tilingScheme),
          credit,
        });
      })
      .with({ type: "wms" }, (c) => {
        return new Cesium.WebMapServiceImageryProvider({
          url: resource.url,
          layers: c.layers,
          parameters: {
            version: c.version,
            format: c.format,
          },
          enablePickFeatures: c.enablePickFeatures ?? false,
          minimumLevel: c.minimumLevel,
          maximumLevel: c.maximumLevel,
          tilingScheme: toTilingScheme(c.tilingScheme),
          credit,
        });
      })
      .with({ type: "wmts" }, (c) => {
        const tileMatrixLabels = c.tileMatrixLabels.trim()
          ? c.tileMatrixLabels
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean)
          : undefined;
        return new Cesium.WebMapTileServiceImageryProvider({
          url: resource.url,
          layer: c.layer,
          style: c.style,
          format: c.format,
          tileMatrixSetID: c.tileMatrixSetID,
          tileMatrixLabels,
          subdomains: c.subdomains.split(","),
          minimumLevel: c.minimumLevel,
          maximumLevel: c.maximumLevel,
          tilingScheme: toTilingScheme(c.tilingScheme),
          credit,
        });
      })
      .exhaustive();

    const layer = viewer.imageryLayers.addImageryProvider(provider);
    layer.show = resource.show;
    layer.alpha = resource.alpha;
    resourceComponent.resourcesData.set(resource.id, layer);

    MessageWriter.info(`Loaded Imagery ${resource.name} successfully.`);
  }
  @System()
  static async terrain(
    message: LoadTerrainMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    // 在地形加载的 System 里
    const resource: Resource = JSON.parse(JSON.stringify(message.resource));
    resourceComponent.resources.unshift(resource);
    const config = resource.extra.terrain;

    if (!config) {
      MessageWriter.error(
        new Error(`Resource ${resource.name} miss terrain config`),
      );
      return;
    }
    const terrainProvider = await Cesium.CesiumTerrainProvider.fromUrl(
      resource.url,
      {
        requestVertexNormals: config.requestVertexNormals ?? true,
        requestWaterMask: config.requestWaterMask ?? false,
        credit: config.credit ? new Cesium.Credit(config.credit) : undefined,
      },
    );
    // 导入一个新的就关闭其他的显示
    resourceComponent.resources.forEach((r) => {
      if (r.extra.terrain && r.id !== resource.id) {
        r.show = false;
      }
    });
    cesiumComponent.viewer.terrainProvider = terrainProvider;

    resourceComponent.resourcesData.set(resource.id, terrainProvider);

    MessageWriter.info(`Loaded Terrain ${resource.name} successfully.`);
  }
  @System()
  static async tiles3d(
    message: LoadTilesMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const resource: Resource = JSON.parse(JSON.stringify(message.resource));
    const viewer = cesiumComponent.viewer;
    const config = resource.extra.tiles;
    resourceComponent.resources.unshift(resource);
    if (!config) {
      MessageWriter.error(
        new Error(`Resource ${resource.name} miss tiles config`),
      );
      return;
    }

    let tileset: Cesium.Cesium3DTileset;

    tileset = await Cesium.Cesium3DTileset.fromUrl(resource.url, {
      maximumScreenSpaceError: config.maximumScreenSpaceError,
      dynamicScreenSpaceError: config.dynamicScreenSpaceError,

      ...(config.type === "mesh"
        ? { maximumMemoryUsage: config.maximumMemoryUsage }
        : {}),
    });

    if (config.type === "las") {
      // PointCloudShading：衰减 + 视锥点照明
      tileset.pointCloudShading.attenuation = config.attenuation;
      tileset.pointCloudShading.eyeDomeLighting = config.eyeDomeLighting;

      const style = buildPointCloudStyle(config);
      if (style) tileset.style = style;
    } else {
      // osgb：倾斜模型只配阴影，样式交给模型自带纹理
      tileset.shadows = Cesium.ShadowMode[config.shadows];
    }

    tileset.show = resource.show;
    viewer.scene.primitives.add(tileset);

    resourceComponent.resourcesData.set(resource.id, tileset);

    MessageWriter.info(`Loaded Tiles ${resource.name} successfully.`);
  }

  @System()
  static async vector(
    message: LoadVectorMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const resource: Resource = JSON.parse(JSON.stringify(message.resource));
    resourceComponent.resources.unshift(resource);
    const viewer = cesiumComponent.viewer;
    const config = resource.extra.vector;

    if (!config) {
      MessageWriter.error(
        new Error(`Resource ${resource.name} miss vector config`),
      );
      return;
    }

    let dataSource: Cesium.DataSource;

    dataSource = await match(config.type)
      .with("geojson", () =>
        Cesium.GeoJsonDataSource.load(resource.url, {
          clampToGround: config.clampToGround,
        }),
      )
      .with("kml", () =>
        Cesium.KmlDataSource.load(resource.url, {
          clampToGround: config.clampToGround,
        }),
      )
      .exhaustive();

    dataSource.name = resource.name || "Vector Layer";
    dataSource.show = resource.show;
    applyVectorStyle(dataSource, config);

    await viewer.dataSources.add(dataSource);

    resourceComponent.resourcesData.set(resource.id, dataSource);

    MessageWriter.info(`Loaded Vector ${resource.name} successfully.`);
  }
  @System()
  static show(
    message: ShowResourceMessage,
    resourceComponent: ResourceComponent,
    cesiumComponent: CesiumComponent,
  ) {
    const { id, show } = message;
    const targetResource = resourceComponent.resources.find((r) => r.id == id)!;
    targetResource.show = show;
    const cesiumData = resourceComponent.resourcesData.get(id)!;

    if (cesiumData instanceof Cesium.CesiumTerrainProvider) {
      // 地形模式下，激活一个地形就取消其他所有的地形显示
      resourceComponent.resources.forEach((r) => {
        if (r.extra.terrain && r.id != id) {
          r.show = false;
        }
      });
      cesiumComponent.viewer.terrainProvider = show
        ? cesiumData
        : new Cesium.EllipsoidTerrainProvider();
    } else {
      cesiumData.show = show;
    }
  }
  @System()
  static sort(
    message: SortResourceMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const { ids } = message;
    const viewer = cesiumComponent.viewer;

    const idIndexMap = new Map<string, number>();
    ids.forEach((id, index) => idIndexMap.set(id, index));

    const sorted = [...resourceComponent.resources].sort((a, b) => {
      const posA = idIndexMap.get(a.id) ?? Infinity;
      const posB = idIndexMap.get(b.id) ?? Infinity;
      return posA - posB;
    });
    resourceComponent.resources = sorted;

    for (let i = ids.length - 1; i >= 0; i--) {
      const id = ids[i]!;

      const targetResource = resourceComponent.resources.find(
        (r) => r.id === id,
      );
      if (!targetResource) continue;

      const cesiumData = resourceComponent.resourcesData.get(targetResource.id);
      if (!cesiumData) continue;

      switch (targetResource.type) {
        case "imagery":
          viewer.imageryLayers.raiseToTop(cesiumData as Cesium.ImageryLayer);
          break;
        case "vector":
          viewer.dataSources.raiseToTop(cesiumData as Cesium.DataSource);
          break;
        case "tiles":
          viewer.scene.primitives.raiseToTop(
            cesiumData as Cesium.Cesium3DTileset,
          );
          break;
        case "terrain":
          // terrain 不参与 imagery/dataSources/primitives 的层叠排序
          break;
        default:
          break;
      }
    }
    SaveSettingsMessage.send((settings) => {
      settings.resources = sorted;
    });

    MessageWriter.info(`Sorted ${ids.length} resources successfully.`);
  }

  @System()
  static operate(
    message: OperateResourceMessage,
    resourceComponent: ResourceComponent,
  ) {
    if (typeof message.operate === "function") {
      message.operate(
        resourceComponent.resources,
        resourceComponent.resourcesData,
      );
    }
  }

  @System()
  static remove(
    message: RemoveResourceMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const { id } = message;
    const viewer = cesiumComponent.viewer;

    const data = resourceComponent.resourcesData.get(id);

    if (data) {
      if (data instanceof Cesium.ImageryLayer) {
        viewer.imageryLayers.remove(data, true);
      } else if (data instanceof Cesium.Cesium3DTileset) {
        viewer.scene.primitives.remove(data);
      } else if (
        data instanceof Cesium.DataSource &&
        viewer.dataSources.contains(data)
      ) {
        viewer.dataSources.remove(data, true);
      } else if (data instanceof Cesium.CesiumTerrainProvider) {
        viewer.terrainProvider = new Cesium.EllipsoidTerrainProvider();
      }
      resourceComponent.resourcesData.delete(id);
    }

    const index = resourceComponent.resources.findIndex((r) => r.id === id);
    if (index !== -1) {
      const resource = resourceComponent.resources[index]!;
      resourceComponent.resources.splice(index, 1);
      // 清除url
      if (
        resource?.extra.vector &&
        typeof resource.url === "string" &&
        resource.url.startsWith("blob:")
      ) {
        URL.revokeObjectURL(resource.url);
      }
      MessageWriter.info(`Removed resource ${resource.name} successfully.`);
    }
  }
  @System()
  static async locate(
    message: LocateResourceMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const { id } = message;
    const viewer = cesiumComponent.viewer;
    const resource = resourceComponent.resources.find((r) => r.id === id);
    const cesiumData = resourceComponent.resourcesData.get(id);

    if (!resource || !cesiumData) {
      MessageWriter.warn(`Locate failed: Resource ${id} not found.`);
      return;
    }

    // 1. 优先直接从后端的 ResourceResponse 拿边界
    const bounds = resource.resource;
    let rect: Cesium.Rectangle | undefined;

    if (
      bounds?.minLon != null &&
      bounds?.minLat != null &&
      bounds?.maxLon != null &&
      bounds?.maxLat != null
    ) {
      rect = Cesium.Rectangle.fromDegrees(
        bounds.minLon,
        bounds.minLat,
        bounds.maxLon,
        bounds.maxLat,
      );
    }

    // 2. 如果没拿到，且是 xyz 类型的 imagery，才去请求 URL 里的 TileJSON
    if (
      !rect &&
      resource.type === "imagery" &&
      resource.extra.imagery?.type === "xyz"
    ) {
      try {
        // 比如 http://localhost:3000/dom/{z}/{x}/{y}.png -> http://localhost:3000/dom
        const tileJsonUrl = resource.url.split("/{z}")[0];
        const response = await fetch(tileJsonUrl!);
        if (response.ok) {
          const tilejson = await response.json();
          if (tilejson.bounds && tilejson.bounds.length === 4) {
            rect = Cesium.Rectangle.fromDegrees(
              tilejson.bounds[0],
              tilejson.bounds[1],
              tilejson.bounds[2],
              tilejson.bounds[3],
            );
          }
        }
      } catch (e) {
        MessageWriter.warn(`Failed to fetch TileJSON for ${resource.name}`);
      }
    }

    // 3. 如果拿到了 rect，直接用 FlyToMessage；否则退化为按对象类型 flyTo
    if (rect) {
      FlyToMessage.send({ destination: rect, duration: 1 });
      MessageWriter.info(`Located ${resource.name} via bounds.`);
      return;
    }

    if (
      cesiumData instanceof Cesium.Cesium3DTileset ||
      cesiumData instanceof Cesium.DataSource
    ) {
      viewer.flyTo(cesiumData, { duration: 1 });
      MessageWriter.info(`Located ${resource.name} via viewer.flyTo.`);
    } else {
      MessageWriter.warn(`Cannot locate resource ${resource.name}.`);
    }
  }
  @System({
    messageClass: LoadSavedResourcesMessage,
  })
  static loadSaved(
    settingsComponent: SettingComponent,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    for (const resource of settingsComponent.resources.reverse()) {
      switch (resource.type) {
        case "imagery":
          LoadImageryMessage.send(resource);
          break;
        case "terrain":
          LoadTerrainMessage.send(resource);
          break;
        case "tiles":
          LoadTilesMessage.send(resource);
          break;
        case "vector":
          LoadVectorMessage.send(resource);
          break;
      }
    }
  }
}

export function registerResourceSystems(app: ViridApp) {
  app.register(ResourceSystem.imagery);
  app.register(ResourceSystem.terrain);
  app.register(ResourceSystem.tiles3d);
  app.register(ResourceSystem.vector);
  app.register(ResourceSystem.show);
  app.register(ResourceSystem.sort);
  app.register(ResourceSystem.operate);
  app.register(ResourceSystem.remove);
  app.register(ResourceSystem.loadSaved);
  app.register(ResourceSystem.locate);
}
