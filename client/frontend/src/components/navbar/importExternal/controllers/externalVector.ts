import { Controller } from "@virid/core";
import { Responsive, Use } from "@virid/vue";
import { useTemplateRef } from "vue";
import {
  type Resource,
  type VectorType,
  LoadVectorMessage,
} from "@/ccs/resource";
import { v4 as uuidv4 } from "uuid";
import { CloseExternalDialogMessage } from "./importExternal";

@Controller()
export class ExternalVectorController {
  @Responsive() public name: string = "";
  @Responsive() public fileName: string = "";
  @Responsive() public format: VectorType = "geojson";
  @Responsive() public clampToGround: boolean = true;

  @Responsive() public pointColor: string = "#ffffff";
  @Responsive() public pointSize: number = 8;
  @Responsive() public pointOutlineColor: string = "#000000";
  @Responsive() public pointOutlineWidth: number = 1;

  @Responsive() public polylineColor: string = "#ffffff";
  @Responsive() public polylineWidth: number = 2;

  @Responsive() public polygonColor: string = "#3b82f680";
  @Responsive() public polygonOutline: boolean = true;
  @Responsive() public polygonOutlineColor: string = "#ffffff";
  @Responsive() public polygonOutlineWidth: number = 1;

  private file: File | null = null;

  @Use(() => useTemplateRef<HTMLInputElement>("fileInput"))
  public fileInput!: ReturnType<typeof useTemplateRef<HTMLInputElement>>;

  public openFileDialog() {
    this.fileInput.value?.click();
  }

  public handleFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    this.file = file;
    this.fileName = file.name;

    const lower = file.name.toLowerCase();
    this.format =
      lower.endsWith(".kml") || lower.endsWith(".kmz") ? "kml" : "geojson";

    // 若用户未手动填名称，用文件名（去后缀）作为默认
    if (!this.name) {
      this.name = file.name.replace(/\.[^.]+$/, "");
    }
  }

  public import() {
    if (!this.name || !this.file) return;

    // blob URL 只在当前页面生命周期内有效
    const url = URL.createObjectURL(this.file);

    const resource: Resource = {
      id: uuidv4(),
      name: this.name,
      url,
      type: "vector",
      show: true,
      alpha: 1,
      extra: {
        vector: {
          type: this.format,
          clampToGround: this.clampToGround,
          point: {
            color: this.pointColor,
            size: this.pointSize,
            outlineColor: this.pointOutlineColor,
            outlineWidth: this.pointOutlineWidth,
          },
          polyline: {
            color: this.polylineColor,
            width: this.polylineWidth,
          },
          polygon: {
            color: this.polygonColor,
            outline: this.polygonOutline,
            outlineColor: this.polygonOutlineColor,
            outlineWidth: this.polygonOutlineWidth,
          },
        },
      },
    };

    LoadVectorMessage.send(resource);
    CloseExternalDialogMessage.send();
  }
}
