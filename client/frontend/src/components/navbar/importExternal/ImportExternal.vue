<template>
  <div>
    <DialogContent
      v-if="ict.isShowing"
      v-model:show="ict.isShowing"
      :height="'auto'"
      :width="'auto'"
    >
      <template #context="{ close }">
        <div class="w-lg flex flex-col">
          <div
            class="flex items-center justify-between gap-2 h-9 px-3 border-b border-white/10"
          >
            <span class="font-medium tracking-wide opacity-60">
              导入外部资源
            </span>
            <Button
              @click="close"
              class="p-1 rounded transition-colors hover:bg-white/10"
              title="关闭"
            >
              <X class="w-4 h-4" />
            </Button>
          </div>

          <div>
            <ExternalImagery v-if="ict.type == 'imagery'"></ExternalImagery>
            <ExternalTerrain
              v-else-if="ict.type == 'terrain'"
            ></ExternalTerrain>
            <ExternalVector v-else-if="ict.type == 'vector'"></ExternalVector>
            <ExternalTiles v-else-if="ict.type == 'tiles'"> </ExternalTiles>
          </div>
        </div>
      </template>
    </DialogContent>
  </div>
</template>

<script setup lang="ts">
  import DialogContent from "@/components/ui/dialog/DialogContent.vue";
  import { useController } from "@virid/vue";
  import { ImportExternalController } from "./controllers";
  import ExternalImagery from "./ExternalImagery.vue";
  import ExternalTerrain from "./ExternalTerrain.vue";
  import ExternalVector from "./ExternalVector.vue";
  import ExternalTiles from "./ExternalTiles.vue";
  import { X } from "lucide-vue-next";
  import Button from "@/components/ui/Button.vue";

  const ict = useController(ImportExternalController);
</script>

<style scoped></style>
