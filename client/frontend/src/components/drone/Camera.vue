<template>
  <div
    v-if="ct.showCamera"
    class="w-[50%] h-full flex items-center justify-center"
  >
    <div class="text-3xl">无连接</div>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { CameraController } from "./controllers";

  const ct = useController(CameraController);
</script>

<style scoped></style>
