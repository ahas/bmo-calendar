<template>
  <!-- A day typed segment by segment, or chosen from the month beside it. -->
  <div class="flex min-w-0 items-center gap-1">
    <UInputDate :id="props.id" v-model="model" :size="props.size" :disabled="props.disabled" :aria-label="props.label" class="min-w-0" />
    <UPopover :content="{ align: 'end' }" :disabled="props.disabled">
      <IconButton :size="props.size" icon="i-material-symbols-calendar-today-outline" :disabled="props.disabled" :label="$t('calendar.action.pick_day')" />

      <template #content>
        <UCalendar v-model="model" :week-starts-on="WEEK_STARTS_ON" class="p-2" />
      </template>
    </UPopover>
  </div>
</template>

<script setup lang="ts">
import { WEEK_STARTS_ON } from "../../domain/dates";

import type { CalendarDate } from "@internationalized/date";

const props = defineProps<{ label: string; id?: string; size?: "md" | "xl"; disabled?: boolean }>();
const model = defineModel<CalendarDate | undefined>();
</script>
