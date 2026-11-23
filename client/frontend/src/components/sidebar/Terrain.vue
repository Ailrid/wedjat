<template>
  <div
    class="group w-full h-24 flex rounded-md transition-colors"
    :class="[
      tc.resource.show ? 'text-white bg-primary/5' : 'text-white/50 opacity-60',
      'hover:bg-white/5 hover:border-white/10',
    ]"
  >
    <!-- 激活指示条 -->
    <div
      class="w-0.5 rounded-l-md"
      :class="tc.resource.show ? 'bg-primary' : 'bg-transparent'"
    ></div>
    <!-- 拖拽手柄 -->
    <div class="flex flex-col h-full w-8 justify-around items-center">
      <Button variant="icon" @click="SortItemMessage.send(true, resource.id)">
        <ArrowUp />
      </Button>
      <Button variant="icon" @click="SortItemMessage.send(false, resource.id)">
        <ArrowDown />
      </Button>
    </div>

    <div class="flex-1 flex flex-col p-2 justify-around">
      <!-- 第一行：名称 + 编辑 + 激活开关 -->
      <div class="flex items-center gap-2 min-w-0">
        <span
          class="flex-1 font-medium truncate leading-tight"
          :title="tc.resource.name"
        >
          {{ tc.resource.name }}
        </span>
        <Button
          variant="icon"
          @click="((tc.isShowing = !tc.isShowing), tc.onSetup())"
        >
          <Edit class="w-4 h-4" />
        </Button>
        <Button variant="icon" @click.stop="tc.handleToggle()">
          <Eye v-if="tc.resource.show" class="w-4 h-4" />
          <EyeOff v-else class="w-4 h-4 opacity-50" />
        </Button>
      </div>

      <!-- 第二行：图标 + 类型 -->
      <div class="flex items-center gap-1.5 min-w-0 pl-0.5">
        <Mountain class="w-3 h-3 opacity-40" />
        <span
          class="font-mono text-xs uppercase tracking-wider truncate opacity-50"
        >
          {{ tc.resource.type }}
        </span>
        <span
          v-if="tc.resource.show"
          class="font-mono text-xs tracking-wider text-primary"
        >
          · ACTIVE
        </span>
      </div>
    </div>

    <DialogContent
      v-if="tc.isShowing"
      v-model:show="tc.isShowing"
      :height="'auto'"
      :width="'auto'"
    >
      <template #context="{ close }">
        <div class="w-lg min-h-144 max-h-169 flex flex-col">
          <div
            class="flex items-center justify-between gap-2 h-9 px-3 border-b border-white/10"
          >
            <div class="flex items-center gap-1">
              <span class="font-medium tracking-wide opacity-60">
                编辑地形：DEM |
              </span>
              <Button
                @click="(tc.saveChange(), close())"
                class="p-1 rounded transition-colors"
              >
                保存
              </Button>
              <Button
                @click="(tc.removeResource(), close())"
                class="p-1 rounded transition-colors"
              >
                删除
              </Button>
            </div>

            <Button
              @click="close"
              class="p-1 rounded transition-colors hover:bg-white/10"
            >
              <X class="w-4 h-4" />
            </Button>
          </div>

          <div
            v-if="tc.resourceCloned.extra.terrain"
            class="flex-1 p-4 gap-4 flex flex-col overflow-auto"
          >
            <div class="field-row">
              <span class="field-label">Name</span>
              <Input v-model="tc.resourceCloned.name" class="field-input" />
            </div>

            <div class="field-row">
              <span class="field-label">Url</span>
              <Input
                v-model="tc.resourceCloned.url"
                class="field-input"
                placeholder="指向含 layer.json 的地形服务目录"
              />
            </div>

            <div class="field-row">
              <span class="field-label">Credit</span>
              <Input
                v-model="tc.resourceCloned.extra.terrain.credit"
                class="field-input"
                placeholder="版权信息"
              />
            </div>

            <div class="w-full h-0.5 bg-white/10"></div>

            <div class="field-row">
              <span class="field-label">Vertex Normals</span>
              <Switch
                v-model="tc.resourceCloned.extra.terrain.requestVertexNormals"
              />
            </div>

            <div class="field-row">
              <span class="field-label">Water Mask</span>
              <Switch
                v-model="tc.resourceCloned.extra.terrain.requestWaterMask"
              />
            </div>
          </div>
        </div>
      </template>
    </DialogContent>
  </div>
</template>

<script setup lang="ts">
  import { type Resource } from "@/ccs/resource";
  import { useController } from "@virid/vue";
  import { TerrainController } from "./controllers/terrain";
  import {
    Eye,
    EyeOff,
    Edit,
    X,
    Mountain,
    ArrowUp,
    ArrowDown,
  } from "lucide-vue-next";
  import Button from "../ui/Button.vue";
  import DialogContent from "../ui/dialog/DialogContent.vue";
  import Input from "@/components/ui/Input.vue";
  import Switch from "@/components/ui/Switch.vue";
  import { SortItemMessage } from "./controllers";

  const prop = defineProps<{
    resource: Resource;
  }>();

  const tc = useController(TerrainController, {
    context: prop,
  });
</script>

<style scoped>
  @reference "@/assets/main.css";

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
