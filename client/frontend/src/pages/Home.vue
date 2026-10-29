<template>
  <div class="h-full w-full flex flex-col">
    <div class="w-full h-9 nav-bar">
      <Navbar></Navbar>
    </div>
    <div class="flex-1 flex">
      <VDivider :initialRatio="0.2">
        <template #left>
          <div class="w-full h-full side-bar">
            <Sidebar></Sidebar>
          </div>
        </template>
        <template #right>
          <HDivider :initialRatio="0.7">
            <template #top>
              <Container />
            </template>
            <template #bottom>
              <div class="w-full h-full panel">
                <Panel></Panel>
              </div>
            </template>
          </HDivider>
        </template>
      </VDivider>
    </div>
    <div class="w-full h-6 state-bar"><Statebar></Statebar></div>
  </div>
</template>

<script setup lang="ts">
  import Navbar from "@/components/navbar/Navbar.vue";
  import Sidebar from "@/components/sidebar/Sidebar.vue";
  import Statebar from "@/components/statebar/Statebar.vue";
  import Panel from "@/components/panel/Panel.vue";
  import Container from "@/components/cesium/Container.vue";
  import VDivider from "@/components/ui/VDivider.vue";
  import HDivider from "@/components/ui/HDivider.vue";
</script>
<style scoped>
  @reference "@/assets/main.css";

  .side-bar {
    @apply bg-sidebar z-10 overflow-hidden border-r border-black/5
           shadow-[4px_0_14px_-4px_rgba(0,0,0,0.08)]
           dark:border-black dark:shadow-none;
  }

  .nav-bar {
    @apply bg-navbar z-50 overflow-hidden border-b border-black/5
           shadow-[0_4px_14px_-4px_rgba(0,0,0,0.08)]
           dark:border-black dark:shadow-none;
  }

  .panel {
    @apply bg-panel z-50 border-t border-black/5
           shadow-[0_-8px_20px_-6px_rgba(0,0,0,0.10)]
           dark:border-black dark:shadow-none;
  }

  .state-bar {
    @apply bg-white/35 z-50 border-t border-black/5 dark:border-black;
  }
</style>
