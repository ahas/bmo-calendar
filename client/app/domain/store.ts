import { computed, ref, shallowRef, watch } from "vue";

import { Refusal, Unreachable } from "./api";
import { fromLocalISO, toLocalISO } from "./dates";

import type { CalendarApi } from "./api";
import type { Calendar, CalendarEvent, CalendarEventRow, CalendarWrite, DateRange, EventWrite, Realm, RepeatScope, ScheduleRow, Source } from "../../shared/types";

// What the screen holds of the calendar service: the calendars and the sources of both realms, the events
// and received schedules of every span it has read, the changes it has made and not yet heard answered,
// and the changes waiting for the network.
//
// A change is drawn at once and sent behind it. A refusal takes it back and says why; a call that never
// reached the service keeps it drawn and puts it in a line that is sent again, in order, once the service
// answers again. Every write names its event's id, and the service answers a write it already holds as it
// stands, so a write sent twice is one write (`docs/api.md`, 「Events」).

/** A word the store has for the reader, as a key of the bundle's words and what fills it. */
export interface Notice {
  tone: "info" | "success" | "warning" | "error";
  title: string;
  params?: Record<string, string>;
}

export interface StoreOptions {
  api: CalendarApi;
  tell: (notice: Notice) => void;
  /** How long a store that lost the service waits between asking whether it answers again. */
  probeMs?: number;
}

interface Chunk {
  span: DateRange;
  rows: CalendarEvent[];
  loading: boolean;
  read: boolean;
  failed: boolean;
}

type Queued =
  | { kind: "add"; realm: Realm; key: string; write: EventWrite & { id: string } }
  | { kind: "change"; realm: Realm; key: string; id: string; write: Partial<EventWrite> }
  | { kind: "cancel"; realm: Realm; key: string; id: string; occurrence: string | null }
  | { kind: "calendar"; realm: Realm; id: string; write: CalendarWrite };

// The words a refusal is told in, by the service's code; any other code is told as a change that failed.
const REFUSAL_WORDS: Record<string, string> = {
  "not-a-writer": "calendar.error.not_a_writer",
  "end-before-start": "calendar.error.end_before_start",
  "unknown-calendar": "calendar.error.unknown_calendar"
};

export function refusalWord(error: unknown): string {
  return error instanceof Refusal ? (REFUSAL_WORDS[error.code] ?? "calendar.error.change_failed") : "calendar.error.change_failed";
}

/** The key one row is known by on the grid: its realm, its id and, for a repeating event, its occurrence. */
export function rowKey(row: Pick<CalendarEventRow, "realm" | "id" | "occurrence">): string {
  return `${row.realm}:${row.id}:${row.occurrence ?? ""}`;
}

export function held(row: CalendarEventRow): CalendarEvent {
  return { ...row, key: rowKey(row) };
}

/** A received schedule as a row of the grid: drawn as an event is, known by its realm, source and item, and
 *  carrying where it came from, which is what keeps every way of changing it off it. */
export function scheduleRow(row: ScheduleRow): CalendarEvent {
  const id = `${row.sourceId}:${row.itemId}`;

  return {
    id,
    realm: row.realm,
    calendarId: "",
    title: row.title,
    description: "",
    start: row.start,
    end: row.end,
    allDay: row.allDay,
    alarm: "none",
    repeat: "none",
    creator: "",
    createdAt: row.createdAt,
    changedAt: row.changedAt,
    told: null,
    occurrence: null,
    key: `schedule:${row.realm}:${id}`,
    origin: { sourceId: row.sourceId, itemId: row.itemId, link: row.link }
  };
}

export function spanKey(span: DateRange): string {
  return `${toLocalISO(span.start)}_${toLocalISO(span.end)}`;
}

function overlaps(row: CalendarEventRow, span: DateRange): boolean {
  const start = fromLocalISO(row.start);
  const end = fromLocalISO(row.end);

  return start < span.end && (end > span.start || (end.getTime() === start.getTime() && start >= span.start));
}

/** Whether a row stands for one occurrence of a repeating event. */
export function repeating(row: Pick<CalendarEventRow, "repeat" | "occurrence">): boolean {
  return row.repeat !== "none" || !!row.occurrence;
}

/** What a write of a row names: the fields the service keeps of it. */
export function writeOf(row: CalendarEventRow): EventWrite {
  return {
    calendarId: row.calendarId,
    title: row.title,
    description: row.description,
    start: row.start,
    end: row.end,
    allDay: row.allDay,
    alarm: row.alarm,
    repeat: row.repeat
  };
}

export function createCalendarStore(options: StoreOptions) {
  const { api, tell } = options;

  const calendars = ref<Record<Realm, Calendar[]>>({ mine: [], organization: [] });
  const sources = ref<Record<Realm, Source[]>>({ mine: [], organization: [] });

  // The layers whose shown or hidden mark is on its way to the service, as `calendar:<realm>:<id>` and
  // `source:<realm>:<id>`, so each shows it is being sent until the service answers.
  const sendingLayers = ref<string[]>([]);

  async function sendingLayer(key: string, send: () => Promise<void>): Promise<void> {
    sendingLayers.value = [...sendingLayers.value, key];

    try {
      await send();
    } finally {
      sendingLayers.value = sendingLayers.value.filter((one) => one !== key);
    }
  }
  const calendarsRead = ref(false);
  const calendarsFailed = ref(false);

  const chunks = shallowRef<Record<string, Chunk>>({});
  const visible = ref<string[]>([]);

  // The rows drawn ahead of the service's answer, by key, and the keys taken off the grid ahead of it.
  const put = ref<Record<string, CalendarEvent>>({});
  const gone = ref<string[]>([]);

  const queue = ref<Queued[]>([]);
  const browserOnline = ref(true);
  const forcedOffline = ref(false);
  const online = computed(() => browserOnline.value && !forcedOffline.value);

  // ---- Calendars ----------------------------------------------------------------------------------

  const groups = computed(() => (["mine", "organization"] as const).map((realm) => ({ realm, calendars: calendars.value[realm], sources: sources.value[realm] })));

  function calendarOf(realm: Realm, id: string): Calendar | undefined {
    return calendars.value[realm].find((calendar) => calendar.id === id);
  }

  function sourceOf(realm: Realm, id: string): Source | undefined {
    return sources.value[realm].find((source) => source.id === id);
  }

  /** What a row is drawn in and named by: its source for a received schedule, else its calendar. */
  function layerOf(row: Pick<CalendarEvent, "realm" | "calendarId" | "origin">): Calendar | Source | undefined {
    return row.origin ? sourceOf(row.realm, row.origin.sourceId) : calendarOf(row.realm, row.calendarId);
  }

  function replaceCalendar(next: Calendar): void {
    calendars.value = { ...calendars.value, [next.realm]: calendars.value[next.realm].map((one) => (one.id === next.id ? next : one)) };
  }

  async function loadCalendars(): Promise<void> {
    try {
      const [mine, organization, mineSources, organizationSources] = await Promise.all([
        api.calendars("mine"),
        api.calendars("organization"),
        api.sources("mine"),
        api.sources("organization")
      ]);

      calendars.value = { mine, organization };
      sources.value = { mine: mineSources, organization: organizationSources };
      calendarsFailed.value = false;
    } catch (error) {
      if (error instanceof Unreachable) forcedOffline.value = true;

      calendarsFailed.value = true;
    } finally {
      calendarsRead.value = true;
    }
  }

  // Shown or hidden on the grid, kept per account by the service (`docs/api.md`, 「Calendars」).
  async function setHidden(calendar: Calendar, hidden: boolean): Promise<void> {
    const before = calendarOf(calendar.realm, calendar.id) ?? calendar;

    replaceCalendar({ ...before, hidden });
    await sendingLayer(`calendar:${calendar.realm}:${calendar.id}`, () =>
      sendCalendar({ kind: "calendar", realm: calendar.realm, id: calendar.id, write: { hidden } }, () => replaceCalendar(before))
    );
  }

  // A source's layer shown or hidden in one realm, kept per account by the service (`docs/api.md`, 「Sources
  // and schedules」). Taken back where the service does not take it; a layer waits for no network.
  async function setSourceHidden(realm: Realm, source: Source, hidden: boolean): Promise<void> {
    const replace = (next: Source) => (sources.value = { ...sources.value, [realm]: sources.value[realm].map((one) => (one.id === next.id ? next : one)) });
    const before = sourceOf(realm, source.id) ?? source;

    replace({ ...before, hidden });

    await sendingLayer(`source:${realm}:${source.id}`, async () => {
      try {
        replace(await api.changeSource(realm, source.id, { hidden }));
      } catch (error) {
        if (error instanceof Unreachable) forcedOffline.value = true;

        replace(before);
        tell({ tone: "error", title: refusalWord(error) });
      }
    });
  }

  async function sendCalendar(request: Extract<Queued, { kind: "calendar" }>, undo: () => void): Promise<void> {
    if (!online.value) {
      queue.value = [...queue.value, request];
      return;
    }

    try {
      replaceCalendar(await api.changeCalendar(request.realm, request.id, request.write));
    } catch (error) {
      if (error instanceof Unreachable) {
        forcedOffline.value = true;
        queue.value = [...queue.value, request];
        return;
      }

      undo();
      tell({ tone: "error", title: refusalWord(error) });
    }
  }

  /** A calendar's settings, saved as one: the answer, or the refusal for the form to show. */
  async function saveCalendar(calendar: Calendar, write: CalendarWrite): Promise<Calendar> {
    const saved = await api.changeCalendar(calendar.realm, calendar.id, write);

    replaceCalendar(saved);

    return saved;
  }

  async function addCalendar(realm: Realm, write: CalendarWrite & { name: string; color: Calendar["color"] }): Promise<Calendar> {
    const made = await api.addCalendar(realm, write);

    calendars.value = { ...calendars.value, [realm]: [...calendars.value[realm], made] };

    return made;
  }

  async function retireCalendar(calendar: Calendar): Promise<void> {
    await api.retireCalendar(calendar.realm, calendar.id);

    calendars.value = { ...calendars.value, [calendar.realm]: calendars.value[calendar.realm].filter((one) => one.id !== calendar.id) };
    await refreshVisible();
  }

  // ---- Spans --------------------------------------------------------------------------------------

  function setChunk(key: string, chunk: Chunk): void {
    chunks.value = { ...chunks.value, [key]: chunk };
  }

  /** The events and received schedules of a span, both realms, read once and kept; `force` reads it again
   *  over what is kept. */
  async function loadSpan(span: DateRange, force = false): Promise<void> {
    const key = spanKey(span);
    const kept = chunks.value[key];

    if (kept && (kept.loading || (kept.read && !force))) return;

    setChunk(key, { span, rows: kept?.rows ?? [], loading: true, read: kept?.read ?? false, failed: false });

    // Each part a span is read in: one realm's events or its schedules, and which kept rows are its own.
    const parts = (["mine", "organization"] as const).flatMap((realm) => [
      { read: () => api.events(realm, span).then((rows) => rows.map(held)), owns: (row: CalendarEvent) => row.realm === realm && !row.origin },
      { read: () => api.schedules(realm, span).then((rows) => rows.map(scheduleRow)), owns: (row: CalendarEvent) => row.realm === realm && !!row.origin }
    ]);
    const read = await Promise.allSettled(parts.map((part) => part.read()));
    const rows = read.flatMap((one) => (one.status === "fulfilled" ? one.value : []));
    const failures = read.flatMap((one) => (one.status === "rejected" ? [one.reason] : []));

    if (failures.some((error) => error instanceof Unreachable)) forcedOffline.value = true;

    // A part that did not answer keeps what was read of it before rather than emptying its rows.
    const keptRows = (kept?.rows ?? []).filter((row) => read.some((one, index) => one.status === "rejected" && parts[index]!.owns(row)));

    setChunk(key, { span, rows: [...rows, ...keptRows], loading: false, read: true, failed: failures.length > 0 });
  }

  /** The spans a view stands on now, which a change the service tells of reads again. */
  function showSpans(spans: DateRange[]): void {
    visible.value = spans.map(spanKey);
  }

  /** Every span in view read again; spans out of view are let go, and read when they come back. */
  async function refreshVisible(): Promise<void> {
    const standing = chunks.value;
    const kept: Record<string, Chunk> = {};

    for (const key of visible.value) if (standing[key]) kept[key] = standing[key];

    chunks.value = kept;
    await Promise.all(Object.values(kept).map((chunk) => loadSpan(chunk.span, true)));
  }

  const pendingSpans = computed(() => Object.values(chunks.value).filter((chunk) => chunk.loading && !chunk.read).map((chunk) => chunk.span));
  const failedSpans = computed(() => Object.values(chunks.value).filter((chunk) => chunk.failed).map((chunk) => chunk.span));

  /** Every row read and drawn, the changes not yet answered over them, on the calendars shown. */
  const allEvents = computed<CalendarEvent[]>(() => {
    const merged = new Map<string, CalendarEvent>();

    for (const chunk of Object.values(chunks.value)) for (const row of chunk.rows) merged.set(row.key, row);
    for (const row of Object.values(put.value)) merged.set(row.key, row);
    for (const key of gone.value) merged.delete(key);

    return [...merged.values()];
  });

  // The rows on layers shown. A schedule shown in the reader's own realm stands there alone, rather than a
  // second time in the organization's.
  const events = computed(() => {
    const shown = allEvents.value.filter((row) => !layerOf(row)?.hidden);
    const own = new Set(shown.filter((row) => row.origin && row.realm === "mine").map((row) => row.id));

    return shown.filter((row) => !(row.origin && row.realm === "organization" && own.has(row.id)));
  });

  // The answered row takes the place of what was drawn for it in every span it falls in. Answers whether
  // any span read so far holds it.
  function landRow(oldKey: string, row: CalendarEvent | null): boolean {
    const next: Record<string, Chunk> = {};
    let landed = false;

    for (const [key, chunk] of Object.entries(chunks.value)) {
      const rows = chunk.rows.filter((one) => one.key !== oldKey && (row === null || one.key !== row.key));
      const holds = row !== null && overlaps(row, chunk.span);

      landed ||= holds;
      next[key] = { ...chunk, rows: holds ? [...rows, row] : rows };
    }

    chunks.value = next;

    return landed;
  }

  function dropPut(key: string): void {
    const { [key]: _dropped, ...rest } = put.value;

    put.value = rest;
  }

  function dropGone(keys: string[]): void {
    gone.value = gone.value.filter((key) => !keys.includes(key));
  }

  // ---- Writes -------------------------------------------------------------------------------------

  // One write sent: drawn already, answered here. A refusal takes it back with `undo`; a call that never
  // landed waits in the line.
  async function send(request: Exclude<Queued, { kind: "calendar" }>, undo: () => void, landed: (row: CalendarEventRow | null) => Promise<void> | void): Promise<void> {
    if (!online.value) {
      queue.value = [...queue.value, request];
      return;
    }

    try {
      await landed(await perform(request));
    } catch (error) {
      if (error instanceof Unreachable) {
        forcedOffline.value = true;
        queue.value = [...queue.value, request];
        return;
      }

      undo();
      tell({ tone: "error", title: refusalWord(error) });
    }
  }

  async function perform(request: Queued): Promise<CalendarEventRow | null> {
    switch (request.kind) {
      case "add":
        return api.addEvent(request.realm, request.write);
      case "change":
        return api.changeEvent(request.realm, request.id, request.write);
      case "cancel":
        await api.cancelEvent(request.realm, request.id, request.occurrence);
        return null;
      case "calendar":
        replaceCalendar(await api.changeCalendar(request.realm, request.id, request.write));
        return null;
    }
  }

  async function settled(request: Queued, row: CalendarEventRow | null): Promise<void> {
    if (request.kind === "calendar") return;

    if (request.kind === "cancel") {
      // A cancelled row is gone from the service's answers from now on.
      if (request.occurrence !== null || !request.key.endsWith(":")) {
        await reloadRead();
      } else {
        landRow(request.key, null);
      }

      dropGone(gone.value.filter((key) => key === request.key || key.startsWith(`${request.realm}:${request.id}:`)));
      return;
    }

    // A repeating event's rows, or an occurrence that became an event of its own, are read again whole.
    if (row === null || repeating(row) || ("occurrence" in request.write && request.write.occurrence)) {
      await reloadRead();
      dropPut(request.key);
      return;
    }

    // A row no span read so far holds stays drawn as the service answered it, until a span that holds it is read.
    if (landRow(request.key, held(row))) dropPut(request.key);
    else put.value = { ...put.value, [request.key]: held(row) };
  }

  /** Every span already read, read again over what is kept of it. */
  async function reloadRead(): Promise<void> {
    await Promise.all(Object.values(chunks.value).map((chunk) => loadSpan(chunk.span, true)));
  }

  /** A new event, drawn at once and sent under the id it was made with. */
  async function addEvent(row: CalendarEvent): Promise<void> {
    const request: Queued = { kind: "add", realm: row.realm, key: row.key, write: { ...writeOf(row), id: row.id } };

    put.value = { ...put.value, [row.key]: row };
    await send(request, () => dropPut(row.key), (answer) => settled(request, answer));
  }

  /**
   * A changed event, drawn at once. A repeating event's row changes that one occurrence, which then
   * stands as an event of its own, or the whole series, as the reader chose.
   */
  async function updateEvent(before: CalendarEvent, after: CalendarEvent, scope: RepeatScope = "one", seriesWrite?: Partial<EventWrite>): Promise<void> {
    if (gone.value.includes(before.key)) return;

    const previous = put.value[before.key];
    const write: Partial<EventWrite> = scope === "all" && seriesWrite ? seriesWrite : writeOf(after);

    if (scope === "one" && before.occurrence) write.occurrence = before.occurrence;

    const request: Queued = { kind: "change", realm: before.realm, key: before.key, id: before.id, write };

    put.value = { ...put.value, [before.key]: { ...after, key: before.key } };
    await send(
      request,
      () => {
        if (previous) put.value = { ...put.value, [before.key]: previous };
        else dropPut(before.key);
      },
      (answer) => settled(request, answer)
    );
  }

  /** An event cancelled: one occurrence of a repeating event, or the whole of it. */
  async function removeEvent(row: CalendarEvent, scope: RepeatScope = "one"): Promise<void> {
    const occurrence = scope === "one" ? (row.occurrence ?? null) : null;
    const keys = scope === "all" ? allEvents.value.filter((one) => one.realm === row.realm && one.id === row.id).map((one) => one.key) : [row.key];
    const fresh = keys.filter((key) => !gone.value.includes(key));
    const request: Queued = { kind: "cancel", realm: row.realm, key: scope === "all" ? `${row.realm}:${row.id}:` : row.key, id: row.id, occurrence };

    gone.value = [...gone.value, ...fresh];
    await send(request, () => dropGone(fresh), (answer) => settled(request, answer));
  }

  // ---- The line of changes waiting for the network ---------------------------------------------------

  let replaying = false;

  /** Every waiting change sent again in the order it was made; one refused is taken back and the rest go on. */
  async function replay(): Promise<void> {
    if (replaying || !online.value || queue.value.length === 0) return;

    replaying = true;

    const waiting = queue.value;
    let landed = false;

    queue.value = [];

    try {
      for (const [index, request] of waiting.entries()) {
        try {
          await settled(request, await perform(request));
          landed = true;
        } catch (error) {
          if (error instanceof Unreachable) {
            queue.value = [...waiting.slice(index), ...queue.value];
            forcedOffline.value = true;
            return;
          }

          takeBack(request);
          tell({ tone: "error", title: "calendar.error.sync_failed" });
        }
      }

      if (landed) tell({ tone: "success", title: "calendar.message.synced" });
    } finally {
      replaying = false;
    }
  }

  function takeBack(request: Queued): void {
    if (request.kind === "calendar") {
      void loadCalendars();
      return;
    }

    if (request.kind === "cancel") dropGone(gone.value.filter((key) => key === request.key || key.startsWith(`${request.realm}:${request.id}:`)));
    else dropPut(request.key);
  }

  /** Whether the service answers again: a store that lost it asks this until it does. */
  async function probe(): Promise<boolean> {
    try {
      await api.calendars("mine");
      forcedOffline.value = false;

      return true;
    } catch {
      return false;
    }
  }

  let probing: ReturnType<typeof setInterval> | null = null;

  watch(forcedOffline, (offline) => {
    if (offline && probing === null) probing = setInterval(() => void probe(), options.probeMs ?? 5_000);

    if (!offline && probing !== null) {
      clearInterval(probing);
      probing = null;
    }
  });

  watch(online, (isOnline) => {
    if (isOnline) void replay();
  });

  function dispose(): void {
    if (probing !== null) clearInterval(probing);

    probing = null;
  }

  /** One event as the service holds it: a repeating event's series, not one of its rows. */
  function readEvent(realm: Realm, id: string): Promise<CalendarEventRow> {
    return api.event(realm, id);
  }

  return {
    readEvent,
    calendars,
    calendarsRead,
    calendarsFailed,
    groups,
    calendarOf,
    sources,
    sourceOf,
    layerOf,
    setSourceHidden,
    sendingLayers,
    loadCalendars,
    setHidden,
    saveCalendar,
    addCalendar,
    retireCalendar,
    loadSpan,
    showSpans,
    refreshVisible,
    reloadRead,
    pendingSpans,
    failedSpans,
    allEvents,
    events,
    addEvent,
    updateEvent,
    removeEvent,
    queue,
    browserOnline,
    forcedOffline,
    online,
    replay,
    probe,
    dispose
  };
}

export type CalendarStore = ReturnType<typeof createCalendarStore>;
