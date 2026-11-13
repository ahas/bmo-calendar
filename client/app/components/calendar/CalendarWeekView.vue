<template>
  <!-- The day and week views: the days' names and the all-day row on top, the hours under them in a scroll
       of their own. A phone shows three days of the week around the day the screen stands on. Each day
       is named to a reader by its full date and its events; the arrow keys walk the days' names. The
       hour's height is the reader's own text size's (`HOUR_REM`). -->
  <div ref="view" class="flex min-h-0 flex-1 flex-col" :style="scale">
    <div class="shrink-0 border-b border-default [scrollbar-gutter:stable] overflow-hidden">
      <div class="grid" :style="gridStyle" @keydown="onHeadKeydown">
        <div />
        <ULink
          v-for="(day, index) in days"
          :key="day.getTime()"
          raw
          :to="pathFor(toCalendarDate(day), 'day')"
          class="focus-inset flex items-center justify-center gap-1 border-s border-default py-2 text-sm transition-colors hover:bg-elevated active:bg-accented"
          :class="isWeekend(day) && 'bg-elevated/50'"
          data-day-head
          :aria-label="dayNames[index]"
          :aria-current="isToday(day) ? 'date' : undefined"
        >
          <span class="text-muted">{{ formatWeekday(day) }}</span>
          <span class="flex size-7 items-center justify-center rounded-full font-semibold tabular-nums" :class="isToday(day) ? 'badge-ink bg-primary text-inverted' : 'text-highlighted'">
            {{ day.getDate() }}
          </span>
        </ULink>
      </div>

      <div role="group" class="grid border-t border-default" :style="{ ...gridStyle, gridTemplateRows: `repeat(${allDayLanes}, ${ALL_DAY_LANE_REM}rem)` }" :aria-label="$t('calendar.label.all_day')">
        <span class="row-span-full self-center pe-2 text-end text-sm text-muted" aria-hidden="true">{{ $t("calendar.label.all_day") }}</span>

        <div
          v-for="(day, index) in days"
          :key="`all-day-${day.getTime()}`"
          :data-date="isoDate(day)"
          class="row-span-full border-s border-default"
          :class="isWeekend(day) && 'bg-elevated/50'"
          :style="{ gridColumn: index + 2 }"
          @pointerdown="onGridPointerdown($event, { kind: 'allDay', day })"
          @dblclick="onGridDblclick($event, { kind: 'allDay', day })"
          @contextmenu="onGridContextmenu($event, { kind: 'allDay', day })"
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

    <UScrollArea ref="container" class="min-h-0 flex-1 snap-y snap-proximity scroll-pt-3 pb-[var(--bmo-create-room)] [scrollbar-gutter:stable]">
      <div data-week-grid class="grid" :style="gridStyle">
        <!-- The hours are a scale a reader does not need: every event says its own times. -->
        <div class="relative h-[calc(var(--calendar-hour)*24)]" aria-hidden="true">
          <span v-for="hour in hours" :key="hour" class="absolute end-2 -translate-y-1/2 text-sm tabular-nums text-muted" :style="{ top: minutesLength(hour * 60) }">
            {{ formatHour(hour) }}
          </span>
        </div>

        <CalendarDayColumn v-for="(day, index) in days" :key="day.getTime()" :day="day" :events="timedEvents[index]!" :first="index === 0" :loading="loading" :label="dayNames[index]!" />
      </div>
    </UScrollArea>
  </div>
</template>

<script setup lang="ts">
import { differenceInCalendarDays, isSameDay, isToday, isWeekend } from "date-fns";
import { useSwipe } from "@vueuse/core";
import { computed, nextTick, onMounted, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import { countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";

import CalendarDayColumn from "./CalendarDayColumn.vue";
import CalendarEventChip from "./CalendarEventChip.vue";
import CalendarEventDraft from "./CalendarEventDraft.vue";
import { dayName } from "../../domain/a11y";
import { eachDay, formatHour, formatWeekday, fromLocalISO, isoDate, toCalendarDate, toDate } from "../../domain/dates";
import { DRAFT_EVENT_KEY, HOUR_REM, layoutAllDay, layoutDay, minutesLength, rootFontPx } from "../../domain/layout";
import { spanKey } from "../../domain/store";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventDraft } from "../../composables/useEventDraft";
import { useGridHold } from "../../composables/useGridHold";
import { useEventMove } from "../../composables/useEventMove";

const viewRef = useTemplateRef<HTMLElement>("view");

const { t } = useI18n();
const { date, range, go, pathFor } = useCalendar();
const { eventsForDay, eventsForDays, pendingSpans } = useCalendarEvents();
const { draftEvent, onGridPointerdown, onGridDblclick, onGridContextmenu, registerHost } = useEventDraft();
const { movingKey, preview } = useEventMove();

registerHost();

// A held finger on an event opens its menu, and on an empty part of a day draws a new event there.
useGridHold(() => viewRef.value);

const narrow = useNarrow();

/** The grid's scale, which every length on it is read in (`minutesLength`). */
const scale = { "--calendar-hour": `${HOUR_REM}rem`, "--calendar-minute": "calc(var(--calendar-hour) / 60)" };

// A phone shows three days of the week, the day the screen stands on among them.
const days = computed(() => {
  const week = eachDay(range.value);

  if (week.length <= 3 || !narrow.value) return week;

  const start = Math.min(Math.max(differenceInCalendarDays(toDate(date.value), range.value.start), 0), week.length - 3);

  return week.slice(start, start + 3);
});

const gridStyle = computed(() => ({ gridTemplateColumns: `3.5rem repeat(${days.value.length}, minmax(0, 1fr))` }));

const dayNames = computed(() => {
  const today = new Date();

  return days.value.map((day) => dayName(t, day, eventsForDay(day).length, today, countText));
});

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

const ALL_DAY_LANE_REM = 1.75;

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

// The left and right arrows walk the days' names; past the first or the last they stop rather than step
// the span, which would take the name the focus stands on away.
function onHeadKeydown(event: KeyboardEvent): void {
  if ((event.key !== "ArrowLeft" && event.key !== "ArrowRight") || event.altKey || event.ctrlKey || event.metaKey) return;

  const heads = [...(event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>("[data-day-head]")];
  const index = heads.indexOf(event.target as HTMLElement);

  if (index === -1) return;

  event.preventDefault();
  event.stopPropagation();
  heads[index + (event.key === "ArrowLeft" ? -1 : 1)]?.focus();
}

// The day opens at seven in the morning, its hour line clear of the top edge.
onMounted(async () => {
  await nextTick();
  container.value?.$el.scrollTo({ top: 7 * HOUR_REM * rootFontPx() - 12 });
});
</script>
