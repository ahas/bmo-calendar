import { addDays, startOfDay } from "date-fns";
import { computed, ref, shallowRef, watch } from "vue";

import { nextOfEach, shiftedSeries } from "../domain/bookings";
import { toLocalISO } from "../domain/dates";
import { spanKey } from "../domain/store";
import { createAppComposable } from "./shared";
import { useCalendarApi } from "./useCalendarApi";
import { useCalendarEvents } from "./useCalendarEvents";
import { useCompanies } from "./useCompanies";
import { usePeople } from "./usePeople";

import type { Booking, BookingWrite, DateRange, RepeatScope, Resource, ResourceWrite } from "../../shared/types";

// What the booking screen holds of the calendar service (`docs/design.md`, 「The console」): the chosen
// company's resources and the bookings of the span in view, the reader's own bookings from today on, and
// the requests waiting on the resources the reader keeps. Every write is sent at once and its refusal
// handed back to the form that made it; once one lands, everything shown is read again, the calendar's
// grid among it, since the reader's bookings stand there too. An answer read before another company was
// chosen is let go as it lands; the first choice, the reader's own company, is no switch, since the reads
// that name no company already read it.

/** How far ahead the reader's own bookings are listed: the coming year. */
const MINE_DAYS = 366;

const _useBookings = () => {
  const { api } = useCalendarApi();
  const { manages, switches } = useCompanies();
  const { me } = usePeople();
  const calendar = useCalendarEvents();

  const resources = ref<Resource[]>([]);
  const resourcesRead = ref(false);
  const resourcesFailed = ref(false);

  const span = shallowRef<DateRange | null>(null);
  const bookings = ref<Booking[]>([]);
  const bookingsRead = ref(false);
  const bookingsFailed = ref(false);

  const mine = ref<Booking[]>([]);
  const mineRead = ref(false);
  const mineFailed = ref(false);

  const requests = ref<Booking[]>([]);
  const requestsRead = ref(false);
  const requestsFailed = ref(false);

  /** Whether the reader keeps a resource of the company, by the people directory's word for the reader. */
  const keeps = computed(() => {
    const user = me.value?.user;

    return !!user && resources.value.some((resource) => resource.keepers.includes(user));
  });

  function resourceOf(id: string): Resource | undefined {
    return resources.value.find((resource) => resource.id === id) ?? calendar.resourceOf(id);
  }

  /** The resources a booking of a company is written on: the chosen company's, or for the reader's own
   *  booking of another company — the administrator's own, read with another chosen — its own company's. */
  function resourcesFor(company: string | null): Resource[] {
    if (company === null || resources.value.some((resource) => resource.company === company)) return resources.value;

    return calendar.ownResources.value.some((resource) => resource.company === company) ? calendar.ownResources.value : resources.value;
  }

  /** Whether the reader may change or cancel a booking: its booker and a keeper of its resource. Where the
   *  directory does not say who the reader is, the service is left to answer. */
  function mayChange(booking: Booking): boolean {
    const user = me.value?.user;

    if (!user) return true;

    return booking.booker === user || !!resourceOf(booking.resourceId)?.keepers.includes(user);
  }

  async function loadResources(): Promise<void> {
    const asked = switches.value;

    try {
      const read = await api.resources();

      if (asked !== switches.value) return;

      resources.value = read;
      resourcesFailed.value = false;
    } catch {
      if (asked === switches.value) resourcesFailed.value = true;
    } finally {
      if (asked === switches.value) resourcesRead.value = true;
    }
  }

  // The answer of a span read after the screen moved on, to another span or another company, is left unread.
  let spanAsked = "";

  async function loadSpan(next: DateRange): Promise<void> {
    const key = `${switches.value}|${spanKey(next)}`;

    if (spanAsked !== key) bookingsRead.value = false;

    span.value = next;
    spanAsked = key;

    try {
      const read = await api.bookings("organization", next);

      if (spanAsked !== key) return;

      bookings.value = read;
      bookingsFailed.value = false;
    } catch {
      if (spanAsked === key) bookingsFailed.value = true;
    } finally {
      if (spanAsked === key) bookingsRead.value = true;
    }
  }

  async function loadMine(): Promise<void> {
    const start = startOfDay(new Date());

    try {
      // One row a booking, by its next occurrence in the year (`docs/design.md`, 「The console」, my bookings).
      mine.value = nextOfEach(await api.bookings("mine", { start, end: addDays(start, MINE_DAYS) }));
      mineFailed.value = false;
    } catch {
      mineFailed.value = true;
    } finally {
      mineRead.value = true;
    }
  }

  async function loadRequests(): Promise<void> {
    const asked = switches.value;

    try {
      const read = await api.bookingRequests();

      if (asked !== switches.value) return;

      requests.value = read;
      requestsFailed.value = false;
    } catch {
      if (asked === switches.value) requestsFailed.value = true;
    } finally {
      if (asked === switches.value) requestsRead.value = true;
    }
  }

  /** The bookings shown read again: the span's, the reader's own and the requests. */
  async function refreshBookings(): Promise<void> {
    await Promise.all([span.value ? loadSpan(span.value) : Promise.resolve(), loadMine(), loadRequests()]);
  }

  async function refreshAll(): Promise<void> {
    await Promise.all([loadResources(), refreshBookings()]);
  }

  /** The company changed: what was read of the last one is let go before the new one is read. */
  async function restart(): Promise<void> {
    resources.value = [];
    bookings.value = [];
    requests.value = [];
    resourcesRead.value = false;
    bookingsRead.value = false;
    requestsRead.value = false;
    await refreshAll();
  }

  // Another company chosen, on whichever screen.
  watch(switches, () => void restart());

  function landed(): void {
    void refreshBookings();
    void calendar.refreshVisible();
  }

  // ---- Bookings -----------------------------------------------------------------------------------

  async function book(write: BookingWrite): Promise<Booking> {
    const made = await api.addBooking({ ...write, id: crypto.randomUUID() });

    landed();

    return made;
  }

  /**
   * A booking changed: one occurrence of a repeating booking, which then stands as a booking of its own, or
   * the whole of it, moved by as much as the occurrence was.
   */
  async function changeBooking(before: Booking, after: Omit<BookingWrite, "id" | "occurrence">, scope: RepeatScope = "all"): Promise<Booking> {
    const write: Partial<BookingWrite> = {};

    for (const field of ["resourceId", "title", "allDay", "repeat"] as const) if (before[field] !== after[field]) Object.assign(write, { [field]: after[field] });

    const moved = before.start !== after.start || before.end !== after.end;
    const occurrence = before.occurrence;

    if (scope === "one" && occurrence) {
      Object.assign(write, { occurrence, start: after.start, end: after.end });
    } else if (moved) {
      Object.assign(write, occurrence ? shiftedSeries(await api.booking(before), before, after) : { start: after.start, end: after.end });
    }

    const changed = await api.changeBooking(before, write);

    landed();

    return changed;
  }

  async function cancelBooking(booking: Booking, scope: RepeatScope = "all"): Promise<void> {
    await api.cancelBooking(booking, scope === "one" ? booking.occurrence : null);
    landed();
  }

  async function decide(booking: Booking, approve: boolean, reason?: string): Promise<void> {
    await api.decideBooking(booking, approve, reason);
    landed();
  }

  // ---- Resources ----------------------------------------------------------------------------------

  async function resourceWrite<T>(send: () => Promise<T>): Promise<T> {
    const answer = await send();

    void loadResources();
    void calendar.loadResourceNames();

    return answer;
  }

  const addResource = (write: ResourceWrite & { name: string; kind: string }) => resourceWrite(() => api.addResource(write));
  const changeResource = (resource: Resource, write: ResourceWrite) => resourceWrite(() => api.changeResource(resource, write));

  async function retireResource(resource: Resource): Promise<void> {
    await resourceWrite(() => api.retireResource(resource));
    landed();
  }

  return {
    resources,
    resourcesRead,
    resourcesFailed,
    span,
    bookings,
    bookingsRead,
    bookingsFailed,
    mine,
    mineRead,
    mineFailed,
    requests,
    requestsRead,
    requestsFailed,
    manages,
    keeps,
    resourceOf,
    resourcesFor,
    mayChange,
    loadResources,
    loadSpan,
    loadMine,
    loadRequests,
    refreshBookings,
    refreshAll,
    restart,
    book,
    changeBooking,
    cancelBooking,
    decide,
    addResource,
    changeResource,
    retireResource
  };
};

export const useBookings = createAppComposable(_useBookings);

/** A local ISO span of the moments a form holds. */
export function isoSpan(span: DateRange): { start: string; end: string } {
  return { start: toLocalISO(span.start), end: toLocalISO(span.end) };
}
