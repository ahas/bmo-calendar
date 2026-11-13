import { describe, expect, test } from "bun:test";

import { originLink } from "../app/domain/origin";

// Where a received schedule opens: a console path in the console itself, an address in a new tab, and a row
// of the calendar's own as no link at all.

describe("a received schedule's link", () => {
  test("opens a console path in the console", () => {
    expect(originLink({ sourceId: "plans", itemId: "a", link: "/plans/items/a" })).toEqual({ to: "/plans/items/a", raw: true });
  });

  test("opens an address in a new tab", () => {
    expect(originLink({ sourceId: "plans", itemId: "a", link: "https://plans.example/a" })).toEqual({ to: "https://plans.example/a", external: true, target: "_blank", raw: true });
  });

  test("is none for the calendar's own event", () => {
    expect(originLink(undefined)).toBeNull();
  });
});
