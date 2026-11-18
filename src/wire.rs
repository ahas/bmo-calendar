//! The service's wire types (`docs/api.md`): what each route takes and answers, in the platform's format, each
//! with its JSON Schema for the OpenAPI document.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One of the two realms, never one for both (`docs/design.md`, 「Two realms, kept apart」); the organization
/// realm is one company's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Realm {
  /// The account's own calendars.
  Mine,
  /// A company's calendars, schedules, resources and bookings.
  Organization,
}

impl Realm {
  /// The word the path and the labels write it as.
  pub fn word(self) -> &'static str {
    match self {
      Self::Mine => "mine",
      Self::Organization => "organization",
    }
  }

  /// The realm a word names.
  pub fn of(word: &str) -> Option<Self> {
    match word {
      "mine" => Some(Self::Mine),
      "organization" => Some(Self::Organization),
      _ => None,
    }
  }
}

/// A calendar's colour: one of the console's six.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Color {
  Primary,
  Secondary,
  Info,
  Success,
  Warning,
  Error,
}

/// Who writes an organization calendar's events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Writers {
  /// The administrator alone.
  Administrator,
  /// Every account adds; the administrator and the event's creator change and cancel.
  Creator,
  /// Every account.
  Everyone,
}

/// Who is told an organization calendar's alarms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Tell {
  /// The event's creator.
  Creator,
  /// Every account of the calendar's company holding a `mine` realm; the owner in the platform's own realm.
  Everyone,
  /// Nobody.
  Nobody,
}

/// An event's alarm: none, or the minutes before its start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Alarm {
  #[serde(rename = "none")]
  None,
  #[serde(rename = "0")]
  AtStart,
  #[serde(rename = "5")]
  Five,
  #[serde(rename = "15")]
  Fifteen,
  #[serde(rename = "30")]
  Thirty,
  #[serde(rename = "60")]
  Hour,
  #[serde(rename = "1440")]
  Day,
}

impl Alarm {
  /// The minutes before the start, or none.
  pub fn minutes(self) -> Option<i64> {
    match self {
      Self::None => None,
      Self::AtStart => Some(0),
      Self::Five => Some(5),
      Self::Fifteen => Some(15),
      Self::Thirty => Some(30),
      Self::Hour => Some(60),
      Self::Day => Some(1440),
    }
  }
}

/// How an event repeats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
  None,
  Daily,
  Weekly,
  Monthly,
  Yearly,
}

/// A calendar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
  pub id: String,
  pub realm: Realm,
  /// The company's id, or `platform`; null in the `mine` realm.
  pub company: Option<String>,
  pub name: String,
  pub color: Color,
  /// Who writes its events; null in the `mine` realm.
  pub writers: Option<Writers>,
  /// Who is told its alarms; null in the `mine` realm.
  pub tell: Option<Tell>,
  /// Whether the caller's account hid it from its grid.
  pub hidden: bool,
  /// The account that made it.
  pub creator: String,
  pub created_at: i64,
}

/// A calendar to make.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewCalendar {
  pub name: String,
  pub color: Color,
  /// Organization only; `creator` where none is given.
  #[serde(default)]
  pub writers: Option<Writers>,
  /// Organization only; `creator` where none is given.
  #[serde(default)]
  pub tell: Option<Tell>,
}

/// A change of a calendar: the fields named.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CalendarChange {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub color: Option<Color>,
  #[serde(default)]
  pub writers: Option<Writers>,
  #[serde(default)]
  pub tell: Option<Tell>,
  /// Hides or shows it for the caller's account alone.
  #[serde(default)]
  pub hidden: Option<bool>,
}

/// An event, or one occurrence of a repeating event in a span's rows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Event {
  pub id: String,
  pub realm: Realm,
  /// The company's id, or `platform`; null in the `mine` realm.
  pub company: Option<String>,
  pub calendar_id: String,
  pub title: String,
  /// May be empty.
  pub description: String,
  /// Local ISO datetime, `YYYY-MM-DDTHH:mm:ss`.
  pub start: String,
  /// Local ISO datetime, after the start.
  pub end: String,
  pub all_day: bool,
  pub alarm: Alarm,
  pub repeat: Repeat,
  /// In a span's rows, the start of the occurrence a repeating event's row stands for; null otherwise.
  pub occurrence: Option<String>,
  /// The account that made it.
  pub creator: String,
  pub created_at: i64,
  pub changed_at: i64,
  /// The occurrence last told, or null.
  pub told: Option<String>,
}

/// An event to write.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewEvent {
  /// 1 to 64 letters, digits, `-` and `_`; one is made where none is given. A POST naming an id the realm
  /// already holds writes over that event and answers 200.
  #[serde(default)]
  pub id: Option<String>,
  pub calendar_id: String,
  /// One line.
  pub title: String,
  #[serde(default)]
  pub description: Option<String>,
  pub start: String,
  pub end: String,
  #[serde(default)]
  pub all_day: Option<bool>,
  #[serde(default)]
  pub alarm: Option<Alarm>,
  #[serde(default)]
  pub repeat: Option<Repeat>,
}

/// A change of an event: the fields named.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventChange {
  #[serde(default)]
  pub calendar_id: Option<String>,
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub start: Option<String>,
  #[serde(default)]
  pub end: Option<String>,
  #[serde(default)]
  pub all_day: Option<bool>,
  #[serde(default)]
  pub alarm: Option<Alarm>,
  #[serde(default)]
  pub repeat: Option<Repeat>,
  /// Limits the change to this occurrence of a repeating event, which becomes an event of its own.
  #[serde(default)]
  pub occurrence: Option<String>,
}

/// A page of members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Page<T> {
  pub items: Vec<T>,
  /// Whether anything stands past the last item.
  pub more: bool,
}

/// The account a service-token call acts for (`docs/api.md`, 「Realms」).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountQuery {
  /// The account, by its key: the one a service-token call acts for, or the one whose mine realm the
  /// administrator's console call reads; a member's is not read.
  #[serde(default)]
  pub account: Option<String>,
  /// The holder of a tool session, which acts for the account agency names as its owner.
  #[serde(default)]
  pub holder_name: Option<String>,
  /// The company an organization call reads: a company's id, or `platform`; the caller's own where none.
  #[serde(default)]
  pub company: Option<String>,
}

/// A span of events.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpanQuery {
  /// Local ISO datetime.
  pub start: String,
  /// Local ISO datetime, at most 92 days after the start.
  pub end: String,
  #[serde(default)]
  pub account: Option<String>,
  #[serde(default)]
  pub holder_name: Option<String>,
  #[serde(default)]
  pub company: Option<String>,
}

/// A cancellation of an event or one occurrence.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelQuery {
  /// The one occurrence cancelled, by its local start.
  #[serde(default)]
  pub occurrence: Option<String>,
  #[serde(default)]
  pub account: Option<String>,
  #[serde(default)]
  pub holder_name: Option<String>,
  #[serde(default)]
  pub company: Option<String>,
}

/// A span of alarm moments.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlarmsQuery {
  /// Epoch ms.
  pub from: i64,
  /// Epoch ms, at most 92 days after `from`.
  pub to: i64,
  #[serde(default)]
  pub account: Option<String>,
  #[serde(default)]
  pub holder_name: Option<String>,
  #[serde(default)]
  pub company: Option<String>,
}

/// One alarm moment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlarmMoment {
  pub realm: Realm,
  pub event_id: String,
  /// The occurrence's local start.
  pub occurrence: String,
  /// Epoch ms.
  pub at: i64,
  pub told: bool,
}

/// One alarm a beat told.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToldAlarm {
  pub realm: Realm,
  /// Null in the `mine` realm.
  pub company: Option<String>,
  pub event_id: String,
  pub occurrence: String,
  /// The alarm's moment, epoch ms.
  pub at: i64,
  /// How late it was told; 0 where it is on time.
  pub late_ms: i64,
  /// The rooms it was told in.
  pub rooms: Vec<i64>,
}

/// One booking telling a beat made (`docs/api.md`, 「Resources and bookings」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToldBooking {
  pub company: String,
  pub booking_id: String,
  /// The instruction's kind: `booking:<bookingId>:requested:<requestedAt>` or
  /// `booking:<bookingId>:decided:<decidedAt>`.
  pub kind: String,
  /// The rooms it was told in.
  pub rooms: Vec<i64>,
}

/// What a beat told.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Beat {
  pub told: Vec<ToldAlarm>,
  pub bookings: Vec<ToldBooking>,
}

/// `GET /health`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Health {
  pub ready: bool,
  /// Null when ready.
  pub reason: Option<String>,
  pub version: String,
}

/// A failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WireError {
  /// kebab-case.
  pub code: String,
  /// An English sentence.
  pub error: String,
}

/// One event the service publishes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StreamEvent {
  pub id: i64,
  pub kind: String,
  pub at: i64,
  pub data: Value,
}

/// Which events to read.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventsQuery {
  /// The last event number seen.
  #[serde(default)]
  pub after: Option<i64>,
  /// Comma-separated kinds.
  #[serde(default)]
  pub kinds: Option<String>,
  /// The most events a page holds, 50 where none is given, at most 500.
  #[serde(default)]
  pub limit: Option<i64>,
}

/// `calendar.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CalendarChanged {
  pub realm: Realm,
  pub company: Option<String>,
  pub calendar_id: String,
}

/// `event.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventChanged {
  pub realm: Realm,
  pub company: Option<String>,
  pub event_id: String,
  /// Null where the change is the whole event's.
  pub occurrence: Option<String>,
}

/// `alarm.told`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlarmTold {
  pub realm: Realm,
  pub company: Option<String>,
  pub event_id: String,
  pub occurrence: String,
  pub at: i64,
}

/// `source.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceChanged {
  pub source_id: String,
}

/// `schedule.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleChanged {
  pub company: String,
  pub source_id: String,
  pub item_id: String,
}

/// A source another unit writes its schedules under (`docs/api.md`, 「Sources and schedules」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Source {
  pub id: String,
  pub name: String,
  pub color: Color,
  /// Whether the caller's account hid its schedules in the realm read.
  pub hidden: bool,
  pub created_at: i64,
}

/// A source's name and colour, as its unit sets them.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceWrite {
  /// One line.
  pub name: String,
  /// `secondary` where none is given.
  #[serde(default)]
  pub color: Option<Color>,
}

/// A change of a source for the caller's account.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceChange {
  /// Hides or shows its schedules in the realm of the path, for the caller's account alone.
  #[serde(default)]
  pub hidden: Option<bool>,
}

/// A schedule as its unit writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleWrite {
  /// One line, not blank.
  pub title: String,
  /// Local ISO datetime, `YYYY-MM-DDTHH:mm:ss`.
  pub start: String,
  /// Local ISO datetime, after the start.
  pub end: String,
  #[serde(default)]
  pub all_day: Option<bool>,
  /// Where the item it stands for is opened: a console path, or an `http` or `https` address.
  pub link: String,
  /// The account key of the person the item is assigned to; null, empty or absent where none.
  #[serde(default)]
  pub assignee: Option<String>,
  /// The id of the company the item belongs to, as the account service names its unit; null, empty or absent
  /// where it belongs to none, which the platform administrator alone reads.
  #[serde(default)]
  pub company: Option<String>,
}

/// One item of a source's whole set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleItem {
  /// 1 to 64 letters, digits, `-` and `_`.
  pub item_id: String,
  #[serde(flatten)]
  pub schedule: ScheduleWrite,
}

/// A source's whole set of schedules.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSet {
  pub items: Vec<ScheduleItem>,
}

/// What a whole set did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Reconciled {
  /// Schedules made or changed.
  pub written: usize,
  /// Schedules that already stood as sent.
  pub unchanged: usize,
  /// Standing schedules the set did not name, removed.
  pub removed: usize,
}

/// A schedule as a read answers it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
  pub source_id: String,
  pub item_id: String,
  /// The realm it is read in.
  pub realm: Realm,
  /// The company it belongs to, or `platform` where it was sent with none.
  pub company: String,
  pub title: String,
  /// Local ISO datetime.
  pub start: String,
  /// Local ISO datetime, after the start.
  pub end: String,
  pub all_day: bool,
  pub link: String,
  /// Null where it has none.
  pub assignee: Option<String>,
  pub created_at: i64,
  pub changed_at: i64,
}

/// `resource.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChanged {
  pub company: String,
  pub resource_id: String,
}

/// `booking.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookingChanged {
  pub company: String,
  pub booking_id: String,
  pub booker: String,
}

/// `booking.decided`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookingDecided {
  pub company: String,
  pub booking_id: String,
  pub state: BookingState,
  pub title: String,
}

/// A company the caller reads (`docs/api.md`, 「Companies」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Company {
  /// The company's unit id, or `platform`.
  pub id: String,
  pub name: String,
  /// Whether it is the caller's own.
  pub own: bool,
  /// Whether the caller writes in its organization realm: its own, and for the administrator the platform's own.
  pub writes: bool,
  /// Whether the caller makes, changes and retires its resources: the administrator and a head of the company,
  /// each in their own company.
  pub manages: bool,
}

/// A company's resource: a room or anything else it books (`docs/api.md`, 「Resources and bookings」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
  pub id: String,
  pub company: String,
  pub name: String,
  /// The word the company groups it under.
  pub kind: String,
  /// Where it stands, in the company's own words.
  pub place: String,
  pub seats: u32,
  /// Whether a booking of it waits for a keeper.
  pub approval: bool,
  /// The account keys of the people who keep it.
  pub keepers: Vec<String>,
  pub creator: String,
  pub created_at: i64,
  pub changed_at: i64,
}

/// A resource to make.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewResource {
  /// 1 to 64 letters, digits, `-` and `_`; one is made where none is given.
  #[serde(default)]
  pub id: Option<String>,
  /// One line, not blank.
  pub name: String,
  /// One line, not blank.
  pub kind: String,
  /// One line.
  #[serde(default)]
  pub place: String,
  /// 1 to 10000.
  pub seats: u32,
  #[serde(default)]
  pub approval: bool,
  /// At least one where `approval` is true.
  #[serde(default)]
  pub keepers: Vec<String>,
}

/// A change of a resource: the fields named.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChange {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub kind: Option<String>,
  #[serde(default)]
  pub place: Option<String>,
  #[serde(default)]
  pub seats: Option<u32>,
  #[serde(default)]
  pub approval: Option<bool>,
  #[serde(default)]
  pub keepers: Option<Vec<String>>,
}

/// Where a booking stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BookingState {
  /// It waits for a keeper; it holds its span.
  Requested,
  /// It stands; it holds its span.
  Booked,
  /// A keeper refused it; it holds no span and leaves once its booker is told.
  Refused,
}

/// A booking, or one occurrence of a repeating booking in a span's rows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Booking {
  pub id: String,
  /// The realm it is read in.
  pub realm: Realm,
  pub company: String,
  pub resource_id: String,
  pub title: String,
  /// Local ISO datetime, `YYYY-MM-DDTHH:mm:ss`.
  pub start: String,
  /// Local ISO datetime, after the start.
  pub end: String,
  pub all_day: bool,
  pub repeat: Repeat,
  /// In a span's rows, the start of the occurrence a repeating booking's row stands for; null otherwise.
  pub occurrence: Option<String>,
  /// The account that booked it.
  pub booker: String,
  pub state: BookingState,
  /// The keeper who approved or refused it, or null.
  pub decided_by: Option<String>,
  /// When it was approved or refused, epoch ms, or null.
  pub decided_at: Option<i64>,
  /// The keeper's words with a refusal, or null.
  pub reason: Option<String>,
  pub created_at: i64,
  pub changed_at: i64,
}

/// A booking to make.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewBooking {
  /// 1 to 64 letters, digits, `-` and `_`; one is made where none is given. A POST naming an id the company
  /// already holds writes over that booking and answers 200.
  #[serde(default)]
  pub id: Option<String>,
  pub resource_id: String,
  /// One line, not blank.
  pub title: String,
  pub start: String,
  pub end: String,
  #[serde(default)]
  pub all_day: Option<bool>,
  #[serde(default)]
  pub repeat: Option<Repeat>,
}

/// A change of a booking: the fields named.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookingChange {
  #[serde(default)]
  pub resource_id: Option<String>,
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub start: Option<String>,
  #[serde(default)]
  pub end: Option<String>,
  #[serde(default)]
  pub all_day: Option<bool>,
  #[serde(default)]
  pub repeat: Option<Repeat>,
  /// Limits the change to this occurrence of a repeating booking, which becomes a booking of its own.
  #[serde(default)]
  pub occurrence: Option<String>,
}

/// A keeper's answer to a requested booking.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
  pub approve: bool,
  /// The keeper's words, with a refusal.
  #[serde(default)]
  pub reason: Option<String>,
}

/// The resources asked for.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesQuery {
  /// Only the resources of this kind.
  #[serde(default)]
  pub kind: Option<String>,
  #[serde(default)]
  pub account: Option<String>,
  #[serde(default)]
  pub holder_name: Option<String>,
  #[serde(default)]
  pub company: Option<String>,
}

/// The resources free for a span.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FreeQuery {
  /// Local ISO datetime.
  pub start: String,
  /// Local ISO datetime, after the start.
  pub end: String,
  /// The fewest seats; 1 where none is given.
  #[serde(default)]
  pub seats: Option<u32>,
  /// How the span repeats; `none` where none is given.
  #[serde(default)]
  pub repeat: Option<Repeat>,
  #[serde(default)]
  pub kind: Option<String>,
  #[serde(default)]
  pub account: Option<String>,
  #[serde(default)]
  pub holder_name: Option<String>,
  #[serde(default)]
  pub company: Option<String>,
}

/// A span of bookings.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookingsQuery {
  /// Local ISO datetime.
  pub start: String,
  /// Local ISO datetime, at most 92 days after the start.
  pub end: String,
  /// Only the bookings of this resource, in the `organization` realm.
  #[serde(default)]
  pub resource_id: Option<String>,
  #[serde(default)]
  pub account: Option<String>,
  #[serde(default)]
  pub holder_name: Option<String>,
  #[serde(default)]
  pub company: Option<String>,
}
