import { afterEach, describe, expect, test } from "bun:test";

import { dayBookings, dayCounts, dragSpan, firstFreeHour, freeGap, kindsOf, meets, metBooking, nextOfEach, placesOf, pressSpan, rowBlocks, shiftedSeries } from "../app/domain/bookings";
import { Refusal, createCalendarApi as apiOver } from "../app/domain/api";
import { administers, booksIn, firstCompany } from "../app/domain/companies";
import { createCalendarStore, fixed, held, refusalText } from "../app/domain/store";
import { COMPANY, booking, calendar, eventRow, resource, settle, startStandIn, until } from "./stand-in";
import { translator } from "./words";

import type { Call } from "../app/domain/api";
import type { StandIn } from "./stand-in";

// The booking screen's laying out of a company's resources and bookings (`docs/design.md`, 「Resources and
// bookings」 and 「The console」), the company a call names (`docs/api.md`, 「Companies」) — a write the row's
// own, waiting for the network or not — the grid letting go of a company no longer chosen, a refusal's words,
// and the reader's own bookings standing on the calendar's grid.

const at = (iso: string) => new Date(iso);
const DAY = { start: at("2026-11-18T00:00:00"), end: at("2026-11-19T00:00:00") };

describe("the resources", () => {
  const listed = [
    resource("a", "Room A", { kind: "Meeting room", place: "Floor 1" }),
    resource("b", "Room B", { kind: "Meeting room", place: "Floor 2" }),
    resource("c", "Room C", { kind: "Meeting room", place: "Floor 1" }),
    resource("d", "Lounge", { kind: "Reception room", place: "Floor 1" })
  ];

  test("stand under their kinds in the order listed, and each kind's under its places", () => {
    expect(kindsOf(listed)).toEqual(["Meeting room", "Reception room"]);
    expect(placesOf(listed, "Meeting room").map((group) => [group.place, group.resources.map((one) => one.id)])).toEqual([
      ["Floor 1", ["a", "c"]],
      ["Floor 2", ["b"]]
    ]);
  });
});

describe("a resource's row", () => {
  const held = [
    booking("organization", "morning", "a", "2026-11-18T09:00:00", "2026-11-18T10:00:00"),
    booking("organization", "late", "a", "2026-11-18T22:00:00", "2026-11-19T02:00:00", { state: "requested" }),
    booking("organization", "gone", "a", "2026-11-18T12:00:00", "2026-11-18T13:00:00", { state: "refused" }),
    booking("organization", "other", "b", "2026-11-18T09:00:00", "2026-11-18T10:00:00")
  ];

  test("holds its own standing bookings, each cut to the row's span; a refused one holds none", () => {
    expect(rowBlocks(held, "a", DAY).map((block) => [block.booking.id, block.from, block.to])).toEqual([
      ["morning", 9 / 24, 10 / 24],
      ["late", 22 / 24, 1]
    ]);
  });

  test("offers a press the snapped span under it, kept short of the next booking", () => {
    const gap = freeGap(held, "a", at("2026-11-18T08:40:00"), DAY)!;

    expect(gap).toEqual({ start: DAY.start, end: at("2026-11-18T09:00:00") });
    expect(pressSpan(at("2026-11-18T08:40:00"), gap, 30, 60)).toEqual({ start: at("2026-11-18T08:30:00"), end: at("2026-11-18T09:00:00") });
  });

  test("offers nothing on a booking, and a free span from where one ends, a refused one holding none", () => {
    expect(freeGap(held, "a", at("2026-11-18T09:30:00"), DAY)).toBeNull();
    expect(freeGap(held, "a", at("2026-11-18T10:00:00"), DAY)).toEqual({ start: at("2026-11-18T10:00:00"), end: at("2026-11-18T22:00:00") });
    expect(freeGap(held, "a", at("2026-11-18T12:30:00"), DAY)?.start).toEqual(at("2026-11-18T10:00:00"));
  });

  test("stops a drag across a booking at the booking's edge, either way it runs", () => {
    const gap = freeGap(held, "a", at("2026-11-18T11:00:00"), DAY)!;

    expect(dragSpan(at("2026-11-18T11:10:00"), at("2026-11-18T23:00:00"), gap, 30)).toEqual({ start: at("2026-11-18T11:00:00"), end: at("2026-11-18T22:00:00") });
    expect(dragSpan(at("2026-11-18T11:10:00"), at("2026-11-18T08:00:00"), gap, 30)).toEqual({ start: at("2026-11-18T10:00:00"), end: at("2026-11-18T11:30:00") });
  });

  test("says a span meets a standing booking, half-open, its own booking left out", () => {
    expect(meets(held, "a", { start: at("2026-11-18T10:00:00"), end: at("2026-11-18T11:00:00") })).toBe(false);
    expect(meets(held, "a", { start: at("2026-11-18T09:30:00"), end: at("2026-11-18T11:00:00") })).toBe(true);
    expect(meets(held, "a", { start: at("2026-11-18T09:30:00"), end: at("2026-11-18T11:00:00") }, "morning")).toBe(false);
  });

  test("names the standing booking a span meets among the rows read, by resource and span", () => {
    expect(metBooking(held, "a", { start: at("2026-11-18T09:30:00"), end: at("2026-11-18T13:00:00") })?.title).toBe("morning");
    expect(metBooking(held, "a", { start: at("2026-11-18T11:00:00"), end: at("2026-11-18T13:00:00") })).toBeUndefined();
  });
});

describe("a week's day cell", () => {
  const held = [
    booking("organization", "nine", "a", "2026-11-18T09:00:00", "2026-11-18T10:30:00"),
    booking("organization", "eight", "a", "2026-11-18T08:00:00", "2026-11-18T09:00:00", { state: "requested" }),
    booking("organization", "gone", "a", "2026-11-18T10:30:00", "2026-11-18T11:00:00", { state: "refused" }),
    booking("organization", "other", "b", "2026-11-18T11:00:00", "2026-11-18T12:00:00")
  ];
  const day = at("2026-11-18T00:00:00");

  test("lists the resource's standing bookings of the day by start", () => {
    expect(dayBookings(held, "a", day).map((one) => one.id)).toEqual(["eight", "nine"]);
  });

  test("offers the first whole free hour from the morning, none already past, and none on a full day", () => {
    expect(firstFreeHour(held, "a", day)).toEqual({ start: at("2026-11-18T11:00:00"), end: at("2026-11-18T12:00:00") });
    expect(firstFreeHour(held, "a", day, at("2026-11-18T11:20:00"))).toEqual({ start: at("2026-11-18T12:00:00"), end: at("2026-11-18T13:00:00") });
    expect(firstFreeHour(held, "a", day, at("2026-11-18T23:30:00"))).toBeNull();
    expect(firstFreeHour([booking("organization", "all", "a", "2026-11-18T09:00:00", "2026-11-19T00:00:00")], "a", day)).toEqual({ start: day, end: at("2026-11-18T01:00:00") });
  });
});

describe("the reader's own list", () => {
  test("stands each booking once, by its next occurrence, among the rest by start", () => {
    const rows = [
      booking("mine", "weekly", "a", "2026-11-25T09:00:00", "2026-11-25T10:00:00", { repeat: "weekly", occurrence: "2026-11-25T09:00:00" }),
      booking("mine", "once", "a", "2026-11-20T14:00:00", "2026-11-20T15:00:00"),
      booking("mine", "weekly", "a", "2026-11-18T09:00:00", "2026-11-18T10:00:00", { repeat: "weekly", occurrence: "2026-11-18T09:00:00" })
    ];

    expect(nextOfEach(rows).map((row) => [row.id, row.start])).toEqual([
      ["weekly", "2026-11-18T09:00:00"],
      ["once", "2026-11-20T14:00:00"]
    ]);
  });
});

describe("the month", () => {
  test("counts each standing booking on every day it holds", () => {
    const days = [at("2026-11-18T00:00:00"), at("2026-11-19T00:00:00"), at("2026-11-20T00:00:00")];
    const counts = dayCounts(
      [
        booking("organization", "two", "a", "2026-11-18T22:00:00", "2026-11-19T02:00:00"),
        booking("organization", "one", "a", "2026-11-19T09:00:00", "2026-11-19T10:00:00"),
        booking("organization", "gone", "a", "2026-11-20T09:00:00", "2026-11-20T10:00:00", { state: "refused" })
      ],
      days
    );

    expect(counts.get("a")).toEqual([1, 2, 0]);
  });
});

describe("a repeating booking moved whole", () => {
  test("moves its series by as much as the occurrence moved, start and end each", () => {
    const series = { start: "2026-11-02T09:00:00", end: "2026-11-02T10:00:00" };
    const row = { start: "2026-11-16T09:00:00", end: "2026-11-16T10:00:00" };

    expect(shiftedSeries(series, row, { start: "2026-11-16T10:00:00", end: "2026-11-16T11:30:00" })).toEqual({ start: "2026-11-02T10:00:00", end: "2026-11-02T11:30:00" });
  });
});

describe("the companies", () => {
  const own = { id: "company-a", name: "Company A", own: true, manages: true };
  const other = { id: "company-b", name: "Company B", own: false, manages: true };
  const platform = { id: "platform", name: "Platform", own: false, manages: false };

  test("are the administrator's where the platform's own realm is listed, the own one first chosen", () => {
    expect(administers([own])).toBe(false);
    expect(administers([own, other, platform])).toBe(true);
    expect(firstCompany([other, own, platform])).toBe("company-a");
    expect(firstCompany([])).toBeNull();
  });

  test("are booked in the reader's own alone, and where they could not be read, in what a read naming none reads", () => {
    expect(booksIn([own], null)).toBe(true);
    expect(booksIn([own, other, platform], "company-a")).toBe(true);
    expect(booksIn([own, other, platform], "company-b")).toBe(false);
    expect(booksIn([], null)).toBe(true);
  });

  test("a call names the chosen company for the administrator alone, a row's write its own row's", async () => {
    const asked: string[] = [];
    const call: Call = async <T>(path: string, options?: Parameters<Call>[1]) => {
      asked.push(`${options?.method ?? "GET"} ${path} ${options?.query?.company ?? "-"}`);

      return { items: [], more: false } as T;
    };
    let chosen: string | null = null;
    const api = apiOver(call, () => chosen);
    const row = booking("mine", "x", "a", "2026-11-18T09:00:00", "2026-11-18T10:00:00");

    await api.resources();
    await api.changeBooking(row, { title: "Stand-up" });

    chosen = "company-b";

    await api.companies();
    await api.resources();
    await api.resources(null);
    await api.calendars("mine");
    await api.changeBooking(row, { title: "Stand-up" });

    expect(asked).toEqual([
      "GET /organization/resources -",
      "PATCH /organization/bookings/x -",
      "GET /organization/companies -",
      "GET /organization/resources company-b",
      "GET /organization/resources -",
      "GET /mine/calendars -",
      "PATCH /organization/bookings/x company-a"
    ]);
  });
});

describe("the reader's bookings on the calendar's grid", () => {
  let standIn: StandIn | null = null;

  afterEach(() => {
    standIn?.stop();
    standIn = null;
  });

  test("stand as the reader's rows, named by their resource and never moved; a refused one is left off", async () => {
    standIn = startStandIn({
      calendars: { mine: [calendar("mine", "mine", "Mine")] },
      resources: [resource("a", "Room A")],
      bookings: {
        mine: [
          booking("mine", "standing", "a", "2026-11-18T09:00:00", "2026-11-18T10:00:00", { state: "requested" }),
          booking("mine", "refused", "a", "2026-11-18T11:00:00", "2026-11-18T12:00:00", { state: "refused" })
        ]
      }
    });

    const store = createCalendarStore({ api: apiOver(standIn.call), tell: () => {}, probeMs: 10 });

    await Promise.all([store.loadCalendars(), store.loadResourceNames(), store.loadSpan({ start: new Date(2026, 10, 18), end: new Date(2026, 10, 19) })]);
    await settle();

    const rows = store.events.value.filter((row) => row.booking);

    expect(rows.map((row) => [row.realm, row.id, fixed(row), store.layerOf(row)?.name])).toEqual([["mine", "standing", true, "Room A"]]);
    expect(store.failedSpans.value).toEqual([]);
    store.dispose();
  });
});

// A service of two companies' organization events, answering each read for the company it names (the
// reader's own where none) and recording every call; a read may be held until the test lets it go.
function twoCompanies() {
  const asked: string[] = [];
  const holds: (() => void)[] = [];
  let holding = false;
  let holdingWrites = false;
  let offline = false;
  const rows = {
    "company-a": [eventRow("organization", "a-standup", "organization", "2026-11-18T09:00:00", "2026-11-18T10:00:00", { company: "company-a" })],
    "company-b": [eventRow("organization", "b-review", "organization", "2026-11-18T11:00:00", "2026-11-18T12:00:00", { company: "company-b" })]
  } as const;

  const call: Call = async <T>(path: string, options?: Parameters<Call>[1]) => {
    const company = options?.query?.company ?? COMPANY;

    asked.push(`${options?.method ?? "GET"} ${path} ${options?.query?.company ?? "-"}`);

    if (offline && options?.method) throw new TypeError("fetch failed");

    if (path === "/organization/events" && !options?.method) {
      if (holding) await new Promise<void>((resolve) => holds.push(resolve));

      return { items: rows[company as keyof typeof rows] ?? [], more: false } as T;
    }

    if (options?.method && holdingWrites) await new Promise<void>((resolve) => holds.push(resolve));

    if (options?.method === "POST") return { ...(options.body as object), realm: "organization", occurrence: null } as T;

    return { items: [], more: false } as T;
  };

  return {
    asked,
    call,
    hold: (on: boolean) => (holding = on),
    holdWrites: (on: boolean) => (holdingWrites = on),
    release: () => holds.splice(0).forEach((resolve) => resolve()),
    offline: (on: boolean) => (offline = on)
  };
}

const SPAN = { start: new Date(2026, 10, 18), end: new Date(2026, 10, 19) };

describe("a company switch on the grid", () => {
  test("lets the last company's rows go at once and reads the span in view again under the new one", async () => {
    const service = twoCompanies();
    let chosen: string | null = "company-a";
    const store = createCalendarStore({ api: apiOver(service.call, () => chosen), company: () => chosen, tell: () => {}, probeMs: 10 });

    store.showSpans([SPAN]);
    await store.loadSpan(SPAN);

    expect(store.events.value.filter((row) => row.realm === "organization").map((row) => row.id)).toEqual(["a-standup"]);

    chosen = "company-b";

    const switching = store.companyChanged();

    expect(store.allEvents.value.filter((row) => row.realm === "organization")).toEqual([]);

    await switching;

    expect(store.allEvents.value.filter((row) => row.realm === "organization").map((row) => row.id)).toEqual(["b-review"]);
    store.dispose();
  });

  test("lets a read still on its way for the last company go, and reads the span under the new one", async () => {
    const service = twoCompanies();
    let chosen: string | null = "company-a";
    const store = createCalendarStore({ api: apiOver(service.call, () => chosen), company: () => chosen, tell: () => {}, probeMs: 10 });

    store.showSpans([SPAN]);
    service.hold(true);

    const first = store.loadSpan(SPAN);

    await settle();
    chosen = "company-b";

    const switching = store.companyChanged();

    await settle();
    service.hold(false);
    service.release();
    await Promise.all([first, switching]);

    expect(store.allEvents.value.filter((row) => row.realm === "organization").map((row) => row.id)).toEqual(["b-review"]);
    expect(service.asked.filter((one) => one.startsWith("GET /organization/events"))).toEqual(["GET /organization/events company-a", "GET /organization/events company-b"]);
    store.dispose();
  });
});

describe("a write on a row", () => {
  test("answered after another company is chosen, stands on none of the new company's grid", async () => {
    const service = twoCompanies();
    let chosen: string | null = "company-a";
    const store = createCalendarStore({ api: apiOver(service.call, () => chosen), company: () => chosen, tell: () => {}, probeMs: 10 });
    const row = held(eventRow("organization", "planning", "organization", "2026-11-18T14:00:00", "2026-11-18T15:00:00", { company: "company-a" }));
    const organization = () => store.allEvents.value.filter((one) => one.realm === "organization").map((one) => one.id);

    store.showSpans([SPAN]);
    await store.loadSpan(SPAN);
    service.holdWrites(true);

    const writing = store.addEvent(row);

    expect(organization()).toEqual(["a-standup", "planning"]);

    chosen = "company-b";
    await store.companyChanged();

    expect(organization()).toEqual(["b-review"]);

    service.holdWrites(false);
    service.release();
    await writing;

    expect(organization()).toEqual(["b-review"]);
    store.dispose();
  });

  test("names the row's own company, chosen when it was made, even sent from the line after another is chosen", async () => {
    const service = twoCompanies();
    let chosen: string | null = "company-a";
    const store = createCalendarStore({ api: apiOver(service.call, () => chosen), company: () => chosen, tell: () => {}, probeMs: 10 });
    const row = held(eventRow("organization", "planning", "organization", "2026-11-18T14:00:00", "2026-11-18T15:00:00", { company: "company-a" }));
    const other = held(eventRow("organization", "b-review", "organization", "2026-11-18T11:00:00", "2026-11-18T12:00:00", { company: "company-b" }));

    await store.updateEvent(other, { ...other, title: "Review" });

    service.offline(true);
    await store.addEvent(row);

    expect(store.queue.value.map((request) => [request.kind, request.company])).toEqual([["add", "company-a"]]);

    chosen = "company-b";
    service.offline(false);

    await until(() => service.asked.filter((one) => one.startsWith("POST")).length === 2);

    expect(service.asked.filter((one) => !one.startsWith("GET"))).toEqual([
      "PATCH /organization/events/b-review company-b",
      "POST /organization/events company-a",
      "POST /organization/events company-a"
    ]);
    store.dispose();
  });
});

describe("a refused booking", () => {
  const t = translator("ko");
  const overlap = new Refusal(
    409,
    "overlap",
    'The booking meets the booking "Review "Q3"" (b-7) of the same resource, first at its occurrence starting 2026-04-06T09:00:00.'
  );
  const unnamed = new Refusal(409, "overlap", "The booking meets another.");

  test("names the booking an overlap met by the title the service answered, though the screen holds no such row", () => {
    expect(refusalText(t, overlap)).toBe(t("calendar.error.overlap_named", { title: 'Review "Q3"' }));
    expect(refusalText(t, overlap, "Stand-up")).toBe(t("calendar.error.overlap_named", { title: 'Review "Q3"' }));
  });

  test("is told in the screen's words alone, the booking met named where the screen holds it", () => {
    expect(refusalText(t, unnamed, "Stand-up")).toBe(t("calendar.error.overlap_named", { title: "Stand-up" }));
    expect(refusalText(t, unnamed)).toBe(t("calendar.error.overlap"));
    expect(refusalText(t, new Refusal(403, "not-a-writer", "Not a writer."))).toBe(t("calendar.error.booking_not_a_writer"));
  });
});
