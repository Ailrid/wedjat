<template>
  <div class="h-full w-full">
    <router-view v-slot="{ Component }">
      <component :is="Component" />
    </router-view>
  </div>
</template>
<script setup lang="ts"></script>

<style lang="css"></style>
