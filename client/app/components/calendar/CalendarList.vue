<template>
  <!-- What stands at the trailing edge of a realm's heading, or of the company row chosen under it: the realm's
       settings where the reader writes in it, else the lock that marks it read-only. -->
  <DefineRealmMark v-slot="{ realm }">
    <CalendarSettingsPopover v-if="writable(realm)" :realm="realm" />
    <HintTip v-else :text="$t('calendar.label.read_only')" class="flex size-8 shrink-0 items-center justify-center" role="img" :aria-label="$t('calendar.label.read_only')">
      <UIcon :name="READ_ONLY_ICON" class="size-4 text-muted" />
    </HintTip>
  </DefineRealmMark>

  <!-- One realm's calendars and sources, under its heading or under the company row chosen. -->
  <DefineLayers v-slot="{ group }">
    <template v-if="calendarsRead">
      <div v-for="calendar in group.calendars" :key="calendar.id" class="flex items-center gap-2 rounded-md ps-2 transition-colors pointer-coarse:min-h-11" :class="writable(group.realm) && 'hover:bg-elevated'">
        <UCheckbox
          :model-value="!calendar.hidden"
          :color="calendar.color"
          :label="calendar.name"
          :disabled="!writable(group.realm)"
          class="min-w-0 flex-1 py-1.5"
          :ui="{ label: 'truncate pointer-coarse:py-3' }"
          @update:model-value="setHidden(calendar, !$event)"
        />
        <UIcon v-if="sendingLayers.includes(`calendar:${group.realm}:${calendar.id}`)" :name="SENDING_ICON" class="size-4 shrink-0 animate-spin text-muted motion-reduce:animate-none" />
        <CalendarSettingsPopover v-if="writable(group.realm)" :realm="group.realm" :calendar="calendar" />
      </div>

      <div v-for="source in group.sources" :key="`source:${source.id}`" class="flex items-center gap-2 rounded-md ps-2 transition-colors pointer-coarse:min-h-11" :class="writable(group.realm) && 'hover:bg-elevated'">
        <UCheckbox
          :model-value="!source.hidden"
          :color="source.color"
          :label="source.name"
          :disabled="!writable(group.realm)"
          class="min-w-0 flex-1 py-1.5"
          :ui="{ label: 'truncate pointer-coarse:py-3' }"
          @update:model-value="setSourceHidden(group.realm, source, !$event)"
        />
        <span v-if="sendingLayers.includes(`source:${group.realm}:${source.id}`)" class="flex size-8 shrink-0 items-center justify-center">
          <UIcon :name="SENDING_ICON" class="size-4 animate-spin text-muted motion-reduce:animate-none" />
        </span>
        <HintTip v-else :text="$t('calendar.label.received')" class="flex size-8 shrink-0 items-center justify-center" role="img" :aria-label="$t('calendar.label.received')">
          <UIcon :name="RECEIVED_ICON" class="size-4 text-muted" />
        </HintTip>
      </div>
    </template>
    <div v-else class="flex flex-col gap-1 ps-2" role="status" :aria-label="$t('calendar.status.loading')">
      <USkeleton class="my-2 h-4 w-32 rounded-sm" />
    </div>
  </DefineLayers>

  <!-- The calendars, the account's own and the organization's apart: each with its colour, shown or hidden on the grid for
       this account, and its settings a press away; a mark spins beside one while its change is sent. Under each
       realm's calendars stand the sources other units push schedules under, one layer each, shown or hidden
       the same way and marked as received. The organization's are the company read. For the administrator the
       companies he reads stand under the organization's heading, searched as it is typed, each a row choosing it, and
       the calendars and sources of the one chosen stand under its row; until the companies are read their rows stand
       as a skeleton. Another company's calendars and sources he reads alone, its row marked read-only by a lock and no
       calendar offered to add, set or hide, nor a source to hide. At the foot stands the way to the booking screen, on
       the same company. -->
  <div class="flex flex-col gap-4">
    <section v-for="group in groups" :key="group.realm" class="flex flex-col gap-1">
      <div class="flex h-8 items-center justify-between gap-2 ps-2">
        <!-- Short of room the heading's words give way, read whole in the shared tooltip of a cut line, and what
             stands at its trailing edge stays there. -->
        <h2 class="flex min-w-0 items-center gap-1.5 text-sm font-semibold text-highlighted">
          <UIcon :name="REALM_ICON[group.realm]" class="size-4 shrink-0 text-muted" />
          <span class="truncate" data-layer-words>{{ $t(group.realm === "mine" ? "calendar.label.mine" : "calendar.label.organization") }}</span>
        </h2>
        <RealmMark v-if="!chosenRow(group.realm)" :realm="group.realm" />
      </div>

      <template v-if="group.realm === 'organization' && props.listed">
        <UInput
          v-model="words"
          :icon="SEARCH_ICON"
          :placeholder="$t('common.placeholder.search')"
          :aria-label="$t('common.aria.search_named', { name: $t('calendar.label.organizations') })"
          class="mb-1"
          data-company-search
        />
        <div v-if="props.companies === null" class="flex flex-col gap-1 ps-2" role="status" :aria-label="$t('calendar.status.loading')" data-company-skeleton>
          <USkeleton v-for="index in COMPANY_SKELETON_ROWS" :key="index" class="my-2 h-4 rounded-sm pointer-coarse:my-3.5" :class="index % 2 === 0 ? 'w-1/2' : 'w-2/3'" />
        </div>
        <p v-else-if="shownCompanies.length === 0" class="ps-2 text-sm text-muted">{{ $t("calendar.status.no_organization_match") }}</p>
        <template v-for="entry in shownCompanies" v-else :key="entry.value">
          <div class="flex items-center gap-2">
            <UButton
              color="neutral"
              :variant="entry.value === props.chosen ? 'soft' : 'ghost'"
              class="min-w-0 flex-1 justify-start px-2 pointer-coarse:min-h-11"
              :aria-current="entry.value === props.chosen ? 'page' : undefined"
              :data-company-row="entry.value"
              @click="emit('choose', String(entry.value))"
            >
              <SearchText class="truncate text-sm font-medium text-highlighted" :text="entry.label" :query="words" data-layer-words />
            </UButton>
            <RealmMark v-if="entry.value === props.chosen" :realm="group.realm" />
          </div>
          <div v-if="entry.value === props.chosen" class="flex flex-col gap-1 ps-4">
            <UseLayers :group="group" />
          </div>
        </template>
      </template>
      <UseLayers v-else :group="group" />
    </section>

    <p v-if="calendarsFailed" role="alert" class="badge-ink flex items-center gap-2 ps-2 text-sm text-error">
      {{ $t("calendar.error.calendars_failed") }}
    </p>

    <UButton :to="{ path: BOOKINGS_PATH, query: companyQuery(company) }" :icon="BOOKING_ICON" :label="$t('calendar.label.bookings')" color="neutral" variant="ghost" block class="justify-start pointer-coarse:min-h-11" />
  </div>
</template>

<script setup lang="ts">
import { createReusableTemplate } from "@vueuse/core";
import { computed, ref } from "vue";
import SearchText from "bmo-ui/app/components/SearchText.vue";
import { countLocale } from "bmo-ui/app/domain/numbers";
import { isSideHeading, sideEntriesMatching } from "bmo-ui/app/domain/side-parts";

import CalendarSettingsPopover from "./CalendarSettingsPopover.vue";
import { BOOKING_ICON } from "../../domain/bookings";
import { companyQuery } from "../../domain/companies";
import { READ_ONLY_ICON, RECEIVED_ICON, REALM_ICON, SENDING_ICON } from "../../domain/colors";
import { BOOKINGS_PATH } from "../../composables/useBookingScreen";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";

import type { SideChoice } from "bmo-ui/app/domain/side-parts";
import type { Realm } from "../../../shared/types";

interface Props {
  // The companies the reader chooses from, as the screen's choice of company offers them; null until they are read.
  companies?: readonly SideChoice[] | null;
  // The company chosen.
  chosen?: string | null;
  // Whether the companies stand under the organization's heading: while they are unread, and then for the
  // administrator alone.
  listed?: boolean;
}

const props = withDefaults(defineProps<Props>(), { companies: null, chosen: null, listed: false });
const emit = defineEmits<{ choose: [company: string] }>();

const SEARCH_ICON = "i-material-symbols-search";
// How many rows the companies' skeleton stands: a few, never taller than a short list.
const COMPANY_SKELETON_ROWS = 3;

const { groups, calendarsRead, calendarsFailed, setHidden, setSourceHidden, sendingLayers, writable } = useCalendarEvents();
const { company } = useCalendar();

const [DefineRealmMark, RealmMark] = createReusableTemplate<{ realm: Realm }>();
const [DefineLayers, UseLayers] = createReusableTemplate<{ group: (typeof groups.value)[number] }>();

const words = ref("");
const shownCompanies = computed(() => sideEntriesMatching(props.companies ?? [], words.value, countLocale()).filter((entry): entry is SideChoice => !isSideHeading(entry)));

/** Whether the realm's mark stands on the company row chosen under its heading rather than on the heading. */
function chosenRow(realm: Realm): boolean {
  return realm === "organization" && props.listed;
}
</script>
