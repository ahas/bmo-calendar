<template>
  <!-- The company's resources by kind (docs/design.md, 「The console」): each with its place, its seats,
       whether a booking of it waits for approval, and its keepers. Where the reader manages them, a resource
       is changed or deleted from its row, and added from the screen's one create button (`add`). -->
  <div class="min-h-0 flex-1 overflow-y-auto pb-[var(--bmo-create-room)]">
    <template v-if="resourcesRead && resources.length">
      <section v-for="kind in kinds" :key="kind" :aria-label="kind">
        <h3 class="flex h-8 items-center border-b border-default bg-elevated/50 px-4 text-sm font-semibold text-highlighted">{{ kind }}</h3>

        <ul class="divide-y divide-default border-b border-default">
          <li v-for="resource in resources.filter((one) => one.kind === kind)" :key="resource.id" class="flex items-center gap-3 px-4 py-2 pointer-coarse:py-3">
            <UIcon :name="BOOKING_ICON" class="size-5 shrink-0 text-muted" />

            <div class="flex min-w-0 flex-1 flex-col">
              <span class="truncate font-medium text-highlighted">{{ resource.name }}</span>
              <span class="truncate text-sm text-muted">{{ termsOf(resource) }}</span>
              <span v-if="resource.keepers.length" class="truncate text-sm text-muted">{{ $t("calendar.label.keepers_named", { names: resource.keepers.map(nameOf).join(", ") }) }}</span>
            </div>

            <div v-if="manages" class="flex shrink-0 items-center gap-1">
              <IconButton icon="i-material-symbols-edit-outline" :label="$t('calendar.aria.edit_named', { name: resource.name })" @click="edit(resource)" />
              <IconButton icon="i-material-symbols-delete-outline" :label="$t('calendar.aria.retire_named', { name: resource.name })" :loading="retiring === resource.id" @click="retire(resource)" />
            </div>
          </li>
        </ul>
      </section>
    </template>

    <p v-else-if="resourcesRead && resourcesFailed" role="alert" class="badge-ink px-4 py-3 text-sm text-error">{{ $t("calendar.error.resources_failed") }}</p>
    <p v-else-if="resourcesRead" class="px-4 py-3 text-sm text-muted">{{ $t("calendar.message.no_resources") }}</p>
    <div v-else class="flex flex-col gap-3 px-4 py-3" role="status" :aria-label="$t('calendar.status.loading')">
      <USkeleton v-for="row in 3" :key="row" class="h-10 w-full rounded-md" />
    </div>

    <ResourceDialog v-model:open="editing" :resource="chosen" :kind="props.kind" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, shallowRef } from "vue";
import { useI18n } from "vue-i18n";
import { useIsland } from "bmo-ui/app/domain/island";
import { countText } from "bmo-ui/app/domain/numbers";
import { askQuestion } from "bmo-ui/app/domain/question";

import ResourceDialog from "./ResourceDialog.vue";
import { BOOKING_ICON, kindsOf, resourceTerms } from "../../domain/bookings";
import { refusalText } from "../../domain/store";
import { useBookings } from "../../composables/useBookings";
import { usePeople } from "../../composables/usePeople";

import type { Resource } from "../../../shared/types";

/** The kind a new resource is first given: the one the screen shows. */
const props = defineProps<{ kind?: string | null }>();

const { t } = useI18n();
const island = useIsland(t);
const { resources, resourcesRead, resourcesFailed, manages, retireResource } = useBookings();
const { nameOf } = usePeople();

const chosen = shallowRef<Resource | null>(null);
const editing = ref(false);
const retiring = ref<string | null>(null);

const kinds = computed(() => kindsOf(resources.value));

function termsOf(resource: Resource): string {
  return resourceTerms(t, resource, countText);
}

function edit(resource: Resource | null): void {
  chosen.value = resource;
  editing.value = true;
}

async function retire(resource: Resource): Promise<void> {
  const taken = await askQuestion({
    title: t("calendar.message.retire_resource"),
    item: resource.name,
    effect: t("calendar.message.retire_resource_effect"),
    confirm: t("calendar.action.retire")
  });

  if (!taken) return;

  retiring.value = resource.id;

  try {
    await retireResource(resource);
  } catch (error) {
    island.say({ tone: "error", title: refusalText(t, error) });
  } finally {
    retiring.value = null;
  }
}

defineExpose({ add: () => edit(null) });
</script>
