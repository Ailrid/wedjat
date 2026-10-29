import { Controller } from "@virid/core";
import { OnHook, Use } from "@virid/vue";
import {
  InitCesiumMessage,
  ResizeCesiumMessage,
  FlyToMessage,
} from "@/ccs/cesium";
import { useTemplateRef, type ShallowRef } from "vue";
import * as Cesium from "cesium";

@Controller()
export class PanelController {

}
