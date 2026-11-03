<template>
  <!-- The events a month cell has no room for, listed over the grid: each one a row a finger can aim at,
       held by the console's one long press for its menu as the grid's events are. -->
  <div ref="surface" class="flex flex-col gap-1">
    <CalendarEventChip v-for="{ event, anchored } in props.events" :key="event.key" :event="event" :anchored="anchored" show-time class="pointer-coarse:h-11" />
  </div>
</template>

<script setup lang="ts">
import { useTemplateRef } from "vue";

import CalendarEventChip from "./CalendarEventChip.vue";
import { useGridHold } from "../../composables/useGridHold";

import type { CalendarEvent } from "../../../shared/types";

const props = defineProps<{ events: { event: CalendarEvent; anchored: boolean }[] }>();

const surfaceRef = useTemplateRef<HTMLElement>("surface");

useGridHold(() => surfaceRef.value);
</script>
