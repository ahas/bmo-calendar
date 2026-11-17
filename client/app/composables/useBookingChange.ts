import { useOverlay } from "@nuxt/ui/composables/useOverlay";
import { useI18n } from "vue-i18n";
import { askQuestion } from "bmo-ui/app/domain/question";

import CalendarScopeDialog from "../components/calendar/CalendarScopeDialog.vue";
import { useBookings } from "./useBookings";

import type { Booking, BookingWrite, RepeatScope } from "../../shared/types";

// A change or a cancellation of a booking as its form and the lists make it (`docs/design.md`, 「Resources and
// bookings」): a repeating booking's row asks first whether it reaches that occurrence alone or the whole
// booking, and a cancellation of one that does not repeat asks whether to go through with it. A refusal is
// thrown to the caller, which shows it where the reader is.

/** Whether a booking's row stands for one occurrence of a repeating booking. */
export function repeats(booking: Pick<Booking, "repeat" | "occurrence">): boolean {
  return booking.repeat !== "none" || !!booking.occurrence;
}

export function useBookingChange() {
  const { t } = useI18n();
  const overlay = useOverlay();
  const { changeBooking, cancelBooking } = useBookings();

  // `wholeOnly` offers the whole booking alone: a change one occurrence cannot take, as its repeat.
  async function askScope(cancelling: boolean, wholeOnly = false): Promise<RepeatScope | null> {
    const title = t(cancelling ? "calendar.message.booking_scope_cancel" : "calendar.message.booking_scope_change");
    const props = { title, cancelling, oneLabel: t("calendar.action.booking_scope_one"), wholeOnly };
    const answer = await overlay.create(CalendarScopeDialog, { props, destroyOnClose: true }).open();

    return (answer as RepeatScope | null | undefined) ?? null;
  }

  /** Answers whether the change was sent; a reader who backed out of the question sent none. A change of the
   *  repeat reaches the whole booking, since one occurrence keeps the repeat it stands in. */
  async function change(before: Booking, after: Omit<BookingWrite, "id" | "occurrence">, whole = false): Promise<boolean> {
    const scope = repeats(before) && !whole ? await askScope(false, after.repeat !== undefined && after.repeat !== before.repeat) : "all";

    if (scope === null) return false;

    await changeBooking(before, after, scope);

    return true;
  }

  /** Answers whether the cancellation was sent. `whole` cancels the whole booking, asking as one that does not
   *  repeat asks. */
  async function cancel(booking: Booking, whole = false): Promise<boolean> {
    let scope: RepeatScope | null = "all";

    if (repeats(booking) && !whole) {
      scope = await askScope(true);
    } else {
      const taken = await askQuestion({
        title: t("calendar.message.booking_cancel"),
        item: booking.title,
        confirm: t("calendar.action.cancel_booking"),
        cancel: t("common.action.close")
      });

      scope = taken ? "all" : null;
    }

    if (scope === null) return false;

    await cancelBooking(booking, scope);

    return true;
  }

  return { change, cancel };
}
