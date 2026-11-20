//! The calendar tool (`docs/api.md`, 「The tool」): what the tool host hands a call and what it answers
//! (bmo-pocket's `docs/mcp-host.md`, 「A call」), and each entry as one call of a calendar route, made with the
//! service token the deployment writes into the data root's `env/calendar.env`. Every call names the session's
//! holder, which the service answers with the account agency names as its owner, else the owner's: that
//! account's own realm, its company's organization realm, its bookings and its own Outlook day.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::{Duration, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// The protocol version every manifest and every handed object names.
pub const PROTOCOL: u32 = 1;

/// The entries this executable answers, as its manifest declares them.
pub const ENTRIES: [&str; 11] = [
  "calendar_list",
  "calendar_events",
  "calendar_show",
  "calendar_add",
  "calendar_change",
  "calendar_cancel",
  "calendar_resources",
  "calendar_free_resources",
  "calendar_book",
  "calendar_cancel_booking",
  "calendar_outlook_day",
];

/// The data root handed in the record's variables.
pub const DATA_ROOT_VARIABLE: &str = "BMO_DATA_DIR";
/// The env file under the data root the service's address and token are read from.
pub const ENV_FILE: &str = "env/calendar.env";

/// How many events the next-events reading answers where it names no count.
const NEXT_DEFAULT: usize = 10;
/// The farthest a next-events reading looks, in days.
const SPAN_DAYS: i64 = 92;

const LOCAL_SHAPE: &str = "%Y-%m-%dT%H:%M:%S";

/// One session's record, as much of it as the tool reads.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
  #[serde(default)]
  pub holder_name: String,
  #[serde(default)]
  pub variables: BTreeMap<String, String>,
}

/// One call handed by the host.
#[derive(Clone, Debug, Deserialize)]
pub struct Handed {
  pub protocol: u32,
  #[serde(default)]
  pub kind: String,
  #[serde(default)]
  pub entry: String,
  #[serde(default)]
  pub arguments: Value,
  #[serde(default)]
  pub session: Session,
}

/// The host's tool result.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Answer {
  pub content: Vec<Value>,
  #[serde(rename = "isError")]
  pub is_error: bool,
  #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
  pub structured: Option<Value>,
}

impl Answer {
  /// Lines a person reads, and the value a program reads beside them.
  pub fn reading(lines: Vec<String>, structured: Value) -> Self {
    let text = if lines.is_empty() { "Nothing stands there.".to_owned() } else { lines.join("\n") };
    Self {
      content: vec![json!({ "type": "text", "text": text })],
      is_error: false,
      structured: Some(structured),
    }
  }

  /// A failure saying why in English.
  pub fn refusal(why: impl Into<String>) -> Self {
    Self {
      content: vec![json!({ "type": "text", "text": why.into() })],
      is_error: true,
      structured: None,
    }
  }

  /// The text the answer carries.
  pub fn text(&self) -> String {
    self
      .content
      .iter()
      .filter_map(|piece| piece["text"].as_str())
      .collect::<Vec<_>>()
      .join("\n")
  }
}

/// The calendar service as the tool reaches it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Service {
  pub url: String,
  pub token: String,
}

/// The data root: the record's, else this process's, else the deployment's conventional one under the home.
pub fn data_root(session: &Session) -> PathBuf {
  session
    .variables
    .get(DATA_ROOT_VARIABLE)
    .map(PathBuf::from)
    .or_else(|| std::env::var(DATA_ROOT_VARIABLE).ok().map(PathBuf::from))
    .or_else(|| std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".local/share/bmo/data")))
    .unwrap_or_else(|| PathBuf::from("/nonexistent"))
}

fn env_file(path: &Path) -> BTreeMap<String, String> {
  std::fs::read_to_string(path)
    .map(|text| {
      text
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect()
    })
    .unwrap_or_default()
}

/// The service: `CALENDAR_ADDRESS` and `CALENDAR_TOKEN` of the call's environment, else of the data root's
/// `env/calendar.env`.
pub fn service(session: &Session) -> Result<Service, String> {
  let file = env_file(&data_root(session).join(ENV_FILE));
  let variable = |name: &str| {
    std::env::var(name)
      .ok()
      .or_else(|| file.get(name).cloned())
      .map(|value| value.trim().to_owned())
      .filter(|value| !value.is_empty())
  };
  match (variable("CALENDAR_ADDRESS"), variable("CALENDAR_TOKEN")) {
    (Some(url), Some(token)) => Ok(Service {
      url: url.trim_end_matches('/').to_owned(),
      token,
    }),
    _ => Err(format!(
      "the calendar's address or token is set nowhere: no CALENDAR_ADDRESS and CALENDAR_TOKEN in {ENV_FILE}"
    )),
  }
}

/// The last part of a full name, `<namespace>/<name>`.
pub fn entry_name(entry: &str) -> &str {
  entry.rsplit('/').next().unwrap_or(entry)
}

/// One call of a route: its answer as JSON, or the service's refusal in words.
async fn call(
  service: &Service,
  method: reqwest::Method,
  path: &str,
  query: &[(&str, String)],
  body: Option<&Value>,
) -> Result<Value, String> {
  let mut request = reqwest::Client::builder()
    .no_proxy()
    .build()
    .map_err(|why| format!("no HTTP client: {why}"))?
    .request(method, format!("{}{path}", service.url))
    .bearer_auth(&service.token)
    .query(query);
  if let Some(body) = body {
    request = request.json(body);
  }
  let response = request
    .send()
    .await
    .map_err(|why| format!("the calendar could not be reached: {why}"))?;
  let status = response.status();
  let text = response
    .text()
    .await
    .map_err(|why| format!("the calendar's answer could not be read: {why}"))?;
  if !status.is_success() {
    let said: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let code = said["code"].as_str().unwrap_or("error");
    let error = said["error"].as_str().unwrap_or(text.as_str());
    return Err(format!("{error} ({code})"));
  }
  if text.trim().is_empty() {
    return Ok(Value::Null);
  }
  serde_json::from_str(&text).map_err(|why| format!("the calendar's answer does not read: {why}"))
}

fn text<'a>(arguments: &'a Value, name: &str) -> Option<&'a str> {
  arguments[name].as_str().filter(|value| !value.is_empty())
}

/// One event as one line.
pub fn event_line(event: &Value) -> String {
  let mut line = format!(
    "{} to {} · {} · calendar {} · id {}",
    event["start"].as_str().unwrap_or(""),
    event["end"].as_str().unwrap_or(""),
    event["title"].as_str().unwrap_or(""),
    event["calendarId"].as_str().unwrap_or(""),
    event["id"].as_str().unwrap_or(""),
  );
  if let Some(occurrence) = event["occurrence"].as_str() {
    line.push_str(&format!(" · occurrence {occurrence}"));
  }
  if event["repeat"].as_str().is_some_and(|repeat| repeat != "none") {
    line.push_str(&format!(" · repeats {}", event["repeat"].as_str().unwrap_or("")));
  }
  if event["alarm"].as_str().is_some_and(|alarm| alarm != "none") {
    line.push_str(&format!(" · alarm {} min before", event["alarm"].as_str().unwrap_or("")));
  }
  line
}

/// One event whole, as lines.
fn event_lines(event: &Value) -> Vec<String> {
  let mut lines = vec![event_line(event)];
  if let Some(description) = event["description"].as_str().filter(|text| !text.is_empty()) {
    lines.push(description.to_owned());
  }
  lines
}

/// One resource as one line.
pub fn resource_line(resource: &Value) -> String {
  let mut line = format!(
    "{} · {} · {} seats · id {}",
    resource["name"].as_str().unwrap_or(""),
    resource["kind"].as_str().unwrap_or(""),
    resource["seats"].as_u64().unwrap_or(0),
    resource["id"].as_str().unwrap_or(""),
  );
  if let Some(place) = resource["place"].as_str().filter(|place| !place.is_empty()) {
    line.push_str(&format!(" · at {place}"));
  }
  if resource["approval"].as_bool() == Some(true) {
    line.push_str(" · a booking waits for a keeper's approval");
  }
  line
}

/// One booking as one line.
pub fn booking_line(booking: &Value) -> String {
  let mut line = format!(
    "{} to {} · {} · resource {} · {} · id {}",
    booking["start"].as_str().unwrap_or(""),
    booking["end"].as_str().unwrap_or(""),
    booking["title"].as_str().unwrap_or(""),
    booking["resourceId"].as_str().unwrap_or(""),
    booking["state"].as_str().unwrap_or(""),
    booking["id"].as_str().unwrap_or(""),
  );
  if booking["repeat"].as_str().is_some_and(|repeat| repeat != "none") {
    line.push_str(&format!(" · repeats {}", booking["repeat"].as_str().unwrap_or("")));
  }
  line
}

/// One event of an Outlook day as one line.
pub fn outlook_line(event: &Value) -> String {
  let time = if event["allDay"].as_bool() == Some(true) {
    "all day".to_owned()
  } else {
    let clock = |field: &str| event[field].as_str().unwrap_or("").get(11..16).unwrap_or("").to_owned();
    format!("{} to {}", clock("start"), clock("end"))
  };
  let mut line = format!("{time} · {}", event["title"].as_str().unwrap_or(""));
  let parts = &event["titleParts"];
  if let Some(levels) = parts["levels"].as_array().filter(|levels| !levels.is_empty()) {
    let levels: Vec<&str> = levels.iter().filter_map(Value::as_str).collect();
    line.push_str(&format!(" · work code levels {}", levels.join(" ")));
    if let Some(group) = parts["group"].as_str() {
      line.push_str(&format!(" · group {group}"));
    }
    line.push_str(&format!(" · task {}", parts["task"].as_str().unwrap_or("")));
  }
  if let Some(organizer) = event["organizer"].as_str().filter(|organizer| !organizer.is_empty()) {
    line.push_str(&format!(" · organized by {organizer}"));
  }
  line.push_str(&format!(" · {} attendees", event["attendees"].as_u64().unwrap_or(0)));
  if let Some(room) = event["room"].as_str().filter(|room| !room.is_empty()) {
    line.push_str(&format!(" · at {room}"));
  }
  line
}

fn resource_lines(page: &Value) -> Vec<String> {
  page["items"]
    .as_array()
    .map(|items| items.iter().map(resource_line).collect())
    .unwrap_or_default()
}

/// The end an added event takes where none is given: an hour after the start, or the next day's start where
/// it takes the whole day.
pub fn default_end(start: &str, all_day: bool) -> Option<String> {
  let start = NaiveDateTime::parse_from_str(start, LOCAL_SHAPE).ok()?;
  let end = if all_day {
    start.date().and_hms_opt(0, 0, 0)? + Duration::days(1)
  } else {
    start + Duration::hours(1)
  };
  Some(end.format(LOCAL_SHAPE).to_string())
}

/// The fields a write names, from the arguments: every one given but the realm, the id and the occurrence.
fn fields(arguments: &Value) -> Map<String, Value> {
  arguments
    .as_object()
    .map(|all| {
      all
        .iter()
        .filter(|(name, value)| {
          !matches!(name.as_str(), "realm" | "id" | "occurrence" | "n") && !value.is_null()
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
    })
    .unwrap_or_default()
}

/// One entry answered against the service.
pub async fn answer(handed: &Handed, service: &Service) -> Answer {
  match answered(handed, service).await {
    Ok(answer) => answer,
    Err(why) => Answer::refusal(why),
  }
}

async fn answered(handed: &Handed, service: &Service) -> Result<Answer, String> {
  let arguments = &handed.arguments;
  let mut who: Vec<(&str, String)> = Vec::new();
  if !handed.session.holder_name.is_empty() {
    who.push(("holderName", handed.session.holder_name.clone()));
  }
  match entry_name(&handed.entry) {
    "calendar_resources" | "calendar_free_resources" | "calendar_book" | "calendar_cancel_booking" => {
      booking_entry(entry_name(&handed.entry), arguments, &who, service).await
    }
    "calendar_outlook_day" => outlook_day(arguments, &who, service).await,
    entry => event_entry(entry, arguments, &who, service).await,
  }
}

/// The entries of a company's resources and bookings, each in the organization realm of the account the holder
/// acts for.
async fn booking_entry(
  entry: &str,
  arguments: &Value,
  who: &[(&str, String)],
  service: &Service,
) -> Result<Answer, String> {
  use reqwest::Method;

  match entry {
    "calendar_resources" => {
      let mut query = who.to_vec();
      if let Some(kind) = text(arguments, "kind") {
        query.push(("kind", kind.to_owned()));
      }
      let page = call(service, Method::GET, "/organization/resources", &query, None).await?;
      Ok(Answer::reading(resource_lines(&page), page))
    }
    "calendar_free_resources" => {
      let start = text(arguments, "start").ok_or("start is the span's local start")?;
      let end = match text(arguments, "end") {
        Some(end) => end.to_owned(),
        None => {
          default_end(start, false).ok_or("start is no local datetime of the shape YYYY-MM-DDTHH:mm:ss")?
        }
      };
      let mut query = who.to_vec();
      query.push(("start", start.to_owned()));
      query.push(("end", end));
      if let Some(seats) = arguments["seats"].as_u64() {
        query.push(("seats", seats.to_string()));
      }
      for name in ["repeat", "kind"] {
        if let Some(value) = text(arguments, name) {
          query.push((name, value.to_owned()));
        }
      }
      let page = call(service, Method::GET, "/organization/resources/free", &query, None).await?;
      Ok(Answer::reading(resource_lines(&page), page))
    }
    "calendar_book" => {
      let mut body = fields(arguments);
      if text(arguments, "end").is_none() {
        let start = text(arguments, "start").ok_or("start is the booking's local start")?;
        let end = default_end(start, arguments["allDay"].as_bool().unwrap_or(false))
          .ok_or("start is no local datetime of the shape YYYY-MM-DDTHH:mm:ss")?;
        body.insert("end".into(), json!(end));
      }
      let booking =
        call(service, Method::POST, "/organization/bookings", who, Some(&Value::Object(body))).await?;
      let said = match booking["state"].as_str() {
        Some("requested") => {
          "Requested: it waits for a keeper's approval, and the booker is told the answer."
        }
        _ => "Booked.",
      };
      Ok(Answer::reading(vec![said.to_owned(), booking_line(&booking)], booking))
    }
    _ => {
      let id = text(arguments, "bookingId").ok_or("bookingId names the booking")?;
      let mut query = who.to_vec();
      if let Some(occurrence) = text(arguments, "occurrence") {
        query.push(("occurrence", occurrence.to_owned()));
      }
      call(service, Method::DELETE, &format!("/organization/bookings/{id}"), &query, None).await?;
      Ok(Answer::reading(vec!["Cancelled.".to_owned()], json!({ "cancelled": true })))
    }
  }
}

/// The day of the person's own Outlook calendar, read on their own node.
async fn outlook_day(
  arguments: &Value,
  who: &[(&str, String)],
  service: &Service,
) -> Result<Answer, String> {
  let mut query = who.to_vec();
  if let Some(date) = text(arguments, "date") {
    query.push(("date", date.to_owned()));
  }
  let day = call(service, reqwest::Method::GET, "/outlook/day", &query, None).await?;
  let mut lines = vec![format!(
    "{} on {} ({}), read from {}:",
    day["date"].as_str().unwrap_or(""),
    day["node"].as_str().unwrap_or(""),
    day["zone"].as_str().unwrap_or(""),
    match day["road"].as_str() {
      Some("outlook") => "the running Outlook",
      _ => "Microsoft Graph",
    }
  )];
  match day["items"].as_array().filter(|items| !items.is_empty()) {
    Some(items) => lines.extend(items.iter().map(outlook_line)),
    None => lines.push("No event stands on that day.".to_owned()),
  }
  Ok(Answer::reading(lines, day))
}

/// The entries of a realm's calendars and events.
async fn event_entry(
  entry: &str,
  arguments: &Value,
  who: &[(&str, String)],
  service: &Service,
) -> Result<Answer, String> {
  use reqwest::Method;

  let realm = text(arguments, "realm").ok_or("realm is mine or organization")?;
  if !matches!(realm, "mine" | "organization") {
    return Err("realm is mine or organization".into());
  }
  let who = who.to_vec();
  let id = || text(arguments, "id").ok_or_else(|| "id names the event".to_owned());
  match entry {
    "calendar_list" => {
      let page = call(service, Method::GET, &format!("/{realm}/calendars"), &who, None).await?;
      let lines = page["items"]
        .as_array()
        .map(|items| {
          items
            .iter()
            .map(|calendar| {
              format!(
                "{} · {} · {}",
                calendar["id"].as_str().unwrap_or(""),
                calendar["name"].as_str().unwrap_or(""),
                calendar["color"].as_str().unwrap_or("")
              )
            })
            .collect()
        })
        .unwrap_or_default();
      Ok(Answer::reading(lines, page))
    }
    "calendar_events" => {
      let (start, end, most) = match (text(arguments, "start"), text(arguments, "end")) {
        (Some(start), Some(end)) => (start.to_owned(), end.to_owned(), None),
        (None, None) => {
          let now = Local::now().naive_local();
          let most = arguments["n"].as_u64().map(|n| n as usize).unwrap_or(NEXT_DEFAULT);
          (
            now.format(LOCAL_SHAPE).to_string(),
            (now + Duration::days(SPAN_DAYS)).format(LOCAL_SHAPE).to_string(),
            Some(most),
          )
        }
        _ => return Err("a span names both start and end".into()),
      };
      let mut query = who.clone();
      query.push(("start", start));
      query.push(("end", end));
      let mut page = call(service, Method::GET, &format!("/{realm}/events"), &query, None).await?;
      if let (Some(most), Some(items)) = (most, page["items"].as_array_mut()) {
        let more = items.len() > most;
        items.truncate(most);
        page["more"] = json!(more);
      }
      let lines = page["items"]
        .as_array()
        .map(|items| items.iter().map(event_line).collect())
        .unwrap_or_default();
      Ok(Answer::reading(lines, page))
    }
    "calendar_show" => {
      let event = call(service, Method::GET, &format!("/{realm}/events/{}", id()?), &who, None).await?;
      Ok(Answer::reading(event_lines(&event), event))
    }
    "calendar_add" => {
      let mut body = fields(arguments);
      if text(arguments, "calendarId").is_none() {
        let page = call(service, Method::GET, &format!("/{realm}/calendars"), &who, None).await?;
        let first = page["items"][0]["id"].as_str().ok_or("the realm holds no calendar")?;
        body.insert("calendarId".into(), json!(first));
      }
      if text(arguments, "end").is_none() {
        let start = text(arguments, "start").ok_or("start is the event's local start")?;
        let end = default_end(start, arguments["allDay"].as_bool().unwrap_or(false))
          .ok_or("start is no local datetime of the shape YYYY-MM-DDTHH:mm:ss")?;
        body.insert("end".into(), json!(end));
      }
      let event =
        call(service, Method::POST, &format!("/{realm}/events"), &who, Some(&Value::Object(body))).await?;
      let mut lines = vec!["Added.".to_owned()];
      lines.extend(event_lines(&event));
      Ok(Answer::reading(lines, event))
    }
    "calendar_change" => {
      let mut body = fields(arguments);
      if let Some(occurrence) = text(arguments, "occurrence") {
        body.insert("occurrence".into(), json!(occurrence));
      }
      let path = format!("/{realm}/events/{}", id()?);
      let event = call(service, Method::PATCH, &path, &who, Some(&Value::Object(body))).await?;
      let mut lines = vec!["Changed.".to_owned()];
      lines.extend(event_lines(&event));
      Ok(Answer::reading(lines, event))
    }
    "calendar_cancel" => {
      let mut query = who.clone();
      if let Some(occurrence) = text(arguments, "occurrence") {
        query.push(("occurrence", occurrence.to_owned()));
      }
      call(service, Method::DELETE, &format!("/{realm}/events/{}", id()?), &query, None).await?;
      Ok(Answer::reading(vec!["Cancelled.".to_owned()], json!({ "cancelled": true })))
    }
    other => Err(format!("the calendar tool answers no entry {other}")),
  }
}

/// The one call this process was handed, answered on standard output.
pub async fn run() -> std::process::ExitCode {
  let mut text = String::new();
  let handed: Result<Handed, String> = std::io::stdin()
    .lock()
    .read_to_string(&mut text)
    .map_err(|why| format!("the call could not be read: {why}"))
    .and_then(|_| serde_json::from_str(&text).map_err(|why| format!("the call does not read: {why}")));
  let answer = match handed {
    Ok(handed) if handed.protocol != PROTOCOL => {
      Answer::refusal(format!("protocol {} is not spoken", handed.protocol))
    }
    Ok(handed) => match service(&handed.session) {
      Ok(service) => answer(&handed, &service).await,
      Err(why) => Answer::refusal(why),
    },
    Err(why) => Answer::refusal(why),
  };
  let text = serde_json::to_string(&answer).unwrap_or_default();
  match std::io::stdout().lock().write_all(text.as_bytes()) {
    Ok(()) => std::process::ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("the answer could not be written: {why}");
      std::process::ExitCode::FAILURE
    }
  }
}
