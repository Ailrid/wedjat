<template>
  <div>
    <DropdownMenu>
      <template #trigger="{ toggle, isOpen }" as-child>
        <Button @click="toggle">导入(I)</Button>
      </template>
      <template #default>
        <div
          class="flex flex-col gap-0.5 min-w-74 p-2 pl-4 pr-4 rounded-sm text-sm text-white"
        >
          <Button class="w-full text-start">导入内部影像</Button>
          <Button class="w-full text-start">导入内部DEM</Button>
          <Button class="w-full text-start">导入内部图层</Button>
          <Button class="w-full text-start">导入内部Tiles</Button>
          <div class="w-full h-0.5 bg-white/10"></div>
          <Button
            class="w-full"
            @click="OpenExternalDialogMessage.send('imagery')"
            ><div class="w-full flex justify-between">
              <span>导入外部影象</span>
              <span class="text-white/50">XYZ / WMS / WMTS</span>
            </div>
          </Button>
          <Button
            class="w-full"
            @click="OpenExternalDialogMessage.send('terrain')"
          >
            <div class="w-full flex justify-between">
              <span>导入外部DEM</span>
              <span class="text-white/50">Cesium Terrain</span>
            </div></Button
          >
          <Button
            class="w-full"
            @click="OpenExternalDialogMessage.send('vector')"
          >
            <div class="w-full flex justify-between">
              <span>导入外部矢量</span>
              <span class="text-white/50">GeoJSON / KML</span>
            </div></Button
          >
          <Button
            class="w-full"
            @click="OpenExternalDialogMessage.send('tiles')"
          >
            <div class="w-full flex justify-between">
              <span>导入外部Tiles</span>
              <span class="text-white/50">tileset.json</span>
            </div></Button
          >
        </div>
      </template>
    </DropdownMenu>
    <ImportExternal></ImportExternal>
    <ImportInternal></ImportInternal>
  </div>
</template>

<script setup lang="ts">
  import Button from "@/components/ui/Button.vue";
  import DropdownMenu from "@/components/ui/DropdownMenu.vue";
  import ImportExternal from "./importExternal/ImportExternal.vue";
  import ImportInternal from "./importInternal/ImportInternal.vue";
  import { OpenExternalDialogMessage } from "./importExternal/controllers/importExternal.ts";
</script>

<style scoped></style>
