//! The routes (`docs/api.md`): who calls (the service token or the console's credential with the reader's key
//! and role), the realms, the calendars, the events, the alarms, the beat, the event stream, the console bundle
//! and the health.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::sse::{Event as Sent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use chrono::{Duration, NaiveDateTime};
use futures_util::{StreamExt, stream};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::alarms;
use crate::app::App;
use crate::cache::RealmKey;
use crate::console::{Bundle, KEPT_FOR_GOOD};
use crate::error::ApiError;
use crate::events::Record;
use crate::model::{
  CALENDAR_TYPE, EVENT_TYPE, Memory, calendar_key, calendar_labels, calendar_of, epoch_of, event_key,
  event_labels, event_of, event_text, fresh_id, hidden_accounts, is_occurrence, local_text, now_ms,
  parse_local, skipped, starts_between, valid_id,
};
use crate::peers;
use crate::wire::{
  AccountQuery, Alarm, AlarmMoment, AlarmsQuery, Calendar, CalendarChange, CancelQuery, Color, Event,
  EventChange, EventsQuery, Health, NewCalendar, NewEvent, Page, Realm, Repeat, SpanQuery, Tell, Writers,
};

/// The longest span a read takes, in days.
pub const SPAN_DAYS: i64 = 92;

/// How often a comment line keeps a stream open.
const KEEP_OPEN: StdDuration = StdDuration::from_secs(15);

type Answer<T> = Result<T, ApiError>;

/// The service as the routes see it: its state and its console bundle, where one is built.
#[derive(Clone)]
pub struct Served {
  pub app: Arc<App>,
  pub bundle: Option<Arc<Bundle>>,
}

/// Every route, the unknown ones answered `no-route`.
pub fn router(served: Served) -> Router {
  Router::new()
    .route("/health", get(health))
    .route("/events", get(events))
    .route("/beats", post(beats))
    .route("/console-manifest", get(manifest))
    .route("/console/words", get(words))
    .route("/console/{file}", get(console_file))
    .route("/{realm}/calendars", get(list_calendars).post(create_calendar))
    .route(
      "/{realm}/calendars/{calendarId}",
      axum::routing::patch(change_calendar).delete(retire_calendar),
    )
    .route("/{realm}/events", get(list_events).post(create_event))
    .route("/{realm}/events/{eventId}", get(read_event).patch(change_event).delete(cancel_event))
    .route("/{realm}/alarms", get(list_alarms))
    .fallback(|| async { ApiError::no_route() })
    .method_not_allowed_fallback(|| async { ApiError::no_route() })
    .with_state(served)
}

/// Who calls.
#[derive(Clone, Debug)]
struct Caller {
  /// The service token's caller, rather than the console's.
  service: bool,
  /// The account the call names: the reader's key, or the service token's `account`.
  key: Option<String>,
  /// The tool session's holder a service-token call names.
  holder: Option<String>,
  /// Whether it acts as the administrator.
  administrator: bool,
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
  headers
    .get(header::AUTHORIZATION)?
    .to_str()
    .ok()?
    .strip_prefix("Bearer ")
    .map(str::trim)
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
  headers.get(name)?.to_str().ok().map(str::trim).filter(|value| !value.is_empty())
}

/// Whether the credential is the service's (true), the console's (false), or neither (401).
fn credential(app: &App, headers: &HeaderMap) -> Answer<bool> {
  let given = bearer(headers).ok_or_else(ApiError::unauthorized)?;
  let server = &app.settings.server.token;
  let console = &app.settings.console.token;
  if !server.is_empty() && given == server {
    Ok(true)
  } else if !console.is_empty() && given == console {
    Ok(false)
  } else {
    Err(ApiError::unauthorized())
  }
}

/// The caller of a route the screens use: the service, or the console with a reader's key and role.
fn caller(
  app: &App,
  headers: &HeaderMap,
  account: Option<String>,
  holder: Option<String>,
) -> Answer<Caller> {
  if credential(app, headers)? {
    return Ok(Caller {
      service: true,
      key: account.filter(|account| !account.is_empty()),
      holder: holder.filter(|holder| !holder.is_empty()),
      administrator: true,
    });
  }
  let key = header_text(headers, "owner-key")
    .ok_or_else(|| ApiError::forbidden("The call carries no Owner-Key."))?;
  if key.chars().count() > 200 {
    return Err(ApiError::forbidden("The Owner-Key is longer than 200 characters."));
  }
  let administrator = match header_text(headers, "owner-role") {
    Some("administrator") => true,
    Some("member") => false,
    _ => return Err(ApiError::forbidden("The call carries no Owner-Role of administrator or member.")),
  };
  Ok(Caller {
    service: false,
    key: Some(key.to_owned()),
    holder: None,
    administrator,
  })
}

/// The account a call acts for: its key, else its holder's (the owner's), else none.
async fn account_of(app: &App, caller: &Caller) -> Answer<Option<String>> {
  if let Some(key) = &caller.key {
    return Ok(Some(key.clone()));
  }
  if caller.holder.is_none() {
    return Ok(None);
  }
  match peers::owner_account(&app.http, &app.settings).await {
    Ok(Some(account)) => Ok(Some(account)),
    Ok(None) => Err(ApiError::no_account("The owner's sender is linked to no account.")),
    Err(fault) => Err(ApiError::no_account(format!("The holder's account could not be found: {fault}"))),
  }
}

/// A realm asked for: its key and the account the call acts for.
struct Asked {
  realm: RealmKey,
  account: Option<String>,
  caller: Caller,
}

async fn asked(
  app: &App,
  headers: &HeaderMap,
  realm: &str,
  account: Option<String>,
  holder: Option<String>,
) -> Answer<Asked> {
  let caller = caller(app, headers, account, holder)?;
  let realm =
    Realm::of(realm).ok_or_else(|| ApiError::not_found(format!("No realm is called {realm}.")))?;
  let account = account_of(app, &caller).await?;
  let realm = match realm {
    Realm::Mine => {
      RealmKey::Mine(account.clone().ok_or_else(|| ApiError::no_account("The call names no account."))?)
    }
    Realm::Organization => RealmKey::Organization,
  };
  Ok(Asked {
    realm,
    account,
    caller,
  })
}

fn actor(asked: &Asked) -> String {
  asked
    .account
    .as_ref()
    .map(|account| format!("account:{account}"))
    .unwrap_or_else(|| "calendar".to_owned())
}

fn query<T: DeserializeOwned>(given: Result<Query<T>, QueryRejection>) -> Answer<T> {
  given.map(|Query(query)| query).map_err(|why| ApiError::bad_request(why.body_text()))
}

fn body<T: DeserializeOwned>(bytes: &Bytes) -> Answer<T> {
  serde_json::from_slice(bytes)
    .map_err(|why| ApiError::bad_request(format!("The body does not read: {why}")))
}

fn json_answer(status: StatusCode, value: impl serde::Serialize) -> Response {
  (status, Json(value)).into_response()
}

// ---------------------------------------------------------------------------------------------------------
// Health

async fn health(State(served): State<Served>, headers: HeaderMap) -> Answer<Json<Health>> {
  credential(&served.app, &headers)?;
  let reason = served.app.unready();
  Ok(Json(Health {
    ready: reason.is_none(),
    reason,
    version: env!("CARGO_PKG_VERSION").to_owned(),
  }))
}

// ---------------------------------------------------------------------------------------------------------
// The cache, as the routes read it

/// A realm's live calendars and events as they stand now.
fn held(app: &App, realm: &RealmKey) -> Answer<(Vec<Memory>, Vec<Memory>)> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(
    cache
      .realms
      .get(realm)
      .map(|held| (held.calendars.values().cloned().collect(), held.events.values().cloned().collect()))
      .unwrap_or_default(),
  )
}

fn calendar_memory(app: &App, realm: &RealmKey, id: &str) -> Answer<Option<Memory>> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(cache.realms.get(realm).and_then(|held| held.calendars.get(id)).cloned())
}

fn event_memory(app: &App, realm: &RealmKey, id: &str) -> Answer<Option<Memory>> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(cache.realms.get(realm).and_then(|held| held.events.get(id)).cloned())
}

fn told(app: &App, brain: i64, event: &str) -> Answer<Option<String>> {
  app.store.last_told(brain, event).map_err(ApiError::failed)
}

/// The event a memory is, with its `told`.
fn event_answer(app: &App, brain: i64, realm: &RealmKey, memory: &Memory) -> Answer<Event> {
  let id = memory.own_id().unwrap_or_default();
  event_of(memory, realm.realm(), told(app, brain, id)?)
    .ok_or_else(|| ApiError::failed("an event without a key"))
}

// ---------------------------------------------------------------------------------------------------------
// Calendars

/// Whether the caller changes an organization calendar's settings: the administrator and its creator.
fn may_set(asked: &Asked, calendar: &Calendar) -> Answer<()> {
  if asked.realm == RealmKey::Organization
    && !asked.caller.administrator
    && asked.account.as_deref() != Some(calendar.creator.as_str())
  {
    return Err(ApiError::not_a_writer("Only the administrator and its creator change this calendar."));
  }
  Ok(())
}

async fn write_calendar(app: &Arc<App>, asked: &Asked, brain: i64, calendar: &Calendar) -> Answer<Memory> {
  let body = json!({
    "key": calendar_key(&calendar.id),
    "type": CALENDAR_TYPE,
    "text": calendar.name,
    "actorName": actor(asked),
    "labels": calendar_labels(calendar, &[]),
  });
  let written = app.brain()?.write(brain, &body).await?;
  app.take_in(&asked.realm, &written);
  Ok(written)
}

/// The calendar a realm is given where its brain holds no live one.
async fn first_calendar(app: &Arc<App>, asked: &Asked, brain: i64) -> Answer<()> {
  let (id, name, color) = match asked.realm {
    RealmKey::Mine(_) => ("mine", app.settings.calendars.mine_name.clone(), Color::Primary),
    RealmKey::Organization => {
      ("organization", app.settings.calendars.organization_name.clone(), Color::Info)
    }
  };
  let organization = asked.realm == RealmKey::Organization;
  let calendar = Calendar {
    id: id.to_owned(),
    realm: asked.realm.realm(),
    name,
    color,
    writers: organization.then_some(Writers::Creator),
    tell: organization.then_some(Tell::Creator),
    hidden: false,
    creator: asked.account.clone().unwrap_or_default(),
    created_at: now_ms(),
  };
  match write_calendar(app, asked, brain, &calendar).await {
    Ok(_) => Ok(()),
    Err(refused) if refused.error.contains("409") => {
      for memory in app.brain()?.memories(brain, CALENDAR_TYPE).await? {
        app.take_in(&asked.realm, &memory);
      }
      Ok(())
    }
    Err(refused) => Err(refused),
  }
}

async fn list_calendars(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<Json<Page<Calendar>>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let brain = app
    .realm(&asked.realm, true)
    .await?
    .ok_or_else(|| ApiError::failed("no brain was made"))?;
  if held(app, &asked.realm)?.0.is_empty() {
    first_calendar(app, &asked, brain).await?;
  }
  let mut calendars: Vec<Calendar> = held(app, &asked.realm)?
    .0
    .iter()
    .filter_map(|memory| calendar_of(memory, asked.realm.realm(), asked.account.as_deref()))
    .collect();
  calendars.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
  Ok(Json(Page {
    items: calendars,
    more: false,
  }))
}

fn one_line(text: &str, what: &str) -> Answer<String> {
  let text = text.trim();
  if text.is_empty() || text.contains('\n') {
    return Err(ApiError::bad_request(format!("The {what} is blank or more than one line.")));
  }
  Ok(text.to_owned())
}

async fn create_calendar(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Response> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let new: NewCalendar = body(&bytes)?;
  let organization = asked.realm == RealmKey::Organization;
  if organization && !asked.caller.administrator {
    return Err(ApiError::not_a_writer("Only the administrator makes an organization calendar."));
  }
  let calendar = Calendar {
    id: fresh_id(),
    realm: asked.realm.realm(),
    name: one_line(&new.name, "name")?,
    color: new.color,
    writers: organization.then(|| new.writers.unwrap_or(Writers::Creator)),
    tell: organization.then(|| new.tell.unwrap_or(Tell::Creator)),
    hidden: false,
    creator: asked.account.clone().unwrap_or_default(),
    created_at: now_ms(),
  };
  let brain = app
    .realm(&asked.realm, true)
    .await?
    .ok_or_else(|| ApiError::failed("no brain was made"))?;
  let written = write_calendar(app, &asked, brain, &calendar).await?;
  let answer = calendar_of(&written, asked.realm.realm(), asked.account.as_deref());
  Ok(json_answer(StatusCode::CREATED, answer))
}

async fn change_calendar(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Json<Calendar>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let change: CalendarChange = body(&bytes)?;
  let brain = app.realm(&asked.realm, false).await?;
  let (Some(brain), Some(memory)) = (brain, calendar_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The realm holds no live calendar {id}.")));
  };
  let standing = calendar_of(&memory, asked.realm.realm(), asked.account.as_deref())
    .ok_or_else(|| ApiError::failed("a calendar without a key"))?;
  let mut next = standing.clone();
  if change.name.is_some() || change.color.is_some() || change.writers.is_some() || change.tell.is_some() {
    may_set(&asked, &standing)?;
  }
  if let Some(name) = &change.name {
    next.name = one_line(name, "name")?;
  }
  if let Some(color) = change.color {
    next.color = color;
  }
  if asked.realm == RealmKey::Organization {
    next.writers = change.writers.or(next.writers);
    next.tell = change.tell.or(next.tell);
  }
  let mut hidden = hidden_accounts(&memory);
  if let Some(hide) = change.hidden {
    let account = asked
      .account
      .clone()
      .ok_or_else(|| ApiError::no_account("A hidden mark is an account's own."))?;
    hidden.retain(|key| *key != account);
    if hide {
      hidden.push(account);
    }
  }
  let labels = calendar_labels(&next, &hidden);
  if labels == memory.labels && next.name == memory.text {
    return Ok(Json(standing));
  }
  let body = json!({ "actorName": actor(&asked), "text": next.name, "labels": labels });
  let written = app.brain()?.version(brain, memory.id, &body).await?;
  app.take_in(&asked.realm, &written);
  calendar_of(&written, asked.realm.realm(), asked.account.as_deref())
    .map(Json)
    .ok_or_else(|| ApiError::failed("a calendar without a key"))
}

async fn retire_calendar(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<StatusCode> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let brain = app.realm(&asked.realm, false).await?;
  let (Some(brain), Some(memory)) = (brain, calendar_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The realm holds no live calendar {id}.")));
  };
  let standing =
    calendar_of(&memory, asked.realm.realm(), None).ok_or_else(|| ApiError::failed("no key"))?;
  may_set(&asked, &standing)?;
  let calls = app.brain()?;
  let actor = actor(&asked);
  let moved = calls.done(brain, memory.id, &actor, "retired").await?;
  app.take_in(&asked.realm, &moved);
  for event in held(app, &asked.realm)?.1.into_iter().filter(|event| event.label("calendar") == id) {
    let moved = calls.done(brain, event.id, &actor, "retired with its calendar").await?;
    app.take_in(&asked.realm, &moved);
  }
  Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------------------------------------
// Events

/// A span read from two local datetimes, at most 92 days long.
fn span(start: &str, end: &str) -> Answer<(NaiveDateTime, NaiveDateTime)> {
  let start = parse_local(start).ok_or_else(|| ApiError::bad_request("start is no local ISO datetime"))?;
  let end = parse_local(end).ok_or_else(|| ApiError::bad_request("end is no local ISO datetime"))?;
  if end <= start || end - start > Duration::days(SPAN_DAYS) {
    return Err(ApiError::span_too_long());
  }
  Ok((start, end))
}

/// The rows of one event that overlap a span: itself, or one per occurrence of a repeating event.
fn rows(event: &Event, memory: &Memory, low: NaiveDateTime, high: NaiveDateTime) -> Vec<Event> {
  let (Some(start), Some(end)) = (parse_local(&event.start), parse_local(&event.end)) else {
    return Vec::new();
  };
  let length = end - start;
  if event.repeat == Repeat::None {
    return if start < high && end > low { vec![event.clone()] } else { Vec::new() };
  }
  starts_between(start, event.repeat, &skipped(memory), low - length, high)
    .into_iter()
    .filter(|occurrence| *occurrence + length > low)
    .map(|occurrence| Event {
      start: local_text(occurrence),
      end: local_text(occurrence + length),
      occurrence: Some(local_text(occurrence)),
      ..event.clone()
    })
    .collect()
}

async fn list_events(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<SpanQuery>, QueryRejection>,
) -> Answer<Json<Page<Event>>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let (low, high) = span(&given.start, &given.end)?;
  let Some(brain) = app.realm(&asked.realm, false).await? else {
    return Ok(Json(Page {
      items: Vec::new(),
      more: false,
    }));
  };
  let mut items = Vec::new();
  for memory in held(app, &asked.realm)?.1 {
    let event = event_answer(app, brain, &asked.realm, &memory)?;
    items.extend(rows(&event, &memory, low, high));
  }
  items.sort_by(|a, b| (&a.start, &a.id).cmp(&(&b.start, &b.id)));
  Ok(Json(Page { items, more: false }))
}

async fn read_event(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<Json<Event>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let brain = app.realm(&asked.realm, false).await?;
  let (Some(brain), Some(memory)) = (brain, event_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The realm holds no live event {id}.")));
  };
  event_answer(app, brain, &asked.realm, &memory).map(Json)
}

/// Whether the caller writes an event under a calendar: always in its own realm; in the organization's by the
/// calendar's `writers`, a change of a standing event also by its creator.
fn may_write(asked: &Asked, calendar: &Memory, creator: Option<&str>) -> Answer<()> {
  if asked.realm != RealmKey::Organization || asked.caller.administrator {
    return Ok(());
  }
  let writers = calendar_of(calendar, Realm::Organization, None).and_then(|calendar| calendar.writers);
  match (writers, creator) {
    (Some(Writers::Everyone), _) | (Some(Writers::Creator), None) => Ok(()),
    (Some(Writers::Creator), Some(creator)) if asked.account.as_deref() == Some(creator) => Ok(()),
    _ => Err(ApiError::not_a_writer("This calendar does not take this caller's writes.")),
  }
}

/// The live calendar an event is written under, or `unknown-calendar`.
fn live_calendar(app: &App, asked: &Asked, id: &str) -> Answer<Memory> {
  calendar_memory(app, &asked.realm, id)?.ok_or_else(|| ApiError::unknown_calendar(id))
}

/// An event's start and end, the end after the start.
fn times(start: &str, end: &str) -> Answer<(NaiveDateTime, NaiveDateTime)> {
  let start = parse_local(start).ok_or_else(|| ApiError::bad_request("start is no local ISO datetime"))?;
  let end = parse_local(end).ok_or_else(|| ApiError::bad_request("end is no local ISO datetime"))?;
  if end <= start {
    return Err(ApiError::end_before_start());
  }
  Ok((start, end))
}

/// An event written: a first version, or a version over the memory standing at its key where one stands and
/// differs. Answers the memory as it stands after the write.
async fn put_event(
  app: &Arc<App>,
  asked: &Asked,
  brain: i64,
  event: &Event,
  skip: &[String],
  standing: Option<&Memory>,
) -> Answer<Memory> {
  let (start, _) = times(&event.start, &event.end)?;
  let labels = event_labels(event, skip);
  let text = event_text(event);
  let calls = app.brain()?;
  let written = match standing {
    Some(memory) if memory.labels == labels && memory.text == text => return Ok(memory.clone()),
    Some(memory) => {
      let body =
        json!({ "actorName": actor(asked), "text": text, "validFrom": epoch_of(start), "labels": labels });
      calls.version(brain, memory.id, &body).await?
    }
    None => {
      let body = json!({
        "key": event_key(&event.id),
        "type": EVENT_TYPE,
        "text": text,
        "validFrom": epoch_of(start),
        "actorName": actor(asked),
        "labels": labels,
      });
      calls.write(brain, &body).await?
    }
  };
  app.take_in(&asked.realm, &written);
  Ok(written)
}

async fn create_event(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Response> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let new: NewEvent = body(&bytes)?;
  let id = new.id.clone().unwrap_or_else(fresh_id);
  if !valid_id(&id) {
    return Err(ApiError::bad_request("An id is 1 to 64 letters, digits, '-' and '_'."));
  }
  let title = one_line(&new.title, "title")?;
  times(&new.start, &new.end)?;
  let brain = app
    .realm(&asked.realm, true)
    .await?
    .ok_or_else(|| ApiError::failed("no brain was made"))?;
  let calendar = live_calendar(app, &asked, &new.calendar_id)?;
  let standing = event_memory(app, &asked.realm, &id)?;
  let standing_event = standing.as_ref().and_then(|memory| event_of(memory, asked.realm.realm(), None));
  may_write(&asked, &calendar, standing_event.as_ref().map(|event| event.creator.as_str()))?;
  if let Some(event) = &standing_event
    && event.calendar_id != new.calendar_id
  {
    let from = live_calendar(app, &asked, &event.calendar_id)?;
    may_write(&asked, &from, Some(&event.creator))?;
  }
  let now = now_ms();
  let event = Event {
    id: id.clone(),
    realm: asked.realm.realm(),
    calendar_id: new.calendar_id.clone(),
    title,
    description: new.description.clone().unwrap_or_default(),
    start: new.start.clone(),
    end: new.end.clone(),
    all_day: new.all_day.unwrap_or(false),
    alarm: new.alarm.unwrap_or(Alarm::None),
    repeat: new.repeat.unwrap_or(Repeat::None),
    occurrence: None,
    creator: standing_event
      .as_ref()
      .map(|event| event.creator.clone())
      .unwrap_or_else(|| asked.account.clone().unwrap_or_default()),
    created_at: standing_event.as_ref().map(|event| event.created_at).unwrap_or(now),
    changed_at: now,
    told: None,
  };
  let skip = standing.as_ref().map(skipped).unwrap_or_default();
  let written = put_event(app, &asked, brain, &event, &skip, standing.as_ref()).await?;
  let status = if standing.is_some() { StatusCode::OK } else { StatusCode::CREATED };
  Ok(json_answer(status, event_answer(app, brain, &asked.realm, &written)?))
}

async fn change_event(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Json<Event>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let change: EventChange = body(&bytes)?;
  let brain = app.realm(&asked.realm, false).await?;
  let (Some(brain), Some(memory)) = (brain, event_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The realm holds no live event {id}.")));
  };
  let standing = event_of(&memory, asked.realm.realm(), None).ok_or_else(|| ApiError::failed("no key"))?;
  let from = live_calendar(app, &asked, &standing.calendar_id)?;
  may_write(&asked, &from, Some(&standing.creator))?;
  if let Some(calendar) = change.calendar_id.as_deref().filter(|calendar| *calendar != standing.calendar_id)
  {
    let to = live_calendar(app, &asked, calendar)?;
    may_write(&asked, &to, None)?;
  }
  let mut next = standing.clone();
  if let Some(calendar) = &change.calendar_id {
    next.calendar_id = calendar.clone();
  }
  if let Some(title) = &change.title {
    next.title = one_line(title, "title")?;
  }
  if let Some(description) = &change.description {
    next.description = description.clone();
  }
  next.all_day = change.all_day.unwrap_or(next.all_day);
  next.alarm = change.alarm.unwrap_or(next.alarm);
  let skip = skipped(&memory);
  let Some(occurrence) = change.occurrence.clone() else {
    next.start = change.start.clone().unwrap_or(next.start);
    next.end = change.end.clone().unwrap_or(next.end);
    next.repeat = change.repeat.unwrap_or(next.repeat);
    let written = put_event(app, &asked, brain, &next, &skip, Some(&memory)).await?;
    return event_answer(app, brain, &asked.realm, &written).map(Json);
  };
  // One occurrence changed on its own: the series skips it, and it stands as an event of its own.
  let (start, end) = times(&standing.start, &standing.end)?;
  let at = parse_local(&occurrence).ok_or_else(|| ApiError::not_an_occurrence(&occurrence))?;
  if standing.repeat == Repeat::None || !is_occurrence(start, standing.repeat, &skip, at) {
    return Err(ApiError::not_an_occurrence(&occurrence));
  }
  next.id = fresh_id();
  next.repeat = Repeat::None;
  next.start = change.start.clone().unwrap_or_else(|| local_text(at));
  next.end = change.end.clone().unwrap_or_else(|| local_text(at + (end - start)));
  next.created_at = now_ms();
  times(&next.start, &next.end)?;
  let mut series_skip = skip.clone();
  series_skip.push(occurrence);
  put_event(app, &asked, brain, &standing, &series_skip, Some(&memory)).await?;
  let written = put_event(app, &asked, brain, &next, &[], None).await?;
  event_answer(app, brain, &asked.realm, &written).map(Json)
}

async fn cancel_event(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<CancelQuery>, QueryRejection>,
) -> Answer<StatusCode> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let brain = app.realm(&asked.realm, false).await?;
  let (Some(brain), Some(memory)) = (brain, event_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The realm holds no live event {id}.")));
  };
  let standing = event_of(&memory, asked.realm.realm(), None).ok_or_else(|| ApiError::failed("no key"))?;
  if let Some(calendar) = calendar_memory(app, &asked.realm, &standing.calendar_id)? {
    may_write(&asked, &calendar, Some(&standing.creator))?;
  } else if asked.realm == RealmKey::Organization && !asked.caller.administrator {
    return Err(ApiError::not_a_writer("The event's calendar is retired."));
  }
  match given.occurrence {
    None => {
      let moved = app.brain()?.done(brain, memory.id, &actor(&asked), "cancelled").await?;
      app.take_in(&asked.realm, &moved);
    }
    Some(occurrence) => {
      let (start, _) = times(&standing.start, &standing.end)?;
      let skip = skipped(&memory);
      let at = parse_local(&occurrence).ok_or_else(|| ApiError::not_an_occurrence(&occurrence))?;
      if standing.repeat == Repeat::None || !is_occurrence(start, standing.repeat, &skip, at) {
        return Err(ApiError::not_an_occurrence(&occurrence));
      }
      let mut skip = skip;
      skip.push(occurrence);
      put_event(app, &asked, brain, &standing, &skip, Some(&memory)).await?;
    }
  }
  Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------------------------------------
// Alarms

async fn list_alarms(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AlarmsQuery>, QueryRejection>,
) -> Answer<Json<Page<AlarmMoment>>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  if given.to <= given.from || given.to - given.from > SPAN_DAYS * 86_400_000 {
    return Err(ApiError::span_too_long());
  }
  let Some(brain) = app.realm(&asked.realm, false).await? else {
    return Ok(Json(Page {
      items: Vec::new(),
      more: false,
    }));
  };
  let mut items = Vec::new();
  for memory in held(app, &asked.realm)?.1 {
    let event_id = memory.own_id().unwrap_or_default().to_owned();
    for moment in alarms::moments(&memory, given.from, given.to) {
      let occurrence = local_text(moment.occurrence);
      items.push(AlarmMoment {
        realm: asked.realm.realm(),
        told: app.store.is_told(brain, &event_id, &occurrence).map_err(ApiError::failed)?,
        event_id: event_id.clone(),
        occurrence,
        at: moment.at,
      });
    }
  }
  items.sort_by(|a, b| (a.at, &a.event_id).cmp(&(b.at, &b.event_id)));
  Ok(Json(Page { items, more: false }))
}

async fn beats(State(served): State<Served>, headers: HeaderMap) -> Answer<Json<crate::wire::Beat>> {
  if !credential(&served.app, &headers)? {
    return Err(ApiError::forbidden("A beat is run by the service token alone."));
  }
  if let Some(reason) = served.app.unready() {
    return Err(ApiError::not_ready(reason));
  }
  alarms::beat(&served.app).await.map(Json)
}

// ---------------------------------------------------------------------------------------------------------
// The event stream

fn sent(event: &crate::wire::StreamEvent) -> Sent {
  Sent::default()
    .event(event.kind.clone())
    .id(event.id.to_string())
    .json_data(event)
    .unwrap_or_else(|_| Sent::default().comment("an event that did not render"))
}

async fn events(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Query<EventsQuery>, QueryRejection>,
) -> Answer<Response> {
  let app = Arc::clone(&served.app);
  let given = query(given)?;
  let caller = caller(&app, &headers, None, None)?;
  let resumed = header_text(&headers, "last-event-id").and_then(|id| id.parse::<i64>().ok());
  let after = resumed.or(given.after);
  let kinds: Option<Vec<String>> = given
    .kinds
    .as_deref()
    .map(|kinds| kinds.split(',').map(str::trim).map(str::to_owned).collect());
  let visible = move |record: &Record| {
    let kind_wanted = kinds.as_ref().is_none_or(|kinds| kinds.contains(&record.event.kind));
    let reader_may = caller.service || record.account.is_none() || record.account == caller.key;
    kind_wanted && reader_may
  };
  let paged = header_text(&headers, "accept").is_some_and(|accepted| accepted.contains("application/json"));
  if paged {
    let limit = given.limit.unwrap_or(50);
    if !(1..=500).contains(&limit) {
      return Err(ApiError::bad_request("limit is 1 to 500"));
    }
    let (records, gone) = app.events.after(after.unwrap_or(app.events.first()));
    if gone && after.is_some() {
      return Err(ApiError::gone(
        "The events after that number are no longer kept; read the collections again.",
      ));
    }
    let records: Vec<_> = records.into_iter().filter(|record| visible(record)).collect();
    let more = records.len() > limit as usize;
    let items = records.into_iter().take(limit as usize).map(|record| record.event).collect();
    return Ok(Json(json!(Page::<crate::wire::StreamEvent> { items, more })).into_response());
  }
  let live = app.events.subscribe();
  let backlog = match after {
    Some(after) => {
      let (records, gone) = app.events.after(after);
      if gone {
        return Err(ApiError::gone(
          "The events after that number are no longer kept; read the collections again.",
        ));
      }
      records
    }
    None => Vec::new(),
  };
  let cursor = backlog
    .last()
    .map(|record| record.event.id)
    .or(after)
    .unwrap_or_else(|| app.events.latest());
  let backlog = stream::iter(backlog);
  let live = stream::unfold((live, cursor), |(mut live, cursor)| async move {
    loop {
      match live.recv().await {
        Ok(record) if record.event.id > cursor => {
          let id = record.event.id;
          return Some((record, (live, id)));
        }
        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
        Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
      }
    }
  });
  let stream = backlog
    .chain(live)
    .filter(move |record| std::future::ready(visible(record)))
    .map(|record| Ok::<_, Infallible>(sent(&record.event)))
    .take_until(app.events.stopped());
  Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_OPEN)).into_response())
}

// ---------------------------------------------------------------------------------------------------------
// The console bundle

fn bundle(served: &Served, headers: &HeaderMap) -> Answer<Arc<Bundle>> {
  credential(&served.app, headers)?;
  served
    .bundle
    .clone()
    .ok_or_else(|| ApiError::not_found("The console bundle is not built here."))
}

async fn manifest(State(served): State<Served>, headers: HeaderMap) -> Answer<Response> {
  Ok(Json(bundle(&served, &headers)?.manifest()).into_response())
}

async fn words(State(served): State<Served>, headers: HeaderMap) -> Answer<Response> {
  Ok(Json(bundle(&served, &headers)?.words.clone()).into_response())
}

async fn console_file(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(file): Path<String>,
) -> Answer<Response> {
  let bundle = bundle(&served, &headers)?;
  let served = bundle
    .file(&file)
    .ok_or_else(|| ApiError::not_found(format!("The bundle has no file {file}.")))?;
  Ok(
    (
      [(header::CONTENT_TYPE, served.content_type), (header::CACHE_CONTROL, KEPT_FOR_GOOD)],
      served.bytes.clone(),
    )
      .into_response(),
  )
}
