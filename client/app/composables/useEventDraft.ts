import { addDays, addMinutes, startOfDay } from "date-fns";
import { useEventListener } from "@vueuse/core";
import { computed, effectScope, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { navigateTo } from "nuxt/app";
import { useOpenedWith } from "bmo-ui/app/domain/form-save";

import { dateAtPoint, toCalendarDate, toDate, toLocalISO } from "../domain/dates";
import { DRAFT_EVENT_KEY, DRAG_THRESHOLD, SNAP_MINUTES, minutesInColumn } from "../domain/layout";
import { DEFAULT_ALARM, DEFAULT_REPEAT } from "../../shared/choices";
import { createAppComposable } from "./shared";
import { calendarKey, useCalendarEvents } from "./useCalendarEvents";
import { useCalendar } from "./useCalendar";

import type { EffectScope } from "vue";
import type { OpenedWith } from "bmo-ui/app/domain/form-save";

import type { QuickEvent } from "../domain/quick";
import type { Alarm, CalendarEvent, Realm, Repeat } from "../../shared/types";

// The event being drawn before it is one: a ghost on the grid with its form open beside it. It stands
// where a drag, a double press, a right press or a held finger drew it, or where the new-event button, the
// `n` key or the quick entry put it, and becomes an event only when its form is saved.

export interface EventDraft {
  start: Date;
  end: Date;
  allDay: boolean;
  title: string;
  /** The calendar it goes in, as `realm:id`. */
  calendar: string;
  description: string;
  alarm: Alarm;
  repeat: Repeat;
}

/** Where a gesture began and what it draws there: the month grid has no time of day under the pointer. */
export interface GridTarget {
  kind: "timed" | "allDay" | "month";
  day: Date;
}

interface Gesture {
  target: GridTarget;
  x: number;
  y: number;
  rect: DOMRect;
  anchorMinutes: number;
  anchorDate: Date;
  currentDate: Date;
  moved: boolean;
}

/** What a double press on the month grid and the new-event button draw on a grid with no time under it. */
const DEFAULT_HOUR = 9;

const _useEventDraft = () => {
  const { view, date, pathFor } = useCalendar();
  const { firstCalendar, calendarByKey, addEvent, rememberCalendar } = useCalendarEvents();

  const gridView = computed(() => (view.value === "year" ? "month" : view.value));

  const draft = shallowRef<EventDraft | null>(null);
  // The pointer is still down: the ghost is being drawn and its form waits for it to settle.
  const drawing = ref(false);
  const open = computed(() => !!draft.value && !drawing.value);

  // What the standing draft opened with, one snapshot to a draft: its form is drawn again wherever the
  // draft moves, so the draft rather than the form holds what it was drawn as.
  const openedWith = shallowRef<{ scope: EffectScope; read: OpenedWith } | null>(null);
  const changed = computed(() => openedWith.value?.read.dirty.value ?? false);

  function forgetOpenedWith(): void {
    openedWith.value?.scope.stop();
    openedWith.value = null;
  }

  // Where among its fields the draft's form held the focus as it was taken down to be drawn again on the
  // day the draft moved to, so the form drawn there gives it back rather than to its title.
  let carriedFocus: number | null = null;
  // Whether the draft's form has stood yet: only its first takes the focus on its own.
  let formStood = false;

  /** Whether a form of the draft stood before the one standing from now. */
  function standForm(): boolean {
    const stood = formStood;

    formStood = true;

    return stood;
  }

  function carryFocus(place: number): void {
    carriedFocus = place;
  }

  function takeCarriedFocus(): number | null {
    const place = carriedFocus;

    carriedFocus = null;

    return place;
  }

  function takeOpenedWith(): void {
    forgetOpenedWith();

    const scope = effectScope(true);

    openedWith.value = { scope, read: scope.run(() => useOpenedWith(() => draft.value, { open }))! };
  }

  const hosts = ref(0);
  const anchors = ref(0);
  let everAnchored = false;

  const pendingScroll = ref(false);
  let origin: HTMLElement | null = null;

  const draftEvent = computed<CalendarEvent | null>(() => {
    const held = draft.value;

    if (!held) return null;

    const calendar = calendarByKey(held.calendar);
    const realm: Realm = calendar?.realm ?? "mine";

    return {
      key: DRAFT_EVENT_KEY,
      id: DRAFT_EVENT_KEY,
      realm,
      company: calendar?.company ?? null,
      calendarId: calendar?.id ?? "",
      title: held.title,
      description: held.description,
      start: toLocalISO(held.start),
      end: toLocalISO(held.end),
      allDay: held.allDay,
      alarm: held.alarm,
      repeat: held.repeat,
      creator: "",
      createdAt: 0,
      changedAt: 0,
      told: null,
      occurrence: null
    };
  });

  function createDraft(input: { start: Date; end: Date; allDay?: boolean; title?: string; scroll?: boolean }): void {
    const calendar = firstCalendar.value;

    draft.value = {
      start: input.start,
      end: input.end,
      allDay: input.allDay ?? false,
      title: input.title ?? "",
      calendar: calendar ? calendarKey(calendar) : "",
      description: "",
      alarm: DEFAULT_ALARM,
      repeat: DEFAULT_REPEAT
    };

    takeOpenedWith();
    carriedFocus = null;
    formStood = false;
    everAnchored = false;
    pendingScroll.value = input.scroll ?? false;
    origin = document.activeElement as HTMLElement | null;
  }

  // Shallow: the form hands back a whole object rather than changing the dates the layout reads.
  function updateDraft(patch: Partial<EventDraft>): void {
    if (draft.value) draft.value = { ...draft.value, ...patch };
  }

  function discardDraft(refocus = false): void {
    if (!draft.value) return;

    draft.value = null;
    forgetOpenedWith();
    carriedFocus = null;
    drawing.value = false;
    pendingScroll.value = false;
    everAnchored = false;

    const target = origin;

    origin = null;

    if (refocus && target?.isConnected) target.focus();
  }

  /** The draft saved as an event, as its form holds it. */
  function commitDraft(values: EventDraft): void {
    const calendar = calendarByKey(values.calendar);

    if (!calendar) return;

    const id = crypto.randomUUID();

    rememberCalendar(calendar);
    void addEvent({
      key: `${calendar.realm}:${id}:`,
      id,
      realm: calendar.realm,
      company: calendar.company,
      calendarId: calendar.id,
      title: values.title.trim(),
      description: values.description,
      start: toLocalISO(values.start),
      end: toLocalISO(values.end),
      allDay: values.allDay,
      alarm: values.alarm,
      repeat: values.repeat,
      creator: "",
      createdAt: Date.now(),
      changedAt: Date.now(),
      told: null,
      occurrence: null
    });

    discardDraft(true);
  }

  /** The new event's first row: a draft an hour from now on the day the screen stands on. */
  async function createAtAnchor(title = ""): Promise<void> {
    if (!hosts.value) await navigateTo(pathFor(date.value, gridView.value));

    const hour = Math.min(23, new Date().getHours() + 1);
    const start = addMinutes(startOfDay(toDate(date.value)), hour * 60);

    createDraft({ start, end: addMinutes(start, 60), title, scroll: true });
  }

  /** The quick entry: what the phrase says, drawn as a draft on its day so what was read can be put right. */
  async function createFromQuick(quick: QuickEvent | null, text: string): Promise<void> {
    if (!quick) return createAtAnchor(text);

    await navigateTo(pathFor(toCalendarDate(quick.start), gridView.value));
    createDraft({ ...quick, scroll: true });
  }

  let gesture: Gesture | null = null;

  function onEmptySpace(event: Event): boolean {
    return !(event.target as HTMLElement | null)?.closest('[data-event],[data-draft],a,button,[role="button"]');
  }

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

  function endGesture(): void {
    gesture = null;
    unbind();
    drawing.value = false;
  }

  function geometry(event: PointerEvent): { start: Date; end: Date; allDay: boolean } {
    const current = gesture!;

    if (current.target.kind === "timed") {
      const day = startOfDay(current.target.day);
      const minutes = minutesInColumn(event.clientY, current.rect);
      const from = Math.min(current.anchorMinutes, minutes);
      const to = Math.max(Math.max(current.anchorMinutes, minutes), from + SNAP_MINUTES);

      return { start: addMinutes(day, from), end: addMinutes(day, to), allDay: false };
    }

    current.currentDate = dateAtPoint(event.clientX, event.clientY) ?? current.currentDate;

    const from = current.anchorDate < current.currentDate ? current.anchorDate : current.currentDate;
    const to = current.anchorDate < current.currentDate ? current.currentDate : current.anchorDate;

    return { start: startOfDay(from), end: addDays(startOfDay(to), 1), allDay: true };
  }

  function onGridPointerdown(event: PointerEvent, target: GridTarget): void {
    // A finger dragging over the grid is scrolling it.
    if (event.button !== 0 || event.pointerType === "touch" || !onEmptySpace(event)) return;

    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();

    gesture = {
      target,
      x: event.clientX,
      y: event.clientY,
      rect,
      anchorMinutes: minutesInColumn(event.clientY, rect, "floor"),
      anchorDate: target.day,
      currentDate: target.day,
      moved: false
    };
    bind();
  }

  function onPointermove(event: PointerEvent): void {
    if (!gesture) return;

    if (!gesture.moved) {
      if (Math.hypot(event.clientX - gesture.x, event.clientY - gesture.y) < DRAG_THRESHOLD) return;

      gesture.moved = true;
      drawing.value = true;
      createDraft(geometry(event));

      return;
    }

    updateDraft(geometry(event));
  }

  function onPointerup(): void {
    if (gesture) endGesture();
  }

  function onGridDblclick(event: MouseEvent, target: GridTarget): void {
    if (!onEmptySpace(event)) return;

    event.preventDefault();
    getSelection()?.removeAllRanges();
    drawAt(event, target);
  }

  // A right press, or the menu press a held finger makes (the grid's hold menu), on an empty part of the
  // grid draws there as a double press does; on an event it is that event's menu.
  function onGridContextmenu(event: MouseEvent, target: GridTarget): void {
    if (!onEmptySpace(event)) return;

    event.preventDefault();
    drawAt(event, target);
  }

  function drawAt(event: MouseEvent, target: GridTarget): void {
    const day = startOfDay(target.day);

    if (target.kind === "allDay") {
      createDraft({ start: day, end: addDays(day, 1), allDay: true });
      return;
    }

    const minutes = target.kind === "month" ? DEFAULT_HOUR * 60 : minutesInColumn(event.clientY, (event.currentTarget as HTMLElement).getBoundingClientRect(), "floor");
    const start = addMinutes(day, minutes);

    createDraft({ start, end: addMinutes(start, 60) });
  }

  function registerHost(): void {
    onMounted(() => hosts.value++);
    onUnmounted(() => hosts.value--);
  }

  function registerAnchor(): void {
    onMounted(() => {
      anchors.value++;
      everAnchored = true;
    });
    onUnmounted(() => anchors.value--);
  }

  // A ghost with nowhere left to stand — the view changed, its row scrolled away — takes its unsaved
  // draft with it.
  watch(anchors, (count) => {
    if (count || !draft.value || !everAnchored) return;

    void nextTick(() => {
      if (!anchors.value) discardDraft();
    });
  });

  useEventListener("keydown", (event: KeyboardEvent) => {
    if (event.key === "Escape" && gesture?.moved) {
      endGesture();
      discardDraft();
    }
  });

  useEventListener("blur", () => {
    if (gesture) endGesture();
  });

  return {
    draft,
    drawing,
    open,
    changed,
    draftEvent,
    pendingScroll,
    createDraft,
    updateDraft,
    discardDraft,
    commitDraft,
    carryFocus,
    takeCarriedFocus,
    standForm,
    createAtAnchor,
    createFromQuick,
    onGridPointerdown,
    onGridDblclick,
    onGridContextmenu,
    registerHost,
    registerAnchor
  };
};

export const useEventDraft = createAppComposable(_useEventDraft);
