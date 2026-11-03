import { describe, expect, test } from "bun:test";

import { KOREAN_WORDS, parseQuickEvent } from "../app/domain/quick";

// The quick entry read by rules alone: a day, a time, a span of times or a length, and an all-day word
// found and cut out of the phrase, the rest its title. The Korean phrases are made of the data file's own
// words, so the test reads the phrases a person types without writing them here.

const NOW = new Date(2026, 10, 3, 10, 0); // a Tuesday morning
const w = KOREAN_WORDS;
const word = (map: Record<string, number>, days: number) => Object.entries(map).find(([, value]) => value === days)![0];

function at(day: number, hour: number, minute = 0): Date {
  return new Date(2026, 10, day, hour, minute);
}

describe("English", () => {
  test("a title, a day and a time", () => {
    expect(parseQuickEvent("Dentist tomorrow at 3pm", NOW)).toEqual({ title: "Dentist", start: at(4, 15), end: at(4, 16), allDay: false });
  });

  test("a day and no time is the whole day", () => {
    expect(parseQuickEvent("Gym friday", NOW)).toEqual({ title: "Gym", start: at(6, 0), end: at(7, 0), allDay: true });
  });

  test("a phrase with neither reads as none", () => {
    expect(parseQuickEvent("Call mum", NOW)).toBeNull();
  });
});

describe("Korean", () => {
  test("tomorrow, an afternoon hour and a title", () => {
    const phrase = `${word(w.relativeDays, 1)} ${w.afternoon[0]} 3${w.hour} X`;

    expect(parseQuickEvent(phrase, NOW)).toEqual({ title: "X", start: at(4, 15), end: at(4, 16), allDay: false });
  });

  test("a month and a day, all day", () => {
    const phrase = `11${w.month} 5${w.day} ${w.allDay[1]} X`;

    expect(parseQuickEvent(phrase, NOW)).toEqual({ title: "X", start: at(5, 0), end: at(6, 0), allDay: true });
  });

  test("a weekday of next week, an evening hour", () => {
    const phrase = `${w.nextWeek[0]} ${w.week} ${w.weekdays[5]}${w.weekdaySuffix} ${w.afternoon[1]} 7${w.hour} X`;

    expect(parseQuickEvent(phrase, NOW)).toEqual({ title: "X", start: at(13, 19), end: at(13, 20), allDay: false });
  });

  test("a span of two hours, the second unmarked", () => {
    const phrase = `${w.afternoon[0]} 3${w.hour}~5${w.hour} X`;

    expect(parseQuickEvent(phrase, NOW)).toEqual({ title: "X", start: at(3, 15), end: at(3, 17), allDay: false });
  });

  test("a length after the time, never read as a second time", () => {
    const phrase = `X ${w.afternoon[0]} 3${w.hour} 2${w.hours}`;

    expect(parseQuickEvent(phrase, NOW)).toEqual({ title: "X", start: at(3, 15), end: at(3, 17), allDay: false });
  });

  test("half past an hour", () => {
    const phrase = `${w.afternoon[0]} 4${w.hour} ${w.half} X`;

    expect(parseQuickEvent(phrase, NOW)).toEqual({ title: "X", start: at(3, 16, 30), end: at(3, 17, 30), allDay: false });
  });

  test("a time already gone today with no day named is tomorrow's", () => {
    const phrase = `${w.morning[0]} 8${w.hour} X`;

    expect(parseQuickEvent(phrase, NOW)?.start).toEqual(at(4, 8));
  });
});
