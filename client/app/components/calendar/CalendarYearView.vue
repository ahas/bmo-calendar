<template>
  <!-- The year: every month on the six weeks the month view gives it, so they line up row by row. A month's
       name opens it; a day opens that day. -->
  <UScrollArea class="min-h-0 flex-1">
    <div class="grid grid-cols-2 gap-x-6 gap-y-8 p-4 sm:grid-cols-3 lg:p-6 xl:grid-cols-4">
      <section v-for="month in months" ref="sections" :key="month.first.getMonth()" :data-month="month.first.getMonth() + 1">
        <ULink
          raw
          :to="pathFor(toCalendarDate(month.first), 'month')"
          class="inline-block rounded-md px-1 text-lg font-semibold transition-colors hover:bg-elevated active:bg-accented"
          :class="today.year === date.year && today.month === month.first.getMonth() + 1 ? 'text-primary' : 'text-highlighted'"
        >
          {{ formatMonth(month.first) }}
        </ULink>

        <div class="mt-2 grid grid-cols-7 text-center text-sm text-muted">
          <span v-for="(weekday, index) in weekdays" :key="index">{{ weekday }}</span>
        </div>

        <div class="mt-1 grid grid-cols-7 justify-items-center gap-y-0.5">
          <template v-for="{ day, inMonth } in month.days" :key="day.getTime()">
            <ULink
              v-if="inMonth"
              raw
              :to="pathFor(toCalendarDate(day), 'day')"
              :aria-label="formatFullDate(day)"
              class="inline-flex aspect-square w-full max-w-7 items-center justify-center rounded-full text-sm tabular-nums transition-colors"
              :class="isToday(day) ? 'bg-primary font-semibold text-inverted hover:bg-primary/75 active:bg-primary/75' : [isWeekend(day) ? 'text-muted' : 'text-default', 'hover:bg-elevated active:bg-accented']"
            >
              {{ day.getDate() }}
            </ULink>
            <span v-else class="aspect-square w-full max-w-7" />
          </template>
        </div>
      </section>
    </div>
  </UScrollArea>
</template>

<script setup lang="ts">
import { addDays, isToday, isWeekend, startOfWeek } from "date-fns";
import { computed, onMounted, useTemplateRef } from "vue";

import { WEEK_STARTS_ON, formatFullDate, formatMonth, formatNarrowWeekday, toCalendarDate, todayDate } from "../../domain/dates";
import { useCalendar } from "../../composables/useCalendar";

const { date, pathFor } = useCalendar();

const weekdays = Array.from({ length: 7 }, (_, index) => formatNarrowWeekday(addDays(startOfWeek(new Date(), { weekStartsOn: WEEK_STARTS_ON }), index)));

const months = computed(() =>
  Array.from({ length: 12 }, (_, index) => {
    const first = new Date(date.value.year, index, 1);
    const start = startOfWeek(first, { weekStartsOn: WEEK_STARTS_ON });

    return {
      first,
      days: Array.from({ length: 42 }, (_, offset) => {
        const day = addDays(start, offset);

        return { day, inMonth: day.getMonth() === index };
      })
    };
  })
);

const today = todayDate();
const sections = useTemplateRef<HTMLElement[]>("sections");

// It opens on the month it was reached from rather than on January.
onMounted(() => {
  sections.value?.find((section) => section.dataset.month === String(date.value.month))?.scrollIntoView({ block: "center" });
});
</script>
