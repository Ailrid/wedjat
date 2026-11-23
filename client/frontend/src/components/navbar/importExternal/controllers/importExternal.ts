import type { ResourceType } from "@/ccs/resource";
import { Controller, EventMessage } from "@virid/core";
import { Listener, Responsive } from "@virid/vue";
export interface ExtraTag {
  isPointCloud: boolean;
}

export class OpenExternalDialogMessage extends EventMessage {
  constructor(
    public type: ResourceType,
    public extra?: ExtraTag,
  ) {
    super();
  }
}
export class CloseExternalDialogMessage extends EventMessage {}

@Controller()
export class ImportExternalController {
  @Responsive()
  public isShowing: boolean = false;
  @Responsive()
  public type: ResourceType = "imagery";
  @Responsive()
  public extra: ExtraTag | null = null;

  @Responsive()
  public url: string = "";

  @Responsive()
  public name: string = "";

  @Listener()
  openDialog(message: OpenExternalDialogMessage) {
    this.isShowing = true;
    this.type = message.type;
    this.extra = message.extra || null;
  }
  @Listener({
    messageClass: CloseExternalDialogMessage,
  })
  closeDialog() {
    this.isShowing = false;
  }
}
