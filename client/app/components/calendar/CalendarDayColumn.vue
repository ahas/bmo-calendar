<template>
  <!-- One day of the time grid: its hour lines, its events laid out side by side where they overlap, the
       ghost of a draft drawn on it, and the present moment where it is today. Its top is midnight, which
       is what a gesture reads a time off. -->
  <div
    data-day-column
    role="group"
    :data-date="isoDate(day)"
    :aria-label="props.label"
    class="relative h-[calc(var(--calendar-hour)*24)] snap-start border-s border-default"
    :class="isWeekend(day) && 'bg-elevated/50'"
    @pointerdown="onGridPointerdown($event, { kind: 'timed', day })"
    @dblclick="onGridDblclick($event, { kind: 'timed', day })"
    @contextmenu="onGridContextmenu($event, { kind: 'timed', day })"
  >
    <div v-for="hour in 23" :key="hour" class="pointer-events-none absolute inset-x-0 snap-start border-t border-default" :style="{ top: minutesLength(hour * 60) }" />

    <USkeleton
      v-for="[hour, hours] in loading ? SKELETONS : []"
      :key="hour"
      class="pointer-events-none absolute inset-x-1 rounded-sm"
      :style="{ top: `calc(${minutesLength(hour * 60)} + 2px)`, height: `calc(${minutesLength(hours * 60)} - 3px)` }"
    />

    <template v-for="positioned in events" :key="positioned.event.key">
      <CalendarEventDraft v-if="positioned.event.key === DRAFT_EVENT_KEY" variant="block" anchored :style="eventBlockStyle(positioned)" />
      <CalendarEventBlock v-else :positioned="positioned" :anchored="anchored(positioned.event)" />
    </template>

    <CalendarNowIndicator v-if="isToday(day)" />
  </div>
</template>

<script setup lang="ts">
import { isSameDay, isToday, isWeekend } from "date-fns";

import CalendarEventBlock from "./CalendarEventBlock.vue";
import CalendarEventDraft from "./CalendarEventDraft.vue";
import CalendarNowIndicator from "./CalendarNowIndicator.vue";
import { fromLocalISO, isoDate } from "../../domain/dates";
import { DRAFT_EVENT_KEY, eventBlockStyle, minutesLength } from "../../domain/layout";
import { useEventDraft } from "../../composables/useEventDraft";

import type { PositionedEvent } from "../../domain/layout";
import type { CalendarEvent } from "../../../shared/types";

// The places a day's events are kept while the span is first read, as [start hour, hours].
const SKELETONS = [
  [9, 1.5],
  [13, 1],
  [16, 2]
] as const;

const props = defineProps<{
  day: Date;
  events: PositionedEvent[];
  // The leftmost column on screen, where a block arriving from a day off screen takes its form.
  first?: boolean;
  loading?: boolean;
  /** The day by its full date and its events, for a reader who does not see the column. */
  label: string;
}>();

const { onGridPointerdown, onGridDblclick, onGridContextmenu } = useEventDraft();

// A block running past midnight is drawn in both days, and the form stands by the one holding its start.
function anchored(event: CalendarEvent): boolean {
  const start = fromLocalISO(event.start);

  return isSameDay(start, props.day) || (!!props.first && start < props.day);
}
</script>
