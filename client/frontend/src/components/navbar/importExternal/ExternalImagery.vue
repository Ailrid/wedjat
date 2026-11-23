<template>
  <div class="w-full min-h-144 max-h-169 flex text-sm">
    <div class="w-14 h-auto flex flex-col bg-black/20 pb-4">
      <Button
        variant="none"
        class="button"
        :class="{ active: ect.tab == 'xyz' }"
        @click="ect.tab = 'xyz'"
      >
        XYZ
      </Button>
      <Button
        variant="none"
        class="button"
        :class="{ active: ect.tab == 'wms' }"
        @click="ect.tab = 'wms'"
      >
        WMS
      </Button>
      <Button
        variant="none"
        class="button"
        :class="{ active: ect.tab == 'wmts' }"
        @click="ect.tab = 'wmts'"
      >
        WMTS
      </Button>
      <div class="flex-1"></div>
      <Button
        variant="icon"
        class="text-white! self-center"
        @click="ect.import()"
      >
        <Import :size="18" />
      </Button>
    </div>

    <div class="flex-1 p-4 gap-4 flex flex-col overflow-auto">
      <div class="field-row">
        <span class="field-label">Name</span>
        <Input v-model="ect.name" class="field-input" required />
      </div>

      <div class="field-row">
        <span class="field-label">Url</span>
        <Input v-model="ect.url" class="field-input" required />
      </div>

      <!-- 通用附加项 -->
      <div class="field-row">
        <span class="field-label">Min Level</span>
        <Input v-model="ect.minLevel" type="number" class="field-input" />
      </div>

      <div class="field-row">
        <span class="field-label">Max Level</span>
        <Input v-model="ect.maxLevel" type="number" class="field-input" />
      </div>

      <div class="field-row">
        <span class="field-label">Tiling Scheme</span>
        <Select
          v-model="ect.tilingScheme"
          :options="[
            { label: 'WebMercator', value: 'WebMercator' },
            { label: 'Geographic', value: 'Geographic' },
          ]"
        />
      </div>

      <div class="field-row">
        <span class="field-label">Credit</span>
        <Input
          v-model="ect.credit"
          class="field-input"
          placeholder="版权信息"
        />
      </div>
      <div class="field-row">
        <span class="field-label">自动加载</span>
        <Switch v-model="ect.autoLoad" />
      </div>

      <div class="w-full h-0.5 bg-white/10"></div>

      <!-- XYZ 专属 -->
      <div v-if="ect.tab == 'xyz'" class="flex flex-col gap-4">
        <div class="field-row">
          <span class="field-label">Subdomains</span>
          <Input
            v-model="ect.xyz.subdomains"
            class="field-input"
            placeholder="0,1,2,3,4,5,6,7"
          />
        </div>
      </div>

      <!-- WMS 专属 -->
      <div v-else-if="ect.tab == 'wms'" class="flex flex-col gap-4">
        <div class="field-row">
          <span class="field-label">Layers</span>
          <Input
            v-model="ect.wms.layers"
            class="field-input"
            placeholder="layer1,layer2"
          />
        </div>

        <div class="field-row">
          <span class="field-label">Version</span>
          <Select
            v-model="ect.wms.version"
            :options="[
              { label: '1.1.1', value: '1.1.1' },
              { label: '1.3.0', value: '1.3.0' },
            ]"
          />
        </div>

        <div class="field-row">
          <span class="field-label">Format</span>
          <Input
            v-model="ect.wms.format"
            class="field-input"
            placeholder="image/png"
            required
          />
        </div>

        <div class="field-row">
          <span class="field-label">Pick Features</span>
          <input
            type="checkbox"
            v-model="ect.wms.enablePickFeatures"
            class="ml-2 accent-white"
          />
        </div>
      </div>

      <!-- WMTS 专属 -->
      <div v-else-if="ect.tab == 'wmts'" class="flex flex-col gap-4">
        <div class="field-row">
          <span class="field-label">Projection</span>
          <Select
            v-model="ect.wmts.projection"
            :options="[
              { label: '经纬度 (c)', value: 'c' },
              { label: '球面墨卡托 (w)', value: 'w' },
            ]"
          />
        </div>

        <div class="field-row">
          <span class="field-label">Layer</span>
          <Input v-model="ect.wmts.layer" class="field-input" required />
        </div>

        <div class="field-row">
          <span class="field-label">Style</span>
          <Input
            v-model="ect.wmts.style"
            class="field-input"
            placeholder="default"
            required
          />
        </div>

        <div class="field-row">
          <span class="field-label">Matrix Set</span>
          <Input
            v-model="ect.wmts.matrixSet"
            class="field-input"
            placeholder="留空则跟随 Projection"
          />
        </div>

        <div class="field-row">
          <span class="field-label">Format</span>
          <Input
            v-model="ect.wmts.format"
            class="field-input"
            placeholder="tiles"
            required
          />
        </div>

        <div class="field-row">
          <span class="field-label">Subdomains</span>
          <Input
            v-model="ect.wmts.subdomains"
            class="field-input"
            placeholder="0,1,2,3,4,5,6,7"
            required
            :validator="validateSubdomains"
          />
        </div>

        <div class="field-row">
          <span class="field-label">Matrix Labels</span>
          <Input
            v-model="ect.wmts.matrixLabels"
            class="field-input"
            placeholder="0,1,2..."
            :validator="validateSubdomains"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { ExternalImageryController } from "./controllers";
  import Button from "@/components/ui/Button.vue";
  import { Import } from "lucide-vue-next";
  import Input from "@/components/ui/Input.vue";
  import Select from "@/components/ui/Select.vue";
  import Switch from "@/components/ui/Switch.vue";

  const ect = useController(ExternalImageryController);
  const validateSubdomains = (value: string | number) => {
    const items = String(value).split(",");

    if (items.some((item) => item.trim() === "")) {
      return "存在空项，请检查逗号分隔";
    }

    if (items.some((item) => /\s/.test(item.trim()))) {
      return "子域不能包含空格";
    }

    return true;
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

  .field-row {
    @apply w-full flex items-center justify-between;
  }

  .field-label {
    @apply w-28  text-white/70;
  }

  .field-input {
    @apply flex-1 mr-2 ml-2;
  }
</style>
