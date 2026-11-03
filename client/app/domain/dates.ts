import { CalendarDate, Time, getLocalTimeZone, parseDate, toCalendarDateTime, today } from "@internationalized/date";
import { addDays, lightFormat, startOfMonth, startOfWeek } from "date-fns";

import type { CalendarView, DateRange } from "../../shared/types";

// The day arithmetic every view stands on. Times are the service's local ISO datetimes, read in the
// reader's own zone, so an appointment at 09:00 is 09:00 for every reader and an all-day event keeps its
// day. Ranges are [start, end): the end is the first instant left out.

/** The week starts on Sunday, as a Korean calendar does. */
export const WEEK_STARTS_ON = 0 as const;

/** How far the month view reaches either side of the current week, in weeks. */
export const WEEKS_AROUND = 260;

/** The month view's rows a route fetch covers: the grid's six weeks and the rows a tall screen shows below. */
export const MONTH_FETCH_WEEKS = 12;

/** The longest span one read may ask for (`docs/api.md`, 「Events」). */
export const MAX_SPAN_DAYS = 92;

/** A local ISO datetime of the minute a date stands at, seconds pinned to nought. */
export function toLocalISO(date: Date): string {
  return lightFormat(date, "yyyy-MM-dd'T'HH:mm:00");
}

/** A local ISO datetime read in the reader's zone. */
export function fromLocalISO(value: string): Date {
  return new Date(value.length === 10 ? `${value}T00:00:00` : value);
}

export function toCalendarDate(date: Date): CalendarDate {
  return new CalendarDate(date.getFullYear(), date.getMonth() + 1, date.getDate());
}

export function toTime(date: Date): Time {
  return new Time(date.getHours(), date.getMinutes());
}

let localTimeZone: string | undefined;

function timeZone(): string {
  return (localTimeZone ??= getLocalTimeZone());
}

export function toDate(date: CalendarDate): Date {
  return date.toDate(timeZone());
}

export function toDateTime(date: CalendarDate, time: Time): Date {
  return toCalendarDateTime(date, time).toDate(timeZone());
}

export function todayDate(): CalendarDate {
  return today(timeZone());
}

/** The local day a date falls on, as a bucket key. */
export function dayKey(date: Date): string {
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
}

/** The day a grid cell stands for, as the `data-date` a gesture reads back off it. */
export function isoDate(date: Date): string {
  return lightFormat(date, "yyyy-MM-dd");
}

let lastISO: string | undefined;
let lastDate: Date | undefined;

export function dateFromISO(iso: string): Date {
  if (iso !== lastISO) {
    lastISO = iso;
    lastDate = toDate(parseDate(iso));
  }

  return lastDate!;
}

/** The day under a point, from whichever cell stands topmost there. */
export function dateAtPoint(x: number, y: number): Date | null {
  for (const element of document.elementsFromPoint(x, y)) {
    const iso = (element as HTMLElement).dataset?.date;

    if (iso) return dateFromISO(iso);
  }

  return null;
}

export function weekRange(date: CalendarDate, days = 7): DateRange {
  const start = days === 7 ? startOfWeek(toDate(date), { weekStartsOn: WEEK_STARTS_ON }) : toDate(date);

  return { start, end: addDays(start, days) };
}

/** Six rows of seven days, so the grid's height never changes from one month to the next. */
export function monthRange(date: CalendarDate): DateRange {
  const start = startOfWeek(startOfMonth(toDate(date)), { weekStartsOn: WEEK_STARTS_ON });

  return { start, end: addDays(start, 42) };
}

export function monthFetchRange(date: CalendarDate): DateRange {
  const { start } = monthRange(date);

  return { start, end: addDays(start, MONTH_FETCH_WEEKS * 7) };
}

export function rangeFor(view: CalendarView, date: CalendarDate): DateRange {
  if (view === "month" || view === "year") return monthFetchRange(date);

  return weekRange(date, view === "day" ? 1 : 7);
}

export function eachDay({ start, end }: DateRange): Date[] {
  const days: Date[] = [];

  for (let day = start; day < end; day = addDays(day, 1)) days.push(day);

  return days;
}

// The formatters, made once per locale: making one costs far more than formatting with it, and these
// run once per chip and per day cell of every week drawn.
interface Formats {
  time: Intl.DateTimeFormat;
  weekday: Intl.DateTimeFormat;
  narrowWeekday: Intl.DateTimeFormat;
  fullDate: Intl.DateTimeFormat;
  month: Intl.DateTimeFormat;
  shortMonth: Intl.DateTimeFormat;
  year: Intl.DateTimeFormat;
  monthDay: Intl.DateTimeFormat;
}

let locale: string | undefined;
let formats: Formats | null = null;

/** The locale every date is written in: the console's own. */
export function setDateLocale(tag: string | undefined): void {
  if (tag === locale && formats !== null) return;

  locale = tag;
  formats = null;
}

function format(): Formats {
  return (formats ??= {
    time: new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit", hourCycle: "h23" }),
    weekday: new Intl.DateTimeFormat(locale, { weekday: "short" }),
    narrowWeekday: new Intl.DateTimeFormat(locale, { weekday: "narrow" }),
    fullDate: new Intl.DateTimeFormat(locale, { weekday: "long", month: "long", day: "numeric", year: "numeric" }),
    month: new Intl.DateTimeFormat(locale, { month: "long" }),
    shortMonth: new Intl.DateTimeFormat(locale, { month: "short" }),
    year: new Intl.DateTimeFormat(locale, { year: "numeric" }),
    monthDay: new Intl.DateTimeFormat(locale, { month: "short", day: "numeric" })
  });
}

export function formatTime(date: Date): string {
  return format().time.format(date);
}

export function formatHour(hour: number): string {
  return `${String(hour).padStart(2, "0")}:00`;
}

export function formatFullDate(date: Date): string {
  return format().fullDate.format(date);
}

export function formatWeekday(date: Date): string {
  return format().weekday.format(date);
}

export function formatNarrowWeekday(date: Date): string {
  return format().narrowWeekday.format(date);
}

export function formatMonth(date: Date): string {
  return format().month.format(date);
}

export function formatShortMonth(date: Date): string {
  return format().shortMonth.format(date);
}

export function formatYear(date: Date): string {
  return format().year.format(date);
}

export function formatMonthDay(date: Date): string {
  return format().monthDay.format(date);
}

export interface RangeTitle {
  months: string;
  year: string;
}

export function formatRangeTitle({ start, end }: DateRange): RangeTitle {
  const last = addDays(end, -1);
  const year = formatYear(last);

  if (start.getMonth() === last.getMonth()) return { months: formatMonth(start), year };

  return { months: `${formatShortMonth(start)} – ${formatShortMonth(last)}`, year };
}
