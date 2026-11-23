import { Controller } from "@virid/core";
import { Responsive } from "@virid/vue";
import { LoadTerrainMessage, type Resource } from "@/ccs/resource";
import { v4 as uuidv4 } from "uuid";
import { CloseExternalDialogMessage } from "./importExternal";
import { SaveSettingsMessage } from "@/ccs/settings";

@Controller()
export class ExternalTerrainController {
  @Responsive()
  public name: string = "";
  @Responsive()
  public url: string = "";
  @Responsive()
  public credit: string = "";

  @Responsive()
  public requestVertexNormals: boolean = true;
  @Responsive()
  public requestWaterMask: boolean = false;

  @Responsive()
  public autoLoad: boolean = false;

  public import() {
    if (!this.name || !this.url) return;
    const resource: Resource = {
      id: uuidv4(),
      name: this.name,
      url: this.url,
      type: "terrain",
      show: true,
      alpha: 1,
      extra: {
        terrain: {
          requestVertexNormals: this.requestVertexNormals,
          requestWaterMask: this.requestWaterMask,
          credit: this.credit,
        },
      },
    };

    if (this.autoLoad) {
      SaveSettingsMessage.send((settings) => {
        settings.resources.push(resource);
      });
    }

    LoadTerrainMessage.send(resource);
    CloseExternalDialogMessage.send();
  }
}
