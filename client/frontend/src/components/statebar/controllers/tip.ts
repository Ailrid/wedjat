import { Controller, EventMessage } from "@virid/core";
import { Project } from "@virid/vue";
import { GlobalComponent } from "@/ccs/global";

@Controller()
export class TipController {
  @Project(GlobalComponent, (i) => i.tips)
  public tips!: string[];

  @Project(GlobalComponent, (i) => i.tips.at(-1) ?? "")
  public tip!: string;

  tipColor(tip: string) {
    if (tip.startsWith("[INFO]")) {
      return {
        color: "#00b906",
      };
    } else if (tip.startsWith("[WARN]")) {
      return {
        color: "#ff9800",
      };
    } else {
      return {
        color: "#ff1100",
      };
    }
  }
}
