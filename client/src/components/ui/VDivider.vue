<template>
  <div ref="rootRef" class="flex h-full w-full overflow-hidden">
    <div
      class="min-w-0 shrink-0 overflow-hidden"
      :style="{ width: leftWidth + 'px' }"
    >
      <slot name="left" />
    </div>

    <div
      class="shrink-0 cursor-col-resize touch-none hover:bg-primary"
      :style="{ width: dividerSize + 'px' }"
      @pointerdown="onDown"
      @pointermove="onMove"
      @pointerup="onUp"
    />

    <div class="min-w-0 flex-1 overflow-hidden">
      <slot name="right" />
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted } from "vue";

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

  function onDown(e: PointerEvent) {
    dragging = true;
    startX = e.clientX;
    startLeft = leftWidth.value;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onMove(e: PointerEvent) {
    if (!dragging) return;
    const total = rootRef.value?.clientWidth ?? 0;
    const max = Math.max(0, total - props.dividerSize);
    leftWidth.value = Math.min(
      Math.max(startLeft + e.clientX - startX, 0),
      max,
    );
  }

  function onUp(e: PointerEvent) {
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }

  onMounted(() => {
    const total = rootRef.value?.clientWidth ?? 0;
    const ratio = Math.min(Math.max(props.initialRatio, 0), 1);
    leftWidth.value = Math.max(0, total - props.dividerSize) * ratio;
  });
</script>
