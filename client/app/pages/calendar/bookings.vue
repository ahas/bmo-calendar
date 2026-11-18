<template>
  <!-- The booking screen (docs/design.md, 「The console」), reached from the calendar screen's sidebar and back
       again: the company's resources of one kind by day, week or month with their bookings, the reader's own
       bookings from today on, the requests waiting on what the reader keeps, and the resources themselves.
       The way back, the parts of the screen and the toolbar acting on the part shown stand in the screen's
       own rows; the one create button makes a new booking, and a resource while the resources stand. On a
       phone the row keeps one line: the title takes its short form and the kind is cut to its room. Where it
       stands is the address. -->
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
            <UDropdownMenu :items="viewItems" :content="{ align: 'end' }">
              <UButton color="neutral" variant="ghost" size="sm" :label="$t(VIEW_LABELS[view])" trailing-icon="i-material-symbols-arrow-drop-down" :aria-label="$t('calendar.aria.view_named', { view: $t(VIEW_LABELS[view]) })" />
            </UDropdownMenu>
          </div>

          <!-- On a phone the kind is cut to its room, its whole name in the shared tooltip of a cut line. -->
          <div v-if="kinds.length" class="min-w-0 shrink!">
            <UDropdownMenu :items="kindItems" :content="{ align: 'end' }">
              <UButton
                color="neutral"
                variant="ghost"
                size="sm"
                :label="shownKind ?? ''"
                trailing-icon="i-material-symbols-arrow-drop-down"
                class="min-w-0 max-sm:max-w-20"
                :aria-label="$t('calendar.aria.kind_named', { kind: shownKind ?? '' })"
              />
            </UDropdownMenu>
          </div>
        </template>

        <!-- The company is chosen beside the views where there is room; on a phone, on the calendar's sidebar. -->
        <div v-if="administrator && !(narrow && tab === 'all')" :class="tab !== 'all' && 'min-w-0 flex-1'">
          <CompanySelect size="sm" />
        </div>
        <div v-else-if="tab !== 'all'" class="min-w-0 flex-1" />

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
import CompanySelect from "../../components/bookings/CompanySelect.vue";
import ResourceList from "../../components/bookings/ResourceList.vue";
import { BOOKING_ICON, bookingKey, kindsOf, placesOf } from "../../domain/bookings";
import { WEEK_STARTS_ON, eachDay, toCalendarDate, toDate, todayDate } from "../../domain/dates";
import { refusalText } from "../../domain/store";
import { useCalendarStream } from "../../domain/stream";
import { useBookingChange } from "../../composables/useBookingChange";
import { useBookingScreen } from "../../composables/useBookingScreen";
import { useBookings } from "../../composables/useBookings";
import { SERVICE } from "../../composables/useCalendarApi";
import { useCompanies } from "../../composables/useCompanies";
import { usePeople } from "../../composables/usePeople";

import type { CalendarDate } from "@internationalized/date";
import type { DropdownMenuItem, TabsItem } from "@nuxt/ui";
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

const resourceListRef = useTemplateRef<InstanceType<typeof ResourceList>>("resourceList");

const { t } = useI18n();
const router = useRouter();
const overlay = useOverlay();
const island = useIsland(t);
const narrow = useNarrow();
// Whether a console draws its title bar over the screen, as the calendar screen reads it.
const consoleBar = inject<object | null>("bmo-console-title-bar", null) !== null;

const { tab, view, kind, date, range, title, shortTitle, prevDate, nextDate, pathFor, go } = useBookingScreen();
const { administrator, books, manages, loadCompanies } = useCompanies();
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

const viewItems = computed<DropdownMenuItem[]>(() =>
  (["day", "week", "month"] as const).map((one) => ({
    label: t(VIEW_LABELS[one]),
    type: "checkbox" as const,
    checked: view.value === one,
    onSelect: () => go({ view: one })
  }))
);

const kindItems = computed<DropdownMenuItem[]>(() =>
  kinds.value.map((one) => ({
    label: one,
    type: "checkbox" as const,
    checked: shownKind.value === one,
    onSelect: () => go({ kind: one })
  }))
);

watch(range, (span) => void loadSpan(span), { immediate: true });

function back(): void {
  if (typeof window !== "undefined" && window.history.state?.back) router.back();
  else void navigateTo("/calendar");
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
