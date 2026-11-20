import { parseDate } from "@internationalized/date";
import { computed, ref, shallowRef } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";

import { addressedCompany } from "../domain/companies";
import { formatMonth, formatRangeTitle, formatYear, rangeFor, setDateLocale, toDate, todayDate } from "../domain/dates";
import { CALENDAR_VIEWS } from "../../shared/types";
import { createAppComposable } from "./shared";

import type { CalendarDate } from "@internationalized/date";
import type { RangeTitle } from "../domain/dates";
import type { CalendarView, DateRange } from "../../shared/types";

// Where the screen stands: the view and the day in its address (`/calendar/week/2026-11-03`), the span
// they cover, the title naming it, and the steps back and forward, each keeping the company the address names
// (`?company=<id>`).

/** The screen's path for a day in a view, naming the company where one is named. */
export function calendarPath(target: CalendarDate, view: CalendarView, company: string | null = null): string {
  const path = `/calendar/${view}/${target.toString()}`;

  return company === null ? path : `${path}?${new URLSearchParams({ company }).toString()}`;
}

const _useCalendar = () => {
  const route = useRoute();
  const { locale } = useI18n();

  setDateLocale(locale.value);

  const view = computed<CalendarView>(() => {
    const asked = route.params.view as string;

    return (CALENDAR_VIEWS as readonly string[]).includes(asked) ? (asked as CalendarView) : "month";
  });

  const date = computed<CalendarDate>(() => {
    try {
      return parseDate(route.params.date as string);
    } catch {
      return todayDate();
    }
  });

  const range = computed<DateRange>(() => rangeFor(view.value, date.value));

  /** The company the address names, which every step keeps; none where it names none. */
  const company = computed(() => addressedCompany(route.query));

  // The month standing at the top of the month view's scroll, which the title follows while it scrolls.
  const visibleMonth = shallowRef<CalendarDate | null>(null);

  const title = computed<RangeTitle>(() => {
    if (view.value === "year") return { months: formatYear(toDate(date.value)), year: "" };

    if (view.value === "month") {
      const focus = visibleMonth.value ?? date.value;

      return { months: formatMonth(toDate(focus)), year: formatYear(toDate(focus)) };
    }

    return formatRangeTitle(range.value);
  });

  // The day view has one column as wide as the grid, so a form has no side to stand beside it on.
  const formSide = computed<"bottom" | "right">(() => (view.value === "day" ? "bottom" : "right"));

  const step = computed(() => {
    switch (view.value) {
      case "year":
        return { years: 1 };
      case "month":
        return { months: 1 };
      case "week":
        return { days: 7 };
      default:
        return { days: 1 };
    }
  });

  const prevDate = computed(() => date.value.subtract(step.value));
  const nextDate = computed(() => date.value.add(step.value));

  function pathFor(target: CalendarDate, targetView: CalendarView = view.value): string {
    return calendarPath(target, targetView, company.value);
  }

  function go(direction: "left" | "right"): void {
    void navigateTo(pathFor(direction === "left" ? prevDate.value : nextDate.value));
  }

  // The calendars, as a side region beside the grid where there is room and a drawer on a phone.
  const isSidebarOpen = ref(false);

  return { view, date, range, company, title, visibleMonth, formSide, prevDate, nextDate, pathFor, go, isSidebarOpen };
};

export const useCalendar = createAppComposable(_useCalendar);
