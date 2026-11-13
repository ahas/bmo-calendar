import { MAX_SPAN_DAYS, toLocalISO } from "./dates";

import type { Calendar, CalendarEventRow, CalendarWrite, DateRange, EventWrite, ListPage, Realm, ScheduleRow, Source } from "../../shared/types";

// The screen's calls to the calendar service, each at the path the service's own API writes it
// (`docs/api.md`): the calendars of a realm, the events of a span and the writes on either, and the
// sources and schedules other units push, which the screen reads and only hides or shows. The caller
// hands the call itself — through the console's relay in the browser, to a stand-in in a test — so this
// module names no address and no fetcher.

export type Call = <T>(path: string, options?: { method?: "GET" | "POST" | "PATCH" | "DELETE"; body?: unknown; query?: Record<string, string> }) => Promise<T>;

/** A write the service answered with a refusal: its status and its code (`docs/api.md`, 「Events」). */
export class Refusal extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string
  ) {
    super(message);
  }
}

/** A call that never reached the service: the network, not the service, said no. */
export class Unreachable extends Error {}

// What the fetcher threw, sorted into the two kinds the screen answers differently: a refusal the
// service gave, which takes the change back, and a call that never landed, which waits to be sent again.
function sorted(error: unknown): Error {
  const failed = error as { response?: { status?: number }; status?: number; statusCode?: number; data?: { code?: unknown; error?: unknown } } | null;
  const status = failed?.response?.status ?? failed?.statusCode ?? failed?.status;

  if (typeof status === "number" && status > 0) {
    const code = typeof failed?.data?.code === "string" ? failed.data.code : "failed";
    const message = typeof failed?.data?.error === "string" ? failed.data.error : String(status);

    return new Refusal(status, code, message);
  }

  return new Unreachable(error instanceof Error ? error.message : String(error));
}

function segment(value: string): string {
  return encodeURIComponent(value);
}

export interface CalendarApi {
  calendars: (realm: Realm) => Promise<Calendar[]>;
  addCalendar: (realm: Realm, write: CalendarWrite & { name: string; color: Calendar["color"] }) => Promise<Calendar>;
  changeCalendar: (realm: Realm, id: string, write: CalendarWrite) => Promise<Calendar>;
  retireCalendar: (realm: Realm, id: string) => Promise<void>;
  events: (realm: Realm, span: DateRange) => Promise<CalendarEventRow[]>;
  event: (realm: Realm, id: string) => Promise<CalendarEventRow>;
  addEvent: (realm: Realm, write: EventWrite & { id: string }) => Promise<CalendarEventRow>;
  changeEvent: (realm: Realm, id: string, write: Partial<EventWrite>) => Promise<CalendarEventRow>;
  cancelEvent: (realm: Realm, id: string, occurrence?: string | null) => Promise<void>;
  sources: (realm: Realm) => Promise<Source[]>;
  changeSource: (realm: Realm, id: string, write: { hidden: boolean }) => Promise<Source>;
  schedules: (realm: Realm, span: DateRange) => Promise<ScheduleRow[]>;
}

export function createCalendarApi(call: Call): CalendarApi {
  async function sent<T>(path: string, options?: Parameters<Call>[1]): Promise<T> {
    try {
      return await call<T>(path, options);
    } catch (error) {
      throw sorted(error);
    }
  }

  // Every page of a list, each one after the last item of the page before: a page that says there is more
  // is followed from its last item on.
  async function paged<T>(path: string, lastId: (item: T) => string, query: Record<string, string> = {}): Promise<T[]> {
    const items: T[] = [];
    let after: string | null = null;

    for (;;) {
      const asked = after === null ? query : { ...query, after };
      const page: ListPage<T> = await sent<ListPage<T>>(path, Object.keys(asked).length ? { query: asked } : undefined);

      items.push(...page.items);

      const last = page.items.at(-1);

      if (!page.more || last === undefined) return items;

      after = lastId(last);
    }
  }

  // The rows of a span, a span longer than the service reads at once read as several.
  async function spanned<T>(path: string, span: DateRange, lastId: (item: T) => string): Promise<T[]> {
    const rows: T[] = [];

    for (const part of spanParts(span)) rows.push(...(await paged<T>(path, lastId, { start: toLocalISO(part.start), end: toLocalISO(part.end) })));

    return rows;
  }

  // A realm's calendars, oldest first.
  const calendars = (realm: Realm) => paged<Calendar>(`/${realm}/calendars`, (calendar) => calendar.id);

  const events = (realm: Realm, span: DateRange) => spanned<CalendarEventRow>(`/${realm}/events`, span, (row) => row.id);

  return {
    calendars,
    addCalendar: (realm, write) => sent<Calendar>(`/${realm}/calendars`, { method: "POST", body: write }),
    changeCalendar: (realm, id, write) => sent<Calendar>(`/${realm}/calendars/${segment(id)}`, { method: "PATCH", body: write }),
    retireCalendar: (realm, id) => sent<void>(`/${realm}/calendars/${segment(id)}`, { method: "DELETE" }),
    events,
    event: (realm, id) => sent<CalendarEventRow>(`/${realm}/events/${segment(id)}`),
    addEvent: (realm, write) => sent<CalendarEventRow>(`/${realm}/events`, { method: "POST", body: write }),
    changeEvent: (realm, id, write) => sent<CalendarEventRow>(`/${realm}/events/${segment(id)}`, { method: "PATCH", body: write }),
    cancelEvent: (realm, id, occurrence) =>
      sent<void>(`/${realm}/events/${segment(id)}`, { method: "DELETE", ...(occurrence ? { query: { occurrence } } : {}) }),
    sources: (realm) => paged<Source>(`/${realm}/sources`, (source) => source.id),
    changeSource: (realm, id, write) => sent<Source>(`/${realm}/sources/${segment(id)}`, { method: "PATCH", body: write }),
    schedules: (realm, span) => spanned<ScheduleRow>(`/${realm}/schedules`, span, (row) => `${row.sourceId}:${row.itemId}`)
  };
}

/** A span cut into parts the service reads at once, each at most `MAX_SPAN_DAYS` long. */
export function spanParts(span: DateRange): DateRange[] {
  const parts: DateRange[] = [];
  let start = span.start;

  while (start < span.end) {
    const limit = new Date(start);

    limit.setDate(limit.getDate() + MAX_SPAN_DAYS);

    const end = limit < span.end ? limit : span.end;

    parts.push({ start, end });
    start = end;
  }

  return parts;
}
