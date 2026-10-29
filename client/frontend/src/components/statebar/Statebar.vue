<template>
  <div class="w-full h-full flex text-xs">
    <Tip></Tip>
    <div class="flex-1"></div>
    <CameraPosition></CameraPosition>
  </div>
</template>

<script setup lang="ts">
  import CameraPosition from "./CameraPosition.vue";
  import Tip from "./Tip.vue";
</script>

<style scoped></style>
