import { System, type ViridApp, MessageWriter } from "@virid/core";
import {
  InitCesiumMessage,
  ResizeCesiumMessage,
  FlyToMessage,
  LoadImageryMessage,
  LoadTerrainMessage,
  LoadVectorMessage,
  Load3DTilesMessage,
  OperateResourceMessage,
  ShowResourceMessage,
  SortResourceMessage,
} from "./message";
import { CesiumComponent, ResourceComponent } from "./component";
import * as Cesium from "cesium";
import bg from "@/assets/imgs/bg.jpg";
import { match, P } from "ts-pattern";

export class CesiumSystem {
  @System()
  static init(_message: InitCesiumMessage, cesiumComponent: CesiumComponent) {
    const viewer = new Cesium.Viewer("map", {
      geocoder: false, // 地理编码控件不显示
      homeButton: false, // 默认相机位置控件不显示
      sceneModePicker: false, // 场景模式控件不显示
      baseLayerPicker: false, // 基础图层控件不显示
      navigationHelpButton: false, // 导航帮助控件不显示
      animation: false, // 动画控件不显示
      timeline: false, // 时间线控件不显示
      fullscreenButton: false, // 全屏控件不显示
      vrButton: false, // vr控件不显示
      infoBox: false, // 信息框控件不显示，点击要素后不弹出信息栏
      selectionIndicator: false, // 选择跟踪控件不显示，点击要素后不弹出锁定框
      contextOptions: {
        webgl: {
          alpha: true,
        },
      },
      // skyAtmosphere: true, // 关闭大气
      scene3DOnly: true, //如果设置为true，则所有几何图形以3D模式绘制以节约GPU资源
      skyBox: false,
    });
    viewer.scene.backgroundColor = new Cesium.Color(0.0, 0.0, 0.0, 0.0); //设置背景色透明
    viewer.scene.canvas.style.backgroundImage = `url(${bg})`; //设置背景图片
    viewer.scene.canvas.style.backgroundSize = "cover";

    viewer.scene.globe.depthTestAgainstTerrain = true;

    // 绑定相机回调
    const handler = new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas);

    handler.setInputAction(function (movement: any) {
      const ellipsoidPos = viewer.camera.pickEllipsoid(
        movement.endPosition,
        Cesium.Ellipsoid.WGS84,
      );

      if (Cesium.defined(ellipsoidPos)) {
        const carto = Cesium.Cartographic.fromCartesian(ellipsoidPos);
        const lon = Cesium.Math.toDegrees(carto.longitude);
        const lat = Cesium.Math.toDegrees(carto.latitude);
        const height = carto.height;
        cesiumComponent.cameraPosition = {
          lat,
          lon,
          height,
          pitch: Cesium.Math.toDegrees(viewer.camera.pitch),
          heading: Cesium.Math.toDegrees(viewer.camera.heading),
          roll: Cesium.Math.toDegrees(viewer.camera.roll),
        };
      } else {
      }
    }, Cesium.ScreenSpaceEventType.MOUSE_MOVE);
    cesiumComponent.screenSpaceEventHandler = handler;
    cesiumComponent.viewer = viewer;
    LoadImageryMessage.send({
      type: "imagery",
      name: "天地图底图",
      url: "https://{s}.tianditu.gov.cn/img_w/wmts?tk=16ea42d08457b4de2ea8228e1a65fe63",
      show: true,
      alpha: 1,
    });
    LoadImageryMessage.send({
      type: "imagery",
      name: "天地图注记",
      url: "http://t0.tianditu.gov.cn/cia_w/wmts?tk=16ea42d08457b4de2ea8228e1a65fe63",
      show: true,
      alpha: 1,
    });
  }
  @System({
    messageClass: ResizeCesiumMessage,
  })
  static resize(cesiumComponent: CesiumComponent) {
    cesiumComponent.viewer.resize();
  }
  @System()
  static flyTo(message: FlyToMessage, cesiumComponent: CesiumComponent) {
    cesiumComponent.viewer.camera.flyTo(message.options);
  }
}

export class ResourceSystem {
  @System()
  static imagery(
    message: LoadImageryMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const { resource } = message;
    const { url } = resource;
    const viewer = cesiumComponent.viewer;

    const provider = match(url)
      // 天地图
      .when(
        (u) => u.includes("tianditu.gov.cn"),
        () => {
          // 如果是 DataServer 格式（包含 {x}{y}{z}）
          if (url.includes("DataServer")) {
            return new Cesium.UrlTemplateImageryProvider({
              url,
              subdomains: ["0", "1", "2", "3", "4", "5", "6", "7"],
              tilingScheme: new Cesium.WebMercatorTilingScheme(),
              maximumLevel: 18,
            });
          }

          // 如果是 WMTS 格式，从 URL 中提取图层名，如 img_w 提取出 img
          const layerMatch = url.match(/\/([a-z]+)_w\/wmts/);
          const layerName = layerMatch ? layerMatch[1] : "img";

          return new Cesium.WebMapTileServiceImageryProvider({
            url,
            layer: layerName!,
            style: "default",
            format: "tiles",
            tileMatrixSetID: "w",
            tilingScheme: new Cesium.WebMercatorTilingScheme(),
            subdomains: ["0", "1", "2", "3", "4", "5", "6", "7"],
            maximumLevel: 18,
          });
        },
      )
      // 标准 XYZ 模板 (如 OSM, CartoDB)
      .when(
        (u) => u.includes("{z}") && u.includes("{x}") && u.includes("{y}"),
        () => new Cesium.UrlTemplateImageryProvider({ url }),
      )
      // WMS 服务
      .when(
        (u) =>
          u.toLowerCase().includes("service=wms") ||
          u.toLowerCase().includes("request=getmap"),
        () =>
          new Cesium.WebMapServiceImageryProvider({
            url: url.split("?")[0]!,
            layers: "0",
            parameters: Object.fromEntries(
              new URLSearchParams(url.split("?")[1] || ""),
            ),
          }),
      )
      // 标准 WMTS 服务 (非天地图)
      .when(
        (u) =>
          u.toLowerCase().includes("service=wmts") ||
          u.toLowerCase().includes("request=getcapabilities"),
        () =>
          new Cesium.WebMapTileServiceImageryProvider({
            url: url.split("?")[0]!,
            layer: "0",
            style: "default",
            format: "image/jpeg",
            tileMatrixSetID: "default",
          }),
      )
      .otherwise(() => {
        MessageWriter.error(new Error(`Unknown Imagery Provider: ${url}`));
        return;
      });

    if (!provider) return;
    const layer = viewer.imageryLayers.addImageryProvider(provider);
    layer.show = resource.show;
    resourceComponent.resourcesData.set(resource.name, layer);
    resourceComponent.resources.unshift(resource);
    MessageWriter.info(`Loaded Imagery ${resource.name} successfully.`);
  }

  @System()
  static async tiles3d(
    message: Load3DTilesMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const { resource } = message;
    const { url } = resource;
    const viewer = cesiumComponent.viewer;

    try {
      const tileset = await Cesium.Cesium3DTileset.fromUrl(url);

      if (url.toLowerCase().includes("pointcloud")) {
        tileset.pointCloudShading.attenuation = true;
      }

      const data = viewer.scene.primitives.add(tileset);
      resourceComponent.resourcesData.set(resource.name, data);
      resourceComponent.resources.unshift(resource);
      MessageWriter.info(`Loaded 3D Tiles: ${resource.name} successfully.`);
    } catch (error) {
      MessageWriter.error(new Error(`Failed to load 3D Tiles: ${url}`));
    }
  }

  @System()
  static async vector(
    message: LoadVectorMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const { resource } = message;
    const { url, name } = resource;
    const viewer = cesiumComponent.viewer;

    try {
      // 优先匹配特殊后缀，其余全部默认当做 GeoJSON 处理
      const dataSource = await match(url)
        .when(
          (u) => u.toLowerCase().split("?")[0]!.endsWith(".kml"),
          () => Cesium.KmlDataSource.load(url),
        )
        .when(
          (u) => u.toLowerCase().split("?")[0]!.endsWith(".czml"),
          () => Cesium.CzmlDataSource.load(url),
        )
        // 默认兜底
        .otherwise(() => Cesium.GeoJsonDataSource.load(url));

      dataSource.name = name || "Vector Layer";
      dataSource.show = resource.show;
      await viewer.dataSources.add(dataSource);
      resourceComponent.resourcesData.set(resource.name, dataSource);
      resourceComponent.resources.unshift(resource);
      MessageWriter.info(`Loaded Vector Data: ${resource.name} successfully.`);
    } catch (error) {
      MessageWriter.error(
        new Error(`Failed to load Vector Data: ${url} - ${error}`),
      );
    }
  }
  @System()
  static show(
    message: ShowResourceMessage,
    resourceComponent: ResourceComponent,
  ) {
    const { name, show } = message;

    const targetResource = resourceComponent.resources.find(
      (r) => r.name === name,
    )!;

    targetResource.show = show;
    const cesiumData = resourceComponent.resourcesData.get(name);

    if (cesiumData) {
      cesiumData.show = show;
    }
  }
  @System()
  static sort(
    message: SortResourceMessage,
    cesiumComponent: CesiumComponent,
    resourceComponent: ResourceComponent,
  ) {
    const { names } = message;
    const viewer = cesiumComponent.viewer;

    const nameIndexMap = new Map<string, number>();
    names.forEach((name, idx) => nameIndexMap.set(name, idx));

    const sorted = [...resourceComponent.resources].sort((a, b) => {
      const posA = nameIndexMap.get(a.name) ?? Infinity;
      const posB = nameIndexMap.get(b.name) ?? Infinity;
      return posA - posB;
    });
    resourceComponent.resources = sorted;

    // 3. 倒序 raiseToTop，保证 names[0] 最终处于集合最顶层
    for (let i = names.length - 1; i >= 0; i--) {
      const name = names[i]!;

      const targetResource = resourceComponent.resources.find(
        (r) => r.name === name,
      );
      if (!targetResource) continue;

      const cesiumData = resourceComponent.resourcesData.get(
        targetResource.name,
      );
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

    MessageWriter.info(`Sorted ${names.length} resources successfully.`);
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
}

export function registerCesiumSystems(app: ViridApp) {
  app.register(CesiumSystem.init);
  app.register(CesiumSystem.resize);
  app.register(CesiumSystem.flyTo);
  app.register(ResourceSystem.imagery);
  // app.register(ResourceSystem.terrain);
  app.register(ResourceSystem.tiles3d);
  app.register(ResourceSystem.vector);
  app.register(ResourceSystem.show);
  app.register(ResourceSystem.sort);
  app.register(ResourceSystem.operate);
}
