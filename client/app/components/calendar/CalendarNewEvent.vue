<template>
  <!-- A new event: drawn on the day the screen stands on, or read out of a phrase that names a day and a time,
       and drawn where it says, its form open to put right what was read. -->
  <UPopover v-model:open="open" :content="content" :ui="{ content: 'w-80 max-w-[calc(100vw-2rem)] p-2' }">
    <UTooltip :text="$t('calendar.action.new_event')" :kbds="['n']">
      <UButton :color="props.floating ? 'primary' : 'neutral'" :variant="props.floating ? 'solid' : 'ghost'" size="sm" icon="i-material-symbols-add" :aria-label="$t('calendar.action.new_event')" />
    </UTooltip>

    <template #content>
      <UButton icon="i-material-symbols-calendar-add-on-outline" :label="$t('calendar.action.new_event')" color="neutral" variant="ghost" block class="justify-start" @click="handOff(() => createAtAnchor())">
        <template #trailing>
          <UKbd value="n" class="ms-auto" />
        </template>
      </UButton>

      <USeparator class="my-2" />

      <form class="px-1 pb-1" @submit.prevent="onSubmit">
        <UFormField :label="$t('calendar.label.quick_entry')">
          <UInput v-model="text" :placeholder="$t('calendar.placeholder.quick_entry')" :maxlength="200" autofocus class="w-full" />
        </UFormField>
      </form>
    </template>
  </UPopover>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";

import { parseQuickEvent } from "../../domain/quick";
import { useEventDraft } from "../../composables/useEventDraft";

const props = defineProps<{ floating?: boolean }>();

const { createAtAnchor, createFromQuick } = useEventDraft();

const open = ref(false);
const text = ref("");

// The draft's form takes the focus as it opens; the focus this menu would hand back to its button as it
// closes must not take it out again.
let handedOff = false;

const content = computed(
  () =>
    ({
      side: props.floating ? "top" : "bottom",
      align: "end",
      onCloseAutoFocus: (event: Event) => {
        if (handedOff) {
          event.preventDefault();
          handedOff = false;
        }
      }
    }) as const
);

function handOff(create: () => Promise<void>): Promise<void> {
  handedOff = true;
  open.value = false;

  return create();
}

async function onSubmit(): Promise<void> {
  const phrase = text.value.trim();

  if (!phrase) return;

  const quick = parseQuickEvent(phrase);

  text.value = "";
  await handOff(() => createFromQuick(quick, phrase));
}
</script>
