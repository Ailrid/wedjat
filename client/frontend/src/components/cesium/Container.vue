<template>
  <div class="w-full h-full" ref="container"></div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { ContainerController } from "./controllers";
  useController(ContainerController);
</script>

<style scoped></style>
