//! The OpenAPI document: the committed one is what the code generates, it states exactly the routes and the
//! tool entries docs/api.md lists, and each entry's input is the schema the tool's folder places.

use std::collections::BTreeSet;

use bmo_calendar::openapi::{openapi, openapi_json};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The routes docs/api.md states, by method and path.
const ROUTES: [&str; 35] = [
  "get /health",
  "put /sources/{sourceId}",
  "put /sources/{sourceId}/schedules",
  "put /sources/{sourceId}/schedules/{itemId}",
  "delete /sources/{sourceId}/schedules/{itemId}",
  "get /{realm}/sources",
  "patch /{realm}/sources/{sourceId}",
  "get /{realm}/schedules",
  "get /{realm}/calendars",
  "post /{realm}/calendars",
  "patch /{realm}/calendars/{calendarId}",
  "delete /{realm}/calendars/{calendarId}",
  "get /{realm}/events",
  "post /{realm}/events",
  "get /{realm}/events/{eventId}",
  "patch /{realm}/events/{eventId}",
  "delete /{realm}/events/{eventId}",
  "get /{realm}/companies",
  "get /{realm}/resources",
  "post /{realm}/resources",
  "get /{realm}/resources/free",
  "patch /{realm}/resources/{resourceId}",
  "delete /{realm}/resources/{resourceId}",
  "get /{realm}/bookings",
  "post /{realm}/bookings",
  "get /{realm}/bookings/requests",
  "get /{realm}/bookings/{bookingId}",
  "patch /{realm}/bookings/{bookingId}",
  "delete /{realm}/bookings/{bookingId}",
  "post /{realm}/bookings/{bookingId}/decision",
  "get /{realm}/alarms",
  "post /beats",
  "get /events",
  "get /console-manifest",
  "get /console/{file}",
];

#[test]
fn the_committed_document_is_the_generated_one() {
  let committed = std::fs::read_to_string(format!("{ROOT}/docs/openapi.json")).unwrap_or_default();
  assert!(
    committed == openapi_json(),
    "docs/openapi.json differs from what the code generates; write it again with `bmo-calendar openapi`"
  );
}

#[test]
fn the_document_states_every_route_and_tool_and_nothing_else() {
  let document = openapi();
  let stated: BTreeSet<String> = document["paths"]
    .as_object()
    .unwrap()
    .iter()
    .flat_map(|(path, methods)| {
      methods.as_object().unwrap().keys().map(move |method| format!("{method} {path}"))
    })
    .collect();
  assert_eq!(stated, ROUTES.iter().map(|route| (*route).to_owned()).collect());
  assert_eq!(document["info"]["version"], env!("CARGO_PKG_VERSION"));
  assert_eq!(
    document["x-events"],
    serde_json::json!([
      "calendar.changed",
      "event.changed",
      "alarm.told",
      "source.changed",
      "schedule.changed",
      "resource.changed",
      "booking.changed",
      "booking.decided"
    ])
  );

  let manifest: toml::Table = std::fs::read_to_string(format!("{ROOT}/tools/calendar/tool.toml"))
    .unwrap()
    .parse()
    .unwrap();
  let entries = manifest["entry"].as_array().unwrap();
  let tools = document["x-tools"].as_array().unwrap();
  assert_eq!(tools.len(), entries.len());
  for (tool, entry) in tools.iter().zip(entries) {
    assert_eq!(tool["name"].as_str(), entry["name"].as_str());
    assert_eq!(tool["namespace"].as_str(), entry["namespace"].as_str());
    let (method, path) = tool["route"].as_str().unwrap().split_once(' ').unwrap();
    assert!(
      ROUTES.contains(&format!("{} {path}", method.to_lowercase()).as_str()),
      "{} calls no route",
      tool["name"]
    );
    let file = entry["input"].as_str().unwrap();
    let placed: serde_json::Value =
      serde_json::from_str(&std::fs::read_to_string(format!("{ROOT}/tools/calendar/{file}")).unwrap())
        .unwrap();
    assert_eq!(tool["input"], placed, "{file}");
  }
}
