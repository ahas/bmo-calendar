<template>
  <!-- A booking's form, or a new booking's, as a centred dialog: a small form, and no part of the page, its
       actions in the dialog's footer. Opened from the reader's own list it acts on the whole booking (`whole`).
       Closed with a change held, it asks first whether to throw the change away. -->
  <UModal v-model:open="open" :dismissible="dismissible" :title="title" :ui="{ content: 'sm:max-w-lg' }" @close:prevent="leave">
    <template #close="{ ui }">
      <ShellClose :class="ui.close()" @close="leave" />
    </template>

    <template #body>
      <BookingForm
        v-if="held"
        ref="form"
        :key="formKey"
        v-model:dirty="dirty"
        :form-id="formId"
        :booking="held.booking"
        :draft="held.draft"
        :whole="held.whole"
        :resources="resourcesFor(held.booking?.company ?? null)"
        @done="open = false"
        @escape="leave"
      />
    </template>

    <template v-if="formRef?.editable" #footer>
      <BookingActions
        :form="formId"
        :existing="!!held?.booking"
        :dirty="dirty"
        :saving="formRef.saving"
        :cancelling="formRef.cancelling"
        :size="coarse ? 'xl' : 'md'"
        @cancel="formRef.cancelIt()"
      />
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { useMediaQuery } from "@vueuse/core";
import { computed, ref, useId, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import ShellClose from "bmo-ui/app/components/ShellClose.vue";
import { useCloseGuard } from "bmo-ui/app/domain/form-save";

import BookingActions from "./BookingActions.vue";
import BookingForm from "./BookingForm.vue";
import { useBookings } from "../../composables/useBookings";

import type { BookingDraft } from "./BookingForm.vue";
import type { Booking } from "../../../shared/types";

const props = defineProps<{ booking?: Booking | null; draft?: BookingDraft | null; whole?: boolean }>();

const open = defineModel<boolean>("open", { default: false });

const formRef = useTemplateRef<InstanceType<typeof BookingForm>>("form");

const { t } = useI18n();
const coarse = useMediaQuery("(pointer: coarse)");
const formId = useId();
const { resourcesFor } = useBookings();

// What the dialog shows is kept until it is off the screen, so a closing dialog does not empty first.
const held = ref<{ booking?: Booking; draft?: BookingDraft; whole?: boolean } | null>(null);
const dirty = ref(false);

// A form for another booking, or another draft, is a form of its own.
const formKey = computed(() => (held.value?.booking ? `${held.value.booking.id}:${held.value.booking.occurrence ?? ""}` : `draft:${held.value?.draft?.resourceId}:${held.value?.draft?.start.getTime()}`));
const title = computed(() => (held.value?.booking ? held.value.booking.title : t("calendar.action.new_booking")));

watch(
  [open, () => props.booking, () => props.draft],
  ([opened]) => {
    if (!opened) return;

    held.value = { booking: props.booking ?? undefined, draft: props.draft ?? undefined, whole: props.whole };
  },
  { immediate: true }
);

const { dismissible, leave } = useCloseGuard(open, () => dirty.value, t);
</script>
