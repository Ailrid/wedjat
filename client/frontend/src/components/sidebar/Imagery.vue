<template>
  <div
    class="group w-full flex rounded-md border border-transparent select-none transition-colors"
    :class="[
      imc.resourceCloned.show
        ? 'text-black dark:text-white'
        : 'text-black/50 dark:text-white/40 opacity-60',
      'hover:bg-black/5 dark:hover:bg-white/5 hover:border-black/10 dark:hover:border-white/10',
    ]"
  >
    <!-- 拖拽手柄 -->
    <div
      class="w-8 shrink-0 flex items-center justify-center cursor-grab text-black/20 dark:text-white/20 group-hover:text-black/50 dark:group-hover:text-white/50 transition-colors"
    >
      <GripVertical class="w-4 h-4" />
    </div>

    <div class="flex-1 min-w-0 flex flex-col gap-1 py-2 pr-2">
      <!-- 第一行：名称 + 显隐按钮 -->
      <div class="flex items-center gap-2 min-w-0">
        <span
          class="flex-1 font-medium truncate leading-tight"
          :title="imc.resourceCloned.name"
        >
          {{ imc.resourceCloned.name || "未命名影像" }}
        </span>
        <button
          class="shrink-0 p-1 rounded transition-colors hover:bg-black/10 dark:hover:bg-white/10 text-black/60 dark:text-white/60 hover:text-black dark:hover:text-white"
          @click.stop="imc.handleShowToggle"
        >
          <Eye v-if="imc.resource.show" class="w-4 h-4" />
          <EyeOff v-else class="w-4 h-4 opacity-50" />
        </button>
      </div>

      <!-- 第二行：图标 + 类型 + 滑块（滑块平时不占位，hover 展开） -->
      <div class="flex items-center gap-1.5 min-w-0 pl-0.5">
        <Image class="w-3 h-3 shrink-0 opacity-40" />
        <span
          class="font-mono text-xs uppercase tracking-wider truncate opacity-50 min-w-0"
        >
          {{ imc.resourceCloned.type }}
        </span>

        <!-- pr-4 给 thumb 右侧留缓冲，避免拖到最右端时寄出容器 -->
        <div
          class="flex-none w-0 pr-4 overflow-hidden opacity-0 group-hover:flex-1 group-hover:opacity-100"
        >
          <Slider
            v-model="imc.alpha"
            :min="0"
            :max="1"
            :step="0.01"
            class="w-full"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { type Resource } from "@/ccs/cesium";
  import { useController } from "@virid/vue";
  import { ImageryController } from "./controllers/imagery";
  import Slider from "@/components/ui/Slider.vue";
  import { Image, Eye, EyeOff, GripVertical } from "lucide-vue-next";

  const prop = defineProps<{
    resource: Resource;
  }>();
  const imc = useController(ImageryController, {
    context: prop,
  });
</script>
