<template>
  <!-- The popover a calendar's settings, or a new calendar's, open in, from the button that opened it. -->
  <UPopover v-model:open="open" :dismissible="!dirty" :content="{ align: 'end', collisionPadding: 16 }" :ui="{ content: 'w-80 max-w-[calc(100vw-2rem)] p-4' }" @close:prevent="leave">
    <UTooltip :text="label">
      <UButton color="neutral" variant="ghost" size="sm" :icon="calendar ? 'i-material-symbols-settings-outline' : 'i-material-symbols-add'" :aria-label="label" />
    </UTooltip>

    <template #content>
      <CalendarSettings v-model:dirty="dirty" :realm="realm" :calendar="calendar" @done="open = false" />
    </template>
  </UPopover>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { askDiscard } from "bmo-ui/app/domain/form-save";

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
