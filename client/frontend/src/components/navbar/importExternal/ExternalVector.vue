<template>
  <div class="w-full min-h-144 max-h-169 flex text-sm">
    <div class="w-14 h-auto flex flex-col bg-black/20 text-sm pb-4">
      <Button variant="none" class="button active">Vector</Button>
      <div class="flex-1"></div>
      <Button
        variant="icon"
        class="text-white! self-center"
        @click="vc.import()"
      >
        <Import :size="18" />
      </Button>
    </div>

    <div class="flex-1 p-4 gap-4 flex flex-col overflow-auto">
      <input
        ref="fileInput"
        type="file"
        accept=".geojson,.json,.kml,.kmz"
        class="hidden"
        @change="vc.handleFileSelect"
      />

      <div class="field-row">
        <span class="field-label">Name</span>
        <Input v-model="vc.name" class="field-input" required />
      </div>

      <div class="field-row">
        <span class="field-label">File</span>
        <button
          type="button"
          @click="vc.openFileDialog()"
          class="flex-1 mr-2 ml-2 px-2 py-1 border border-foreground/10 rounded text-left truncate outline-none focus:ring-1 focus:ring-primary focus:border-primary hover:bg-white/5"
        >
          <span :class="vc.fileName ? '' : 'opacity-50'">
            {{ vc.fileName || "选择 .geojson / .kml / .kmz 文件..." }}
          </span>
        </button>
      </div>

      <div class="field-row">
        <span class="field-label">Format</span>
        <span class="flex-1 mr-2 ml-2 opacity-60 font-mono">
          {{ vc.format }}
        </span>
      </div>

      <div class="field-row">
        <span class="field-label">Clamp To Ground</span>
        <Switch v-model="vc.clampToGround" />
      </div>
      <p class="text-white/70">点样式</p>
      <div class="w-full h-0.5 bg-white/10"></div>
      <div class="flex flex-col gap-4">
        <div class="field-row">
          <span class="field-label">Color</span>

          <div class="flex-1 mr-2 ml-2">
            <ColorPicker v-model="vc.pointColor" />
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">Size</span>
          <Input v-model="vc.pointSize" type="number" class="field-input" />
        </div>

        <div class="field-row">
          <span class="field-label">Outline Color</span>
          <div class="flex-1 mr-2 ml-2">
            <ColorPicker v-model="vc.pointOutlineColor" />
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">Outline Width</span>
          <Input
            v-model="vc.pointOutlineWidth"
            type="number"
            class="field-input"
          />
        </div>
      </div>

      <div class="w-full h-0.5 bg-white/10"></div>
      <p class="text-white/70">线样式</p>
      <div class="w-full h-0.5 bg-white/10"></div>
      <div class="flex flex-col gap-4">
        <div class="field-row">
          <span class="field-label">Color</span>

          <div class="flex-1 mr-2 ml-2">
            <ColorPicker v-model="vc.polylineColor" />
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">Width</span>
          <Input v-model="vc.polylineWidth" type="number" class="field-input" />
        </div>
      </div>

      <div class="w-full h-0.5 bg-white/10"></div>
      <p class="text-white/70">面样式</p>
      <div class="w-full h-0.5 bg-white/10"></div>
      <div class="flex flex-col gap-4">
        <div class="field-row">
          <span class="field-label">Fill Color</span>

          <div class="flex-1 mr-2 ml-2">
            <ColorPicker v-model="vc.polygonColor" />
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">Outline</span>
          <Switch v-model="vc.polygonOutline" />
        </div>

        <div class="field-row">
          <span class="field-label">Outline Color</span>

          <div class="flex-1 mr-2 ml-2">
            <ColorPicker v-model="vc.polygonOutlineColor" />
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">Outline Width</span>
          <Input
            v-model="vc.polygonOutlineWidth"
            type="number"
            class="field-input"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { ExternalVectorController } from "./controllers";
  import Button from "@/components/ui/Button.vue";
  import { Import } from "lucide-vue-next";
  import Input from "@/components/ui/Input.vue";
  import Switch from "@/components/ui/Switch.vue";
  import ColorPicker from "@/components/ui/ColorPicker.vue";

  const vc = useController(ExternalVectorController);
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
