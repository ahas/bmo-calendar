<template>
  <!-- A form floating over the screen: a popover beside what opened it where there is room, and on a phone
       a centred dialog, so neither the screen's edge nor the keyboard cuts it (`docs/design.md`, 「The
       console」). What opens it is either its trigger, pressed to open it, or an anchor it only stands by. It
       closes only through its open state: every way it is asked to close, its close, Esc, a press outside and
       the back gesture, reaches the screen as `update:open` with false, and the screen's close guard decides
       whether it goes. Neither the dialog nor the popover is told to refuse a close on its own, so a focus the
       dialog holds off by itself, as a menu handing it back to its trigger, is no request to close. -->
  <template v-if="narrow">
    <slot v-if="props.anchor" />

    <UModal :open="props.open" :title="props.title" :content="opening" @update:open="emit('update:open', $event)">
      <template v-if="!props.anchor" #default>
        <slot />
      </template>

      <template #close="{ ui }">
        <!-- Pressed, it reaches the screen through the modal's own close alone, so it asks only once. -->
        <ShellClose :class="ui.close()" />
      </template>

      <template #body>
        <slot name="content" />
      </template>
    </UModal>
  </template>
  <UPopover
    v-else
    :open="props.open"
    :content="popoverContent"
    :ui="{ content: `${props.width} max-w-[calc(100vw-2rem)] p-4` }"
    @update:open="emit('update:open', $event)"
  >
    <template v-if="props.anchor" #anchor>
      <slot />
    </template>
    <template v-else #default>
      <slot />
    </template>

    <template #content>
      <slot name="content" />
    </template>
  </UPopover>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ShellClose from "bmo-ui/app/components/ShellClose.vue";
import { useNarrow } from "bmo-ui/app/domain/screen";

import type { PopoverProps } from "@nuxt/ui";

interface Props {
  open: boolean;
  /** The dialog's title on a phone, where it stands apart from what opened it. */
  title: string;
  /** Where the popover stands beside what opened it. */
  content?: PopoverProps["content"];
  /** The popover's width class. */
  width?: string;
  /** The default slot is an anchor it stands by, not a trigger that opens it. */
  anchor?: boolean;
  /** Opening leaves the focus to the form inside rather than moving it to the form's first field. */
  keepFocus?: boolean;
}

const props = withDefaults(defineProps<Props>(), { content: undefined, width: "w-[26rem]", anchor: false, keepFocus: false });

const emit = defineEmits<{
  "update:open": [open: boolean];
}>();

const narrow = useNarrow();

// What opening does with the focus, handed to the dialog and the popover alike.
const opening = computed(() => ({
  onOpenAutoFocus(event: Event) {
    if (props.keepFocus) event.preventDefault();
  }
}));
const popoverContent = computed(() => ({ ...props.content, ...opening.value }));
</script>
