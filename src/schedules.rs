//! The sources and the schedules other units push (`docs/api.md`, 「Sources and schedules」): a unit's
//! writes under its own source, by the service token alone, the sources kept in the platform's own brain and
//! each schedule in the brain of the company it names, and the reads of them for both realms. A source is
//! data: nothing here tells one source from another.

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
  Answer, Asked, Credential, Named, Served, actor, asked, body, credential, json_answer, one_line,
  own_company, query, span,
};
use crate::app::App;
use crate::cache::RealmKey;
use crate::error::ApiError;
use crate::model::{
  Memory, SCHEDULE_TYPE, SOURCE_TYPE, ScheduleFault, accounts_in, check_schedule, epoch_of, now_ms,
  parse_local, schedule_company, schedule_id, schedule_key, schedule_labels, schedule_of, schedule_text,
  source_hidden_label, source_key, source_labels, source_of, valid_id,
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
// The brains, as these routes read and write them

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

/// A realm's brain, made where none stands yet.
async fn made_brain(app: &Arc<App>, realm: &RealmKey) -> Answer<i64> {
  app.realm(realm, true).await?.ok_or_else(|| ApiError::failed("no brain was made"))
}

/// The live sources the platform's brain holds now.
fn sources(app: &App) -> Answer<Vec<Memory>> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(
    cache
      .realms
      .get(&RealmKey::Platform)
      .map(|held| held.sources.values().cloned().collect())
      .unwrap_or_default(),
  )
}

/// The live schedules of every company and the platform, each with the realm it is kept in.
fn every_schedule(app: &App) -> Answer<Vec<(RealmKey, Memory)>> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(
    cache
      .realms
      .iter()
      .filter(|(realm, _)| realm.organization())
      .flat_map(|(realm, held)| held.schedules.values().map(|memory| (realm.clone(), memory.clone())))
      .collect(),
  )
}

/// The live schedules one realm holds now.
fn schedules_of(app: &App, realm: &RealmKey) -> Answer<Vec<Memory>> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(
    cache
      .realms
      .get(realm)
      .map(|held| held.schedules.values().cloned().collect())
      .unwrap_or_default(),
  )
}

fn source_memory(app: &App, source: &str) -> Answer<Option<Memory>> {
  Ok(sources(app)?.into_iter().find(|memory| memory.own_id() == Some(source)))
}

/// The schedule standing at a source's item, in whichever company it is kept, with that company's realm.
fn schedule_memory(app: &App, source: &str, item: &str) -> Answer<Option<(RealmKey, Memory)>> {
  let id = schedule_id(source, item);
  Ok(
    every_schedule(app)?
      .into_iter()
      .find(|(_, memory)| memory.own_id() == Some(id.as_str())),
  )
}

/// The realm a schedule written is kept in: its company's, else the platform's.
fn realm_of_write(write: &ScheduleWrite) -> RealmKey {
  schedule_company(write).map_or(RealmKey::Platform, |company| RealmKey::Company(company.to_owned()))
}

/// The name a unit's writes are recorded under in the brain.
fn source_actor(source: &str) -> String {
  format!("source:{source}")
}

/// A memory written: a version over the standing one where one stands, else a first version under its type
/// and key; taken in.
async fn put_memory(
  app: &Arc<App>,
  (realm, brain): (&RealmKey, i64),
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
  app.take_in(realm, &written);
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
  put_memory(app, (&RealmKey::Platform, brain), (SOURCE_TYPE, source_key(source)), standing, body).await
}

/// The source a schedule is written under: the one standing, else one made in the platform's brain, named by
/// its id.
async fn known_source(app: &Arc<App>, source: &str) -> Answer<()> {
  if source_memory(app, source)?.is_none() {
    let brain = made_brain(app, &RealmKey::Platform).await?;
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

/// One schedule written over what stands at its key, where it differs, in the brain of the company it names; a
/// schedule standing in another company's leaves it. Answers the memory and the realm it is kept in.
async fn write_schedule(
  app: &Arc<App>,
  source: &str,
  item: &str,
  write: &ScheduleWrite,
) -> Answer<(RealmKey, Memory, Outcome)> {
  let realm = realm_of_write(write);
  let standing = schedule_memory(app, source, item)?;
  let created_at = standing
    .as_ref()
    .map_or_else(now_ms, |(_, memory)| crate::model::number(memory, "createdAt"));
  let labels = schedule_labels(source, item, write, created_at);
  let text = schedule_text(write);
  let here = standing.as_ref().filter(|(kept, _)| *kept == realm).map(|(_, memory)| memory);
  if let Some(memory) = here
    && memory.labels == labels
    && memory.text == text
  {
    return Ok((realm, memory.clone(), Outcome::Unchanged));
  }
  if let Some((kept, memory)) = standing.as_ref().filter(|(kept, _)| *kept != realm) {
    let moved_to = format!("moved to {}", realm.company().unwrap_or_default());
    remove_schedule(app, kept, source, memory, &moved_to).await?;
  }
  let brain = made_brain(app, &realm).await?;
  let valid_from = parse_local(&write.start).map(epoch_of).unwrap_or_default();
  let body =
    json!({ "actorName": source_actor(source), "text": text, "validFrom": valid_from, "labels": labels });
  let outcome = if standing.is_some() { Outcome::Changed } else { Outcome::Made };
  let key = schedule_key(source, item);
  let written = put_memory(app, (&realm, brain), (SCHEDULE_TYPE, key), here, body).await?;
  Ok((realm, written, outcome))
}

/// A standing schedule closed in the realm it is kept in.
async fn remove_schedule(
  app: &Arc<App>,
  realm: &RealmKey,
  source: &str,
  memory: &Memory,
  reason: &str,
) -> Answer<()> {
  let brain = made_brain(app, realm).await?;
  let moved = app.brain()?.done(brain, memory.id, &source_actor(source), reason).await?;
  app.take_in(realm, &moved);
  Ok(())
}

/// A schedule as a read answers it, read in a realm, kept in a company's brain or the platform's.
fn schedule_read(memory: &Memory, realm: Realm, kept: &RealmKey) -> Schedule {
  schedule_of(memory, realm, kept.company().unwrap_or_default())
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
  let brain = made_brain(app, &RealmKey::Platform).await?;
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
  if let Some(reason) = app.unready() {
    return Err(ApiError::not_ready(reason));
  }
  let _writing = app.schedule_lock.lock().await;
  known_source(app, source).await?;
  let (realm, written, outcome) = write_schedule(app, source, item, &write).await?;
  let status = if outcome == Outcome::Made { StatusCode::CREATED } else { StatusCode::OK };
  Ok(json_answer(status, schedule_read(&written, Realm::Organization, &realm)))
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
  if let Some(reason) = app.unready() {
    return Err(ApiError::not_ready(reason));
  }
  let _writing = app.schedule_lock.lock().await;
  if let Some((realm, memory)) = schedule_memory(app, source, item)? {
    remove_schedule(app, &realm, source, &memory, REMOVED).await?;
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
  if let Some(reason) = app.unready() {
    return Err(ApiError::not_ready(reason));
  }
  let _writing = app.schedule_lock.lock().await;
  known_source(app, source).await?;
  let mut done = Reconciled {
    written: 0,
    unchanged: 0,
    removed: 0,
  };
  for item in &set.items {
    match write_schedule(app, source, &item.item_id, &item.schedule).await?.2 {
      Outcome::Unchanged => done.unchanged += 1,
      Outcome::Made | Outcome::Changed => done.written += 1,
    }
  }
  let unnamed: Vec<(RealmKey, Memory)> = every_schedule(app)?
    .into_iter()
    .filter(|(_, memory)| memory.label("source") == source && !named.contains(memory.label("item")))
    .collect();
  for (realm, memory) in &unnamed {
    remove_schedule(app, realm, source, memory, REMOVED).await?;
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
  let asked = asked(app, &headers, &realm, Named::of(given.account, given.holder_name, None)).await?;
  app.realm(&RealmKey::Platform, false).await?;
  let mut sources: Vec<Source> = sources(app)?
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
  let named = Named::of(given.account, given.holder_name, None);
  let asked = asked(app, &headers, &realm, named).await?.for_writing()?;
  let change: SourceChange = body(&bytes)?;
  let brain = app.realm(&RealmKey::Platform, false).await?;
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
  put_memory(app, (&RealmKey::Platform, brain), (SOURCE_TYPE, key), Some(memory), body).await
}

async fn list_schedules(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<SpanQuery>, QueryRejection>,
) -> Answer<Json<Page<Schedule>>> {
  let app = &served.app;
  let given = query(given)?;
  let asked =
    asked(app, &headers, &realm, Named::of(given.account, given.holder_name, given.company)).await?;
  let (low, high) = span(&given.start, &given.end)?;
  // A reader's own realm shows the schedules assigned to it of its own company alone.
  let kept = match &asked.realm {
    RealmKey::Mine(account) => {
      own_company(app, Some(account)).await?.map_or(RealmKey::NoCompany, RealmKey::Company)
    }
    organization => organization.clone(),
  };
  app.realm(&kept, false).await?;
  let mut items: Vec<Schedule> = schedules_of(app, &kept)?
    .iter()
    .map(|memory| schedule_read(memory, asked.realm.realm(), &kept))
    .filter(|schedule| match &asked.realm {
      RealmKey::Mine(account) => schedule.assignee.as_deref() == Some(account.as_str()),
      _ => true,
    })
    .filter(|schedule| match (parse_local(&schedule.start), parse_local(&schedule.end)) {
      (Some(start), Some(end)) => start < high && end > low,
      _ => false,
    })
    .collect();
  items.sort_by(|a, b| (&a.start, &a.source_id, &a.item_id).cmp(&(&b.start, &b.source_id, &b.item_id)));
  Ok(Json(Page { items, more: false }))
}
