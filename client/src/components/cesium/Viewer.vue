<template>
  <div class="absolute z-1000" id="map" :style="cct.mapRect"></div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { CesiumController } from "./controllers";
  const cct = useController(CesiumController);
</script>

<style scoped></style>
