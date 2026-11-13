//! The sources and the schedules other units push (`docs/api.md`, 「Sources and schedules」): a unit's
//! writes under its own source, by the service token alone, kept in the organization realm's brain, and the
//! reads of them for both realms. A source is data: nothing here tells one source from another.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Json, Response};
use axum::routing::{get, patch, put};
use serde_json::json;

use crate::api::{
  Answer, Asked, Credential, Served, actor, asked, body, credential, json_answer, one_line, query, span,
};
use crate::app::App;
use crate::cache::{Held, RealmKey};
use crate::error::ApiError;
use crate::model::{
  Memory, SCHEDULE_TYPE, SOURCE_TYPE, ScheduleFault, accounts_in, check_schedule, epoch_of, now_ms,
  parse_local, schedule_id, schedule_key, schedule_labels, schedule_of, schedule_text, source_hidden_label,
  source_key, source_labels, source_of, valid_id,
};
use crate::wire::{
  AccountQuery, Color, Page, Realm, Reconciled, Schedule, ScheduleSet, ScheduleWrite, Source, SourceChange,
  SourceWrite, SpanQuery,
};

/// The reason a removed schedule's memory is closed with.
const REMOVED: &str = "removed by its source";

/// The routes of sources and schedules, under the same guard as the screens' others.
pub fn routes() -> Router<Served> {
  Router::new()
    .route("/sources/{sourceId}", put(put_source))
    .route("/sources/{sourceId}/schedules", put(put_set))
    .route("/sources/{sourceId}/schedules/{itemId}", put(put_one).delete(remove_one))
    .route("/{realm}/sources", get(list_sources))
    .route("/{realm}/sources/{sourceId}", patch(change_source))
    .route("/{realm}/schedules", get(list_schedules))
}

// ---------------------------------------------------------------------------------------------------------
// The organization realm, as these routes read and write it

/// The writes are the units' own: the service token's alone.
fn service_only(app: &App, headers: &HeaderMap) -> Answer<()> {
  if credential(app, headers)? != Credential::Service {
    return Err(ApiError::forbidden("Sources and schedules are written by the service token alone."));
  }
  Ok(())
}

/// An id a path names, in the shape a key takes.
fn checked_id<'a>(id: &'a str, what: &str) -> Answer<&'a str> {
  if valid_id(id) {
    Ok(id)
  } else {
    Err(ApiError::bad_request(format!("A {what} is 1 to 64 letters, digits, '-' and '_'.")))
  }
}

/// The organization realm's brain, made where none stands yet.
async fn organization_brain(app: &Arc<App>) -> Answer<i64> {
  app
    .realm(&RealmKey::Organization, true)
    .await?
    .ok_or_else(|| ApiError::failed("no brain was made"))
}

/// The live sources and schedules the organization realm holds now.
fn held(app: &App) -> Answer<(Vec<Memory>, Vec<Memory>)> {
  organization(app, |held| {
    (held.sources.values().cloned().collect(), held.schedules.values().cloned().collect())
  })
}

/// What the organization realm's cache answers, or the default where it holds no brain yet.
fn organization<T: Default>(app: &App, read: impl FnOnce(&Held) -> T) -> Answer<T> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(cache.realms.get(&RealmKey::Organization).map(read).unwrap_or_default())
}

fn source_memory(app: &App, source: &str) -> Answer<Option<Memory>> {
  organization(app, |held| held.sources.get(source).cloned())
}

fn schedule_memory(app: &App, source: &str, item: &str) -> Answer<Option<Memory>> {
  organization(app, |held| held.schedules.get(&schedule_id(source, item)).cloned())
}

/// The name a unit's writes are recorded under in the brain.
fn source_actor(source: &str) -> String {
  format!("source:{source}")
}

/// A memory written: a version over the standing one where one stands, else a first version under its type
/// and key; taken in.
async fn put_memory(
  app: &Arc<App>,
  brain: i64,
  (kind, key): (&str, String),
  standing: Option<&Memory>,
  mut body: serde_json::Value,
) -> Answer<Memory> {
  let calls = app.brain()?;
  let written = match standing {
    Some(memory) => calls.version(brain, memory.id, &body).await?,
    None => {
      body["type"] = json!(kind);
      body["key"] = json!(key);
      calls.write(brain, &body).await?
    }
  };
  app.take_in(&RealmKey::Organization, &written);
  Ok(written)
}

/// A source written with its name and colour, every account's `hidden` mark kept.
async fn write_source(
  app: &Arc<App>,
  brain: i64,
  source: &str,
  name: &str,
  color: Color,
  standing: Option<&Memory>,
) -> Answer<Memory> {
  let created_at = standing
    .and_then(|memory| source_of(memory, Realm::Organization, None))
    .map_or_else(now_ms, |known| known.created_at);
  let labels = source_labels(color, created_at, standing);
  if let Some(memory) = standing
    && memory.text == name
    && memory.labels == labels
  {
    return Ok(memory.clone());
  }
  let body = json!({ "actorName": source_actor(source), "text": name, "labels": labels });
  put_memory(app, brain, (SOURCE_TYPE, source_key(source)), standing, body).await
}

/// The source a schedule is written under: the one standing, else one made, named by its id.
async fn known_source(app: &Arc<App>, brain: i64, source: &str) -> Answer<()> {
  if source_memory(app, source)?.is_none() {
    write_source(app, brain, source, source, Color::Secondary, None).await?;
  }
  Ok(())
}

/// A schedule written in its shape, or the refusal its fault is.
fn checked_schedule(write: &ScheduleWrite) -> Answer<()> {
  check_schedule(write).map_err(|fault| match fault {
    ScheduleFault::Shape(why) => ApiError::bad_request(why),
    ScheduleFault::EndBeforeStart => ApiError::end_before_start(),
  })
}

/// What a write of one schedule did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
  Made,
  Changed,
  Unchanged,
}

/// One schedule written over what stands at its key, where it differs.
async fn write_schedule(
  app: &Arc<App>,
  brain: i64,
  source: &str,
  item: &str,
  write: &ScheduleWrite,
) -> Answer<(Memory, Outcome)> {
  let standing = schedule_memory(app, source, item)?;
  let created_at = standing
    .as_ref()
    .map_or_else(now_ms, |memory| schedule_of(memory, Realm::Organization).created_at);
  let labels = schedule_labels(source, item, write, created_at);
  let text = schedule_text(write);
  if let Some(memory) = &standing
    && memory.labels == labels
    && memory.text == text
  {
    return Ok((memory.clone(), Outcome::Unchanged));
  }
  let valid_from = parse_local(&write.start).map(epoch_of).unwrap_or_default();
  let body =
    json!({ "actorName": source_actor(source), "text": text, "validFrom": valid_from, "labels": labels });
  let outcome = if standing.is_some() { Outcome::Changed } else { Outcome::Made };
  let written =
    put_memory(app, brain, (SCHEDULE_TYPE, schedule_key(source, item)), standing.as_ref(), body).await?;
  Ok((written, outcome))
}

/// A standing schedule closed.
async fn remove_schedule(app: &Arc<App>, brain: i64, source: &str, memory: &Memory) -> Answer<()> {
  let moved = app.brain()?.done(brain, memory.id, &source_actor(source), REMOVED).await?;
  app.take_in(&RealmKey::Organization, &moved);
  Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// The units' writes

async fn put_source(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(source): Path<String>,
  bytes: Bytes,
) -> Answer<Response> {
  let app = &served.app;
  service_only(app, &headers)?;
  let source = checked_id(&source, "source id")?;
  let given: SourceWrite = body(&bytes)?;
  let name = one_line(&given.name, "name")?;
  let brain = organization_brain(app).await?;
  let _writing = app.schedule_lock.lock().await;
  let standing = source_memory(app, source)?;
  let color = given
    .color
    .or_else(|| {
      standing
        .as_ref()
        .and_then(|memory| source_of(memory, Realm::Organization, None))
        .map(|known| known.color)
    })
    .unwrap_or(Color::Secondary);
  let written = write_source(app, brain, source, &name, color, standing.as_ref()).await?;
  let status = if standing.is_some() { StatusCode::OK } else { StatusCode::CREATED };
  let answer = source_of(&written, Realm::Organization, None)
    .ok_or_else(|| ApiError::failed("a source without a key"))?;
  Ok(json_answer(status, answer))
}

async fn put_one(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((source, item)): Path<(String, String)>,
  bytes: Bytes,
) -> Answer<Response> {
  let app = &served.app;
  service_only(app, &headers)?;
  let source = checked_id(&source, "source id")?;
  let item = checked_id(&item, "item id")?;
  let write: ScheduleWrite = body(&bytes)?;
  checked_schedule(&write)?;
  let brain = organization_brain(app).await?;
  let _writing = app.schedule_lock.lock().await;
  known_source(app, brain, source).await?;
  let (written, outcome) = write_schedule(app, brain, source, item, &write).await?;
  let status = if outcome == Outcome::Made { StatusCode::CREATED } else { StatusCode::OK };
  Ok(json_answer(status, schedule_of(&written, Realm::Organization)))
}

async fn remove_one(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((source, item)): Path<(String, String)>,
) -> Answer<StatusCode> {
  let app = &served.app;
  service_only(app, &headers)?;
  let source = checked_id(&source, "source id")?;
  let item = checked_id(&item, "item id")?;
  let Some(brain) = app.realm(&RealmKey::Organization, false).await? else {
    return Ok(StatusCode::NO_CONTENT);
  };
  let _writing = app.schedule_lock.lock().await;
  if let Some(memory) = schedule_memory(app, source, item)? {
    remove_schedule(app, brain, source, &memory).await?;
  }
  Ok(StatusCode::NO_CONTENT)
}

async fn put_set(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(source): Path<String>,
  bytes: Bytes,
) -> Answer<Json<Reconciled>> {
  let app = &served.app;
  service_only(app, &headers)?;
  let source = checked_id(&source, "source id")?;
  let set: ScheduleSet = body(&bytes)?;
  let mut named = BTreeSet::new();
  for item in &set.items {
    checked_id(&item.item_id, "item id")?;
    checked_schedule(&item.schedule)?;
    if !named.insert(item.item_id.as_str()) {
      return Err(ApiError::bad_request(format!("The item {} is named twice.", item.item_id)));
    }
  }
  let brain = organization_brain(app).await?;
  let _writing = app.schedule_lock.lock().await;
  known_source(app, brain, source).await?;
  let mut done = Reconciled {
    written: 0,
    unchanged: 0,
    removed: 0,
  };
  for item in &set.items {
    match write_schedule(app, brain, source, &item.item_id, &item.schedule).await?.1 {
      Outcome::Unchanged => done.unchanged += 1,
      Outcome::Made | Outcome::Changed => done.written += 1,
    }
  }
  let unnamed: Vec<Memory> = held(app)?
    .1
    .into_iter()
    .filter(|memory| memory.label("source") == source && !named.contains(memory.label("item")))
    .collect();
  for memory in &unnamed {
    remove_schedule(app, brain, source, memory).await?;
    done.removed += 1;
  }
  Ok(Json(done))
}

// ---------------------------------------------------------------------------------------------------------
// The screens' reads, and a reader's layers

async fn list_sources(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<Json<Page<Source>>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  app.realm(&RealmKey::Organization, false).await?;
  let mut sources: Vec<Source> = held(app)?
    .0
    .iter()
    .filter_map(|memory| source_of(memory, asked.realm.realm(), asked.account.as_deref()))
    .collect();
  sources.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
  Ok(Json(Page {
    items: sources,
    more: false,
  }))
}

async fn change_source(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, source)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Json<Source>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let change: SourceChange = body(&bytes)?;
  let brain = app.realm(&RealmKey::Organization, false).await?;
  let _writing = app.schedule_lock.lock().await;
  let (Some(brain), Some(memory)) = (brain, source_memory(app, &source)?) else {
    return Err(ApiError::not_found(format!("The calendar holds no source {source}.")));
  };
  let written = match change.hidden {
    Some(hidden) => hide(app, &asked, brain, &memory, hidden).await?,
    None => memory,
  };
  source_of(&written, asked.realm.realm(), asked.account.as_deref())
    .map(Json)
    .ok_or_else(|| ApiError::failed("a source without a key"))
}

/// A source hidden or shown in the asked realm for the asked account.
async fn hide(app: &Arc<App>, asked: &Asked, brain: i64, memory: &Memory, hidden: bool) -> Answer<Memory> {
  let account = asked
    .account
    .clone()
    .ok_or_else(|| ApiError::no_account("A source is hidden for an account; the call names none."))?;
  let label = source_hidden_label(asked.realm.realm());
  let mut accounts: Vec<String> =
    accounts_in(memory, label).into_iter().filter(|key| *key != account).collect();
  if hidden {
    accounts.push(account);
  }
  let mut labels = memory.labels.clone();
  labels.insert(label.to_owned(), accounts.join(","));
  if labels == memory.labels {
    return Ok(memory.clone());
  }
  let body = json!({ "actorName": actor(asked), "text": memory.text, "labels": labels });
  let key = memory.key.clone().unwrap_or_default();
  put_memory(app, brain, (SOURCE_TYPE, key), Some(memory), body).await
}

async fn list_schedules(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<SpanQuery>, QueryRejection>,
) -> Answer<Json<Page<Schedule>>> {
  let app = &served.app;
  let given = query(given)?;
  let asked = asked(app, &headers, &realm, given.account, given.holder_name).await?;
  let (low, high) = span(&given.start, &given.end)?;
  app.realm(&RealmKey::Organization, false).await?;
  let mut items: Vec<Schedule> = held(app)?
    .1
    .iter()
    .map(|memory| schedule_of(memory, asked.realm.realm()))
    .filter(|schedule| match &asked.realm {
      RealmKey::Mine(account) => schedule.assignee.as_deref() == Some(account.as_str()),
      RealmKey::Organization => true,
    })
    .filter(|schedule| match (parse_local(&schedule.start), parse_local(&schedule.end)) {
      (Some(start), Some(end)) => start < high && end > low,
      _ => false,
    })
    .collect();
  items.sort_by(|a, b| (&a.start, &a.source_id, &a.item_id).cmp(&(&b.start, &b.source_id, &b.item_id)));
  Ok(Json(Page { items, more: false }))
}
