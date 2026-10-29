<template>
  <div class="w-full h-full text-sm flex items-center">
    <Import></Import>
    <Edit> </Edit>
    <Connect></Connect>
    <Help></Help>
  </div>
</template>

<script setup lang="ts">
  import Connect from "./Connect.vue";
  import Edit from "./Edit.vue";
  import Import from "./Import.vue";
  import Help from "./Help.vue";
</script>

<style scoped></style>
