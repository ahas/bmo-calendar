<template>
  <!-- An event in a month cell or the all-day row: a bar for an all-day event, a dot and a title for a
       timed one. Dragged a day at a time; pressed to open its form. -->
  <CalendarEventPopover v-slot="{ open }" :event="event" :disabled="suppressed" :anchored="anchored">
    <button
      v-bind="$attrs"
      type="button"
      data-event
      class="flex h-6 min-w-0 select-none items-center gap-1 rounded-sm px-1.5 text-start text-sm text-highlighted transition-colors"
      :class="event.allDay ? EVENT_FILL[color] : ['hover:bg-elevated active:bg-accented data-active:bg-elevated', CHIP_COMPACT_FILL[color]]"
      :data-active="open || moving || undefined"
      :aria-label="event.allDay ? event.title : `${event.title}, ${formatTime(fromLocalISO(event.start))}`"
      @click.stop
      @pointerdown="onPointerdown($event, event)"
    >
      <span v-if="!event.allDay" class="size-2 shrink-0 rounded-full max-lg:hidden" :class="CALENDAR_DOT[color]" />
      <UIcon :name="REALM_ICON[event.realm]" class="size-3.5 shrink-0 text-muted" :class="event.realm === 'mine' && 'max-sm:hidden'" />
      <span class="truncate font-medium max-sm:flex-1">{{ event.title }}</span>
      <span v-if="showTime && !event.allDay" data-time class="ms-auto shrink-0 tabular-nums text-muted">{{ formatTime(fromLocalISO(event.start)) }}</span>
    </button>
  </CalendarEventPopover>
</template>

<script setup lang="ts">
import { computed } from "vue";

import CalendarEventPopover from "./CalendarEventPopover.vue";
import { CALENDAR_DOT, CHIP_COMPACT_FILL, EVENT_FILL, REALM_ICON } from "../../domain/colors";
import { formatTime, fromLocalISO } from "../../domain/dates";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventMove } from "../../composables/useEventMove";

import type { CalendarEvent } from "../../../shared/types";

defineOptions({ inheritAttrs: false });

const props = withDefaults(defineProps<{ event: CalendarEvent; showTime?: boolean; anchored?: boolean }>(), { anchored: true });

const { calendarFor } = useCalendarEvents();
const { movingKey, suppressed, onPointerdown } = useEventMove();

const color = computed(() => calendarFor(props.event)?.color ?? "primary");
const moving = computed(() => movingKey.value === props.event.key);
</script>
