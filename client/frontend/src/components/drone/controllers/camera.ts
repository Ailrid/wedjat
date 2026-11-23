import { Controller, EventMessage } from "@virid/core";
import { Listener, Responsive } from "@virid/vue";

export class ToggleCameraMessage extends EventMessage {}

@Controller()
export class CameraController {
  @Responsive()
  public showCamera: boolean = false;

  @Listener({
    messageClass: ToggleCameraMessage,
  })
  show() {
    this.showCamera = !this.showCamera;
  }
}
