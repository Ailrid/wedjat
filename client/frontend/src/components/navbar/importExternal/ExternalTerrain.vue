<template>
  <div class="w-full min-h-144 max-h-169 flex text-sm">
    <div class="w-14 h-auto flex flex-col bg-black/20 text-sm pb-4">
      <Button variant="none" class="button active">DEM</Button>
      <div class="flex-1"></div>
      <Button
        variant="icon"
        class="text-white! self-center"
        @click="etc.import()"
      >
        <Import :size="18" />
      </Button>
    </div>

    <div class="flex-1 p-4 gap-4 flex flex-col">
      <div class="field-row">
        <span class="field-label">Name</span>
        <Input
          v-model="etc.name"
          class="field-input"
          required
          empty-message="名称不能为空"
        />
      </div>

      <div class="field-row">
        <span class="field-label">Url</span>
        <Input
          v-model="etc.url"
          class="field-input"
          required
          empty-message="地址不能为空"
          placeholder="指向含 layer.json 的地形服务目录"
        />
      </div>

      <div class="field-row">
        <span class="field-label">Credit</span>
        <Input
          v-model="etc.credit"
          class="field-input"
          placeholder="版权信息"
        />
      </div>

      <div class="w-full h-0.5 bg-white/10"></div>
      <p class="text-white/70">附加配置</p>

      <div class="flex-1 overflow-auto flex flex-col gap-4">
        <div class="field-row">
          <span class="field-label">Vertex Normals</span>
          <Switch v-model="etc.requestVertexNormals" />
        </div>

        <div class="field-row">
          <span class="field-label">Water Mask</span>
          <Switch v-model="etc.requestWaterMask" />
        </div>

        <div class="field-row">
          <span class="field-label">自动加载</span>
          <Switch v-model="etc.autoLoad" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { ExternalTerrainController } from "./controllers";
  import Button from "@/components/ui/Button.vue";
  import { Import } from "lucide-vue-next";
  import Input from "@/components/ui/Input.vue";
  import Switch from "@/components/ui/Switch.vue";

  const etc = useController(ExternalTerrainController);
</script>

<style scoped>
  @reference "@/assets/main.css";

  .active {
    @apply text-white!;
    @apply shadow-[inset_2px_0_0_0_#fff];
  }

  .button {
    @apply w-14 h-14 flex items-center justify-center hover:text-white cursor-pointer text-white/50 text-xs;
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
