import { Controller } from "@virid/core";
import { Project, Env } from "@virid/vue";
import { type Resource, ResourceComponent } from "@/ccs/cesium";

@Controller()
export class TilesController {
  @Env()
  public resource!: Resource;
}
