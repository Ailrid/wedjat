<template>
  <div ref="rootRef" class="flex h-full w-full flex-col overflow-hidden">
    <div
      class="min-h-0 shrink-0 overflow-hidden"
      :style="{ height: topHeight + 'px' }"
    >
      <slot name="top" />
    </div>

    <div
      class="shrink-0 cursor-row-resize touch-none hover:bg-primary"
      :style="{ height: dividerSize + 'px' }"
      @pointerdown="onDown"
      @pointermove="onMove"
      @pointerup="onUp"
    />

    <div class="min-h-0 flex-1 overflow-hidden">
      <slot name="bottom" />
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted } from "vue";

  const props = withDefaults(
    defineProps<{
      /** 初始化时上部分占比 0~1 */
      initialRatio?: number;
      /** 分割线厚度 px */
      dividerSize?: number;
    }>(),
    { initialRatio: 0.7, dividerSize: 2 },
  );

  const rootRef = ref<HTMLDivElement | null>(null);
  const topHeight = ref(0);

  let startY = 0;
  let startTop = 0;
  let dragging = false;

  function onDown(e: PointerEvent) {
    dragging = true;
    startY = e.clientY;
    startTop = topHeight.value;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onMove(e: PointerEvent) {
    if (!dragging) return;
    const total = rootRef.value?.clientHeight ?? 0;
    const max = Math.max(0, total - props.dividerSize);
    topHeight.value = Math.min(Math.max(startTop + e.clientY - startY, 0), max);
  }

  function onUp(e: PointerEvent) {
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }

  onMounted(() => {
    const total = rootRef.value?.clientHeight ?? 0;
    const ratio = Math.min(Math.max(props.initialRatio, 0), 1);
    topHeight.value = Math.max(0, total - props.dividerSize) * ratio;
  });
</script>
