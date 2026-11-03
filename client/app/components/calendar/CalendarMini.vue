<template>
  <!-- The small month beside the grid: a day pressed is where the screen goes. -->
  <UCalendar v-model="model" :week-starts-on="WEEK_STARTS_ON" :year-controls="false" fixed-weeks class="px-2" />
</template>

<script setup lang="ts">
import { computed } from "vue";
import { navigateTo } from "nuxt/app";

import { WEEK_STARTS_ON } from "../../domain/dates";
import { useCalendar } from "../../composables/useCalendar";

import type { CalendarDate } from "@internationalized/date";

const { date, pathFor } = useCalendar();

const model = computed({
  get: () => date.value,
  set(value: CalendarDate | null | undefined) {
    if (value) void navigateTo(pathFor(value));
  }
});
</script>
