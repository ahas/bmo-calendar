//! The platform administrator's deletion of an account or a company for good: the realm the calendar keeps of it,
//! counted and let go of (`docs/api.md`, 「Removing an account or a company」).

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get};
use axum::{Json, Router};
use std::collections::BTreeMap;

use crate::api::{Answer, Credential, Served, credential};
use crate::app::App;
use crate::cache::RealmKey;
use crate::error::ApiError;
use crate::wire::{RemovalCounts, Removed, UnitRemovalQuery as UnitQuery};

/// The role a console call names to reach a removal.
const ADMINISTRATOR: &str = "administrator";

/// The removal routes, outside the screens' guard.
pub fn routes() -> Router<Served> {
  Router::new()
    .route("/accounts/{account}/removal", get(account_removal))
    .route("/accounts/{account}", delete(remove_account))
    .route("/org-units/{unit}/removal", get(unit_removal))
    .route("/org-units/{unit}", delete(remove_unit))
}

// The service token, or the console credential naming the administrator's role; anyone else is refused.
fn administrator(app: &App, headers: &HeaderMap) -> Answer<()> {
  if credential(app, headers)? == Credential::Service {
    return Ok(());
  }
  let role = headers.get("owner-role").and_then(|value| value.to_str().ok()).map(str::trim);
  if role == Some(ADMINISTRATOR) {
    Ok(())
  } else {
    Err(ApiError::forbidden("A removal is the platform administrator's alone."))
  }
}

// An account's, a unit's or a company's id: positive decimal digits.
fn key(given: &str) -> Answer<&str> {
  let digits = !given.is_empty() && given.len() <= 20 && given.bytes().all(|byte| byte.is_ascii_digit());
  if digits && given.bytes().any(|byte| byte != b'0') {
    Ok(given)
  } else {
    Err(ApiError::bad_request(format!("{given:?} is no account's, unit's or company's id.")))
  }
}

// The realm a unit's removal reaches: its company's, where the unit is the company; none for a unit below one.
fn unit_realm(unit: &str, query: &UnitQuery) -> Answer<Option<RealmKey>> {
  let unit = key(unit)?;
  match query.company.as_deref() {
    Some(company) if key(company)? == unit => Ok(Some(RealmKey::Company(unit.to_owned()))),
    _ => Ok(None),
  }
}

// What a realm holds live, by the kinds its removal answers; read from the brain where it is not held yet.
async fn held(app: &Arc<App>, realm: Option<&RealmKey>, company: bool) -> Answer<BTreeMap<String, u64>> {
  let kinds: &[&str] = if company {
    &["calendars", "events", "schedules", "resources", "bookings"]
  } else {
    &["calendars", "events"]
  };
  let mut counts: BTreeMap<String, u64> = kinds.iter().map(|kind| ((*kind).to_owned(), 0)).collect();
  let Some(realm) = realm else {
    return Ok(counts);
  };
  if app.realm_brain(realm, false).await?.is_none() {
    return Ok(counts);
  }
  let cache = app.cache.lock().map_err(|_| ApiError::failed("The cache's lock is poisoned."))?;
  if let Some(held) = cache.realms.get(realm) {
    for (kind, count) in counts.iter_mut() {
      let standing = match kind.as_str() {
        "calendars" => held.calendars.len(),
        "events" => held.events.len(),
        "schedules" => held.schedules.len(),
        "resources" => held.resources.len(),
        _ => held.bookings.len(),
      };
      *count = u64::try_from(standing).unwrap_or(u64::MAX);
    }
  }
  Ok(counts)
}

// A realm let go of: counted, its brain deleted on the brain, its told marks and its cache with it.
async fn let_go(app: &Arc<App>, realm: Option<&RealmKey>, company: bool) -> Answer<BTreeMap<String, u64>> {
  let counts = held(app, realm, company).await?;
  let Some(realm) = realm else {
    return Ok(counts);
  };
  let _held = app.realm_lock.lock().await;
  if let Some(brain) = app.held(realm) {
    app.brain()?.let_go(brain).await?;
    if let Ok(mut cache) = app.cache.lock() {
      cache.realms.remove(realm);
    }
    app.store.forget_brain(brain).map_err(ApiError::failed)?;
  }
  Ok(counts)
}

async fn account_removal(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(account): Path<String>,
) -> Answer<Json<RemovalCounts>> {
  administrator(&served.app, &headers)?;
  let realm = RealmKey::Mine(key(&account)?.to_owned());
  Ok(Json(RemovalCounts {
    counts: held(&served.app, Some(&realm), false).await?,
  }))
}

async fn remove_account(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(account): Path<String>,
) -> Answer<Json<Removed>> {
  administrator(&served.app, &headers)?;
  let realm = RealmKey::Mine(key(&account)?.to_owned());
  Ok(Json(Removed {
    removed: let_go(&served.app, Some(&realm), false).await?,
  }))
}

async fn unit_removal(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(unit): Path<String>,
  Query(query): Query<UnitQuery>,
) -> Answer<Json<RemovalCounts>> {
  administrator(&served.app, &headers)?;
  let realm = unit_realm(&unit, &query)?;
  Ok(Json(RemovalCounts {
    counts: held(&served.app, realm.as_ref(), true).await?,
  }))
}

async fn remove_unit(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(unit): Path<String>,
  Query(query): Query<UnitQuery>,
) -> Answer<Json<Removed>> {
  administrator(&served.app, &headers)?;
  let realm = unit_realm(&unit, &query)?;
  Ok(Json(Removed {
    removed: let_go(&served.app, realm.as_ref(), true).await?,
  }))
}
