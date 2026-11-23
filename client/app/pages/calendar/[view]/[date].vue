<template>
  <!-- The calendar screen (docs/design.md, 「The console」): the calendars of both realms beside one grid. Where
       there is room the calendars stand beside the grid behind a splitter; on a phone the grid is the body,
       the calendars rise from the bottom edge, the way to today stands in a bar at its foot and a new event
       floats over that bar. For the administrator the calendars' list holds the companies he chooses from. The
       view, the day and the company are the address. -->
  <UDashboardPanel id="calendar" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0 overflow-y-hidden [&>*]:max-w-none' }">
    <!-- Beside the title stands only the screen's name and mark; what acts on the screen stands in its toolbar. -->
    <template v-if="!consoleBar" #header>
      <UDashboardNavbar :title="$t('calendar.title')" icon="i-material-symbols-calendar-month-outline" />
    </template>

    <template #body>
      <ToolBar :label="$t('calendar.aria.toolbar')">
        <div v-if="narrow">
          <IconButton size="sm" icon="i-material-symbols-calendar-view-month-outline" :label="$t('calendar.action.calendars')" @click="isSidebarOpen = true" />
        </div>

        <div>
          <IconButton size="sm" icon="i-material-symbols-chevron-left" :label="$t('calendar.action.previous')" :kbds="['arrowleft']" :to="pathFor(prevDate)" />
          <IconButton size="sm" icon="i-material-symbols-chevron-right" :label="$t('calendar.action.next')" :kbds="['arrowright']" :to="pathFor(nextDate)" />
        </div>

        <div class="min-w-0 flex-1">
          <UPopover v-model:open="pickerOpen" :content="{ align: 'start' }">
            <UButton color="neutral" variant="ghost" size="sm" class="max-w-full py-1 text-lg/6 font-semibold text-highlighted" :aria-label="$t('calendar.aria.pick_day_named', { title: `${title.months} ${title.year}`.trim() })">
              <!-- Short of room the year gives way, and the months stand whole: they are what the title is read
                   for, and the toolbar flows sideways before it cuts them. -->
              <span class="shrink-0 whitespace-nowrap">{{ title.months }}</span>
              <span v-if="title.year" class="shrink-[100] truncate font-normal text-muted">{{ title.year }}</span>
            </UButton>

            <template #content>
              <DateCalendar v-model="pickedDay" :week-starts-on="WEEK_STARTS_ON" class="p-2" />
            </template>
          </UPopover>
        </div>

        <div v-if="!narrow">
          <HintTip :text="$t('calendar.action.today')" :kbds="['t']" press>
            <UButton color="neutral" variant="ghost" size="sm" :label="$t('calendar.action.today')" :to="pathFor(todayDate())" />
          </HintTip>
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
            @update:model-value="(one: CalendarView) => void navigateTo(pathFor(date, one))"
          >
            <!-- The key stands last on every row, past the chosen row's check, so the keys line up down the list. -->
            <template #item-trailing="{ item }">
              <UKbd :value="VIEW_KEYS[(item as { value: CalendarView }).value]" class="order-last" />
            </template>
          </ChoiceSelect>
        </div>

        <!-- On a phone the mark stands alone, so the row keeps its steps and its title; its words stay in its
             tooltip and for a screen reader. -->
        <div v-if="!online">
          <HintTip :text="narrow ? offlineLabel : $t('calendar.label.offline_hint')">
            <UBadge icon="i-material-symbols-cloud-off-outline" color="warning" variant="soft">
              <span :class="narrow && 'sr-only'">{{ offlineLabel }}</span>
            </UBadge>
          </HintTip>
        </div>

        <div>
          <IconButton size="sm" icon="i-material-symbols-refresh" :label="$t('calendar.action.refresh')" :loading="refreshing" @click="refresh" />
        </div>

        <CalendarNewEvent />
      </ToolBar>

      <!-- The span a step lands on, said once it has landed. -->
      <p class="sr-only" aria-live="polite" aria-atomic="true">{{ announced }}</p>

      <!-- For the administrator the companies stand in the calendars' list, under the organization's heading, so
           the grid shares the width with one side region alone. -->
      <div class="flex min-h-0 min-w-0 flex-1 flex-col" data-calendar-body>
        <p v-if="failed" role="alert" class="badge-ink flex shrink-0 items-center gap-3 border-b border-default px-4 py-2 text-sm text-error">
          {{ $t("calendar.error.events_failed") }}
        </p>

        <template v-if="narrow">
          <div class="flex min-h-0 flex-1 flex-col">
            <CalendarMonthView v-if="view === 'month'" />
            <CalendarYearView v-else-if="view === 'year'" />
            <CalendarWeekView v-else :key="view" />
          </div>

          <ToolBar :label="$t('calendar.aria.foot_bar')" data-screen-foot class="box-content border-t border-default pb-[var(--bmo-safe-bottom)]">
            <div>
              <UButton color="neutral" variant="ghost" size="sm" :label="$t('calendar.action.today')" :to="pathFor(todayDate())" />
            </div>
          </ToolBar>

          <UDrawer v-model:open="isSidebarOpen" :title="$t('calendar.action.calendars')" :ui="SHEET">
            <template #body>
              <UScrollArea ref="sheetBody" class="min-h-0 flex-1">
                <div class="flex flex-col gap-4 p-4">
                  <CalendarList :companies="companyEntries" :chosen="chosenCompany" :listed="companiesListed" @choose="choose" />
                </div>
              </UScrollArea>
            </template>
          </UDrawer>
        </template>

        <USplitter v-else id="calendar-splitter" auto-save-id="calendar-splitter" :items="SIDES" :ui="{ handle: SPLIT_HANDLE }" class="min-h-0 flex-1">
          <!-- The list scrolls on its own over the mini calendar, which stands at the sidebar's foot whatever the list
               scrolls. -->
          <template #side>
            <div class="flex min-h-0 min-w-0 flex-1 flex-col">
              <UScrollArea class="min-h-0 min-w-0 flex-1" data-calendar-side-list>
                <div class="flex flex-col gap-4 p-4">
                  <CalendarList :companies="companyEntries" :chosen="chosenCompany" :listed="companiesListed" @choose="choose" />
                </div>
              </UScrollArea>

              <div class="shrink-0 border-t border-default p-4" data-calendar-mini>
                <CalendarMini />
              </div>
            </div>
          </template>

          <template #grid>
            <div class="flex min-h-0 min-w-0 flex-1 flex-col">
              <CalendarMonthView v-if="view === 'month'" />
              <CalendarYearView v-else-if="view === 'year'" />
              <CalendarWeekView v-else :key="view" />
            </div>
          </template>
        </USplitter>
      </div>
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { defineShortcuts } from "@nuxt/ui/composables/defineShortcuts";
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue";
import { navigateTo } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { useIsland } from "bmo-ui/app/domain/island";
import { countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { FULL_SHEET, SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";

import CalendarList from "../../../components/calendar/CalendarList.vue";
import CalendarMini from "../../../components/calendar/CalendarMini.vue";
import CalendarMonthView from "../../../components/calendar/CalendarMonthView.vue";
import CalendarNewEvent from "../../../components/calendar/CalendarNewEvent.vue";
import CalendarWeekView from "../../../components/calendar/CalendarWeekView.vue";
import CalendarYearView from "../../../components/calendar/CalendarYearView.vue";
import { rangeWords } from "../../../domain/a11y";
import { BOOKING_ICON } from "../../../domain/bookings";
import { WEEK_STARTS_ON, formatFullDate, fromLocalISO, rangeFor, toDate, todayDate } from "../../../domain/dates";
import { CALENDAR_CHANGED, useCalendarStream } from "../../../domain/stream";
import { SERVICE } from "../../../composables/useCalendarApi";
import { useCalendarEvents } from "../../../composables/useCalendarEvents";
import { useCalendar } from "../../../composables/useCalendar";
import { useCompanies } from "../../../composables/useCompanies";
import { useCompanyChoice } from "../../../composables/useCompanyChoice";
import { useEventDraft } from "../../../composables/useEventDraft";
import { useEventEditor } from "../../../composables/useEventEditor";

import type { CalendarDate } from "@internationalized/date";
import type { SplitterItem } from "@nuxt/ui";
import type { CalendarStreamEvent } from "../../../domain/stream";
import type { CalendarView } from "../../../../shared/types";

// One screen for every view: a change of day keeps the view standing rather than building it again.
definePageMeta({ key: (route) => `calendar-${String(route.params.view)}` });

// The calendars' region is sized in pixels, room for a heading and its mark, so the grid keeps the rest of the width:
// beside the console's navigation and right panel at 1280 its day columns still hold a title of four syllables.
const SIDES: SplitterItem[] = [
  { slot: "side", minSize: 208, defaultSize: 224, maxSize: 400, sizeUnit: "px" },
  { slot: "grid", minSize: 50, defaultSize: 80 }
];

const VIEW_LABELS: Record<CalendarView, string> = {
  day: "calendar.label.view_day",
  week: "calendar.label.view_week",
  month: "calendar.label.view_month",
  year: "calendar.label.view_year"
};

const VIEW_KEYS: Record<CalendarView, string> = { day: "d", week: "w", month: "m", year: "y" };

// The sheet takes the focus as it opens, so a reader is in it and kept in it; it is the dialog that takes
// it rather than a control in it, so nothing rings and no keyboard rises.
const SHEET = { ...FULL_SHEET, content: `${FULL_SHEET.content} focus-visible:outline-none` };

const sheetBodyRef = useTemplateRef<{ $el: HTMLElement }>("sheetBody");

// The company the address names is followed first, so nothing of the organization realm is read before it.
const { entries: companyEntries, chosen: chosenCompany, listed: companiesListed, choose: chooseCompany } = useCompanyChoice();
const { t, locale } = useI18n();
const island = useIsland(t);
const narrow = useNarrow();
// Whether a console draws its title bar over the screen, under the key bmo-ui's bundle contract names; where
// none does, the screen draws its own. Nothing of the screen's is laid into that bar.
const consoleBar = inject<object | null>("bmo-console-title-bar", null) !== null;

const { view, date, range, title, visibleMonth, prevDate, nextDate, pathFor, go, isSidebarOpen } = useCalendar();
const { online, queue, loadCalendars, loadResourceNames, loadSpan, showSpans, refreshVisible, failedSpans, readEvent, allEvents } = useCalendarEvents();
const { loadCompanies } = useCompanies();
const { draft } = useEventDraft();
const { editingKey } = useEventEditor();

const offlineLabel = computed(() => (queue.value.length ? t("calendar.label.offline_waiting", { n: countText(queue.value.length) }) : t("calendar.label.offline")));
const failed = computed(() => failedSpans.value.length > 0);

const viewItems = computed(() => (["day", "week", "month", "year"] as const).map((one) => ({ label: t(VIEW_LABELS[one]), value: one })));

// Nothing is said as the screen opens; a step to another span is said once it has landed.
const announced = ref("");
const shownWords = computed(() => rangeWords(view.value, range.value, toDate(view.value === "month" ? (visibleMonth.value ?? date.value) : date.value), locale.value));

watch(shownWords, (words) => {
  announced.value = words;
});

const pickerOpen = ref(false);
const pickedDay = computed({
  get: () => date.value,
  set(value: CalendarDate | null | undefined) {
    if (!value) return;

    pickerOpen.value = false;
    void navigateTo(pathFor(value));
  }
});

// The day and week views read their span, and warm the spans a step away; the month view reads its own
// spans as it scrolls, and the year draws no events.
watch(
  [view, range],
  () => {
    if (view.value === "month") return;

    if (view.value === "year") {
      showSpans([]);
      return;
    }

    showSpans([range.value]);
    void loadSpan(range.value);
    void loadSpan(rangeFor(view.value, prevDate.value));
    void loadSpan(rangeFor(view.value, nextDate.value));
  },
  { immediate: true }
);

onMounted(() => {
  void loadCalendars();
  void loadResourceNames();
  void loadCompanies();
});

const refreshing = ref(false);

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await Promise.all([loadCalendars(), loadResourceNames(), refreshVisible()]);
  } finally {
    refreshing.value = false;
  }
}

// A burst of changes the stream tells of is gathered before the spans in view are read again.
const GATHER_MS = 400;
let gathering: ReturnType<typeof setTimeout> | null = null;

function gatherRefresh(calendars: boolean): void {
  if (calendars) void loadCalendars();
  if (gathering !== null) clearTimeout(gathering);

  gathering = setTimeout(() => {
    gathering = null;
    void refreshVisible();
  }, GATHER_MS);
}

async function ring(data: Extract<CalendarStreamEvent, { kind: "alarm.told" }>["data"]): Promise<void> {
  let description: string | null = null;

  try {
    const event = await readEvent(data.realm, data.eventId, data.company);

    description = `${event.title} · ${formatFullDate(fromLocalISO(data.occurrence ?? event.start))}`;
  } catch {
    // The event could not be read; the notice still says an alarm came.
  }

  island.say({ key: `calendar-alarm-${data.eventId}-${data.occurrence}`, tone: "info", icon: "i-material-symbols-alarm-outline", title: t("calendar.notice.alarm.told.title"), description });
}

// A keeper's decision on one of the reader's bookings, said while the reader stands on the screen; the
// console says it to a reader elsewhere.
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

useCalendarStream(
  SERVICE,
  (event) => {
    if (event.kind === "alarm.told") {
      void ring(event.data);
      return;
    }

    if (event.kind === "booking.decided") ringDecision(event.data);
    if (event.kind === "resource.changed") void loadResourceNames();

    gatherRefresh(event.kind === CALENDAR_CHANGED || event.kind === "source.changed");
  },
  () => {
    void loadResourceNames();
    gatherRefresh(true);
  }
);

onBeforeUnmount(() => {
  if (gathering !== null) clearTimeout(gathering);
});

// A key that moves the screen takes the grid from under a form, so none acts while one is open.
function unlessEditing(handler: () => void) {
  return () => {
    if (!draft.value && !editingKey.value) handler();
  };
}

defineShortcuts({
  t: unlessEditing(() => void navigateTo(pathFor(todayDate()))),
  d: unlessEditing(() => void navigateTo(pathFor(date.value, "day"))),
  w: unlessEditing(() => void navigateTo(pathFor(date.value, "week"))),
  m: unlessEditing(() => void navigateTo(pathFor(date.value, "month"))),
  y: unlessEditing(() => void navigateTo(pathFor(date.value, "year"))),
  arrowleft: unlessEditing(() => go("left")),
  arrowright: unlessEditing(() => go("right"))
});

// The drawer's sheet holds no focus of its own as it opens (the drawer library sets that aside), so it is
// handed the focus once it stands; closing hands it back to what opened it.
watch(isSidebarOpen, async (opened) => {
  if (!opened) return;

  await nextTick();
  await new Promise((resolve) => requestAnimationFrame(resolve));
  sheetBodyRef.value?.$el.closest<HTMLElement>('[role="dialog"]')?.focus();
});

// A company chosen in the calendars' drawer puts the drawer away first, so the choice is the one step of the history.
function choose(value: string): void {
  isSidebarOpen.value = false;
  chooseCompany(value);
}

// The calendars' drawer steps aside once what it chose is on its way.
watch([() => date.value.toString(), () => !!draft.value], () => {
  if (narrow.value) isSidebarOpen.value = false;
});
</script>
