//! Resources and bookings as the brain keeps them (`docs/design.md`, 「What a calendar and an event are in the
//! brain」, 「Resources and bookings」): the memory read back into the API's members, the labels written, and
//! whether two bookings of one resource ever meet.

use chrono::{Datelike, NaiveDateTime};

use crate::model::{
  Labels, Memory, accounts_in, label_word, local_text, number, parse_local, skipped, spans_between,
};
use crate::wire::{Booking, BookingState, Realm, Repeat, Resource};

/// How far a repeating booking is checked from the later of two starts, in years: the span in which the
/// weekdays and the days of every month come round again, so a pair that would ever meet meets within it.
pub const CHECKED_YEARS: i32 = 28;

/// The most seats a resource has.
pub const MOST_SEATS: u32 = 10_000;

/// The key a resource is kept under.
pub fn resource_key(id: &str) -> String {
  format!("resource:{id}")
}

/// The key a booking is kept under.
pub fn booking_key(id: &str) -> String {
  format!("booking:{id}")
}

/// A label's words read as one of an enum's words.
fn word<T: for<'a> serde::Deserialize<'a>>(word: &str) -> Option<T> {
  serde_json::from_value(serde_json::Value::String(word.to_owned())).ok()
}

/// A resource read from its memory, kept in a company's brain.
pub fn resource_of(memory: &Memory, company: &str) -> Option<Resource> {
  Some(Resource {
    id: memory.own_id()?.to_owned(),
    company: company.to_owned(),
    name: memory.text.clone(),
    kind: memory.label("kind").to_owned(),
    place: memory.label("place").to_owned(),
    seats: memory.label("seats").parse().unwrap_or(1),
    approval: memory.label("approval") == "true",
    keepers: accounts_in(memory, "keepers"),
    creator: memory.label("creator").to_owned(),
    created_at: number(memory, "createdAt"),
    changed_at: memory.created_at,
  })
}

/// The labels of a resource.
pub fn resource_labels(resource: &Resource) -> Labels {
  let mut labels = Labels::new();
  labels.insert("kind".into(), resource.kind.clone());
  labels.insert("place".into(), resource.place.clone());
  labels.insert("seats".into(), resource.seats.to_string());
  labels.insert("approval".into(), resource.approval.to_string());
  labels.insert("keepers".into(), resource.keepers.join(","));
  labels.insert("creator".into(), resource.creator.clone());
  labels.insert("createdAt".into(), resource.created_at.to_string());
  labels
}

/// A booking read from its memory, whole, as it is read in a realm.
pub fn booking_of(memory: &Memory, realm: Realm, company: &str) -> Option<Booking> {
  let optional = |name: &str| Some(memory.label(name)).filter(|text| !text.is_empty()).map(str::to_owned);
  Some(Booking {
    id: memory.own_id()?.to_owned(),
    realm,
    company: company.to_owned(),
    resource_id: memory.label("resource").to_owned(),
    title: memory.text.clone(),
    start: memory.label("start").to_owned(),
    end: memory.label("end").to_owned(),
    all_day: memory.label("allDay") == "true",
    repeat: word(memory.label("repeat")).unwrap_or(Repeat::None),
    occurrence: None,
    booker: memory.label("booker").to_owned(),
    state: word(memory.label("state")).unwrap_or(BookingState::Booked),
    decided_by: optional("decidedBy"),
    decided_at: optional("decidedAt").and_then(|at| at.parse().ok()),
    reason: optional("reason"),
    created_at: number(memory, "createdAt"),
    changed_at: memory.created_at,
  })
}

/// The labels of a booking: its own, its skipped occurrences, and the moment it last came to stand requested.
pub fn booking_labels(booking: &Booking, skip: &[String], requested_at: Option<i64>) -> Labels {
  let mut labels = Labels::new();
  labels.insert("resource".into(), booking.resource_id.clone());
  labels.insert("start".into(), booking.start.clone());
  labels.insert("end".into(), booking.end.clone());
  labels.insert("allDay".into(), booking.all_day.to_string());
  labels.insert("repeat".into(), label_word(&booking.repeat));
  labels.insert("skip".into(), skip.join(","));
  labels.insert("booker".into(), booking.booker.clone());
  labels.insert("state".into(), label_word(&booking.state));
  labels.insert("decidedBy".into(), booking.decided_by.clone().unwrap_or_default());
  labels.insert("decidedAt".into(), booking.decided_at.map(|at| at.to_string()).unwrap_or_default());
  labels.insert("reason".into(), booking.reason.clone().unwrap_or_default());
  labels.insert("requestedAt".into(), requested_at.map(|at| at.to_string()).unwrap_or_default());
  labels.insert("realm".into(), Realm::Organization.word().into());
  labels.insert("createdAt".into(), booking.created_at.to_string());
  labels
}

/// The moment a booking last came to stand requested, where it did.
pub fn requested_at(memory: &Memory) -> Option<i64> {
  memory.label("requestedAt").parse().ok()
}

/// Whether a booking holds its span: requested or booked.
pub fn holds_span(state: BookingState) -> bool {
  matches!(state, BookingState::Requested | BookingState::Booked)
}

/// A booking's times as the overlap check reads them: its start and end, its repeat and its skips.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Times {
  pub start: NaiveDateTime,
  pub end: NaiveDateTime,
  pub repeat: Repeat,
  pub skip: Vec<String>,
}

impl Times {
  /// A booking's times, where its start and end read.
  pub fn of(booking: &Booking, skip: Vec<String>) -> Option<Self> {
    Some(Self {
      start: parse_local(&booking.start)?,
      end: parse_local(&booking.end)?,
      repeat: booking.repeat,
      skip,
    })
  }

  /// A standing booking's times, from its memory.
  pub fn of_memory(memory: &Memory) -> Option<Self> {
    Some(Self {
      start: parse_local(memory.label("start"))?,
      end: parse_local(memory.label("end"))?,
      repeat: word(memory.label("repeat")).unwrap_or(Repeat::None),
      skip: skipped(memory),
    })
  }

  /// The moment after which no occurrence is looked at: the end of a booking that does not repeat, else
  /// [`CHECKED_YEARS`] after the start.
  fn horizon(&self, from: NaiveDateTime) -> NaiveDateTime {
    if self.repeat == Repeat::None {
      return self.end;
    }
    from
      .with_year(from.year() + CHECKED_YEARS)
      .or_else(|| (from + chrono::Duration::days(1)).with_year(from.year() + CHECKED_YEARS))
      .unwrap_or(from)
  }

  fn spans(&self, low: NaiveDateTime, high: NaiveDateTime) -> Vec<(NaiveDateTime, NaiveDateTime)> {
    spans_between((self.start, self.end), self.repeat, &self.skip, low, high)
  }
}

/// The first occurrence of `a` that meets an occurrence of `b`, by its start, where one does: spans are
/// half-open, so one may start where another ends.
pub fn first_meeting(a: &Times, b: &Times) -> Option<String> {
  let later = a.start.max(b.start);
  let low = a.start.min(b.start);
  let high = a.horizon(later).min(b.horizon(later));
  if high <= low {
    return None;
  }
  let ours = a.spans(low, high);
  let theirs = b.spans(low, high);
  let (mut i, mut j) = (0, 0);
  // Each list is in order of start, and of end too, since a series' occurrences are of one length.
  while i < ours.len() && j < theirs.len() {
    let (our_start, our_end) = ours[i];
    let (their_start, their_end) = theirs[j];
    if our_end <= their_start {
      i += 1;
    } else if their_end <= our_start {
      j += 1;
    } else {
      return Some(local_text(our_start));
    }
  }
  None
}
