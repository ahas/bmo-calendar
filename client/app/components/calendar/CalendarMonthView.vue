<template>
  <!-- The month view: the weekdays' names, then the weeks scrolling on without end either way, read in
       spans of six weeks as they come near. Where the scroll comes to rest, the month at its top becomes
       the screen's address. -->
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="grid h-9 shrink-0 grid-cols-7 overflow-hidden border-b border-default [scrollbar-gutter:stable]">
      <span v-for="(weekday, index) in weekdays" :key="weekday" class="flex items-center justify-end border-default pe-2 text-sm text-muted" :class="[index !== 0 && 'border-s', isWeekendIndex(index) && 'bg-elevated/50']">
        {{ weekday }}
      </span>
    </div>

    <UScrollArea
      ref="scrollArea"
      :items="weeks"
      :virtualize="{ estimateSize: ROW_HEIGHT, skipMeasurement: true, overscan: 4, initialOffset }"
      :ui="{ item: 'snap-start' }"
      class="min-h-0 flex-1 snap-y snap-proximity [scrollbar-gutter:stable]"
      @scroll="onScroll"
    >
      <template #default="{ item }">
        <CalendarMonthWeek :week-start="item" :loading="rowLoading(item)" :style="{ height: `${ROW_HEIGHT}px` }" />
      </template>
    </UScrollArea>
  </div>
</template>

<script setup lang="ts">
import { addDays, addWeeks, differenceInCalendarWeeks, startOfWeek } from "date-fns";
import { useEventListener } from "@vueuse/core";
import { onUnmounted, ref, shallowRef, unref, useTemplateRef, watch } from "vue";
import { navigateTo } from "nuxt/app";

import CalendarMonthWeek from "./CalendarMonthWeek.vue";
import { WEEKS_AROUND, WEEK_STARTS_ON, formatWeekday, monthRange, toCalendarDate } from "../../domain/dates";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventDraft } from "../../composables/useEventDraft";

import type { CalendarDate } from "@internationalized/date";
import type { DateRange } from "../../../shared/types";

const ROW_HEIGHT = 144;
const CHUNK_WEEKS = 6;

const { date, pathFor, visibleMonth } = useCalendar();
const { loadSpan, showSpans, pendingSpans } = useCalendarEvents();
const { draft, pendingScroll, registerHost } = useEventDraft();

registerHost();

const scrollElement = shallowRef<Element | null>(null);

const firstWeek = addWeeks(startOfWeek(new Date(), { weekStartsOn: WEEK_STARTS_ON }), -WEEKS_AROUND);
const weeks = Array.from({ length: WEEKS_AROUND * 2 + 1 }, (_, index) => addWeeks(firstWeek, index));
const weekdays = Array.from({ length: 7 }, (_, index) => formatWeekday(addDays(firstWeek, index)));

function isWeekendIndex(index: number): boolean {
  const day = addDays(firstWeek, index).getDay();

  return day === 0 || day === 6;
}

function indexOf(day: Date): number {
  return Math.min(weeks.length - 1, Math.max(0, differenceInCalendarWeeks(day, firstWeek, { weekStartsOn: WEEK_STARTS_ON })));
}

// The list is drawn from the month it opens on, from its very first pass.
function initialOffset(): number {
  return indexOf(monthRange(date.value).start) * ROW_HEIGHT;
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

  getVirtualizer()?.scrollToIndex(indexOf(draft.value.start), { align: "center", behavior: "smooth" });
  loadVisibleChunks();
});

function scrollToMonth(target: CalendarDate, smooth = false): void {
  getVirtualizer()?.scrollToIndex(indexOf(monthRange(target).start), { align: "start", behavior: smooth ? "smooth" : "auto" });
}

// The month whose last day of the top week stands at the top of the scroll.
function dockedMonth(): CalendarDate | null {
  const offset = scrollElement.value?.scrollTop;

  if (offset == null) return null;

  const top = Math.min(weeks.length - 1, Math.max(0, Math.floor(offset / ROW_HEIGHT)));

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
</script>
