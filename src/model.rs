//! Calendars and events as the brain keeps them (`docs/design.md`, 「What a calendar and an event are in the
//! brain」): the memory read back into the API's members, the labels written, local datetimes, and the
//! occurrences of a repeating event.

use std::collections::BTreeMap;

use chrono::{Datelike, Duration, Local, LocalResult, Months, NaiveDate, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};

use crate::wire::{
  Alarm, Calendar, Color, Event, Realm, Repeat, Schedule, ScheduleWrite, Source, Tell, Writers,
};

/// The labels of a memory: the caller's own metadata.
pub type Labels = BTreeMap<String, String>;

/// The memory type a calendar is kept as.
pub const CALENDAR_TYPE: &str = "calendar";
/// The memory type an event is kept as.
pub const EVENT_TYPE: &str = "event";
/// The memory type a source is kept as.
pub const SOURCE_TYPE: &str = "source";
/// The memory type a received schedule is kept as.
pub const SCHEDULE_TYPE: &str = "schedule";
/// The memory type a company's resource is kept as.
pub const RESOURCE_TYPE: &str = "resource";
/// The memory type a booking of a resource is kept as.
pub const BOOKING_TYPE: &str = "booking";
/// The memory type the record of the move into companies is kept as.
pub const MIGRATION_TYPE: &str = "migration";

/// The longest assignee a schedule names, in characters: the longest Owner-Key a call carries.
const ASSIGNEE_CHARS: usize = 200;
/// The label that marks a schedule its assignee's alone.
const PRIVATE_LABEL: &str = "private";

/// The shape of a local datetime.
const LOCAL_SHAPE: &str = "%Y-%m-%dT%H:%M:%S";

/// One memory as the brain answers it, as much of it as the calendar reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
  pub id: i64,
  #[serde(default)]
  pub key: Option<String>,
  #[serde(rename = "type")]
  pub kind: String,
  pub text: String,
  pub status: String,
  #[serde(default)]
  pub valid_from: i64,
  #[serde(default)]
  pub labels: Labels,
  #[serde(default)]
  pub created_at: i64,
}

impl Memory {
  /// Whether the memory holds: `active` or `pending`.
  pub fn live(&self) -> bool {
    matches!(self.status.as_str(), "active" | "pending")
  }

  /// The calendar's, event's, source's, schedule's, resource's or booking's own id, the key less its type's
  /// prefix; a schedule's is its source's id and its item's, joined by a colon.
  pub fn own_id(&self) -> Option<&str> {
    let key = self.key.as_deref()?;
    match self.kind.as_str() {
      CALENDAR_TYPE => key.strip_prefix("calendar:"),
      EVENT_TYPE => key.strip_prefix("event:"),
      SOURCE_TYPE => key.strip_prefix("source:"),
      SCHEDULE_TYPE => key.strip_prefix("schedule:"),
      RESOURCE_TYPE => key.strip_prefix("resource:"),
      BOOKING_TYPE => key.strip_prefix("booking:"),
      _ => None,
    }
  }

  pub fn label(&self, name: &str) -> &str {
    self.labels.get(name).map(String::as_str).unwrap_or("")
  }
}

/// The key a calendar is kept under.
pub fn calendar_key(id: &str) -> String {
  format!("calendar:{id}")
}

/// The key an event is kept under.
pub fn event_key(id: &str) -> String {
  format!("event:{id}")
}

/// The key a source is kept under.
pub fn source_key(id: &str) -> String {
  format!("source:{id}")
}

/// A schedule's own id: its source's and its item's.
pub fn schedule_id(source: &str, item: &str) -> String {
  format!("{source}:{item}")
}

/// The key a schedule is kept under.
pub fn schedule_key(source: &str, item: &str) -> String {
  format!("schedule:{}", schedule_id(source, item))
}

/// Whether an id the caller names can stand in a key: 1 to 64 letters, digits, `-` and `_`.
pub fn valid_id(id: &str) -> bool {
  (1..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A fresh id: 32 hex digits from the process's random state and the moment.
pub fn fresh_id() -> String {
  use std::collections::hash_map::RandomState;
  use std::hash::{BuildHasher, Hasher};
  use std::sync::atomic::{AtomicU64, Ordering};

  static COUNT: AtomicU64 = AtomicU64::new(0);
  let count = COUNT.fetch_add(1, Ordering::Relaxed);
  let half = |salt: u64| {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(salt);
    hasher.write_i64(now_ms());
    hasher.write_u64(count);
    hasher.finish()
  };
  format!("{:016x}{:016x}", half(1), half(2))
}

/// This moment, epoch ms.
pub fn now_ms() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|since| since.as_millis() as i64)
    .unwrap_or(0)
}

/// A local datetime read from its ISO shape, `YYYY-MM-DDTHH:mm:ss`.
pub fn parse_local(text: &str) -> Option<NaiveDateTime> {
  if text.len() != 19 {
    return None;
  }
  NaiveDateTime::parse_from_str(text, LOCAL_SHAPE).ok()
}

/// A local datetime in its ISO shape.
pub fn local_text(at: NaiveDateTime) -> String {
  at.format(LOCAL_SHAPE).to_string()
}

/// The moment a local datetime names in the host's time zone, epoch ms; a wall-clock time a change of offset
/// skips is read an hour on.
pub fn epoch_of(at: NaiveDateTime) -> i64 {
  match Local.from_local_datetime(&at) {
    LocalResult::Single(moment) | LocalResult::Ambiguous(moment, _) => moment.timestamp_millis(),
    LocalResult::None => Local
      .from_local_datetime(&(at + Duration::hours(1)))
      .earliest()
      .map(|moment| moment.timestamp_millis())
      .unwrap_or_else(|| at.and_utc().timestamp_millis()),
  }
}

/// The local datetime of a moment in the host's time zone.
pub fn local_of(ms: i64) -> NaiveDateTime {
  Local
    .timestamp_millis_opt(ms)
    .single()
    .map(|moment| moment.naive_local())
    .unwrap_or_else(|| chrono::DateTime::from_timestamp_millis(ms).unwrap_or_default().naive_utc())
}

/// A label's word read as one of an enum's words.
fn word<T: for<'a> Deserialize<'a>>(word: &str) -> Option<T> {
  serde_json::from_value(serde_json::Value::String(word.to_owned())).ok()
}

/// An enum's word as a label writes it.
pub fn label_word<T: Serialize>(value: &T) -> String {
  serde_json::to_value(value)
    .ok()
    .and_then(|value| value.as_str().map(str::to_owned))
    .unwrap_or_default()
}

/// The accounts a `hidden` label names.
pub fn hidden_accounts(memory: &Memory) -> Vec<String> {
  accounts_in(memory, "hidden")
}

/// The accounts a label lists, joined by commas.
pub fn accounts_in(memory: &Memory, label: &str) -> Vec<String> {
  memory
    .label(label)
    .split(',')
    .filter(|key| !key.is_empty())
    .map(str::to_owned)
    .collect()
}

/// The occurrences a `skip` label names.
pub fn skipped(memory: &Memory) -> Vec<String> {
  memory
    .label("skip")
    .split(',')
    .filter(|one| !one.is_empty())
    .map(str::to_owned)
    .collect()
}

/// A label's number, 0 where it holds none.
pub fn number(memory: &Memory, name: &str) -> i64 {
  memory.label(name).parse().unwrap_or(0)
}

/// A calendar read from its memory in a realm and company, `hidden` for the account named.
pub fn calendar_of(
  memory: &Memory,
  realm: Realm,
  company: Option<String>,
  account: Option<&str>,
) -> Option<Calendar> {
  let id = memory.own_id()?.to_owned();
  let organization = realm == Realm::Organization;
  Some(Calendar {
    id,
    realm,
    company,
    name: memory.text.clone(),
    color: word(memory.label("color")).unwrap_or(Color::Primary),
    writers: organization.then(|| word(memory.label("writers")).unwrap_or(Writers::Creator)),
    tell: organization.then(|| word(memory.label("tell")).unwrap_or(Tell::Creator)),
    hidden: account.is_some_and(|account| hidden_accounts(memory).iter().any(|key| key == account)),
    creator: memory.label("creator").to_owned(),
    created_at: number(memory, "createdAt"),
  })
}

/// The labels of a calendar.
pub fn calendar_labels(calendar: &Calendar, hidden: &[String]) -> Labels {
  let mut labels = Labels::new();
  labels.insert("color".into(), label_word(&calendar.color));
  labels.insert("realm".into(), calendar.realm.word().into());
  if let Some(writers) = calendar.writers {
    labels.insert("writers".into(), label_word(&writers));
  }
  if let Some(tell) = calendar.tell {
    labels.insert("tell".into(), label_word(&tell));
  }
  labels.insert("hidden".into(), hidden.join(","));
  labels.insert("creator".into(), calendar.creator.clone());
  labels.insert("createdAt".into(), calendar.created_at.to_string());
  labels
}

/// An event read from its memory in a realm and company, whole: no occurrence.
pub fn event_of(
  memory: &Memory,
  realm: Realm,
  company: Option<String>,
  told: Option<String>,
) -> Option<Event> {
  let id = memory.own_id()?.to_owned();
  let (title, description) = match memory.text.split_once("\n\n") {
    Some((title, description)) => (title.to_owned(), description.to_owned()),
    None => (memory.text.clone(), String::new()),
  };
  Some(Event {
    id,
    realm,
    company,
    calendar_id: memory.label("calendar").to_owned(),
    title,
    description,
    start: memory.label("start").to_owned(),
    end: memory.label("end").to_owned(),
    all_day: memory.label("allDay") == "true",
    alarm: word(memory.label("alarm")).unwrap_or(Alarm::None),
    repeat: word(memory.label("repeat")).unwrap_or(Repeat::None),
    occurrence: None,
    creator: memory.label("creator").to_owned(),
    created_at: number(memory, "createdAt"),
    changed_at: memory.created_at,
    told,
  })
}

/// The memory text of an event: its title, then its description after a blank line where it has one.
pub fn event_text(event: &Event) -> String {
  if event.description.trim().is_empty() {
    event.title.clone()
  } else {
    format!("{}\n\n{}", event.title, event.description)
  }
}

/// The labels of an event.
pub fn event_labels(event: &Event, skip: &[String]) -> Labels {
  let mut labels = Labels::new();
  labels.insert("calendar".into(), event.calendar_id.clone());
  labels.insert("start".into(), event.start.clone());
  labels.insert("end".into(), event.end.clone());
  labels.insert("allDay".into(), event.all_day.to_string());
  labels.insert("alarm".into(), label_word(&event.alarm));
  labels.insert("repeat".into(), label_word(&event.repeat));
  labels.insert("creator".into(), event.creator.clone());
  labels.insert("realm".into(), event.realm.word().into());
  labels.insert("createdAt".into(), event.created_at.to_string());
  labels.insert("skip".into(), skip.join(","));
  labels
}

/// The `n`th repetition of a start, or none where the month has no such day.
fn nth(start: NaiveDateTime, repeat: Repeat, n: i64) -> Option<NaiveDateTime> {
  match repeat {
    Repeat::None => (n == 0).then_some(start),
    Repeat::Daily => start.checked_add_signed(Duration::days(n)),
    Repeat::Weekly => start.checked_add_signed(Duration::weeks(n)),
    Repeat::Monthly | Repeat::Yearly => {
      let months = if repeat == Repeat::Monthly { n } else { n * 12 };
      let date = start.date();
      let month = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)?
        .checked_add_months(Months::new(u32::try_from(months).ok()?))?;
      let day = NaiveDate::from_ymd_opt(month.year(), month.month(), date.day())?;
      Some(day.and_time(start.time()))
    }
  }
}

/// How many repetitions lie wholly before `low`, less one, as a place to start counting from.
fn first_index(start: NaiveDateTime, repeat: Repeat, low: NaiveDateTime) -> i64 {
  if low <= start {
    return 0;
  }
  let days = (low - start).num_days();
  let index = match repeat {
    Repeat::None => 0,
    Repeat::Daily => days,
    Repeat::Weekly => days / 7,
    Repeat::Monthly => {
      i64::from(low.year() - start.year()) * 12 + i64::from(low.month()) - i64::from(start.month())
    }
    Repeat::Yearly => i64::from(low.year() - start.year()),
  };
  (index - 1).max(0)
}

/// The occurrence starts of a series that fall in `[low, high)`, the skipped ones left out.
pub fn starts_between(
  start: NaiveDateTime,
  repeat: Repeat,
  skip: &[String],
  low: NaiveDateTime,
  high: NaiveDateTime,
) -> Vec<NaiveDateTime> {
  let mut starts = Vec::new();
  let mut index = first_index(start, repeat, low);
  loop {
    let next = nth(start, repeat, index);
    index += 1;
    match next {
      Some(at) if at >= high => break,
      Some(at) => {
        if at >= low && !skip.contains(&local_text(at)) {
          starts.push(at);
        }
      }
      None if repeat == Repeat::None || index > 100_000 => break,
      None => {}
    }
    if repeat == Repeat::None {
      break;
    }
  }
  starts
}

/// The occurrences of a series of this start, end, repeat and skips that overlap `[low, high)`, each its start
/// and end; a series that does not repeat is its one span.
pub fn spans_between(
  (start, end): (NaiveDateTime, NaiveDateTime),
  repeat: Repeat,
  skip: &[String],
  low: NaiveDateTime,
  high: NaiveDateTime,
) -> Vec<(NaiveDateTime, NaiveDateTime)> {
  let length = end - start;
  starts_between(start, repeat, skip, low - length, high)
    .into_iter()
    .map(|at| (at, at + length))
    .filter(|(at, until)| *until > low && *at < high)
    .collect()
}

/// Whether a local start is one of a series' occurrences.
pub fn is_occurrence(
  start: NaiveDateTime,
  repeat: Repeat,
  skip: &[String],
  occurrence: NaiveDateTime,
) -> bool {
  starts_between(start, repeat, skip, occurrence, occurrence + Duration::seconds(1)).contains(&occurrence)
}

/// The label a source keeps the accounts in that hid its schedules in a realm.
pub fn source_hidden_label(realm: Realm) -> &'static str {
  match realm {
    Realm::Mine => "hiddenMine",
    Realm::Organization => "hiddenOrganization",
  }
}

/// A source read from its memory, `hidden` for the account named in the realm named.
pub fn source_of(memory: &Memory, realm: Realm, account: Option<&str>) -> Option<Source> {
  let hidden = accounts_in(memory, source_hidden_label(realm));
  Some(Source {
    id: memory.own_id()?.to_owned(),
    name: memory.text.clone(),
    color: word(memory.label("color")).unwrap_or(Color::Secondary),
    hidden: account.is_some_and(|account| hidden.iter().any(|key| key == account)),
    created_at: number(memory, "createdAt"),
  })
}

/// The labels of a source: its colour, the moment it was made and the accounts that hid it in each realm,
/// the standing memory's kept where one stands.
pub fn source_labels(color: Color, created_at: i64, standing: Option<&Memory>) -> Labels {
  let mut labels = Labels::new();
  labels.insert("color".into(), label_word(&color));
  labels.insert("realm".into(), Realm::Organization.word().into());
  for realm in [Realm::Mine, Realm::Organization] {
    let name = source_hidden_label(realm);
    let hidden = standing.map(|memory| memory.label(name)).unwrap_or("");
    labels.insert(name.into(), hidden.to_owned());
  }
  labels.insert("createdAt".into(), created_at.to_string());
  labels
}

/// Why a schedule written does not stand: a field outside its shape, or its end not after its start.
#[derive(Debug)]
pub enum ScheduleFault {
  Shape(String),
  EndBeforeStart,
}

/// Whether a schedule written stands: its title one line and not blank, its end after its start, its link a
/// console path or an `http` or `https` address, its assignee no longer than an Owner-Key, its company an id, and
/// a private one naming both.
pub fn check_schedule(write: &ScheduleWrite) -> Result<(), ScheduleFault> {
  let shape = |why: &str| Err(ScheduleFault::Shape(why.to_owned()));
  if write.title.trim().is_empty() || write.title.contains('\n') {
    return shape("A title is one line, not blank.");
  }
  let (Some(start), Some(end)) = (parse_local(&write.start), parse_local(&write.end)) else {
    return shape("start and end are local ISO datetimes.");
  };
  if !valid_link(&write.link) {
    return shape("A link is a console path or an http or https address.");
  }
  if write
    .assignee
    .as_deref()
    .is_some_and(|assignee| assignee.chars().count() > ASSIGNEE_CHARS)
  {
    return shape("An assignee is at most 200 characters.");
  }
  if schedule_company(write).is_some_and(|company| !valid_id(company)) {
    return shape("A company is 1 to 64 letters, digits, '-' and '_'.");
  }
  if is_private(write) && (assignee_of(write).is_none() || schedule_company(write).is_none()) {
    return shape("A private schedule names its assignee and its company.");
  }
  if end <= start {
    return Err(ScheduleFault::EndBeforeStart);
  }
  Ok(())
}

/// Whether a link opens somewhere: a console path, one `/` and then the path, or an absolute `http` or
/// `https` address.
pub fn valid_link(link: &str) -> bool {
  if link.chars().any(|c| c.is_whitespace() || c.is_control()) {
    return false;
  }
  if link.starts_with('/') {
    // A second slash or a backslash after the first is read by a browser as another site's address.
    return !link.starts_with("//") && !link.starts_with("/\\");
  }
  reqwest::Url::parse(link).is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.has_host())
}

/// The assignee a write names, none where it names an empty one.
fn assignee_of(write: &ScheduleWrite) -> Option<&str> {
  write.assignee.as_deref().map(str::trim).filter(|assignee| !assignee.is_empty())
}

/// Whether a schedule written is its assignee's alone.
fn is_private(write: &ScheduleWrite) -> bool {
  write.private.unwrap_or(false)
}

/// The company a schedule written names, none where it names none or the platform's own word.
pub fn schedule_company(write: &ScheduleWrite) -> Option<&str> {
  write
    .company
    .as_deref()
    .map(str::trim)
    .filter(|company| !company.is_empty() && *company != crate::realm::PLATFORM)
}

/// The memory text of a schedule: its title.
pub fn schedule_text(write: &ScheduleWrite) -> String {
  write.title.trim().to_owned()
}

/// The labels of a schedule, `createdAt` the moment its first version was written.
pub fn schedule_labels(source: &str, item: &str, write: &ScheduleWrite, created_at: i64) -> Labels {
  let mut labels = Labels::new();
  labels.insert("source".into(), source.to_owned());
  labels.insert("item".into(), item.to_owned());
  labels.insert("start".into(), write.start.clone());
  labels.insert("end".into(), write.end.clone());
  labels.insert("allDay".into(), write.all_day.unwrap_or(false).to_string());
  labels.insert("link".into(), write.link.clone());
  labels.insert("assignee".into(), assignee_of(write).unwrap_or("").to_owned());
  // Only a private schedule carries the mark, so an open schedule's labels never change for it.
  if is_private(write) {
    labels.insert(PRIVATE_LABEL.into(), "true".into());
  }
  labels.insert("realm".into(), Realm::Organization.word().into());
  labels.insert("createdAt".into(), created_at.to_string());
  labels
}

/// Whether a schedule's memory is its assignee's alone.
pub fn schedule_private(memory: &Memory) -> bool {
  memory.label(PRIVATE_LABEL) == "true"
}

/// A schedule read from its memory, as it is read in a realm, kept in a company's brain or the platform's.
pub fn schedule_of(memory: &Memory, realm: Realm, company: String) -> Schedule {
  let assignee = memory.label("assignee");
  Schedule {
    source_id: memory.label("source").to_owned(),
    item_id: memory.label("item").to_owned(),
    realm,
    company,
    title: memory.text.clone(),
    start: memory.label("start").to_owned(),
    end: memory.label("end").to_owned(),
    all_day: memory.label("allDay") == "true",
    link: memory.label("link").to_owned(),
    assignee: (!assignee.is_empty()).then(|| assignee.to_owned()),
    private: schedule_private(memory),
    created_at: number(memory, "createdAt"),
    changed_at: memory.created_at,
  }
}
