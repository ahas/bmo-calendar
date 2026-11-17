<template>
  <!-- The app shell the console mounts a bundle's screens in. -->
  <UApp :locale="ko">
    <NuxtPage />
  </UApp>
</template>

<script setup lang="ts">
import { ko } from "@nuxt/ui/locale";
</script>
