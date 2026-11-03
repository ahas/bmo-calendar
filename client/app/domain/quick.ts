import { addDays, addMinutes, startOfDay } from "date-fns";

// The quick entry: a phrase as a person types it — 「내일 오후 3시 치과」, 「11월 5일 종일 워크숍」,
// "Dentist tomorrow at 3pm" — read into a title and a time. The reading is done here, in the browser, by
// rules alone: a day, a time or a span of times, and an all-day word are found and cut out, and what is
// left is the title. A phrase with no day and no time in it reads as none, and the caller picks a time.

export interface QuickEvent {
  title: string;
  start: Date;
  end: Date;
  allDay: boolean;
}

interface Found {
  from: number;
  to: number;
}

const WEEKDAYS_KO = ["일", "월", "화", "수", "목", "금", "토"];
const WEEKDAYS_EN = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
const RELATIVE_DAYS: Record<string, number> = { 오늘: 0, 내일: 1, 모레: 2, 글피: 3, today: 0, tonight: 0, tomorrow: 1 };

// Words a phrase spends leading into a time, which the title is not left holding.
const CONNECTORS = new Set(["at", "on", "from", "to", "until", "by", "the", "this", "next", "and", "-", "~", "@"]);

/** The hour a marked or unmarked clock reading stands for, on a 24-hour clock. */
function hourOf(hour: number, marker: string | undefined): number {
  const mark = (marker ?? "").toLowerCase();

  if (["오후", "저녁", "밤", "pm"].includes(mark)) return hour < 12 ? hour + 12 : hour;
  if (["오전", "아침", "새벽", "am"].includes(mark)) return hour === 12 ? 0 : hour;
  if (mark === "점심" || mark === "낮") return hour < 6 ? hour + 12 : hour;

  // An hour from one to six said with no mark is the afternoon's: nobody books 3 o'clock at night.
  return hour >= 1 && hour <= 6 ? hour + 12 : hour;
}

interface Clock {
  minutes: number;
  found: Found;
}

const MARK = "(오전|오후|아침|점심|낮|저녁|밤|새벽)?\\s*";
const KO_CLOCK = `(\\d{1,2})\\s*시(?!간)(?:\\s*(\\d{1,2})\\s*분|\\s*(반))?`;
const COLON_CLOCK = "(\\d{1,2}):(\\d{2})";
const EN_CLOCK = "(\\d{1,2})(?::(\\d{2}))?\\s*(am|pm)";

function clocksIn(text: string): Clock[] {
  const clocks: Clock[] = [];
  const pattern = new RegExp(`${MARK}(?:${KO_CLOCK}|${COLON_CLOCK}|${EN_CLOCK})(?:에|부터|까지)?`, "gi");

  for (const match of text.matchAll(pattern)) {
    const marker = match[1];
    let hour: number;
    let minute = 0;

    if (match[2] !== undefined) {
      hour = Number(match[2]);
      minute = match[4] !== undefined ? 30 : Number(match[3] ?? 0);
      hour = hourOf(hour, marker);
    } else if (match[5] !== undefined) {
      hour = Number(match[5]);
      minute = Number(match[6]);
      hour = marker ? hourOf(hour, marker) : hour;
    } else {
      hour = hourOf(Number(match[7]), match[9]);
      minute = Number(match[8] ?? 0);
    }

    if (hour > 23 || minute > 59) continue;

    clocks.push({ minutes: hour * 60 + minute, found: { from: match.index, to: match.index + match[0].length } });
  }

  return clocks;
}

interface Day {
  date: Date;
  found: Found;
}

function dayIn(text: string, now: Date): Day | null {
  const today = startOfDay(now);
  const lower = text.toLowerCase();

  // 2026-11-05 or 2026.11.05
  const iso = /(\d{4})[-.](\d{1,2})[-.](\d{1,2})(?:일)?(?:에)?/.exec(text);

  if (iso) return { date: new Date(Number(iso[1]), Number(iso[2]) - 1, Number(iso[3])), found: { from: iso.index, to: iso.index + iso[0].length } };

  // 11월 5일, 11/5: this year's, or next year's where it has gone by.
  const monthDay = /(\d{1,2})\s*월\s*(\d{1,2})\s*일(?:에)?|(?<![\d:])(\d{1,2})\/(\d{1,2})(?![\d:])/.exec(text);

  if (monthDay) {
    const month = Number(monthDay[1] ?? monthDay[3]) - 1;
    const day = Number(monthDay[2] ?? monthDay[4]);
    let date = new Date(today.getFullYear(), month, day);

    if (date < today) date = new Date(today.getFullYear() + 1, month, day);

    return { date, found: { from: monthDay.index, to: monthDay.index + monthDay[0].length } };
  }

  // 오늘, 내일, 모레, 글피, today, tomorrow
  const relative = /(오늘|내일|모레|글피|today|tonight|tomorrow)(?:은|에)?/i.exec(text);

  if (relative) return { date: addDays(today, RELATIVE_DAYS[relative[1]!.toLowerCase()]!), found: { from: relative.index, to: relative.index + relative[0].length } };

  // (이번 주|다음 주) 금요일, 금요일
  const koWeekday = /(?:(이번|다음|담)\s*주\s*)?([일월화수목금토])요일(?:에|날)?/.exec(text);

  if (koWeekday) {
    const target = WEEKDAYS_KO.indexOf(koWeekday[2]!);
    const thisWeekStart = addDays(today, -today.getDay());
    let date: Date;

    if (koWeekday[1] === "다음" || koWeekday[1] === "담") date = addDays(thisWeekStart, 7 + target);
    else if (koWeekday[1] === "이번") date = addDays(thisWeekStart, target);
    else date = addDays(today, (target - today.getDay() + 7) % 7);

    return { date, found: { from: koWeekday.index, to: koWeekday.index + koWeekday[0].length } };
  }

  // (next) friday
  const enWeekday = new RegExp(`(next\\s+)?(${WEEKDAYS_EN.join("|")})`).exec(lower);

  if (enWeekday) {
    const target = WEEKDAYS_EN.indexOf(enWeekday[2]!);
    const ahead = (target - today.getDay() + 7) % 7;

    return { date: addDays(today, enWeekday[1] ? ahead + 7 : ahead), found: { from: enWeekday.index, to: enWeekday.index + enWeekday[0].length } };
  }

  // 5일: this month's, or next month's where it has gone by.
  const dayOnly = /(?<![\d월])(\d{1,2})\s*일(?:에)?(?!\s*(?:동안|간))/.exec(text);

  if (dayOnly) {
    const day = Number(dayOnly[1]);
    let date = new Date(today.getFullYear(), today.getMonth(), day);

    if (date < today) date = new Date(today.getFullYear(), today.getMonth() + 1, day);

    return { date, found: { from: dayOnly.index, to: dayOnly.index + dayOnly[0].length } };
  }

  return null;
}

function allDayIn(text: string): Found | null {
  const match = /(하루\s*종일|종일|all[\s-]?day)/i.exec(text);

  return match ? { from: match.index, to: match.index + match[0].length } : null;
}

/** How long a span runs where it says so: 「2시간」, 「30분 동안」, "for 2 hours". */
function lengthIn(text: string): { minutes: number; found: Found } | null {
  const ko = /(\d{1,2})\s*시간(?:\s*(\d{1,2})\s*분)?(?:\s*동안)?|(\d{1,3})\s*분\s*동안/.exec(text);

  if (ko) {
    const minutes = ko[3] !== undefined ? Number(ko[3]) : Number(ko[1]) * 60 + Number(ko[2] ?? 0);

    return { minutes, found: { from: ko.index, to: ko.index + ko[0].length } };
  }

  const en = /for\s+(\d{1,2})\s*(hours?|h|minutes?|min)/i.exec(text);

  if (en) return { minutes: Number(en[1]) * (en[2]!.toLowerCase().startsWith("h") ? 60 : 1), found: { from: en.index, to: en.index + en[0].length } };

  return null;
}

/** The phrase with what was read out of it cut away, the words left leading into a cut dropped. */
function titleWithout(text: string, cuts: Found[]): string {
  const sorted = [...cuts].sort((a, b) => a.from - b.from);
  const parts: string[] = [];
  let at = 0;

  for (const cut of sorted) {
    if (cut.from >= at) parts.push(text.slice(at, cut.from));

    at = Math.max(at, cut.to);
  }

  parts.push(text.slice(at));

  return parts
    .flatMap((part, index) => {
      const words = part.split(/\s+/).filter(Boolean);

      while (words.length && CONNECTORS.has(words.at(-1)!.toLowerCase())) words.pop();
      while (index > 0 && words.length && CONNECTORS.has(words[0]!.toLowerCase())) words.shift();

      return words;
    })
    .join(" ")
    .replace(/^[~\-–]+|[~\-–]+$/g, "")
    .trim();
}

/** Reads a phrase into a title and a time, or none where it names neither a day nor a time. */
export function parseQuickEvent(phrase: string, now: Date = new Date()): QuickEvent | null {
  const text = phrase.trim();

  if (!text) return null;

  const day = dayIn(text, now);
  const clocks = clocksIn(text);
  const allDay = allDayIn(text);
  const length = lengthIn(text);

  if (!day && clocks.length === 0 && !allDay) return null;

  const cuts = [day?.found, allDay, length?.found, ...clocks.map((clock) => clock.found)].filter((cut): cut is Found => !!cut);

  // A range written between two clocks — 「3시~5시」, "3pm - 5pm" — keeps its joining mark out of the title.
  const title = titleWithout(text, cuts);
  let date = day?.date ?? startOfDay(now);

  if (allDay || clocks.length === 0) {
    return { title, start: date, end: addDays(date, 1), allDay: true };
  }

  const first = clocks[0]!;

  // A time already gone by today, with no day named, is tomorrow's.
  if (!day && addMinutes(date, first.minutes) < now) date = addDays(date, 1);

  const start = addMinutes(date, first.minutes);
  let end: Date;

  if (clocks[1]) {
    end = addMinutes(date, clocks[1].minutes);

    if (end <= start) end = addMinutes(end, 12 * 60 <= first.minutes ? 24 * 60 : 12 * 60);
    if (end <= start) end = addMinutes(start, 60);
  } else {
    end = addMinutes(start, length?.minutes ?? 60);
  }

  return { title, start, end, allDay: false };
}
