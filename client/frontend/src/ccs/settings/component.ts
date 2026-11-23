import { Component } from "@virid/core";
import { Responsive } from "@virid/vue";
import { type Resource } from "@/ccs/resource";

@Component()
export class SettingComponent {
  @Responsive()
  public resources: Resource[] = [];
}
