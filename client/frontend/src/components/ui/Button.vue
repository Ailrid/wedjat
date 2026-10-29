<template>
  <button
    class="cursor-pointer leading-none"
    :class="[variantMap[props.variant]]"
  >
    <slot></slot>
  </button>
</template>

<script setup lang="ts">
  const props = withDefaults(
    defineProps<{
      variant?: "icon" | "outline" | "none";
    }>(),
    {
      variant: "outline",
    },
  );

  const variantMap = {
    icon: "rounded-sm",
    outline: "rounded-sm hover:bg-white/10 hover:text-white p-2",
    none: "",
  };
</script>

<style scoped></style>
