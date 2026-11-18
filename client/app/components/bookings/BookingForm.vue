<template>
  <!-- A booking's form, or a new booking's (docs/design.md, 「The console」): its title, its resource with the
       resource's seats and whether it waits for approval, all day, its start and end, and its repeat, with
       one save at its foot. Moving the start moves the end with it, so the booking keeps its length. A
       booking the reader may not change is shown, not offered: its booker and its state stand with it. A
       repeat changed on one occurrence's row reaches the whole booking, as said under it, since one occurrence
       keeps the repeat it stands in. A refusal stays in the form, naming the booking an overlap met by the title
       the service's refusal gives, else by the booking the screen holds there. Opened on a whole booking
       (`whole`), a change or a cancellation reaches all of it. Standing in a dialog, the form names itself by
       `formId` and its actions stand in the dialog's footer (`BookingActions`); in place, they stand under it. -->
  <UForm :id="props.formId" :state="state" :validate="validate" class="flex flex-col gap-3" @submit="save" @keydown.escape.capture.stop="emit('escape')">
    <div class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
      <label :for="titleId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.event_title") }}</label>
      <UFormField name="title" :ui="{ error: 'badge-ink' }">
        <UInput :id="titleId" v-model="state.title" :maxlength="200" :autofocus="!props.booking && !narrow" :disabled="!editable" :size="fieldSize" class="w-full" />
      </UFormField>

      <label :for="resourceId" class="self-start whitespace-nowrap pt-1.5 text-sm text-muted">{{ $t("calendar.label.resource") }}</label>
      <div class="flex min-w-0 flex-col gap-1">
        <ChoiceSelect :id="resourceId" v-model="state.resourceId" :items="resourceItems" value-key="value" :disabled="!editable" :size="fieldSize" class="w-full">
          <template #leading>
            <UIcon :name="BOOKING_ICON" class="size-4 text-muted" />
          </template>
        </ChoiceSelect>
        <p v-if="chosen" class="text-sm text-muted">{{ termsOf(chosen) }}</p>
      </div>

      <label :for="allDayId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.all_day") }}</label>
      <UCheckbox :id="allDayId" v-model="state.allDay" :disabled="!editable" :size="fieldSize" class="justify-self-start" />

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.starts") }}</span>
      <div class="flex min-w-0 flex-wrap items-center gap-2">
        <CalendarDateField v-model="state.startDate" :size="fieldSize" :disabled="!editable" :label="$t('calendar.label.starts')" />
        <UInputTime v-if="!state.allDay" v-model="state.startTime" :size="fieldSize" :hour-cycle="24" :disabled="!editable" :aria-label="$t('calendar.aria.start_time')" />
      </div>

      <span class="self-start whitespace-nowrap pt-1.5 text-sm text-muted">{{ $t("calendar.label.ends") }}</span>
      <UFormField name="end" :ui="{ error: 'badge-ink' }">
        <div class="flex min-w-0 flex-wrap items-center gap-2">
          <CalendarDateField v-model="state.endDate" :size="fieldSize" :disabled="!editable" :label="$t('calendar.label.ends')" />
          <UInputTime v-if="!state.allDay" v-model="state.endTime" :size="fieldSize" :hour-cycle="24" :disabled="!editable" :aria-label="$t('calendar.aria.end_time')" />
        </div>
      </UFormField>

      <label :for="repeatId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.repeat") }}</label>
      <div class="flex min-w-0 flex-col gap-1">
        <ChoiceSelect :id="repeatId" v-model="state.repeat" :items="repeatItems" value-key="value" :disabled="!editable" :size="fieldSize" class="w-full" />
        <p v-if="wholeOnly" class="text-sm text-muted">{{ $t("calendar.message.booking_repeat_whole") }}</p>
      </div>

      <template v-if="props.booking">
        <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.booker") }}</span>
        <span class="min-w-0 truncate text-sm">{{ nameOf(props.booking.booker) }}</span>

        <span class="self-start whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.state") }}</span>
        <div class="flex min-w-0 flex-col gap-1 text-sm">
          <span class="flex items-center gap-1.5">
            <UIcon :name="STATE_ICONS[props.booking.state]" class="badge-ink size-4 shrink-0" :class="STATE_COLORS[props.booking.state]" />
            {{ $t(STATE_LABELS[props.booking.state]) }}
          </span>
          <span v-if="props.booking.reason" class="text-muted">{{ props.booking.reason }}</span>
        </div>
      </template>
    </div>

    <p v-if="meetsStanding && !refused" class="badge-ink text-sm text-warning">{{ $t("calendar.message.booking_meets") }}</p>
    <p v-if="refused" role="alert" class="badge-ink text-sm text-error">{{ refused }}</p>

    <BookingActions v-if="!props.formId && editable" :existing="!!props.booking" :dirty="dirty" :saving="saving" :cancelling="cancelling" :size="fieldSize" @cancel="cancelIt" />
  </UForm>
</template>

<script setup lang="ts">
import { Time } from "@internationalized/date";
import { useMediaQuery } from "@vueuse/core";
import { addDays, subMilliseconds } from "date-fns";
import { computed, onBeforeUnmount, ref, shallowReactive, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
import { countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";

import BookingActions from "./BookingActions.vue";
import CalendarDateField from "../calendar/CalendarDateField.vue";
import { movedEnd } from "../../domain/a11y";
import { BOOKING_ICON, STATE_COLORS, STATE_ICONS, STATE_LABELS, meets, metBooking, resourceTerms } from "../../domain/bookings";
import { fromLocalISO, toCalendarDate, toDate, toDateTime, toLocalISO, toTime } from "../../domain/dates";
import { refusalText } from "../../domain/store";
import { DEFAULT_REPEAT, REPEAT_CHOICES, choiceItems } from "../../../shared/choices";
import { repeats, useBookingChange } from "../../composables/useBookingChange";
import { isoSpan, useBookings } from "../../composables/useBookings";
import { useCalendarEvents } from "../../composables/useCalendarEvents";
import { usePeople } from "../../composables/usePeople";

import type { CalendarDate } from "@internationalized/date";
import type { FormError } from "@nuxt/ui";
import type { Booking, Repeat, Resource } from "../../../shared/types";

/** A new booking's first values: the resource and the span a press or a drag offered. */
export interface BookingDraft {
  resourceId: string;
  start: Date;
  end: Date;
  allDay?: boolean;
}

const props = defineProps<{
  booking?: Booking;
  draft?: BookingDraft;
  resources: Resource[];
  /** A change or a cancellation reaches the whole booking, as from the reader's own list. */
  whole?: boolean;
  /** The form's id, where its actions stand outside it, in a dialog's footer. */
  formId?: string;
}>();

const emit = defineEmits<{
  done: [];
  escape: [];
}>();

/** Whether the form holds a change from what is saved, for the dialog's guard on closing. */
const dirty = defineModel<boolean>("dirty", { default: false });

const { t } = useI18n();
const { bookings, mine, book, mayChange } = useBookings();
const { allEvents } = useCalendarEvents();
const { change, cancel } = useBookingChange();
const { nameOf } = usePeople();

const narrow = useNarrow();
// A finger is given the fields' largest size to aim at.
const coarse = useMediaQuery("(pointer: coarse)");

const titleId = useId();
const resourceId = useId();
const allDayId = useId();
const repeatId = useId();

interface FormState {
  title: string;
  resourceId: string;
  allDay: boolean;
  startDate?: CalendarDate;
  startTime?: Time;
  endDate?: CalendarDate;
  endTime?: Time;
  repeat: Repeat;
}

function initialState(): FormState {
  const start = props.booking ? fromLocalISO(props.booking.start) : props.draft!.start;
  const end = props.booking ? fromLocalISO(props.booking.end) : props.draft!.end;
  const allDay = props.booking?.allDay ?? props.draft?.allDay ?? false;

  return {
    title: props.booking?.title ?? "",
    resourceId: props.booking?.resourceId ?? props.draft?.resourceId ?? props.resources[0]?.id ?? "",
    allDay,
    startDate: toCalendarDate(start),
    // The last day an all-day booking holds is a moment before its end, the midnight after it.
    endDate: toCalendarDate(allDay ? subMilliseconds(end, 1) : end),
    startTime: allDay ? new Time(9) : toTime(start),
    endTime: allDay ? new Time(10) : toTime(end),
    repeat: props.booking?.repeat ?? DEFAULT_REPEAT
  };
}

// Shallow, so the date and the time keep their own private fields unwrapped.
const state = shallowReactive<FormState>(initialState());

function span(data: FormState): { start: Date; end: Date } | null {
  if (!data.startDate || !data.endDate) return null;

  if (data.allDay) return { start: toDate(data.startDate), end: addDays(toDate(data.endDate), 1) };

  if (!data.startTime || !data.endTime) return null;

  return { start: toDateTime(data.startDate, data.startTime), end: toDateTime(data.endDate, data.endTime) };
}

// What the form holds, as one line, so a field typed back to what is saved is no change.
function signature(data: FormState): string {
  const period = span(data);

  return JSON.stringify([data.title.trim(), data.resourceId, data.allDay, period && toLocalISO(period.start), period && toLocalISO(period.end), data.repeat]);
}

let saved = signature(state);

function startOf(data: FormState): Date | null {
  return data.startDate ? (data.startTime ? toDateTime(data.startDate, data.startTime) : toDate(data.startDate)) : null;
}

function endOf(data: FormState): Date | null {
  return data.endDate ? (data.endTime ? toDateTime(data.endDate, data.endTime) : toDate(data.endDate)) : null;
}

const refused = ref("");
const saving = ref(false);
const cancelling = ref(false);

const editable = computed(() => !props.booking || mayChange(props.booking));
const fieldSize = computed(() => (coarse.value ? "xl" : "md"));
const chosen = computed(() => props.resources.find((resource) => resource.id === state.resourceId));

const resourceItems = computed(() => props.resources.map((resource) => ({ label: resource.name, value: resource.id, description: termsOf(resource) })));
const repeatItems = computed(() => choiceItems(t, REPEAT_CHOICES, DEFAULT_REPEAT));

// A span that meets a standing booking of the resource, as far as the screen has read, is said before the
// save; the service is the one that refuses it.
// Every booking the screen has read: the company's of the span in view, the reader's own, and the reader's
// on the calendar's grid.
const known = computed<Booking[]>(() => [...bookings.value, ...mine.value, ...allEvents.value.flatMap((row) => (row.booking ? [row.booking] : []))]);

// A repeat changed on a repeating booking's row can reach only the whole booking.
const wholeOnly = computed(() => !props.whole && !!props.booking && repeats(props.booking) && state.repeat !== props.booking.repeat);

const meetsStanding = computed(() => {
  const period = span(state);

  return !!period && period.end > period.start && meets(known.value, state.resourceId, period, props.booking?.id);
});

// Moving the start moves the end by as much: this is how a booking is moved without a drag.
watch(
  () => startOf(state)?.getTime(),
  (now, before) => {
    const end = endOf(state);

    if (now === undefined || before === undefined || now === before || !end) return;

    const moved = movedEnd({ start: new Date(before), end }, new Date(now));

    state.endDate = toCalendarDate(moved);
    state.endTime = toTime(moved);
  }
);

watch(
  () => signature(state),
  (now) => {
    dirty.value = now !== saved;
    refused.value = "";
  },
  { immediate: true }
);

function termsOf(resource: Resource): string {
  return resourceTerms(t, resource, countText);
}

function validate(data: FormState): FormError[] {
  const period = span(data);
  const errors: FormError[] = [];

  if (!data.title.trim()) errors.push({ name: "title", message: t("calendar.error.no_title") });
  if (!period) errors.push({ name: "end", message: t("calendar.error.no_time") });
  else if (period.end <= period.start) errors.push({ name: "end", message: t("calendar.error.end_before_start") });

  return errors;
}

async function save(): Promise<void> {
  const period = span(state);

  if (!period || !state.resourceId || saving.value) return;

  const write = { resourceId: state.resourceId, title: state.title.trim(), allDay: state.allDay, repeat: state.repeat, ...isoSpan(period) };

  saving.value = true;
  refused.value = "";

  try {
    if (props.booking) {
      if (!(await change(props.booking, write, props.whole))) return;
    } else {
      await book(write);
    }

    saved = signature(state);
    dirty.value = false;
    emit("done");
  } catch (error) {
    refused.value = refusalText(t, error, metBooking(known.value, write.resourceId, period, props.booking?.id)?.title);
  } finally {
    saving.value = false;
  }
}

async function cancelIt(): Promise<void> {
  if (!props.booking || cancelling.value) return;

  cancelling.value = true;
  refused.value = "";

  try {
    if (await cancel(props.booking, props.whole)) {
      dirty.value = false;
      emit("done");
    }
  } catch (error) {
    refused.value = refusalText(t, error);
  } finally {
    cancelling.value = false;
  }
}

onBeforeUnmount(() => (dirty.value = false));

defineExpose({ editable, saving, cancelling, cancelIt });
</script>
