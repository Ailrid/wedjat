<template>
  <div ref="dropdownRef">
    <slot name="trigger" :toggle="toggle" :is-open="isOpen"></slot>
    <Teleport to="#dialog">
      <Transition
        enter-active-class="transition duration-100 ease-out"
        enter-from-class="transform scale-95 opacity-0"
        enter-to-class="transform scale-100 opacity-100"
        leave-active-class="transition duration-75 ease-in"
        leave-from-class="transform scale-100 opacity-100"
        leave-to-class="transform scale-95 opacity-0"
      >
        <div
          v-if="isOpen"
          ref="contentRef"
          :style="dropdownStyle"
          class="fixed z-1000 bg-card border-black/50 border rounded-sm"
          @click="isOpen = false"
        >
          <slot></slot>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted, onUnmounted, computed } from "vue";
  const props = withDefaults(
    defineProps<{
      position?: "top" | "bottom" | "left" | "right";
    }>(),
    {
      position: "bottom",
    },
  );
  const isOpen = ref(false);
  const dropdownRef = ref<HTMLElement | null>(null);
  const contentRef = ref<HTMLElement | null>(null);
  const triggerRect = ref<DOMRect | null>(null);

  // 计算弹出位置（对齐触发器下方）
  const dropdownStyle = computed(() => {
    if (!triggerRect.value) return {};
    if (props.position === "bottom") {
      return {
        top: `${triggerRect.value.bottom + 4}px`,
        left: `${triggerRect.value.left}px`,
      };
    } else if (props.position === "top") {
      return {
        bottom: `${window.innerHeight - triggerRect.value.top + 4}px`,
        left: `${triggerRect.value.left}px`,
      };
    } else if (props.position === "left") {
      return {
        top: `${triggerRect.value.top}px`,
        right: `${window.innerWidth - triggerRect.value.left + 4}px`,
      };
    } else if (props.position === "right") {
      return {
        top: `${triggerRect.value.top}px`,
        left: `${triggerRect.value.right + 4}px`,
      };
    }
  });

  const updateRect = () => {
    if (dropdownRef.value) {
      triggerRect.value = dropdownRef.value.getBoundingClientRect();
    }
  };

  // 处理点击外部关闭
  const handleClickOutside = (event: MouseEvent) => {
    if (
      dropdownRef.value &&
      !dropdownRef.value.contains(event.target as Node) &&
      contentRef.value &&
      !contentRef.value.contains(event.target as Node)
    ) {
      isOpen.value = false;
    }
  };

  onMounted(() => {
    window.addEventListener("click", handleClickOutside);
    window.addEventListener("resize", updateRect);
    window.addEventListener("scroll", updateRect, true);
  });

  onUnmounted(() => {
    window.removeEventListener("click", handleClickOutside);
    window.removeEventListener("resize", updateRect);
    window.removeEventListener("scroll", updateRect, true);
  });

  // 暴露位置更新方法给父组件在某些特殊布局下调用
  defineExpose({ updateRect });

  // 监听 isOpen 并在打开时刷新位置
  import { watch } from "vue";
  watch(isOpen, (newVal) => {
    if (newVal) updateRect();
  });

  function toggle() {
    isOpen.value = !isOpen.value;
  }
</script>
