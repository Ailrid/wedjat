<template>
  <div class="w-full min-h-144 max-h-169 flex text-sm">
    <div class="w-14 h-auto flex flex-col bg-black/20 text-sm pb-4">
      <Button
        variant="none"
        class="button"
        :class="{ active: ets.tab == 'pointCloud' }"
        @click="ets.tab = 'pointCloud'"
      >
        PCD
      </Button>
      <Button
        variant="none"
        class="button"
        :class="{ active: ets.tab == 'model' }"
        @click="ets.tab = 'model'"
      >
        OSGB
      </Button>
      <div class="flex-1"></div>
      <Button
        variant="icon"
        class="text-white! self-center"
        @click="ets.import()"
      >
        <Import :size="18" />
      </Button>
    </div>

    <div class="flex-1 p-4 gap-4 flex flex-col min-h-0 overflow-auto">
      <div class="field-row">
        <span class="field-label">Name</span>
        <Input v-model="ets.name" class="field-input" required />
      </div>
      <div class="field-row">
        <span class="field-label">Url</span>
        <Input
          v-model="ets.url"
          class="field-input"
          required
          placeholder="指向 tileset.json"
        />
      </div>
      <div class="field-row">
        <span class="field-label">Credit</span>
        <Input
          v-model="ets.credit"
          class="field-input"
          placeholder="版权信息"
        />
      </div>
      <div class="field-row">
        <span class="field-label">自动加载</span>
        <Switch v-model="ets.autoLoad" />
      </div>

      <div class="w-full h-0.5 bg-white/10"></div>

      <!-- ==================== 点云 ==================== -->
      <template v-if="ets.tab == 'pointCloud'">
        <p class="text-white/70">样式</p>

        <div class="field-row">
          <span class="field-label">Style Mode</span>
          <Select
            v-model="ets.styleMode"
            :options="[
              { label: 'Original', value: 'original' },
              { label: 'Uniform', value: 'uniform' },
              { label: 'Classification', value: 'classification' },
              { label: 'Elevation', value: 'elevation' },
            ]"
          />
        </div>

        <div
          v-if="ets.styleMode == 'original'"
          class="text-xs text-white/40 pl-2"
        >
          保留点云自带的分类 / 强度颜色。
        </div>

        <div v-else-if="ets.styleMode == 'uniform'" class="field-row">
          <span class="field-label">Color</span>
          <div class="flex-1 mr-2 ml-2">
            <ColorPicker v-model="ets.uniformColor" />
          </div>
        </div>

        <!-- Classification 规则 -->
        <template v-else-if="ets.styleMode == 'classification'">
          <div class="flex flex-col gap-2">
            <div
              v-for="(rule, i) in ets.classificationRules"
              :key="i"
              class="field-row"
            >
              <Input
                v-model="rule.classification"
                type="number"
                class="w-16"
                placeholder="ID"
              />
              <div class="flex-1 mx-2">
                <ColorPicker v-model="rule.color" />
              </div>
              <Switch v-model="rule.visible" />
              <Button
                variant="icon"
                class="ml-2"
                @click="ets.removeClassificationRule(i)"
              >
                <Trash2 :size="14" />
              </Button>
            </div>
            <Button
              variant="icon"
              class="self-end text-white/50 hover:text-white"
              @click="ets.addClassificationRule()"
            >
              添加类别
            </Button>
          </div>
        </template>

        <!-- Elevation 分段 -->
        <template v-else-if="ets.styleMode == 'elevation'">
          <div class="flex flex-col gap-2">
            <div
              v-for="(stop, i) in ets.elevationStops"
              :key="i"
              class="field-row"
            >
              <Input
                v-model="stop.elevation"
                type="number"
                class="w-24"
                placeholder="高程 (m)"
              />
              <div class="flex-1 mx-2">
                <ColorPicker v-model="stop.color" />
              </div>
              <Button
                variant="icon"
                class=""
                @click="ets.removeElevationStop(i)"
              >
                <Trash2 :size="14" />
              </Button>
            </div>
            <Button
              variant="icon"
              class="self-end text-white/50 hover:text-white"
              @click="ets.addElevationStop()"
            >
              添加分段
            </Button>
          </div>
        </template>

        <div class="w-full h-0.5 bg-white/10"></div>
        <p class="text-white/70">点云表现</p>

        <div class="field-row">
          <span class="field-label">Attenuation</span>
          <Switch v-model="ets.attenuation" />
        </div>
        <div class="field-row">
          <span class="field-label">Eye Dome Lighting</span>
          <Switch v-model="ets.eyeDomeLighting" />
        </div>

        <div class="w-full h-0.5 bg-white/10"></div>
        <p class="text-white/70">精度</p>

        <div class="field-row">
          <span class="field-label">Max SSE</span>
          <Input
            v-model="ets.pcMaximumScreenSpaceError"
            type="number"
            class="field-input"
          />
        </div>
        <div class="field-row">
          <span class="field-label">Dynamic SSE</span>
          <Switch v-model="ets.pcDynamicScreenSpaceError" />
        </div>
      </template>

      <!-- ==================== 三维模型 (OSGB) ==================== -->
      <template v-else-if="ets.tab == 'model'">
        <p class="text-white/70">精度</p>

        <div class="field-row">
          <span class="field-label">Max SSE</span>
          <Input
            v-model="ets.mdMaximumScreenSpaceError"
            type="number"
            class="field-input"
          />
        </div>
        <div class="field-row">
          <span class="field-label">Dynamic SSE</span>
          <Switch v-model="ets.mdDynamicScreenSpaceError" />
        </div>
        <div class="field-row">
          <span class="field-label">Max Memory</span>
          <Input
            v-model="ets.mdMaximumMemoryUsage"
            type="number"
            class="field-input"
            placeholder="MB"
          />
        </div>

        <div class="w-full h-0.5 bg-white/10"></div>
        <p class="text-white/70">渲染</p>

        <div class="field-row">
          <span class="field-label">Shadows</span>
          <Select
            v-model="ets.mdShadows"
            :options="[
              { label: 'ENABLED', value: 'ENABLED' },
              { label: 'DISABLED', value: 'DISABLED' },
              { label: 'RECEIVE_ONLY', value: 'RECEIVE_ONLY' },
              { label: 'CAST_ONLY', value: 'CAST_ONLY' },
            ]"
          />
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useController } from "@virid/vue";
  import { ExternalTilesController } from "./controllers";
  import Button from "@/components/ui/Button.vue";
  import { Import, Trash2 } from "lucide-vue-next";
  import Input from "@/components/ui/Input.vue";
  import Select from "@/components/ui/Select.vue";
  import Switch from "@/components/ui/Switch.vue";
  import ColorPicker from "@/components/ui/ColorPicker.vue";

  const ets = useController(ExternalTilesController);
</script>

<style scoped>
  @reference "@/assets/main.css";

  .active {
    @apply text-white!;
    @apply shadow-[inset_2px_0_0_0_#fff];
  }

  .button {
    @apply w-14 h-14 flex items-center justify-center hover:text-white cursor-pointer;
  }

  .field-row {
    @apply w-full flex items-center justify-between;
  }

  .field-label {
    @apply min-w-28;
  }

  .field-input {
    @apply flex-1 mr-2 ml-2;
  }
</style>
