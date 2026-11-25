import type { Origin } from "../../shared/types";

// Where a received schedule's detail leads (`docs/design.md`, 「The console」): to the item it stands for, a console
// path in the console itself and an address in a new tab, and only from the reader's own schedule, the one the
// service marks as assigned to the reader's account. Another's leads nowhere, since the item may not be the
// reader's to open.

/** The way to the item a received row stands for, as a button takes it. */
export interface OriginLink {
  to: string;
  external?: true;
  target?: "_blank";
}

/** The way a received row's detail offers to its item, or none: for another's row, and a row of the calendar's own. */
export function originLink(origin: Origin | undefined): OriginLink | null {
  if (!origin?.own) return null;

  return /^https?:\/\//i.test(origin.link) ? { to: origin.link, external: true, target: "_blank" } : { to: origin.link };
}
