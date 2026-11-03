<template>
  <!-- A day typed segment by segment, or chosen from the month beside it. -->
  <div class="flex min-w-0 items-center gap-1">
    <UInputDate v-model="model" :aria-label="props.label" class="min-w-0" />
    <UPopover :content="{ align: 'end' }">
      <UTooltip :text="$t('calendar.action.pick_day')">
        <UButton color="neutral" variant="ghost" icon="i-material-symbols-calendar-today-outline" :aria-label="$t('calendar.action.pick_day')" />
      </UTooltip>

      <template #content>
        <UCalendar v-model="model" :week-starts-on="WEEK_STARTS_ON" class="p-2" />
      </template>
    </UPopover>
  </div>
</template>

<script setup lang="ts">
import { WEEK_STARTS_ON } from "../../domain/dates";

import type { CalendarDate } from "@internationalized/date";

const props = defineProps<{ label: string }>();
const model = defineModel<CalendarDate | undefined>();
</script>
