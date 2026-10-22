import { Controller } from "@virid/core";
import { Listener, OnHook, Project, Responsive } from "@virid/vue";
import {
  InitCesiumMessage,
  FlyToMessage,
  ResizeCesiumMessage,
} from "@/ccs/cesium";
import * as Cesium from "cesium";
import { ContainerResizeMessage } from "./container";
@Controller()
export class CesiumController {
  @Responsive()
  public rect: DOMRect = new DOMRect();

  @Project()
  get mapRect() {
    return {
      width: `${this.rect.width}px`,
      height: `${this.rect.height}px`,
      left: `${this.rect.left}px`,
      top: `${this.rect.top}px`,
    };
  }

  // 挂载容器
  @OnHook("onMounted")
  mounted() {
    InitCesiumMessage.send();
    FlyToMessage.send({
      destination: Cesium.Cartesian3.fromDegrees(113.0, 33.0, 20000000),
      orientation: {
        heading: Cesium.Math.toRadians(0.0), // 正北方向
        pitch: Cesium.Math.toRadians(-90.0), // 垂直向下看
        roll: 0.0,
      },
      duration: 1.0,
    });
  }

  // 监听容器尺寸变化
  @Listener()
  resize(message: ContainerResizeMessage) {
    this.rect = message.rect;
    ResizeCesiumMessage.send();
  }
}
