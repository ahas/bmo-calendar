<template>
  <!-- An event's form, or a draft's: every field the service keeps of an event, with one save at its
       foot. A draft's ghost follows the form as it is filled; an event changes only when it is saved. -->
  <UForm ref="form" :state="state" :validate="validate" class="flex flex-col gap-3" @submit="save" @keydown.enter.capture="onEnter">
    <div class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
      <label :for="titleId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.event_title") }}</label>
      <UFormField name="title">
        <UInput :id="titleId" v-model="state.title" :placeholder="$t('calendar.label.untitled')" :maxlength="200" autofocus class="w-full" />
      </UFormField>

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.calendar") }}</span>
      <USelect v-model="state.calendar" :items="calendarItems" :aria-label="$t('calendar.label.calendar')" class="w-full">
        <template #leading>
          <span class="m-0.5 block size-2.5 rounded-full" :class="CALENDAR_DOT[color]" />
        </template>
      </USelect>

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.all_day") }}</span>
      <UCheckbox v-model="state.allDay" :aria-label="$t('calendar.label.all_day')" class="justify-self-start" />

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.starts") }}</span>
      <div class="flex min-w-0 flex-wrap items-center gap-2">
        <CalendarDateField v-model="state.startDate" :label="$t('calendar.label.starts')" />
        <UInputTime v-if="!state.allDay" v-model="state.startTime" :hour-cycle="24" :aria-label="$t('calendar.aria.start_time')" />
      </div>

      <span class="self-start whitespace-nowrap pt-1.5 text-sm text-muted">{{ $t("calendar.label.ends") }}</span>
      <UFormField name="end">
        <div class="flex min-w-0 flex-wrap items-center gap-2">
          <CalendarDateField v-model="state.endDate" :label="$t('calendar.label.ends')" />
          <UInputTime v-if="!state.allDay" v-model="state.endTime" :hour-cycle="24" :aria-label="$t('calendar.aria.end_time')" />
        </div>
      </UFormField>

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.alarm") }}</span>
      <USelect v-model="state.alarm" :items="alarmItems" :aria-label="$t('calendar.label.alarm')" class="w-full" />

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.repeat") }}</span>
      <USelect v-model="state.repeat" :items="repeatItems" :aria-label="$t('calendar.label.repeat')" class="w-full" />

      <label :for="notesId" class="self-start whitespace-nowrap pt-1.5 text-sm text-muted">{{ $t("calendar.label.notes") }}</label>
      <GrowingTextarea :id="notesId" v-model="state.description" class="w-full" />
    </div>

    <div class="flex items-center justify-end gap-2">
      <UButton v-if="event" :label="$t('calendar.action.delete')" color="error" variant="ghost" :loading="removing" @click="remove" />
      <UButton type="submit" :label="$t('common.action.save')" :disabled="!!event && !dirty" :loading="saving" />
    </div>
  </UForm>
</template>

<script setup lang="ts">
import { CalendarDate, Time } from "@internationalized/date";
import { addDays, subMilliseconds } from "date-fns";
import { computed, onBeforeUnmount, ref, shallowReactive, useId, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import GrowingTextarea from "bmo-ui/app/components/GrowingTextarea.vue";

import CalendarDateField from "./CalendarDateField.vue";
import { CALENDAR_DOT } from "../../domain/colors";
import { fromLocalISO, toCalendarDate, toDate, toDateTime, toLocalISO, toTime } from "../../domain/dates";
import { ALARM_CHOICES, DEFAULT_ALARM, DEFAULT_REPEAT, REPEAT_CHOICES, choiceItems } from "../../../shared/choices";
import { calendarKey, useCalendarEvents } from "../../composables/useCalendarEvents";
import { useEventChange } from "../../composables/useEventChange";
import { useEventEditor } from "../../composables/useEventEditor";

import type { FormError } from "@nuxt/ui";
import type { EventDraft } from "../../composables/useEventDraft";
import type { Alarm, Calendar, CalendarEvent, Repeat } from "../../../shared/types";

const props = defineProps<{ event?: CalendarEvent; draft?: EventDraft }>();

const emit = defineEmits<{
  update: [patch: Partial<EventDraft>];
  commit: [values: EventDraft];
  done: [];
}>();

/** Whether the form holds a change from what is saved, for the popover's guard on closing. */
const dirty = defineModel<boolean>("dirty", { default: false });

const { t } = useI18n();
const { groups, lastUsed, calendarByKey } = useCalendarEvents();
const { changeEvent, cancelEvent } = useEventChange();

const titleId = useId();
const notesId = useId();

if (props.event) useEventEditor().registerAnchor();

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

const alarmItems = computed(() => choiceItems(t, ALARM_CHOICES, DEFAULT_ALARM));
const repeatItems = computed(() => choiceItems(t, REPEAT_CHOICES, DEFAULT_REPEAT));

// A draft's ghost walks to where the form puts it as it is filled.
watch(
  () => signature(state),
  () => {
    if (props.event) return;

    const period = span(state);

    if (!period || period.end <= period.start) return;

    emit("update", { title: state.title, calendar: state.calendar, description: state.description, allDay: state.allDay, alarm: state.alarm, repeat: state.repeat, ...period });
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
