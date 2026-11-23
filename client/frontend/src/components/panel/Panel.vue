<template>
  <div class="w-full h-full flex flex-col text-[13px]">
    <!-- 顶部栏：左侧 tab，右侧连接状态 -->
    <div class="w-full h-8 shrink-0 flex items-stretch pr-2 pl-2">
      <div class="h-full flex items-stretch">
        <Button
          variant="none"
          v-for="tab in tabs"
          :key="tab.key"
          class="relative h-full px-3 flex items-center transition-colors cursor-pointer"
          :class="
            panel.tab === tab.key
              ? 'text-white'
              : 'text-white/50 hover:text-white/80'
          "
          @click="panel.tab = tab.key"
        >
          <span>{{ tab.label }}</span>
          <span
            v-if="panel.tab === tab.key"
            class="absolute left-0 right-0 bottom-0 h-0.5 bg-primary"
          ></span>
        </Button>
      </div>
      <div class="ml-auto flex items-center gap-2 p-1 pr-2 pl-2">
        <span class="w-3 h-3 bg-red-800 rounded-full inline-block"></span>
        <label class="text-white/70">尚未连接</label>
      </div>
    </div>

    <!-- 当前 tab 内容 -->
    <div class="flex-1 min-h-0 overflow-hidden">
      <component :is="componentMap[panel.tab]"></component>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import type { Component } from "vue";
  import { PanelController, type PanelTab } from "./controllers";
  import Locate from "./Locate.vue";
  import Vision from "./Vision.vue";
  import Inertial from "./Inertial.vue";
  import Setting from "./Setting.vue";
  import Button from "../ui/Button.vue";

  const panel = useController(PanelController);

  const tabs: { key: PanelTab; label: string }[] = [
    { key: "locate", label: "定位状态" },
    { key: "vision", label: "视觉定位" },
    { key: "inertial", label: "惯性导航" },
    { key: "setting", label: "系统设置" },
  ];

  const componentMap: Record<PanelTab, Component> = {
    locate: Locate,
    vision: Vision,
    inertial: Inertial,
    setting: Setting,
  };
</script>

<style scoped></style>
