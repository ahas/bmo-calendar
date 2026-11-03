//! The service's wire types (`docs/api.md`): what each route takes and answers, in the platform's format, each
//! with its JSON Schema for the OpenAPI document.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One of the two realms, never one for both (`docs/design.md`, 「Two realms, kept apart」).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Realm {
  /// The account's own calendars.
  Mine,
  /// The organization's calendars.
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
  /// The owner and every account holding a `mine` realm.
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
  /// The account, by its key.
  #[serde(default)]
  pub account: Option<String>,
  /// The holder of a tool session, which acts for the owner.
  #[serde(default)]
  pub holder_name: Option<String>,
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
  pub event_id: String,
  pub occurrence: String,
  /// The alarm's moment, epoch ms.
  pub at: i64,
  /// How late it was told; 0 where it is on time.
  pub late_ms: i64,
  /// The rooms it was told in.
  pub rooms: Vec<i64>,
}

/// What a beat told.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Beat {
  pub told: Vec<ToldAlarm>,
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
  pub calendar_id: String,
}

/// `event.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventChanged {
  pub realm: Realm,
  pub event_id: String,
  /// Null where the change is the whole event's.
  pub occurrence: Option<String>,
}

/// `alarm.told`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlarmTold {
  pub realm: Realm,
  pub event_id: String,
  pub occurrence: String,
  pub at: i64,
}

/// A role a reader holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
  Administrator,
  Member,
}

/// The console bundle's manifest (bmo-ui's `docs/console-bundles.md`, 「The manifest」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleManifest {
  pub name: String,
  pub title: String,
  pub icon: String,
  pub screen_path: String,
  pub service_name: String,
  pub bundle: String,
  pub stylesheet: String,
  pub words: String,
  pub api_prefix: String,
  pub role: Role,
  pub owner_key: bool,
  pub events: bool,
  pub websocket: bool,
  pub push_kinds: Vec<String>,
  pub calls: Vec<String>,
  pub relays: Vec<Value>,
  pub share_reads: Vec<String>,
}
