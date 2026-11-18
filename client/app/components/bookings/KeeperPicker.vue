<template>
  <!-- A resource's keepers, chosen among the company's people through the console's people directory and
       each drawn by the name it answers; a keeper is taken off by its own mark. The directory answers the
       reader's own company alone, so where the console offers none, or the administrator keeps another
       company's resources, keepers are written by their account keys. -->
  <div class="flex min-w-0 flex-col gap-2">
    <ul v-if="model.length" class="flex flex-wrap gap-1">
      <li v-for="user in model" :key="user">
        <UBadge color="neutral" variant="soft" size="lg" class="gap-0.5 py-0 pe-0">
          <span class="truncate">{{ nameOf(user) }}</span>
          <IconButton size="xs" icon="i-material-symbols-close" :label="$t('calendar.aria.remove_named', { name: nameOf(user) })" @click="remove(user)" />
        </UBadge>
      </li>
    </ul>

    <ChoiceSelect
      v-if="searchable"
      :id="props.id"
      :model-value="undefined"
      :items="[]"
      :find="find"
      value-key="value"
      :placeholder="$t('calendar.placeholder.keeper_search')"
      class="w-full"
      @update:model-value="add"
    />
    <UInputTags v-else :id="props.id" v-model="model" :placeholder="$t('calendar.placeholder.keeper_keys')" class="w-full" />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

import { useCompanies } from "../../composables/useCompanies";
import { usePeople } from "../../composables/usePeople";

import type { SelectMenuItem } from "@nuxt/ui";

const props = defineProps<{ id?: string }>();

/** The keepers' account keys. */
const model = defineModel<string[]>({ default: () => [] });

const { found, nameOf, search } = usePeople();
const { current } = useCompanies();

const searchable = computed(() => found && (current.value?.own ?? true));

/** The people the directory finds for the words typed, those already keeping it left out. */
async function find(words: string): Promise<SelectMenuItem[]> {
  const people = await search(words);

  return people
    .filter((person) => !model.value.includes(person.user))
    .map((person) => ({ label: person.name || person.user, value: person.user, description: [person.organization, person.jobTitle].filter(Boolean).join(" · ") }));
}

function add(user: unknown): void {
  if (typeof user !== "string" || !user || model.value.includes(user)) return;

  model.value = [...model.value, user];
}

function remove(user: string): void {
  model.value = model.value.filter((one) => one !== user);
}
</script>
