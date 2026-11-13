// The calendar service's answers as the screen reads them (`docs/api.md`, 「Calendars」 and 「Events」),
// and the screen's own shapes built on them. Times the service names as local ISO datetimes stay that
// shape, `YYYY-MM-DDTHH:mm:ss`, so a 09:00 appointment is 09:00 wherever it is read.

export type Realm = "mine" | "organization";

export const REALMS: readonly Realm[] = ["mine", "organization"];

export type CalendarColor = "primary" | "secondary" | "info" | "success" | "warning" | "error";

export const CALENDAR_COLORS: readonly CalendarColor[] = ["primary", "secondary", "info", "success", "warning", "error"];

export type Writers = "administrator" | "creator" | "everyone";

export type Tell = "creator" | "everyone" | "nobody";

export type CalendarView = "day" | "week" | "month" | "year";

export const CALENDAR_VIEWS: readonly CalendarView[] = ["day", "week", "month", "year"];

/** Minutes before the start an alarm is told at, written as a string, or none. */
export type Alarm = "none" | "0" | "5" | "15" | "30" | "60" | "1440";

export type Repeat = "none" | "daily" | "weekly" | "monthly" | "yearly";

export interface Calendar {
  id: string;
  realm: Realm;
  name: string;
  color: CalendarColor;
  /** Who writes it and who is told; null in the `mine` realm. */
  writers: Writers | null;
  tell: Tell | null;
  hidden: boolean;
  creator: string;
  createdAt: number;
}

/** One row of an event as the service answers it; a repeating event answers one row per occurrence. */
export interface CalendarEventRow {
  id: string;
  realm: Realm;
  calendarId: string;
  title: string;
  description: string;
  start: string;
  end: string;
  allDay: boolean;
  alarm: Alarm;
  repeat: Repeat;
  creator: string;
  createdAt: number;
  changedAt: number;
  told: string | null;
  /** In a span's rows, the start of the occurrence a repeating event's row stands for; null otherwise. */
  occurrence?: string | null;
}

/** A source another unit writes its schedules under (`docs/api.md`, 「Sources and schedules」). */
export interface Source {
  id: string;
  name: string;
  color: CalendarColor;
  /** Whether the reader hid its schedules in the realm it was read in. */
  hidden: boolean;
  createdAt: number;
}

/** One schedule another unit pushed, as a realm's read answers it. */
export interface ScheduleRow {
  sourceId: string;
  itemId: string;
  realm: Realm;
  title: string;
  start: string;
  end: string;
  allDay: boolean;
  /** Where the item it stands for opens: a console path, or an `http` or `https` address. */
  link: string;
  assignee: string | null;
  createdAt: number;
  changedAt: number;
}

/** Where a row received from another unit came from and opens: never written from the screen. */
export interface Origin {
  sourceId: string;
  itemId: string;
  link: string;
}

/** A row as the screen holds it: the key one row is known by on the grid, across both realms, and, for a
 *  received schedule, its origin. */
export interface CalendarEvent extends CalendarEventRow {
  key: string;
  origin?: Origin;
}

/** What a write names (`docs/api.md`, 「Events」). */
export interface EventWrite {
  id?: string;
  calendarId: string;
  title: string;
  description?: string;
  start: string;
  end: string;
  allDay?: boolean;
  alarm?: Alarm;
  repeat?: Repeat;
  occurrence?: string;
}

export interface CalendarWrite {
  name?: string;
  color?: CalendarColor;
  writers?: Writers;
  tell?: Tell;
  hidden?: boolean;
}

export interface ListPage<T> {
  items: T[];
  more: boolean;
}

export interface DateRange {
  start: Date;
  end: Date;
}

/** Which rows of a repeating event a change or a cancellation reaches. */
export type RepeatScope = "one" | "all";
