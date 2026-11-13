import { onBeforeUnmount, onMounted } from "vue";
import { relayAddress } from "bmo-ui/app/domain/relay";

import type { Realm } from "../../shared/types";

// The calendar service's event stream as the screen reads it (`docs/api.md`, 「Events the calendar
// publishes」), through the console's relay. The browser resumes it after the last event it read; where
// the service no longer keeps the events since then it refuses the stream, and the screen opens it afresh
// and reads what it shows again.

// The kind a calendar's change stands under, `calendar.changed`. It is put together from its parts, since the
// bundle build reads a dotted string the screen writes whole under the bundle's name as a key of its words.
const CALENDAR_DOMAIN = "calendar";

export const CALENDAR_CHANGED = `${CALENDAR_DOMAIN}.changed` as const;

export type CalendarStreamEvent =
  | { kind: typeof CALENDAR_CHANGED; data: { realm: Realm; calendarId: string } }
  | { kind: "event.changed"; data: { realm: Realm; eventId: string; occurrence: string | null } }
  | { kind: "alarm.told"; data: { realm: Realm; eventId: string; occurrence: string; at: number } }
  | { kind: "source.changed"; data: { sourceId: string } }
  | { kind: "schedule.changed"; data: { sourceId: string; itemId: string } };

export const STREAM_KINDS = [CALENDAR_CHANGED, "event.changed", "alarm.told", "source.changed", "schedule.changed"] as const;

// How long a stream the service refused waits before it is opened afresh.
const REOPEN_MS = 3_000;

/** Calls `onEvent` with each event while the screen holding it stands, and `onReset` each time it is opened afresh. */
export function useCalendarStream(service: string, onEvent: (event: CalendarStreamEvent) => void, onReset: () => void): void {
  let source: EventSource | null = null;
  let reopening: ReturnType<typeof setTimeout> | null = null;
  let standing = false;

  function read(message: MessageEvent<string>): void {
    try {
      const envelope = JSON.parse(message.data) as { kind?: string; data?: unknown };

      onEvent({ kind: (envelope.kind ?? message.type) as CalendarStreamEvent["kind"], data: envelope.data } as CalendarStreamEvent);
    } catch {
      // An event that does not read is passed over; the next one says where things stand.
    }
  }

  function open(): void {
    source = new EventSource(relayAddress(service, `/events?kinds=${STREAM_KINDS.join(",")}`));

    for (const kind of STREAM_KINDS) source.addEventListener(kind, read);

    source.onerror = () => {
      if (source?.readyState !== EventSource.CLOSED || !standing) return;

      source = null;
      reopening = setTimeout(() => {
        reopening = null;

        if (!standing) return;

        open();
        onReset();
      }, REOPEN_MS);
    };
  }

  onMounted(() => {
    standing = true;
    open();
  });

  onBeforeUnmount(() => {
    standing = false;
    source?.close();
    source = null;

    if (reopening !== null) clearTimeout(reopening);
  });
}
