import type { CalendarColor } from "../../shared/types";

// The class maps a calendar's colour is drawn with, written whole so the stylesheet sees every class.
// The colour is carried by the fill and the stripe; the words stay in the page's own text colour, so they
// read on every colour in either mode.

/** A block on the time grid and an all-day bar: a tint of the colour, darker under the pointer and while held. */
export const EVENT_FILL: Record<CalendarColor, string> = {
  primary: "bg-primary/15 hover:bg-primary/25 active:bg-primary/25 data-active:bg-primary/25",
  secondary: "bg-secondary/15 hover:bg-secondary/25 active:bg-secondary/25 data-active:bg-secondary/25",
  info: "bg-info/15 hover:bg-info/25 active:bg-info/25 data-active:bg-info/25",
  success: "bg-success/15 hover:bg-success/25 active:bg-success/25 data-active:bg-success/25",
  warning: "bg-warning/15 hover:bg-warning/25 active:bg-warning/25 data-active:bg-warning/25",
  error: "bg-error/15 hover:bg-error/25 active:bg-error/25 data-active:bg-error/25"
};

/** A timed chip in a month cell on a phone, where the dot gives way: the tint carries the colour. */
export const CHIP_COMPACT_FILL: Record<CalendarColor, string> = {
  primary: "max-lg:bg-primary/15",
  secondary: "max-lg:bg-secondary/15",
  info: "max-lg:bg-info/15",
  success: "max-lg:bg-success/15",
  warning: "max-lg:bg-warning/15",
  error: "max-lg:bg-error/15"
};

/** The dot, the stripe and the swatch. */
export const CALENDAR_DOT: Record<CalendarColor, string> = {
  primary: "bg-primary",
  secondary: "bg-secondary",
  info: "bg-info",
  success: "bg-success",
  warning: "bg-warning",
  error: "bg-error"
};

/** The mark a received schedule carries beside its realm's, and its source carries in the sidebar. */
export const RECEIVED_ICON = "i-material-symbols-link";

/** The mark a realm's heading carries where the reader reads it and writes nothing in it. */
export const READ_ONLY_ICON = "i-material-symbols-lock-outline";

/** The mark that spins while a change is on its way. */
export const SENDING_ICON = "i-material-symbols-progress-activity";

/** The mark a realm's event carries beside its title, so the two realms are told apart on one grid. */
export const REALM_ICON = {
  mine: "i-material-symbols-person-outline",
  organization: "i-material-symbols-groups-outline"
} as const;
