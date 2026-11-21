<template>
  <!-- The month view of the booking screen (docs/design.md, 「The console」): a grid of days, each naming how
       many bookings each resource of the kind shown holds on it; on a phone, where a cell has no room for names,
       how many it holds in all. A day is pressed to open its day view; each is named to a reader by its full
       date and what it holds. -->
  <div class="flex min-h-0 flex-1 flex-col">
    <div data-body-head class="grid min-h-(--bmo-list-head) shrink-0 grid-cols-7 items-center border-b border-default" aria-hidden="true">
      <span v-for="day in props.days.slice(0, 7)" :key="day.getDay()" class="py-2 text-center text-sm text-muted">{{ formatWeekday(day) }}</span>
    </div>

    <div class="grid min-h-0 flex-1 auto-rows-[minmax(6rem,1fr)] grid-cols-7 overflow-y-auto pb-[var(--bmo-create-room)]">
      <ULink
        v-for="(day, index) in props.days"
        :key="day.getTime()"
        raw
        :to="props.dayPath(day)"
        class="focus-inset flex min-w-0 flex-col gap-1 border-b border-default p-1 text-sm transition-colors hover:bg-elevated active:bg-accented [&:not(:nth-child(7n+1))]:border-s"
        :class="[isWeekend(day) && 'bg-elevated/50', !isSameMonth(day, props.month) && 'text-muted']"
        :aria-label="dayLabel(day, index)"
        :aria-current="isToday(day) ? 'date' : undefined"
      >
        <span class="flex size-7 shrink-0 items-center justify-center self-start rounded-full font-semibold tabular-nums" :class="isToday(day) && 'badge-ink bg-primary text-inverted'">
          {{ day.getDate() }}
        </span>

        <template v-if="props.loading">
          <USkeleton class="h-4 w-full rounded-sm" />
        </template>
        <UBadge v-else-if="narrow" v-show="totalOf(index)" v-bind="COUNT_BADGE" color="neutral" :label="countText(totalOf(index))" class="self-start" />
        <template v-else>
          <span v-for="line in linesOf(index).shown" :key="line.resource.id" class="flex min-w-0 items-center gap-1">
            <span class="min-w-0 flex-1 truncate">{{ line.resource.name }}</span>
            <UBadge v-bind="COUNT_BADGE" color="neutral" :label="countText(line.count)" />
          </span>
          <span v-if="linesOf(index).rest" class="truncate text-muted">{{ $t("calendar.label.more", { n: countText(linesOf(index).rest) }) }}</span>
        </template>
      </ULink>
    </div>
  </div>
</template>

<script setup lang="ts">
import { isSameMonth, isToday, isWeekend } from "date-fns";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { COUNT_BADGE, countText } from "bmo-ui/app/domain/numbers";
import { useNarrow } from "bmo-ui/app/domain/screen";

import { dayCounts } from "../../domain/bookings";
import { formatFullDate, formatWeekday } from "../../domain/dates";

import type { Booking, Resource } from "../../../shared/types";

/** How many resources a day names before the rest are counted. */
const SHOWN_LINES = 3;

const props = defineProps<{
  days: Date[];
  month: Date;
  resources: Resource[];
  bookings: Booking[];
  loading?: boolean;
  dayPath: (day: Date) => { path: string; query: Record<string, string> };
}>();

const { t } = useI18n();
const narrow = useNarrow();

const counts = computed(() => dayCounts(props.bookings, props.days));

interface Line {
  resource: Resource;
  count: number;
}

// Each day's resources holding a booking on it, in the order the resources stand.
const lines = computed<Line[][]>(() =>
  props.days.map((_, index) => props.resources.map((resource) => ({ resource, count: counts.value.get(resource.id)?.[index] ?? 0 })).filter((line) => line.count > 0))
);

function linesOf(index: number): { shown: Line[]; rest: number } {
  const all = lines.value[index] ?? [];

  return { shown: all.slice(0, SHOWN_LINES), rest: Math.max(0, all.length - SHOWN_LINES) };
}

function totalOf(index: number): number {
  return (lines.value[index] ?? []).reduce((sum, line) => sum + line.count, 0);
}

function dayLabel(day: Date, index: number): string {
  const parts = [formatFullDate(day)];

  if (isToday(day)) parts.push(t("calendar.aria.today"));

  const held = lines.value[index] ?? [];

  if (held.length === 0) parts.push(t("calendar.aria.bookings_none"));
  else for (const line of held) parts.push(t("calendar.aria.resource_bookings", { name: line.resource.name, n: countText(line.count) }));

  return parts.join(", ");
}
</script>
