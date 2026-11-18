<template>
  <!-- What pressing an event opens: its form, beside it, or on a phone as a centred dialog. A right press
       or a held finger (the grid's hold menu) offers the same form and the cancellation as a menu. Where a
       mouse points, neither is made until the event is first pointed at, since the month view puts hundreds
       of events up at once; on a touch screen they stand from the first. A received schedule opens neither:
       it is a link to the item it stands for. The reader's own booking opens its booking's form, and its menu
       offers its cancellation. An event of a realm the reader only reads opens its form read-only, and offers
       no menu. -->
  <div ref="root" class="contents" @pointerover="arm" @focusin="arm" @contextmenu="arm">
    <slot v-if="!active" :open="false" />

    <UContextMenu v-else :items="items" :disabled="disabled || !writable(event.realm)">
      <div class="contents">
        <CalendarFormFloat :open="open" :title="event.title || $t('calendar.label.untitled')" :content="{ side: formSide, collisionPadding: 16 }" @update:open="onUpdateOpen">
          <slot :open="open" />

          <template #content>
            <BookingForm v-if="event.booking" v-model:dirty="dirty" :booking="event.booking" :resources="resourcesFor(event.booking.company)" @done="closeEvent(event.key)" @escape="leave" />
            <CalendarEventForm v-else v-model:dirty="dirty" :event="event" @done="closeEvent(event.key)" @escape="leave" />
          </template>
        </CalendarFormFloat>
      </div>
    </UContextMenu>
  </div>
</template>

<script setup lang="ts">
import { useMediaQuery } from "@vueuse/core";
import { computed, nextTick, ref, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import { useCloseGuard } from "bmo-ui/app/domain/form-save";
import { useIsland } from "bmo-ui/app/domain/island";

import BookingForm from "../bookings/BookingForm.vue";
import CalendarEventForm from "./CalendarEventForm.vue";
import CalendarFormFloat from "./CalendarFormFloat.vue";
import { refusalText } from "../../domain/store";
import { useBookingChange } from "../../composables/useBookingChange";
import { useBookings } from "../../composables/useBookings";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventChange } from "../../composables/useEventChange";
import { useEventEditor } from "../../composables/useEventEditor";

import type { ContextMenuItem } from "@nuxt/ui";
import type { CalendarEvent } from "../../../shared/types";

const props = withDefaults(defineProps<{ event: CalendarEvent; disabled?: boolean; anchored?: boolean }>(), { anchored: true });

const { t } = useI18n();
const { formSide } = useCalendar();
const { editingKey, openEvent, closeEvent } = useEventEditor();
const { cancelEvent } = useEventChange();
const { resourcesFor } = useBookings();
const { writable } = useCalendarEvents();
const bookingChange = useBookingChange();
const island = useIsland(t);

// A touch screen's events stand armed from the first: the hold that asks for the menu is timed from the
// finger landing, and arming then would put a new event in place of the one the finger is on.
const coarse = useMediaQuery("(pointer: coarse)");
const armed = ref(false);
const dirty = ref(false);
const root = useTemplateRef("root");

// A received schedule is changed by its unit alone, so nothing here offers to change it.
const received = computed(() => !!props.event.origin);

function arm(event: Event): void {
  if (armed.value || received.value) return;

  armed.value = true;

  // Arming puts the wrapped event in place of the bare one, so a keyboard on it is handed the new one.
  if (event.type === "focusin") void nextTick(() => root.value?.querySelector<HTMLElement>("[data-event]")?.focus());
}

const open = computed(() => props.anchored && editingKey.value === props.event.key);
const active = computed(() => !received.value && (armed.value || open.value || coarse.value));

function onUpdateOpen(value: boolean): void {
  if (props.disabled) return;

  if (value) openEvent(props.event.key);
  else void leave();
}

// Closing a form that holds a change asks first whether to throw it away, one question at a time.
const shown = computed({
  get: () => open.value,
  set(value: boolean) {
    if (value) return;

    dirty.value = false;
    closeEvent(props.event.key);
  }
});
const { leave } = useCloseGuard(shown, () => dirty.value, t);

// A booking is cancelled rather than deleted, and a refusal of it is told where the reader is.
async function cancelBooking(): Promise<void> {
  const booking = props.event.booking;

  if (!booking) return;

  try {
    await bookingChange.cancel(booking);
  } catch (error) {
    island.say({ tone: "error", title: refusalText(t, error) });
  }
}

const items = computed<ContextMenuItem[]>(() => [
  { label: t("calendar.action.edit"), icon: "i-material-symbols-edit-outline", onSelect: () => openEvent(props.event.key) },
  props.event.booking
    ? { label: t("calendar.action.cancel_booking"), icon: "i-material-symbols-event-busy-outline", color: "error", onSelect: () => void cancelBooking() }
    : { label: t("calendar.action.delete"), icon: "i-material-symbols-delete-outline", color: "error", onSelect: () => void cancelEvent(props.event) }
]);
</script>
