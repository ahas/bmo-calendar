import { computed, watch } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";

import { addressedCompany } from "../domain/companies";
import { useCompanies } from "./useCompanies";

import type { SideChoice } from "bmo-ui/app/domain/side-parts";

// The administrator's choice of company as a screen offers it in its side list (`docs/design.md`, 「The
// console」): the companies as the list's entries in the order the service lists them, his own named as his,
// the company read chosen, and a choice made by moving the screen's address to name it, a step of the history.
// The company read follows the address for as long as the screen stands, so a screen calls this before it
// reads anything of the organization realm. Until the companies are read the list stands as its skeleton; a member,
// and a reader whose companies did not answer, are then offered no list.

export function useCompanyChoice() {
  const route = useRoute();
  const { t } = useI18n();
  const { companies, companiesRead, administrator, company, followAddress } = useCompanies();

  watch(() => addressedCompany(route.query), followAddress, { immediate: true });

  /** The companies as the side list's entries; none until they are read. */
  const entries = computed<SideChoice[] | null>(() =>
    companiesRead.value ? companies.value.map((one) => ({ value: one.id, label: one.own ? t("calendar.label.own_company", { name: one.name }) : one.name })) : null
  );

  /** The screen's address, kept where it stands, naming the company chosen. */
  function choose(value: string | number): void {
    void navigateTo({ path: route.path, query: { ...route.query, company: String(value) } });
  }

  /** Whether the list stands: while the companies are unread, as its skeleton, and then for the administrator alone. */
  const listed = computed(() => !companiesRead.value || administrator.value);

  return { entries, chosen: company, listed, choose };
}
