import { Component, type ViridApp } from "@virid/core";
import { Responsive } from "@virid/vue";

@Component()
export class GlobalComponent {
  @Responsive()
  public tips: string[] = [];
}

export function bindGlobalComponent(app: ViridApp) {
  app.bind(GlobalComponent);
}
