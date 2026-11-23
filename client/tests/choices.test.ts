import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import { ALARM_CHOICES, DEFAULT_ALARM, DEFAULT_REPEAT, DEFAULT_TELL, DEFAULT_WRITERS, REPEAT_CHOICES, TELL_CHOICES, WRITERS_CHOICES, choiceItems } from "../shared/choices";
import { PACKS, translator, word } from "./words";

// The alarm and repeat an event's form offers, and who writes and who is told on an organization's
// calendar: each a real value of the service's, the one the service takes where none is named marked as the
// default and no other, and no choice standing for "the default" in place of a value.

const t = translator("ko");

function marked(key: string): string {
  return word(PACKS.ko, "calendar.label.default_mark")!.replace("{value}", word(PACKS.ko, key)!);
}

describe("the alarm", () => {
  test("offers none, at the start and the five leads, in that order, as the service writes them", () => {
    expect(ALARM_CHOICES.map((choice) => choice.value)).toEqual(["none", "0", "5", "15", "30", "60", "1440"]);
  });

  test("marks none as the default, and only none", () => {
    const items = choiceItems(t, ALARM_CHOICES, DEFAULT_ALARM);

    expect(DEFAULT_ALARM).toBe("none");
    expect(items[0]).toEqual({ value: "none", label: marked("calendar.label.alarm_none") });
    expect(items.slice(1).map((item) => item.label)).toEqual(ALARM_CHOICES.slice(1).map((choice) => word(PACKS.ko, choice.label)!));
    expect(items.filter((item) => item.label.endsWith(word(PACKS.ko, "calendar.label.default_mark")!.replace("{value}", "")))).toHaveLength(1);
  });
});

describe("the repeat", () => {
  test("offers none, daily, weekly, monthly and yearly, as the service writes them", () => {
    expect(REPEAT_CHOICES.map((choice) => choice.value)).toEqual(["none", "daily", "weekly", "monthly", "yearly"]);
  });

  test("marks none as the default, and only none", () => {
    const items = choiceItems(t, REPEAT_CHOICES, DEFAULT_REPEAT);

    expect(DEFAULT_REPEAT).toBe("none");
    expect(items[0]).toEqual({ value: "none", label: marked("calendar.label.repeat_none") });
    expect(items.slice(1).every((item) => !item.label.includes("("))).toBe(true);
  });
});

describe("an organization's calendar", () => {
  test("marks the service's defaults for who writes and who is told", () => {
    expect(DEFAULT_WRITERS).toBe("creator");
    expect(DEFAULT_TELL).toBe("creator");
    expect(choiceItems(t, WRITERS_CHOICES, DEFAULT_WRITERS).find((item) => item.value === "creator")!.label).toBe(marked("calendar.label.writers_creator"));
    expect(choiceItems(t, TELL_CHOICES, DEFAULT_TELL).find((item) => item.value === "creator")!.label).toBe(marked("calendar.label.tell_creator"));
  });
});

describe("the event form", () => {
  const form = readFileSync(new URL("../app/components/calendar/CalendarEventForm.vue", import.meta.url), "utf8");

  test("offers the alarm and the repeat from these choices, defaults marked", () => {
    expect(form).toContain("choiceItems(t, ALARM_CHOICES, DEFAULT_ALARM)");
    expect(form).toContain("choiceItems(t, REPEAT_CHOICES, DEFAULT_REPEAT)");
    expect(form).toContain(':items="alarmItems"');
    expect(form).toContain(':items="repeatItems"');
  });

  test("starts a new event on the defaults", () => {
    expect(form).toContain("props.draft?.alarm ?? DEFAULT_ALARM");
    expect(form).toContain("props.draft?.repeat ?? DEFAULT_REPEAT");
  });
});

describe("the words", () => {
  test("no choice is a word for the default standing in place of a value", () => {
    for (const pack of Object.values(PACKS)) {
      const defaultWord = word(pack, "calendar.label.default_mark")!.replace("{value}", "").replace(/[()\uFF08\uFF09\s]/g, "");

      for (const choice of [...ALARM_CHOICES, ...REPEAT_CHOICES, ...WRITERS_CHOICES, ...TELL_CHOICES]) {
        expect(word(pack, choice.label)).not.toContain(defaultWord);
      }
    }
  });
});
