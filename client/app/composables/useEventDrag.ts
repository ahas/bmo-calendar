import { addDays, addMinutes } from "date-fns";
import { useEventListener } from "@vueuse/core";
import { ref, toValue } from "vue";

import { fromLocalISO } from "../domain/dates";
import { DRAG_THRESHOLD, SNAP_MINUTES, minutesFromOffset, pxPerMinute, snapMinutes } from "../domain/layout";

import type { MaybeRefOrGetter } from "vue";
import type { CalendarEvent } from "../../shared/types";

// A block on the time grid moved across days or stretched from its foot by the pointer, on a quarter-hour
// snap. The block is moved as a preview; the event changes only when it is let go.
export function useEventDrag(event: MaybeRefOrGetter<CalendarEvent>, options: { onCommit: (start: Date, end: Date) => void }) {
  const dragging = ref(false);
  // Keeps the form from opening on the click that ends a drag.
  const suppressed = ref(false);
  const mode = ref<"move" | "resize">("move");
  const deltaMinutes = ref(0);
  const deltaDays = ref(0);
  const deltaX = ref(0);

  let active = false;
  let startX = 0;
  let startY = 0;
  let columnRects: DOMRect[] = [];
  let columnIndex = -1;
  let perMinute = 0;

  function reset(): void {
    active = false;
    dragging.value = false;
    deltaMinutes.value = 0;
    deltaDays.value = 0;
    deltaX.value = 0;
  }

  // A finger on a block scrolls the grid; it moves the event through its form instead.
  function onPointerdown(pointerEvent: PointerEvent): void {
    if (pointerEvent.button !== 0 || pointerEvent.pointerType === "touch") return;

    const target = pointerEvent.currentTarget as HTMLElement;

    mode.value = (pointerEvent.target as HTMLElement).closest("[data-resize-handle]") ? "resize" : "move";
    startX = pointerEvent.clientX;
    startY = pointerEvent.clientY;

    const grid = target.closest("[data-week-grid]");

    columnRects = grid ? [...grid.querySelectorAll("[data-day-column]")].map((column) => column.getBoundingClientRect()) : [];
    columnIndex = columnRects.findIndex((rect) => pointerEvent.clientX >= rect.left && pointerEvent.clientX < rect.right);
    perMinute = columnRects[0] ? pxPerMinute(columnRects[0]) : 0;

    target.setPointerCapture(pointerEvent.pointerId);
    active = true;
  }

  function onPointermove(pointerEvent: PointerEvent): void {
    if (!active) return;

    const dx = pointerEvent.clientX - startX;
    const dy = pointerEvent.clientY - startY;

    if (!dragging.value) {
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return;

      dragging.value = true;
      suppressed.value = true;
    }

    deltaMinutes.value = snapMinutes(minutesFromOffset(dy, perMinute));

    if (mode.value === "move" && columnIndex !== -1) {
      const targetIndex = columnRects.findIndex((rect) => pointerEvent.clientX >= rect.left && pointerEvent.clientX < rect.right);

      if (targetIndex !== -1) {
        deltaDays.value = targetIndex - columnIndex;
        deltaX.value = columnRects[targetIndex]!.left - columnRects[columnIndex]!.left;
      }
    }
  }

  function release(): void {
    setTimeout(() => {
      suppressed.value = false;
    });
  }

  function onPointerup(): void {
    if (active) {
      if (dragging.value && (deltaMinutes.value !== 0 || deltaDays.value !== 0)) {
        const source = toValue(event);
        const start = fromLocalISO(source.start);
        const end = fromLocalISO(source.end);

        if (mode.value === "move") {
          // Days for the step across columns, so the wall clock holds over a change of summer time.
          const shift = (date: Date) => addMinutes(addDays(date, deltaDays.value), deltaMinutes.value);

          options.onCommit(shift(start), shift(end));
        } else {
          const resized = addMinutes(end, deltaMinutes.value);

          options.onCommit(start, resized > addMinutes(start, SNAP_MINUTES) ? resized : addMinutes(start, SNAP_MINUTES));
        }
      }

      reset();
    }

    release();
  }

  function onPointercancel(): void {
    reset();
    release();
  }

  useEventListener("keydown", (keyboardEvent: KeyboardEvent) => {
    if (keyboardEvent.key === "Escape" && dragging.value) reset();
  });

  useEventListener("blur", () => {
    if (active || suppressed.value) {
      reset();
      release();
    }
  });

  return { dragging, suppressed, mode, deltaMinutes, deltaX, onPointerdown, onPointermove, onPointerup, onPointercancel };
}
