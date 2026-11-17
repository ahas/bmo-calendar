<template>
  <!-- The week view of the booking screen (docs/design.md, 「The console」): a row for each resource of the kind
       shown, grouped by place, across the seven days of the week, each day's cell listing that day's bookings
       by start, each with its time and title. A press on a day's cell, or a held finger on it, opens the
       booking form on that day's first free hour of that resource; a booking is pressed to open it. Where the
       reader may not book — another company than the reader's own is read — a cell offers nothing. -->
  <div ref="scroller" class="min-h-0 flex-1 overflow-auto pb-[var(--bmo-create-room)]">
    <div class="w-max min-w-full">
      <div class="sticky top-0 z-10 flex border-b border-default bg-default" aria-hidden="true">
        <div class="sticky start-0 z-10 w-32 shrink-0 border-e border-default bg-default sm:w-48" />
        <span
          v-for="day in props.days"
          :key="day.getTime()"
          class="flex h-9 w-28 shrink-0 items-center gap-1 border-e border-default px-2 text-sm last:border-e-0 sm:min-w-32 sm:flex-1 pointer-coarse:h-10"
          :class="isToday(day) ? 'font-semibold text-highlighted' : 'text-muted'"
        >
          <span class="tabular-nums">{{ formatMonthDay(day) }}</span>
          <span>{{ formatWeekday(day) }}</span>
        </span>
      </div>

      <template v-if="props.loading && !props.groups.length">
        <div v-for="row in 4" :key="row" class="flex h-16 border-b border-default" role="status" :aria-label="$t('calendar.status.loading')">
          <div class="sticky start-0 flex w-32 shrink-0 items-center border-e border-default bg-default px-3 sm:w-48">
            <USkeleton class="h-4 w-24 rounded-sm" />
          </div>
        </div>
      </template>

      <section v-for="group in props.groups" :key="group.place" :aria-label="group.place || $t('calendar.label.no_place')">
        <div class="flex h-8 items-center border-b border-default bg-elevated/50">
          <h3 class="sticky start-0 px-3 text-sm font-semibold text-highlighted">{{ group.place || $t("calendar.label.no_place") }}</h3>
        </div>

        <div v-for="resource in group.resources" :key="resource.id" class="flex border-b border-default">
          <ResourceCell :resource="resource" />

          <div
            v-for="(day, index) in props.days"
            :key="day.getTime()"
            data-week-cell
            role="group"
            :aria-label="cellName(resource, day, index)"
            class="flex min-h-16 w-28 shrink-0 flex-col gap-1 border-e border-default p-1 last:border-e-0 sm:min-w-32 sm:flex-1"
            :class="[isWeekend(day) && 'bg-elevated/50', props.bookable && 'cursor-pointer transition-colors hover:bg-elevated active:bg-accented']"
            @pointerdown="onPointerdown"
            @click="onClick($event, resource.id, day)"
            @contextmenu="onContextmenu($event, resource.id, day)"
          >
            <button
              v-for="booking in cellOf(resource.id, index)"
              :key="bookingKey(booking)"
              type="button"
              data-booking
              class="focus-inset flex h-6 min-w-0 select-none items-center gap-1 rounded-sm px-1.5 text-start text-sm text-highlighted transition-colors"
              :class="EVENT_FILL[STATE_TINT[booking.state]]"
              :aria-label="bookingName(booking, resource.name)"
              @click.stop="emit('open', booking)"
            >
              <span class="size-2 shrink-0 rounded-full" :class="CALENDAR_DOT[STATE_TINT[booking.state]]" />
              <span class="shrink-0 tabular-nums text-toned">{{ startOf(booking) }}</span>
              <span class="truncate font-medium">{{ booking.title }}</span>
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { isToday, isWeekend } from "date-fns";
import { computed, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import { countText } from "bmo-ui/app/domain/numbers";

import ResourceCell from "./ResourceCell.vue";
import { eventWhen } from "../../domain/a11y";
import { STATE_LABELS, STATE_TINT, bookingKey, dayBookings, firstFreeHour } from "../../domain/bookings";
import { CALENDAR_DOT, EVENT_FILL } from "../../domain/colors";
import { formatFullDate, formatMonthDay, formatTime, formatWeekday, fromLocalISO } from "../../domain/dates";
import { useGridHold } from "../../composables/useGridHold";
import { usePeople } from "../../composables/usePeople";

import type { BookingDraft } from "./BookingForm.vue";
import type { PlaceGroup } from "../../domain/bookings";
import type { Booking, Resource } from "../../../shared/types";

const props = defineProps<{
  days: Date[];
  groups: PlaceGroup[];
  bookings: Booking[];
  loading?: boolean;
  /** Whether a press on a day's cell offers a span for a new booking. */
  bookable?: boolean;
}>();

const emit = defineEmits<{
  book: [draft: BookingDraft];
  open: [booking: Booking];
}>();

const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const { t } = useI18n();
const { knownName } = usePeople();

// Each resource's bookings by day, laid once per change of the bookings or the week.
const cells = computed(() => {
  const laid = new Map<string, Booking[][]>();

  for (const group of props.groups) for (const resource of group.resources) laid.set(resource.id, props.days.map((day) => dayBookings(props.bookings, resource.id, day)));

  return laid;
});

function cellOf(resourceId: string, index: number): Booking[] {
  return cells.value.get(resourceId)?.[index] ?? [];
}

function startOf(booking: Booking): string {
  return booking.allDay ? t("calendar.label.all_day") : formatTime(fromLocalISO(booking.start));
}

function cellName(resource: Resource, day: Date, index: number): string {
  const held = cellOf(resource.id, index).length;

  return [resource.name, formatFullDate(day), held ? t("calendar.aria.bookings_count", { n: countText(held) }) : t("calendar.aria.bookings_none")].join(", ");
}

function bookingName(booking: Booking, resource: string): string {
  return [booking.title, eventWhen(t, booking), resource, knownName(booking.booker), t(STATE_LABELS[booking.state])].filter(Boolean).join(", ");
}

// A press on the cell, not on a booking in it, offers the day's first free hour; on today none already past.
function offer(event: Event, resourceId: string, day: Date): void {
  if (!props.bookable || (event.target as HTMLElement | null)?.closest("[data-booking]")) return;

  const now = new Date();
  const span = firstFreeHour(props.bookings, resourceId, day, isToday(day) ? now : undefined);

  if (span) emit("book", { resourceId, ...span });
}

// A finger's tap scrolls and reads; on a phone the held finger is the press (`onContextmenu`).
let touched = false;

function onPointerdown(event: PointerEvent): void {
  touched = event.pointerType === "touch";
}

function onClick(event: MouseEvent, resourceId: string, day: Date): void {
  if (!touched) offer(event, resourceId, day);
}

// A right press, or the menu press a held finger makes, offers as a press does.
function onContextmenu(event: MouseEvent, resourceId: string, day: Date): void {
  if ((event.target as HTMLElement | null)?.closest("[data-booking]")) return;

  event.preventDefault();
  offer(event, resourceId, day);
}

useGridHold(() => scrollerRef.value);
</script>
