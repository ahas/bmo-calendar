import { differenceInCalendarDays, differenceInMinutes, startOfDay } from "date-fns";

import { fromLocalISO } from "./dates";

import type { CalendarEvent } from "../../shared/types";

// Where each event stands on a day's column and in the all-day row: events that overlap, directly or
// through another, form a cluster, and each cluster splits into the fewest columns that hold it.

/**
 * An hour's height on the time grid, in the reader's own text size, so a larger font grows the grid rather
 * than cutting what stands in it. The grid declares it as `--calendar-hour` and a minute as
 * `--calendar-minute`; a gesture reads the pixels a minute takes off the column it is made on.
 */
export const HOUR_REM = 4;
/** Below this length a block has room for its title alone, as a line of the reader's own text size. */
export const COMPACT_MINUTES = 42;
/** Every move and resize lands on a quarter hour. */
export const SNAP_MINUTES = 15;
/** The height drawn at the least, not a duration: a quarter-hour event still reads. */
export const MIN_EVENT_MINUTES = 30;
export const DAY_MINUTES = 24 * 60;
/** How far a pointer travels before a press is a drag. */
export const DRAG_THRESHOLD = 5;
/** The key the draft rides through the layout under. */
export const DRAFT_EVENT_KEY = "draft";

export interface PositionedEvent {
  event: CalendarEvent;
  /** The minute of the day it is drawn from. */
  startMinute: number;
  /** How many minutes it is drawn over, at least the least a block is drawn at. */
  minutes: number;
  left: number;
  width: number;
}

export interface AllDayPositionedEvent {
  event: CalendarEvent;
  colStart: number;
  colSpan: number;
  lane: number;
}

export function layoutDay(events: CalendarEvent[], day: Date): PositionedEvent[] {
  const dayStart = startOfDay(day);
  const items = events
    .map((event) => ({
      event,
      startMin: Math.max(0, differenceInMinutes(fromLocalISO(event.start), dayStart)),
      endMin: Math.min(DAY_MINUTES, differenceInMinutes(fromLocalISO(event.end), dayStart))
    }))
    .filter((item) => item.endMin > item.startMin)
    .map((item) => ({ ...item, endMin: Math.max(item.endMin, item.startMin + MIN_EVENT_MINUTES) }))
    .sort((a, b) => a.startMin - b.startMin || b.endMin - a.endMin);

  const positioned: PositionedEvent[] = [];
  let cluster: typeof items = [];
  let clusterEnd = -Infinity;

  function flush(): void {
    const columns: number[] = [];
    const assigned = cluster.map((item) => {
      let index = columns.findIndex((end) => end <= item.startMin);

      if (index === -1) index = columns.length;
      columns[index] = item.endMin;

      return index;
    });

    for (const [index, item] of cluster.entries()) {
      positioned.push({
        event: item.event,
        startMinute: item.startMin,
        minutes: item.endMin - item.startMin,
        left: (assigned[index]! / columns.length) * 100,
        width: 100 / columns.length
      });
    }
  }

  for (const item of items) {
    if (cluster.length && item.startMin >= clusterEnd) {
      flush();
      cluster = [];
      clusterEnd = -Infinity;
    }

    cluster.push(item);
    clusterEnd = Math.max(clusterEnd, item.endMin);
  }

  if (cluster.length) flush();

  return positioned;
}

export function layoutAllDay(events: CalendarEvent[], days: Date[]): AllDayPositionedEvent[] {
  const first = days[0];

  if (!first) return [];

  const items = events
    .map((event) => {
      const colStart = Math.max(0, differenceInCalendarDays(fromLocalISO(event.start), first));
      // An all-day event's end is the midnight after its last day, which that day does not reach.
      const colEnd = Math.min(days.length, Math.max(colStart + 1, differenceInCalendarDays(fromLocalISO(event.end), first)));

      return { event, colStart, colSpan: colEnd - colStart };
    })
    .filter((item) => item.colSpan > 0)
    .sort((a, b) => a.colStart - b.colStart || b.colSpan - a.colSpan);

  const lanes: number[] = [];

  return items.map((item) => {
    let lane = lanes.findIndex((end) => end <= item.colStart);

    if (lane === -1) lane = lanes.length;
    lanes[lane] = item.colStart + item.colSpan;

    return { ...item, lane };
  });
}

export function snapMinutes(minutes: number): number {
  return Math.round(minutes / SNAP_MINUTES) * SNAP_MINUTES;
}

/** The pixels a minute takes on a day's column, which stands for the whole day. */
export function pxPerMinute(column: { height: number }): number {
  return column.height / DAY_MINUTES;
}

export function minutesFromOffset(px: number, perMinute: number): number {
  return perMinute > 0 ? px / perMinute : 0;
}

/** The minute of the day a pointer stands at in a day's column, snapped and held to the day. */
export function minutesInColumn(clientY: number, rect: DOMRect, mode: "floor" | "nearest" = "nearest"): number {
  const raw = minutesFromOffset(clientY - rect.top, pxPerMinute(rect));
  const snapped = mode === "floor" ? Math.floor(raw / SNAP_MINUTES) * SNAP_MINUTES : snapMinutes(raw);

  return Math.min(DAY_MINUTES, Math.max(0, snapped));
}

/** A length of so many minutes on the time grid, in its own scale. */
export function minutesLength(minutes: number): string {
  return `calc(var(--calendar-minute) * ${minutes})`;
}

/** The box a block and the draft share; a block adds its own drag on top. */
export function eventBlockStyle(positioned: PositionedEvent, minutes = positioned.minutes): Record<string, string> {
  return {
    top: `calc(${minutesLength(positioned.startMinute)} + 2px)`,
    height: `calc(${minutesLength(minutes)} - 3px)`,
    insetInlineStart: `calc(${positioned.left}% + 1px)`,
    width: `calc(${positioned.width}% - 2px)`
  };
}

/** The pixels a rem takes in the reader's own text size; 16 where no page stands to read it from. */
export function rootFontPx(): number {
  if (typeof document === "undefined") return 16;

  return Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
}
