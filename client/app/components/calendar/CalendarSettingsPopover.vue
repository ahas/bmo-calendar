<template>
  <!-- What a calendar's settings, or a new calendar's, open in: a popover from the button that opened it,
       or on a phone a centred dialog. -->
  <CalendarFormFloat :open="open" :title="label" :content="{ align: 'end', collisionPadding: 16 }" width="w-80" @update:open="(standing: boolean) => (standing ? (open = true) : void leave())">
    <IconButton size="sm" :icon="calendar ? 'i-material-symbols-settings-outline' : 'i-material-symbols-add'" :label="label" class="pointer-coarse:size-11 pointer-coarse:justify-center" />

    <template #content>
      <CalendarSettings v-model:dirty="dirty" :realm="realm" :calendar="calendar" @done="open = false" />
    </template>
  </CalendarFormFloat>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { askDiscard } from "bmo-ui/app/domain/form-save";

import CalendarFormFloat from "./CalendarFormFloat.vue";
import CalendarSettings from "./CalendarSettings.vue";

import type { Calendar, Realm } from "../../../shared/types";

const props = defineProps<{ realm: Realm; calendar?: Calendar }>();

const { t } = useI18n();
const open = ref(false);
const dirty = ref(false);

const label = computed(() => (props.calendar ? t("calendar.aria.settings_named", { name: props.calendar.name }) : t("calendar.action.add_calendar")));

async function leave(): Promise<void> {
  if (!dirty.value || (await askDiscard(t))) {
    dirty.value = false;
    open.value = false;
  }
}
</script>
