import { Controller, EventMessage } from "@virid/core";
import { OnHook, Use } from "@virid/vue";
import { useTemplateRef, type ShallowRef } from "vue";

export class ContainerResizeMessage extends EventMessage {
  constructor(public rect: DOMRect) {
    super();
  }
}

@Controller()
export class ContainerController {
  @Use(() => useTemplateRef("container"))
  public containerRef!: ShallowRef<HTMLElement | null>;

  public observer!: ResizeObserver;

  resize() {
    if (!this.containerRef.value) return;
    const rect = this.containerRef.value.getBoundingClientRect();
    ContainerResizeMessage.send(rect);
  }

  @OnHook("onMounted")
  onMounted() {
    this.resize();
    if (!this.containerRef.value) return;
    this.observer = new ResizeObserver(() => {
      if (!this.containerRef.value) return;
      const rect = this.containerRef.value.getBoundingClientRect();
      ContainerResizeMessage.send(rect);
    });
    this.observer.observe(this.containerRef.value);
  }
  @OnHook("onUnmounted")
  onUnmounted() {
    this.observer.disconnect();
  }
}
