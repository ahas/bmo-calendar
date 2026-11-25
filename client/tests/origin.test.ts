import { describe, expect, test } from "bun:test";

import { originLink } from "../app/domain/origin";

import type { Origin } from "../shared/types";

// Where a received schedule's detail leads: the reader's own schedule to its item, a console path in the console
// itself and an address in a new tab; another's, and a row of the calendar's own, nowhere.

function origin(extra: Partial<Origin>): Origin {
  return { sourceId: "plans", itemId: "a", link: "/plans/items/a", assignee: "7", own: true, ...extra };
}

describe("a received schedule's link", () => {
  test("opens the reader's own item's console path in the console", () => {
    expect(originLink(origin({}))).toEqual({ to: "/plans/items/a" });
  });

  test("opens the reader's own item's address in a new tab", () => {
    expect(originLink(origin({ link: "https://plans.example/a" }))).toEqual({ to: "https://plans.example/a", external: true, target: "_blank" });
  });

  test("is none for another's schedule, whatever its link", () => {
    expect(originLink(origin({ own: false }))).toBeNull();
    expect(originLink(origin({ own: false, link: "https://plans.example/a" }))).toBeNull();
  });

  test("is none for the calendar's own event", () => {
    expect(originLink(undefined)).toBeNull();
  });
});
