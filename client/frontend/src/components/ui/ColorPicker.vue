<template>
  <div>
    <slot name="trigger" :open="open" :current="modelValue">
      <button
        type="button"
        @click="open"
        class="w-full flex items-center gap-2 border border-foreground/10 rounded px-2 py-1 outline-none hover:border-foreground/30 focus:ring-1 focus:ring-primary focus:border-primary transition-colors"
      >
        <span
          class="w-4 h-4 rounded border border-foreground/20"
          :style="{ backgroundColor: modelValue }"
        />
        <span class="font-mono text-sm truncate">{{ modelValue }}</span>
      </button>
    </slot>

    <DialogContent
      v-if="isShowing"
      v-model:show="isShowing"
      :height="'auto'"
      :width="'auto'"
    >
      <template #context="{ close }">
        <div class="w-80 flex flex-col">
          <div
            class="flex items-center justify-between h-9 px-3 border-b border-white/10"
          >
            <span class="font-medium tracking-wide opacity-60">选择颜色</span>
            <Button
              @click="close"
              class="p-1 rounded transition-colors hover:bg-white/10"
            >
              <X class="w-4 h-4" />
            </Button>
          </div>

          <div class="p-4 flex flex-col gap-4">
            <!-- 色板：8 列 × 4 行 -->
            <div class="grid grid-cols-8 gap-1.5">
              <button
                v-for="(hex, name) in paletteHex"
                :key="name"
                type="button"
                :title="name"
                :style="{ backgroundColor: hex }"
                class="aspect-square rounded border border-foreground/10 transition-all hover:scale-110 hover:border-foreground/40"
                :class="
                  draft.toLowerCase() === hex
                    ? 'ring-2 ring-primary ring-offset-1 ring-offset-background'
                    : ''
                "
                @click="draft = hex"
              />
            </div>

            <div class="w-full h-0.5 bg-white/10"></div>

            <!-- Hex 输入 -->
            <div class="w-full flex items-center">
              <span class="w-16 text-white/70">Hex</span>
              <Input
                v-model="draft"
                class="flex-1 mr-2 ml-2"
                placeholder="#ffffff"
                :validator="validateHex"
              />
            </div>

            <!-- 预览 -->
            <div class="w-full flex items-center">
              <span class="w-16 text-white/70">预览</span>
              <div
                class="flex-1 mr-2 ml-2 h-7 rounded border border-foreground/10"
                :style="{ backgroundColor: draft }"
              />
            </div>
          </div>

          <div class="flex justify-end gap-2 p-3 border-t border-white/10">
            <Button @click="close" class="px-3 py-1 rounded hover:bg-white/10">
              取消
            </Button>
            <Button
              @click="confirm(close)"
              :disabled="!isValid"
              class="px-3 py-1 rounded bg-primary/20 text-primary hover:bg-primary/30 disabled:opacity-40 disabled:cursor-not-allowed"
            >
              确定
            </Button>
          </div>
        </div>
      </template>
    </DialogContent>
  </div>
</template>

<script setup lang="ts">
  import { computed, ref } from "vue";
  import DialogContent from "@/components/ui/dialog/DialogContent.vue";
  import Button from "@/components/ui/Button.vue";
  import Input from "@/components/ui/Input.vue";
  import { X } from "lucide-vue-next";

  const props = defineProps<{
    modelValue: string;
  }>();

  const emit = defineEmits<{
    (e: "update:modelValue", value: string): void;
  }>();

  const isShowing = ref(false);
  const draft = ref(props.modelValue);

  const open = () => {
    draft.value = props.modelValue;
    isShowing.value = true;
  };

  const confirm = (close: () => void) => {
    if (!isValid.value) return;
    emit("update:modelValue", draft.value);
    close();
  };

  /** 调色板：[r, g, b]，8 列 × 4 行 = 32 色 */
  const palette: Record<string, [number, number, number]> = {
    // --- 第 1 行：中性色 + 高频色 ---
    white: [255, 255, 255],
    lightGray: [212, 212, 216],
    neutral: [115, 115, 115],
    darkGray: [63, 63, 70],
    black: [0, 0, 0],
    red: [239, 68, 68],
    roseRed: [225, 29, 72],
    pink: [244, 114, 182],
    // --- 第 2 行：紫 → 青 ---
    fuchsia: [192, 38, 211],
    purple: [147, 51, 234],
    indigo: [79, 70, 229],
    blue: [37, 99, 235],
    sky: [14, 165, 233],
    cyan: [6, 182, 212],
    teal: [13, 148, 136],
    emerald: [16, 185, 129],
    // --- 第 3 行：绿 → 暖色 ---
    green: [34, 197, 94],
    lime: [101, 163, 13],
    yellow: [234, 179, 8],
    amber: [245, 158, 11],
    orange: [249, 115, 22],
    warmOrange: [234, 88, 12],
    deepOrange: [194, 65, 12],
    bloodRed: [153, 27, 27],
    // --- 第 4 行：深色变体 ---
    darkRed: [127, 29, 29],
    darkOrange: [124, 45, 18],
    darkYellow: [113, 63, 18],
    darkGreen: [20, 83, 45],
    darkTeal: [19, 78, 74],
    darkBlue: [30, 58, 138],
    darkPurple: [76, 29, 149],
    darkPink: [131, 24, 67],
  };

  function rgbToHex([r, g, b]: [number, number, number]): string {
    return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
  }

  const paletteHex = computed(() =>
    Object.fromEntries(
      Object.entries(palette).map(([name, rgb]) => [name, rgbToHex(rgb)]),
    ),
  );

  const validateHex = (value: string | number) => {
    const v = String(value).trim();
    if (!v) return "颜色值不能为空";
    if (!/^#[0-9a-fA-F]{6}$/.test(v)) {
      return "格式应为 #rrggbb";
    }
    return true;
  };

  const isValid = computed(() => {
    return /^#[0-9a-fA-F]{6}$/.test(draft.value.trim());
  });
</script>
