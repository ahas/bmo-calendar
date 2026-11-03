import { addDays, differenceInCalendarDays } from "date-fns";
import { useEventListener } from "@vueuse/core";
import { computed, ref, shallowRef } from "vue";

import { dateAtPoint, fromLocalISO, toLocalISO } from "../domain/dates";
import { DRAG_THRESHOLD } from "../domain/layout";
import { createAppComposable } from "./shared";
import { useEventChange } from "./useEventChange";

import type { CalendarEvent } from "../../shared/types";

// A chip moved a whole day at a time: the month grid has no time of day, and an all-day bar keeps its
// span, so only the date moves. One for the whole app, since the month view puts hundreds of chips up.
const _useEventMove = () => {
  const { changeEvent } = useEventChange();

  const source = shallowRef<CalendarEvent | null>(null);
  const deltaDays = ref(0);
  const suppressed = ref(false);

  const movingKey = computed(() => source.value?.key ?? null);

  // Where it would land, drawn through the layout in the cell under the pointer.
  const preview = computed<CalendarEvent | null>(() => {
    if (!source.value || !deltaDays.value) return source.value;

    return {
      ...source.value,
      start: toLocalISO(addDays(fromLocalISO(source.value.start), deltaDays.value)),
      end: toLocalISO(addDays(fromLocalISO(source.value.end), deltaDays.value))
    };
  });

  let gesture: { event: CalendarEvent; x: number; y: number; origin: Date; current: Date; moved: boolean; cancelled: boolean } | null = null;

  function bind(): void {
    document.addEventListener("pointermove", onPointermove);
    document.addEventListener("pointerup", onPointerup);
    document.addEventListener("pointercancel", onPointerup);
  }

  function unbind(): void {
    document.removeEventListener("pointermove", onPointermove);
    document.removeEventListener("pointerup", onPointerup);
    document.removeEventListener("pointercancel", onPointerup);
  }

  function reset(): void {
    gesture = null;
    unbind();
    source.value = null;
    deltaDays.value = 0;
  }

  function release(): void {
    setTimeout(() => {
      suppressed.value = false;
    });
  }

  function onPointerdown(pointerEvent: PointerEvent, event: CalendarEvent): void {
    if (pointerEvent.button !== 0 || pointerEvent.pointerType === "touch") return;

    // A chip listed over the grid has no day under it to move to.
    if ((pointerEvent.target as HTMLElement).closest("[data-reka-popper-content-wrapper]")) return;

    const origin = dateAtPoint(pointerEvent.clientX, pointerEvent.clientY);

    if (!origin) return;

    gesture = { event, x: pointerEvent.clientX, y: pointerEvent.clientY, origin, current: origin, moved: false, cancelled: false };
    bind();
  }

  function onPointermove(pointerEvent: PointerEvent): void {
    if (!gesture || gesture.cancelled) return;

    if (!gesture.moved) {
      if (Math.hypot(pointerEvent.clientX - gesture.x, pointerEvent.clientY - gesture.y) < DRAG_THRESHOLD) return;

      gesture.moved = true;
      suppressed.value = true;
      source.value = gesture.event;
    }

    gesture.current = dateAtPoint(pointerEvent.clientX, pointerEvent.clientY) ?? gesture.current;
    deltaDays.value = differenceInCalendarDays(gesture.current, gesture.origin);
  }

  function onPointerup(): void {
    if (!gesture) return;

    const event = source.value;
    const delta = deltaDays.value;

    if (gesture.moved && event && delta) {
      void changeEvent(event, {
        ...event,
        start: toLocalISO(addDays(fromLocalISO(event.start), delta)),
        end: toLocalISO(addDays(fromLocalISO(event.end), delta))
      });
    }

    reset();
    release();
  }

  useEventListener("keydown", (keyboardEvent: KeyboardEvent) => {
    if (keyboardEvent.key === "Escape" && gesture?.moved) {
      gesture.cancelled = true;
      gesture.moved = false;
      source.value = null;
      deltaDays.value = 0;
    }
  });

  useEventListener("blur", () => {
    if (gesture || suppressed.value) {
      reset();
      release();
    }
  });

  return { movingKey, preview, suppressed, onPointerdown };
};

export const useEventMove = createAppComposable(_useEventMove);
