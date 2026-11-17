//! The tool's manifest and its entries, each called against a stand-in calendar service that records what it
//! was asked.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as Route, RawQuery, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use calendar_tool::{Answer, ENTRIES, Handed, Service, Session, answer, default_end, service};
use serde_json::{Value, json};

const DIR: &str = env!("CARGO_MANIFEST_DIR");

/// One request the stand-in was asked: method, path, query and body.
type Asked = (String, String, String, Value);

#[derive(Clone, Default)]
struct Stand {
  asked: Arc<Mutex<Vec<Asked>>>,
}

fn event(id: &str, start: &str) -> Value {
  json!({ "id": id, "title": format!("Event {id}"), "start": start, "end": start, "calendarId": "mine",
    "occurrence": null, "repeat": "none", "alarm": "none", "description": "" })
}

async fn answer_for(
  State(stand): State<Stand>,
  method: Method,
  Route(rest): Route<String>,
  RawQuery(query): RawQuery,
  body: Bytes,
) -> Response {
  let query = query.unwrap_or_default();
  let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
  stand
    .asked
    .lock()
    .unwrap()
    .push((method.to_string(), format!("/{rest}"), query.clone(), body.clone()));
  if rest.starts_with("mine") && !query.contains("holderName=") {
    return (
      StatusCode::UNPROCESSABLE_ENTITY,
      Json(json!({ "code": "no-account", "error": "No account." })),
    )
      .into_response();
  }
  let parts: Vec<&str> = rest.split('/').collect();
  match (method.as_str(), parts.as_slice()) {
    ("GET", [_, "calendars"]) => {
      Json(json!({ "items": [{ "id": "first", "name": "Own", "color": "primary" }, { "id": "second", "name": "Side", "color": "info" }], "more": false }))
        .into_response()
    }
    ("GET", [_, "events"]) => Json(json!({
      "items": [event("a", "2026-03-01T09:00:00"), event("b", "2026-03-02T09:00:00"), event("c", "2026-03-03T09:00:00")],
      "more": false,
    }))
    .into_response(),
    ("GET", [_, "events", id]) => Json(event(id, "2026-03-01T09:00:00")).into_response(),
    ("POST", [_, "events"]) => {
      let mut made = body.clone();
      made["id"] = json!("made");
      (StatusCode::CREATED, Json(made)).into_response()
    }
    ("PATCH", [_, "events", id]) => {
      let mut changed = event(id, "2026-03-01T09:00:00");
      if let Some(title) = body["title"].as_str() {
        changed["title"] = json!(title);
      }
      Json(changed).into_response()
    }
    ("DELETE", [_, "events", _]) => StatusCode::NO_CONTENT.into_response(),
    ("GET", ["organization", "resources"]) | ("GET", ["organization", "resources", "free"]) => Json(json!({
      "items": [{ "id": "room-a", "name": "Room A", "kind": "meeting room", "place": "Floor 2", "seats": 8, "approval": true }],
      "more": false,
    }))
    .into_response(),
    ("POST", ["organization", "bookings"]) => {
      let mut made = body.clone();
      made["id"] = json!("booked");
      made["state"] = json!("requested");
      made["repeat"] = json!("none");
      (StatusCode::CREATED, Json(made)).into_response()
    }
    ("DELETE", ["organization", "bookings", _]) => StatusCode::NO_CONTENT.into_response(),
    _ => (StatusCode::NOT_FOUND, Json(json!({ "code": "no-route", "error": "No route." }))).into_response(),
  }
}

async fn stand() -> (Stand, Service) {
  let stand = Stand::default();
  let router = Router::new()
    .route("/{*rest}", get(answer_for).post(answer_for).patch(answer_for).delete(answer_for))
    .with_state(stand.clone());
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
  (
    stand,
    Service {
      url,
      token: "tool-token".into(),
    },
  )
}

fn handed(entry: &str, arguments: Value) -> Handed {
  serde_json::from_value(json!({
    "protocol": 1,
    "kind": "call",
    "entry": format!("bmo/{entry}"),
    "arguments": arguments,
    "session": { "holderName": "agent-a" },
  }))
  .unwrap()
}

fn query_of(text: &str) -> BTreeMap<String, String> {
  text
    .split('&')
    .filter_map(|pair| pair.split_once('='))
    .map(|(key, value)| (key.to_owned(), value.replace("%3A", ":")))
    .collect()
}

fn manifest() -> toml::Table {
  std::fs::read_to_string(Path::new(DIR).join("tool.toml")).unwrap().parse().unwrap()
}

#[test]
fn the_manifest_declares_every_entry_the_executable_answers() {
  let manifest = manifest();
  let entries = manifest["entry"].as_array().unwrap();
  let names: Vec<&str> = entries.iter().map(|entry| entry["name"].as_str().unwrap()).collect();
  assert_eq!(names, ENTRIES);
  for entry in entries {
    assert_eq!(entry["namespace"].as_str(), Some("bmo"));
    assert_eq!(entry["capabilities"].as_array().unwrap(), &vec![toml::Value::from("room.tools")]);
    let file = entry["input"].as_str().unwrap();
    let schema: Value =
      serde_json::from_str(&std::fs::read_to_string(Path::new(DIR).join(file)).unwrap()).unwrap();
    assert_eq!(schema["type"], "object", "{file}");
    let realmed = !entry["name"].as_str().unwrap().contains("resource")
      && !entry["name"].as_str().unwrap().contains("book");
    assert_eq!(
      schema["required"].as_array().unwrap().contains(&json!("realm")),
      realmed,
      "{file}: an event entry names a realm, a booking entry stands in the company"
    );
    assert!(Path::new(DIR).join(entry["manual"].as_str().unwrap()).is_file());
  }
}

#[tokio::test]
async fn the_calendars_are_listed_for_the_holder() {
  let (stand, service) = stand().await;
  let answer = answer(&handed("calendar_list", json!({ "realm": "mine" })), &service).await;
  assert!(!answer.is_error, "{}", answer.text());
  assert_eq!(answer.text(), "first · Own · primary\nsecond · Side · info");
  let asked = stand.asked.lock().unwrap().clone();
  assert_eq!(asked[0].1, "/mine/calendars");
  assert_eq!(query_of(&asked[0].2)["holderName"], "agent-a");
}

#[tokio::test]
async fn an_added_event_takes_the_first_calendar_and_an_hour() {
  let (stand, service) = stand().await;
  let arguments =
    json!({ "realm": "mine", "title": "Dentist", "start": "2026-03-04T15:00:00", "alarm": "60" });
  let answer = answer(&handed("calendar_add", arguments), &service).await;
  assert!(!answer.is_error, "{}", answer.text());
  assert!(
    answer
      .text()
      .starts_with("Added.\n2026-03-04T15:00:00 to 2026-03-04T16:00:00 · Dentist")
  );
  let asked = stand.asked.lock().unwrap().clone();
  assert_eq!((asked[0].0.as_str(), asked[0].1.as_str()), ("GET", "/mine/calendars"));
  assert_eq!((asked[1].0.as_str(), asked[1].1.as_str()), ("POST", "/mine/events"));
  assert_eq!(
    asked[1].3,
    json!({ "calendarId": "first", "title": "Dentist", "start": "2026-03-04T15:00:00", "end": "2026-03-04T16:00:00", "alarm": "60" })
  );
  assert_eq!(default_end("2026-03-04T15:00:00", true).as_deref(), Some("2026-03-05T00:00:00"));
}

#[tokio::test]
async fn the_next_events_are_read_from_now_and_cut_to_n() {
  let (stand, service) = stand().await;
  let answer =
    answer(&handed("calendar_events", json!({ "realm": "organization", "n": 2 })), &service).await;
  assert_eq!(answer.text().lines().count(), 2);
  assert_eq!(answer.structured.as_ref().unwrap()["more"], true);
  let asked = stand.asked.lock().unwrap().clone();
  let query = query_of(&asked[0].2);
  assert!(query.contains_key("start") && query.contains_key("end"));

  let answer = calendar_tool::answer(
    &handed("calendar_events", json!({ "realm": "mine", "start": "2026-03-01T00:00:00" })),
    &service,
  )
  .await;
  assert!(answer.is_error, "a span names both ends");
}

#[tokio::test]
async fn a_change_and_a_cancellation_name_the_occurrence() {
  let (stand, service) = stand().await;
  let change = json!({ "realm": "mine", "id": "a", "occurrence": "2026-03-01T09:00:00", "title": "Moved", "end": null });
  let changed = answer(&handed("calendar_change", change), &service).await;
  assert!(changed.text().starts_with("Changed.\n"), "{}", changed.text());
  let cancelled = answer(
    &handed(
      "calendar_cancel",
      json!({ "realm": "mine", "id": "a", "occurrence": "2026-03-01T09:00:00" }),
    ),
    &service,
  )
  .await;
  assert_eq!(cancelled.text(), "Cancelled.");

  let asked = stand.asked.lock().unwrap().clone();
  assert_eq!((asked[0].0.as_str(), asked[0].1.as_str()), ("PATCH", "/mine/events/a"));
  assert_eq!(asked[0].3, json!({ "title": "Moved", "occurrence": "2026-03-01T09:00:00" }));
  assert_eq!((asked[1].0.as_str(), asked[1].1.as_str()), ("DELETE", "/mine/events/a"));
  assert_eq!(query_of(&asked[1].2)["occurrence"], "2026-03-01T09:00:00");
}

#[tokio::test]
async fn a_refusal_comes_back_with_its_code() {
  let (_, service) = stand().await;
  let mut call = handed("calendar_show", json!({ "realm": "mine", "id": "a" }));
  call.session.holder_name = String::new();
  let refused: Answer = answer(&call, &service).await;
  assert!(refused.is_error);
  assert_eq!(refused.text(), "No account. (no-account)");

  let wrong = answer(&handed("calendar_show", json!({ "realm": "elsewhere", "id": "a" })), &service).await;
  assert!(wrong.is_error);
}

#[test]
fn the_service_is_found_in_the_data_roots_env_file() {
  let root = std::env::temp_dir().join(format!("calendar-tool-test-{}", std::process::id()));
  std::fs::create_dir_all(root.join("env")).unwrap();
  std::fs::write(
    root.join("env/calendar.env"),
    "CALENDAR_ADDRESS=http://[::1]:1/\nCALENDAR_TOKEN=tool-token\n",
  )
  .unwrap();
  let session = Session {
    holder_name: "agent-a".into(),
    variables: [("BMO_DATA_DIR".to_owned(), root.display().to_string())].into(),
  };
  assert_eq!(
    service(&session),
    Ok(Service {
      url: "http://[::1]:1".into(),
      token: "tool-token".into()
    })
  );
  std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn a_free_room_is_found_and_booked_for_the_holder() {
  let (stand, service) = stand().await;
  let free = answer(
    &handed("calendar_free_resources", json!({ "start": "2026-03-04T10:00:00", "seats": 8 })),
    &service,
  )
  .await;
  assert!(!free.is_error, "{}", free.text());
  assert_eq!(
    free.text(),
    "Room A · meeting room · 8 seats · id room-a · at Floor 2 · a booking waits for a keeper's approval"
  );
  let booked = answer(
    &handed(
      "calendar_book",
      json!({ "resourceId": "room-a", "title": "Planning", "start": "2026-03-04T10:00:00" }),
    ),
    &service,
  )
  .await;
  assert!(
    booked.text().starts_with("Requested: it waits for a keeper's approval"),
    "{}",
    booked.text()
  );
  let cancelled = answer(
    &handed(
      "calendar_cancel_booking",
      json!({ "bookingId": "booked", "occurrence": "2026-03-11T10:00:00" }),
    ),
    &service,
  )
  .await;
  assert_eq!(cancelled.text(), "Cancelled.");

  let asked = stand.asked.lock().unwrap().clone();
  assert_eq!((asked[0].0.as_str(), asked[0].1.as_str()), ("GET", "/organization/resources/free"));
  let query = query_of(&asked[0].2);
  assert_eq!(query["end"], "2026-03-04T11:00:00", "an hour where no end is given");
  assert_eq!((query["seats"].as_str(), query["holderName"].as_str()), ("8", "agent-a"));
  assert_eq!((asked[1].0.as_str(), asked[1].1.as_str()), ("POST", "/organization/bookings"));
  assert_eq!(
    asked[1].3,
    json!({ "resourceId": "room-a", "title": "Planning", "start": "2026-03-04T10:00:00", "end": "2026-03-04T11:00:00" })
  );
  assert_eq!(query_of(&asked[1].2)["holderName"], "agent-a");
  assert_eq!((asked[2].0.as_str(), asked[2].1.as_str()), ("DELETE", "/organization/bookings/booked"));
  assert_eq!(query_of(&asked[2].2)["occurrence"], "2026-03-11T10:00:00");
}
