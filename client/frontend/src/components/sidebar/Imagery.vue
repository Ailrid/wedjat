<template>
  <div
    class="group w-full h-24 flex rounded-md"
    :class="[
      imc.resource.show ? 'text-white ' : 'text-white/50  opacity-60',
      'hover:bg-white/5 hover:border-white/10',
    ]"
  >
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
      <!-- 第一行：名称 + 显隐按钮 -->
      <div class="flex items-center gap-2 min-w-0">
        <span
          class="flex-1 font-medium truncate leading-tight"
          :title="imc.resource.name"
        >
          {{ imc.resource.name }}
        </span>
        <Button variant="icon" v-if="imc.resource.extra.imagery?.type == 'xyz'">
          <Locate
            class="w-4 h-4"
            @click="LocateResourceMessage.send(resource.id)"
          />
        </Button>
        <Button
          variant="icon"
          @click="((imc.isShowing = !imc.isShowing), imc.onSetup())"
        >
          <Edit class="w-4 h-4" />
        </Button>
        <Button variant="icon" @click.stop="imc.handleShowToggle()">
          <Eye v-if="imc.resource.show" class="w-4 h-4" />
          <EyeOff v-else class="w-4 h-4 opacity-50" />
        </Button>
      </div>

      <!-- 第二行：图标 + 类型 + 滑块（滑块平时不占位，hover 展开） -->
      <div class="flex items-center gap-1.5 min-w-0 pl-0.5">
        <Image class="w-3 h-3 shrink-0 opacity-40" />
        <span
          class="font-mono text-xs uppercase tracking-wider truncate opacity-50 min-w-0 cursor-pointer"
        >
          {{ imc.resource.type }}
        </span>

        <!-- pr-4 给 thumb 右侧留缓冲，避免拖到最右端时寄出容器 -->
        <div
          class="flex-none w-0 pr-4 overflow-hidden opacity-0 group-hover:flex-1 group-hover:opacity-100"
        >
          <Slider
            v-model="imc.alpha"
            :min="0"
            :max="1"
            :step="0.01"
            class="w-full"
          />
        </div>
      </div>
    </div>
    <DialogContent
      v-if="imc.isShowing"
      v-model:show="imc.isShowing"
      :height="'auto'"
      :width="'auto'"
    >
      <template #context="{ close }">
        <div class="w-lg min-h-144 max-h-169 flex flex-col">
          <div
            class="shrink-0 flex items-center justify-between gap-2 h-9 px-3 border-b border-white/10"
          >
            <div>
              <span class="font-medium tracking-wide opacity-60">
                编辑影像：{{
                  imc.resourceCloned.extra.imagery?.type || "内部资源"
                }}格式 |
              </span>
              <Button
                @click="(imc.saveChange(), close())"
                class="p-1 rounded transition-colors"
              >
                保存
              </Button>
              <Button
                @click="(imc.removeResource(), close())"
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
          <!-- 外部资源配置项 -->
          <div
            v-if="imc.resourceCloned.extra.imagery"
            class="flex-1 p-4 gap-4 flex flex-col overflow-auto"
          >
            <div class="field-row">
              <span class="field-label">Name</span>
              <Input v-model="imc.resourceCloned.name" class="field-input" />
            </div>

            <div class="field-row">
              <span class="field-label">Url</span>
              <Input v-model="imc.resourceCloned.url" class="field-input" />
            </div>

            <!-- 通用附加项 -->
            <div class="field-row">
              <span class="field-label">Min Level</span>
              <Input
                v-model="imc.resourceCloned.extra.imagery.minimumLevel"
                type="number"
                class="field-input"
              />
            </div>

            <div class="field-row">
              <span class="field-label">Max Level</span>
              <Input
                v-model="imc.resourceCloned.extra.imagery.maximumLevel"
                type="number"
                class="field-input"
              />
            </div>

            <div class="field-row">
              <span class="field-label">Tiling Scheme</span>
              <Select
                v-model="imc.resourceCloned.extra.imagery.tilingScheme"
                :options="[
                  { label: 'WebMercator', value: 'WebMercator' },
                  { label: 'Geographic', value: 'Geographic' },
                ]"
              />
            </div>

            <div class="field-row">
              <span class="field-label">Credit</span>
              <Input
                v-model="imc.resourceCloned.extra.imagery.credit"
                class="field-input"
                placeholder="版权信息"
              />
            </div>

            <div class="w-full h-0.5 bg-white/10"></div>

            <!-- XYZ 专属 -->
            <div
              v-if="imc.resourceCloned.extra.imagery.type == 'xyz'"
              class="flex flex-col gap-4"
            >
              <div class="field-row">
                <span class="field-label">Subdomains</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.subdomains"
                  class="field-input"
                  placeholder="0,1,2,3,4,5,6,7"
                  :validator="validateSubdomains"
                />
              </div>
            </div>

            <!-- WMS 专属 -->
            <div
              v-else-if="imc.resourceCloned.extra.imagery.type == 'wms'"
              class="flex flex-col gap-4"
            >
              <div class="field-row">
                <span class="field-label">Layers</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.layers"
                  class="field-input"
                  placeholder="layer1,layer2"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Version</span>
                <Select
                  v-model="imc.resourceCloned.extra.imagery.version"
                  :options="[
                    { label: '1.1.1', value: '1.1.1' },
                    { label: '1.3.0', value: '1.3.0' },
                  ]"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Format</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.format"
                  class="field-input"
                  placeholder="image/png"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Pick Features</span>
                <input
                  type="checkbox"
                  v-model="imc.resourceCloned.extra.imagery.enablePickFeatures"
                  class="ml-2 accent-white"
                />
              </div>
            </div>

            <!-- WMTS 专属 -->
            <div
              v-else-if="imc.resourceCloned.extra.imagery.type == 'wmts'"
              class="flex flex-col gap-4"
            >
              <div class="field-row">
                <span class="field-label">Projection</span>
                <Select
                  v-model="imc.resourceCloned.extra.imagery.projection"
                  :options="[
                    { label: '经纬度 (c)', value: 'c' },
                    { label: '球面墨卡托 (w)', value: 'w' },
                  ]"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Layer</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.layer"
                  class="field-input"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Style</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.style"
                  class="field-input"
                  placeholder="default"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Matrix Set</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.tileMatrixSetID"
                  class="field-input"
                  placeholder="留空则跟随 Projection"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Format</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.format"
                  class="field-input"
                  placeholder="tiles"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Subdomains</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.subdomains"
                  class="field-input"
                  placeholder="0,1,2,3,4,5,6,7"
                  :validator="validateSubdomains"
                />
              </div>

              <div class="field-row">
                <span class="field-label">Matrix Labels</span>
                <Input
                  v-model="imc.resourceCloned.extra.imagery.tileMatrixLabels"
                  class="field-input"
                  placeholder="0,1,2..."
                  :validator="validateSubdomains"
                />
              </div>
            </div>
          </div>
          <!-- TODO：内部资源配置项 -->
        </div>
      </template>
    </DialogContent>
  </div>
</template>

<script setup lang="ts">
  import { LocateResourceMessage, type Resource } from "@/ccs/resource";
  import { useController } from "@virid/vue";
  import { ImageryController } from "./controllers/imagery";
  import Slider from "@/components/ui/Slider.vue";
  import {
    Image,
    Eye,
    EyeOff,
    ArrowUp,
    ArrowDown,
    Locate,
    Edit,
    X,
  } from "lucide-vue-next";
  import Button from "../ui/Button.vue";
  import { SortItemMessage } from "./controllers";
  import DialogContent from "../ui/dialog/DialogContent.vue";
  import Input from "@/components/ui/Input.vue";
  import Select from "@/components/ui/Select.vue";
  const prop = defineProps<{
    resource: Resource;
  }>();
  const imc = useController(ImageryController, {
    context: prop,
  });
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
    @apply w-28 shrink-0 text-white/70;
  }

  .field-input {
    @apply flex-1 mr-2 ml-2;
  }
</style>
