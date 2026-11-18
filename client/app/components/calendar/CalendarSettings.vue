<template>
  <!-- A calendar's settings, or a new calendar's: its name and colour, and in the organization's realm who
       writes it and who is told of its alarms. One save at its foot; a refusal stays in the form. -->
  <UForm :state="state" :validate="validate" class="flex flex-col gap-3" @submit="save">
    <div class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
      <label :for="nameId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.name") }}</label>
      <UFormField name="name" :ui="{ error: 'badge-ink' }">
        <UInput :id="nameId" v-model="state.name" :maxlength="100" autofocus class="w-full" />
      </UFormField>

      <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.color") }}</span>
      <ChoiceSelect v-model="state.color" :items="colorItems" value-key="value" :aria-label="$t('calendar.label.color')" class="w-full">
        <template #leading>
          <span class="m-0.5 block size-2.5 rounded-full" :class="CALENDAR_DOT[state.color]" />
        </template>
        <template #item-leading="{ item }">
          <span class="m-0.5 block size-2.5 rounded-full" :class="CALENDAR_DOT[(item as { value: CalendarColor }).value]" />
        </template>
      </ChoiceSelect>

      <template v-if="realm === 'organization'">
        <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.writers") }}</span>
        <ChoiceSelect v-model="state.writers" :items="writersItems" value-key="value" :aria-label="$t('calendar.label.writers')" class="w-full" />

        <span class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.tell") }}</span>
        <ChoiceSelect v-model="state.tell" :items="tellItems" value-key="value" :aria-label="$t('calendar.label.tell')" class="w-full" />
      </template>
    </div>

    <p v-if="refused" role="alert" class="badge-ink text-sm text-error">{{ refused }}</p>

    <div class="flex items-center justify-end gap-2">
      <UButton v-if="calendar" :label="$t('calendar.action.delete')" color="error" variant="ghost" class="badge-ink" :loading="removing" @click="remove" />
      <UButton type="submit" :label="$t('common.action.save')" :disabled="!!calendar && !dirty" :loading="saving" />
    </div>
  </UForm>
</template>

<script setup lang="ts">
import { computed, reactive, ref, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
import { askQuestion } from "bmo-ui/app/domain/question";

import { CALENDAR_DOT } from "../../domain/colors";
import { refusalWord } from "../../domain/store";
import { COLOR_CHOICES, DEFAULT_TELL, DEFAULT_WRITERS, TELL_CHOICES, WRITERS_CHOICES, choiceItems } from "../../../shared/choices";
import { useCalendarEvents } from "../../composables/useCalendarEvents";

import type { FormError } from "@nuxt/ui";
import type { Calendar, CalendarColor, Realm, Tell, Writers } from "../../../shared/types";

const props = defineProps<{ realm: Realm; calendar?: Calendar }>();
const emit = defineEmits<{ done: [] }>();
const dirty = defineModel<boolean>("dirty", { default: false });

const { t } = useI18n();
const { saveCalendar, addCalendar, retireCalendar } = useCalendarEvents();

const nameId = useId();

interface SettingsState {
  name: string;
  color: CalendarColor;
  writers: Writers;
  tell: Tell;
}

const state = reactive<SettingsState>({
  name: props.calendar?.name ?? "",
  color: props.calendar?.color ?? "info",
  writers: props.calendar?.writers ?? DEFAULT_WRITERS,
  tell: props.calendar?.tell ?? DEFAULT_TELL
});

let saved = JSON.stringify(state);

watch(state, () => (dirty.value = JSON.stringify(state) !== saved), { immediate: true, deep: true });

const colorItems = computed(() => COLOR_CHOICES.map((choice) => ({ value: choice.value, label: t(choice.label) })));
const writersItems = computed(() => choiceItems(t, WRITERS_CHOICES, DEFAULT_WRITERS));
const tellItems = computed(() => choiceItems(t, TELL_CHOICES, DEFAULT_TELL));

function validate(data: SettingsState): FormError[] {
  return data.name.trim() ? [] : [{ name: "name", message: t("calendar.error.no_name") }];
}

const saving = ref(false);
const removing = ref(false);
const refused = ref("");

async function save(): Promise<void> {
  if (saving.value) return;

  saving.value = true;
  refused.value = "";

  const write = { name: state.name.trim(), color: state.color, ...(props.realm === "organization" ? { writers: state.writers, tell: state.tell } : {}) };

  try {
    if (props.calendar) await saveCalendar(props.calendar, write);
    else await addCalendar(props.realm, write);

    saved = JSON.stringify(state);
    dirty.value = false;
    emit("done");
  } catch (error) {
    refused.value = t(refusalWord(error));
  } finally {
    saving.value = false;
  }
}

async function remove(): Promise<void> {
  const calendar = props.calendar;

  if (!calendar || removing.value) return;

  const taken = await askQuestion({
    title: t("calendar.message.delete_calendar"),
    item: calendar.name,
    effect: t("calendar.message.delete_calendar_effect"),
    confirm: t("calendar.action.delete")
  });

  if (!taken) return;

  removing.value = true;
  refused.value = "";

  try {
    await retireCalendar(calendar);
    dirty.value = false;
    emit("done");
  } catch (error) {
    refused.value = t(refusalWord(error));
  } finally {
    removing.value = false;
  }
}
</script>
