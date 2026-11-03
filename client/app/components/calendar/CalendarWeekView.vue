<template>
  <!-- The day and week views: the days' names and the all-day row on top, the hours under them in a scroll
       of their own. A phone shows three days of the week around the day the screen stands on. -->
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="shrink-0 border-b border-default [scrollbar-gutter:stable] overflow-hidden">
      <div class="grid" :style="gridStyle">
        <div />
        <ULink raw
          v-for="day in days"
          :key="day.getTime()"
          :to="pathFor(toCalendarDate(day), 'day')"
          class="flex items-center justify-center gap-1 border-s border-default py-2 text-sm transition-colors hover:bg-elevated active:bg-accented"
          :class="isWeekend(day) && 'bg-elevated/50'"
        >
          <span class="text-muted">{{ formatWeekday(day) }}</span>
          <span class="flex size-7 items-center justify-center rounded-full font-semibold tabular-nums" :class="isToday(day) ? 'bg-primary text-inverted' : 'text-highlighted'">
            {{ day.getDate() }}
          </span>
        </ULink>
      </div>

      <div class="grid border-t border-default" :style="{ ...gridStyle, gridTemplateRows: `repeat(${allDayLanes}, ${ALL_DAY_LANE_HEIGHT}px)` }">
        <span class="row-span-full self-center pe-2 text-end text-sm text-muted">{{ $t("calendar.label.all_day") }}</span>

        <div
          v-for="(day, index) in days"
          :key="`all-day-${day.getTime()}`"
          :data-date="isoDate(day)"
          class="row-span-full border-s border-default"
          :class="isWeekend(day) && 'bg-elevated/50'"
          :style="{ gridColumn: index + 2 }"
          @pointerdown="onGridPointerdown($event, { kind: 'allDay', day })"
          @dblclick="onGridDblclick($event, { kind: 'allDay', day })"
        />

        <template v-for="{ event, colStart, colSpan, lane } in allDayEvents" :key="event.key">
          <CalendarEventDraft
            v-if="event.key === DRAFT_EVENT_KEY"
            variant="chip"
            anchored
            class="mx-1 mt-1"
            :style="{ gridColumn: `${colStart + 2} / span ${colSpan}`, gridRow: lane + 1 }"
          />
          <CalendarEventChip v-else :event="event" class="mx-1 mt-1" :style="{ gridColumn: `${colStart + 2} / span ${colSpan}`, gridRow: lane + 1 }" />
        </template>
      </div>
    </div>

    <UScrollArea ref="container" class="min-h-0 flex-1 snap-y snap-proximity scroll-pt-3 [scrollbar-gutter:stable]">
      <div data-week-grid class="grid" :style="gridStyle">
        <div class="relative" :style="{ height: `${24 * HOUR_HEIGHT}px` }">
          <span v-for="hour in hours" :key="hour" class="absolute end-2 -translate-y-1/2 text-sm tabular-nums text-muted" :style="{ top: `${hour * HOUR_HEIGHT}px` }">
            {{ formatHour(hour) }}
          </span>
        </div>

        <CalendarDayColumn v-for="(day, index) in days" :key="day.getTime()" :day="day" :events="timedEvents[index]!" :first="index === 0" :loading="loading" />
      </div>
    </UScrollArea>
  </div>
</template>

<script setup lang="ts">
import { differenceInCalendarDays, isSameDay, isToday, isWeekend } from "date-fns";
import { useSwipe } from "@vueuse/core";
import { computed, nextTick, onMounted, useTemplateRef } from "vue";
import { useNarrow } from "bmo-ui/app/domain/screen";

import CalendarDayColumn from "./CalendarDayColumn.vue";
import CalendarEventChip from "./CalendarEventChip.vue";
import CalendarEventDraft from "./CalendarEventDraft.vue";
import { eachDay, formatHour, formatWeekday, fromLocalISO, isoDate, toCalendarDate, toDate } from "../../domain/dates";
import { DRAFT_EVENT_KEY, HOUR_HEIGHT, layoutAllDay, layoutDay } from "../../domain/layout";
import { spanKey } from "../../domain/store";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventDraft } from "../../composables/useEventDraft";
import { useEventMove } from "../../composables/useEventMove";

const { date, range, go, pathFor } = useCalendar();
const { eventsForDay, eventsForDays, pendingSpans } = useCalendarEvents();
const { draftEvent, onGridPointerdown, onGridDblclick, registerHost } = useEventDraft();
const { movingKey, preview } = useEventMove();

registerHost();

const narrow = useNarrow();

// A phone shows three days of the week, the day the screen stands on among them.
const days = computed(() => {
  const week = eachDay(range.value);

  if (week.length <= 3 || !narrow.value) return week;

  const start = Math.min(Math.max(differenceInCalendarDays(toDate(date.value), range.value.start), 0), week.length - 3);

  return week.slice(start, start + 3);
});

const gridStyle = computed(() => ({ gridTemplateColumns: `3.5rem repeat(${days.value.length}, minmax(0, 1fr))` }));

// The draft rides among the day's events, so the day makes room for it as it would for an event.
const timedEvents = computed(() => {
  const drafted = draftEvent.value && !draftEvent.value.allDay ? draftEvent.value : null;

  return days.value.map((day) =>
    layoutDay([...eventsForDay(day).filter((event) => !event.allDay), ...(drafted && isSameDay(fromLocalISO(drafted.start), day) ? [drafted] : [])], day)
  );
});

const allDayEvents = computed(() =>
  layoutAllDay(
    [
      ...eventsForDays(days.value).filter((event) => event.allDay && event.key !== movingKey.value),
      ...(draftEvent.value?.allDay ? [draftEvent.value] : []),
      ...(preview.value?.allDay ? [preview.value] : [])
    ],
    days.value
  )
);

// The skeleton stands only while the span is first read and nothing of it is drawn yet.
const loading = computed(
  () => pendingSpans.value.some((span) => spanKey(span) === spanKey(range.value)) && !allDayEvents.value.length && timedEvents.value.every((day) => !day.length)
);

const hours = Array.from({ length: 23 }, (_, index) => index + 1);

const ALL_DAY_LANE_HEIGHT = 28;

// The row always stands, empty or not, so the grid does not drop when the first all-day event comes.
const allDayLanes = computed(() => allDayEvents.value.reduce((lanes, { lane }) => Math.max(lanes, lane + 1), 1));

const container = useTemplateRef<{ $el: HTMLElement }>("container");

// A sideways swipe steps back or forward a span; one mostly up or down stays a scroll.
let onEvent = false;

const { lengthX, lengthY } = useSwipe(() => container.value?.$el, {
  threshold: 50,
  onSwipeStart(event) {
    onEvent = !!(event.target as Element).closest("[data-event]");
  },
  onSwipeEnd(_event, direction) {
    if (onEvent || draftEvent.value || Math.abs(lengthX.value) < 2 * Math.abs(lengthY.value)) return;

    if (direction === "left") go("right");
    else if (direction === "right") go("left");
  }
});

// The day opens at seven in the morning, its hour line clear of the top edge.
onMounted(async () => {
  await nextTick();
  container.value?.$el.scrollTo({ top: 7 * HOUR_HEIGHT - 12 });
});
</script>
