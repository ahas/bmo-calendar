<template>
  <!-- The month view: the weekdays' names, then the weeks scrolling on without end either way, read in
       spans of six weeks as they come near. Where the scroll comes to rest, the month at its top becomes
       the screen's address. To a reader it is one grid of days under the weekdays, walked with the arrow
       keys, Home, End, Page Up and Page Down; Enter opens the day and `n` draws a new event on it. -->
  <div
    ref="grid"
    role="grid"
    class="flex min-h-0 flex-1 flex-col"
    :aria-label="gridLabel"
    :aria-rowcount="weeks.length + 1"
    aria-colcount="7"
    @keydown="onKeydown"
    @focusin="onFocusin"
  >
    <div role="row" aria-rowindex="1" class="grid h-9 shrink-0 grid-cols-7 overflow-hidden border-b border-default [scrollbar-gutter:stable]">
      <span
        v-for="(weekday, index) in weekdays"
        :key="weekday.short"
        role="columnheader"
        class="flex items-center justify-end border-default pe-2 text-sm text-muted"
        :class="[index !== 0 && 'border-s', isWeekendIndex(index) && 'bg-elevated/50']"
        :aria-label="weekday.long"
      >
        {{ weekday.short }}
      </span>
    </div>

    <UScrollArea
      ref="scrollArea"
      :items="weeks"
      :virtualize="{ estimateSize: rowHeight, skipMeasurement: true, overscan: 4, initialOffset }"
      :ui="{ item: 'snap-start' }"
      class="min-h-0 flex-1 snap-y snap-proximity pb-[var(--bmo-create-room)] [scrollbar-gutter:stable]"
      @scroll="onScroll"
    >
      <template #default="{ item, index }">
        <CalendarMonthWeek :week-start="item" :row-index="index + 2" :tab-day="tabDay" :loading="rowLoading(item)" :style="{ height: `${ROW_REM}rem` }" />
      </template>
    </UScrollArea>
  </div>
</template>

<script setup lang="ts">
import { addDays, addWeeks, differenceInCalendarWeeks, startOfWeek } from "date-fns";
import { useEventListener } from "@vueuse/core";
import { computed, nextTick, onUnmounted, ref, shallowRef, unref, useTemplateRef, watch } from "vue";
import { navigateTo } from "nuxt/app";
import { useI18n } from "vue-i18n";

import CalendarMonthWeek from "./CalendarMonthWeek.vue";
import { isDayKey, rangeWords, stepDay } from "../../domain/a11y";
import { WEEKS_AROUND, WEEK_STARTS_ON, dateFromISO, formatLongWeekday, formatWeekday, isoDate, monthRange, toCalendarDate, toDate, todayDate } from "../../domain/dates";
import { rootFontPx } from "../../domain/layout";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventDraft } from "../../composables/useEventDraft";
import { useGridHold } from "../../composables/useGridHold";
import { useMotion } from "../../composables/useMotion";

import type { CalendarDate } from "@internationalized/date";
import type { DateRange } from "../../../shared/types";

/** A week's height, in the reader's own text size; the list is told it in pixels once, as it opens. */
const ROW_REM = 9;
const CHUNK_WEEKS = 6;

const gridRef = useTemplateRef<HTMLElement>("grid");

const { locale } = useI18n();
const { date, pathFor, visibleMonth } = useCalendar();
const { loadSpan, showSpans, pendingSpans } = useCalendarEvents();
const { draft, pendingScroll, registerHost } = useEventDraft();
const { scrollBehavior } = useMotion();

registerHost();

// A held finger on an event opens its menu, and on an empty day draws a new event there.
useGridHold(() => gridRef.value);

const rowHeight = ROW_REM * rootFontPx();

const scrollElement = shallowRef<Element | null>(null);

const firstWeek = addWeeks(startOfWeek(new Date(), { weekStartsOn: WEEK_STARTS_ON }), -WEEKS_AROUND);
const weeks = Array.from({ length: WEEKS_AROUND * 2 + 1 }, (_, index) => addWeeks(firstWeek, index));
const weekdays = Array.from({ length: 7 }, (_, index) => ({ short: formatWeekday(addDays(firstWeek, index)), long: formatLongWeekday(addDays(firstWeek, index)) }));

function isWeekendIndex(index: number): boolean {
  const day = addDays(firstWeek, index).getDay();

  return day === 0 || day === 6;
}

function indexOf(day: Date): number {
  return Math.min(weeks.length - 1, Math.max(0, differenceInCalendarWeeks(day, firstWeek, { weekStartsOn: WEEK_STARTS_ON })));
}

// The list is drawn from the month it opens on, from its very first pass.
function initialOffset(): number {
  return indexOf(monthRange(date.value).start) * rowHeight;
}

const scrollArea = useTemplateRef<{ virtualizer?: unknown }>("scrollArea");

interface Virtualizer {
  scrollElement: Element | null;
  getVirtualItems: () => { index: number }[];
  scrollToIndex: (index: number, options?: { align?: "start" | "center"; behavior?: "auto" | "smooth" }) => void;
}

function getVirtualizer(): Virtualizer | undefined {
  return unref(scrollArea.value?.virtualizer) as Virtualizer | undefined;
}

function visibleRange(): { first: number; last: number } | null {
  const items = getVirtualizer()?.getVirtualItems() ?? [];

  if (!items.length) return null;

  return { first: items[0]!.index, last: items[items.length - 1]!.index };
}

function chunkSpan(chunk: number): DateRange {
  const start = addWeeks(firstWeek, chunk * CHUNK_WEEKS);

  return { start, end: addWeeks(start, CHUNK_WEEKS) };
}

// The six-week spans around what is on screen, each read once; the ones on screen are the spans a change
// the service tells of reads again.
function loadVisibleChunks(): void {
  const visible = visibleRange();

  if (!visible) return;

  const first = Math.max(0, Math.floor((visible.first - CHUNK_WEEKS) / CHUNK_WEEKS));
  const last = Math.min(Math.floor((weeks.length - 1) / CHUNK_WEEKS), Math.floor((visible.last + CHUNK_WEEKS) / CHUNK_WEEKS));
  const spans: DateRange[] = [];

  for (let chunk = first; chunk <= last; chunk++) spans.push(chunkSpan(chunk));

  showSpans(spans);

  for (const span of spans) void loadSpan(span);
}

function rowLoading(week: Date): boolean {
  return pendingSpans.value.some((span) => week >= span.start && week < span.end);
}

watch(pendingScroll, (pending) => {
  if (!pending || !draft.value) return;

  getVirtualizer()?.scrollToIndex(indexOf(draft.value.start), { align: "center", behavior: scrollBehavior.value });
  loadVisibleChunks();
});

function scrollToMonth(target: CalendarDate, smooth = false): void {
  getVirtualizer()?.scrollToIndex(indexOf(monthRange(target).start), { align: "start", behavior: smooth ? scrollBehavior.value : "auto" });
}

// The month whose last day of the top week stands at the top of the scroll.
function dockedMonth(): CalendarDate | null {
  const offset = scrollElement.value?.scrollTop;

  if (offset == null) return null;

  const top = Math.min(weeks.length - 1, Math.max(0, Math.floor(offset / rowHeight)));

  return toCalendarDate(addDays(weeks[top]!, 6)).set({ day: 1 });
}

function update(): void {
  const docked = dockedMonth();

  if (docked && (!visibleMonth.value || docked.compare(visibleMonth.value) !== 0)) visibleMonth.value = docked;
}

useEventListener(
  scrollElement,
  "scroll",
  () => {
    update();
    loadVisibleChunks();
  },
  { passive: true }
);

function applyInitialPosition(): boolean {
  const element = getVirtualizer()?.scrollElement;

  if (!element) return false;

  scrollElement.value = element;
  scrollToMonth(date.value);

  if (element.scrollTop === 0) return false;

  update();
  loadVisibleChunks();

  return true;
}

watch(
  scrollArea,
  async () => {
    if (applyInitialPosition()) return;

    for (let attempt = 0; attempt < 60; attempt++) {
      await new Promise((resolve) => setTimeout(resolve, 50));

      if (applyInitialPosition()) return;
    }
  },
  { flush: "post", once: true }
);

const syncing = ref(false);

async function syncRoute(): Promise<void> {
  const target = dockedMonth();

  if (!target || (target.year === date.value.year && target.month === date.value.month)) return;

  syncing.value = true;
  await navigateTo(pathFor(target), { replace: true });
  syncing.value = false;
}

function onScroll(isScrolling: boolean): void {
  if (!isScrolling) void syncRoute();
}

// A jump from outside — the small calendar, the steps, today — scrolls the list there.
watch(
  () => date.value.toString(),
  () => {
    if (syncing.value) return;

    const docked = dockedMonth();

    if (docked && docked.year === date.value.year && docked.month === date.value.month) return;

    const visible = visibleRange();
    const target = indexOf(monthRange(date.value).start);
    const smooth = !!visible && Math.abs(target - Math.round((visible.first + visible.last) / 2)) <= 12;

    scrollToMonth(date.value, smooth);
    loadVisibleChunks();
  }
);

onUnmounted(() => {
  visibleMonth.value = null;
});

const focusMonth = computed(() => visibleMonth.value ?? date.value);
const gridLabel = computed(() => rangeWords("month", monthRange(focusMonth.value), toDate(focusMonth.value), locale.value));

// The day the keyboard stands on in the grid: the one it last walked to, else today where today is in the
// month at the top, else that month's first.
const keyDay = shallowRef<Date | null>(null);

const tabDay = computed(() => {
  if (keyDay.value) return isoDate(keyDay.value);

  const today = todayDate();
  const month = focusMonth.value;

  return isoDate(toDate(today.year === month.year && today.month === month.month ? today : month.set({ day: 1 })));
});

// A month scrolled to while the keyboard is elsewhere gives the grid's way in back to that month.
watch(focusMonth, () => {
  if (!gridRef.value?.contains(document.activeElement)) keyDay.value = null;
});

function cellOf(day: Date): HTMLElement | null {
  return gridRef.value?.querySelector<HTMLElement>(`[role="gridcell"][data-date="${isoDate(day)}"]`) ?? null;
}

// The day walked to may stand in a week the list has not drawn: the list goes there first.
async function focusDay(day: Date): Promise<void> {
  if (day < firstWeek || day >= addWeeks(firstWeek, weeks.length)) return;

  keyDay.value = day;

  for (let attempt = 0; attempt < 10; attempt++) {
    const cell = cellOf(day);

    if (cell) {
      cell.focus();
      return;
    }

    getVirtualizer()?.scrollToIndex(indexOf(day), { align: "center" });
    await nextTick();
    await new Promise((resolve) => requestAnimationFrame(resolve));
  }
}

function onFocusin(event: FocusEvent): void {
  const target = event.target as HTMLElement;

  if (target.getAttribute("role") === "gridcell" && target.dataset.date) keyDay.value = dateFromISO(target.dataset.date);
}

// Keys that walk the days are the grid's while a day holds the focus, and go no further: the screen's own
// arrow keys step the span.
function onKeydown(event: KeyboardEvent): void {
  const cell = event.target as HTMLElement;

  if (cell.getAttribute("role") !== "gridcell" || !cell.dataset.date || event.altKey || event.ctrlKey || event.metaKey) return;

  const day = dateFromISO(cell.dataset.date);

  if (isDayKey(event.key)) {
    event.preventDefault();
    event.stopPropagation();
    void focusDay(stepDay(day, event.key));
  } else if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    event.stopPropagation();
    void navigateTo(pathFor(toCalendarDate(day), "day"));
  }
}
</script>
