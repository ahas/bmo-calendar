import { afterEach, describe, expect, test } from "bun:test";

import { createCalendarApi } from "../app/domain/api";
import { createCalendarStore, held } from "../app/domain/store";
import { calendar, eventRow, scheduleRow, settle, source, startStandIn, unreachable } from "./stand-in";

import type { Call } from "../app/domain/api";
import type { Notice } from "../app/domain/store";
import type { StandIn } from "./stand-in";

// The screen's hold on the calendar service, against a stand-in of its routes: the two realms kept apart,
// a calendar hidden for the account, a refused write taken back, a write sent again after the network
// failed landing once, a span read page by page, and the schedules other units push laid on the grid in
// their sources' layers.

const SPAN = { start: new Date(2026, 10, 1), end: new Date(2026, 11, 13) };

let standIn: StandIn | null = null;

afterEach(() => {
  standIn?.stop();
  standIn = null;
});

function storeOver(call: Call) {
  const notices: Notice[] = [];
  const store = createCalendarStore({ api: createCalendarApi(call), tell: (notice) => notices.push(notice), probeMs: 10 });

  return { store, notices };
}

describe("the realms", () => {
  test("the calendars of each realm stand in a group of their own, the account's first", async () => {
    standIn = startStandIn({
      calendars: { mine: [calendar("mine", "mine", "Mine")], organization: [calendar("organization", "organization", "Shared"), calendar("organization", "team", "Team")] }
    });

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();

    expect(store.groups.value.map((group) => [group.realm, group.calendars.map((one) => one.id)])).toEqual([
      ["mine", ["mine"]],
      ["organization", ["organization", "team"]]
    ]);
    expect(standIn.calls).toContain("GET /mine/calendars");
    expect(standIn.calls).toContain("GET /organization/calendars");
  });

  test("one grid holds both realms' events, an id shared across realms kept as two events", async () => {
    standIn = startStandIn({
      calendars: { mine: [calendar("mine", "mine", "Mine")], organization: [calendar("organization", "organization", "Shared")] },
      events: {
        mine: [eventRow("mine", "a", "mine", "2026-11-03T09:00:00", "2026-11-03T10:00:00")],
        organization: [eventRow("organization", "a", "organization", "2026-11-03T11:00:00", "2026-11-03T12:00:00")]
      }
    });

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.loadSpan(SPAN);

    expect(store.events.value.map((event) => [event.realm, event.id, event.key]).sort()).toEqual([
      ["mine", "a", "mine:a:"],
      ["organization", "a", "organization:a:"]
    ]);
  });

  test("a new event is written to its calendar's realm alone", async () => {
    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")], organization: [calendar("organization", "organization", "Shared")] } });

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.addEvent(held(eventRow("organization", "new-one", "organization", "2026-11-04T09:00:00", "2026-11-04T10:00:00")));

    expect(standIn.events.organization.map((row) => row.id)).toEqual(["new-one"]);
    expect(standIn.events.mine).toEqual([]);
  });
});

describe("visibility", () => {
  test("hiding a calendar is kept by the service for the account and takes its events off the grid", async () => {
    standIn = startStandIn({
      calendars: { mine: [calendar("mine", "mine", "Mine")], organization: [calendar("organization", "organization", "Shared")] },
      events: { organization: [eventRow("organization", "meeting", "organization", "2026-11-03T09:00:00", "2026-11-03T10:00:00")] }
    });

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.loadSpan(SPAN);
    expect(store.events.value.map((event) => event.id)).toEqual(["meeting"]);

    await store.setHidden(store.calendarOf("organization", "organization")!, true);

    expect(standIn.calls).toContain("PATCH /organization/calendars/organization");
    expect(standIn.calendars.organization[0]!.hidden).toBe(true);
    expect(store.events.value).toEqual([]);

    // Another page, read afresh, finds it hidden still.
    const again = storeOver(standIn.call).store;

    await again.loadCalendars();
    await again.loadSpan(SPAN);

    expect(again.calendarOf("organization", "organization")!.hidden).toBe(true);
    expect(again.events.value).toEqual([]);
    expect(again.allEvents.value.map((event) => event.id)).toEqual(["meeting"]);
  });

  test("a hiding the service refuses is taken back", async () => {
    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")] } });

    const { store, notices } = storeOver(standIn.call);

    await store.loadCalendars();
    standIn.calendars.mine = [];
    await store.setHidden(store.calendarOf("mine", "mine")!, true);

    expect(store.calendarOf("mine", "mine")!.hidden).toBe(false);
    expect(notices.map((notice) => notice.tone)).toEqual(["error"]);
  });
});

describe("a refused write", () => {
  test("a new event under a calendar the reader may not write is taken back with the refusal's words", async () => {
    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")], organization: [calendar("organization", "board", "Board", { writers: "administrator" })] } });
    standIn.refused.add("organization:board");

    const { store, notices } = storeOver(standIn.call);

    await store.loadCalendars();

    const added = store.addEvent(held(eventRow("organization", "x", "board", "2026-11-03T09:00:00", "2026-11-03T10:00:00")));

    // Drawn at once, ahead of the answer.
    expect(store.events.value.map((event) => event.id)).toEqual(["x"]);

    await added;

    expect(store.events.value).toEqual([]);
    expect(notices).toEqual([{ tone: "error", title: "calendar.error.not_a_writer" }]);
  });

  test("a moved event the reader may not change goes back to where it stood", async () => {
    const row = eventRow("organization", "x", "board", "2026-11-03T09:00:00", "2026-11-03T10:00:00");

    standIn = startStandIn({ calendars: { organization: [calendar("organization", "board", "Board", { writers: "administrator" })] }, events: { organization: [row] } });
    standIn.refused.add("organization:board");

    const { store, notices } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.loadSpan(SPAN);

    const before = store.events.value[0]!;

    await store.updateEvent(before, { ...before, start: "2026-11-04T09:00:00", end: "2026-11-04T10:00:00" });

    expect(store.events.value.map((event) => event.start)).toEqual(["2026-11-03T09:00:00"]);
    expect(notices.map((notice) => notice.title)).toEqual(["calendar.error.not_a_writer"]);
  });

  test("a cancellation the reader may not make puts the event back", async () => {
    standIn = startStandIn({
      calendars: { organization: [calendar("organization", "board", "Board", { writers: "administrator" })] },
      events: { organization: [eventRow("organization", "x", "board", "2026-11-03T09:00:00", "2026-11-03T10:00:00")] }
    });
    standIn.refused.add("organization:board");

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.loadSpan(SPAN);
    await store.removeEvent(store.events.value[0]!);

    expect(store.events.value.map((event) => event.id)).toEqual(["x"]);
  });
});

describe("the line of changes waiting for the network", () => {
  test("a write that landed but whose answer was lost is sent again and stands once", async () => {
    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")] } });

    const service = standIn;
    let lose = true;
    // The first write reaches the service and its answer never comes back.
    const call: Call = async (path, options) => {
      const answer = await service.call(path, options);

      if (lose && options?.method === "POST") {
        lose = false;
        throw new TypeError("fetch failed");
      }

      return answer as never;
    };

    const { store, notices } = storeOver(call);

    await store.loadCalendars();
    await store.addEvent(held(eventRow("mine", "dentist", "mine", "2026-11-04T15:00:00", "2026-11-04T16:00:00")));

    expect(store.online.value).toBe(false);
    expect(store.queue.value.map((request) => request.kind)).toEqual(["add"]);
    expect(store.events.value.map((event) => event.id)).toEqual(["dentist"]);

    // The service answers again: the line is sent, and the write it already holds is answered as it stands.
    while (store.queue.value.length) await settle();

    expect(standIn.calls.filter((one) => one === "POST /mine/events")).toHaveLength(2);
    expect(standIn.events.mine.map((row) => row.id)).toEqual(["dentist"]);
    expect(store.events.value.map((event) => event.id)).toEqual(["dentist"]);
    expect(notices).toEqual([{ tone: "success", title: "calendar.message.synced" }]);

    store.dispose();
  });

  test("changes made with no network wait in order, drawn, and go once it is back", async () => {
    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")] } });

    const service = standIn;
    let down = true;
    const call: Call = (path, options) => (down && options?.method ? unreachable(path, options) : service.call(path, options));
    const { store } = storeOver(call);

    await store.loadCalendars();

    const first = held(eventRow("mine", "one", "mine", "2026-11-04T09:00:00", "2026-11-04T10:00:00"));

    await store.addEvent(first);
    await store.updateEvent(store.events.value[0]!, { ...first, title: "renamed" });

    expect(store.queue.value.map((request) => request.kind)).toEqual(["add", "change"]);
    expect(store.events.value.map((event) => event.title)).toEqual(["renamed"]);

    down = false;

    while (store.queue.value.length) await settle();

    expect(standIn.events.mine.map((row) => [row.id, row.title])).toEqual([["one", "renamed"]]);

    store.dispose();
  });
});

describe("spans", () => {
  test("a span answered in pages is read to its last page, from each page's last row", async () => {
    const rows = Array.from({ length: 5 }, (_, index) => eventRow("mine", `e${index}`, "mine", `2026-11-0${index + 2}T09:00:00`, `2026-11-0${index + 2}T10:00:00`));

    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")] }, events: { mine: rows }, pageSize: 2 });

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.loadSpan(SPAN);

    expect(store.events.value.map((event) => event.id).sort()).toEqual(["e0", "e1", "e2", "e3", "e4"]);
    expect(standIn.calls.filter((one) => one.startsWith("GET /mine/events")).map((one) => new URL(one.split(" ")[1]!, "http://x").searchParams.get("after"))).toEqual([null, "e1", "e3"]);
  });

  test("a span longer than the service reads at once is read in parts of at most 92 days", async () => {
    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")] } });

    const { store } = storeOver(standIn.call);

    await store.loadSpan({ start: new Date(2026, 0, 1), end: new Date(2026, 11, 31) });

    const asked = standIn.calls.filter((one) => one.startsWith("GET /mine/events")).map((one) => new URL(one.split(" ")[1]!, "http://x").searchParams);

    expect(asked.length).toBe(4);

    for (const query of asked) {
      const days = (new Date(query.get("end")!).getTime() - new Date(query.get("start")!).getTime()) / 86_400_000;

      expect(days).toBeLessThanOrEqual(92);
    }
  });

  test("a span already read is not asked again until it is refreshed", async () => {
    standIn = startStandIn({ calendars: { mine: [calendar("mine", "mine", "Mine")] } });

    const { store } = storeOver(standIn.call);

    await store.loadSpan(SPAN);
    await store.loadSpan(SPAN);
    expect(standIn.calls.filter((one) => one.startsWith("GET /mine/events"))).toHaveLength(1);

    store.showSpans([SPAN]);
    standIn.events.mine.push(eventRow("mine", "late", "mine", "2026-11-05T09:00:00", "2026-11-05T10:00:00"));
    await store.refreshVisible();

    expect(standIn.calls.filter((one) => one.startsWith("GET /mine/events"))).toHaveLength(2);
    expect(store.allEvents.value.map((event) => event.id)).toEqual(["late"]);
  });
});

describe("received schedules", () => {
  const RECEIVED = {
    sources: { mine: [source("plans")], organization: [source("plans"), source("work", { color: "success" })] },
    schedules: {
      mine: [scheduleRow("mine", "plans", "own", "2026-11-05T09:00:00", "2026-11-05T10:00:00", { assignee: "7" })],
      organization: [
        scheduleRow("organization", "plans", "own", "2026-11-05T09:00:00", "2026-11-05T10:00:00", { assignee: "7" }),
        scheduleRow("organization", "plans", "team", "2026-11-06T09:00:00", "2026-11-06T10:00:00"),
        scheduleRow("organization", "work", "build", "2026-11-07T09:00:00", "2026-11-07T10:00:00")
      ]
    }
  };

  test("stand on the grid in their sources' layers, one assigned to the reader once, as the reader's own", async () => {
    standIn = startStandIn(RECEIVED);

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.loadSpan(SPAN);

    expect(store.groups.value.map((group) => [group.realm, group.sources.map((one) => one.id)])).toEqual([
      ["mine", ["plans"]],
      ["organization", ["plans", "work"]]
    ]);
    expect(store.events.value.map((row) => row.key).sort()).toEqual(["organization:plans:team", "organization:work:build", "mine:plans:own"].map((key) => `schedule:${key}`).sort());

    const build = store.events.value.find((row) => row.origin?.itemId === "build")!;

    expect(build.origin).toEqual({ sourceId: "work", itemId: "build", link: "/items/build" });
    expect(store.layerOf(build)?.color).toBe("success");
  });

  test("a layer hidden in one realm leaves that realm's rows alone off the grid, and is kept per realm", async () => {
    standIn = startStandIn(RECEIVED);

    const { store } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.loadSpan(SPAN);
    const sent = store.setSourceHidden("mine", store.sourceOf("mine", "plans")!, true);

    expect(store.sendingLayers.value).toEqual(["source:mine:plans"]);
    await sent;
    expect(store.sendingLayers.value).toEqual([]);
    expect(standIn.calls).toContain("PATCH /mine/sources/plans");
    expect(standIn.sources.mine[0]!.hidden).toBe(true);
    expect(store.events.value.map((row) => row.key).sort()).toEqual(["schedule:organization:plans:own", "schedule:organization:plans:team", "schedule:organization:work:build"]);

    await store.setSourceHidden("organization", store.sourceOf("organization", "plans")!, true);

    expect(store.events.value.map((row) => row.key)).toEqual(["schedule:organization:work:build"]);
  });

  test("a layer the service does not take is shown as it was, and the reader told", async () => {
    standIn = startStandIn(RECEIVED);
    standIn.refused.add("organization:source:work");

    const { store, notices } = storeOver(standIn.call);

    await store.loadCalendars();
    await store.setSourceHidden("organization", store.sourceOf("organization", "work")!, true);

    expect(store.sourceOf("organization", "work")!.hidden).toBe(false);
    expect(notices.map((notice) => notice.tone)).toEqual(["error"]);
  });
});
