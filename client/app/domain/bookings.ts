import { addDays, addMinutes, differenceInMinutes, startOfDay } from "date-fns";

import { dayKey, fromLocalISO, toLocalISO } from "./dates";

import type { Booking, BookingState, CalendarColor, DateRange, Resource } from "../../shared/types";

// How the booking screen lays a company's resources and their bookings out (`docs/design.md`, 「Resources
// and bookings」 and 「The console」): the resources of one kind grouped by place, each one's bookings on its
// row across the hours, how many each holds on a day of the month, and the free span a press or a drag on
// a row offers. A requested or booked booking holds its span; a refused one holds none, and a span ends
// where the next may begin.

/** The states a booking holds its span in. */
export const STANDING_STATES: readonly BookingState[] = ["requested", "booked"];

/** The words of each state and the mark it is shown by in a list. */
export const STATE_LABELS: Record<BookingState, string> = {
  requested: "calendar.label.state_requested",
  booked: "calendar.label.state_booked",
  refused: "calendar.label.state_refused"
};

export const STATE_ICONS: Record<BookingState, string> = {
  requested: "i-material-symbols-hourglass-top",
  booked: "i-material-symbols-check-circle-outline",
  refused: "i-material-symbols-block"
};

export const STATE_COLORS: Record<BookingState, string> = {
  requested: "text-warning",
  booked: "text-success",
  refused: "text-error"
};

/** The mark a booking carries on the calendar's grid and a resource in the booking screen. */
export const BOOKING_ICON = "i-material-symbols-meeting-room-outline";

/** The colour the reader's own bookings are drawn in on the calendar's grid. */
export const BOOKING_COLOR = "success" as const;

export function standing(booking: Pick<Booking, "state">): boolean {
  return STANDING_STATES.includes(booking.state);
}

/** The kinds the resources are grouped under, in the order the service lists them (by kind, place and name). */
export function kindsOf(resources: readonly Resource[]): string[] {
  return [...new Set(resources.map((resource) => resource.kind))];
}

export interface PlaceGroup {
  place: string;
  resources: Resource[];
}

/** The resources of one kind, grouped by their place in the order the service lists them. */
export function placesOf(resources: readonly Resource[], kind: string): PlaceGroup[] {
  const groups = new Map<string, Resource[]>();

  for (const resource of resources) {
    if (resource.kind !== kind) continue;

    const group = groups.get(resource.place);

    if (group) group.push(resource);
    else groups.set(resource.place, [resource]);
  }

  return [...groups].map(([place, held]) => ({ place, resources: held }));
}

/** A booking's span as moments. */
export function spanOf(booking: Pick<Booking, "start" | "end">): DateRange {
  return { start: fromLocalISO(booking.start), end: fromLocalISO(booking.end) };
}

/** The key one row of a booking is known by: its id and, for a repeating booking, its occurrence. */
export function bookingKey(booking: Pick<Booking, "id" | "occurrence">): string {
  return `${booking.id}:${booking.occurrence ?? ""}`;
}

/** The standing bookings of one resource that overlap a span; spans are half-open. */
export function standingOn(bookings: readonly Booking[], resourceId: string, span: DateRange): Booking[] {
  return bookings.filter((booking) => {
    if (booking.resourceId !== resourceId || !standing(booking)) return false;

    const held = spanOf(booking);

    return held.start < span.end && held.end > span.start;
  });
}

export interface RowBlock {
  booking: Booking;
  /** Where it starts and ends on the row, as shares of the row's span from 0 to 1. */
  from: number;
  to: number;
}

/** A resource's standing bookings laid on its row, which stands for a span, each cut to the span. */
export function rowBlocks(bookings: readonly Booking[], resourceId: string, span: DateRange): RowBlock[] {
  const whole = span.end.getTime() - span.start.getTime();

  if (whole <= 0) return [];

  return standingOn(bookings, resourceId, span).map((booking) => {
    const held = spanOf(booking);
    const start = Math.max(held.start.getTime(), span.start.getTime());
    const end = Math.min(held.end.getTime(), span.end.getTime());

    return { booking, from: (start - span.start.getTime()) / whole, to: (end - span.start.getTime()) / whole };
  });
}

/** The moment a share of a row's span stands at. */
export function momentAt(span: DateRange, share: number): Date {
  const clamped = Math.min(1, Math.max(0, share));

  return new Date(span.start.getTime() + clamped * (span.end.getTime() - span.start.getTime()));
}

/** The standing booking of the resource a span meets among the rows given, the booking `except` names left
 *  out; none where the rows hold none, as where the one met stands beyond what was read. */
export function metBooking(bookings: readonly Booking[], resourceId: string, span: DateRange, except?: string): Booking | undefined {
  return standingOn(bookings, resourceId, span).find((booking) => booking.id !== except);
}

/** Whether a span meets a standing booking of the resource, the booking `except` names left out. */
export function meets(bookings: readonly Booking[], resourceId: string, span: DateRange, except?: string): boolean {
  return standingOn(bookings, resourceId, span).some((booking) => booking.id !== except);
}

/** The free span around a moment on a resource's row, within the row's own span; none where a standing
 *  booking holds the moment. */
export function freeGap(bookings: readonly Booking[], resourceId: string, at: Date, bounds: DateRange): DateRange | null {
  let start = bounds.start;
  let end = bounds.end;

  for (const booking of standingOn(bookings, resourceId, bounds)) {
    const held = spanOf(booking);

    if (held.start <= at && at < held.end) return null;
    if (held.end <= at && held.end > start) start = held.end;
    if (held.start > at && held.start < end) end = held.start;
  }

  return { start, end };
}

function minuteOfDay(moment: Date): number {
  return differenceInMinutes(moment, startOfDay(moment));
}

function floorTo(moment: Date, snap: number): Date {
  const minutes = minuteOfDay(moment);

  return addMinutes(startOfDay(moment), Math.floor(minutes / snap) * snap);
}

function ceilTo(moment: Date, snap: number): Date {
  const floored = floorTo(moment, snap);

  return floored.getTime() === moment.getTime() ? moment : addMinutes(floored, snap);
}

function clampTo(moment: Date, gap: DateRange): Date {
  if (moment < gap.start) return gap.start;
  if (moment > gap.end) return gap.end;

  return moment;
}

/** The span a press offers: from the snapped moment under it, so long, kept inside the free gap. */
export function pressSpan(at: Date, gap: DateRange, snap: number, length: number): DateRange {
  const start = clampTo(floorTo(at, snap), gap);
  const wanted = addMinutes(start, length);

  return { start, end: wanted < gap.end ? wanted : gap.end };
}

/** The span a drag offers: from where it began to where it stands, snapped outward, kept inside the free gap. */
export function dragSpan(anchor: Date, current: Date, gap: DateRange, snap: number): DateRange {
  const from = anchor < current ? anchor : current;
  const to = anchor < current ? current : anchor;
  const start = clampTo(floorTo(from, snap), gap);
  let end = clampTo(ceilTo(to, snap), gap);

  if (end <= start) {
    const least = addMinutes(start, snap);

    end = least < gap.end ? least : gap.end;
  }

  return { start, end };
}

/** How many standing bookings each resource holds on each day: by resource id, a count per day given. */
export function dayCounts(bookings: readonly Booking[], days: readonly Date[]): Map<string, number[]> {
  const index = new Map(days.map((day, at) => [dayKey(day), at]));
  const counts = new Map<string, number[]>();

  for (const booking of bookings) {
    if (!standing(booking)) continue;

    const held = spanOf(booking);
    const row = counts.get(booking.resourceId) ?? new Array<number>(days.length).fill(0);
    let day = startOfDay(held.start);

    do {
      const at = index.get(dayKey(day));

      if (at !== undefined) row[at]! += 1;

      day = addDays(day, 1);
    } while (day < held.end);

    counts.set(booking.resourceId, row);
  }

  return counts;
}

/** A repeating booking's whole series moved by as much as one of its rows was: its start and its end each
 *  shifted by the minutes the row's were. */
export function shiftedSeries(series: Pick<Booking, "start" | "end">, before: Pick<Booking, "start" | "end">, after: { start: string; end: string }): { start: string; end: string } {
  const startShift = differenceInMinutes(fromLocalISO(after.start), fromLocalISO(before.start));
  const endShift = differenceInMinutes(fromLocalISO(after.end), fromLocalISO(before.end));

  return { start: toLocalISO(addMinutes(fromLocalISO(series.start), startShift)), end: toLocalISO(addMinutes(fromLocalISO(series.end), endShift)) };
}

/** Bookings by start, the earliest first; rows of one start by title. */
export function byStart(a: Pick<Booking, "start" | "title">, b: Pick<Booking, "start" | "title">): number {
  return a.start.localeCompare(b.start) || a.title.localeCompare(b.title);
}

type Translate = (key: string, params?: Record<string, unknown>) => string;

/** A resource's terms as a line names them: its place, its seats and whether a booking of it waits. */
export function resourceTerms(t: Translate, resource: Pick<Resource, "place" | "seats" | "approval">, count: (n: number) => string = String): string {
  const parts = [resource.place, t("calendar.label.seats_count", { n: count(resource.seats) })];

  if (resource.approval) parts.push(t("calendar.label.approval_needed"));

  return parts.filter(Boolean).join(" · ");
}

/** The hour a press on a week's day cell first looks from, so the working day is offered before the night. */
export const FIRST_FREE_FROM_HOUR = 9;

/**
 * The span a press on a resource's day cell offers: the day's first whole hour no standing booking meets,
 * looked for from the working morning to midnight and then from midnight to the morning, never before
 * `notBefore` (now, on today); none where the day has no free hour.
 */
export function firstFreeHour(bookings: readonly Booking[], resourceId: string, day: Date, notBefore?: Date): DateRange | null {
  const midnight = startOfDay(day);
  const hours = [...Array.from({ length: 24 - FIRST_FREE_FROM_HOUR }, (_, at) => FIRST_FREE_FROM_HOUR + at), ...Array.from({ length: FIRST_FREE_FROM_HOUR }, (_, at) => at)];

  for (const hour of hours) {
    const span = { start: addMinutes(midnight, hour * 60), end: addMinutes(midnight, (hour + 1) * 60) };

    if (notBefore && span.start < notBefore) continue;
    if (!meets(bookings, resourceId, span)) return span;
  }

  return null;
}

/** A resource's standing bookings that stand on a day, by start: a week's day cell. */
export function dayBookings(bookings: readonly Booking[], resourceId: string, day: Date): Booking[] {
  const midnight = startOfDay(day);

  return standingOn(bookings, resourceId, { start: midnight, end: addDays(midnight, 1) }).sort(byStart);
}

/**
 * One row for each booking, by its next occurrence: the rows a span's read answers, one per occurrence of a
 * repeating booking, brought down to the earliest of each booking, so a repeating booking stands once among
 * the rest (`docs/design.md`, 「The console」, my bookings).
 */
export function nextOfEach(rows: readonly Booking[]): Booking[] {
  const first = new Map<string, Booking>();

  for (const row of [...rows].sort(byStart)) if (!first.has(row.id)) first.set(row.id, row);

  return [...first.values()];
}

/** The colour a booking is drawn in on the booking screen's rows: a request waiting stands apart. */
export const STATE_TINT: Record<BookingState, CalendarColor> = {
  requested: "warning",
  booked: "info",
  refused: "error"
};
