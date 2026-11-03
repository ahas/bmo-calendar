<template>
  <!-- The calendars, the account's own and the organization's apart: each with its colour, shown or hidden on the grid for
       this account, and its settings a press away. -->
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
        <div v-for="calendar in group.calendars" :key="calendar.id" class="flex items-center gap-2 rounded-md ps-2 transition-colors hover:bg-elevated">
          <UCheckbox
            :model-value="!calendar.hidden"
            :color="calendar.color"
            :label="calendar.name"
            class="min-w-0 flex-1 py-1.5"
            :ui="{ label: 'truncate' }"
            @update:model-value="setHidden(calendar, !$event)"
          />
          <CalendarSettingsPopover :realm="group.realm" :calendar="calendar" />
        </div>
      </template>
      <div v-else class="flex flex-col gap-1 ps-2" role="status" :aria-label="$t('calendar.status.loading')">
        <USkeleton class="my-2 h-4 w-32 rounded-sm" />
      </div>
    </section>

    <p v-if="calendarsFailed" role="alert" class="flex items-center gap-2 ps-2 text-sm text-error">
      {{ $t("calendar.error.calendars_failed") }}
    </p>
  </div>
</template>

<script setup lang="ts">
import CalendarSettingsPopover from "./CalendarSettingsPopover.vue";
import { REALM_ICON } from "../../domain/colors";
import { useCalendarEvents } from "../../composables/useCalendarEvents";

const { groups, calendarsRead, calendarsFailed, setHidden } = useCalendarEvents();
</script>
