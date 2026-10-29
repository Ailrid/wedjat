import { Controller, MessageWriter } from "@virid/core";
import { CesiumComponent, type CameraPosition } from "@/ccs/cesium";
import { Project } from "@virid/vue";
import { FlyToMessage } from "@/ccs/cesium";
import * as Cesium from "cesium";

@Controller()
export class CameraPositionController {
  @Project(CesiumComponent, (i) => i.cameraPosition)
  public cameraPosition!: CameraPosition;

  public resetCamera() {
    FlyToMessage.send({
      destination: Cesium.Cartesian3.fromDegrees(113.0, 33.0, 20000000),
      orientation: {
        heading: Cesium.Math.toRadians(0.001), // 正北方向
        pitch: Cesium.Math.toRadians(-90.0), // 垂直向下看
        roll: 0.0,
      },
      duration: 1.0,
    });
    MessageWriter.info("Reset Camera Successfully!");
  }
}
