<template>
  <!-- The present moment on today's column, moved once a minute. -->
  <div class="pointer-events-none absolute inset-x-0 z-10 border-t-2 border-error" :style="{ top }">
    <div class="absolute -top-[5px] -start-1 size-2 rounded-full bg-error" />
  </div>
</template>

<script setup lang="ts">
import { useNow } from "@vueuse/core";
import { computed } from "vue";

import { minutesLength } from "../../domain/layout";

const now = useNow({ interval: 60_000 });

const top = computed(() => minutesLength(now.value.getHours() * 60 + now.value.getMinutes()));
</script>
