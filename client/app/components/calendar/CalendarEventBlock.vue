<template>
  <!-- An event on the time grid: dragged by a mouse to move it, its foot dragged to stretch it, pressed to
       open its form. A received schedule is a link to the item it stands for instead, never dragged or
       stretched. Its calendar's or source's colour is its fill and stripe, its realm its mark; its words keep
       the page's own text colour. Where it stands narrow it sheds its marks and its end time before its title. -->
  <CalendarEventPopover :event="event" :disabled="suppressed" :anchored="anchored">
    <component
      :is="opens ? ULink : 'button'"
      v-bind="opens ?? { type: 'button' }"
      data-event
      class="@container focus-inset absolute flex select-none flex-col items-start overflow-hidden rounded-sm py-1 ps-3 pe-2 text-start text-sm text-highlighted transition-colors"
      :class="[EVENT_FILL[color], dragging ? 'z-20' : 'z-5']"
      :style="style"
      :aria-label="name"
      @click.stop
      v-on="event.origin ? {} : { pointerdown: onPointerdown, pointermove: onPointermove, pointerup: onPointerup, pointercancel: onPointercancel }"
    >
      <span class="absolute inset-y-1 start-1 w-1 rounded-full" :class="CALENDAR_DOT[color]" />

      <span class="flex w-full min-w-0 items-center gap-1">
        <UIcon :name="REALM_ICON[event.realm]" class="size-3.5 shrink-0 text-muted @max-[4rem]:hidden" />
        <UIcon v-if="event.origin" :name="RECEIVED_ICON" class="size-3.5 shrink-0 text-muted @max-[4rem]:hidden" />
        <span class="truncate font-medium">{{ event.title }}</span>
        <UIcon v-if="repeats" name="i-material-symbols-repeat" class="size-3.5 shrink-0 text-muted @max-[4rem]:hidden" />
      </span>
      <span v-if="!compact || dragging" class="w-full truncate tabular-nums text-toned">{{ times.start }}<span class="@max-[5.25rem]:hidden">~{{ times.end }}</span></span>

      <span v-if="!event.origin" data-resize-handle class="absolute inset-x-0 bottom-0 h-2 cursor-ns-resize" />
    </component>
  </CalendarEventPopover>
</template>

<script setup lang="ts">
import ULink from "@nuxt/ui/components/Link.vue";
import { addMinutes } from "date-fns";
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import CalendarEventPopover from "./CalendarEventPopover.vue";
import { eventName } from "../../domain/a11y";
import { CALENDAR_DOT, EVENT_FILL, RECEIVED_ICON, REALM_ICON } from "../../domain/colors";
import { formatTime, fromLocalISO, toLocalISO } from "../../domain/dates";
import { COMPACT_MINUTES, MIN_EVENT_MINUTES, SNAP_MINUTES, eventBlockStyle, minutesLength } from "../../domain/layout";
import { originLink } from "../../domain/origin";
import { repeating } from "../../domain/store";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventChange } from "../../composables/useEventChange";
import { useEventDrag } from "../../composables/useEventDrag";

import type { PositionedEvent } from "../../domain/layout";

const props = withDefaults(defineProps<{ positioned: PositionedEvent; anchored?: boolean }>(), { anchored: true });

const { t } = useI18n();
const { layerOf } = useCalendarEvents();
const { changeEvent } = useEventChange();

const event = computed(() => props.positioned.event);
const color = computed(() => layerOf(event.value)?.color ?? "primary");
const repeats = computed(() => repeating(event.value));
const name = computed(() => eventName(t, event.value, layerOf(event.value)?.name));
const opens = computed(() => originLink(event.value.origin));

const { dragging, suppressed, mode, deltaMinutes, deltaX, onPointerdown, onPointermove, onPointerup, onPointercancel } = useEventDrag(event, {
  onCommit(start, end) {
    void changeEvent(event.value, { ...event.value, start: toLocalISO(start), end: toLocalISO(end) });
  }
});

const style = computed(() => {
  const minutes = dragging.value && mode.value === "resize" ? Math.max(props.positioned.minutes + deltaMinutes.value, MIN_EVENT_MINUTES) : props.positioned.minutes;

  return {
    ...eventBlockStyle(props.positioned, minutes),
    transform: dragging.value && mode.value === "move" ? `translate(${deltaX.value}px, ${minutesLength(deltaMinutes.value)})` : undefined
  };
});

// While it is dragged it names the times it would land on.
const times = computed(() => {
  const shift = dragging.value && mode.value === "move" ? deltaMinutes.value : 0;
  const start = addMinutes(fromLocalISO(event.value.start), shift);
  const end = addMinutes(fromLocalISO(event.value.end), dragging.value ? (mode.value === "resize" ? deltaMinutes.value : shift) : 0);

  return { start: formatTime(start), end: formatTime(end > start ? end : addMinutes(start, SNAP_MINUTES)) };
});

const compact = computed(() => props.positioned.minutes < COMPACT_MINUTES);
</script>
