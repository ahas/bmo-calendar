//! Calendars: made on first read in each realm's own brain, written with their labels, hidden per account,
//! guarded in the organization's realm, and retired with their events.

use reqwest::Method;
use serde_json::json;

use crate::support::{Service, labels};

#[tokio::test]
async fn each_realm_is_given_its_calendar_on_first_read_in_a_brain_of_its_own() {
  let service = Service::fresh().await;

  let (status, mine) = service.get("/mine/calendars?account=2").await;
  assert_eq!(status, 200, "{mine}");
  assert_eq!(mine["items"].as_array().unwrap().len(), 1);
  assert_eq!(mine["items"][0]["id"], "mine");
  assert_eq!(mine["items"][0]["name"], "My calendar");
  assert_eq!(mine["items"][0]["color"], "primary");
  assert_eq!(mine["items"][0]["writers"], json!(null));

  let (status, organization) = service.get("/organization/calendars?account=1").await;
  assert_eq!(status, 200);
  assert_eq!(organization["items"][0]["id"], "organization");
  assert_eq!(organization["items"][0]["color"], "info");
  assert_eq!(organization["items"][0]["writers"], "creator");
  assert_eq!(organization["items"][0]["tell"], "creator");
  assert_eq!(organization["items"][0]["company"], "10");

  let brains = service.stand.peers.lock().unwrap().brains.clone();
  let keys: Vec<&str> = brains.iter().map(|brain| brain["accountKey"].as_str().unwrap()).collect();
  assert_eq!(
    keys,
    ["organization", "account:2", "company:10"],
    "the platform's own brain keeps the move's record; the organization realm is the company's"
  );
  assert!(brains.iter().all(|brain| brain["agentKey"] == "calendar"));

  let memory = &service.stand.live("calendar:mine")[0];
  assert_eq!(memory["type"], "calendar");
  assert_eq!(memory["text"], "My calendar");
  let labels = labels(memory);
  assert_eq!(labels["color"], "primary");
  assert_eq!(labels["realm"], "mine");
  assert_eq!(labels["hidden"], "");
  assert_eq!(labels["creator"], "2");

  let (_, again) = service.get("/mine/calendars?account=2").await;
  assert_eq!(again["items"].as_array().unwrap().len(), 1, "a second read makes nothing more");
}

#[tokio::test]
async fn a_mine_call_names_its_account() {
  let service = Service::fresh().await;

  let (status, refusal) = service.get("/mine/calendars").await;
  assert_eq!((status, refusal["code"].as_str()), (422, Some("no-account")));

  let (status, _) = service.get("/mine/calendars?holderName=agent-a").await;
  assert_eq!(status, 200, "a holder acts for the owner");
  assert!(
    service
      .stand
      .peers
      .lock()
      .unwrap()
      .brains
      .iter()
      .any(|brain| brain["accountKey"] == "account:1")
  );

  let (status, refusal) = service.get("/elsewhere/calendars").await;
  assert_eq!((status, refusal["code"].as_str()), (404, Some("not-found")));
}

#[tokio::test]
async fn hidden_is_each_accounts_own() {
  let service = Service::fresh().await;
  service.get("/organization/calendars?account=1").await;

  let (status, hidden) = service
    .console(
      Method::PATCH,
      "/organization/calendars/organization",
      "2",
      "member",
      Some(json!({ "hidden": true })),
    )
    .await;
  assert_eq!(status, 200, "{hidden}");
  assert_eq!(hidden["hidden"], true);
  assert_eq!(labels(&service.stand.live("calendar:organization")[0])["hidden"], "2");

  let (_, other) = service.console(Method::GET, "/organization/calendars", "3", "member", None).await;
  assert_eq!(other["items"][0]["hidden"], false);
  let (_, own) = service.console(Method::GET, "/organization/calendars", "2", "member", None).await;
  assert_eq!(own["items"][0]["hidden"], true);

  let (_, shown) = service
    .console(
      Method::PATCH,
      "/organization/calendars/organization",
      "2",
      "member",
      Some(json!({ "hidden": false })),
    )
    .await;
  assert_eq!(shown["hidden"], false);
  assert_eq!(labels(&service.stand.live("calendar:organization")[0])["hidden"], "");
}

#[tokio::test]
async fn the_organizations_calendars_are_set_by_the_administrator_and_their_creator() {
  let service = Service::fresh().await;
  let body = json!({ "name": "Team", "color": "success", "writers": "administrator", "tell": "everyone" });

  let (status, refusal) = service
    .console(Method::POST, "/organization/calendars", "2", "member", Some(body.clone()))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("not-a-writer")));

  let (status, made) = service
    .console(Method::POST, "/organization/calendars", "1", "administrator", Some(body))
    .await;
  assert_eq!(status, 201, "{made}");
  assert_eq!(made["writers"], "administrator");
  assert_eq!(made["tell"], "everyone");
  assert_eq!(made["creator"], "1");
  let path = format!("/organization/calendars/{}", made["id"].as_str().unwrap());

  let (status, refusal) = service
    .console(Method::PATCH, &path, "2", "member", Some(json!({ "name": "Ours" })))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("not-a-writer")));
  let (status, changed) = service
    .console(Method::PATCH, &path, "1", "administrator", Some(json!({ "name": "Ours" })))
    .await;
  assert_eq!((status, changed["name"].as_str()), (200, Some("Ours")));

  let (status, refusal) = service
    .console(
      Method::POST,
      "/mine/calendars",
      "2",
      "member",
      Some(json!({ "name": " ", "color": "info" })),
    )
    .await;
  assert_eq!((status, refusal["code"].as_str()), (400, Some("bad-request")));
  let (status, _) = service
    .console(
      Method::POST,
      "/mine/calendars",
      "2",
      "member",
      Some(json!({ "name": "x", "color": "pink" })),
    )
    .await;
  assert_eq!(status, 400);
}

#[tokio::test]
async fn a_retired_calendar_takes_its_events_with_it() {
  let service = Service::fresh().await;
  let (_, calendar) = service
    .post("/mine/calendars?account=2", json!({ "name": "Side", "color": "warning" }))
    .await;
  let calendar = calendar["id"].as_str().unwrap().to_owned();
  let (status, event) = service
    .post(
      "/mine/events?account=2",
      json!({ "calendarId": calendar, "title": "Gone", "start": "2026-03-02T10:00:00", "end": "2026-03-02T11:00:00" }),
    )
    .await;
  assert_eq!(status, 201, "{event}");

  let (status, _) = service.delete(&format!("/mine/calendars/{calendar}?account=2")).await;
  assert_eq!(status, 204);
  let lineage = service.stand.lineage(&format!("calendar:{calendar}"));
  assert_eq!(lineage[0]["status"], "done");
  assert_eq!(lineage[0]["reason"], "retired");
  let event_memory = &service.stand.lineage(&format!("event:{}", event["id"].as_str().unwrap()))[0];
  assert_eq!(event_memory["status"], "done");

  let (_, page) = service.get("/mine/calendars?account=2").await;
  assert!(page["items"].as_array().unwrap().iter().all(|one| one["id"] != calendar.as_str()));
}
