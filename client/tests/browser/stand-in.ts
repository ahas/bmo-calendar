import type { Calendar, CalendarEventRow, Company, Realm, Source } from "../../shared/types";

// The calendar service as the harness page's relay answers the platform administrator (`docs/api.md`, 「Companies」,
// 「Calendars」 and 「Events」): his own company, which he writes in, the platform's own realm, and another company,
// which he reads alone (「What the administrator writes」). Each company's organization realm holds a calendar and one
// event in the week the tests open, the other company's a source another unit pushes schedules under as well
// (「Sources and schedules」); his own realm holds one calendar and one event. Every other read answers an empty list, and a
// write is refused.

/** The week the tests open, and a day in it every event stands on. */
export const DAY = "2026-11-18";

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

function event(realm: Realm, company: string | null, calendarId: string, id: string, title: string, hours: [string, string]): CalendarEventRow {
  return {
    id,
    realm,
    company,
    calendarId,
    title,
    description: "",
    start: `${DAY}T${hours[0]}:00`,
    end: `${DAY}T${hours[1]}:00`,
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

/** What one realm holds. */
interface Held {
  calendars: Calendar[];
  events: CalendarEventRow[];
  sources?: Source[];
}

const MINE: Held = { calendars: [calendar("mine", null, "personal", "Personal", "primary")], events: [event("mine", null, "personal", "event-own", "Dentist", ["08:00", "09:00"])] };

const ORGANIZATION: Record<string, Held> = {
  [OWN_COMPANY]: {
    calendars: [calendar("organization", OWN_COMPANY, "company-own", "Hanbit company", "info")],
    events: [event("organization", OWN_COMPANY, "company-own", "event-company", "Weekly meeting", ["13:00", "14:00"])]
  },
  [OTHER_COMPANY]: {
    calendars: [calendar("organization", OTHER_COMPANY, "company-other", "Nuri company", "success")],
    events: [{ ...event("organization", OTHER_COMPANY, "company-other", "event-other", "Nuri weekly meeting", ["10:00", "11:00"]), description: "Third floor room" }],
    sources: [{ id: "source-projects", name: "Project schedules", color: "warning", hidden: false, createdAt: 1 }]
  }
};

export interface Answer {
  status: number;
  body: unknown;
}

const items = (list: unknown[]): Answer => ({ status: 200, body: { items: list, more: false } });

/** The answer the stand-in gives a call: what it holds for a read, a refusal for any write. */
export function answerOf(method: string, path: string, query: URLSearchParams): Answer {
  if (method !== "GET") return { status: 403, body: { code: "forbidden", error: "The stand-in takes no write." } };
  if (path === "/organization/companies") return items(COMPANIES);

  const [realm, kind] = path.split("/").filter(Boolean) as [Realm, string];
  const held = realm === "mine" ? MINE : ORGANIZATION[query.get("company") ?? OWN_COMPANY];

  if (kind === "calendars") return items(held?.calendars ?? []);
  if (kind === "sources") return items(held?.sources ?? []);
  if (kind === "events") return items((held?.events ?? []).filter((row) => row.start < query.get("end")! && row.end > query.get("start")!));

  return items([]);
}
