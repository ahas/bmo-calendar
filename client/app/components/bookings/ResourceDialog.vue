<template>
  <!-- A resource's form, or a new resource's, as a centred dialog (docs/design.md, 「The console」): its name,
       its kind, its place, its seats, whether a booking of it waits for approval, and its keepers. One save
       at its foot; a refusal stays in the form. Closed with a change held, it asks first: every request of the
       modal to close reaches the guard through its open state, and the guard alone refuses. -->
  <UModal :open="open" :title="title" :ui="{ content: 'sm:max-w-lg' }" @update:open="(standing: boolean) => (standing ? (open = true) : void leave())">
    <template #close="{ ui }">
      <!-- Pressed, it reaches the guard through the modal's own close alone, so it asks only once. -->
      <ShellClose :class="ui.close()" />
    </template>

    <template #body>
      <UForm :id="formId" :state="state" :validate="validate" class="flex flex-col gap-3" @submit="save">
        <div class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
          <label :for="nameId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.name") }}</label>
          <UFormField name="name" :ui="{ error: 'badge-ink' }">
            <UInput :id="nameId" v-model="state.name" :maxlength="100" :autofocus="!narrow" class="w-full" />
          </UFormField>

          <label :for="kindId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.kind") }}</label>
          <UFormField name="kind" :ui="{ error: 'badge-ink' }">
            <UInput :id="kindId" v-model="state.kind" :maxlength="100" :placeholder="$t('calendar.placeholder.kind')" class="w-full" />
          </UFormField>

          <label :for="placeId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.place") }}</label>
          <UInput :id="placeId" v-model="state.place" :maxlength="100" class="w-full" />

          <label :for="seatsId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.seats") }}</label>
          <NumberInput :id="seatsId" v-model="state.seats" :digits="5" :min="1" :max="MAX_SEATS" :unit="$t('calendar.label.seats_unit')" />

          <label :for="approvalId" class="whitespace-nowrap text-sm text-muted">{{ $t("calendar.label.approval") }}</label>
          <UCheckbox :id="approvalId" v-model="state.approval" class="justify-self-start" />

          <label :for="keepersId" class="self-start whitespace-nowrap pt-1.5 text-sm text-muted">{{ $t("calendar.label.keepers") }}</label>
          <UFormField name="keepers" :ui="{ error: 'badge-ink' }">
            <KeeperPicker :id="keepersId" v-model="state.keepers" />
          </UFormField>
        </div>

        <p v-if="refused" role="alert" class="badge-ink text-sm text-error">{{ refused }}</p>
      </UForm>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-end gap-2">
        <UButton type="submit" :form="formId" :label="$t('common.action.save')" :disabled="!dirty" :loading="saving" />
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { computed, reactive, ref, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
import NumberInput from "bmo-ui/app/components/NumberInput.vue";
import ShellClose from "bmo-ui/app/components/ShellClose.vue";
import { useCloseGuard } from "bmo-ui/app/domain/form-save";
import { useNarrow } from "bmo-ui/app/domain/screen";

import KeeperPicker from "./KeeperPicker.vue";
import { refusalText } from "../../domain/store";
import { useBookings } from "../../composables/useBookings";

import type { FormError } from "@nuxt/ui";
import type { Resource } from "../../../shared/types";

/** The most seats a resource holds (`docs/api.md`, 「Resources and bookings」). */
const MAX_SEATS = 10_000;

const props = defineProps<{ resource?: Resource | null; kind?: string | null }>();

const open = defineModel<boolean>("open", { default: false });

const { t } = useI18n();
const { addResource, changeResource } = useBookings();
const narrow = useNarrow();

const formId = useId();
const nameId = useId();
const kindId = useId();
const placeId = useId();
const seatsId = useId();
const approvalId = useId();
const keepersId = useId();

interface ResourceState {
  name: string;
  kind: string;
  place: string;
  seats: number;
  approval: boolean;
  keepers: string[];
}

function initialState(): ResourceState {
  const resource = props.resource;

  return {
    name: resource?.name ?? "",
    kind: resource?.kind ?? props.kind ?? "",
    place: resource?.place ?? "",
    seats: resource?.seats ?? 1,
    approval: resource?.approval ?? false,
    keepers: [...(resource?.keepers ?? [])]
  };
}

const state = reactive<ResourceState>(initialState());

const refused = ref("");
const saving = ref(false);
const saved = ref(JSON.stringify(state));

const dirty = computed(() => JSON.stringify(state) !== saved.value);
const title = computed(() => props.resource?.name ?? t("calendar.action.add_resource"));

// Opened again, on another resource or a new one, it stands on that one.
watch(open, (opened) => {
  if (!opened) return;

  Object.assign(state, initialState());
  saved.value = JSON.stringify(state);
  refused.value = "";
});

watch(state, () => (refused.value = ""), { deep: true });

function validate(data: ResourceState): FormError[] {
  const errors: FormError[] = [];

  if (!data.name.trim()) errors.push({ name: "name", message: t("calendar.error.no_name") });
  if (!data.kind.trim()) errors.push({ name: "kind", message: t("calendar.error.no_kind") });
  if (data.approval && data.keepers.length === 0) errors.push({ name: "keepers", message: t("calendar.error.no_keepers") });

  return errors;
}

async function save(): Promise<void> {
  if (saving.value) return;

  const write = { name: state.name.trim(), kind: state.kind.trim(), place: state.place.trim(), seats: state.seats, approval: state.approval, keepers: state.keepers };

  saving.value = true;
  refused.value = "";

  try {
    if (props.resource) await changeResource(props.resource, write);
    else await addResource({ ...write, id: crypto.randomUUID() });

    saved.value = JSON.stringify(state);
    open.value = false;
  } catch (error) {
    refused.value = refusalText(t, error);
  } finally {
    saving.value = false;
  }
}

const { leave } = useCloseGuard(open, () => dirty.value, t);
</script>
