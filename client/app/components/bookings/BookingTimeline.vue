<template>
  <!-- The day view of the booking screen (docs/design.md, 「The console」): a row for each resource of the kind
       shown, grouped by place, across the hours of the day, its bookings standing on it, each hour wide enough
       that an hour's booking names its title, scrolled sideways where the hours outrun the screen. A press on a
       free span of a row, or a drag across one, offers that span for a new booking; a span is never drawn over
       a booking it would meet. A finger scrolls the rows, and a held finger stands for the press. Where the
       reader may not book — another company than the reader's own is read — nothing is offered. -->
  <div ref="scroller" class="min-h-0 flex-1 overflow-auto pb-[var(--bmo-create-room)] [--hour:4rem] sm:[--hour:6rem]">
    <div class="w-max min-w-full">
      <div class="sticky top-0 z-10 flex border-b border-default bg-default" aria-hidden="true">
        <div class="sticky start-0 z-10 w-32 shrink-0 border-e border-default bg-default sm:w-48" />
        <div class="relative h-9 shrink-0 pointer-coarse:h-10" :style="{ width: trackWidth }">
          <span v-for="hour in HOURS" :key="hour" class="absolute inset-y-0 flex items-center ps-2 text-sm tabular-nums text-muted" :style="{ insetInlineStart: hourAt(hour) }">
            {{ formatHour(hour) }}
          </span>
        </div>
      </div>

      <template v-if="props.loading && !props.groups.length">
        <div v-for="row in 4" :key="row" class="flex h-12 border-b border-default" role="status" :aria-label="$t('calendar.status.loading')">
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
            data-track
            class="relative h-12 shrink-0 bg-[repeating-linear-gradient(to_right,var(--ui-border)_0_1px,transparent_1px_var(--hour))]"
            :style="{ width: trackWidth }"
            role="group"
            :aria-label="resource.name"
            @pointerdown="onPointerdown($event, resource.id)"
            @contextmenu="onContextmenu($event, resource.id)"
          >
            <button
              v-for="block in blocksOf(resource.id)"
              :key="bookingKey(block.booking)"
              type="button"
              data-booking
              class="focus-inset absolute inset-y-1 flex min-w-0 select-none flex-col justify-center overflow-hidden rounded-sm ps-3 pe-1 text-start text-sm text-highlighted transition-colors"
              :class="EVENT_FILL[STATE_TINT[block.booking.state]]"
              :style="{ insetInlineStart: share(block.from), width: `calc(${share(block.to - block.from)} - 2px)` }"
              :aria-label="blockName(block.booking, resource.name)"
              @click="emit('open', block.booking)"
            >
              <span class="absolute inset-y-1 start-1 w-1 rounded-full" :class="CALENDAR_DOT[STATE_TINT[block.booking.state]]" />
              <span class="w-full truncate font-medium">{{ block.booking.title }}</span>
              <span class="w-full truncate tabular-nums text-toned">{{ whenOf(block.booking) }}</span>
            </button>

            <div
              v-if="ghostOn(resource.id)"
              class="pointer-events-none absolute inset-y-1 rounded-sm border-2 border-dashed border-primary bg-primary/10"
              :style="{ insetInlineStart: share(ghostShare!.from), width: `calc(${share(ghostShare!.to - ghostShare!.from)} - 2px)` }"
            />
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { addDays } from "date-fns";
import { useEventListener } from "@vueuse/core";
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";

import ResourceCell from "./ResourceCell.vue";
import { eventWhen } from "../../domain/a11y";
import { STATE_LABELS, STATE_TINT, bookingKey, dragSpan, freeGap, momentAt, pressSpan, rowBlocks } from "../../domain/bookings";
import { CALENDAR_DOT, EVENT_FILL } from "../../domain/colors";
import { formatHour } from "../../domain/dates";
import { DRAG_THRESHOLD } from "../../domain/layout";
import { useGridHold } from "../../composables/useGridHold";
import { usePeople } from "../../composables/usePeople";

import type { BookingDraft } from "./BookingForm.vue";
import type { PlaceGroup, RowBlock } from "../../domain/bookings";
import type { Booking, DateRange } from "../../../shared/types";

const HOURS = Array.from({ length: 24 }, (_, hour) => hour);
/** The hour the day view opens scrolled to, so the working day stands in view. */
const FIRST_HOUR = 8;
/** What a press offers, in minutes, and what a drag snaps to. */
const PRESS_MINUTES = 60;
const SNAP_MINUTES = 30;
/** How far short of the first hour the day view opens, so that hour's label stands clear of the resources. */
const OPEN_MARGIN_PX = 8;

const props = defineProps<{
  day: Date;
  groups: PlaceGroup[];
  bookings: Booking[];
  loading?: boolean;
  /** Whether a press or a drag offers a span for a new booking. */
  bookable?: boolean;
  /** A new booking's span while its form stands open. */
  draft?: BookingDraft | null;
}>();

const emit = defineEmits<{
  book: [draft: BookingDraft];
  open: [booking: Booking];
}>();

const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const { t } = useI18n();
const { knownName } = usePeople();

// The span being drawn by a drag, on its resource's row.
const drawn = ref<BookingDraft | null>(null);

const rowSpan = computed<DateRange>(() => ({ start: props.day, end: addDays(props.day, 1) }));
const trackWidth = "calc(var(--hour) * 24)";
const ghost = computed(() => drawn.value ?? props.draft ?? null);
const ghostShare = computed(() => {
  if (!ghost.value) return null;

  const whole = rowSpan.value.end.getTime() - rowSpan.value.start.getTime();
  const from = (ghost.value.start.getTime() - rowSpan.value.start.getTime()) / whole;
  const to = (ghost.value.end.getTime() - rowSpan.value.start.getTime()) / whole;

  return { from: Math.max(0, from), to: Math.min(1, to) };
});

// Each resource's blocks, laid once per change of the bookings or the span.
const blocks = computed(() => {
  const laid = new Map<string, RowBlock[]>();

  for (const group of props.groups) for (const resource of group.resources) laid.set(resource.id, rowBlocks(props.bookings, resource.id, rowSpan.value));

  return laid;
});

function blocksOf(resourceId: string): RowBlock[] {
  return blocks.value.get(resourceId) ?? [];
}

function ghostOn(resourceId: string): boolean {
  return ghost.value?.resourceId === resourceId && !!ghostShare.value && ghostShare.value.to > ghostShare.value.from;
}

function hourAt(hours: number): string {
  return `calc(var(--hour) * ${hours})`;
}

function share(fraction: number): string {
  return `${fraction * 100}%`;
}

function whenOf(booking: Booking): string {
  return eventWhen(t, booking);
}

function blockName(booking: Booking, resource: string): string {
  return [booking.title, whenOf(booking), resource, knownName(booking.booker), t(STATE_LABELS[booking.state])].filter(Boolean).join(", ");
}

function momentUnder(clientX: number, track: HTMLElement): Date {
  const rect = track.getBoundingClientRect();

  return momentAt(rowSpan.value, (clientX - rect.left) / rect.width);
}

// ---- A press, a drag and a held finger --------------------------------------------------------------

interface Gesture {
  resourceId: string;
  track: HTMLElement;
  x: number;
  anchor: Date;
  gap: DateRange;
  moved: boolean;
}

let gesture: Gesture | null = null;

function onBlock(event: Event): boolean {
  return !!(event.target as HTMLElement | null)?.closest("[data-booking]");
}

/** The span a press at a point of a row offers, or none where a booking holds the point. */
function pressed(resourceId: string, track: HTMLElement, clientX: number): BookingDraft | null {
  const at = momentUnder(clientX, track);
  const gap = freeGap(props.bookings, resourceId, at, rowSpan.value);

  return gap ? { resourceId, ...pressSpan(at, gap, SNAP_MINUTES, PRESS_MINUTES) } : null;
}

function onPointerdown(event: PointerEvent, resourceId: string): void {
  // A finger dragging over the rows is scrolling them; a held finger is the press (`onContextmenu`).
  if (!props.bookable || event.button !== 0 || event.pointerType === "touch" || onBlock(event)) return;

  const track = event.currentTarget as HTMLElement;
  const anchor = momentUnder(event.clientX, track);
  const gap = freeGap(props.bookings, resourceId, anchor, rowSpan.value);

  if (!gap) return;

  event.preventDefault();
  gesture = { resourceId, track, x: event.clientX, anchor, gap, moved: false };
}

useEventListener("pointermove", (event: PointerEvent) => {
  if (!gesture) return;

  if (!gesture.moved && Math.abs(event.clientX - gesture.x) < DRAG_THRESHOLD) return;

  gesture.moved = true;
  drawn.value = { resourceId: gesture.resourceId, ...dragSpan(gesture.anchor, momentUnder(event.clientX, gesture.track), gesture.gap, SNAP_MINUTES) };
});

function endGesture(event: PointerEvent | null): void {
  const held = gesture;

  gesture = null;

  if (!held) return;

  const offered = held.moved ? drawn.value : event ? pressed(held.resourceId, held.track, event.clientX) : null;

  drawn.value = null;

  if (offered && offered.end > offered.start) emit("book", offered);
}

useEventListener("pointerup", (event: PointerEvent) => endGesture(event));
useEventListener("pointercancel", () => endGesture(null));

useEventListener("keydown", (event: KeyboardEvent) => {
  if (event.key === "Escape" && gesture) {
    gesture = null;
    drawn.value = null;
  }
});

// A right press, or the menu press a held finger makes, on a free span offers it as a press does.
function onContextmenu(event: MouseEvent, resourceId: string): void {
  if (!props.bookable || onBlock(event)) return;

  event.preventDefault();

  const offered = pressed(resourceId, event.currentTarget as HTMLElement, event.clientX);

  if (offered) emit("book", offered);
}

useGridHold(() => scrollerRef.value);

// The day view opens on the working day rather than at midnight.
async function scrollToDay(): Promise<void> {
  await nextTick();

  const scroller = scrollerRef.value;
  const track = scroller?.querySelector<HTMLElement>("[data-track]");

  if (!scroller || !track) return;

  scroller.scrollLeft = Math.max(0, (track.offsetWidth / 24) * FIRST_HOUR - OPEN_MARGIN_PX);
}

watch(
  () => [props.day.getTime(), props.groups.length > 0] as const,
  () => void scrollToDay()
);

onMounted(() => void scrollToDay());
</script>
