import {
  System,
  InfoMessage,
  WarnMessage,
  ErrorMessage,
  ViridApp,
} from "@virid/core";
import { GlobalComponent } from "./component";

export class LogSystem {
  @System()
  static info(message: InfoMessage, globalComponent: GlobalComponent) {
    globalComponent.tips.push(`[INFO] ${message.context} `);
  }
  @System()
  static warn(message: WarnMessage, globalComponent: GlobalComponent) {
    globalComponent.tips.push(`[WARN] ${message.context}`);
  }
  @System()
  static error(message: ErrorMessage, globalComponent: GlobalComponent) {
    globalComponent.tips.push(`[ERROR] ${message.context} ${message.error}`);
  }
}

export function registerGlobalSystems(app: ViridApp) {
  app.register(LogSystem.error);
  app.register(LogSystem.warn);
  app.register(LogSystem.info);
}
