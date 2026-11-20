<template>
  <!-- The calendars, the account's own and the organization's apart: each with its colour, shown or hidden on the grid for
       this account, and its settings a press away; a mark spins beside one while its change is sent. Under each
       realm's calendars stand the sources other units push schedules under, one layer each, shown or hidden
       the same way and marked as received. The organization's are the company read, which the administrator
       chooses in the screen's list of companies; another company's calendars and sources he reads alone, the
       heading marked read-only by a lock and no calendar offered to add, set or hide, nor a source to hide. At the foot
       stands the way to the booking screen, on the same company. -->
  <div class="flex flex-col gap-4">
    <section v-for="group in groups" :key="group.realm" class="flex flex-col gap-1">
      <div class="flex h-8 items-center justify-between gap-2 ps-2">
        <!-- Short of room the heading's words give way, read whole in the shared tooltip of a cut line, and what
             stands at its trailing edge stays there. -->
        <h2 class="flex min-w-0 items-center gap-1.5 text-sm font-semibold text-highlighted">
          <UIcon :name="REALM_ICON[group.realm]" class="size-4 shrink-0 text-muted" />
          <span class="truncate">{{ $t(group.realm === "mine" ? "calendar.label.mine" : "calendar.label.organization") }}</span>
        </h2>
        <CalendarSettingsPopover v-if="writable(group.realm)" :realm="group.realm" />
        <HintTip v-else :text="$t('calendar.label.read_only')" class="flex size-8 shrink-0 items-center justify-center" role="img" :aria-label="$t('calendar.label.read_only')">
          <UIcon :name="READ_ONLY_ICON" class="size-4 text-muted" />
        </HintTip>
      </div>

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
    </section>

    <p v-if="calendarsFailed" role="alert" class="badge-ink flex items-center gap-2 ps-2 text-sm text-error">
      {{ $t("calendar.error.calendars_failed") }}
    </p>

    <UButton :to="{ path: BOOKINGS_PATH, query: companyQuery(company) }" :icon="BOOKING_ICON" :label="$t('calendar.label.bookings')" color="neutral" variant="ghost" block class="justify-start pointer-coarse:min-h-11" />
  </div>
</template>

<script setup lang="ts">
import CalendarSettingsPopover from "./CalendarSettingsPopover.vue";
import { BOOKING_ICON } from "../../domain/bookings";
import { companyQuery } from "../../domain/companies";
import { READ_ONLY_ICON, RECEIVED_ICON, REALM_ICON, SENDING_ICON } from "../../domain/colors";
import { BOOKINGS_PATH } from "../../composables/useBookingScreen";
import { useCalendar } from "../../composables/useCalendar";
import { useCalendarEvents } from "../../composables/useCalendarEvents";

const { groups, calendarsRead, calendarsFailed, setHidden, setSourceHidden, sendingLayers, writable } = useCalendarEvents();
const { company } = useCalendar();
</script>
