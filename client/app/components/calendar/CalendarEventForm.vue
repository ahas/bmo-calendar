<template>
  <!-- An event's form, or a draft's: every field the service keeps of an event, with one save at its
       foot. A draft's ghost follows the form as it is filled; an event changes only when it is saved.
       The date and time segments take every key but Tab, Escape among them, so Escape is caught on its
       way down and handed to whatever closes the form. Moving the start moves the end with it, so the
       event keeps its length; the end alone changes its length. -->
  <UForm
    ref="form"
    :state="state"
    :validate="validate"
    class="flex flex-col gap-3"
    @submit="save"
    @keydown.enter.capture="onEnter"
    @keydown.escape.capture.stop="emit('escape')"
  >
    <div ref="fields" class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2" @focusout="onFieldsFocusout">
      <label :for="titleId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.event_title") }}</label>
      <UFormField name="title">
        <UInput :id="titleId" v-model="state.title" :placeholder="$t('calendar.label.untitled')" :maxlength="200" :autofocus="draft ? !redrawn : !narrow" :size="fieldSize" class="w-full" />
      </UFormField>

      <label :for="calendarId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.calendar") }}</label>
      <USelect :id="calendarId" v-model="state.calendar" :items="calendarItems" :size="fieldSize" class="w-full">
        <template #leading>
          <span class="m-0.5 block size-2.5 rounded-full" :class="CALENDAR_DOT[color]" />
        </template>
      </USelect>

      <label :for="allDayId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.all_day") }}</label>
      <UCheckbox :id="allDayId" v-model="state.allDay" :size="fieldSize" class="justify-self-start" />

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.starts") }}</span>
      <div class="flex min-w-0 flex-wrap items-center gap-2">
        <CalendarDateField v-model="state.startDate" :size="fieldSize" :label="$t('calendar.label.starts')" />
        <UInputTime v-if="!state.allDay" v-model="state.startTime" :size="fieldSize" :hour-cycle="24" :aria-label="$t('calendar.aria.start_time')" />
      </div>

      <span class="self-start whitespace-nowrap pt-1.5 text-sm text-muted">{{ $t("calendar.label.ends") }}</span>
      <UFormField name="end" :ui="{ error: 'badge-ink' }">
        <div class="flex min-w-0 flex-wrap items-center gap-2">
          <CalendarDateField :id="endDateId" v-model="state.endDate" :size="fieldSize" :label="$t('calendar.label.ends')" />
          <UInputTime v-if="!state.allDay" :id="endTimeId" v-model="state.endTime" :size="fieldSize" :hour-cycle="24" :aria-label="$t('calendar.aria.end_time')" />
        </div>
      </UFormField>

      <label :for="alarmId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.alarm") }}</label>
      <USelect :id="alarmId" v-model="state.alarm" :items="alarmItems" :size="fieldSize" class="w-full" />

      <label :for="repeatId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.repeat") }}</label>
      <USelect :id="repeatId" v-model="state.repeat" :items="repeatItems" :size="fieldSize" class="w-full" />

      <label :for="notesId" class="self-start whitespace-nowrap pt-1.5 text-sm text-muted">{{ $t("calendar.label.notes") }}</label>
      <GrowingTextarea :id="notesId" v-model="state.description" class="w-full" />
    </div>

    <div class="flex items-center justify-end gap-2">
      <UButton v-if="event" :label="$t('calendar.action.delete')" color="error" variant="ghost" class="badge-ink" :size="fieldSize" :loading="removing" @click="remove" />
      <UButton type="submit" :label="$t('common.action.save')" :size="fieldSize" :disabled="!!event && !dirty" :loading="saving" />
    </div>
  </UForm>
</template>

<script setup lang="ts">
import { CalendarDate, Time } from "@internationalized/date";
import { useMediaQuery } from "@vueuse/core";
import { addDays, subMilliseconds } from "date-fns";
import { computed, onBeforeUnmount, onMounted, ref, shallowReactive, useId, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import GrowingTextarea from "bmo-ui/app/components/GrowingTextarea.vue";
import { useNarrow } from "bmo-ui/app/domain/screen";

import CalendarDateField from "./CalendarDateField.vue";
import { movedEnd } from "../../domain/a11y";
import { CALENDAR_DOT } from "../../domain/colors";
import { fromLocalISO, toCalendarDate, toDate, toDateTime, toLocalISO, toTime } from "../../domain/dates";
import { ALARM_CHOICES, DEFAULT_ALARM, DEFAULT_REPEAT, REPEAT_CHOICES, choiceItems } from "../../../shared/choices";
import { calendarKey, useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventChange } from "../../composables/useEventChange";
import { useEventDraft } from "../../composables/useEventDraft";
import { useEventEditor } from "../../composables/useEventEditor";

import type { FormError } from "@nuxt/ui";
import type { EventDraft } from "../../composables/useEventDraft";
import type { Alarm, Calendar, CalendarEvent, Repeat } from "../../../shared/types";

const props = defineProps<{
  event?: CalendarEvent;
  draft?: EventDraft;
  /** The draft's form drawn again where the draft moved to, rather than its first. */
  redrawn?: boolean;
}>();

const emit = defineEmits<{
  update: [patch: Partial<EventDraft>];
  commit: [values: EventDraft];
  done: [];
  escape: [];
}>();

/** Whether the form holds a change from what is saved, for the popover's guard on closing. */
const dirty = defineModel<boolean>("dirty", { default: false });

const { t } = useI18n();
const { groups, lastUsed, calendarByKey } = useCalendarEvents();
const { changeEvent, cancelEvent } = useEventChange();

const narrow = useNarrow();
// A finger is given the fields' largest size to aim at.
const coarse = useMediaQuery("(pointer: coarse)");

const titleId = useId();
const calendarId = useId();
const allDayId = useId();
const endDateId = useId();
const endTimeId = useId();
const alarmId = useId();
const repeatId = useId();
const notesId = useId();

if (props.event) useEventEditor().registerAnchor();

// A draft moved to another day by its start is drawn there again, form and all: the field that held the
// focus here gives it to the same field there, so a date typed segment by segment goes on being typed.
// Only the draft's first form takes the focus on its own, at its title; one drawn again takes none of
// its own, and since it may stand before the form it follows is taken down, it gives the focus once the
// whole redraw is over: to the field that held it, else to the title where the focus went with the form.
const drafts = props.draft ? useEventDraft() : null;
const fields = useTemplateRef<HTMLElement>("fields");

/** The fields' controls the keyboard reaches, in the order it reaches them. */
function controls(): HTMLElement[] {
  return [...(fields.value?.querySelectorAll<HTMLElement>('input:not([type="hidden"]), textarea, button, [tabindex]:not([tabindex="-1"])') ?? [])];
}

onMounted(() => {
  if (!drafts || !props.redrawn) return;

  setTimeout(() => {
    const place = drafts.takeCarriedFocus();

    if (place !== null) controls()[place]?.focus();
    else if (document.activeElement === document.body) document.getElementById(titleId)?.focus();
  });
});

onBeforeUnmount(() => {
  const place = controls().indexOf(document.activeElement as HTMLElement);

  if (drafts && place >= 0) drafts.carryFocus(place);
});

interface FormState {
  title: string;
  calendar: string;
  allDay: boolean;
  startDate?: CalendarDate;
  startTime?: Time;
  endDate?: CalendarDate;
  endTime?: Time;
  alarm: Alarm;
  repeat: Repeat;
  description: string;
}

function initialState(): FormState {
  const start = props.event ? fromLocalISO(props.event.start) : props.draft!.start;
  const end = props.event ? fromLocalISO(props.event.end) : props.draft!.end;
  const allDay = props.event?.allDay ?? props.draft?.allDay ?? false;

  return {
    title: props.event?.title ?? props.draft?.title ?? "",
    calendar: props.event ? calendarKey({ realm: props.event.realm, id: props.event.calendarId }) : props.draft!.calendar,
    allDay,
    startDate: toCalendarDate(start),
    // The last day an all-day event covers is a moment before its end, the midnight after it.
    endDate: toCalendarDate(allDay ? subMilliseconds(end, 1) : end),
    startTime: allDay ? new Time(9) : toTime(start),
    endTime: allDay ? new Time(10) : toTime(end),
    alarm: props.event?.alarm ?? props.draft?.alarm ?? DEFAULT_ALARM,
    repeat: props.event?.repeat ?? props.draft?.repeat ?? DEFAULT_REPEAT,
    description: props.event?.description ?? props.draft?.description ?? ""
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

  return JSON.stringify([data.title, data.calendar, data.allDay, period && toLocalISO(period.start), period && toLocalISO(period.end), data.alarm, data.repeat, data.description]);
}

const saved = signature(state);

// The moments the fields name, their times read even while the event is all day, so turning the mark on
// or off moves neither.
function startOf(data: FormState): Date | null {
  return data.startDate ? (data.startTime ? toDateTime(data.startDate, data.startTime) : toDate(data.startDate)) : null;
}

function endOf(data: FormState): Date | null {
  return data.endDate ? (data.endTime ? toDateTime(data.endDate, data.endTime) : toDate(data.endDate)) : null;
}

// Moving the start moves the end by as much: this is how an event is moved without a drag.
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
  (now) => (dirty.value = now !== saved),
  { immediate: true }
);

function validate(data: FormState): FormError[] {
  const period = span(data);

  if (!period) return [{ name: "end", message: t("calendar.error.no_time") }];
  if (period.end <= period.start) return [{ name: "end", message: t("calendar.error.end_before_start") }];

  return [];
}

// The calendars to put it in, a realm to a group, the last one used first. An event keeps its realm.
const calendarItems = computed(() => {
  const ordered = [...groups.value].sort((a, b) => Number(b.calendars.some((one) => calendarKey(one) === lastUsed.value)) - Number(a.calendars.some((one) => calendarKey(one) === lastUsed.value)));
  const item = (calendar: Calendar) => ({ label: calendar.name, value: calendarKey(calendar) });

  return ordered
    .filter((group) => !props.event || group.realm === props.event.realm)
    .filter((group) => group.calendars.length > 0)
    .map((group) => [
      { type: "label" as const, label: t(group.realm === "mine" ? "calendar.label.mine" : "calendar.label.organization") },
      ...[...group.calendars].sort((a, b) => Number(calendarKey(b) === lastUsed.value) - Number(calendarKey(a) === lastUsed.value)).map(item)
    ]);
});

const color = computed(() => calendarByKey(state.calendar)?.color ?? "primary");
const fieldSize = computed(() => (coarse.value ? "xl" : "md"));

const alarmItems = computed(() => choiceItems(t, ALARM_CHOICES, DEFAULT_ALARM));
const repeatItems = computed(() => choiceItems(t, REPEAT_CHOICES, DEFAULT_REPEAT));

// A draft's ghost walks to where the form puts it as it is filled. A date or a time is typed segment by
// segment, and a segment keeps the digits typed so far for the next one, so while the focus stands on a
// segment the ghost waits for the focus to leave them: a ghost walked to another day is drawn there again
// with its form, and the digits waiting would be lost.
const SEGMENT = "[data-reka-date-field-segment], [data-reka-time-field-segment]";
let waiting = false;

function onSegment(target: EventTarget | null): boolean {
  return target instanceof Element && target.matches(SEGMENT) && !!fields.value?.contains(target);
}

function walkGhost(): void {
  waiting = false;

  const period = span(state);

  if (!period || period.end <= period.start) return;

  emit("update", { title: state.title, calendar: state.calendar, description: state.description, allDay: state.allDay, alarm: state.alarm, repeat: state.repeat, ...period });
}

// The focus lands on its new control only after the focus leaving has been told, so the ghost walks once
// it has: the form drawn on the day it walks to gives the focus to the control it went to.
function onFieldsFocusout(event: FocusEvent): void {
  if (waiting && !onSegment(event.relatedTarget)) setTimeout(walkGhost);
}

watch(
  () => signature(state),
  () => {
    if (props.event) return;

    if (onSegment(document.activeElement)) waiting = true;
    else walkGhost();
  }
);

const form = useTemplateRef("form");
const saving = ref(false);
const removing = ref(false);

// Enter saves from any field but the notes, a select opening its list and a checkbox.
function onEnter(event: KeyboardEvent): void {
  const target = event.target as HTMLElement;

  if (target.tagName === "TEXTAREA" || target.closest('[role="combobox"],[role="checkbox"],button')) return;

  event.preventDefault();
  event.stopPropagation();
  void form.value?.submit();
}

async function save(): Promise<void> {
  const period = span(state);
  const calendar = calendarByKey(state.calendar);

  if (!period || !calendar || saving.value) return;

  const title = state.title.trim() || t("calendar.label.untitled");

  if (!props.event) {
    emit("commit", { title, calendar: state.calendar, description: state.description, allDay: state.allDay, alarm: state.alarm, repeat: state.repeat, ...period });
    return;
  }

  saving.value = true;

  try {
    const sent = await changeEvent(props.event, {
      ...props.event,
      calendarId: calendar.id,
      title,
      description: state.description,
      start: toLocalISO(period.start),
      end: toLocalISO(period.end),
      allDay: state.allDay,
      alarm: state.alarm,
      repeat: state.repeat
    });

    if (sent) {
      dirty.value = false;
      emit("done");
    }
  } finally {
    saving.value = false;
  }
}

async function remove(): Promise<void> {
  if (!props.event || removing.value) return;

  removing.value = true;

  try {
    if (await cancelEvent(props.event)) {
      dirty.value = false;
      emit("done");
    }
  } finally {
    removing.value = false;
  }
}

onBeforeUnmount(() => (dirty.value = false));
</script>
