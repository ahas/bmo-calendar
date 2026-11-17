<template>
  <!-- A booking form's actions: the cancellation of a booking that stands, and its save, or a new booking's
       booking. Under the form in place; in a dialog's footer, naming the form they submit (`form`). -->
  <div class="flex w-full items-center justify-end gap-2">
    <UButton v-if="props.existing" :label="$t('calendar.action.cancel_booking')" color="error" variant="ghost" class="badge-ink" :size="props.size" :loading="props.cancelling" @click="emit('cancel')" />
    <UButton type="submit" :form="props.form" :label="props.existing ? $t('common.action.save') : $t('calendar.action.book')" :size="props.size" :disabled="props.existing && !props.dirty" :loading="props.saving" />
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  existing: boolean;
  dirty: boolean;
  saving: boolean;
  cancelling: boolean;
  size?: "md" | "xl";
  /** The id of the form the save submits, where the actions stand outside it. */
  form?: string;
}>();

const emit = defineEmits<{ cancel: [] }>();
</script>
