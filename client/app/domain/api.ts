import { MAX_SPAN_DAYS, toLocalISO } from "./dates";

import type {
  Booking,
  BookingWrite,
  Calendar,
  CalendarEventRow,
  CalendarWrite,
  Company,
  DateRange,
  EventWrite,
  ListPage,
  Realm,
  Repeat,
  Resource,
  ResourceWrite,
  ScheduleRow,
  Source
} from "../../shared/types";

// The screen's calls to the calendar service, each at the path the service's own API writes it
// (`docs/api.md`): the companies the reader reads, the calendars of a realm, the events of a span and the
// writes on either, the sources and schedules other units push, which the screen reads and only hides or
// shows, and a company's resources and their bookings. The caller hands the call itself — through the
// console's relay in the browser, to a stand-in in a test — so this module names no address and no fetcher.
//
// Every call in the organization realm reads the company the caller chose (`docs/api.md`, 「Companies」):
// the administrator's choice, named in the query, and for every other reader none, so a member's call is
// always its own company's. A call on a row the screen already holds — a calendar, an event, a booking, a
// resource — names the row's own company instead, so a change goes to the company the row stands in
// whichever company is chosen by the time it is sent.

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

/** The company a call in the organization realm names: the administrator's choice, or null for none. */
export type CompanyChoice = () => string | null;

/** What the free resources of a span are asked for (`docs/api.md`, 「Resources and bookings」). */
export interface FreeQuery {
  span: DateRange;
  seats?: number;
  repeat?: Repeat;
  kind?: string;
}

export interface CalendarApi {
  companies: () => Promise<Company[]>;
  calendars: (realm: Realm) => Promise<Calendar[]>;
  addCalendar: (realm: Realm, write: CalendarWrite & { name: string; color: Calendar["color"] }) => Promise<Calendar>;
  changeCalendar: (realm: Realm, id: string, write: CalendarWrite, company?: string | null) => Promise<Calendar>;
  retireCalendar: (realm: Realm, id: string, company?: string | null) => Promise<void>;
  events: (realm: Realm, span: DateRange) => Promise<CalendarEventRow[]>;
  event: (realm: Realm, id: string, company?: string | null) => Promise<CalendarEventRow>;
  /** A write on an event names the company of its row: a new one the company of the calendar it goes in. */
  addEvent: (realm: Realm, write: EventWrite & { id: string }, company?: string | null) => Promise<CalendarEventRow>;
  changeEvent: (realm: Realm, id: string, write: Partial<EventWrite>, company?: string | null) => Promise<CalendarEventRow>;
  cancelEvent: (realm: Realm, id: string, occurrence?: string | null, company?: string | null) => Promise<void>;
  sources: (realm: Realm) => Promise<Source[]>;
  changeSource: (realm: Realm, id: string, write: { hidden: boolean }) => Promise<Source>;
  schedules: (realm: Realm, span: DateRange) => Promise<ScheduleRow[]>;
  /** The company's live resources; `company` null reads the reader's own company whatever is chosen. */
  resources: (company?: string | null) => Promise<Resource[]>;
  freeResources: (query: FreeQuery) => Promise<Resource[]>;
  addResource: (write: ResourceWrite & { name: string; kind: string }) => Promise<Resource>;
  changeResource: (resource: Pick<Resource, "id" | "company">, write: ResourceWrite) => Promise<Resource>;
  retireResource: (resource: Pick<Resource, "id" | "company">) => Promise<void>;
  /** The bookings of a span: in the organization realm the company's standing ones, in the reader's own
   *  realm the reader's, refused ones among them. */
  bookings: (realm: Realm, span: DateRange, resourceId?: string) => Promise<Booking[]>;
  booking: (booking: Pick<Booking, "id" | "company">) => Promise<Booking>;
  bookingRequests: () => Promise<Booking[]>;
  addBooking: (write: BookingWrite & { id: string }) => Promise<Booking>;
  changeBooking: (booking: Pick<Booking, "id" | "company">, write: Partial<BookingWrite>) => Promise<Booking>;
  cancelBooking: (booking: Pick<Booking, "id" | "company">, occurrence?: string | null) => Promise<void>;
  decideBooking: (booking: Pick<Booking, "id" | "company">, approve: boolean, reason?: string) => Promise<Booking>;
}

const ORGANIZATION_PREFIX = "/organization/";
const COMPANIES_PATH = "/organization/companies";

export function createCalendarApi(call: Call, chosenCompany: CompanyChoice = () => null): CalendarApi {
  // The company a call names: none for a reader who chose none, else the row's own or the one chosen. A
  // read of the reader's own company names none at all (`company` null).
  function companyQuery(path: string, company: string | null | undefined): Record<string, string> {
    const chosen = chosenCompany();

    if (!path.startsWith(ORGANIZATION_PREFIX) || path === COMPANIES_PATH || chosen === null || company === null) return {};

    return { company: company ?? chosen };
  }

  async function sent<T>(path: string, options?: Parameters<Call>[1], company?: string | null): Promise<T> {
    const scoped = companyQuery(path, company);
    const query = { ...scoped, ...(options?.query ?? {}) };
    const asked = Object.keys(query).length ? { ...options, query } : options;

    try {
      return await call<T>(path, asked);
    } catch (error) {
      throw sorted(error);
    }
  }

  // Every page of a list, each one after the last item of the page before: a page that says there is more
  // is followed from its last item on.
  async function paged<T>(path: string, lastId: (item: T) => string, query: Record<string, string> = {}, company?: string | null): Promise<T[]> {
    const items: T[] = [];
    let after: string | null = null;

    for (;;) {
      const asked = after === null ? query : { ...query, after };
      const page: ListPage<T> = await sent<ListPage<T>>(path, Object.keys(asked).length ? { query: asked } : undefined, company);

      items.push(...page.items);

      const last = page.items.at(-1);

      if (!page.more || last === undefined) return items;

      after = lastId(last);
    }
  }

  // The rows of a span, a span longer than the service reads at once read as several.
  async function spanned<T>(path: string, span: DateRange, lastId: (item: T) => string, query: Record<string, string> = {}): Promise<T[]> {
    const rows: T[] = [];

    for (const part of spanParts(span)) rows.push(...(await paged<T>(path, lastId, { ...query, start: toLocalISO(part.start), end: toLocalISO(part.end) })));

    return rows;
  }

  // A realm's calendars, oldest first.
  const calendars = (realm: Realm) => paged<Calendar>(`/${realm}/calendars`, (calendar) => calendar.id);

  const events = (realm: Realm, span: DateRange) => spanned<CalendarEventRow>(`/${realm}/events`, span, (row) => row.id);

  // A page of bookings is followed from its last row's id, as a page of events is.
  const bookingKey = (row: Booking) => row.id;
  const bookingPath = (booking: Pick<Booking, "id">) => `/organization/bookings/${segment(booking.id)}`;
  const resourcePath = (resource: Pick<Resource, "id">) => `/organization/resources/${segment(resource.id)}`;

  return {
    companies: () => paged<Company>(COMPANIES_PATH, (company) => company.id),
    calendars,
    addCalendar: (realm, write) => sent<Calendar>(`/${realm}/calendars`, { method: "POST", body: write }),
    changeCalendar: (realm, id, write, company) => sent<Calendar>(`/${realm}/calendars/${segment(id)}`, { method: "PATCH", body: write }, company),
    retireCalendar: (realm, id, company) => sent<void>(`/${realm}/calendars/${segment(id)}`, { method: "DELETE" }, company),
    events,
    event: (realm, id, company) => sent<CalendarEventRow>(`/${realm}/events/${segment(id)}`, undefined, company),
    addEvent: (realm, write, company) => sent<CalendarEventRow>(`/${realm}/events`, { method: "POST", body: write }, company),
    changeEvent: (realm, id, write, company) => sent<CalendarEventRow>(`/${realm}/events/${segment(id)}`, { method: "PATCH", body: write }, company),
    cancelEvent: (realm, id, occurrence, company) =>
      sent<void>(`/${realm}/events/${segment(id)}`, { method: "DELETE", ...(occurrence ? { query: { occurrence } } : {}) }, company),
    sources: (realm) => paged<Source>(`/${realm}/sources`, (source) => source.id),
    changeSource: (realm, id, write) => sent<Source>(`/${realm}/sources/${segment(id)}`, { method: "PATCH", body: write }),
    schedules: (realm, span) => spanned<ScheduleRow>(`/${realm}/schedules`, span, (row) => `${row.sourceId}:${row.itemId}`),
    resources: (company) => paged<Resource>("/organization/resources", (resource) => resource.id, {}, company),
    freeResources: ({ span, seats, repeat, kind }) =>
      paged<Resource>("/organization/resources/free", (resource) => resource.id, {
        start: toLocalISO(span.start),
        end: toLocalISO(span.end),
        ...(seats ? { seats: String(seats) } : {}),
        ...(repeat ? { repeat } : {}),
        ...(kind ? { kind } : {})
      }),
    addResource: (write) => sent<Resource>("/organization/resources", { method: "POST", body: write }),
    changeResource: (resource, write) => sent<Resource>(resourcePath(resource), { method: "PATCH", body: write }, resource.company),
    retireResource: (resource) => sent<void>(resourcePath(resource), { method: "DELETE" }, resource.company),
    bookings: (realm, span, resourceId) => spanned<Booking>(`/${realm}/bookings`, span, bookingKey, resourceId ? { resourceId } : {}),
    booking: (booking) => sent<Booking>(bookingPath(booking), undefined, booking.company),
    bookingRequests: () => paged<Booking>("/organization/bookings/requests", bookingKey),
    addBooking: (write) => sent<Booking>("/organization/bookings", { method: "POST", body: write }),
    changeBooking: (booking, write) => sent<Booking>(bookingPath(booking), { method: "PATCH", body: write }, booking.company),
    cancelBooking: (booking, occurrence) => sent<void>(bookingPath(booking), { method: "DELETE", ...(occurrence ? { query: { occurrence } } : {}) }, booking.company),
    decideBooking: (booking, approve, reason) =>
      sent<Booking>(`${bookingPath(booking)}/decision`, { method: "POST", body: { approve, ...(reason ? { reason } : {}) } }, booking.company)
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
