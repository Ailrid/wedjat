import { Controller } from "@virid/core";
import { Responsive } from "@virid/vue";

export type PanelTab = "locate" | "vision" | "inertial" | "setting";

@Controller()
export class PanelController {
  @Responsive()
  public tab: PanelTab = "locate";
}
