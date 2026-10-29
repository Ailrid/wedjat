import { Controller } from "@virid/core";
import { OnHook, Project, Use } from "@virid/vue";
import { useTemplateRef, type ShallowRef } from "vue";
import {
  type Resource,
  ResourceComponent,
  SortResourceMessage,
} from "@/ccs/cesium";

@Controller()
export class ResourceController {
  @Project(ResourceComponent, (i) => i.resources)
  public resources!: Resource[];

  @Use(() => useTemplateRef("containerRef"))
  public containerRef!: ShallowRef<HTMLElement | null>;

  /** 当前正在拖拽的源索引，-1 表示未拖拽 */
  public dragIndex: number = -1;

  private scrollTimer: number | null = null;
  /** 记录当前滚动方向，用于方向变化时重建 timer */
  private scrollSpeed: number = 0;

  @OnHook("onUnmounted")
  public onUnmounted() {
    // 防止组件在拖拽过程中被销毁导致 timer 泄漏
    this.stopAutoScroll();
  }

  public onDragStart(index: number, event: DragEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.closest("input, button")) {
      event.preventDefault();
      return;
    }
    this.dragIndex = index;

    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      // Firefox 必须 setData 才能触发拖拽
      event.dataTransfer.setData("text/plain", "");
    }
  }

  public onDragOver(event: DragEvent) {
    if (this.dragIndex === -1 || !this.containerRef.value) return;

    const rect = this.containerRef.value.getBoundingClientRect();
    const threshold = 40; // 距离边缘 40px 触发滚动
    const maxSpeed = 8;

    let speed = 0;
    if (event.clientY < rect.top + threshold) {
      speed = -maxSpeed;
    } else if (event.clientY > rect.bottom - threshold) {
      speed = maxSpeed;
    }

    if (speed === 0) {
      this.stopAutoScroll();
      return;
    }
    // 方向未变化，复用现有 timer（闭包里的 speed 仍然有效）
    if (this.scrollSpeed === speed) return;

    // 方向发生变化，重建 timer
    this.stopAutoScroll();
    this.scrollSpeed = speed;
    this.scrollTimer = window.setInterval(() => {
      this.containerRef.value?.scrollBy(0, speed);
    }, 16);
  }

  public onDragEnd(event: DragEvent) {
    this.stopAutoScroll();

    if (this.dragIndex === -1 || !this.containerRef.value) {
      this.dragIndex = -1;
      return;
    }

    // 拖出容器再松手视为取消，避免用容器外坐标误判
    const containerRect = this.containerRef.value.getBoundingClientRect();
    if (
      event.clientY < containerRect.top ||
      event.clientY > containerRect.bottom
    ) {
      this.dragIndex = -1;
      return;
    }

    // 关键修复：li 在 ul 里面，不是 container 的直接子元素
    const listEl = this.containerRef.value.querySelector("ul");
    if (!listEl) {
      this.dragIndex = -1;
      return;
    }
    const children = Array.from(listEl.children) as HTMLElement[];

    // 根据子元素中线计算插入间隙索引 [0, children.length]
    const mouseY = event.clientY;
    let targetIndex = children.length;
    for (let i = 0; i < children.length; i++) {
      const rect = children[i]!.getBoundingClientRect();
      if (mouseY < rect.top + rect.height / 2) {
        targetIndex = i;
        break;
      }
    }

    // map 会返回普通可写数组，不需要 JSON 深拷贝整个 Resource 对象

    let names = this.resources.map((r) => r.name);
    names = JSON.parse(JSON.stringify(names));
    const [movedItem] = names.splice(this.dragIndex, 1);

    // 从上往下拖时，移除后目标索引要减 1
    let insertIndex =
      this.dragIndex < targetIndex ? targetIndex - 1 : targetIndex;
    insertIndex = Math.max(0, Math.min(insertIndex, names.length));

    // 无实际变化（包括拖到自己中线上/下方）直接返回，避免发送无效消息
    if (insertIndex === this.dragIndex) {
      this.dragIndex = -1;
      return;
    }

    names.splice(insertIndex, 0, movedItem!);
    SortResourceMessage.send(names);
    console.log(names);
    this.dragIndex = -1;
  }

  private stopAutoScroll() {
    if (this.scrollTimer !== null) {
      clearInterval(this.scrollTimer);
      this.scrollTimer = null;
    }
    this.scrollSpeed = 0;
  }
}
