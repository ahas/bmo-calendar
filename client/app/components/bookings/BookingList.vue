<template>
  <!-- A list of bookings, each with its state, its title, when it is, its resource and, where it says so,
       who booked it and the reason it was refused; its actions stand at the row's end. The reader's own
       bookings stand one row a booking, a repeating one naming how it repeats and its next date, and are
       changed or cancelled whole from it; a keeper's requests are approved with one press or refused with a
       reason. -->
  <div class="min-h-0 flex-1 overflow-y-auto pb-[var(--bmo-create-room)]">
    <ul v-if="props.read && props.bookings.length" class="divide-y divide-default">
      <li v-for="booking in props.bookings" :key="bookingKey(booking)" class="flex items-center gap-3 px-4 py-2 pointer-coarse:py-3">
        <HintTip :text="$t(STATE_LABELS[booking.state])" class="flex shrink-0">
          <UIcon :name="STATE_ICONS[booking.state]" class="badge-ink size-5" :class="STATE_COLORS[booking.state]" role="img" :aria-label="$t(STATE_LABELS[booking.state])" />
        </HintTip>

        <div class="flex min-w-0 flex-1 flex-col">
          <span class="truncate font-medium text-highlighted">{{ booking.title }}</span>
          <span class="truncate text-sm text-muted">{{ detailOf(booking) }}</span>
          <span v-if="booking.reason" class="text-sm text-muted">{{ $t("calendar.label.refusal_reason", { reason: booking.reason }) }}</span>
        </div>

        <div class="flex shrink-0 items-center gap-1">
          <slot name="actions" :booking="booking" />
        </div>
      </li>
    </ul>

    <p v-else-if="props.read && props.failed" role="alert" class="badge-ink px-4 py-3 text-sm text-error">{{ $t("calendar.error.bookings_failed") }}</p>
    <p v-else-if="props.read" class="px-4 py-3 text-sm text-muted">{{ props.empty }}</p>
    <div v-else class="flex flex-col gap-3 px-4 py-3" role="status" :aria-label="$t('calendar.status.loading')">
      <USkeleton v-for="row in 3" :key="row" class="h-10 w-full rounded-md" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { eventWhen } from "../../domain/a11y";
import { STATE_COLORS, STATE_ICONS, STATE_LABELS, bookingKey } from "../../domain/bookings";
import { formatMonthDay, formatWeekday, fromLocalISO } from "../../domain/dates";
import { REPEAT_CHOICES } from "../../../shared/choices";
import { useBookings } from "../../composables/useBookings";
import { usePeople } from "../../composables/usePeople";

import type { Booking } from "../../../shared/types";

const props = defineProps<{
  bookings: Booking[];
  read: boolean;
  failed: boolean;
  /** The words of the list with nothing in it. */
  empty: string;
  /** Whether each row names its booker. */
  booker?: boolean;
}>();

const { t } = useI18n();
const { resourceOf } = useBookings();
const { nameOf } = usePeople();

/** When a booking is — a repeating one, how it repeats and its next date — on what, and for a keeper's list
 *  who asked. */
function detailOf(booking: Booking): string {
  const start = fromLocalISO(booking.start);
  const when = `${formatMonthDay(start)} (${formatWeekday(start)}) ${eventWhen(t, booking)}`;
  const repeat = REPEAT_CHOICES.find((choice) => choice.value === booking.repeat);
  const parts = [
    ...(booking.repeat !== "none" && repeat ? [t(repeat.label), t("calendar.label.next_occurrence", { when })] : [when]),
    resourceOf(booking.resourceId)?.name ?? ""
  ];

  if (props.booker) parts.push(nameOf(booking.booker));

  return parts.filter(Boolean).join(" · ");
}
</script>
