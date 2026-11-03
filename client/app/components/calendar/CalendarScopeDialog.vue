<template>
  <!-- Which rows of a repeating event a change or a cancellation reaches: this one occurrence, which then
       stands as an event of its own, or the whole series (`docs/api.md`, 「Events」). A choice of two ways
       through, so a centred dialog rather than the message pill. -->
  <UModal v-model:open="open" :title="props.title" :ui="{ content: 'sm:max-w-md' }">
    <template #close="{ ui }">
      <ShellClose :class="ui.close()" @close="emit('close', null)" />
    </template>

    <template #footer>
      <div class="flex w-full flex-wrap items-center justify-end gap-2">
        <UButton :label="$t('common.action.cancel')" color="neutral" variant="ghost" @click="emit('close', null)" />
        <UButton :label="$t('calendar.action.scope_one')" color="neutral" variant="outline" @click="emit('close', 'one')" />
        <UButton :label="$t('calendar.action.scope_all')" :color="props.cancelling ? 'error' : 'primary'" @click="emit('close', 'all')" />
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import ShellClose from "bmo-ui/app/components/ShellClose.vue";

import type { RepeatScope } from "../../../shared/types";

const props = defineProps<{ title: string; cancelling?: boolean }>();
const open = defineModel<boolean>("open", { default: false });

const emit = defineEmits<{ close: [scope: RepeatScope | null] }>();
</script>
