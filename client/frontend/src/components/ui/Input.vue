<template>
  <div class="flex flex-col" :class="$attrs.class">
    <input
      v-bind="inputAttrs"
      :type="type"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step"
      @input="onInput"
      @blur="touched = true"
      class="hide-arrows w-full border rounded px-2 py-1 outline-none focus:ring-1 bg-transparent text-white border-foreground/10 focus:ring-primary focus:border-primary"
      :class="
        showError
          ? 'border-red-500! focus:border-red-500! focus:ring-red-500!'
          : ''
      "
      :aria-invalid="showError"
    />
    <span v-if="showError" class="text-red-500 text-xs mt-0.5">
      {{ errorMessage }}
    </span>
  </div>
</template>

<script setup lang="ts">
  import { computed, ref, useAttrs } from "vue";

  defineOptions({ inheritAttrs: false });

  interface Props {
    modelValue: string | number;
    step?: string | number;
    type?: string;
    min?: string | number;
    max?: string | number;
    /** 是否必填 */
    required?: boolean;
    /** 自定义验证函数：返回 true 通过，返回 string 作为错误消息，返回 false 用默认错误 */
    validator?: (value: string | number) => boolean | string;
    /** 必填为空时显示的警告字符串 */
    emptyMessage?: string;
  }

  const props = withDefaults(defineProps<Props>(), {
    modelValue: "",
    type: "text",
    step: 1,
    min: 0,
    max: 100,
    required: false,
    emptyMessage: "此项不能为空",
  });

  const emit = defineEmits(["update:modelValue"]);

  const attrs = useAttrs();
  const inputAttrs = computed(() => {
    const clone: Record<string, unknown> = { ...attrs };
    delete clone.class;
    return clone;
  });

  const touched = ref(false);

  const errorMessage = computed(() => {
    const value = props.modelValue;
    const isEmpty =
      value === "" ||
      value === undefined ||
      value === null ||
      (typeof value === "number" && Number.isNaN(value));

    if (isEmpty) {
      return props.required ? props.emptyMessage : "";
    }

    if (props.validator) {
      const result = props.validator(value);
      if (result === false) return "输入值不合法";
      if (typeof result === "string") return result;
    }

    return "";
  });

  const showError = computed(() => touched.value && !!errorMessage.value);

  const onInput = (event: Event) => {
    const target = event.target as HTMLInputElement;
    let value: string | number = target.value;

    if (props.type === "number") {
      value = value === "" ? "" : Number(value.trim());
      if (Number.isNaN(value)) return;
    }

    emit("update:modelValue", value);
  };

  defineExpose({
    /** 主动触发验证（例如表单提交前），返回是否通过 */
    validate: () => {
      touched.value = true;
      return !errorMessage.value;
    },
  });
</script>

<style scoped>
  @reference "@/assets/main.css";

  .hide-arrows::-webkit-outer-spin-button,
  .hide-arrows::-webkit-inner-spin-button {
    @apply m-0 appearance-none;
  }
</style>
