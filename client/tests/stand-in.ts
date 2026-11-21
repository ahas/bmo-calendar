import { ofetch } from "ofetch";

import type { Call } from "../app/domain/api";
import type { Booking, Calendar, CalendarEventRow, Realm, Resource, ScheduleRow, Source } from "../shared/types";

// A stand-in of the calendar service's routes the screen calls (`docs/api.md`, 「Calendars」 and
// 「Events」), served on the loopback for the length of a test: two realms, their calendars with a hidden
// mark, events in spans answered in pages, every write idempotent on its id, and a calendar a write under
// is refused `not-a-writer`; and each realm's sources and received schedules, as the realm's reads answer
// them (「Sources and schedules」), a source's hidden mark refused where it is listed as refused; and the
// company's resources and each realm's bookings of a span (「Resources and bookings」).

export interface StandIn {
  url: string;
  calendars: Record<Realm, Calendar[]>;
  events: Record<Realm, CalendarEventRow[]>;
  sources: Record<Realm, Source[]>;
  schedules: Record<Realm, ScheduleRow[]>;
  resources: Resource[];
  bookings: Record<Realm, Booking[]>;
  /** Every call made, as `METHOD path?query`. */
  calls: string[];
  /** Calendars a write under is refused, as `realm:id`, and sources a hidden mark is refused for, as
   *  `realm:source:id`. */
  refused: Set<string>;
  /** How many rows one page of events holds. */
  pageSize: number;
  stop: () => void;
  call: Call;
}

/** The company every organization row of the stand-in stands in. */
export const COMPANY = "company-a";

export function resource(id: string, name: string, extra: Partial<Resource> = {}): Resource {
  return { id, company: COMPANY, name, kind: "Meeting room", place: "Floor 1", seats: 4, approval: false, keepers: [], creator: "7", createdAt: 1, changedAt: 1, ...extra };
}

export function booking(realm: Realm, id: string, resourceId: string, start: string, end: string, extra: Partial<Booking> = {}): Booking {
  return {
    id,
    realm,
    company: COMPANY,
    resourceId,
    title: id,
    start,
    end,
    allDay: false,
    repeat: "none",
    occurrence: null,
    booker: "7",
    state: "booked",
    decidedBy: null,
    decidedAt: null,
    reason: null,
    createdAt: 1,
    changedAt: 1,
    ...extra
  };
}

export function calendar(realm: Realm, id: string, name: string, extra: Partial<Calendar> = {}): Calendar {
  return {
    id,
    realm,
    company: realm === "mine" ? null : COMPANY,
    name,
    color: realm === "mine" ? "primary" : "info",
    writers: realm === "mine" ? null : "creator",
    tell: realm === "mine" ? null : "creator",
    hidden: false,
    creator: "7",
    createdAt: 1,
    ...extra
  };
}

export function eventRow(realm: Realm, id: string, calendarId: string, start: string, end: string, extra: Partial<CalendarEventRow> = {}): CalendarEventRow {
  return {
    id,
    realm,
    company: realm === "mine" ? null : COMPANY,
    calendarId,
    title: id,
    description: "",
    start,
    end,
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

export function source(id: string, extra: Partial<Source> = {}): Source {
  return { id, name: id, color: "warning", hidden: false, createdAt: 1, ...extra };
}

export function scheduleRow(realm: Realm, sourceId: string, itemId: string, start: string, end: string, extra: Partial<ScheduleRow> = {}): ScheduleRow {
  return { sourceId, itemId, realm, company: COMPANY, title: itemId, start, end, allDay: false, link: `/items/${itemId}`, assignee: null, private: false, createdAt: 1, changedAt: 1, ...extra };
}

function refusal(status: number, code: string, error: string): Response {
  return Response.json({ code, error }, { status });
}

interface Seed {
  calendars?: Partial<Record<Realm, Calendar[]>>;
  events?: Partial<Record<Realm, CalendarEventRow[]>>;
  sources?: Partial<Record<Realm, Source[]>>;
  schedules?: Partial<Record<Realm, ScheduleRow[]>>;
  resources?: Resource[];
  bookings?: Partial<Record<Realm, Booking[]>>;
  pageSize?: number;
}

export function startStandIn(seed: Seed = {}): StandIn {
  const state: Omit<StandIn, "url" | "stop" | "call"> = {
    calendars: { mine: [...(seed.calendars?.mine ?? [])], organization: [...(seed.calendars?.organization ?? [])] },
    events: { mine: [...(seed.events?.mine ?? [])], organization: [...(seed.events?.organization ?? [])] },
    sources: { mine: [...(seed.sources?.mine ?? [])], organization: [...(seed.sources?.organization ?? [])] },
    schedules: { mine: [...(seed.schedules?.mine ?? [])], organization: [...(seed.schedules?.organization ?? [])] },
    resources: [...(seed.resources ?? [])],
    bookings: { mine: [...(seed.bookings?.mine ?? [])], organization: [...(seed.bookings?.organization ?? [])] },
    calls: [],
    refused: new Set(),
    pageSize: seed.pageSize ?? 100
  };

  const server = Bun.serve({
    port: 0,
    async fetch(request) {
      const url = new URL(request.url);
      const parts = url.pathname.split("/").filter(Boolean).map(decodeURIComponent);
      const realm = parts[0] as Realm;

      state.calls.push(`${request.method} ${url.pathname}${url.search}`);

      if (realm !== "mine" && realm !== "organization") return refusal(404, "not-found", "No such realm.");

      const body = request.method === "POST" || request.method === "PATCH" ? ((await request.json()) as Record<string, unknown>) : {};

      if (parts[1] === "calendars") {
        if (parts.length === 2 && request.method === "GET") return Response.json({ items: state.calendars[realm], more: false });

        const index = state.calendars[realm].findIndex((one) => one.id === parts[2]);

        if (index === -1) return refusal(404, "not-found", "No such calendar.");

        if (request.method === "PATCH") {
          state.calendars[realm][index] = { ...state.calendars[realm][index]!, ...(body as Partial<Calendar>) };

          return Response.json(state.calendars[realm][index]);
        }
      }

      if (parts[1] === "sources") {
        if (parts.length === 2 && request.method === "GET") return Response.json({ items: state.sources[realm], more: false });

        const index = state.sources[realm].findIndex((one) => one.id === parts[2]);

        if (index === -1) return refusal(404, "not-found", "No such source.");
        if (state.refused.has(`${realm}:source:${parts[2]}`)) return refusal(502, "upstream-failed", "The brain failed.");

        if (request.method === "PATCH") {
          state.sources[realm][index] = { ...state.sources[realm][index]!, ...(body as Partial<Source>) };

          return Response.json(state.sources[realm][index]);
        }
      }

      if (parts[1] === "schedules" && request.method === "GET") {
        const start = url.searchParams.get("start")!;
        const end = url.searchParams.get("end")!;

        return Response.json({ items: state.schedules[realm].filter((row) => row.start < end && row.end > start), more: false });
      }

      if (parts[1] === "resources" && parts.length === 2 && request.method === "GET") return Response.json({ items: state.resources, more: false });

      if (parts[1] === "bookings" && parts.length === 2 && request.method === "GET") {
        const start = url.searchParams.get("start")!;
        const end = url.searchParams.get("end")!;

        return Response.json({ items: state.bookings[realm].filter((row) => row.start < end && row.end > start), more: false });
      }

      if (parts[1] === "events") {
        if (parts.length === 2 && request.method === "GET") {
          const start = url.searchParams.get("start")!;
          const end = url.searchParams.get("end")!;
          const after = url.searchParams.get("after");
          const inSpan = state.events[realm].filter((row) => row.start < end && row.end > start).sort((a, b) => a.start.localeCompare(b.start) || a.id.localeCompare(b.id));
          const from = after === null ? 0 : inSpan.findIndex((row) => row.id === after) + 1;
          const items = inSpan.slice(from, from + state.pageSize);

          return Response.json({ items, more: from + items.length < inSpan.length });
        }

        if (parts.length === 2 && request.method === "POST") {
          const write = body as Partial<CalendarEventRow> & { id: string; calendarId: string };

          if (!state.calendars[realm].some((one) => one.id === write.calendarId)) return refusal(422, "unknown-calendar", "No such calendar.");
          if (state.refused.has(`${realm}:${write.calendarId}`)) return refusal(403, "not-a-writer", "Not a writer of this calendar.");

          const held = state.events[realm].find((row) => row.id === write.id);

          if (held) return Response.json(held, { status: 200 });

          const row = eventRow(realm, write.id, write.calendarId, write.start!, write.end!, { ...write, occurrence: null });

          state.events[realm].push(row);

          return Response.json(row, { status: 201 });
        }

        const index = state.events[realm].findIndex((row) => row.id === parts[2]);

        if (index === -1) return refusal(404, "not-found", "No such event.");

        const held = state.events[realm][index]!;

        if (request.method === "GET") return Response.json(held);
        if (state.refused.has(`${realm}:${held.calendarId}`)) return refusal(403, "not-a-writer", "Not a writer of this calendar.");

        if (request.method === "PATCH") {
          state.events[realm][index] = { ...held, ...(body as Partial<CalendarEventRow>), occurrence: null };

          return Response.json(state.events[realm][index]);
        }

        if (request.method === "DELETE") {
          state.events[realm].splice(index, 1);

          return new Response(null, { status: 204 });
        }
      }

      return refusal(404, "no-route", "No such route.");
    }
  });

  const base = `http://127.0.0.1:${server.port}`;
  const fetcher = ofetch.create({ baseURL: base });

  return {
    ...state,
    url: base,
    stop: () => server.stop(true),
    call: <T>(path: string, options?: Parameters<Call>[1]) => fetcher<T>(path, options as never) as Promise<T>
  };
}

/** A call that never reaches anything: what a browser with no network answers. */
export const unreachable: Call = () => ofetch("http://[::1]:1/");

/** Waits for the store's writes and reads in flight to land. */
export function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 30));
}

/** Waits until the condition holds, a few milliseconds at a time and two seconds at most, so a test asserts
 *  what it waited for rather than a moment in between: a store sending its line of changes has emptied the
 *  line before the service answers the first of them. */
export async function until(condition: () => boolean): Promise<void> {
  for (let beat = 0; beat < 400 && !condition(); beat++) await new Promise((resolve) => setTimeout(resolve, 5));
}
