import type { Alarm, CalendarColor, Repeat, Tell, Writers } from "./types";

// The choices the event form and the calendar settings offer, each a real value of the service's with the
// key its words are read by. The value the service takes where a write names none is the default, and is
// the one marked as the default where it is shown (`calendar.label.default_mark`).

export interface Choice<T> {
  value: T;
  label: string;
}

export const ALARM_CHOICES: readonly Choice<Alarm>[] = [
  { value: "none", label: "calendar.label.alarm_none" },
  { value: "0", label: "calendar.label.alarm_at_start" },
  { value: "5", label: "calendar.label.alarm_5m" },
  { value: "15", label: "calendar.label.alarm_15m" },
  { value: "30", label: "calendar.label.alarm_30m" },
  { value: "60", label: "calendar.label.alarm_1h" },
  { value: "1440", label: "calendar.label.alarm_1d" }
];

export const DEFAULT_ALARM: Alarm = "none";

export const REPEAT_CHOICES: readonly Choice<Repeat>[] = [
  { value: "none", label: "calendar.label.repeat_none" },
  { value: "daily", label: "calendar.label.repeat_daily" },
  { value: "weekly", label: "calendar.label.repeat_weekly" },
  { value: "monthly", label: "calendar.label.repeat_monthly" },
  { value: "yearly", label: "calendar.label.repeat_yearly" }
];

export const DEFAULT_REPEAT: Repeat = "none";

export const WRITERS_CHOICES: readonly Choice<Writers>[] = [
  { value: "administrator", label: "calendar.label.writers_administrator" },
  { value: "creator", label: "calendar.label.writers_creator" },
  { value: "everyone", label: "calendar.label.writers_everyone" }
];

export const DEFAULT_WRITERS: Writers = "creator";

export const TELL_CHOICES: readonly Choice<Tell>[] = [
  { value: "creator", label: "calendar.label.tell_creator" },
  { value: "everyone", label: "calendar.label.tell_everyone" },
  { value: "nobody", label: "calendar.label.tell_nobody" }
];

export const DEFAULT_TELL: Tell = "creator";

export const COLOR_CHOICES: readonly Choice<CalendarColor>[] = [
  { value: "primary", label: "calendar.label.color_primary" },
  { value: "secondary", label: "calendar.label.color_secondary" },
  { value: "info", label: "calendar.label.color_info" },
  { value: "success", label: "calendar.label.color_success" },
  { value: "warning", label: "calendar.label.color_warning" },
  { value: "error", label: "calendar.label.color_error" }
];

type Translate = (key: string, params?: Record<string, unknown>) => string;

/** The items a select offers for a setting: each value in its words, the default one marked. */
export function choiceItems<T extends string | number>(t: Translate, choices: readonly Choice<T>[], fallback: T): { label: string; value: T }[] {
  return choices.map((choice) => ({
    value: choice.value,
    label: choice.value === fallback ? t("calendar.label.default_mark", { value: t(choice.label) }) : t(choice.label)
  }));
}
