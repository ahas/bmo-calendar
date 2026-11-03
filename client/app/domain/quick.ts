import { addDays, addMinutes, startOfDay } from "date-fns";

import KO_DATA from "../../i18n/data/ko.json";

// The quick entry: a phrase as a person types it, read into a title and a time. The reading is done here,
// in the browser, by rules alone: a day, a time or a span of times, a length and an all-day word are found
// and cut out, and what is left is the title. A phrase with no day and no time in it reads as none, and the
// caller picks a time. English is read by the rules written here; Korean by the words the data file beside
// the language pack holds (`i18n/data/ko.json`), so no Korean stands in the code.

export interface QuickEvent {
  title: string;
  start: Date;
  end: Date;
  allDay: boolean;
}

/** The words a language reads a phrase by. */
export interface QuickWords {
  relativeDays: Record<string, number>;
  weekdays: string[];
  weekdaySuffix: string;
  week: string;
  thisWeek: string[];
  nextWeek: string[];
  afternoon: string[];
  morning: string[];
  midday: string[];
  hour: string;
  minute: string;
  half: string;
  hours: string;
  during: string;
  month: string;
  day: string;
  allDay: string[];
  /** Particles a time or a day carries after it, cut out with it. */
  trailing: string[];
}

export const KOREAN_WORDS: QuickWords = KO_DATA.quick;

interface Found {
  from: number;
  to: number;
}

const ENGLISH_WEEKDAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
const ENGLISH_RELATIVE_DAYS: Record<string, number> = { today: 0, tonight: 0, tomorrow: 1 };

// Words a phrase spends leading into a time, which the title is not left holding.
const CONNECTORS = new Set(["at", "on", "from", "to", "until", "by", "the", "this", "next", "and", "-", "~", "@"]);

function escaped(word: string): string {
  return word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\s+/g, "\\s*");
}

function anyOf(words: readonly string[]): string {
  return [...words].sort((a, b) => b.length - a.length).map(escaped).join("|");
}

function found(match: RegExpExecArray | RegExpMatchArray): Found {
  return { from: match.index!, to: match.index! + match[0].length };
}

/** The patterns a language's words make, built once per set of words. */
function patterns(words: QuickWords) {
  const trailing = `(?:${anyOf(words.trailing)})?`;
  const marks = [...words.afternoon, ...words.morning, ...words.midday];
  const hour = escaped(words.hour);
  const hours = escaped(words.hours);
  const minute = escaped(words.minute);
  const day = escaped(words.day);
  const month = escaped(words.month);
  const during = escaped(words.during);
  // An hour word that begins a length word, the hour inside "two hours", is not a clock.
  const notLength = words.hours.startsWith(words.hour) && words.hours.length > words.hour.length ? `(?!${escaped(words.hours.slice(words.hour.length))})` : "";

  return {
    clock: new RegExp(
      `(${anyOf(marks)}|am|pm)?\\s*(?:(\\d{1,2})\\s*${hour}${notLength}(?:\\s*(\\d{1,2})\\s*${minute}|\\s*(${escaped(words.half)}))?|(\\d{1,2}):(\\d{2})|(\\d{1,2})(?::(\\d{2}))?\\s*(am|pm))${trailing}`,
      "gi"
    ),
    iso: new RegExp(`(\\d{4})[-.](\\d{1,2})[-.](\\d{1,2})(?:${day})?${trailing}`),
    monthDay: new RegExp(`(\\d{1,2})\\s*${month}\\s*(\\d{1,2})\\s*${day}${trailing}|(?<![\\d:])(\\d{1,2})/(\\d{1,2})(?![\\d:])`),
    relative: new RegExp(`(${anyOf([...Object.keys(words.relativeDays), ...Object.keys(ENGLISH_RELATIVE_DAYS)])})${trailing}`, "i"),
    weekday: new RegExp(`(?:(${anyOf([...words.thisWeek, ...words.nextWeek])})\\s*${escaped(words.week)}\\s*)?(${anyOf(words.weekdays)})${escaped(words.weekdaySuffix)}${trailing}`),
    englishWeekday: new RegExp(`(next\\s+)?(${ENGLISH_WEEKDAYS.join("|")})`, "i"),
    dayOnly: new RegExp(`(?<![\\d${month}])(\\d{1,2})\\s*${day}${trailing}(?!\\s*(?:${during}))`),
    allDay: new RegExp(`(${anyOf(words.allDay)}|all[\\s-]?day)`, "i"),
    length: new RegExp(`(\\d{1,2})\\s*${hours}(?:\\s*(\\d{1,2})\\s*${minute})?(?:\\s*${during})?|(\\d{1,3})\\s*${minute}\\s*${during}|for\\s+(\\d{1,2})\\s*(hours?|h|minutes?|min)`, "i")
  };
}

/** The hour a marked or unmarked clock reading stands for, on a 24-hour clock. */
function hourOf(words: QuickWords, hour: number, marker: string | undefined): number {
  const mark = (marker ?? "").toLowerCase();

  if (mark === "pm" || words.afternoon.includes(mark)) return hour < 12 ? hour + 12 : hour;
  if (mark === "am" || words.morning.includes(mark)) return hour === 12 ? 0 : hour;
  if (words.midday.includes(mark)) return hour < 6 ? hour + 12 : hour;

  // An hour from one to six said with no mark is the afternoon's: nobody books three o'clock at night.
  return hour >= 1 && hour <= 6 ? hour + 12 : hour;
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
export function parseQuickEvent(phrase: string, now: Date = new Date(), words: QuickWords = KOREAN_WORDS): QuickEvent | null {
  const text = phrase.trim();

  if (!text) return null;

  const read = patterns(words);
  const today = startOfDay(now);
  const cuts: Found[] = [];

  // The clocks: 3pm, 15:30, and the language's own hour words with their marks.
  const clocks: number[] = [];

  for (const match of text.matchAll(read.clock)) {
    let hour: number;
    let minute: number;

    if (match[2] !== undefined) {
      hour = hourOf(words, Number(match[2]), match[1]);
      minute = match[4] !== undefined ? 30 : Number(match[3] ?? 0);
    } else if (match[5] !== undefined) {
      hour = match[1] ? hourOf(words, Number(match[5]), match[1]) : Number(match[5]);
      minute = Number(match[6]);
    } else {
      hour = hourOf(words, Number(match[7]), match[9]);
      minute = Number(match[8] ?? 0);
    }

    if (hour > 23 || minute > 59) continue;

    clocks.push(hour * 60 + minute);
    cuts.push(found(match));
  }

  // The day, the first way of naming one that the phrase holds.
  let date: Date | null = null;
  const iso = read.iso.exec(text);
  const monthDay = iso ? null : read.monthDay.exec(text);
  const relative = iso || monthDay ? null : read.relative.exec(text);
  const weekday = iso || monthDay || relative ? null : read.weekday.exec(text);
  const englishWeekday = iso || monthDay || relative || weekday ? null : read.englishWeekday.exec(text);
  const dayOnly = iso || monthDay || relative || weekday || englishWeekday ? null : read.dayOnly.exec(text);

  if (iso) {
    date = new Date(Number(iso[1]), Number(iso[2]) - 1, Number(iso[3]));
    cuts.push(found(iso));
  } else if (monthDay) {
    const month = Number(monthDay[1] ?? monthDay[3]) - 1;
    const day = Number(monthDay[2] ?? monthDay[4]);

    date = new Date(today.getFullYear(), month, day);

    if (date < today) date = new Date(today.getFullYear() + 1, month, day);

    cuts.push(found(monthDay));
  } else if (relative) {
    const word = relative[1]!;

    date = addDays(today, words.relativeDays[word] ?? ENGLISH_RELATIVE_DAYS[word.toLowerCase()] ?? 0);
    cuts.push(found(relative));
  } else if (weekday) {
    const target = words.weekdays.indexOf(weekday[2]!);
    const weekStart = addDays(today, -today.getDay());

    if (weekday[1] && words.nextWeek.includes(weekday[1])) date = addDays(weekStart, 7 + target);
    else if (weekday[1]) date = addDays(weekStart, target);
    else date = addDays(today, (target - today.getDay() + 7) % 7);

    cuts.push(found(weekday));
  } else if (englishWeekday) {
    const target = ENGLISH_WEEKDAYS.indexOf(englishWeekday[2]!.toLowerCase());
    const ahead = (target - today.getDay() + 7) % 7;

    date = addDays(today, englishWeekday[1] ? ahead + 7 : ahead);
    cuts.push(found(englishWeekday));
  } else if (dayOnly) {
    const day = Number(dayOnly[1]);

    date = new Date(today.getFullYear(), today.getMonth(), day);

    if (date < today) date = new Date(today.getFullYear(), today.getMonth() + 1, day);

    cuts.push(found(dayOnly));
  }

  const allDay = read.allDay.exec(text);
  const length = read.length.exec(text);

  if (allDay) cuts.push(found(allDay));

  let minutes: number | null = null;

  if (length) {
    cuts.push(found(length));

    if (length[3] !== undefined) minutes = Number(length[3]);
    else if (length[4] !== undefined) minutes = Number(length[4]) * (length[5]!.toLowerCase().startsWith("h") ? 60 : 1);
    else minutes = Number(length[1]) * 60 + Number(length[2] ?? 0);
  }

  if (!date && clocks.length === 0 && !allDay) return null;

  const title = titleWithout(text, cuts);
  let day = date ?? today;

  if (allDay || clocks.length === 0) return { title, start: day, end: addDays(day, 1), allDay: true };

  const first = clocks[0]!;

  // A time already gone by today, with no day named, is tomorrow's.
  if (!date && addMinutes(day, first) < now) day = addDays(day, 1);

  const start = addMinutes(day, first);
  let end: Date;

  if (clocks[1] !== undefined) {
    end = addMinutes(day, clocks[1]);

    if (end <= start) end = addMinutes(end, 12 * 60);
    if (end <= start) end = addMinutes(start, 60);
  } else {
    end = addMinutes(start, minutes ?? 60);
  }

  return { title, start, end, allDay: false };
}
