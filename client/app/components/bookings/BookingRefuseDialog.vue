<template>
  <!-- A keeper's refusal of a requested booking: the reason the booker is told, written before the refusal
       goes. Something to fill in, so a centred dialog rather than the message pill. -->
  <UModal v-model:open="open" :title="$t('calendar.message.refuse_booking')" :description="props.booking" :ui="{ content: 'sm:max-w-md' }">
    <template #close="{ ui }">
      <ShellClose :class="ui.close()" @close="emit('close', null)" />
    </template>

    <template #body>
      <form :id="formId" @submit.prevent="refuse">
        <UFormField :label="$t('calendar.label.reason')">
          <GrowingTextarea v-model="reason" :maxlength="500" class="w-full" />
        </UFormField>
      </form>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-end gap-2">
        <UButton :label="$t('common.action.cancel')" color="neutral" variant="ghost" @click="emit('close', null)" />
        <UButton type="submit" :form="formId" :label="$t('calendar.action.refuse')" color="error" />
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { ref, useId } from "vue";
import GrowingTextarea from "bmo-ui/app/components/GrowingTextarea.vue";
import ShellClose from "bmo-ui/app/components/ShellClose.vue";

/** The booking refused, by its title. */
const props = defineProps<{ booking: string }>();

const emit = defineEmits<{ close: [reason: string | null] }>();

const open = defineModel<boolean>("open", { default: false });

const formId = useId();
const reason = ref("");

function refuse(): void {
  emit("close", reason.value.trim());
}
</script>
