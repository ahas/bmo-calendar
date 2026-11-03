<template>
  <!-- What pressing an event opens: its form, beside it. A right press or a held finger offers the same
       form and the cancellation as a menu. Neither is made until the event is first pointed at, since the
       month view puts hundreds of events up at once. -->
  <div ref="root" class="contents" @pointerover="arm" @focusin="arm" @contextmenu="arm">
    <slot v-if="!active" :open="false" />

    <UContextMenu v-else :items="items" :disabled="disabled">
      <div class="contents">
        <UPopover
          :open="open"
          :dismissible="!dirty"
          :ui="{ content: 'w-[22rem] max-w-[calc(100vw-2rem)] p-4' }"
          :content="{ side: formSide, collisionPadding: 16 }"
          @update:open="onUpdateOpen"
          @close:prevent="leave"
        >
          <slot :open="open" />

          <template #content>
            <CalendarEventForm v-model:dirty="dirty" :event="event" @done="closeEvent(event.key)" />
          </template>
        </UPopover>
      </div>
    </UContextMenu>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import { askDiscard } from "bmo-ui/app/domain/form-save";

import CalendarEventForm from "./CalendarEventForm.vue";
import { useCalendar } from "../../composables/useCalendar";
import { useEventChange } from "../../composables/useEventChange";
import { useEventEditor } from "../../composables/useEventEditor";

import type { ContextMenuItem } from "@nuxt/ui";
import type { CalendarEvent } from "../../../shared/types";

const props = withDefaults(defineProps<{ event: CalendarEvent; disabled?: boolean; anchored?: boolean }>(), { anchored: true });

const { t } = useI18n();
const { formSide } = useCalendar();
const { editingKey, openEvent, closeEvent } = useEventEditor();
const { cancelEvent } = useEventChange();

const armed = ref(false);
const dirty = ref(false);
const root = useTemplateRef("root");

function arm(event: Event): void {
  if (armed.value) return;

  armed.value = true;

  // Arming puts the wrapped event in place of the bare one, so a keyboard on it is handed the new one.
  if (event.type === "focusin") void nextTick(() => root.value?.querySelector<HTMLElement>("[data-event]")?.focus());
}

const open = computed(() => props.anchored && editingKey.value === props.event.key);
const active = computed(() => armed.value || open.value);

function onUpdateOpen(value: boolean): void {
  if (props.disabled) return;

  if (value) openEvent(props.event.key);
  else closeEvent(props.event.key);
}

// Closing a form that holds a change asks first whether to throw it away.
async function leave(): Promise<void> {
  if (!dirty.value || (await askDiscard(t))) {
    dirty.value = false;
    closeEvent(props.event.key);
  }
}

const items = computed<ContextMenuItem[]>(() => [
  { label: t("calendar.action.edit"), icon: "i-material-symbols-edit-outline", onSelect: () => openEvent(props.event.key) },
  { label: t("calendar.action.delete"), icon: "i-material-symbols-delete-outline", color: "error", onSelect: () => void cancelEvent(props.event) }
]);
</script>
