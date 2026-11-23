import { SettingComponent } from "./component";
import { registerSettingSystems } from "./system";
import { type ViridApp } from "@virid/core";
export * from "./component";

export * from "./system";
export * from "./message";
export function bindSettingBunch(app: ViridApp) {
  app.bind(SettingComponent);
  registerSettingSystems(app);
}
