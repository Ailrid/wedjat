<template>
  <div class="w-full h-full flex items-center">
    <div class="w-12"></div>
    <div class="h-full flex items-center gap-2">
      <ImportNav></ImportNav>
      <UpLoadNav></UpLoadNav>
      <EditNav> </EditNav>
      <ConnectNav></ConnectNav>
      <HelpNav></HelpNav>
    </div>
    <div class="flex-1"></div>
    <div class="h-full flex items-center">
      <Button @click="ToggleCameraMessage.send()">
        <Sidebar class="w-5 h-5"></Sidebar>
      </Button>
    </div>
  </div>
</template>

<script setup lang="ts">
  import ConnectNav from "./ConnectNav.vue";
  import EditNav from "./EditNav.vue";
  import ImportNav from "./ImportNav.vue";
  import HelpNav from "./HelpNav.vue";
  import UpLoadNav from "./UpLoadNav.vue";
  import Button from "@/components/ui/Button.vue";
  import { Sidebar } from "lucide-vue-next";
  import { ToggleCameraMessage } from "../drone/controllers";
</script>

<style scoped></style>
