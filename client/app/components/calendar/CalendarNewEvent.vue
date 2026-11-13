<template>
  <!-- A new event (docs/design.md, 「The console」): the layer's one create button, which stands at the
       toolbar's right end on a wide screen and floats on a phone, wrapped by a popover that draws a new
       event on the day the screen stands on, or reads one out of a phrase that names a day and a time and
       draws it where it says, its form open to put right what was read. The button's own key presses it. -->
  <UPopover v-model:open="open" :content="content" :reference="anchor" :ui="{ content: 'w-80 max-w-[calc(100vw-2rem)] p-2' }">
    <CreateButton ref="button" :label="$t('calendar.action.new_event')" />

    <template #content>
      <UButton icon="i-material-symbols-calendar-add-on-outline" :label="$t('calendar.action.new_event')" color="neutral" variant="ghost" block class="justify-start" @click="handOff(() => createAtAnchor())" />

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
import { computed, ref, useTemplateRef, watch } from "vue";
import CreateButton from "bmo-ui/app/components/CreateButton.vue";
import { useNarrow } from "bmo-ui/app/domain/screen";

import { parseQuickEvent } from "../../domain/quick";
import { useEventDraft } from "../../composables/useEventDraft";

const { createAtAnchor, createFromQuick } = useEventDraft();

const narrow = useNarrow();
const button = useTemplateRef<InstanceType<typeof CreateButton>>("button");

const open = ref(false);
const text = ref("");

// The popover stands against the button itself and hands the focus back to it: what the popover takes for
// its trigger is the cell the button stands in on a wide screen, and on a phone, where the button floats
// in a layer of its own, the place it was set in the screen, and neither takes the focus.
const anchor = computed(() => button.value?.element() ?? undefined);

// The draft's form takes the focus as it opens, and a press outside puts the focus where it landed; in
// neither case does the closing popover take it back to the button. The library may call its closing
// handler more than once, so what happened is read afresh each time the popover opens.
let handedOff = false;
let pressedOutside = false;

watch(open, (opened) => {
  if (!opened) return;

  handedOff = false;
  pressedOutside = false;
});

const content = computed(
  () =>
    ({
      side: narrow.value ? "top" : "bottom",
      align: "end",
      // At the toolbar's right end the button stands only a cell's inner room from the screen's edge; the
      // popover may come as near the edge as the button does, so it ends on the button's own edge.
      collisionPadding: 4,
      onInteractOutside: () => {
        pressedOutside = true;
      },
      onCloseAutoFocus: (event: Event) => {
        event.preventDefault();

        if (!handedOff && !pressedOutside) anchor.value?.focus();
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
