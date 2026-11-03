import { describe, expect, test } from "bun:test";

import { DAY_KEYS, dayName, eventName, eventWhen, isDayKey, movedEnd, rangeWords, scrollMotion, stepDay } from "../app/domain/a11y";
import { CALENDAR_DOT, CHIP_COMPACT_FILL, EVENT_FILL } from "../app/domain/colors";
import { formatFullDate, setDateLocale } from "../app/domain/dates";
import { COMPACT_MINUTES, DAY_MINUTES, MIN_EVENT_MINUTES, eventBlockStyle, layoutDay, minutesFromOffset, minutesInColumn, pxPerMinute } from "../app/domain/layout";
import { translator } from "./words";

import type { CalendarEvent } from "../shared/types";

// What the calendar says to a reader who does not see it, how the keyboard walks its days, how it moves for
// a reader who asked for less movement, how an event moves without a drag, and the grid's scale in the
// reader's own text size.

setDateLocale("en-US");

const t = translator("en");
const count = (n: number) => n.toLocaleString("en-US");

function event(extra: Partial<CalendarEvent>): CalendarEvent {
  return {
    key: "mine:dentist:",
    id: "dentist",
    realm: "mine",
    calendarId: "mine",
    title: "dentist",
    description: "",
    start: "2026-11-03T15:00:00",
    end: "2026-11-03T16:00:00",
    allDay: false,
    alarm: "none",
    repeat: "none",
    creator: "7",
    createdAt: 1,
    changedAt: 1,
    told: null,
    occurrence: null,
    ...extra
  };
}

describe("a day's name", () => {
  const today = new Date(2026, 10, 3, 9);

  test("is its full date, today's mark and how many events it holds", () => {
    const day = new Date(2026, 10, 3);

    expect(dayName(t, day, 3, today, count)).toBe(`${formatFullDate(day)}, ${t("calendar.aria.today")}, ${t("calendar.aria.events_count", { n: "3" })}`);
  });

  test("says when a day holds none, and carries no today's mark on another day", () => {
    const day = new Date(2026, 10, 4);

    expect(dayName(t, day, 0, today, count)).toBe(`${formatFullDate(day)}, ${t("calendar.aria.events_none")}`);
  });

  test("writes the count with its thousands separator", () => {
    expect(dayName(t, new Date(2026, 10, 4), 1234, today, count)).toEndWith(t("calendar.aria.events_count", { n: "1,234" }));
  });
});

describe("an event's name", () => {
  test("is its title, its times and its calendar", () => {
    expect(eventName(t, event({}), "calendar-a")).toBe("dentist, 15:00~16:00, calendar-a");
  });

  test("names its repeat, and an untitled event by the untitled word", () => {
    expect(eventName(t, event({ title: "", repeat: "weekly" }), "calendar-a")).toBe(`${t("calendar.label.untitled")}, 15:00~16:00, calendar-a, ${t("calendar.label.repeat_weekly")}`);
  });

  test("says all day for an all-day event of one day", () => {
    expect(eventWhen(t, event({ allDay: true, start: "2026-11-05T00:00:00", end: "2026-11-06T00:00:00" }))).toBe(t("calendar.label.all_day"));
  });

  test("names the first and the last day an all-day event covers, the last a day before its end", () => {
    const when = eventWhen(t, event({ allDay: true, start: "2026-11-05T00:00:00", end: "2026-11-07T00:00:00" }));

    expect(when).toStartWith("Nov 5~Nov 6");
    expect(when).toEndWith(t("calendar.label.all_day"));
  });

  test("names both days of a timed event running past midnight", () => {
    expect(eventWhen(t, event({ start: "2026-11-03T22:00:00", end: "2026-11-04T01:00:00" }))).toBe("Nov 3 22:00~Nov 4 01:00");
  });
});

describe("the span a step lands on", () => {
  test("is the day for the day view, the month for the month view and the year for the year view", () => {
    const day = new Date(2026, 10, 3);

    expect(rangeWords("day", { start: day, end: new Date(2026, 10, 4) }, day, "en-US")).toBe(formatFullDate(day));
    expect(rangeWords("month", { start: day, end: day }, day, "en-US")).toBe("November 2026");
    expect(rangeWords("year", { start: day, end: day }, day, "en-US")).toBe("2026");
  });

  test("is the first and the last day for the week view", () => {
    const words = rangeWords("week", { start: new Date(2026, 10, 1), end: new Date(2026, 10, 8) }, new Date(2026, 10, 3), "en-US");

    expect(words).toContain("November 1");
    expect(words).toContain("7, 2026");
  });
});

describe("the keys that walk the days", () => {
  const day = new Date(2026, 10, 4);

  test("step a day sideways, a week up and down, to the week's ends and a month either way", () => {
    expect(stepDay(day, "ArrowLeft")).toEqual(new Date(2026, 10, 3));
    expect(stepDay(day, "ArrowRight")).toEqual(new Date(2026, 10, 5));
    expect(stepDay(day, "ArrowUp")).toEqual(new Date(2026, 9, 28));
    expect(stepDay(day, "ArrowDown")).toEqual(new Date(2026, 10, 11));
    expect(stepDay(day, "Home")).toEqual(new Date(2026, 10, 1));
    expect(stepDay(day, "End").getDate()).toBe(7);
    expect(stepDay(day, "PageUp")).toEqual(new Date(2026, 9, 4));
    expect(stepDay(day, "PageDown")).toEqual(new Date(2026, 11, 4));
  });

  test("are those and no others", () => {
    expect(DAY_KEYS.every(isDayKey)).toBe(true);
    expect(isDayKey("Enter")).toBe(false);
    expect(isDayKey("n")).toBe(false);
  });
});

describe("movement", () => {
  test("a scroll the screen makes on its own glides only for a reader who did not ask for less movement", () => {
    expect(scrollMotion(true)).toBe("auto");
    expect(scrollMotion(false)).toBe("smooth");
  });

  test("moving an event's start moves its end by as much, across days as well", () => {
    const before = { start: new Date(2026, 10, 3, 12), end: new Date(2026, 10, 3, 13, 30) };

    expect(movedEnd(before, new Date(2026, 10, 3, 14))).toEqual(new Date(2026, 10, 3, 15, 30));
    expect(movedEnd(before, new Date(2026, 10, 5, 23))).toEqual(new Date(2026, 10, 6, 0, 30));
  });
});

describe("the time grid's scale", () => {
  test("lays events out in minutes, never in pixels", () => {
    const [placed] = layoutDay([event({})], new Date(2026, 10, 3));

    expect(placed).toMatchObject({ startMinute: 15 * 60, minutes: 60, left: 0, width: 100 });
    expect(layoutDay([event({ end: "2026-11-03T15:10:00" })], new Date(2026, 10, 3))[0]!.minutes).toBe(MIN_EVENT_MINUTES);
    expect(COMPACT_MINUTES).toBeGreaterThan(MIN_EVENT_MINUTES);
  });

  test("draws a block in the grid's own minute, so a larger text grows it", () => {
    const style = eventBlockStyle(layoutDay([event({})], new Date(2026, 10, 3))[0]!);

    expect(style.top).toBe("calc(calc(var(--calendar-minute) * 900) + 2px)");
    expect(style.height).toBe("calc(calc(var(--calendar-minute) * 60) - 3px)");
  });

  test("reads the minute a pointer stands at off the column's own height, at any text size", () => {
    for (const hourPx of [64, 96]) {
      const rect = { top: 100, height: 24 * hourPx } as DOMRect;

      expect(pxPerMinute(rect)).toBe(hourPx / 60);
      expect(minutesInColumn(100 + 9.5 * hourPx, rect)).toBe(9 * 60 + 30);
      expect(minutesFromOffset(hourPx, pxPerMinute(rect))).toBe(60);
    }

    expect(minutesInColumn(10_000, { top: 0, height: 24 * 64 } as DOMRect)).toBe(DAY_MINUTES);
  });
});

describe("an event's colour", () => {
  test("is a fill and a stripe, never the colour of its words, so they read on every colour in both themes", () => {
    for (const classes of [...Object.values(EVENT_FILL), ...Object.values(CHIP_COMPACT_FILL), ...Object.values(CALENDAR_DOT)]) {
      expect(classes.split(" ").filter((name) => /(^|:)text-/.test(name))).toEqual([]);
    }
  });
});
