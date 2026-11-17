//! The companies (`docs/design.md`, 「Two realms, kept apart」, 「The move into companies」; `docs/api.md`,
//! 「Companies」): the companies a caller reads, and the one move at start of the organization's calendars and
//! events from the platform's own brain into the brains of their creators' companies.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use axum::Router;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Json;
use axum::routing::get;
use serde_json::json;

use crate::api::{Answer, Named, Served, asked_organization, query};
use crate::app::App;
use crate::cache::RealmKey;
use crate::error::ApiError;
use crate::model::{CALENDAR_TYPE, MIGRATION_TYPE, Memory};
use crate::peers;
use crate::realm::PLATFORM;
use crate::wire::{AccountQuery, Company, Page};

/// The key the record of the move is kept under in the platform's brain.
pub const MOVE_KEY: &str = "migration:company-realms";

/// The name the move writes under.
const ACTOR: &str = "calendar";

/// The route of the companies, under the same guard as the screens' others.
pub fn routes() -> Router<Served> {
  Router::new().route("/{realm}/companies", get(list_companies))
}

async fn list_companies(
  State(served): State<Served>,
  headers: HeaderMap,
  Path(realm): Path<String>,
  given: Result<Query<AccountQuery>, QueryRejection>,
) -> Answer<Json<Page<Company>>> {
  let app = &served.app;
  let given = query(given)?;
  let asked =
    asked_organization(app, &headers, &realm, Named::of(given.account, given.holder_name, None)).await?;
  let own = match asked.account.as_deref() {
    Some(account) => peers::company_of(&app.http, &app.settings, account).await?,
    None => None,
  };
  let mut items: Vec<Company> = Vec::new();
  if let Some(own) = &own {
    let manages = asked.caller.administrator
      || match asked.account.as_deref() {
        Some(account) => peers::heads(&app.http, &app.settings, &own.id, account).await?,
        None => false,
      };
    items.push(Company {
      id: own.id.clone(),
      name: own.name.clone(),
      own: true,
      manages,
    });
  }
  if !asked.caller.administrator {
    return Ok(Json(Page { items, more: false }));
  }
  let mut others: BTreeMap<String, String> = BTreeMap::new();
  let asker = match &asked.account {
    Some(account) => Some(account.clone()),
    None => peers::owner_account(&app.http, &app.settings).await?,
  };
  if let Some(asker) = asker {
    for company in peers::companies(&app.http, &app.settings, &asker).await? {
      others.insert(company.id, company.name);
    }
  }
  if let Ok(cache) = app.cache.lock() {
    for realm in cache.realms.keys() {
      if let RealmKey::Company(company) = realm {
        others.entry(company.clone()).or_insert_with(|| company.clone());
      }
    }
  }
  let mut rest: Vec<Company> = others
    .into_iter()
    .filter(|(id, _)| own.as_ref().is_none_or(|own| own.id != *id))
    .map(|(id, name)| Company {
      id,
      name,
      own: false,
      manages: true,
    })
    .collect();
  rest.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
  items.extend(rest);
  items.push(Company {
    id: PLATFORM.to_owned(),
    name: app.settings.calendars.platform_name.clone(),
    own: false,
    manages: false,
  });
  Ok(Json(Page { items, more: false }))
}

// ---------------------------------------------------------------------------------------------------------
// The move into companies

/// What the move did, as its record keeps it.
#[derive(Default)]
struct Moved {
  events: usize,
  calendars: usize,
  events_stayed: usize,
  calendars_stayed: usize,
}

/// The live memories of a type a realm holds now.
fn held(app: &App, realm: &RealmKey, calendars: bool) -> Vec<Memory> {
  let Ok(cache) = app.cache.lock() else {
    return Vec::new();
  };
  cache
    .realms
    .get(realm)
    .map(|held| {
      let map = if calendars { &held.calendars } else { &held.events };
      map.values().cloned().collect()
    })
    .unwrap_or_default()
}

/// Whether a company's realm holds a live memory at a key.
fn holds(app: &App, realm: &RealmKey, memory: &Memory) -> bool {
  let Some(id) = memory.own_id() else {
    return false;
  };
  let calendars = memory.kind == CALENDAR_TYPE;
  held(app, realm, calendars).iter().any(|standing| standing.own_id() == Some(id))
}

/// A memory written in a company's brain as it stands in the platform's: its type, key, words, labels and
/// `validFrom`; nothing where the company already holds it live.
async fn copy(app: &Arc<App>, realm: &RealmKey, memory: &Memory) -> Result<(), ApiError> {
  if holds(app, realm, memory) {
    return Ok(());
  }
  let brain = app
    .realm_brain(realm, true)
    .await?
    .ok_or_else(|| ApiError::failed("no brain was made"))?;
  let body = json!({
    "key": memory.key,
    "type": memory.kind,
    "text": memory.text,
    "validFrom": memory.valid_from,
    "actorName": ACTOR,
    "labels": memory.labels,
  });
  let written = app.brain()?.write(brain, &body).await?;
  app.take_in(realm, &written);
  Ok(())
}

/// The company an account stands in, asked once for each account.
async fn company(
  app: &App,
  known: &mut HashMap<String, Option<String>>,
  account: &str,
) -> Result<Option<String>, ApiError> {
  if account.is_empty() {
    return Ok(None);
  }
  if let Some(company) = known.get(account) {
    return Ok(company.clone());
  }
  let company = peers::company_of(&app.http, &app.settings, account).await?.map(|company| company.id);
  known.insert(account.to_owned(), company.clone());
  Ok(company)
}

/// The move into companies, once: each live organization event of the platform's brain goes to its creator's
/// company with its told marks, each calendar to its creator's company and to every company one of its events
/// went to, and the record is written, the platform's brain made for it where none stands, so whatever the
/// platform's own realm comes to hold later is never moved. A start that finds the record moves nothing.
pub async fn migrate(app: &Arc<App>) -> Result<(), ApiError> {
  let platform = app
    .realm_brain(&RealmKey::Platform, true)
    .await?
    .ok_or_else(|| ApiError::failed("no brain was made"))?;
  let calls = app.brain()?;
  if calls
    .memories(platform, MIGRATION_TYPE)
    .await?
    .iter()
    .any(|memory| memory.key.as_deref() == Some(MOVE_KEY))
  {
    return Ok(());
  }
  let mut known: HashMap<String, Option<String>> = HashMap::new();
  let mut moved = Moved::default();
  let calendars = held(app, &RealmKey::Platform, true);
  let mut wanted: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
  for calendar in &calendars {
    if let Some(company) = company(app, &mut known, calendar.label("creator")).await? {
      wanted
        .entry(calendar.own_id().unwrap_or_default().to_owned())
        .or_default()
        .insert(company);
    }
  }
  for event in held(app, &RealmKey::Platform, false) {
    let Some(company) = company(app, &mut known, event.label("creator")).await? else {
      moved.events_stayed += 1;
      continue;
    };
    let realm = RealmKey::Company(company.clone());
    let calendar_id = event.label("calendar");
    if let Some(calendar) = calendars.iter().find(|calendar| calendar.own_id() == Some(calendar_id)) {
      copy(app, &realm, calendar).await?;
      wanted.entry(calendar_id.to_owned()).or_default().insert(company.clone());
    }
    copy(app, &realm, &event).await?;
    let to = app
      .realm_brain(&realm, false)
      .await?
      .ok_or_else(|| ApiError::failed("no brain was made"))?;
    let event_id = event.own_id().unwrap_or_default();
    app.store.copy_told(platform, to, event_id).map_err(ApiError::failed)?;
    let reason = format!("moved to company {company}");
    let closed = calls.done(platform, event.id, ACTOR, &reason).await?;
    app.take_in(&RealmKey::Platform, &closed);
    moved.events += 1;
  }
  let staying: BTreeSet<String> = held(app, &RealmKey::Platform, false)
    .iter()
    .map(|event| event.label("calendar").to_owned())
    .collect();
  for calendar in &calendars {
    let id = calendar.own_id().unwrap_or_default();
    let companies = wanted.get(id).cloned().unwrap_or_default();
    for company in &companies {
      copy(app, &RealmKey::Company(company.clone()), calendar).await?;
    }
    if companies.is_empty() || staying.contains(id) {
      moved.calendars_stayed += 1;
      continue;
    }
    let reason = format!("moved to companies {}", companies.into_iter().collect::<Vec<_>>().join(","));
    let closed = calls.done(platform, calendar.id, ACTOR, &reason).await?;
    app.take_in(&RealmKey::Platform, &closed);
    moved.calendars += 1;
  }
  let record = json!({
    "key": MOVE_KEY,
    "type": MIGRATION_TYPE,
    "text": "The organization's calendars and events stand in their companies' realms.",
    "actorName": ACTOR,
    "labels": {
      "movedEvents": moved.events.to_string(),
      "movedCalendars": moved.calendars.to_string(),
      "stayedEvents": moved.events_stayed.to_string(),
      "stayedCalendars": moved.calendars_stayed.to_string(),
    },
  });
  calls.write(platform, &record).await?;
  Ok(())
}
