import type { Origin } from "../../shared/types";

// Where a received schedule opens (`docs/design.md`, 「The console」): the item it stands for, a console path
// in the console itself and an address in a new tab. The link keeps the event's own look, so it is drawn raw.

/** The link a received row is pressed through, or none for a row of the calendar's own. */
export function originLink(origin: Origin | undefined): Record<string, unknown> | null {
  if (!origin) return null;

  return /^https?:\/\//i.test(origin.link) ? { to: origin.link, external: true, target: "_blank", raw: true } : { to: origin.link, raw: true };
}
