<template>
  <div />
</template>

<script setup lang="ts">
import { navigateTo, useRoute } from "nuxt/app";

import { addressedCompany } from "../../domain/companies";
import { todayDate } from "../../domain/dates";
import { calendarPath } from "../../composables/useCalendar";

// The screen's own path opens on this month, on the company its address names.
void navigateTo(calendarPath(todayDate(), "month", addressedCompany(useRoute().query)), { replace: true });
</script>
