import { addDays, startOfDay } from "date-fns";
import { useOnline } from "@vueuse/core";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { relayCall } from "bmo-ui/app/domain/relay";
import { useIsland } from "bmo-ui/app/domain/island";

import { createCalendarApi } from "../domain/api";
import { dayKey, fromLocalISO } from "../domain/dates";
import { createCalendarStore } from "../domain/store";
import { createAppComposable } from "./shared";

import type { RelayOptions } from "bmo-ui/app/domain/relay";
import type { Call } from "../domain/api";
import type { Calendar, CalendarEvent, Realm } from "../../shared/types";

// The calendars and events the screen draws, read and written through the console's relay to the
// calendar service, with the words the store has for the reader put on the console's message pill.

export const SERVICE = "calendar";

const relay: Call = <T>(path: string, options?: Parameters<Call>[1]) => relayCall<T>(SERVICE, path, options as RelayOptions);

// The calendar a new event was last put in, kept in this browser alone: a convenience of the reader's,
// not a setting of the account.
const LAST_USED = "bmo-calendar.last-used";

function readLastUsed(): string | null {
  try {
    return localStorage.getItem(LAST_USED);
  } catch {
    return null;
  }
}

export function calendarKey(calendar: Pick<Calendar, "realm" | "id">): string {
  return `${calendar.realm}:${calendar.id}`;
}

const _useCalendarEvents = () => {
  const { t } = useI18n();
  const island = useIsland(t);
  const store = createCalendarStore({
    api: createCalendarApi(relay),
    tell: (notice) => island.say({ tone: notice.tone, title: t(notice.title, notice.params ?? {}) })
  });

  const browserOnline = useOnline();

  watch(browserOnline, (isOnline) => (store.browserOnline.value = isOnline), { immediate: true });

  function calendarFor(event: Pick<CalendarEvent, "realm" | "calendarId">): Calendar | undefined {
    return store.calendarOf(event.realm, event.calendarId);
  }

  // Every event put in the days it covers once per change, so each day reads its own bucket rather than
  // scanning every event the views have read.
  const eventsByDay = computed(() => {
    const buckets = new Map<string, CalendarEvent[]>();

    for (const event of store.events.value) {
      const end = fromLocalISO(event.end);
      let day = startOfDay(fromLocalISO(event.start));

      do {
        const key = dayKey(day);
        const bucket = buckets.get(key);

        if (bucket) bucket.push(event);
        else buckets.set(key, [event]);

        day = addDays(day, 1);
      } while (day < end);
    }

    return buckets;
  });

  function eventsForDay(day: Date): CalendarEvent[] {
    return eventsByDay.value.get(dayKey(day)) ?? [];
  }

  function eventsForDays(days: Date[]): CalendarEvent[] {
    const seen = new Set<string>();

    return days.flatMap((day) =>
      eventsForDay(day).filter((event) => {
        if (seen.has(event.key)) return false;

        seen.add(event.key);

        return true;
      })
    );
  }

  const lastUsed = ref<string | null>(readLastUsed());

  function rememberCalendar(calendar: Pick<Calendar, "realm" | "id">): void {
    lastUsed.value = calendarKey(calendar);

    try {
      localStorage.setItem(LAST_USED, lastUsed.value);
    } catch {
      // A browser that keeps nothing asks again from the first calendar next time.
    }
  }

  /** The calendar a new event goes in first: the last one used where it is still shown, else the first shown. */
  const firstCalendar = computed<Calendar | undefined>(() => {
    const all = [...store.calendars.value.mine, ...store.calendars.value.organization];
    const shown = all.filter((calendar) => !calendar.hidden);

    return shown.find((calendar) => calendarKey(calendar) === lastUsed.value) ?? shown[0] ?? all[0];
  });

  function calendarByKey(key: string): Calendar | undefined {
    const [realm, ...rest] = key.split(":");

    return store.calendarOf(realm as Realm, rest.join(":"));
  }

  return { ...store, calendarFor, eventsForDay, eventsForDays, lastUsed, rememberCalendar, firstCalendar, calendarByKey };
};

export const useCalendarEvents = createAppComposable(_useCalendarEvents);
