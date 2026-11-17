<template>
  <!-- An event in a month cell or the all-day row: a bar for an all-day event, a dot and a title for a
       timed one, its words in the page's own text colour. Dragged a day at a time by a mouse; pressed to
       open its form. A received schedule is a link to the item it stands for instead, and is never moved; the
       reader's own booking is never moved either, and opens its booking's form.
       Named whole for a reader who does not see it, however much of it is cut. -->
  <CalendarEventPopover v-slot="{ open }" :event="event" :disabled="suppressed" :anchored="anchored">
    <component
      :is="opens ? ULink : 'button'"
      v-bind="{ ...$attrs, ...(opens ?? { type: 'button' }) }"
      data-event
      class="focus-inset flex h-6 min-w-0 select-none items-center gap-1 rounded-sm px-1.5 text-start text-sm text-highlighted transition-colors"
      :class="event.allDay ? EVENT_FILL[color] : ['hover:bg-elevated active:bg-accented data-active:bg-elevated', CHIP_COMPACT_FILL[color]]"
      :data-active="open || moving || undefined"
      :aria-label="name"
      @click.stop
      v-on="fixed(event) ? {} : { pointerdown: (pointer: PointerEvent) => onPointerdown(pointer, event) }"
    >
      <span v-if="!event.allDay" class="size-2 shrink-0 rounded-full max-lg:hidden" :class="CALENDAR_DOT[color]" />
      <UIcon :name="REALM_ICON[event.realm]" class="size-3.5 shrink-0 text-muted" :class="event.realm === 'mine' && 'max-sm:hidden'" />
      <UIcon v-if="mark" :name="mark" class="size-3.5 shrink-0 text-muted max-sm:hidden" />
      <span class="truncate font-medium max-sm:flex-1">{{ event.title }}</span>
      <span v-if="showTime && !event.allDay" data-time class="ms-auto shrink-0 tabular-nums text-muted">{{ formatTime(fromLocalISO(event.start)) }}</span>
    </component>
  </CalendarEventPopover>
</template>

<script setup lang="ts">
import ULink from "@nuxt/ui/components/Link.vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import CalendarEventPopover from "./CalendarEventPopover.vue";
import { eventName } from "../../domain/a11y";
import { CALENDAR_DOT, CHIP_COMPACT_FILL, EVENT_FILL, RECEIVED_ICON, REALM_ICON } from "../../domain/colors";
import { originLink } from "../../domain/origin";
import { BOOKING_ICON } from "../../domain/bookings";
import { fixed } from "../../domain/store";
import { formatTime, fromLocalISO } from "../../domain/dates";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventMove } from "../../composables/useEventMove";

import type { CalendarEvent } from "../../../shared/types";

defineOptions({ inheritAttrs: false });

const props = withDefaults(defineProps<{ event: CalendarEvent; showTime?: boolean; anchored?: boolean }>(), { anchored: true });

const { t } = useI18n();
const { layerOf } = useCalendarEvents();
const { movingKey, suppressed, onPointerdown } = useEventMove();

const color = computed(() => layerOf(props.event)?.color ?? "primary");
const moving = computed(() => movingKey.value === props.event.key);
const name = computed(() => eventName(t, props.event, layerOf(props.event)?.name));
const opens = computed(() => originLink(props.event.origin));
const mark = computed(() => (props.event.origin ? RECEIVED_ICON : props.event.booking ? BOOKING_ICON : null));
</script>
