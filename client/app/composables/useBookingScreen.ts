import { parseDate } from "@internationalized/date";
import { computed } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";

import { addressedCompany } from "../domain/companies";
import { formatFullDate, formatMonth, formatMonthDay, formatRangeTitle, formatWeekday, formatYear, monthRange, setDateLocale, toDate, todayDate, weekRange } from "../domain/dates";

import type { CalendarDate } from "@internationalized/date";
import type { RangeTitle } from "../domain/dates";
import type { DateRange } from "../../shared/types";

// Where the booking screen stands, kept in its address (`/calendar/bookings?view=week&date=2026-11-03`), so a
// step, a view, a kind and a tab are each a step of the history and a kept link opens the same view: the
// part of the screen open, the view and the day of the resources' rows, and the kind of resource shown. Every
// move keeps the company the address names (`company`).

export const BOOKINGS_PATH = "/calendar/bookings";

export type BookingView = "day" | "week" | "month";

export const BOOKING_VIEWS: readonly BookingView[] = ["day", "week", "month"];

/** The parts of the screen: every booking of the company by resource, the reader's own, the requests on
 *  what the reader keeps, and the resources themselves. */
export type BookingTab = "all" | "mine" | "requests" | "resources";

export const BOOKING_TABS: readonly BookingTab[] = ["all", "mine", "requests", "resources"];

function oneOf<T extends string>(asked: unknown, choices: readonly T[], fallback: T): T {
  return typeof asked === "string" && (choices as readonly string[]).includes(asked) ? (asked as T) : fallback;
}

/** The span a view of the resources' rows covers on a day. */
export function bookingRange(view: BookingView, date: CalendarDate): DateRange {
  if (view === "month") return monthRange(date);

  return weekRange(date, view === "day" ? 1 : 7);
}

export function useBookingScreen() {
  const route = useRoute();
  const { locale } = useI18n();

  // Dates are written in the console's language, whichever screen of the bundle is opened first.
  setDateLocale(locale.value);

  const tab = computed<BookingTab>(() => oneOf(route.query.tab, BOOKING_TABS, "all"));
  const view = computed<BookingView>(() => oneOf(route.query.view, BOOKING_VIEWS, "day"));
  const kind = computed<string | null>(() => (typeof route.query.kind === "string" && route.query.kind ? route.query.kind : null));
  const company = computed(() => addressedCompany(route.query));

  const date = computed<CalendarDate>(() => {
    try {
      return parseDate(String(route.query.date ?? ""));
    } catch {
      return todayDate();
    }
  });

  const range = computed(() => bookingRange(view.value, date.value));

  const title = computed<RangeTitle>(() => {
    if (view.value === "month") return { months: formatMonth(toDate(date.value)), year: formatYear(toDate(date.value)) };
    if (view.value === "day") return { months: formatFullDate(toDate(date.value)), year: "" };

    return formatRangeTitle(range.value);
  });

  /** The title where the row has little room, a phone's: the month for the month view, else the day it stands
   *  on (a week's first) by its month, day and weekday. */
  const shortTitle = computed(() => {
    if (view.value === "month") return formatMonth(toDate(date.value));

    const day = view.value === "week" ? range.value.start : toDate(date.value);

    return `${formatMonthDay(day)} (${formatWeekday(day)})`;
  });

  const step = computed(() => (view.value === "month" ? { months: 1 } : view.value === "week" ? { days: 7 } : { days: 1 }));
  const prevDate = computed(() => date.value.subtract(step.value));
  const nextDate = computed(() => date.value.add(step.value));

  /** The screen's address with the parts named changed and the rest as they stand. */
  function pathFor(change: { tab?: BookingTab; view?: BookingView; date?: CalendarDate; kind?: string | null }): { path: string; query: Record<string, string> } {
    const query: Record<string, string> = {};
    const next = {
      tab: change.tab ?? tab.value,
      view: change.view ?? view.value,
      date: (change.date ?? date.value).toString(),
      kind: change.kind === undefined ? kind.value : change.kind
    };

    if (next.tab !== "all") query.tab = next.tab;
    if (next.view !== "day") query.view = next.view;
    if (next.date !== todayDate().toString()) query.date = next.date;
    if (next.kind) query.kind = next.kind;
    if (company.value !== null) query.company = company.value;

    return { path: BOOKINGS_PATH, query };
  }

  function go(change: Parameters<typeof pathFor>[0]): void {
    void navigateTo(pathFor(change));
  }

  return { tab, view, kind, company, date, range, title, shortTitle, prevDate, nextDate, pathFor, go };
}
