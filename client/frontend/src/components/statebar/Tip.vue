<template>
  <Dialog>
    <template #trigger="{ open }">
      <button
        @click="open"
        class="h-full flex items-center justify-center hover:bg-black text-white transition-colors px-2 cursor-pointer"
      >
        <span class="leading-none tabular-nums">
          {{ stc.tip }}
        </span>
      </button>
    </template>

    <template #context="{ close }">
      <div class="w-lg h-128 flex flex-col">
        <div
          class="shrink-0 flex items-center justify-between gap-2 h-9 px-3 border-b border-white/10"
        >
          <span class="text-xs font-medium tracking-wide opacity-60">
            历史记录
            <span class="ml-1 tabular-nums">{{ stc.tips.length }}</span>
          </span>
          <Button
            @click="close"
            class="p-1 rounded transition-colors hover:bg-white/10"
            title="关闭"
          >
            <X class="w-4 h-4" />
          </Button>
        </div>

        <div class="overflow-auto py-1">
          <div
            v-for="(tip, index) in stc.tips"
            :key="index"
            class="flex w-full items-center gap-2 px-3 py-1.5 leading-snug tabular-nums hover:bg-white/5"
            :style="stc.tipColor(tip)"
          >
            <span class="w-5 h-full text-xs opacity-40 text-center">
              {{ index + 1 }}
            </span>
            <span>
              {{ tip }}
            </span>
          </div>
        </div>
      </div>
    </template>
  </Dialog>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { TipController } from "./controllers";
  import Dialog from "../ui/dialog/Dialog.vue";
  import Button from "../ui/Button.vue";
  import { X } from "lucide-vue-next";

  const stc = useController(TipController);
</script>
