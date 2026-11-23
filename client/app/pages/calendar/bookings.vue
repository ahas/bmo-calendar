<template>
  <!-- The booking screen (docs/design.md, 「The console」), reached from the calendar screen's sidebar and back
       again: the company's resources of one kind by day, week or month with their bookings, the reader's own
       bookings from today on, the requests waiting on what the reader keeps, and the resources themselves.
       The way back, the parts of the screen and the toolbar acting on the part shown stand in the screen's
       own rows; the one create button makes a new booking, and a resource while the resources stand. On a
       phone the row keeps one line: the title takes its short form and the kind is cut to its room. For the
       administrator the part shown stands beside the list of companies under those rows. Where it stands, the
       company with it, is the address. -->
  <UDashboardPanel id="calendar-bookings" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0 overflow-y-hidden [&>*]:max-w-none' }">
    <template v-if="!consoleBar" #header>
      <UDashboardNavbar :title="$t('calendar.label.bookings')" :icon="BOOKING_ICON" />
    </template>

    <template #body>
      <div class="flex h-9 shrink-0 items-stretch pointer-coarse:h-10" :class="TAB_ROW_LINE">
        <div class="flex shrink-0 items-center border-e border-default px-1">
          <IconButton size="sm" icon="i-material-symbols-arrow-back" :label="$t('calendar.action.back')" @click="back" />
        </div>
        <UTabs :model-value="tab" :items="tabItems" variant="link" :content="false" :ui="TAB_ROW_UI" class="min-w-0 flex-1" @update:model-value="go({ tab: $event as BookingTab })" />
      </div>

      <ToolBar :label="$t('calendar.aria.bookings_toolbar')">
        <template v-if="tab === 'all'">
          <div>
            <IconButton size="sm" icon="i-material-symbols-chevron-left" :label="$t('calendar.action.previous')" :to="pathFor({ date: prevDate })" />
            <IconButton size="sm" icon="i-material-symbols-chevron-right" :label="$t('calendar.action.next')" :to="pathFor({ date: nextDate })" />
          </div>

          <div class="min-w-0 flex-1">
            <UPopover v-model:open="pickerOpen" :content="{ align: 'start' }">
              <UButton
                color="neutral"
                variant="ghost"
                size="sm"
                class="max-w-full min-w-0 py-1 font-semibold text-highlighted"
                :class="narrow ? 'text-sm' : 'text-lg/6'"
                :aria-label="$t('calendar.aria.pick_day_named', { title: `${title.months} ${title.year}`.trim() })"
              >
                <span v-if="narrow" class="truncate">{{ shortTitle }}</span>
                <template v-else>
                  <span class="shrink-0 whitespace-nowrap">{{ title.months }}</span>
                  <span v-if="title.year" class="shrink-[100] truncate font-normal text-muted">{{ title.year }}</span>
                </template>
              </UButton>

              <template #content>
                <DateCalendar v-model="pickedDay" :week-starts-on="WEEK_STARTS_ON" class="p-2" />
              </template>
            </UPopover>
          </div>

          <div v-if="!narrow">
            <UButton color="neutral" variant="ghost" size="sm" :label="$t('calendar.action.today')" :to="pathFor({ date: todayDate() })" />
          </div>

          <div>
            <ChoiceSelect
              :model-value="view"
              :items="viewItems"
              value-key="value"
              color="neutral"
              variant="ghost"
              size="sm"
              trailing-icon="i-material-symbols-arrow-drop-down"
              :aria-label="$t('calendar.aria.view_named', { view: $t(VIEW_LABELS[view]) })"
              @update:model-value="(one: BookingView) => go({ view: one })"
            />
          </div>

          <!-- On a phone the kind is cut to its room, its whole name in the shared tooltip of a cut line. -->
          <div v-if="kinds.length" class="min-w-0 shrink!">
            <ChoiceSelect
              :model-value="shownKind ?? undefined"
              :items="kindItems"
              value-key="value"
              color="neutral"
              variant="ghost"
              size="sm"
              trailing-icon="i-material-symbols-arrow-drop-down"
              class="min-w-0 max-sm:max-w-20"
              :aria-label="$t('calendar.aria.kind_named', { kind: shownKind ?? '' })"
              @update:model-value="(one: string) => go({ kind: one })"
            />
          </div>
        </template>

        <div v-if="tab !== 'all'" class="min-w-0 flex-1" />

        <div>
          <IconButton size="sm" icon="i-material-symbols-refresh" :label="$t('calendar.action.refresh')" :loading="refreshing" @click="refresh" />
        </div>

        <!-- While the resources stand, the one create button adds a resource where the reader manages them, and
             stands absent otherwise; elsewhere it makes a booking, in the reader's own company alone. -->
        <template v-if="tab === 'resources'">
          <CreateButton v-if="manages" :key="tab" :label="$t('calendar.action.add_resource')" @click="resourceListRef?.add()" />
        </template>
        <CreateButton v-else :key="tab" :label="$t('calendar.action.new_booking')" :disabled="!resources.length || !books" @click="bookAnew" />
      </ToolBar>

      <!-- For the administrator the companies stand beside the part shown; a member reads one company and is
           shown the part alone. -->
      <ChoiceSidebar
        :label="$t('calendar.label.organizations')"
        :entries="companyEntries"
        :chosen="chosenCompany"
        :listed="companiesListed"
        save-id="calendar-bookings-companies"
        :no-match-text="$t('calendar.status.no_organization_match')"
        @choose="chooseCompany"
      >
        <template v-if="tab === 'all'">
          <p v-if="bookingsFailed" role="alert" class="badge-ink shrink-0 border-b border-default px-4 py-2 text-sm text-error">{{ $t("calendar.error.bookings_failed") }}</p>

          <p v-if="resourcesRead && !resources.length" class="px-4 py-3 text-sm" :class="resourcesFailed ? 'badge-ink text-error' : 'text-muted'" :role="resourcesFailed ? 'alert' : undefined">
            {{ $t(resourcesFailed ? "calendar.error.resources_failed" : "calendar.message.no_resources") }}
          </p>
          <BookingMonth
            v-else-if="view === 'month'"
            :days="days"
            :month="toDate(date)"
            :resources="kindResources"
            :bookings="bookings"
            :loading="!bookingsRead"
            :day-path="(day: Date) => pathFor({ view: 'day', date: toCalendarDate(day) })"
          />
          <BookingWeek v-else-if="view === 'week'" :days="days" :groups="groups" :bookings="bookings" :loading="!resourcesRead" :bookable="books" @book="bookOn" @open="openBooking" />
          <BookingTimeline
            v-else
            :day="days[0]!"
            :groups="groups"
            :bookings="bookings"
            :loading="!resourcesRead"
            :bookable="books"
            :draft="dialogOpen ? draft : null"
            @book="bookOn"
            @open="openBooking"
          />
        </template>

        <BookingList v-else-if="tab === 'mine'" :bookings="mine" :read="mineRead" :failed="mineFailed" :empty="$t('calendar.message.no_bookings')">
          <template #actions="{ booking }">
            <template v-if="booking.state !== 'refused'">
              <IconButton icon="i-material-symbols-edit-outline" :label="$t('calendar.aria.edit_named', { name: booking.title })" @click="openBooking(booking, true)" />
              <IconButton icon="i-material-symbols-event-busy-outline" :label="$t('calendar.aria.cancel_named', { name: booking.title })" :loading="acting === bookingKey(booking)" @click="cancelOne(booking)" />
            </template>
          </template>
        </BookingList>

        <BookingList v-else-if="tab === 'requests'" :bookings="requests" :read="requestsRead" :failed="requestsFailed" :empty="$t('calendar.message.no_requests')" booker>
          <template #actions="{ booking }">
            <IconButton icon="i-material-symbols-check" :label="$t('calendar.aria.approve_named', { name: booking.title })" :loading="acting === bookingKey(booking)" @click="approve(booking)" />
            <IconButton icon="i-material-symbols-close" :label="$t('calendar.aria.refuse_named', { name: booking.title })" @click="refuse(booking)" />
          </template>
        </BookingList>

        <ResourceList v-else ref="resourceList" :kind="shownKind" />
      </ChoiceSidebar>

      <BookingDialog v-model:open="dialogOpen" :booking="dialogBooking" :draft="draft" :whole="dialogWhole" />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { addMinutes, startOfDay } from "date-fns";
import { useOverlay } from "@nuxt/ui/composables/useOverlay";
import { computed, inject, onBeforeUnmount, onMounted, ref, shallowRef, useTemplateRef, watch } from "vue";
import { navigateTo, useRouter } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ChoiceSidebar from "bmo-ui/app/components/ChoiceSidebar.vue";
import CreateButton from "bmo-ui/app/components/CreateButton.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { useIsland } from "bmo-ui/app/domain/island";
import { COUNT_BADGE, countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { TAB_ROW_LINE, TAB_ROW_UI } from "bmo-ui/app/domain/tabs";

import BookingDialog from "../../components/bookings/BookingDialog.vue";
import BookingList from "../../components/bookings/BookingList.vue";
import BookingMonth from "../../components/bookings/BookingMonth.vue";
import BookingRefuseDialog from "../../components/bookings/BookingRefuseDialog.vue";
import BookingTimeline from "../../components/bookings/BookingTimeline.vue";
import BookingWeek from "../../components/bookings/BookingWeek.vue";
import ResourceList from "../../components/bookings/ResourceList.vue";
import { BOOKING_ICON, bookingKey, kindsOf, placesOf } from "../../domain/bookings";
import { companyQuery } from "../../domain/companies";
import { WEEK_STARTS_ON, eachDay, toCalendarDate, toDate, todayDate } from "../../domain/dates";
import { refusalText } from "../../domain/store";
import { useCalendarStream } from "../../domain/stream";
import { useBookingChange } from "../../composables/useBookingChange";
import { BOOKING_VIEWS, useBookingScreen } from "../../composables/useBookingScreen";
import { useBookings } from "../../composables/useBookings";
import { SERVICE } from "../../composables/useCalendarApi";
import { useCompanies } from "../../composables/useCompanies";
import { useCompanyChoice } from "../../composables/useCompanyChoice";
import { usePeople } from "../../composables/usePeople";

import type { CalendarDate } from "@internationalized/date";
import type { TabsItem } from "@nuxt/ui";
import type { BookingDraft } from "../../components/bookings/BookingForm.vue";
import type { BookingTab, BookingView } from "../../composables/useBookingScreen";
import type { CalendarStreamEvent } from "../../domain/stream";
import type { Booking } from "../../../shared/types";

const VIEW_LABELS: Record<BookingView, string> = {
  day: "calendar.label.view_day",
  week: "calendar.label.view_week",
  month: "calendar.label.view_month"
};

/** The hour a new booking made from the create button starts at on a day other than today. */
const DEFAULT_HOUR = 9;
/** A burst of changes the stream tells of is gathered before what is shown is read again. */
const GATHER_MS = 400;

// The company the address names is followed first, so nothing of the organization realm is read before it.
const { entries: companyEntries, chosen: chosenCompany, listed: companiesListed, choose: chooseCompany } = useCompanyChoice();

const resourceListRef = useTemplateRef<InstanceType<typeof ResourceList>>("resourceList");

const { t } = useI18n();
const router = useRouter();
// The calendar address the screen was entered from, read once as it opens: the way back leaves to it however many
// steps the screen's own parts, views and days took since, rather than stepping back through them.
const enteredFrom = enteredFromCalendar(typeof window === "undefined" ? null : window.history.state?.back);
const overlay = useOverlay();
const island = useIsland(t);
const narrow = useNarrow();
// Whether a console draws its title bar over the screen, as the calendar screen reads it.
const consoleBar = inject<object | null>("bmo-console-title-bar", null) !== null;

const { tab, view, kind, company, date, range, title, shortTitle, prevDate, nextDate, pathFor, go } = useBookingScreen();
const { books, manages, loadCompanies } = useCompanies();
const { loadMe } = usePeople();
const {
  resources,
  resourcesRead,
  resourcesFailed,
  bookings,
  bookingsRead,
  bookingsFailed,
  mine,
  mineRead,
  mineFailed,
  requests,
  requestsRead,
  requestsFailed,
  keeps,
  loadSpan,
  loadResources,
  loadMine,
  loadRequests,
  refreshBookings,
  refreshAll,
  decide
} = useBookings();
const bookingChange = useBookingChange();

const pickerOpen = ref(false);
const dialogOpen = ref(false);
const dialogWhole = ref(false);
const refreshing = ref(false);
const acting = ref<string | null>(null);
const draft = shallowRef<BookingDraft | null>(null);
const dialogBooking = shallowRef<Booking | null>(null);

const kinds = computed(() => kindsOf(resources.value));
const shownKind = computed(() => (kind.value && kinds.value.includes(kind.value) ? kind.value : (kinds.value[0] ?? null)));
const groups = computed(() => (shownKind.value === null ? [] : placesOf(resources.value, shownKind.value)));
const kindResources = computed(() => groups.value.flatMap((group) => group.resources));
const days = computed(() => eachDay(range.value));

const pickedDay = computed({
  get: () => date.value,
  set(value: CalendarDate | null | undefined) {
    if (!value) return;

    pickerOpen.value = false;
    void navigateTo(pathFor({ date: value }));
  }
});

// The requests stand as a part of the screen for a keeper, and for anyone a request waits on.
const tabItems = computed<TabsItem[]>(() => {
  const items: TabsItem[] = [
    { label: t("calendar.label.bookings_all"), value: "all" },
    { label: t("calendar.label.bookings_mine"), value: "mine" }
  ];

  if (keeps.value || requests.value.length || tab.value === "requests") {
    items.push({
      label: t("calendar.label.requests"),
      value: "requests",
      ...(requests.value.length ? { badge: { ...COUNT_BADGE, color: "primary" as const, label: countText(requests.value.length) } } : {})
    });
  }

  items.push({ label: t("calendar.label.resources"), value: "resources" });

  return items;
});

const viewItems = computed(() => BOOKING_VIEWS.map((one) => ({ label: t(VIEW_LABELS[one]), value: one })));

const kindItems = computed(() => kinds.value.map((one) => ({ label: one, value: one })));

watch(range, (span) => void loadSpan(span), { immediate: true });

// The way back leaves the screen: to the calendar address it was entered from, the company chosen here carried
// along, else to the calendar screen.
function back(): void {
  if (enteredFrom !== null) {
    const left = router.resolve(enteredFrom);

    void navigateTo({ path: left.path, query: { ...left.query, ...companyQuery(company.value) } });
  } else void navigateTo({ path: "/calendar", query: companyQuery(company.value) });
}

// A calendar address of this bundle the browser names as the entry before this screen, none for any other.
function enteredFromCalendar(previous: unknown): string | null {
  if (typeof previous !== "string") return null;

  const path = router.resolve(previous).path;

  return path.startsWith("/calendar") && !path.startsWith("/calendar/bookings") ? previous : null;
}

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await refreshAll();
  } finally {
    refreshing.value = false;
  }
}

function bookOn(offered: BookingDraft): void {
  if (!books.value) return;

  dialogBooking.value = null;
  dialogWhole.value = false;
  draft.value = offered;
  dialogOpen.value = true;
}

/** A new booking from the create button: on the first resource shown, the next hour of today or the
 *  morning of another day. */
function bookAnew(): void {
  const resource = kindResources.value[0] ?? resources.value[0];

  if (!resource) return;

  const day = toDate(date.value);
  const isToday = date.value.compare(todayDate()) === 0;
  const hour = isToday ? Math.min(23, new Date().getHours() + 1) : DEFAULT_HOUR;
  const start = addMinutes(startOfDay(day), hour * 60);

  bookOn({ resourceId: resource.id, start, end: addMinutes(start, 60) });
}

/** A booking opened in its form: from the views one occurrence, from the reader's own list the whole of it. */
function openBooking(booking: Booking, whole = false): void {
  draft.value = null;
  dialogBooking.value = booking;
  dialogWhole.value = whole;
  dialogOpen.value = true;
}

async function act(booking: Booking, send: () => Promise<unknown>): Promise<void> {
  acting.value = bookingKey(booking);

  try {
    await send();
  } catch (error) {
    island.say({ tone: "error", title: refusalText(t, error) });
  } finally {
    acting.value = null;
  }
}

// The reader's own list cancels the whole booking.
function cancelOne(booking: Booking): void {
  void act(booking, () => bookingChange.cancel(booking, true));
}

function approve(booking: Booking): void {
  void act(booking, () => decide(booking, true));
}

async function refuse(booking: Booking): Promise<void> {
  const reason = (await overlay.create(BookingRefuseDialog, { props: { booking: booking.title }, destroyOnClose: true }).open()) as string | null | undefined;

  if (reason === null || reason === undefined) return;

  await act(booking, () => decide(booking, false, reason));
}

let gathering: ReturnType<typeof setTimeout> | null = null;

function gatherRefresh(): void {
  if (gathering !== null) clearTimeout(gathering);

  gathering = setTimeout(() => {
    gathering = null;
    void refreshBookings();
  }, GATHER_MS);
}

function ringDecision(data: Extract<CalendarStreamEvent, { kind: "booking.decided" }>["data"]): void {
  const approved = data.state === "booked";

  island.say({
    key: `calendar-booking-${data.bookingId}`,
    tone: approved ? "success" : "warning",
    icon: BOOKING_ICON,
    title: t(approved ? "calendar.message.booking_approved" : "calendar.message.booking_refused"),
    description: data.title
  });
}

// The span in view is read as the address names it (`range`); the rest as the screen opens.
onMounted(() => {
  void loadCompanies();
  void loadMe();
  void loadResources();
  void loadMine();
  void loadRequests();
});

useCalendarStream(
  SERVICE,
  (event) => {
    if (event.kind === "resource.changed") void loadResources();
    if (event.kind === "booking.decided") ringDecision(event.data);
    if (event.kind === "booking.changed" || event.kind === "booking.decided" || event.kind === "resource.changed") gatherRefresh();
  },
  () => void refreshAll()
);

onBeforeUnmount(() => {
  if (gathering !== null) clearTimeout(gathering);
});
</script>
