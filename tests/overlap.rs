//! Whether two bookings of one resource ever meet (`docs/design.md`, 「Resources and bookings」): spans that
//! touch never meet, and repeating ones are walked occurrence by occurrence.

use bmo_calendar::booking::{Times, first_meeting};
use bmo_calendar::model::parse_local;
use bmo_calendar::wire::Repeat;

fn times(start: &str, end: &str, repeat: Repeat, skip: &[&str]) -> Times {
  Times {
    start: parse_local(start).unwrap(),
    end: parse_local(end).unwrap(),
    repeat,
    skip: skip.iter().map(|one| (*one).to_owned()).collect(),
  }
}

#[test]
fn spans_that_touch_never_meet_and_spans_that_cross_do() {
  let a = times("2026-03-02T09:00:00", "2026-03-02T10:00:00", Repeat::None, &[]);
  let touching = times("2026-03-02T10:00:00", "2026-03-02T11:00:00", Repeat::None, &[]);
  let crossing = times("2026-03-02T09:30:00", "2026-03-02T11:00:00", Repeat::None, &[]);
  assert_eq!(first_meeting(&a, &touching), None);
  assert_eq!(first_meeting(&a, &crossing).as_deref(), Some("2026-03-02T09:00:00"));
}

#[test]
fn a_weekly_series_meets_a_single_booking_weeks_later_but_not_on_a_skipped_week() {
  let weekly = times("2026-03-02T09:00:00", "2026-03-02T10:00:00", Repeat::Weekly, &[]);
  let later = times("2026-04-13T09:30:00", "2026-04-13T10:30:00", Repeat::None, &[]);
  assert_eq!(first_meeting(&weekly, &later).as_deref(), Some("2026-04-13T09:00:00"));
  let skipping =
    times("2026-03-02T09:00:00", "2026-03-02T10:00:00", Repeat::Weekly, &["2026-04-13T09:00:00"]);
  assert_eq!(first_meeting(&skipping, &later), None);
}

#[test]
fn a_monthly_series_meets_a_weekly_one_where_the_day_falls_on_its_weekday() {
  // The 15th of a month is a Sunday in 2026-03; a weekly Monday series meets it the first month the 15th is
  // a Monday, 2026-06-15.
  let monthly = times("2026-03-15T09:00:00", "2026-03-15T10:00:00", Repeat::Monthly, &[]);
  let mondays = times("2026-03-02T09:30:00", "2026-03-02T10:30:00", Repeat::Weekly, &[]);
  assert_eq!(first_meeting(&monthly, &mondays).as_deref(), Some("2026-06-15T09:00:00"));
}

#[test]
fn two_daily_series_at_different_hours_never_meet() {
  let morning = times("2026-03-02T09:00:00", "2026-03-02T10:00:00", Repeat::Daily, &[]);
  let afternoon = times("2026-03-05T14:00:00", "2026-03-05T15:00:00", Repeat::Daily, &[]);
  assert_eq!(first_meeting(&morning, &afternoon), None);
}
