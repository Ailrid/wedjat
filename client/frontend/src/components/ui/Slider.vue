<template>
  <div
    :class="['group relative flex h-6 w-full items-center', $attrs.class]"
    @wheel.prevent="handleWheel"
  >
    <div class="absolute h-1.5 w-full rounded-lg bg-gray-200"></div>

    <div
      class="bg-primary absolute h-1.5 rounded-lg"
      :style="{ width: `${percentage}%` }"
    ></div>

    <div
      class="bg-primary absolute h-4 w-4 rounded-full border-2 border-white shadow-sm transition-transform group-hover:scale-110"
      :style="{ left: `calc(${percentage}% - 0.5rem)` }"
    ></div>

    <input
      type="range"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step"
      @input="updateValue"
      class="absolute z-10 h-full w-full cursor-pointer opacity-0"
    />
  </div>
</template>

<script setup lang="ts">
  import { computed } from "vue";

  interface Props {
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
    /** 滚轮一次滚动的步长，默认等于 step */
    wheelStep?: number;
  }

  const props = withDefaults(defineProps<Props>(), {
    modelValue: 0,
    min: 0,
    max: 1,
    step: 0.01,
  });

  const emit = defineEmits(["update:modelValue"]);

  const percentage = computed(() => {
    const range = props.max - props.min;
    return ((props.modelValue - props.min) / range) * 100;
  });

  const updateValue = (event: Event) => {
    const target = event.target as HTMLInputElement;
    emit("update:modelValue", parseFloat(target.value));
  };

  const handleWheel = (event: WheelEvent) => {
    // 忽略纯水平滚动和 Ctrl+滚轮（浏览器缩放）
    if (event.deltaY === 0 || event.ctrlKey) return;

    const direction = event.deltaY > 0 ? 1 : -1; // 向上滚 = 增大
    const stepValue = props.wheelStep ?? props.step;

    let next = props.modelValue + direction * stepValue;

    // 1) clamp 到范围
    next = Math.min(props.max, Math.max(props.min, next));
    // 2) 对齐到 step 的整数倍，并修掉浮点误差
    next = Math.round(next / props.step) * props.step;
    next = parseFloat(next.toFixed(10));

    if (next !== props.modelValue) {
      emit("update:modelValue", next);
    }
  };
</script>
