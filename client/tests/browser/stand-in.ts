import type { Calendar, CalendarEventRow, Company, Realm, Resource, Source } from "../../shared/types";

// The calendar service as the harness page's relay answers a member of his own company and the platform administrator (`docs/api.md`, 「Companies」,
// 「Calendars」 and 「Events」): his own company, which he writes in, the platform's own realm, and another company,
// which he reads alone (「What the administrator writes」). Each company's organization realm holds a calendar, one
// event in the week the tests open and a resource (「Resources and bookings」), the other company's a source another unit
// pushes schedules under as well (「Sources and schedules」); his own realm holds one calendar, one event in that week and
// a busy day a week later, more events than a month cell has room for, and an all-day event a week earlier. Every other read answers an empty list, and a
// write is refused.

/** The week the tests open, and a day in it every event but the busy day's stands on. */
export const DAY = "2026-11-18";

/** The day a week later whose events outnumber a month cell's room. */
export const BUSY_DAY = "2026-11-25";

/** A day a week earlier with one all-day event, drawn in the month as a bar. */
export const ALL_DAY = "2026-11-11";

export const OWN_COMPANY = "10";
export const OTHER_COMPANY = "20";

export const COMPANIES: Company[] = [
  { id: OWN_COMPANY, name: "Hanbit", own: true, writes: true, manages: true },
  { id: "platform", name: "Platform", own: false, writes: true, manages: true },
  { id: OTHER_COMPANY, name: "Nuri", own: false, writes: false, manages: false }
];

function calendar(realm: Realm, company: string | null, id: string, name: string, color: Calendar["color"]): Calendar {
  return { id, realm, company, name, color, writers: realm === "mine" ? null : "creator", tell: realm === "mine" ? null : "creator", hidden: false, creator: "1", createdAt: 1 };
}

function event(realm: Realm, company: string | null, calendarId: string, id: string, title: string, hours: [string, string], day = DAY): CalendarEventRow {
  return {
    id,
    realm,
    company,
    calendarId,
    title,
    description: "",
    start: `${day}T${hours[0]}:00`,
    end: `${day}T${hours[1]}:00`,
    allDay: false,
    alarm: "none",
    repeat: "none",
    creator: "1",
    createdAt: 1,
    changedAt: 1,
    told: null,
    occurrence: null
  };
}

function resource(company: string, id: string, name: string): Resource {
  return { id, company, name, kind: "Meeting room", place: "", seats: 8, approval: false, keepers: [], creator: "1", createdAt: 1, changedAt: 1 };
}

/** What one realm holds. */
interface Held {
  calendars: Calendar[];
  events: CalendarEventRow[];
  sources?: Source[];
  resources?: Resource[];
}

const BUSY = ["Gym", "Call", "Lunch", "Sync", "Demo", "Read", "Plan"].map((title, at) =>
  event("mine", null, "personal", `event-busy-${at}`, title, [`${String(9 + at).padStart(2, "0")}:00`, `${String(10 + at).padStart(2, "0")}:00`], BUSY_DAY)
);

const DAY_OFF: CalendarEventRow = { ...event("mine", null, "personal", "event-day-off", "Day off", ["00:00", "00:00"], ALL_DAY), end: "2026-11-12T00:00:00", allDay: true };

const MINE: Held = {
  calendars: [calendar("mine", null, "personal", "Personal", "primary")],
  events: [event("mine", null, "personal", "event-own", "Dentist", ["08:00", "09:00"]), ...BUSY, DAY_OFF]
};

const ORGANIZATION: Record<string, Held> = {
  [OWN_COMPANY]: {
    calendars: [calendar("organization", OWN_COMPANY, "company-own", "Hanbit company", "info")],
    events: [event("organization", OWN_COMPANY, "company-own", "event-company", "Hanbit review", ["13:00", "14:00"])],
    resources: [resource(OWN_COMPANY, "resource-own", "Hanbit board room")]
  },
  [OTHER_COMPANY]: {
    calendars: [calendar("organization", OTHER_COMPANY, "company-other", "Nuri company", "success")],
    events: [{ ...event("organization", OTHER_COMPANY, "company-other", "event-other", "Nuri weekly meeting", ["10:00", "11:00"]), description: "Third floor room" }],
    sources: [{ id: "source-projects", name: "Project schedules", color: "warning", hidden: false, createdAt: 1 }],
    resources: [resource(OTHER_COMPANY, "resource-other", "Nuri hall")]
  }
};

/** Who the stand-in answers: a member reads his own company alone, whatever a call names (「Companies」). */
export type Reader = "administrator" | "member";

/** The companies a member reads: his own alone. */
export const MEMBER_COMPANIES: Company[] = [{ ...COMPANIES[0]!, manages: false }];

export interface Answer {
  status: number;
  body: unknown;
}

const items = (list: unknown[]): Answer => ({ status: 200, body: { items: list, more: false } });

/** The answer the stand-in gives a call: what it holds for a read, a refusal for any write. */
export function answerOf(method: string, path: string, query: URLSearchParams, reader: Reader = "administrator"): Answer {
  if (method !== "GET") return { status: 403, body: { code: "forbidden", error: "The stand-in takes no write." } };
  if (path === "/organization/companies") return items(reader === "member" ? MEMBER_COMPANIES : COMPANIES);

  const [realm, kind] = path.split("/").filter(Boolean) as [Realm, string];
  const company = reader === "member" ? OWN_COMPANY : (query.get("company") ?? OWN_COMPANY);
  const held = realm === "mine" ? MINE : ORGANIZATION[company];

  if (kind === "calendars") return items(held?.calendars ?? []);
  if (kind === "sources") return items(held?.sources ?? []);
  if (kind === "resources") return items(held?.resources ?? []);
  if (kind === "events") return items((held?.events ?? []).filter((row) => row.start < query.get("end")! && row.end > query.get("start")!));

  return items([]);
}
