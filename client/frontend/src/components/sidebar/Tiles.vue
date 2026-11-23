<template>
  <div
    class="group w-full h-24 flex rounded-md"
    :class="[
      tic.resource.show ? 'text-white' : 'text-white/50 opacity-60',
      'hover:bg-white/5 hover:border-white/10',
    ]"
  >
    <!-- 拖拽手柄占位（与 Imagery 一致的排版） -->
    <div class="flex flex-col h-full w-8 justify-around items-center">
      <Button variant="icon" @click="SortItemMessage.send(true, resource.id)">
        <ArrowUp />
      </Button>
      <Button variant="icon" @click="SortItemMessage.send(false, resource.id)">
        <ArrowDown />
      </Button>
    </div>

    <div class="flex-1 flex flex-col p-2 justify-around">
      <!-- 第一行：名称 + 操作 -->
      <div class="flex items-center gap-2 min-w-0">
        <span
          class="flex-1 font-medium truncate leading-tight"
          :title="tic.resource.name"
        >
          {{ tic.resource.name }}
        </span>
        <Button variant="icon">
          <Locate
            class="w-4 h-4"
            @click="LocateResourceMessage.send(resource.id)"
          />
        </Button>
        <Button
          variant="icon"
          @click="((tic.isShowing = !tic.isShowing), tic.onSetup())"
        >
          <Edit class="w-4 h-4" />
        </Button>
        <Button variant="icon" @click.stop="tic.handleShowToggle()">
          <Eye v-if="tic.resource.show" class="w-4 h-4" />
          <EyeOff v-else class="w-4 h-4 opacity-50" />
        </Button>
      </div>

      <!-- 第二行：图标 + 类型 -->
      <div class="flex items-center gap-1.5 min-w-0 pl-0.5">
        <Box v-if="tic.osgb" class="w-3 h-3 shrink-0 opacity-40" />
        <Cloud v-else class="w-3 h-3 shrink-0 opacity-40" />
        <span
          class="font-mono text-xs uppercase tracking-wider truncate opacity-50 min-w-0"
        >
          {{ tic.osgb ? "osgb" : "pointcloud" }}
        </span>
      </div>
    </div>

    <DialogContent
      v-if="tic.isShowing"
      v-model:show="tic.isShowing"
      :height="'auto'"
      :width="'auto'"
    >
      <template #context="{ close }">
        <div class="w-lg min-h-144 max-h-169 flex flex-col">
          <!-- 头部 -->
          <div
            class="shrink-0 flex items-center justify-between gap-2 h-9 px-3 border-b border-white/10"
          >
            <div>
              <span class="font-medium tracking-wide opacity-60">
                编辑模型：{{ tic.osgb ? "OSGB" : "点云" }} |
              </span>
              <Button
                @click="(tic.saveChange(), close())"
                class="p-1 rounded transition-colors"
              >
                保存
              </Button>
              <Button
                @click="(tic.removeResource(), close())"
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

          <div class="flex-1 p-4 gap-4 flex flex-col overflow-auto">
            <!-- 公共字段 -->
            <div class="field-row">
              <span class="field-label">Name</span>
              <Input v-model="tic.resourceCloned.name" class="field-input" />
            </div>
            <div class="field-row">
              <span class="field-label">Url</span>
              <Input v-model="tic.resourceCloned.url" class="field-input" />
            </div>
            <div class="field-row">
              <span class="field-label">SSE</span>
              <Input
                v-model.number="
                  tic.resourceCloned.extra.tiles!.maximumScreenSpaceError
                "
                type="number"
                class="field-input"
              />
            </div>
            <div class="field-row">
              <span class="field-label">Dynamic SSE</span>
              <input
                type="checkbox"
                v-model="
                  tic.resourceCloned.extra.tiles!.dynamicScreenSpaceError
                "
                class="ml-2 accent-white"
              />
            </div>
            <div class="field-row">
              <span class="field-label">Credit</span>
              <Input
                v-model="tic.resourceCloned.extra.tiles!.credit"
                class="field-input"
                placeholder="版权信息"
              />
            </div>

            <div class="w-full h-0.5 bg-white/10"></div>

            <!-- ============ OSGB ============ -->
            <template v-if="tic.osgb">
              <div class="field-row">
                <span class="field-label">Shadows</span>
                <Select
                  v-model="tic.osgb.shadows"
                  :options="[
                    { label: 'ENABLED', value: 'ENABLED' },
                    { label: 'DISABLED', value: 'DISABLED' },
                    { label: 'RECEIVE_ONLY', value: 'RECEIVE_ONLY' },
                    { label: 'CAST_ONLY', value: 'CAST_ONLY' },
                  ]"
                />
              </div>
              <div class="field-row">
                <span class="field-label">Max Memory (MB)</span>
                <Input
                  v-model.number="tic.osgb.maximumMemoryUsage"
                  type="number"
                  class="field-input"
                />
              </div>
            </template>

            <!-- ============ 点云 ============ -->
            <template v-else-if="tic.pcd">
              <div class="field-row">
                <span class="field-label">Style Mode</span>
                <Select
                  v-model="tic.pcd.styleMode"
                  :options="[
                    { label: 'Original', value: 'original' },
                    { label: 'Uniform', value: 'uniform' },
                    { label: 'Classification', value: 'classification' },
                    { label: 'Elevation', value: 'elevation' },
                  ]"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Attenuation</span>
                <input
                  type="checkbox"
                  v-model="tic.pcd.attenuation"
                  class="ml-2 accent-white"
                />
              </div>
              <div class="field-row">
                <span class="field-label">Eye Dome Lighting</span>
                <input
                  type="checkbox"
                  v-model="tic.pcd.eyeDomeLighting"
                  class="ml-2 accent-white"
                />
              </div>

              <!-- uniform -->
              <div v-if="tic.pcd.styleMode === 'uniform'" class="field-row">
                <span class="field-label">Uniform Color</span>
                <input
                  type="color"
                  v-model="tic.pcd.uniformColor"
                  class="w-10 h-7 bg-transparent border border-white/20 rounded"
                />
              </div>

              <!-- classification -->
              <template v-if="tic.pcd.styleMode === 'classification'">
                <div class="flex items-center justify-between">
                  <span class="text-white/70">分类规则</span>
                  <Button @click="tic.addClassificationRule()" class="text-xs">
                    + 添加
                  </Button>
                </div>
                <div
                  v-for="(rule, i) in tic.pcd.classificationRules"
                  :key="i"
                  class="flex items-center gap-2 pl-0.5"
                >
                  <Input
                    v-model.number="rule.classification"
                    type="number"
                    class="w-20"
                    placeholder="分类号"
                  />
                  <input
                    type="color"
                    v-model="rule.color"
                    class="w-10 h-7 bg-transparent border border-white/20 rounded"
                  />
                  <label class="flex items-center gap-1 text-xs opacity-70">
                    <input
                      type="checkbox"
                      v-model="rule.visible"
                      class="accent-white"
                    />
                    显示
                  </label>
                  <Button
                    @click="tic.removeClassificationRule(i)"
                    class="ml-auto text-xs opacity-60 hover:opacity-100"
                  >
                    删除
                  </Button>
                </div>
              </template>

              <!-- elevation -->
              <template v-if="tic.pcd.styleMode === 'elevation'">
                <div class="flex items-center justify-between">
                  <span class="text-white/70">高程分段</span>
                  <Button @click="tic.addElevationStop()" class="text-xs">
                    + 添加
                  </Button>
                </div>
                <div
                  v-for="(stop, i) in tic.pcd.elevationStops"
                  :key="i"
                  class="flex items-center gap-2 pl-0.5"
                >
                  <Input
                    v-model.number="stop.elevation"
                    type="number"
                    class="w-24"
                    placeholder="高程(m)"
                  />
                  <input
                    type="color"
                    v-model="stop.color"
                    class="w-10 h-7 bg-transparent border border-white/20 rounded"
                  />
                  <Button
                    @click="tic.removeElevationStop(i)"
                    class="ml-auto text-xs opacity-60 hover:opacity-100"
                  >
                    删除
                  </Button>
                </div>
              </template>
            </template>
          </div>
        </div>
      </template>
    </DialogContent>
  </div>
</template>

<script setup lang="ts">
  import { LocateResourceMessage, type Resource } from "@/ccs/resource";
  import { useController } from "@virid/vue";
  import { TilesController } from "./controllers/tiles";
  import {
    Eye,
    EyeOff,
    ArrowUp,
    ArrowDown,
    Edit,
    X,
    Box,
    Cloud,
    Locate,
  } from "lucide-vue-next";
  import Button from "../ui/Button.vue";
  import { SortItemMessage } from "./controllers";
  import DialogContent from "../ui/dialog/DialogContent.vue";
  import Input from "@/components/ui/Input.vue";
  import Select from "@/components/ui/Select.vue";

  const prop = defineProps<{
    resource: Resource;
  }>();
  const tic = useController(TilesController, {
    context: prop,
  });
</script>

<style scoped>
  @reference "@/assets/main.css";

  .field-row {
    @apply w-full flex items-center justify-between;
  }

  .field-label {
    @apply w-28 shrink-0 text-white/70;
  }

  .field-input {
    @apply flex-1 mr-2 ml-2;
  }
</style>
