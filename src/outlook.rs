//! A person's Outlook day (`docs/design.md` and `docs/api.md`, 「A person's Outlook day」): the account a tool
//! session's holder acts for, its own node, one PowerShell command run there through the node service, and the
//! day read from the running Outlook, else from Microsoft Graph with a token the page-token service lends for the person. Nothing read
//! is kept.

use axum::Router;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::Json;
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::{self, Answer, Served};
use crate::app::App;
use crate::error::ApiError;
use crate::peers;
use crate::title;
use crate::wire::{OutlookDay, OutlookDayQuery, OutlookEvent, OutlookRoad};

/// The command run on the node.
const SCRIPT: &str = include_str!("outlook_day.ps1");
/// The variable the command takes its day from.
const DAY_VARIABLE: &str = "BMO_CALENDAR_DAY";
/// The Graph scopes a page token that reads a calendar holds one of: the one that reads, and those that also
/// write.
const SCOPES: [&str; 4] = [
  "Calendars.Read",
  "Calendars.ReadWrite",
  "Calendars.Read.Shared",
  "Calendars.ReadWrite.Shared",
];
/// The fields of a Graph event read, and none of its content.
const GRAPH_FIELDS: &str = "subject,start,end,isAllDay,organizer,attendees,location,isCancelled";
/// How many Graph events a page is asked for.
const GRAPH_PAGE: &str = "100";
/// How many pages of one day are followed: a bound on what is read, not on time.
const GRAPH_PAGES: usize = 20;
/// The length of a local ISO datetime, `YYYY-MM-DDTHH:mm:ss`.
const LOCAL_LENGTH: usize = 19;
/// How much of what a failed command wrote to its error stream a refusal carries, in characters.
const SAID_LENGTH: usize = 500;

pub fn routes() -> Router<Served> {
  Router::new().route("/outlook/day", get(outlook_day))
}

/// What the command on the node answers: the day, the PC's zone and the day's bounds in UTC, whether Outlook was
/// read, and its events where it was.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeDay {
  date: String,
  zone: String,
  start_utc: String,
  end_utc: String,
  outlook: OutlookState,
  #[serde(default)]
  why: String,
  #[serde(default)]
  events: Vec<Found>,
}

/// Whether the command on the node read Outlook.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum OutlookState {
  /// Its events were read.
  Running,
  /// No Outlook process runs, so COM was not asked.
  NotRunning,
  /// One runs and could not be read.
  Unreachable,
}

/// One event as either road reads it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Found {
  title: String,
  start: String,
  end: String,
  all_day: bool,
  organizer: String,
  attendees: u32,
  room: String,
}

impl Found {
  fn event(self) -> OutlookEvent {
    OutlookEvent {
      title_parts: title::parts(&self.title),
      title: self.title,
      start: self.start,
      end: self.end,
      all_day: self.all_day,
      organizer: self.organizer,
      attendees: self.attendees,
      room: self.room,
    }
  }
}

async fn outlook_day(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Query<OutlookDayQuery>, QueryRejection>,
) -> Answer<Json<OutlookDay>> {
  let app = &served.app;
  let given: OutlookDayQuery = api::query(given)?;
  let caller = api::caller(app, &headers, None, given.holder_name)?;
  if !caller.service {
    return Err(ApiError::forbidden("A person's Outlook day is read through the tool alone."));
  }
  let date = day(given.date)?;
  let account = api::account_of(app, &caller)
    .await?
    .ok_or_else(|| ApiError::no_account("The call names no tool session's holder."))?;
  let place = place(app, &account).await?;
  let mut read = node_day(app, &place, date.as_deref()).await?;
  let (road, found) = if read.outlook == OutlookState::Running {
    (OutlookRoad::Outlook, std::mem::take(&mut read.events))
  } else {
    (OutlookRoad::Graph, graph_day(app, &place.account, &read).await?)
  };
  Ok(Json(OutlookDay {
    date: read.date,
    zone: read.zone,
    road,
    node: place.node,
    items: found.into_iter().map(Found::event).collect(),
  }))
}

/// The day named, `YYYY-MM-DD`, or none for the PC's today.
fn day(named: Option<String>) -> Answer<Option<String>> {
  let Some(named) = named.filter(|named| !named.trim().is_empty()) else {
    return Ok(None);
  };
  let reads = named.len() == 10 && NaiveDate::parse_from_str(&named, "%Y-%m-%d").is_ok();
  if !reads {
    return Err(ApiError::bad_request(format!("The date {named} is no day of the shape YYYY-MM-DD.")));
  }
  Ok(Some(named))
}

/// Where an account's day is read: its own node, and the account the node service and the page-token service know it by.
struct Place {
  node: String,
  account: String,
}

/// The account's place. For the deployment's own account, the node and the account the settings name, each where
/// they name one; for every account, the node agency's record names under the account it is known by, where the
/// record is not one several accounts share.
async fn place(app: &App, account: &str) -> Answer<Place> {
  let named_node = app.settings.outlook.owner_node.trim();
  let named_account = app.settings.outlook.owner_account.trim();
  let owner = (!named_node.is_empty() || !named_account.is_empty())
    && peers::owner_account(&app.http, &app.settings).await?.as_deref() == Some(account);
  let known_by = if owner && !named_account.is_empty() { named_account } else { account };
  if owner && !named_node.is_empty() {
    return Ok(Place {
      node: named_node.to_owned(),
      account: known_by.to_owned(),
    });
  }
  match peers::person_node(&app.http, &app.settings, known_by).await? {
    Some(record) if !record.shared => Ok(Place {
      node: record.node,
      account: known_by.to_owned(),
    }),
    Some(record) => Err(ApiError::no_node(format!(
      "Account {known_by}'s node {} is one several accounts share, so no node of its own.",
      record.node
    ))),
    None => Err(ApiError::no_node(format!("Account {known_by} has no node of its own."))),
  }
}

/// The command run on the account's node, its answer read off the last line it wrote.
async fn node_day(app: &App, place: &Place, date: Option<&str>) -> Answer<NodeDay> {
  let Place { node, account } = place;
  let mut body = json!({ "account": account, "command": SCRIPT, "shell": "powershell" });
  if let Some(date) = date {
    body["env"] = json!({ DAY_VARIABLE: date });
  }
  let run = peers::node_run(&app.http, &app.settings, node, &body).await?;
  if run["exitCode"].as_i64() != Some(0) {
    let said = run["stderr"].as_str().unwrap_or("").trim();
    let tail: String = said.chars().rev().take(SAID_LENGTH).collect::<Vec<_>>().into_iter().rev().collect();
    return Err(ApiError::upstream(format!(
      "The read on node {node} ended with {}: {tail}",
      run["exitCode"]
        .as_i64()
        .map_or_else(|| "no exit code".to_owned(), |code| format!("code {code}"))
    )));
  }
  if run["stdoutCut"].as_bool() == Some(true) {
    return Err(ApiError::upstream(format!(
      "The read on node {node} answered more than its run carries."
    )));
  }
  let last = run["stdout"].as_str().unwrap_or("").lines().rev().find(|line| !line.trim().is_empty());
  let bytes = STANDARD
    .decode(last.unwrap_or("").trim())
    .map_err(|why| ApiError::upstream(format!("The read on node {node} answered no base64: {why}")))?;
  serde_json::from_slice(&bytes).map_err(|why| {
    ApiError::upstream(format!("The read on node {node} answered nothing that reads: {why}"))
  })
}

/// The day read from Microsoft Graph with the account's page token, in the PC's own zone.
async fn graph_day(app: &App, account: &str, day: &NodeDay) -> Answer<Vec<Found>> {
  let outlook = if day.outlook == OutlookState::Unreachable {
    format!("Outlook runs on the node but could not be read ({})", day.why)
  } else {
    "Outlook is not running on the node".to_owned()
  };
  let token = peers::page_token(&app.http, &app.settings, account, &SCOPES)
    .await
    .map_err(|fault| ApiError::no_road(format!("{outlook}, and no page token was lent: {fault}")))?;
  let refused =
    |why: String| ApiError::no_road(format!("{outlook}, and Microsoft Graph refused the read: {why}"));
  let base = app.settings.outlook.graph_address.trim_end_matches('/');
  let mut address = reqwest::Url::parse(&format!("{base}/me/calendarView"))
    .map_err(|why| ApiError::failed(format!("Graph's address does not read: {why}")))?;
  address
    .query_pairs_mut()
    .append_pair("startDateTime", &day.start_utc)
    .append_pair("endDateTime", &day.end_utc)
    .append_pair("$select", GRAPH_FIELDS)
    .append_pair("$orderby", "start/dateTime")
    .append_pair("$top", GRAPH_PAGE);
  let zone = format!("outlook.timezone=\"{}\"", day.zone);
  let mut next = Some(address.to_string());
  let mut found = Vec::new();
  for _ in 0..GRAPH_PAGES {
    let Some(page_address) = next.take() else {
      break;
    };
    let response = app
      .http
      .get(&page_address)
      .bearer_auth(&token)
      .header("Prefer", &zone)
      .send()
      .await
      .map_err(|why| refused(why.to_string()))?;
    let status = response.status();
    let page: Value = response.json().await.map_err(|why| refused(why.to_string()))?;
    if !status.is_success() {
      let message = page["error"]["message"].as_str().unwrap_or("no reason given");
      return Err(refused(format!("{status}: {message}")));
    }
    let events = page["value"].as_array().map(Vec::as_slice).unwrap_or_default();
    found.extend(events.iter().filter(|event| event["isCancelled"] != true).map(graph_event));
    next = page["@odata.nextLink"].as_str().map(str::to_owned);
  }
  Ok(found)
}

/// One Graph event as the command on the node would have read it from Outlook.
fn graph_event(event: &Value) -> Found {
  let text = |value: &Value| value.as_str().unwrap_or("").to_owned();
  let local = |value: &Value| value.as_str().unwrap_or("").chars().take(LOCAL_LENGTH).collect();
  let address = |value: &Value| value["emailAddress"]["address"].as_str().unwrap_or("").to_lowercase();
  let organizer = address(&event["organizer"]);
  let attendees = event["attendees"]
    .as_array()
    .map(Vec::as_slice)
    .unwrap_or_default()
    .iter()
    .filter(|attendee| attendee["type"] != "resource" && address(attendee) != organizer)
    .count();
  Found {
    title: text(&event["subject"]),
    start: local(&event["start"]["dateTime"]),
    end: local(&event["end"]["dateTime"]),
    all_day: event["isAllDay"].as_bool().unwrap_or(false),
    organizer: text(&event["organizer"]["emailAddress"]["name"]),
    attendees: u32::try_from(attendees).unwrap_or(u32::MAX),
    room: text(&event["location"]["displayName"]),
  }
}
