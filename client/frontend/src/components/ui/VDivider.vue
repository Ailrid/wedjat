<template>
  <div ref="rootRef" class="flex h-full w-full">
    <div
      class="h-full overflow-auto"
      :style="{ width: leftWidth + 'px', flexShrink: 0 }"
    >
      <slot name="left" />
    </div>

    <div
      class="cursor-col-resize touch-none hover:bg-primary"
      :style="{ width: dividerSize + 'px', flexShrink: 0 }"
      @pointerdown="onDown"
      @pointermove="onMove"
      @pointerup="onUp"
      @pointercancel="onUp"
    />

    <div class="flex-1">
      <slot name="right" />
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted, onBeforeUnmount } from "vue";

  const props = withDefaults(
    defineProps<{
      /** 初始化时左部分占比 0~1 */
      initialRatio?: number;
      /** 分割线宽度 px */
      dividerSize?: number;
    }>(),
    { initialRatio: 0.3, dividerSize: 2 },
  );

  const rootRef = ref<HTMLDivElement | null>(null);
  const leftWidth = ref(0);

  let startX = 0;
  let startLeft = 0;
  let dragging = false;

  // 当前占比，用于窗口尺寸变化时按比例重算
  let currentRatio = clamp(props.initialRatio);

  function clamp(v: number) {
    return Math.min(Math.max(v, 0), 1);
  }

  /** 可分配给左/右区域的总宽度（扣掉分割线） */
  function getAvailable() {
    const total = rootRef.value?.clientWidth ?? 0;
    return Math.max(0, total - props.dividerSize);
  }

  function applyRatio(ratio: number) {
    currentRatio = clamp(ratio);
    leftWidth.value = getAvailable() * currentRatio;
  }

  function onDown(e: PointerEvent) {
    dragging = true;
    startX = e.clientX;
    startLeft = leftWidth.value;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onMove(e: PointerEvent) {
    if (!dragging) return;
    const max = getAvailable();
    const next = Math.min(Math.max(startLeft + e.clientX - startX, 0), max);
    leftWidth.value = next;
    currentRatio = max > 0 ? next / max : 0;
  }

  function onUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    const el = e.currentTarget as HTMLElement;
    if (el.hasPointerCapture?.(e.pointerId)) {
      el.releasePointerCapture(e.pointerId);
    }
  }

  let ro: ResizeObserver | null = null;

  onMounted(() => {
    // 先按初始比例算一次（即使容器宽度暂为 0，RO 首帧也会纠正）
    applyRatio(props.initialRatio);

    ro = new ResizeObserver(() => {
      // 拖拽中不要打断用户操作
      if (dragging) return;
      leftWidth.value = getAvailable() * currentRatio;
    });
    if (rootRef.value) ro.observe(rootRef.value);
  });

  onBeforeUnmount(() => {
    ro?.disconnect();
    ro = null;
  });
</script>
