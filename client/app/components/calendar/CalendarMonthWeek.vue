<template>
  <!-- One week of the month grid: the days' numbers, then a fixed stack of slots each day fills. All-day
       bars span the days they cover and hold their lane across them; timed events fill the slots the bars
       leave, and the last free slot counts what does not fit. -->
  <div class="grid min-w-0 grid-cols-7" :style="gridStyle">
    <div
      v-for="({ day }, index) in cells"
      :key="`day-${day.getTime()}`"
      :data-date="isoDate(day)"
      class="row-span-full border-b border-default"
      :class="[index !== 0 && 'border-s', index !== 0 && day.getDate() === 1 && 'border-s-accented', endsMonth(day) && 'border-b-accented', isWeekend(day) && 'bg-elevated/50']"
      :style="{ gridColumn: index + 1 }"
      @pointerdown="onGridPointerdown($event, { kind: 'month', day })"
      @dblclick="onGridDblclick($event, { kind: 'month', day })"
    />

    <ULink
      v-for="({ day }, index) in cells"
      :key="`number-${day.getTime()}`"
      raw
      :to="pathFor(toCalendarDate(day), 'day')"
      class="row-start-1 m-0.5 inline-flex h-7 min-w-7 select-none items-center justify-center justify-self-end rounded-full px-1.5 text-sm font-semibold tabular-nums transition-colors"
      :class="isToday(day) ? 'bg-primary text-inverted hover:bg-primary/75 active:bg-primary/75' : [inFocus(day) ? 'text-highlighted' : 'text-muted', 'hover:bg-elevated active:bg-accented']"
      :style="{ gridColumn: index + 1 }"
    >
      {{ label(day) }}
    </ULink>

    <USkeleton
      v-for="[day, slot] in loading ? SKELETONS : []"
      :key="`skeleton-${day}-${slot}`"
      class="pointer-events-none mx-0.5 h-6 self-start rounded-sm"
      :style="{ gridColumn: day + 1, gridRow: slot + 2 }"
    />

    <template v-for="{ event, colStart, colSpan, lane, continuesBefore, continuesAfter } in bars" :key="event.key">
      <CalendarEventDraft
        v-if="event.key === DRAFT_EVENT_KEY"
        variant="chip"
        :anchored="!continuesBefore"
        :continues-before="continuesBefore"
        :continues-after="continuesAfter"
        class="mx-0.5 self-start"
        :style="{ gridColumn: `${colStart + 1} / span ${colSpan}`, gridRow: lane + 2 }"
      />
      <CalendarEventChip
        v-else
        :event="event"
        :anchored="!continuesBefore"
        class="mx-0.5 self-start"
        :class="[continuesBefore && 'rounded-s-none', continuesAfter && 'rounded-e-none']"
        :style="{ gridColumn: `${colStart + 1} / span ${colSpan}`, gridRow: lane + 2 }"
      />
    </template>

    <template v-for="({ day, events: dayEvents, more }, index) in cells" :key="`events-${day.getTime()}`">
      <template v-for="{ event, slot, anchored } in dayEvents" :key="event.key">
        <CalendarEventDraft v-if="event.key === DRAFT_EVENT_KEY" variant="chip" anchored class="mx-0.5 self-start" :style="{ gridColumn: index + 1, gridRow: slot + 2 }" />
        <CalendarEventChip
          v-else
          :event="event"
          :anchored="anchored"
          show-time
          class="mx-0.5 self-start max-lg:**:data-time:hidden"
          :style="{ gridColumn: index + 1, gridRow: slot + 2 }"
        />
      </template>

      <UPopover v-if="more" :content="{ side: 'right', align: 'start' }" :ui="{ content: 'flex w-64 flex-col gap-1 p-2' }">
        <template #default="{ open }">
          <UButton
            color="neutral"
            variant="ghost"
            size="sm"
            class="mx-0.5 h-6 justify-start self-start px-1.5 font-normal text-muted"
            :class="open && 'bg-elevated'"
            :label="$t('calendar.label.more', { n: countText(more.hidden) })"
            :style="{ gridColumn: index + 1, gridRow: more.slot + 2 }"
          />
        </template>

        <template #content>
          <CalendarEventChip v-for="{ event, anchored } in more.events" :key="event.key" :event="event" :anchored="anchored" show-time />
        </template>
      </UPopover>
    </template>
  </div>
</template>

<script setup lang="ts">
import { addDays, isSameDay, isToday, isWeekend } from "date-fns";
import { computed } from "vue";
import { countText } from "bmo-ui/app/domain/numbers";

import CalendarEventChip from "./CalendarEventChip.vue";
import CalendarEventDraft from "./CalendarEventDraft.vue";
import { formatMonthDay, fromLocalISO, isoDate, toCalendarDate } from "../../domain/dates";
import { DRAFT_EVENT_KEY, layoutAllDay } from "../../domain/layout";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventDraft } from "../../composables/useEventDraft";
import { useEventMove } from "../../composables/useEventMove";

import type { AllDayPositionedEvent } from "../../domain/layout";
import type { CalendarEvent } from "../../../shared/types";

const SLOT_HEIGHT = 26;
const MAX_SLOTS = 4;
// Bars never take the last slot, so a day always has room for an event or the count of the rest.
const MAX_LANES = MAX_SLOTS - 1;

const props = defineProps<{ weekStart: Date; loading?: boolean }>();

const { date, visibleMonth, pathFor } = useCalendar();
const { eventsForDay, eventsForDays } = useCalendarEvents();
const { draftEvent, onGridPointerdown, onGridDblclick } = useEventDraft();
const { movingKey, preview } = useEventMove();

const days = computed(() => Array.from({ length: 7 }, (_, index) => addDays(props.weekStart, index)));
const weekEnd = computed(() => addDays(props.weekStart, 7));

const gridStyle = {
  gridTemplateRows: ["auto", ...Array.from({ length: MAX_LANES }, () => `${SLOT_HEIGHT}px`), `minmax(${SLOT_HEIGHT}px, 1fr)`].join(" ")
};

function inWeek(event: CalendarEvent | null): CalendarEvent | null {
  return event && fromLocalISO(event.start) < weekEnd.value && fromLocalISO(event.end) > props.weekStart ? event : null;
}

const weekDraft = computed(() => inWeek(draftEvent.value));
const weekMoved = computed(() => inWeek(preview.value));

function others(events: CalendarEvent[]): CalendarEvent[] {
  return movingKey.value ? events.filter((event) => event.key !== movingKey.value) : events;
}

const lanes = computed(() =>
  layoutAllDay(
    [...others(eventsForDays(days.value)).filter((event) => event.allDay), ...(weekDraft.value?.allDay ? [weekDraft.value] : []), ...(weekMoved.value?.allDay ? [weekMoved.value] : [])],
    days.value
  )
);

// The draft is the one bar never cut: past the last lane it takes that lane.
const placed = computed(() => lanes.value.map((bar) => (bar.event.key === DRAFT_EVENT_KEY ? { ...bar, lane: Math.min(bar.lane, MAX_LANES - 1) } : bar)));

const bars = computed(() =>
  placed.value
    .filter((bar) => bar.lane < MAX_LANES)
    .map((bar) => ({ ...bar, continuesBefore: fromLocalISO(bar.event.start) < props.weekStart, continuesAfter: fromLocalISO(bar.event.end) > weekEnd.value }))
);

function covers(bar: AllDayPositionedEvent, index: number): boolean {
  return index >= bar.colStart && index < bar.colStart + bar.colSpan;
}

const cells = computed(() => {
  const drafted = weekDraft.value && !weekDraft.value.allDay ? weekDraft.value : null;
  const moved = weekMoved.value && !weekMoved.value.allDay ? weekMoved.value : null;

  return days.value.map((day, index) => {
    const timed = [
      ...others(eventsForDay(day)).filter((event) => !event.allDay),
      ...(drafted && isSameDay(fromLocalISO(drafted.start), day) ? [drafted] : []),
      ...(moved && isSameDay(fromLocalISO(moved.start), day) ? [moved] : [])
    ].sort((a, b) => a.start.localeCompare(b.start));

    const covering = placed.value.filter((bar) => covers(bar, index));
    const occupied = new Set(covering.filter((bar) => bar.lane < MAX_LANES).map((bar) => bar.lane));
    const dropped = covering.filter((bar) => bar.lane >= MAX_LANES);
    const free = Array.from({ length: MAX_SLOTS }, (_, slot) => slot).filter((slot) => !occupied.has(slot));
    const overflows = dropped.length > 0 || timed.length > free.length;
    let visible = timed.slice(0, overflows ? Math.max(free.length - 1, 0) : free.length);

    if (drafted && timed.includes(drafted) && !visible.includes(drafted)) visible = [...visible.slice(0, -1), drafted];

    const slot = free[visible.length];
    const shown = new Set([...visible.map((event) => event.key), ...covering.filter((bar) => bar.lane < MAX_LANES).map((bar) => bar.event.key)]);

    return {
      day,
      events: visible.map((event, position) => ({ event, slot: free[position]!, anchored: isSameDay(fromLocalISO(event.start), day) })),
      more:
        overflows && slot !== undefined
          ? {
              slot,
              hidden: dropped.length + timed.length - visible.length,
              events: [...covering.map((bar) => bar.event), ...timed].filter((event) => event.key !== DRAFT_EVENT_KEY).map((event) => ({ event, anchored: !shown.has(event.key) }))
            }
          : null
    };
  });
});

// Placeholder chips while the week's events are first read, as [day, slot].
const SKELETONS: [number, number][] = [
  [0, 0],
  [1, 0],
  [1, 1],
  [3, 0],
  [4, 0],
  [4, 1],
  [6, 0]
];

const loading = computed(() => props.loading && !bars.value.length && cells.value.every((cell) => !cell.events.length && !cell.more));

const focus = computed(() => visibleMonth.value ?? date.value);

function inFocus(day: Date): boolean {
  return day.getMonth() + 1 === focus.value.month && day.getFullYear() === focus.value.year;
}

// The line between two months is drawn stronger, stepping around the first of the month.
function endsMonth(day: Date): boolean {
  return addDays(day, 7).getMonth() !== day.getMonth();
}

function label(day: Date): string {
  return day.getDate() === 1 ? formatMonthDay(day) : String(day.getDate());
}
</script>
