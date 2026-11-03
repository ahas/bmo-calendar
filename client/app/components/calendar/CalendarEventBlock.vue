<template>
  <!-- An event on the time grid: dragged to move it, its foot dragged to stretch it, pressed to open its
       form. Its calendar's colour is its fill and stripe, its realm its mark. -->
  <CalendarEventPopover :event="event" :disabled="suppressed" :anchored="anchored">
    <button
      type="button"
      data-event
      class="absolute flex touch-none select-none flex-col items-start overflow-hidden rounded-sm py-1 ps-3 pe-2 text-start text-sm text-highlighted transition-colors"
      :class="[EVENT_FILL[color], dragging ? 'z-20' : 'z-5']"
      :style="style"
      :aria-label="`${event.title}, ${previewTimes}`"
      @click.stop
      @pointerdown="onPointerdown"
      @pointermove="onPointermove"
      @pointerup="onPointerup"
      @pointercancel="onPointercancel"
    >
      <span class="absolute inset-y-1 start-1 w-1 rounded-full" :class="CALENDAR_DOT[color]" />

      <span class="flex w-full min-w-0 items-center gap-1">
        <UIcon :name="REALM_ICON[event.realm]" class="size-3.5 shrink-0 text-muted" />
        <span class="truncate font-medium">{{ event.title }}</span>
        <UIcon v-if="repeats" name="i-material-symbols-repeat" class="size-3.5 shrink-0 text-muted" />
      </span>
      <span v-if="!compact || dragging" class="w-full truncate tabular-nums text-toned">{{ previewTimes }}</span>

      <span data-resize-handle class="absolute inset-x-0 bottom-0 h-2 cursor-ns-resize" />
    </button>
  </CalendarEventPopover>
</template>

<script setup lang="ts">
import { addMinutes } from "date-fns";
import { computed } from "vue";

import CalendarEventPopover from "./CalendarEventPopover.vue";
import { CALENDAR_DOT, EVENT_FILL, REALM_ICON } from "../../domain/colors";
import { formatTime, fromLocalISO, toLocalISO } from "../../domain/dates";
import { MIN_EVENT_MINUTES, PX_PER_MINUTE, SNAP_MINUTES, eventBlockStyle } from "../../domain/layout";
import { repeating } from "../../domain/store";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventChange } from "../../composables/useEventChange";
import { useEventDrag } from "../../composables/useEventDrag";

import type { PositionedEvent } from "../../domain/layout";

const props = withDefaults(defineProps<{ positioned: PositionedEvent; anchored?: boolean }>(), { anchored: true });

const { calendarFor } = useCalendarEvents();
const { changeEvent } = useEventChange();

const event = computed(() => props.positioned.event);
const color = computed(() => calendarFor(event.value)?.color ?? "primary");
const repeats = computed(() => repeating(event.value));

const { dragging, suppressed, mode, deltaMinutes, deltaX, onPointerdown, onPointermove, onPointerup, onPointercancel } = useEventDrag(event, {
  onCommit(start, end) {
    void changeEvent(event.value, { ...event.value, start: toLocalISO(start), end: toLocalISO(end) });
  }
});

const style = computed(() => {
  const height =
    dragging.value && mode.value === "resize"
      ? Math.max(props.positioned.height + deltaMinutes.value * PX_PER_MINUTE, MIN_EVENT_MINUTES * PX_PER_MINUTE)
      : props.positioned.height;

  return {
    ...eventBlockStyle(props.positioned, height),
    transform: dragging.value && mode.value === "move" ? `translate(${deltaX.value}px, ${deltaMinutes.value * PX_PER_MINUTE}px)` : undefined
  };
});

// While it is dragged it names the times it would land on.
const previewTimes = computed(() => {
  const shift = dragging.value && mode.value === "move" ? deltaMinutes.value : 0;
  const start = addMinutes(fromLocalISO(event.value.start), shift);
  const end = addMinutes(fromLocalISO(event.value.end), dragging.value ? (mode.value === "resize" ? deltaMinutes.value : shift) : 0);

  return `${formatTime(start)}~${formatTime(end > start ? end : addMinutes(start, SNAP_MINUTES))}`;
});

const compact = computed(() => props.positioned.height < 44);
</script>
