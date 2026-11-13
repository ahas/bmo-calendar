<template>
  <!-- The calendars, the account's own and the organization's apart: each with its colour, shown or hidden on the grid for
       this account, and its settings a press away; a mark spins beside one while its change is sent. Under each
       realm's calendars stand the sources other units push schedules under, one layer each, shown or hidden
       the same way and marked as received. -->
  <div class="flex flex-col gap-4">
    <section v-for="group in groups" :key="group.realm" class="flex flex-col gap-1">
      <div class="flex h-8 items-center justify-between gap-2 ps-2">
        <h2 class="flex items-center gap-1.5 text-sm font-semibold text-highlighted">
          <UIcon :name="REALM_ICON[group.realm]" class="size-4 text-muted" />
          {{ $t(group.realm === "mine" ? "calendar.label.mine" : "calendar.label.organization") }}
        </h2>
        <CalendarSettingsPopover :realm="group.realm" />
      </div>

      <template v-if="calendarsRead">
        <div v-for="calendar in group.calendars" :key="calendar.id" class="flex items-center gap-2 rounded-md ps-2 transition-colors hover:bg-elevated pointer-coarse:min-h-11">
          <UCheckbox
            :model-value="!calendar.hidden"
            :color="calendar.color"
            :label="calendar.name"
            class="min-w-0 flex-1 py-1.5"
            :ui="{ label: 'truncate pointer-coarse:py-3' }"
            @update:model-value="setHidden(calendar, !$event)"
          />
          <UIcon v-if="sendingLayers.includes(`calendar:${group.realm}:${calendar.id}`)" :name="SENDING_ICON" class="size-4 shrink-0 animate-spin text-muted motion-reduce:animate-none" />
          <CalendarSettingsPopover :realm="group.realm" :calendar="calendar" />
        </div>

        <div v-for="source in group.sources" :key="`source:${source.id}`" class="flex items-center gap-2 rounded-md ps-2 transition-colors hover:bg-elevated pointer-coarse:min-h-11">
          <UCheckbox
            :model-value="!source.hidden"
            :color="source.color"
            :label="source.name"
            class="min-w-0 flex-1 py-1.5"
            :ui="{ label: 'truncate pointer-coarse:py-3' }"
            @update:model-value="setSourceHidden(group.realm, source, !$event)"
          />
          <span v-if="sendingLayers.includes(`source:${group.realm}:${source.id}`)" class="flex size-8 shrink-0 items-center justify-center">
            <UIcon :name="SENDING_ICON" class="size-4 animate-spin text-muted motion-reduce:animate-none" />
          </span>
          <UTooltip v-else :text="$t('calendar.label.received')">
            <span class="flex size-8 shrink-0 items-center justify-center">
              <UIcon :name="RECEIVED_ICON" class="size-4 text-muted" role="img" :aria-label="$t('calendar.label.received')" />
            </span>
          </UTooltip>
        </div>
      </template>
      <div v-else class="flex flex-col gap-1 ps-2" role="status" :aria-label="$t('calendar.status.loading')">
        <USkeleton class="my-2 h-4 w-32 rounded-sm" />
      </div>
    </section>

    <p v-if="calendarsFailed" role="alert" class="badge-ink flex items-center gap-2 ps-2 text-sm text-error">
      {{ $t("calendar.error.calendars_failed") }}
    </p>
  </div>
</template>

<script setup lang="ts">
import CalendarSettingsPopover from "./CalendarSettingsPopover.vue";
import { RECEIVED_ICON, REALM_ICON, SENDING_ICON } from "../../domain/colors";
import { useCalendarEvents } from "../../composables/useCalendarEvents";

const { groups, calendarsRead, calendarsFailed, setHidden, setSourceHidden, sendingLayers } = useCalendarEvents();
</script>
