import { Controller } from "@virid/core";
import { Responsive } from "@virid/vue";

type Tabs = "resource" | "entity";

@Controller()
export class SidebarController {
  @Responsive()
  public tab: Tabs = "resource";
}
