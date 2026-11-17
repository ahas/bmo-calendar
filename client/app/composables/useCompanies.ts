import { computed, ref } from "vue";

import { administers, booksIn, currentCompany, firstCompany } from "../domain/companies";
import { createAppComposable } from "./shared";
import { useCalendarApi } from "./useCalendarApi";

import type { Company } from "../../shared/types";

// The administrator's choice of company (`docs/design.md`, 「The console」): read once from the companies
// the reader reads, the administrator's own first chosen, and kept for both of the bundle's screens while
// the console stands. A member reads one company, its own, and is offered no choice and sends none. Whether
// the reader makes, changes and retires resources, and whether the reader books, is the company read's.

const _useCompanies = () => {
  const { api, company } = useCalendarApi();

  const companies = ref<Company[]>([]);
  const companiesRead = ref(false);
  // How many times the company read has changed from one the reads already stood on; what holds rows of the
  // organization realm reads them again on each.
  const switches = ref(0);

  const administrator = computed(() => administers(companies.value));
  const chosen = computed(() => companies.value.find((one) => one.id === company.value) ?? null);
  /** The company the organization realm is read in: the one chosen, else the reader's own. */
  const current = computed(() => currentCompany(companies.value, company.value));
  /** Whether the reader makes, changes and retires the current company's resources. */
  const manages = computed(() => current.value?.manages ?? false);
  /** Whether the reader books in the current company (`booksIn`). */
  const books = computed(() => booksIn(companies.value, company.value));

  // The first choice, the reader's own company, is what a read naming none already read: no change.
  function setCompany(next: string | null): void {
    if (next === company.value) return;

    const quiet = company.value === null && companies.value.find((one) => one.own)?.id === next;

    company.value = next;

    if (!quiet) switches.value += 1;
  }

  let reading: Promise<void> | null = null;

  async function read(): Promise<void> {
    try {
      const listed = await api.companies();

      companies.value = listed;

      if (!administers(listed)) setCompany(null);
      else if (!listed.some((one) => one.id === company.value)) setCompany(firstCompany(listed));
    } catch {
      // A list that did not answer leaves the reader on its own company, as a read naming none reads.
    } finally {
      companiesRead.value = true;
      reading = null;
    }
  }

  /** The companies read, once for the console's stay unless asked again. */
  function loadCompanies(again = false): Promise<void> {
    if (reading) return reading;
    if (companiesRead.value && !again) return Promise.resolve();

    reading = read();

    return reading;
  }

  function chooseCompany(id: string): void {
    if (administrator.value && companies.value.some((one) => one.id === id)) setCompany(id);
  }

  return { companies, companiesRead, administrator, company, chosen, current, manages, books, switches, loadCompanies, chooseCompany };
};

export const useCompanies = createAppComposable(_useCompanies);
