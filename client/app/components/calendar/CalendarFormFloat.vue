<template>
  <!-- A form floating over the screen: a popover beside what opened it where there is room, and on a phone
       a centred dialog, so neither the screen's edge nor the keyboard cuts it (`docs/design.md`, 「The
       console」). What opens it is either its trigger, pressed to open it, or an anchor it only stands by. -->
  <template v-if="narrow">
    <slot v-if="props.anchor" />

    <UModal :open="props.open" :dismissible="props.dismissible" :title="props.title" @update:open="emit('update:open', $event)" @close:prevent="emit('close:prevent')">
      <template v-if="!props.anchor" #default>
        <slot />
      </template>

      <template #close="{ ui }">
        <ShellClose :class="ui.close()" @close="close" />
      </template>

      <template #body>
        <slot name="content" />
      </template>
    </UModal>
  </template>
  <UPopover
    v-else
    :open="props.open"
    :dismissible="props.dismissible"
    :content="props.content"
    :ui="{ content: `${props.width} max-w-[calc(100vw-2rem)] p-4` }"
    @update:open="emit('update:open', $event)"
    @close:prevent="emit('close:prevent')"
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
import ShellClose from "bmo-ui/app/components/ShellClose.vue";
import { useNarrow } from "bmo-ui/app/domain/screen";

import type { PopoverProps } from "@nuxt/ui";

interface Props {
  open: boolean;
  /** Whether a press outside or Escape may close it; where not, it asks first through `close:prevent`. */
  dismissible: boolean;
  /** The dialog's title on a phone, where it stands apart from what opened it. */
  title: string;
  /** Where the popover stands beside what opened it. */
  content?: PopoverProps["content"];
  /** The popover's width class. */
  width?: string;
  /** The default slot is an anchor it stands by, not a trigger that opens it. */
  anchor?: boolean;
}

const props = withDefaults(defineProps<Props>(), { content: undefined, width: "w-[26rem]", anchor: false });

const emit = defineEmits<{
  "update:open": [open: boolean];
  "close:prevent": [];
}>();

const narrow = useNarrow();

function close(): void {
  if (props.dismissible) emit("update:open", false);
  else emit("close:prevent");
}
</script>
