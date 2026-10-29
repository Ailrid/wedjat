<template>
  <div class="flex items-center gap-3 leading-none pr-2">
    <button class="button" @click="cpt.resetCamera">
      <CircleArrowUp
        :size="16"
        class="shrink-0"
        :style="{ rotate: `${cpt.cameraPosition.heading}deg` }"
      />
      <span>复位</span>
    </button>

    <div class="leading-none tabular-nums">
      经度：{{ cpt.cameraPosition.lon.toFixed(6) }}
    </div>
    <div class="leading-none tabular-nums">
      纬度：{{ cpt.cameraPosition.lat.toFixed(6) }}
    </div>
    <div class="leading-none tabular-nums">
      偏航：{{ cpt.cameraPosition.heading.toFixed(2) }}
    </div>
  </div>
</template>
<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { CameraPositionController } from "./controllers";
  import { CircleArrowUp } from "lucide-vue-next";
  let cpt = useController(CameraPositionController);
</script>

<style scoped>
  @reference "@/assets/main.css";

  .button {
    @apply inline-flex h-full items-center justify-center gap-1 px-2 leading-none hover:bg-black active:bg-black transition-colors cursor-pointer;
  }
</style>
