<template>
  <div class="w-full h-full flex">
    <div class="flex-1" ref="container"></div>
    <Camera></Camera>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { ContainerController } from "./controllers";
  import Camera from "../drone/Camera.vue";
  useController(ContainerController);
</script>

<style scoped></style>
