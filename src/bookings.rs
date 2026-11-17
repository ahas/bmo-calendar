//! A company's resources and their bookings (`docs/design.md`, 「Resources and bookings」; `docs/api.md`,
//! 「Resources and bookings」): the routes, the one writer that keeps two bookings of a resource from meeting,
//! the keepers' decisions, and the tellings of requests and decisions the beat makes.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Json, Response};
use axum::routing::{get, patch, post};
use chrono::NaiveDateTime;
use serde_json::{Value, json};

use crate::alarms;
use crate::api::{
  Answer, Asked, Named, Served, actor, asked, asked_organization, body, json_answer, one_line, own_company,
  query, span,
};
use crate::app::App;
use crate::booking::{
  MOST_SEATS, Times, booking_key, booking_labels, booking_of, first_meeting, holds_span, requested_at,
  resource_key, resource_labels, resource_of,
};
use crate::cache::RealmKey;
use crate::error::ApiError;
use crate::events::Scope;
use crate::model::{
  BOOKING_TYPE, Memory, RESOURCE_TYPE, epoch_of, fresh_id, is_occurrence, local_text, now_ms, parse_local,
  skipped, spans_between, valid_id,
};
use crate::peers;
use crate::wire::{
  AccountQuery, Booking, BookingChange, BookingDecided, BookingState, BookingsQuery, CancelQuery, Decision,
  FreeQuery, NewBooking, NewResource, Page, Realm, Repeat, Resource, ResourceChange, ResourcesQuery,
  ToldBooking,
};

/// The routes of resources and bookings, under the same guard as the screens' others.
pub fn routes() -> Router<Served> {
  Router::new()
    .route("/{realm}/resources", get(list_resources).post(create_resource))
    .route("/{realm}/resources/free", get(free_resources))
    .route("/{realm}/resources/{resourceId}", patch(change_resource).delete(retire_resource))
    .route("/{realm}/bookings", get(list_bookings).post(create_booking))
    .route("/{realm}/bookings/requests", get(list_requests))
    .route(
      "/{realm}/bookings/{bookingId}",
      get(read_booking).patch(change_booking).delete(cancel_booking),
    )
    .route("/{realm}/bookings/{bookingId}/decision", post(decide))
}

// ---------------------------------------------------------------------------------------------------------
// The cache, as these routes read it

/// A realm's live resources and bookings as they stand now.
fn held(app: &App, realm: &RealmKey) -> Answer<(Vec<Memory>, Vec<Memory>)> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  Ok(
    cache
      .realms
      .get(realm)
      .map(|held| (held.resources.values().cloned().collect(), held.bookings.values().cloned().collect()))
      .unwrap_or_default(),
  )
}

fn resource_memory(app: &App, realm: &RealmKey, id: &str) -> Answer<Option<Memory>> {
  Ok(held(app, realm)?.0.into_iter().find(|memory| memory.own_id() == Some(id)))
}

fn booking_memory(app: &App, realm: &RealmKey, id: &str) -> Answer<Option<Memory>> {
  Ok(held(app, realm)?.1.into_iter().find(|memory| memory.own_id() == Some(id)))
}

/// The company word of a realm's rows.
fn company_word(realm: &RealmKey) -> String {
  realm.company().unwrap_or_default()
}

fn resource_answer(realm: &RealmKey, memory: &Memory) -> Answer<Resource> {
  resource_of(memory, &company_word(realm)).ok_or_else(|| ApiError::failed("a resource without a key"))
}

fn booking_answer(realm: &RealmKey, read_in: Realm, memory: &Memory) -> Answer<Booking> {
  booking_of(memory, read_in, &company_word(realm))
    .ok_or_else(|| ApiError::failed("a booking without a key"))
}

/// A realm's brain, made where none stands; a call with no company is refused `no-company`.
async fn made_brain(app: &Arc<App>, realm: &RealmKey) -> Answer<i64> {
  app.realm(realm, true).await?.ok_or_else(|| ApiError::failed("no brain was made"))
}

/// A memory written: a version over the standing one where one stands, else a first version under its type
/// and key; taken in. Nothing is written where the standing one already holds the words and labels.
async fn put(
  app: &Arc<App>,
  (realm, brain): (&RealmKey, i64),
  (kind, key): (&str, String),
  standing: Option<&Memory>,
  mut body: Value,
) -> Answer<Memory> {
  if let Some(memory) = standing
    && body["text"].as_str() == Some(memory.text.as_str())
    && serde_json::to_value(&memory.labels).ok().as_ref() == Some(&body["labels"])
    && body["validFrom"].as_i64().is_none_or(|from| from == memory.valid_from)
  {
    return Ok(memory.clone());
  }
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

/// The account a write acts for: a resource is kept and a booking made by an account.
fn account(asked: &Asked) -> Answer<String> {
  asked.account.clone().ok_or_else(|| ApiError::no_account("The call names no account."))
}

/// The tellings of requests and decisions made soon, apart from the call that made them.
fn tell_soon(app: &Arc<App>) {
  let app = Arc::clone(app);
  tokio::spawn(async move {
    if let Err(why) = alarms::tell_bookings(&app).await {
      eprintln!("bmo-calendar: a booking's telling failed: {}", why.error);
    }
  });
}

// ---------------------------------------------------------------------------------------------------------
// Resources

/// Whether the caller makes, changes and retires a company's resources: the administrator and a head of the
/// company. Resources stand in a company's realm alone.
async fn may_keep_resources(app: &App, asked: &Asked) -> Answer<String> {
  let RealmKey::Company(company) = &asked.realm else {
    return Err(ApiError::no_company());
  };
  if asked.caller.administrator {
    return Ok(company.clone());
  }
  let account = account(asked)?;
  if peers::heads(&app.http, &app.settings, company, &account).await? {
    return Ok(company.clone());
  }
  Err(ApiError::not_a_writer(
    "Only the administrator and the company's heads keep its resources.",
  ))
}

/// A resource's fields in their shape, its keepers each of the company.
async fn checked_resource(app: &App, company: &str, resource: &mut Resource) -> Answer<()> {
  resource.name = one_line(&resource.name, "name")?;
  resource.kind = one_line(&resource.kind, "kind")?;
  if resource.place.contains('\n') {
    return Err(ApiError::bad_request("A place is one line."));
  }
  resource.place = resource.place.trim().to_owned();
  if !(1..=MOST_SEATS).contains(&resource.seats) {
    return Err(ApiError::bad_request("Seats are a whole number from 1 to 10000."));
  }
  let mut keepers: Vec<String> = Vec::new();
  for keeper in resource.keepers.iter().map(|keeper| keeper.trim()).filter(|keeper| !keeper.is_empty()) {
    if keeper.chars().count() > 200 || keeper.contains(',') {
      return Err(ApiError::bad_request("A keeper is an account key of at most 200 characters."));
    }
    if !keepers.iter().any(|known| known == keeper) {
      keepers.push(keeper.to_owned());
    }
  }
  if resource.approval && keepers.is_empty() {
    return Err(ApiError::bad_request("A resource whose bookings wait for approval has a keeper."));
  }
  for keeper in &keepers {
    let stands = peers::company_of(&app.http, &app.settings, keeper).await?;
    if stands.is_none_or(|stands| stands.id != company) {
      return Err(ApiError::not_in_company(format!("The keeper {keeper} stands in no company {company}.")));
    }
  }
  resource.keepers = keepers;
  Ok(())
}

async fn write_resource(
  app: &Arc<App>,
  asked: &Asked,
  resource: &Resource,
  standing: Option<&Memory>,
) -> Answer<Memory> {
  let brain = made_brain(app, &asked.realm).await?;
  let body = json!({
    "actorName": actor(asked),
    "text": resource.name,
    "labels": resource_labels(resource),
  });
  put(app, (&asked.realm, brain), (RESOURCE_TYPE, resource_key(&resource.id)), standing, body).await
}

async fn list_resources(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<ResourcesQuery>, QueryRejection>,
) -> Answer<Json<Page<Resource>>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  app.realm(&asked.realm, false).await?;
  let mut items = Vec::new();
  for memory in held(app, &asked.realm)?.0 {
    let resource = resource_answer(&asked.realm, &memory)?;
    if given.kind.as_ref().is_none_or(|kind| *kind == resource.kind) {
      items.push(resource);
    }
  }
  items.sort_by(|a, b| (&a.kind, &a.place, &a.name, &a.id).cmp(&(&b.kind, &b.place, &b.name, &b.id)));
  Ok(Json(Page { items, more: false }))
}

async fn create_resource(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Response> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let company = may_keep_resources(app, &asked).await?;
  let new: NewResource = body(&bytes)?;
  let id = new.id.clone().unwrap_or_else(fresh_id);
  if !valid_id(&id) {
    return Err(ApiError::bad_request("An id is 1 to 64 letters, digits, '-' and '_'."));
  }
  app.realm(&asked.realm, false).await?;
  let standing = resource_memory(app, &asked.realm, &id)?;
  let known = standing.as_ref().map(|memory| resource_answer(&asked.realm, memory)).transpose()?;
  let now = now_ms();
  let mut resource = Resource {
    id,
    company: company.clone(),
    name: new.name,
    kind: new.kind,
    place: new.place,
    seats: new.seats,
    approval: new.approval,
    keepers: new.keepers,
    creator: known
      .as_ref()
      .map_or_else(|| asked.account.clone().unwrap_or_default(), |known| known.creator.clone()),
    created_at: known.as_ref().map_or(now, |known| known.created_at),
    changed_at: now,
  };
  checked_resource(app, &company, &mut resource).await?;
  let written = write_resource(app, &asked, &resource, standing.as_ref()).await?;
  let status = if standing.is_some() { StatusCode::OK } else { StatusCode::CREATED };
  Ok(json_answer(status, resource_answer(&asked.realm, &written)?))
}

async fn change_resource(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Json<Resource>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let change: ResourceChange = body(&bytes)?;
  app.realm(&asked.realm, false).await?;
  let Some(memory) = resource_memory(app, &asked.realm, &id)? else {
    return Err(ApiError::not_found(format!("The company holds no live resource {id}.")));
  };
  let company = may_keep_resources(app, &asked).await?;
  let mut resource = resource_answer(&asked.realm, &memory)?;
  resource.name = change.name.unwrap_or(resource.name);
  resource.kind = change.kind.unwrap_or(resource.kind);
  resource.place = change.place.unwrap_or(resource.place);
  resource.seats = change.seats.unwrap_or(resource.seats);
  resource.approval = change.approval.unwrap_or(resource.approval);
  resource.keepers = change.keepers.unwrap_or(resource.keepers);
  checked_resource(app, &company, &mut resource).await?;
  let written = write_resource(app, &asked, &resource, Some(&memory)).await?;
  resource_answer(&asked.realm, &written).map(Json)
}

async fn retire_resource(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<StatusCode> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let brain = app.realm(&asked.realm, false).await?;
  let (Some(brain), Some(memory)) = (brain, resource_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The company holds no live resource {id}.")));
  };
  may_keep_resources(app, &asked).await?;
  let _writing = app.booking_lock.lock().await;
  let calls = app.brain()?;
  let actor = actor(&asked);
  let moved = calls.done(brain, memory.id, &actor, "retired").await?;
  app.take_in(&asked.realm, &moved);
  for booking in held(app, &asked.realm)?.1.into_iter().filter(|booking| booking.label("resource") == id) {
    let moved = calls.done(brain, booking.id, &actor, "retired with its resource").await?;
    app.take_in(&asked.realm, &moved);
  }
  Ok(StatusCode::NO_CONTENT)
}

async fn free_resources(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<FreeQuery>, QueryRejection>,
) -> Answer<Json<Page<Resource>>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account.clone(), given.holder_name.clone(), given.company.clone());
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let (start, end) = times(&given.start, &given.end)?;
  let wanted = Times {
    start,
    end,
    repeat: given.repeat.unwrap_or(Repeat::None),
    skip: Vec::new(),
  };
  let seats = given.seats.unwrap_or(1);
  app.realm(&asked.realm, false).await?;
  let (resources, bookings) = held(app, &asked.realm)?;
  let mut items = Vec::new();
  for memory in &resources {
    let resource = resource_answer(&asked.realm, memory)?;
    if resource.seats < seats || given.kind.as_ref().is_some_and(|kind| *kind != resource.kind) {
      continue;
    }
    if meeting(&bookings, &resource.id, &wanted, &[]).is_none() {
      items.push(resource);
    }
  }
  items.sort_by(|a, b| (a.seats, &a.name, &a.id).cmp(&(b.seats, &b.name, &b.id)));
  Ok(Json(Page { items, more: false }))
}

// ---------------------------------------------------------------------------------------------------------
// Bookings

/// A start and an end, the end after the start.
fn times(start: &str, end: &str) -> Answer<(NaiveDateTime, NaiveDateTime)> {
  let start = parse_local(start).ok_or_else(|| ApiError::bad_request("start is no local ISO datetime"))?;
  let end = parse_local(end).ok_or_else(|| ApiError::bad_request("end is no local ISO datetime"))?;
  if end <= start {
    return Err(ApiError::end_before_start());
  }
  Ok((start, end))
}

/// An all-day booking's span widened to its days whole: from its first day's midnight to the midnight after
/// its last.
fn whole_days(booking: &mut Booking) {
  let (Some(start), Some(end)) = (parse_local(&booking.start), parse_local(&booking.end)) else {
    return;
  };
  if !booking.all_day {
    return;
  }
  let first = start.date().and_time(chrono::NaiveTime::MIN);
  let last = if end.time() == chrono::NaiveTime::MIN {
    end
  } else {
    end.date().and_time(chrono::NaiveTime::MIN) + chrono::Duration::days(1)
  };
  booking.start = local_text(first);
  booking.end = local_text(last);
}

/// The standing booking of a resource a span would meet, and the first occurrence of the span that meets it:
/// every requested or booked one but those named.
fn meeting(
  bookings: &[Memory],
  resource: &str,
  wanted: &Times,
  except: &[&str],
) -> Option<(Memory, String)> {
  bookings
    .iter()
    .filter(|memory| memory.label("resource") == resource)
    .filter(|memory| memory.own_id().is_some_and(|id| !except.contains(&id)))
    .filter(|memory| {
      booking_of(memory, Realm::Organization, "").is_some_and(|booking| holds_span(booking.state))
    })
    .find_map(|memory| {
      let other = Times::of_memory(memory)?;
      let at = first_meeting(wanted, &other)?;
      Some((memory.clone(), at))
    })
}

/// A booking's span refused where it meets another standing booking of its resource.
fn refuse_meeting(bookings: &[Memory], booking: &Booking, skip: &[String], except: &[&str]) -> Answer<()> {
  if !holds_span(booking.state) {
    return Ok(());
  }
  let wanted =
    Times::of(booking, skip.to_vec()).ok_or_else(|| ApiError::bad_request("the times do not read"))?;
  match meeting(bookings, &booking.resource_id, &wanted, except) {
    Some((other, at)) => Err(ApiError::overlap(format!(
      "The booking meets the booking \"{}\" ({}) of the same resource, first at its occurrence starting {at}.",
      other.text,
      other.own_id().unwrap_or_default()
    ))),
    None => Ok(()),
  }
}

/// The state a written booking stands in: requested on a resource that waits for approval where the writer
/// keeps it not, else booked.
fn arriving_state(resource: &Resource, account: &str) -> BookingState {
  if resource.approval && !resource.keepers.iter().any(|keeper| keeper == account) {
    BookingState::Requested
  } else {
    BookingState::Booked
  }
}

/// A booking brought to the state a write that moved its span leaves it in.
fn arrive(booking: &mut Booking, resource: &Resource, account: &str, now: i64) -> Option<i64> {
  booking.state = arriving_state(resource, account);
  booking.decided_by = None;
  booking.decided_at = None;
  booking.reason = None;
  (booking.state == BookingState::Requested).then_some(now)
}

/// Whether the caller changes or cancels a booking: its booker and a keeper of its resource.
fn may_change(account: &str, booking: &Booking, resource: Option<&Resource>) -> Answer<()> {
  let keeps = resource.is_some_and(|resource| resource.keepers.iter().any(|keeper| keeper == account));
  if booking.booker == account || keeps {
    return Ok(());
  }
  Err(ApiError::not_a_writer(
    "Only its booker and a keeper of its resource change this booking.",
  ))
}

/// A booking written, its skips and the moment it came to stand requested with it.
async fn write_booking(
  app: &Arc<App>,
  asked: &Asked,
  brain: i64,
  booking: &Booking,
  (skip, requested): (&[String], Option<i64>),
  standing: Option<&Memory>,
) -> Answer<Memory> {
  let (start, _) = times(&booking.start, &booking.end)?;
  let body = json!({
    "actorName": actor(asked),
    "text": booking.title,
    "validFrom": epoch_of(start),
    "labels": booking_labels(booking, skip, requested),
  });
  put(app, (&asked.realm, brain), (BOOKING_TYPE, booking_key(&booking.id)), standing, body).await
}

/// The live resource a booking is written on, or `unknown-resource`.
fn live_resource(app: &App, realm: &RealmKey, id: &str) -> Answer<Resource> {
  let memory = resource_memory(app, realm, id)?.ok_or_else(|| ApiError::unknown_resource(id))?;
  resource_answer(realm, &memory)
}

async fn create_booking(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Response> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let new: NewBooking = body(&bytes)?;
  let id = new.id.clone().unwrap_or_else(fresh_id);
  if !valid_id(&id) {
    return Err(ApiError::bad_request("An id is 1 to 64 letters, digits, '-' and '_'."));
  }
  let title = one_line(&new.title, "title")?;
  times(&new.start, &new.end)?;
  let account = account(&asked)?;
  if own_company(app, Some(&account)).await?.map(RealmKey::Company) != Some(asked.realm.clone()) {
    return Err(ApiError::not_a_writer("A booking is made in its booker's own company."));
  }
  let brain = made_brain(app, &asked.realm).await?;
  let _writing = app.booking_lock.lock().await;
  let resource = live_resource(app, &asked.realm, &new.resource_id)?;
  let standing = booking_memory(app, &asked.realm, &id)?;
  let known = standing
    .as_ref()
    .map(|memory| booking_answer(&asked.realm, Realm::Organization, memory))
    .transpose()?;
  if let Some(known) = &known {
    let kept = live_resource(app, &asked.realm, &known.resource_id).ok();
    may_change(&account, known, kept.as_ref())?;
  }
  let now = now_ms();
  let mut booking = Booking {
    id: id.clone(),
    realm: Realm::Organization,
    company: company_word(&asked.realm),
    resource_id: resource.id.clone(),
    title,
    start: new.start.clone(),
    end: new.end.clone(),
    all_day: new.all_day.unwrap_or(false),
    repeat: new.repeat.unwrap_or(Repeat::None),
    occurrence: None,
    booker: known.as_ref().map_or_else(|| account.clone(), |known| known.booker.clone()),
    state: BookingState::Booked,
    decided_by: None,
    decided_at: None,
    reason: None,
    created_at: known.as_ref().map_or(now, |known| known.created_at),
    changed_at: now,
  };
  whole_days(&mut booking);
  let requested = match (&known, &standing) {
    (Some(known), Some(memory)) if !moved(known, &booking) && known.state != BookingState::Refused => {
      booking.state = known.state;
      booking.decided_by = known.decided_by.clone();
      booking.decided_at = known.decided_at;
      booking.reason = known.reason.clone();
      requested_at(memory)
    }
    _ => arrive(&mut booking, &resource, &account, now),
  };
  let skip = standing.as_ref().map(skipped).unwrap_or_default();
  refuse_meeting(&held(app, &asked.realm)?.1, &booking, &skip, &[id.as_str()])?;
  let written = write_booking(app, &asked, brain, &booking, (&skip, requested), standing.as_ref()).await?;
  if booking.state == BookingState::Requested {
    tell_soon(app);
  }
  let status = if standing.is_some() { StatusCode::OK } else { StatusCode::CREATED };
  Ok(json_answer(status, booking_answer(&asked.realm, Realm::Organization, &written)?))
}

/// Whether a write moves a booking: its span, all-day, repeat or resource changed.
fn moved(before: &Booking, after: &Booking) -> bool {
  before.start != after.start
    || before.end != after.end
    || before.all_day != after.all_day
    || before.repeat != after.repeat
    || before.resource_id != after.resource_id
}

async fn change_booking(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Json<Booking>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let change: BookingChange = body(&bytes)?;
  let brain = app.realm(&asked.realm, false).await?;
  let account = account(&asked)?;
  let _writing = app.booking_lock.lock().await;
  let (Some(brain), Some(memory)) = (brain, booking_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The company holds no live booking {id}.")));
  };
  let standing = booking_answer(&asked.realm, Realm::Organization, &memory)?;
  let kept = live_resource(app, &asked.realm, &standing.resource_id).ok();
  may_change(&account, &standing, kept.as_ref())?;
  let resource_id = change.resource_id.clone().unwrap_or_else(|| standing.resource_id.clone());
  let resource = live_resource(app, &asked.realm, &resource_id)?;
  let mut next = standing.clone();
  next.resource_id = resource_id;
  if let Some(title) = &change.title {
    next.title = one_line(title, "title")?;
  }
  next.all_day = change.all_day.unwrap_or(next.all_day);
  next.changed_at = now_ms();
  let skip = skipped(&memory);
  let bookings = held(app, &asked.realm)?.1;
  let Some(occurrence) = change.occurrence.clone() else {
    next.start = change.start.clone().unwrap_or(next.start);
    next.end = change.end.clone().unwrap_or(next.end);
    next.repeat = change.repeat.unwrap_or(next.repeat);
    times(&next.start, &next.end)?;
    whole_days(&mut next);
    let mut requested = requested_at(&memory);
    if moved(&standing, &next) || standing.state == BookingState::Refused {
      let now = next.changed_at;
      requested = arrive(&mut next, &resource, &account, now);
    }
    refuse_meeting(&bookings, &next, &skip, &[id.as_str()])?;
    let written = write_booking(app, &asked, brain, &next, (&skip, requested), Some(&memory)).await?;
    if next.state == BookingState::Requested {
      tell_soon(app);
    }
    return booking_answer(&asked.realm, Realm::Organization, &written).map(Json);
  };
  // One occurrence changed on its own: the series skips it, and it stands as a booking of its own.
  let (start, end) = times(&standing.start, &standing.end)?;
  let at = parse_local(&occurrence).ok_or_else(|| ApiError::not_an_occurrence(&occurrence))?;
  if standing.repeat == Repeat::None || !is_occurrence(start, standing.repeat, &skip, at) {
    return Err(ApiError::not_an_occurrence(&occurrence));
  }
  let mut series_skip = skip.clone();
  series_skip.push(occurrence);
  let mut series = standing.clone();
  series.changed_at = next.changed_at;
  let mut own = next.clone();
  own.id = fresh_id();
  own.repeat = Repeat::None;
  own.created_at = next.changed_at;
  let as_it_was = Booking {
    start: local_text(at),
    end: local_text(at + (end - start)),
    repeat: Repeat::None,
    ..standing.clone()
  };
  own.start = change.start.clone().unwrap_or_else(|| as_it_was.start.clone());
  own.end = change.end.clone().unwrap_or_else(|| as_it_was.end.clone());
  times(&own.start, &own.end)?;
  whole_days(&mut own);
  let mut requested = requested_at(&memory);
  if moved(&as_it_was, &own) || standing.state == BookingState::Refused {
    let now = own.changed_at;
    requested = arrive(&mut own, &resource, &account, now);
  }
  // The series, with the occurrence skipped, stands beside the new booking; the two are checked as they will be.
  let mut after: Vec<Memory> =
    bookings.into_iter().filter(|other| other.own_id() != Some(id.as_str())).collect();
  let mut series_memory = memory.clone();
  series_memory.labels = booking_labels(&series, &series_skip, requested_at(&memory));
  after.push(series_memory);
  refuse_meeting(&after, &own, &[], &[own.id.as_str()])?;
  write_booking(app, &asked, brain, &series, (&series_skip, requested_at(&memory)), Some(&memory)).await?;
  let written = write_booking(app, &asked, brain, &own, (&[], requested), None).await?;
  if own.state == BookingState::Requested {
    tell_soon(app);
  }
  booking_answer(&asked.realm, Realm::Organization, &written).map(Json)
}

async fn cancel_booking(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<CancelQuery>, QueryRejection>,
) -> Answer<StatusCode> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let brain = app.realm(&asked.realm, false).await?;
  let account = account(&asked)?;
  let _writing = app.booking_lock.lock().await;
  let (Some(brain), Some(memory)) = (brain, booking_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The company holds no live booking {id}.")));
  };
  let standing = booking_answer(&asked.realm, Realm::Organization, &memory)?;
  let kept = live_resource(app, &asked.realm, &standing.resource_id).ok();
  may_change(&account, &standing, kept.as_ref())?;
  match given.occurrence {
    None => {
      let moved = app.brain()?.done(brain, memory.id, &actor(&asked), "cancelled").await?;
      app.take_in(&asked.realm, &moved);
    }
    Some(occurrence) => {
      let (start, _) = times(&standing.start, &standing.end)?;
      let mut skip = skipped(&memory);
      let at = parse_local(&occurrence).ok_or_else(|| ApiError::not_an_occurrence(&occurrence))?;
      if standing.repeat == Repeat::None || !is_occurrence(start, standing.repeat, &skip, at) {
        return Err(ApiError::not_an_occurrence(&occurrence));
      }
      skip.push(occurrence);
      write_booking(app, &asked, brain, &standing, (&skip, requested_at(&memory)), Some(&memory)).await?;
    }
  }
  Ok(StatusCode::NO_CONTENT)
}

async fn decide(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
  bytes: Bytes,
) -> Answer<Json<Booking>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  let decision: Decision = body(&bytes)?;
  let brain = app.realm(&asked.realm, false).await?;
  let account = account(&asked)?;
  let _writing = app.booking_lock.lock().await;
  let (Some(brain), Some(memory)) = (brain, booking_memory(app, &asked.realm, &id)?) else {
    return Err(ApiError::not_found(format!("The company holds no live booking {id}.")));
  };
  let mut booking = booking_answer(&asked.realm, Realm::Organization, &memory)?;
  let keeps = live_resource(app, &asked.realm, &booking.resource_id)
    .is_ok_and(|resource| resource.keepers.contains(&account));
  if !keeps {
    return Err(ApiError::not_a_keeper("Only a keeper of its resource decides a booking."));
  }
  if booking.state != BookingState::Requested {
    return Err(ApiError::not_requested(&id));
  }
  booking.state = if decision.approve { BookingState::Booked } else { BookingState::Refused };
  booking.decided_by = Some(account);
  booking.decided_at = Some(now_ms());
  booking.reason = if decision.approve {
    None
  } else {
    decision
      .reason
      .map(|reason| reason.trim().replace('\n', " "))
      .filter(|reason| !reason.is_empty())
  };
  let skip = skipped(&memory);
  let written =
    write_booking(app, &asked, brain, &booking, (&skip, requested_at(&memory)), Some(&memory)).await?;
  app.events.publish(
    "booking.decided",
    BookingDecided {
      company: booking.company.clone(),
      booking_id: booking.id.clone(),
      state: booking.state,
      title: booking.title.clone(),
    },
    Scope::Account(booking.booker.clone()),
  );
  tell_soon(app);
  booking_answer(&asked.realm, Realm::Organization, &written).map(Json)
}

async fn read_booking(
  State(served): State<Served>,
  headers: HeaderMap,
  Path((realm, id)): Path<(String, String)>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<Json<Booking>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  app.realm(&asked.realm, false).await?;
  let Some(memory) = booking_memory(app, &asked.realm, &id)? else {
    return Err(ApiError::not_found(format!("The company holds no live booking {id}.")));
  };
  booking_answer(&asked.realm, Realm::Organization, &memory).map(Json)
}

/// The rows of one booking that overlap a span: itself, or one per occurrence of a repeating booking.
fn rows(booking: &Booking, memory: &Memory, low: NaiveDateTime, high: NaiveDateTime) -> Vec<Booking> {
  let (Some(start), Some(end)) = (parse_local(&booking.start), parse_local(&booking.end)) else {
    return Vec::new();
  };
  spans_between((start, end), booking.repeat, &skipped(memory), low, high)
    .into_iter()
    .map(|(at, until)| Booking {
      start: local_text(at),
      end: local_text(until),
      occurrence: (booking.repeat != Repeat::None).then(|| local_text(at)),
      ..booking.clone()
    })
    .collect()
}

async fn list_bookings(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<BookingsQuery>, QueryRejection>,
) -> Answer<Json<Page<Booking>>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked(app, &headers, &realm, named).await?;
  let (low, high) = span(&given.start, &given.end)?;
  // A booker's own realm shows its bookings of its own company, every state of them.
  let kept = match &asked.realm {
    RealmKey::Mine(account) => {
      own_company(app, Some(account)).await?.map_or(RealmKey::NoCompany, RealmKey::Company)
    }
    organization => organization.clone(),
  };
  app.realm(&kept, false).await?;
  let mut items = Vec::new();
  for memory in held(app, &kept)?.1 {
    let booking = booking_answer(&kept, asked.realm.realm(), &memory)?;
    let shown = match &asked.realm {
      RealmKey::Mine(account) => booking.booker == *account,
      _ => {
        holds_span(booking.state)
          && given.resource_id.as_ref().is_none_or(|resource| *resource == booking.resource_id)
      }
    };
    if shown {
      items.extend(rows(&booking, &memory, low, high));
    }
  }
  items.sort_by(|a, b| (&a.start, &a.id).cmp(&(&b.start, &b.id)));
  Ok(Json(Page { items, more: false }))
}

async fn list_requests(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<Json<Page<Booking>>> {
  let app = &served.app;
  let given = query(given)?;
  let named = Named::of(given.account, given.holder_name, given.company);
  let asked = asked_organization(app, &headers, &realm, named).await?;
  app.realm(&asked.realm, false).await?;
  let Some(account) = asked.account.clone() else {
    return Ok(Json(Page {
      items: Vec::new(),
      more: false,
    }));
  };
  let (resources, bookings) = held(app, &asked.realm)?;
  let kept: Vec<String> = resources
    .iter()
    .filter_map(|memory| resource_answer(&asked.realm, memory).ok())
    .filter(|resource| resource.keepers.contains(&account))
    .map(|resource| resource.id)
    .collect();
  let mut waiting: Vec<(i64, Booking)> = Vec::new();
  for memory in &bookings {
    let booking = booking_answer(&asked.realm, Realm::Organization, memory)?;
    if booking.state == BookingState::Requested && kept.contains(&booking.resource_id) {
      waiting.push((requested_at(memory).unwrap_or(booking.changed_at), booking));
    }
  }
  waiting.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
  Ok(Json(Page {
    items: waiting.into_iter().map(|(_, booking)| booking).collect(),
    more: false,
  }))
}

// ---------------------------------------------------------------------------------------------------------
// The tellings

/// One telling due: whom, what and where it is marked.
struct Telling {
  realm: RealmKey,
  brain: i64,
  memory: Memory,
  booking: Booking,
  resource: Option<Resource>,
  /// `requested:<at>` or `decided:<at>`.
  mark: String,
  accounts: Vec<String>,
}

/// The tellings of every company's bookings not yet made, and the refused bookings already told, which leave.
/// A refused booking already told, which leaves: its realm, its brain and its memory.
type Leaving = (RealmKey, i64, Memory);

fn due(app: &App) -> Answer<(Vec<Telling>, Vec<Leaving>)> {
  let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
  let mut tellings = Vec::new();
  let mut leaving = Vec::new();
  for (realm, held) in &cache.realms {
    let RealmKey::Company(company) = realm else {
      continue;
    };
    for memory in held.bookings.values() {
      let Some(booking) = booking_of(memory, Realm::Organization, company) else {
        continue;
      };
      let resource =
        held.resources.get(&booking.resource_id).and_then(|memory| resource_of(memory, company));
      let (mark, accounts) = match (booking.state, requested_at(memory), booking.decided_at) {
        (BookingState::Requested, Some(at), _) => (
          format!("requested:{at}"),
          resource.as_ref().map(|resource| resource.keepers.clone()).unwrap_or_default(),
        ),
        (BookingState::Booked | BookingState::Refused, _, Some(at)) => {
          (format!("decided:{at}"), vec![booking.booker.clone()])
        }
        _ => continue,
      };
      let told = app.store.is_told(held.brain, &booking.id, &mark).map_err(ApiError::failed)?;
      if told {
        if booking.state == BookingState::Refused {
          leaving.push((realm.clone(), held.brain, memory.clone()));
        }
        continue;
      }
      tellings.push(Telling {
        realm: realm.clone(),
        brain: held.brain,
        memory: memory.clone(),
        booking,
        resource,
        mark,
        accounts,
      });
    }
  }
  Ok((tellings, leaving))
}

/// Every booking telling not yet made, made once: the keepers of a booking that came to stand requested, the
/// booker of one decided. A refused booking leaves once its booker is told. A telling the receiver does not
/// take is made again at the next beat.
pub async fn tell_due(app: &Arc<App>) -> Result<Vec<ToldBooking>, ApiError> {
  let (tellings, mut leaving) = due(app)?;
  let mut told = Vec::new();
  for one in tellings {
    let kind = format!("booking:{}:{}", one.booking.id, one.mark);
    let booker = booker_name(app, &one.booking.booker).await;
    let order = |listeners: &[alarms::Listener]| order(&one, &booker, listeners);
    match alarms::tell(app, &one.accounts, &one.booking.title, &kind, order).await {
      Ok(rooms) => {
        let now = now_ms();
        app
          .store
          .mark_told(one.brain, &one.booking.id, &one.mark, now, now)
          .map_err(ApiError::failed)?;
        if one.booking.state == BookingState::Refused {
          leaving.push((one.realm.clone(), one.brain, one.memory.clone()));
        }
        told.push(ToldBooking {
          company: one.booking.company.clone(),
          booking_id: one.booking.id.clone(),
          kind,
          rooms,
        });
      }
      Err(why) => eprintln!("bmo-calendar: the booking {} could not be told: {why}", one.booking.id),
    }
  }
  for (realm, brain, memory) in leaving {
    let reason = match memory.label("reason") {
      "" => "refused".to_owned(),
      reason => format!("refused: {reason}"),
    };
    let moved = app.brain()?.done(brain, memory.id, "calendar", &reason).await?;
    app.take_in(&realm, &moved);
  }
  Ok(told)
}

/// How a booker is named in an order: their name and account, else their account.
async fn booker_name(app: &App, booker: &str) -> String {
  match peers::user(&app.http, &app.settings, booker).await {
    Ok(Some(user)) => match user.name {
      Some(name) => format!("{name} (account {booker})"),
      None => format!("the member with account {booker}"),
    },
    _ => format!("the member with account {booker}"),
  }
}

/// The order a room's supervisor is handed for a booking, in English: a marker naming the telling, then the
/// booking's data alone.
fn order(one: &Telling, booker: &str, listeners: &[alarms::Listener]) -> String {
  let booking = &one.booking;
  let resource = match &one.resource {
    Some(resource) if resource.place.is_empty() => format!("\"{}\"", resource.name),
    Some(resource) => format!("\"{}\" ({})", resource.name, resource.place),
    None => format!("the resource {}", booking.resource_id),
  };
  let start = booking.start.replace('T', " ");
  let end = booking.end.replace('T', " ");
  let when = if booking.all_day {
    format!("from {} to {}", &start[..10.min(start.len())], &end[..10.min(end.len())])
  } else {
    format!("from {start} to {end} (local time)")
  };
  let repeat = match booking.repeat {
    Repeat::None => String::new(),
    repeat => format!(", repeating {}", crate::model::label_word(&repeat)),
  };
  let mut text = if one.mark.starts_with("requested:") {
    format!(
      "CALENDAR BOOKING REQUEST. The booking \"{}\" of {resource} {when}{repeat} by {booker} waits for a keeper's approval.",
      booking.title
    )
  } else {
    let decided = match (booking.state, &booking.reason) {
      (BookingState::Refused, Some(reason)) => format!("was refused: \"{reason}\""),
      (BookingState::Refused, None) => "was refused".to_owned(),
      _ => "was approved".to_owned(),
    };
    format!(
      "CALENDAR BOOKING DECISION. The booking \"{}\" of {resource} {when}{repeat} {decided}.",
      booking.title
    )
  };
  for listener in listeners {
    text.push_str(&listener.named());
  }
  text
}
