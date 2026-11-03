import { addDays, addMinutes, addMonths, differenceInMinutes, endOfWeek, isSameDay, startOfWeek, subMilliseconds } from "date-fns";

import { WEEK_STARTS_ON, formatFullDate, formatMonthDay, formatTime, fromLocalISO } from "./dates";
import { REPEAT_CHOICES } from "../../shared/choices";

import type { CalendarEvent, CalendarView, DateRange } from "../../shared/types";

// What the screen says to a reader who does not see it, and how it moves for one who asked for less
// movement: the names of the days and the events, the span a step lands on, the day a key walks to, and
// the move of an event that keeps how long it lasts.

type Translate = (key: string, params?: Record<string, unknown>) => string;

/** A day of the grid by its full date, whether it is today, and how many events it holds. */
export function dayName(t: Translate, day: Date, count: number, today: Date, countText: (n: number) => string = String): string {
  const parts = [formatFullDate(day)];

  if (isSameDay(day, today)) parts.push(t("calendar.aria.today"));

  parts.push(count > 0 ? t("calendar.aria.events_count", { n: countText(count) }) : t("calendar.aria.events_none"));

  return parts.join(", ");
}

/** When an event happens, as words: its times, its days where it runs past one, or the all-day mark. */
export function eventWhen(t: Translate, event: Pick<CalendarEvent, "start" | "end" | "allDay">): string {
  const start = fromLocalISO(event.start);
  const end = fromLocalISO(event.end);

  if (event.allDay) {
    // The last day an all-day event covers is a moment before its end, the midnight after it.
    const last = subMilliseconds(end, 1);

    return isSameDay(start, last) ? t("calendar.label.all_day") : `${formatMonthDay(start)}~${formatMonthDay(last)}, ${t("calendar.label.all_day")}`;
  }

  if (isSameDay(start, end) || differenceInMinutes(end, start) <= 0) return `${formatTime(start)}~${formatTime(end)}`;

  return `${formatMonthDay(start)} ${formatTime(start)}~${formatMonthDay(end)} ${formatTime(end)}`;
}

/** An event's name to a reader who does not see it: its title, when, its calendar and its repeat. */
export function eventName(t: Translate, event: Pick<CalendarEvent, "title" | "start" | "end" | "allDay" | "repeat">, calendar: string | undefined): string {
  const parts = [event.title || t("calendar.label.untitled"), eventWhen(t, event)];

  if (calendar) parts.push(calendar);

  const repeat = REPEAT_CHOICES.find((choice) => choice.value === event.repeat);

  if (repeat && event.repeat !== "none") parts.push(t(repeat.label));

  return parts.join(", ");
}

/** The span a view shows, as the words said once a step has landed on it. */
export function rangeWords(view: CalendarView, range: DateRange, focus: Date, locale?: string): string {
  if (view === "year") return new Intl.DateTimeFormat(locale, { year: "numeric" }).format(focus);
  if (view === "month") return new Intl.DateTimeFormat(locale, { year: "numeric", month: "long" }).format(focus);
  if (view === "day") return formatFullDate(range.start);

  const last = addDays(range.end, -1);

  return new Intl.DateTimeFormat(locale, { year: "numeric", month: "long", day: "numeric" }).formatRange(range.start, last);
}

/** The keys that walk the days of a grid, and where each one lands from a day. */
export const DAY_KEYS = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown"] as const;

export type DayKey = (typeof DAY_KEYS)[number];

export function isDayKey(key: string): key is DayKey {
  return (DAY_KEYS as readonly string[]).includes(key);
}

export function stepDay(day: Date, key: DayKey): Date {
  switch (key) {
    case "ArrowLeft":
      return addDays(day, -1);
    case "ArrowRight":
      return addDays(day, 1);
    case "ArrowUp":
      return addDays(day, -7);
    case "ArrowDown":
      return addDays(day, 7);
    case "Home":
      return startOfWeek(day, { weekStartsOn: WEEK_STARTS_ON });
    case "End":
      return endOfWeek(day, { weekStartsOn: WEEK_STARTS_ON });
    case "PageUp":
      return addMonths(day, -1);
    case "PageDown":
      return addMonths(day, 1);
  }
}

/** How a scroll the screen makes on its own moves: at once for a reader who asked for less movement. */
export function scrollMotion(reduced: boolean): "auto" | "smooth" {
  return reduced ? "auto" : "smooth";
}

/** An event's end once its start is moved: it lasts as long as it did, so moving the start moves it. */
export function movedEnd(before: { start: Date; end: Date }, start: Date): Date {
  return addMinutes(start, differenceInMinutes(before.end, before.start));
}
