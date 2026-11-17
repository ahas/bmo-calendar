import { PLATFORM_COMPANY } from "../../shared/types";

import type { Company } from "../../shared/types";

// The companies the reader reads (`docs/api.md`, 「Companies」). A member reads its own company alone; the
// platform administrator reads every company and the platform's own realm, which the service lists for
// the administrator alone, and so is what tells the administrator's list apart.

/** Whether the list is the administrator's: it names the platform's own realm. */
export function administers(companies: readonly Company[]): boolean {
  return companies.some((company) => company.id === PLATFORM_COMPANY);
}

/** The company first chosen: the reader's own, else the first listed; none where none is listed. */
export function firstCompany(companies: readonly Company[]): string | null {
  return (companies.find((company) => company.own) ?? companies[0])?.id ?? null;
}

/** The company the organization realm is read in: the one chosen, else the reader's own; none where the
 *  companies could not be read. */
export function currentCompany(companies: readonly Company[], chosen: string | null): Company | null {
  return companies.find((company) => company.id === chosen) ?? companies.find((company) => company.own) ?? null;
}

/**
 * Whether the reader books in the company read: only in the reader's own (`docs/api.md`, 「Resources and
 * bookings」), so not while the administrator reads another. Where the companies could not be read and none
 * is chosen, the reads naming none are the reader's own company's, and the service refuses the rest.
 */
export function booksIn(companies: readonly Company[], chosen: string | null): boolean {
  const current = currentCompany(companies, chosen);

  return current ? current.own : chosen === null;
}
