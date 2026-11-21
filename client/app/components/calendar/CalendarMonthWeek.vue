<template>
  <!-- One week of the month grid, a row of the grid's table: each day a cell holding its number, then a
       fixed stack of slots it fills. All-day bars stand in the cell of their first day this week and run
       across the days they cover, holding their lane; timed events fill the slots the bars leave, and the
       last free slot counts what does not fit. What a cell shows beside a title goes by the cell's own width,
       wherever the grid stands, so a narrow cell keeps its room for the titles. A cell is named by its full date
       and its events, and the keyboard walks the cells (the month view). -->
  <div role="row" :aria-rowindex="props.rowIndex" class="grid min-w-0 grid-cols-7" :style="gridStyle">
    <div
      v-for="(cell, index) in cells"
      :key="cell.iso"
      role="gridcell"
      class="@container/day focus-inset row-span-full grid grid-cols-[minmax(0,1fr)] grid-rows-subgrid border-b border-default"
      :class="[index !== 0 && (cell.day.getDate() === 1 ? 'shadow-[inset_1px_0_0_var(--ui-border-accented)]' : 'shadow-[inset_1px_0_0_var(--ui-border)]'), endsMonth(cell.day) && 'border-b-accented', isWeekend(cell.day) && 'bg-elevated/50']"
      :data-date="cell.iso"
      :tabindex="cell.iso === props.tabDay ? 0 : -1"
      :aria-label="cell.name"
      :aria-current="cell.today ? 'date' : undefined"
      @pointerdown="onGridPointerdown($event, { kind: 'month', day: cell.day })"
      @dblclick="onGridDblclick($event, { kind: 'month', day: cell.day })"
      @contextmenu="onGridContextmenu($event, { kind: 'month', day: cell.day })"
    >
      <!-- The number opens the day for a pointer; the cell says the date to a reader and opens the day on
           Enter, so the number is left out of both. On a touch screen it reaches past its circle to a
           finger's width. -->
      <ULink
        raw
        :to="pathFor(toCalendarDate(cell.day), 'day')"
        tabindex="-1"
        aria-hidden="true"
        class="relative row-start-1 m-0.5 inline-flex h-7 min-w-7 select-none items-center justify-center justify-self-end rounded-full px-1.5 text-sm font-semibold tabular-nums transition-colors pointer-coarse:after:absolute pointer-coarse:after:-inset-2"
        :class="cell.today ? 'badge-ink bg-primary text-inverted hover:bg-primary/75 active:bg-primary/75' : [inFocus(cell.day) ? 'text-highlighted' : 'text-muted', 'hover:bg-elevated active:bg-accented']"
      >
        <!-- The first of a month names its month where the cell has room for it. -->
        <span v-if="cell.day.getDate() === 1" class="whitespace-nowrap @max-[6rem]/day:hidden">{{ formatMonthDay(cell.day) }}</span>
        <span :class="cell.day.getDate() === 1 && '@min-[6rem]/day:hidden'">{{ cell.day.getDate() }}</span>
      </ULink>

      <USkeleton v-for="slot in skeletons[index]" :key="`skeleton-${slot}`" class="pointer-events-none mx-0.5 h-6 self-start rounded-sm" :style="{ gridRow: slot + 2 }" />

      <template v-for="{ event, colSpan, lane, continuesBefore, continuesAfter } in cell.bars" :key="event.key">
        <CalendarEventDraft
          v-if="event.key === DRAFT_EVENT_KEY"
          variant="chip"
          :anchored="!continuesBefore"
          :continues-before="continuesBefore"
          :continues-after="continuesAfter"
          class="relative z-1 mx-0.5 self-start"
          :style="barStyle(lane, colSpan)"
        />
        <CalendarEventChip
          v-else
          :event="event"
          :anchored="!continuesBefore"
          class="relative z-1 mx-0.5 self-start"
          :class="[continuesBefore && 'rounded-s-none', continuesAfter && 'rounded-e-none']"
          :style="barStyle(lane, colSpan)"
        />
      </template>

      <template v-for="{ event, slot, anchored } in cell.events" :key="event.key">
        <CalendarEventDraft v-if="event.key === DRAFT_EVENT_KEY" variant="chip" anchored class="mx-0.5 self-start" :style="{ gridRow: slot + 2 }" />
        <CalendarEventChip v-else :event="event" :anchored="anchored" show-time class="mx-0.5 self-start @max-[6rem]/day:px-1 @max-[8rem]/day:**:data-time:hidden" :style="{ gridRow: slot + 2 }" />
      </template>

      <UPopover v-if="cell.more" :content="{ side: 'right', align: 'start' }" :ui="{ content: 'w-64 p-2' }">
        <template #default="{ open }">
          <UButton
            color="neutral"
            variant="ghost"
            size="sm"
            class="mx-0.5 h-6 justify-start self-start px-1.5 font-normal text-muted @max-[6rem]/day:px-1"
            :class="open && 'bg-elevated'"
            :style="{ gridRow: cell.more.slot + 2 }"
          >
            <!-- A narrow cell counts the rest in the short form. -->
            <span class="@max-[6rem]/day:hidden">{{ $t("calendar.label.more", { n: countText(cell.more.hidden) }) }}</span>
            <span class="@min-[6rem]/day:hidden">{{ $t("calendar.label.more_short", { n: countText(cell.more.hidden) }) }}</span>
          </UButton>
        </template>

        <template #content>
          <CalendarMoreList :events="cell.more.events" />
        </template>
      </UPopover>
    </div>
  </div>
</template>

<script setup lang="ts">
import { addDays, isSameDay, isWeekend } from "date-fns";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { countText } from "bmo-ui/app/domain/numbers";

import CalendarEventChip from "./CalendarEventChip.vue";
import CalendarEventDraft from "./CalendarEventDraft.vue";
import CalendarMoreList from "./CalendarMoreList.vue";
import { dayName } from "../../domain/a11y";
import { formatMonthDay, fromLocalISO, isoDate, toCalendarDate } from "../../domain/dates";
import { DRAFT_EVENT_KEY, layoutAllDay } from "../../domain/layout";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventDraft } from "../../composables/useEventDraft";
import { useEventMove } from "../../composables/useEventMove";

import type { AllDayPositionedEvent } from "../../domain/layout";
import type { CalendarEvent } from "../../../shared/types";

/** A slot's height, in the reader's own text size, so a larger font grows the slots rather than cutting their chips. */
const SLOT_REM = 1.625;
const MAX_SLOTS = 4;
// Bars never take the last slot, so a day always has room for an event or the count of the rest.
const MAX_LANES = MAX_SLOTS - 1;

const props = defineProps<{
  weekStart: Date;
  loading?: boolean;
  /** Its place among the grid's rows, the weekdays' row first. */
  rowIndex: number;
  /** The day the keyboard reaches the grid on, as `YYYY-MM-DD`. */
  tabDay: string;
}>();

const { t } = useI18n();
const { date, visibleMonth, pathFor } = useCalendar();
const { eventsForDay, eventsForDays } = useCalendarEvents();
const { draftEvent, onGridPointerdown, onGridDblclick, onGridContextmenu } = useEventDraft();
const { movingKey, preview } = useEventMove();

const days = computed(() => Array.from({ length: 7 }, (_, index) => addDays(props.weekStart, index)));
const weekEnd = computed(() => addDays(props.weekStart, 7));

const gridStyle = {
  gridTemplateRows: ["auto", ...Array.from({ length: MAX_LANES }, () => `${SLOT_REM}rem`), `minmax(${SLOT_REM}rem, 1fr)`].join(" ")
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

/** A bar stands in its first day's cell and runs across the cells of the days it covers. */
function barStyle(lane: number, colSpan: number): Record<string, string | number> {
  return { gridRow: lane + 2, width: `calc(${colSpan * 100}% - 0.25rem)` };
}

const cells = computed(() => {
  const today = new Date();
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

    const count = eventsForDay(day).length;

    return {
      day,
      iso: isoDate(day),
      today: isSameDay(day, today),
      name: dayName(t, day, count, today, countText),
      bars: bars.value.filter((bar) => bar.colStart === index),
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

// The skeleton's slots in each day's cell.
const skeletons = computed(() => Array.from({ length: 7 }, (_, index) => (loading.value ? SKELETONS.filter(([column]) => column === index).map(([, slot]) => slot) : [])));

const focus = computed(() => visibleMonth.value ?? date.value);

function inFocus(day: Date): boolean {
  return day.getMonth() + 1 === focus.value.month && day.getFullYear() === focus.value.year;
}

// The line between two months is drawn stronger, stepping around the first of the month.
function endsMonth(day: Date): boolean {
  return addDays(day, 7).getMonth() !== day.getMonth();
}
</script>
