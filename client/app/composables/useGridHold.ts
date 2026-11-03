import { useEventListener } from "@vueuse/core";
import { useHoldMenu } from "bmo-ui/app/domain/holdmenu";

// A held finger on the calendar, through the console's one long press: on an event it opens the event's
// menu, and on an empty part of the grid it is the menu press the grid reads as drawing a new event there
// (`useEventDraft`). Once the hold has fired, the finger's lift presses nothing, wherever it lands — the
// menu or the new event's dialog may have come up under it — and the next press starts afresh.
export function useGridHold(surface: () => HTMLElement | null): void {
  let touched = false;
  let held = false;

  useHoldMenu({ surface });

  useEventListener(
    surface,
    "pointerdown",
    (event: PointerEvent) => {
      touched = event.pointerType === "touch";
    },
    { capture: true, passive: true }
  );

  useEventListener(
    surface,
    "contextmenu",
    () => {
      if (touched) held = true;
    },
    { capture: true }
  );

  // A new press, anywhere, is no lift of the hold.
  useEventListener(
    "pointerdown",
    () => {
      held = false;
    },
    { capture: true, passive: true }
  );

  function swallow(event: Event): void {
    if (!held) return;

    if (event.type === "click") held = false;

    event.preventDefault();
    event.stopPropagation();
  }

  useEventListener("pointerup", swallow, { capture: true });
  useEventListener("click", swallow, { capture: true });
}
