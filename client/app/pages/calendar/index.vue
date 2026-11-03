<template>
  <div />
</template>

<script setup lang="ts">
import { navigateTo } from "nuxt/app";

import { todayDate } from "../../domain/dates";
import { calendarPath } from "../../composables/useCalendar";

// The screen's own path opens on this month.
void navigateTo(calendarPath(todayDate(), "month"), { replace: true });
</script>
