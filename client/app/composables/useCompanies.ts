import { computed, ref } from "vue";

import { administers, booksIn, currentCompany, firstCompany, writesIn } from "../domain/companies";
import { createAppComposable } from "./shared";
import { useCalendarApi } from "./useCalendarApi";

import type { Company } from "../../shared/types";

// The administrator's choice of company (`docs/design.md`, 「The console」): the company the screen's address
// names, among the companies the reader reads, read once for both of the bundle's screens while the console
// stands; an address naming none, or one he does not read, reads his own. A member reads one company, its own,
// and is offered no choice and sends none, whatever the address names. Whether the reader writes in its
// organization realm, makes, changes and retires resources, and books, is the company read's.

const _useCompanies = () => {
  const { api, company } = useCalendarApi();

  const companies = ref<Company[]>([]);
  const companiesRead = ref(false);
  // How many times the company read has changed from one the reads already stood on; what holds rows of the
  // organization realm reads them again on each.
  const switches = ref(0);
  // The company the screen's address names, none where it names none.
  const asked = ref<string | null>(null);

  const administrator = computed(() => administers(companies.value));
  const chosen = computed(() => companies.value.find((one) => one.id === company.value) ?? null);
  /** The company the organization realm is read in: the one chosen, else the reader's own. */
  const current = computed(() => currentCompany(companies.value, company.value));
  /** Whether the reader makes, changes and retires the current company's resources. */
  const manages = computed(() => current.value?.manages ?? false);
  /** Whether the reader books in the current company (`booksIn`). */
  const books = computed(() => booksIn(companies.value, company.value));
  /** Whether the reader writes in the current company's organization realm (`writesIn`). */
  const writes = computed(() => writesIn(companies.value, company.value));

  // The first choice, the reader's own company, is what a read naming none already read: no change.
  function setCompany(next: string | null): void {
    if (next === company.value) return;

    const quiet = company.value === null && companies.value.find((one) => one.own)?.id === next;

    company.value = next;

    if (!quiet) switches.value += 1;
  }

  // The company read as the address asks: before the companies are read, the one it names, which the read
  // then settles; once they are, the one it names where the administrator reads it, else his own, and none
  // for anyone else, a list that did not answer among them, as a read naming none reads.
  function settle(): void {
    if (!companiesRead.value) setCompany(asked.value);
    else if (!administers(companies.value)) setCompany(null);
    else setCompany(companies.value.some((one) => one.id === asked.value) ? asked.value : firstCompany(companies.value));
  }

  let reading: Promise<void> | null = null;

  async function read(): Promise<void> {
    try {
      companies.value = await api.companies();
    } catch {
      // A list that did not answer leaves the reader on its own company (`settle`).
    } finally {
      companiesRead.value = true;
      reading = null;
      settle();
    }
  }

  /** The companies read, once for the console's stay unless asked again. */
  function loadCompanies(again = false): Promise<void> {
    if (reading) return reading;
    if (companiesRead.value && !again) return Promise.resolve();

    reading = read();

    return reading;
  }

  /** The company the screen's address names, which the company read follows. */
  function followAddress(id: string | null): void {
    asked.value = id;
    settle();
  }

  return { companies, companiesRead, administrator, company, chosen, current, manages, books, writes, switches, loadCompanies, followAddress };
};

export const useCompanies = createAppComposable(_useCompanies);
