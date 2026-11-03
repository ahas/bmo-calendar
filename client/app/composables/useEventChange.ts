import { differenceInMinutes, addMinutes } from "date-fns";
import { useOverlay } from "@nuxt/ui/composables/useOverlay";
import { useI18n } from "vue-i18n";

import CalendarScopeDialog from "../components/calendar/CalendarScopeDialog.vue";
import { fromLocalISO, toLocalISO } from "../domain/dates";
import { repeating } from "../domain/store";
import { useCalendarEvents } from "./useCalendarEvents";

import type { CalendarEvent, EventWrite, RepeatScope } from "../../shared/types";

// A change or a cancellation of an event as the grid and the form make it. A repeating event's row asks
// first whether the change reaches that occurrence alone or the whole series; the whole series takes the
// change as the row moved, from the series' own start.

const FIELDS = ["calendarId", "title", "description", "allDay", "alarm", "repeat"] as const;

export function useEventChange() {
  const { t } = useI18n();
  const overlay = useOverlay();
  const { updateEvent, removeEvent, readEvent } = useCalendarEvents();

  async function askScope(cancelling: boolean): Promise<RepeatScope | null> {
    const title = t(cancelling ? "calendar.message.scope_cancel" : "calendar.message.scope_change");
    const answer = await overlay.create(CalendarScopeDialog, { props: { title, cancelling }, destroyOnClose: true }).open();

    return (answer as RepeatScope | null | undefined) ?? null;
  }

  // The series moved by as much as the row was, and changed in every other field the row was.
  async function seriesWrite(before: CalendarEvent, after: CalendarEvent): Promise<Partial<EventWrite>> {
    const write: Partial<EventWrite> = {};

    for (const field of FIELDS) if (before[field] !== after[field]) Object.assign(write, { [field]: after[field] });

    if (before.start !== after.start || before.end !== after.end) {
      const series = await readEvent(before.realm, before.id);
      const startShift = differenceInMinutes(fromLocalISO(after.start), fromLocalISO(before.start));
      const endShift = differenceInMinutes(fromLocalISO(after.end), fromLocalISO(before.end));

      write.start = toLocalISO(addMinutes(fromLocalISO(series.start), startShift));
      write.end = toLocalISO(addMinutes(fromLocalISO(series.end), endShift));
    }

    return write;
  }

  /** Answers whether the change was sent; a reader who backed out of the question sent none. */
  async function changeEvent(before: CalendarEvent, after: CalendarEvent): Promise<boolean> {
    if (!repeating(before)) {
      await updateEvent(before, after);
      return true;
    }

    const scope = await askScope(false);

    if (scope === null) return false;

    if (scope === "one") await updateEvent(before, after, "one");
    else await updateEvent(before, after, "all", await seriesWrite(before, after));

    return true;
  }

  async function cancelEvent(event: CalendarEvent): Promise<boolean> {
    if (!repeating(event)) {
      await removeEvent(event);
      return true;
    }

    const scope = await askScope(true);

    if (scope === null) return false;

    await removeEvent(event, scope);

    return true;
  }

  return { changeEvent, cancelEvent };
}
