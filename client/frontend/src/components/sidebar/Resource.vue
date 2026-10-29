<template>
  <div
    ref="containerRef"
    class="w-full h-full overflow-auto"
    @dragover.prevent="rec.onDragOver($event)"
    @dragend="rec.onDragEnd($event)"
  >
    <ul class="p-2">
      <li
        v-for="(item, index) in rec.resources"
        :key="item.name"
        draggable="true"
        @dragstart="rec.onDragStart(index, $event)"
        class="mb-1"
      >
        <Imagery v-if="item.type === 'imagery'" :resource="item" />
        <Terrain v-else-if="item.type === 'terrain'" :resource="item" />
        <Vector v-else-if="item.type === 'vector'" :resource="item" />
        <Tiles v-else-if="item.type === 'tiles'" :resource="item" />
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { ResourceController } from "./controllers";
  import Imagery from "./Imagery.vue";
  import Terrain from "./Terrain.vue";
  import Vector from "./Vector.vue";
  import Tiles from "./Tiles.vue";

  const rec = useController(ResourceController);
</script>
