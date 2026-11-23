import { System, type ViridApp } from "@virid/core";
import {
  InitCesiumMessage,
  ResizeCesiumMessage,
  FlyToMessage,
} from "./message";
import { CesiumComponent } from "./component";
import * as Cesium from "cesium";
import bg from "@/assets/imgs/bg.jpg";

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
      baseLayer: false,
    });
    viewer.scene.backgroundColor = new Cesium.Color(0.0, 0.0, 0.0, 0.0); //设置背景色透明
    viewer.scene.canvas.style.backgroundImage = `url(${bg})`; //设置背景图片
    viewer.scene.canvas.style.backgroundSize = "cover";

    viewer.scene.globe.depthTestAgainstTerrain = true;
    const controller = viewer.scene.screenSpaceCameraController;

    // 降低缩放速度
    controller.zoomFactor = 1.0; 

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
    // 监听左键双击事件
    handler.setInputAction((movement: any) => {
      const pickedObject = viewer.scene.pick(movement.position);

      if (!Cesium.defined(pickedObject)) viewer.trackedEntity = undefined;
    }, Cesium.ScreenSpaceEventType.LEFT_DOUBLE_CLICK);
    cesiumComponent.screenSpaceEventHandler = handler;
    cesiumComponent.viewer = viewer;
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

export function registerCesiumSystems(app: ViridApp) {
  app.register(CesiumSystem.init);
  app.register(CesiumSystem.resize);
  app.register(CesiumSystem.flyTo);
}
