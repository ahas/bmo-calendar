//! Events: written, changed and cancelled through the brain's routes with the exact labels, idempotent by id,
//! refused as documented, kept apart by realm, and refreshed by the brain's stream.

use bmo_calendar::model::{epoch_of, parse_local};
use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{Service, ids, labels};

fn lunch() -> Value {
  json!({
    "calendarId": "mine",
    "title": "Lunch",
    "description": "At the corner.",
    "start": "2026-03-02T12:00:00",
    "end": "2026-03-02T13:00:00",
    "alarm": "15",
  })
}

async fn with_calendar(account: &str) -> Service {
  let service = Service::fresh().await;
  service.get(&format!("/mine/calendars?account={account}")).await;
  service
}

#[tokio::test]
async fn an_event_is_written_as_a_memory_with_its_labels() {
  let service = with_calendar("2").await;

  let (status, event) = service.post("/mine/events?account=2", lunch()).await;
  assert_eq!(status, 201, "{event}");
  assert_eq!(event["title"], "Lunch");
  assert_eq!(event["description"], "At the corner.");
  assert_eq!(event["alarm"], "15");
  assert_eq!(event["repeat"], "none");
  assert_eq!(event["allDay"], false);
  assert_eq!(event["creator"], "2");
  assert_eq!(event["told"], Value::Null);
  assert_eq!(event["occurrence"], Value::Null);

  let id = event["id"].as_str().unwrap();
  let memory = &service.stand.live(&format!("event:{id}"))[0];
  assert_eq!(memory["type"], "event");
  assert_eq!(memory["text"], "Lunch\n\nAt the corner.");
  assert_eq!(memory["validFrom"], epoch_of(parse_local("2026-03-02T12:00:00").unwrap()));
  let labels = labels(memory);
  let expected: Vec<(&str, &str)> = vec![
    ("allDay", "false"),
    ("alarm", "15"),
    ("calendar", "mine"),
    ("creator", "2"),
    ("end", "2026-03-02T13:00:00"),
    ("realm", "mine"),
    ("repeat", "none"),
    ("skip", ""),
    ("start", "2026-03-02T12:00:00"),
  ];
  for (name, value) in expected {
    assert_eq!(labels[name], value, "label {name}");
  }
  assert!(labels["createdAt"].parse::<i64>().unwrap() > 0);

  let (status, read) = service.get(&format!("/mine/events/{id}?account=2")).await;
  assert_eq!((status, &read), (200, &event));
}

#[tokio::test]
async fn a_change_is_a_new_version_and_a_cancellation_is_done() {
  let service = with_calendar("2").await;
  let (_, event) = service.post("/mine/events?account=2", lunch()).await;
  let id = event["id"].as_str().unwrap();
  let path = format!("/mine/events/{id}?account=2");

  let (status, changed) = service
    .patch(
      &path,
      json!({ "title": "Late lunch", "start": "2026-03-02T13:00:00", "end": "2026-03-02T14:00:00" }),
    )
    .await;
  assert_eq!(status, 200, "{changed}");
  assert_eq!(changed["title"], "Late lunch");
  assert_eq!(changed["description"], "At the corner.");
  assert_eq!(changed["createdAt"], event["createdAt"]);
  let lineage = service.stand.lineage(&format!("event:{id}"));
  assert_eq!(lineage.len(), 2);
  assert_eq!(lineage[0]["status"], "superseded");
  assert_eq!(lineage[1]["status"], "active");
  assert_eq!(lineage[1]["validFrom"], epoch_of(parse_local("2026-03-02T13:00:00").unwrap()));
  assert_eq!(labels(&lineage[1])["start"], "2026-03-02T13:00:00");

  let (status, refusal) = service.patch(&path, json!({ "end": "2026-03-02T12:00:00" })).await;
  assert_eq!((status, refusal["code"].as_str()), (422, Some("end-before-start")));

  let (status, _) = service.delete(&path).await;
  assert_eq!(status, 204);
  let lineage = service.stand.lineage(&format!("event:{id}"));
  assert_eq!(lineage[1]["status"], "done");
  assert_eq!(lineage[1]["reason"], "cancelled");
  let (status, refusal) = service.get(&path).await;
  assert_eq!((status, refusal["code"].as_str()), (404, Some("not-found")));
}

#[tokio::test]
async fn a_post_naming_an_id_it_holds_writes_over_it() {
  let service = with_calendar("2").await;
  let mut body = lunch();
  body["id"] = json!("offline-1");

  let (first, event) = service.post("/mine/events?account=2", body.clone()).await;
  let (second, again) = service.post("/mine/events?account=2", body.clone()).await;
  assert_eq!((first, second), (201, 200));
  assert_eq!(event, again);
  assert_eq!(service.stand.lineage("event:offline-1").len(), 1, "the same write twice writes once");

  body["title"] = json!("Lunch moved");
  let (status, moved) = service.post("/mine/events?account=2", body).await;
  assert_eq!((status, moved["title"].as_str()), (200, Some("Lunch moved")));
  assert_eq!(service.stand.live("event:offline-1").len(), 1);
  assert_eq!(service.stand.lineage("event:offline-1").len(), 2);

  let mut bad = lunch();
  bad["id"] = json!("not an id");
  let (status, _) = service.post("/mine/events?account=2", bad).await;
  assert_eq!(status, 400);
}

#[tokio::test]
async fn a_write_is_refused_as_documented() {
  let service = with_calendar("2").await;
  let refused = async |body: Value| {
    let (status, answer) = service.post("/mine/events?account=2", body).await;
    (status, answer["code"].as_str().unwrap_or("").to_owned())
  };

  let mut body = lunch();
  body["end"] = json!("2026-03-02T12:00:00");
  assert_eq!(refused(body).await, (422, "end-before-start".to_owned()));
  let mut body = lunch();
  body["calendarId"] = json!("nowhere");
  assert_eq!(refused(body).await, (422, "unknown-calendar".to_owned()));
  let mut body = lunch();
  body["start"] = json!("2026-03-02 12:00");
  assert_eq!(refused(body).await, (400, "bad-request".to_owned()));
  let mut body = lunch();
  body["title"] = json!("two\nlines");
  assert_eq!(refused(body).await, (400, "bad-request".to_owned()));
  let mut body = lunch();
  body["alarm"] = json!("7");
  assert_eq!(refused(body).await, (400, "bad-request".to_owned()));

  let (status, refusal) = service
    .get("/mine/events?account=2&start=2026-01-01T00:00:00&end=2026-04-04T00:00:00")
    .await;
  assert_eq!((status, refusal["code"].as_str()), (422, Some("span-too-long")));
  let (status, _) = service
    .get("/mine/events?account=2&start=2026-01-02T00:00:00&end=2026-01-01T00:00:00")
    .await;
  assert_eq!(status, 422);
}

#[tokio::test]
async fn the_organizations_writers_are_held_to_its_calendar() {
  let service = Service::fresh().await;
  service.get("/organization/calendars?account=1").await;
  let (_, closed) = service
    .console(
      Method::POST,
      "/organization/calendars",
      "1",
      "administrator",
      Some(json!({ "name": "Board", "color": "error", "writers": "administrator" })),
    )
    .await;
  let event = |calendar: &str| json!({ "calendarId": calendar, "title": "Review", "start": "2026-03-03T09:00:00", "end": "2026-03-03T10:00:00" });

  let (status, refusal) = service
    .console(
      Method::POST,
      "/organization/events",
      "2",
      "member",
      Some(event(closed["id"].as_str().unwrap())),
    )
    .await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("not-a-writer")));

  let (status, made) = service
    .console(Method::POST, "/organization/events", "2", "member", Some(event("organization")))
    .await;
  assert_eq!(status, 201, "{made}");
  assert_eq!(made["creator"], "2");
  let path = format!("/organization/events/{}", made["id"].as_str().unwrap());
  let change = json!({ "title": "Review again" });

  let (status, refusal) = service.console(Method::PATCH, &path, "3", "member", Some(change.clone())).await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("not-a-writer")));
  let (status, _) = service.console(Method::PATCH, &path, "2", "member", Some(change.clone())).await;
  assert_eq!(status, 200);
  let (status, _) = service.console(Method::PATCH, &path, "1", "administrator", Some(change)).await;
  assert_eq!(status, 200);
  let (status, _) = service.console(Method::DELETE, &path, "3", "member", None).await;
  assert_eq!(status, 403);
}

#[tokio::test]
async fn the_realms_are_kept_apart() {
  let service = Service::fresh().await;
  for account in ["2", "3"] {
    service.console(Method::GET, "/mine/calendars", account, "member", None).await;
  }
  let (_, own) = service.console(Method::POST, "/mine/events", "2", "member", Some(lunch())).await;
  let span = "/mine/events?start=2026-03-01T00:00:00&end=2026-03-03T00:00:00";

  let (_, seen) = service.console(Method::GET, span, "2", "member", None).await;
  assert_eq!(ids(&seen).into_iter().collect::<Vec<_>>(), [own["id"].as_str().unwrap().to_owned()]);
  let (_, other) = service.console(Method::GET, span, "3", "member", None).await;
  assert!(other["items"].as_array().unwrap().is_empty());
  let path = format!("/mine/events/{}", own["id"].as_str().unwrap());
  let (status, _) = service.console(Method::GET, &path, "3", "member", None).await;
  assert_eq!(status, 404);
  let (_, organization) = service
    .get("/organization/events?account=1&start=2026-03-01T00:00:00&end=2026-03-03T00:00:00")
    .await;
  assert!(organization["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_write_another_writer_makes_reaches_the_service_through_the_brains_stream() {
  let service = with_calendar("2").await;
  let brain = service.stand.brain("account:2");
  service.stand.write(
    brain,
    json!({
      "key": "event:elsewhere",
      "type": "event",
      "text": "Written elsewhere",
      "actorName": "tester",
      "labels": { "calendar": "mine", "start": "2026-03-02T08:00:00", "end": "2026-03-02T09:00:00",
        "allDay": "false", "alarm": "none", "repeat": "none", "creator": "2", "realm": "mine",
        "createdAt": "1", "skip": "" },
    }),
  );
  service
    .until(async |service| service.get("/mine/events/elsewhere?account=2").await.0 == 200)
    .await;
  let (_, event) = service.get("/mine/events/elsewhere?account=2").await;
  assert_eq!(event["title"], "Written elsewhere");

  let (_, page) = service
    .ask(
      Method::GET,
      "/events",
      Some(crate::support::TOKEN),
      &[("accept", "application/json")],
      None,
    )
    .await;
  assert!(
    page["items"]
      .as_array()
      .unwrap()
      .iter()
      .any(|one| one["kind"] == "event.changed" && one["data"]["eventId"] == "elsewhere"),
    "{page}"
  );
}
