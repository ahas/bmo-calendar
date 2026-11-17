<template>
  <!-- The administrator's choice of company (docs/design.md, 「The console」): every company and the platform's
       own among them, the administrator's own first chosen. The organization's calendars, events, schedules,
       resources and bookings then read the company chosen. A member reads one company and sees no choice. -->
  <USelect
    v-if="administrator"
    :model-value="company ?? undefined"
    :items="items"
    :size="props.size"
    :aria-label="$t('calendar.label.company')"
    icon="i-material-symbols-domain"
    class="min-w-0"
    @update:model-value="chooseCompany(String($event))"
  />
</template>

<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";

import { useCompanies } from "../../composables/useCompanies";

const props = withDefaults(defineProps<{ size?: "sm" | "md" }>(), { size: "md" });

const { t } = useI18n();
const { companies, administrator, company, loadCompanies, chooseCompany } = useCompanies();

const items = computed(() => companies.value.map((one) => ({ value: one.id, label: one.own ? t("calendar.label.own_company", { name: one.name }) : one.name })));

onMounted(() => void loadCompanies());
</script>
