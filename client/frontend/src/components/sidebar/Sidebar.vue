<template>
  <div class="w-full h-full flex">
    <div class="w-14 h-full bg-black/20 flex flex-col">
      <button
        class="button"
        :class="{ active: sbc.tab == 'resource' }"
        @click="sbc.tab = 'resource'"
      >
        <Map></Map>
      </button>
      <button
        class="button"
        :class="{ active: sbc.tab == 'entity' }"
        @click="sbc.tab = 'entity'"
      >
        <Blocks></Blocks>
      </button>
    </div>
    <div class="flex-1">
      <component :is="componentMap[sbc.tab]"></component>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { Map, Blocks } from "lucide-vue-next";
  import { SidebarController } from "./controllers";
  import Resource from "./Resource.vue";
  import Entity from "./Entity.vue";
  const sbc = useController(SidebarController);
  const componentMap = {
    resource: Resource,
    entity: Entity,
  };
</script>

<style scoped>
  @reference "@/assets/main.css";

  .active {
    @apply text-white!;
    @apply shadow-[inset_2px_0_0_0_#fff];
  }

  .button {
    @apply w-14 h-14 flex items-center justify-center hover:text-white cursor-pointer text-white/50;
  }
</style>
