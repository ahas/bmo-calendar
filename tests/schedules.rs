//! Sources and schedules other units push: one written, changed and removed by its source and item id, a
//! source's whole set reconciled, the reads of both realms, each account's layers, and the refusals.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{Service, labels};

fn schedule(title: &str, start: &str, end: &str, assignee: Option<&str>) -> Value {
  json!({ "title": title, "start": start, "end": end, "link": "/schedule/items/1", "assignee": assignee })
}

fn item(id: &str, title: &str, assignee: Option<&str>) -> Value {
  let mut item = schedule(title, "2026-11-20T09:00:00", "2026-11-20T10:00:00", assignee);
  item["itemId"] = json!(id);
  item
}

/// The `sourceId:itemId` of each schedule a page holds.
fn pairs(page: &Value) -> Vec<String> {
  page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| format!("{}:{}", one["sourceId"].as_str().unwrap(), one["itemId"].as_str().unwrap()))
    .collect()
}

const SPAN: &str = "start=2026-11-01T00:00:00&end=2026-12-01T00:00:00";

#[tokio::test]
async fn a_unit_writes_changes_and_removes_one_schedule_by_its_source_and_item() {
  let service = Service::fresh().await;
  let path = "/sources/plans/schedules/task-1";
  let first = schedule("Review", "2026-11-20T09:00:00", "2026-11-20T10:00:00", Some("2"));

  let (status, made) = service.call(Method::PUT, path, Some(first.clone())).await;
  assert_eq!(status, 201, "{made}");
  assert_eq!(made["sourceId"], "plans");
  assert_eq!(made["itemId"], "task-1");
  assert_eq!(made["assignee"], "2");
  assert_eq!(made["allDay"], false);

  let memory = &service.stand.live("schedule:plans:task-1")[0];
  assert_eq!(memory["type"], "schedule");
  assert_eq!(memory["text"], "Review");
  let kept = labels(memory);
  assert_eq!(kept["link"], "/schedule/items/1");
  assert_eq!(kept["assignee"], "2");
  assert_eq!(kept["realm"], "organization");
  let brains = service.stand.peers.lock().unwrap().brains.clone();
  let organization = brains.iter().find(|brain| brain["accountKey"] == "organization").unwrap();
  assert_eq!(memory["brainId"], organization["id"], "kept in the organization's brain alone");
  assert_eq!(brains.len(), 1);

  let source = &service.stand.live("source:plans")[0];
  assert_eq!(source["text"], "plans", "a source first written to is named by its id");
  assert_eq!(labels(source)["color"], "secondary");

  let (status, _) = service.call(Method::PUT, path, Some(first)).await;
  assert_eq!(status, 200);
  assert_eq!(service.stand.lineage("schedule:plans:task-1").len(), 1, "the same write writes nothing");

  let moved = schedule("Review", "2026-11-21T09:00:00", "2026-11-21T10:00:00", None);
  let (status, changed) = service.call(Method::PUT, path, Some(moved)).await;
  assert_eq!(status, 200);
  assert_eq!(
    (changed["start"].as_str(), &changed["assignee"]),
    (Some("2026-11-21T09:00:00"), &json!(null))
  );
  assert_eq!(service.stand.lineage("schedule:plans:task-1").len(), 2);

  assert_eq!(service.delete(path).await.0, 204);
  assert!(service.stand.live("schedule:plans:task-1").is_empty());
  assert_eq!(service.delete(path).await.0, 204, "a schedule that does not stand is no failure");

  let (_, page) = service
    .ask(
      Method::GET,
      "/events",
      Some(crate::support::TOKEN),
      &[("Accept", "application/json")],
      None,
    )
    .await;
  let kinds: Vec<&str> = page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| one["kind"].as_str().unwrap())
    .collect();
  assert!(kinds.contains(&"source.changed") && kinds.contains(&"schedule.changed"), "{kinds:?}");
}

#[tokio::test]
async fn a_whole_set_writes_what_differs_and_removes_what_it_does_not_name() {
  let service = Service::fresh().await;
  let set = json!({ "items": [item("a", "A", None), item("b", "B", None), item("c", "C", None)] });
  let (status, done) = service.call(Method::PUT, "/sources/plans/schedules", Some(set)).await;
  assert_eq!(status, 200, "{done}");
  assert_eq!(done, json!({ "written": 3, "unchanged": 0, "removed": 0 }));
  service.call(Method::PUT, "/sources/work/schedules/x", Some(item("x", "X", None))).await;

  let set = json!({ "items": [item("a", "A", None), item("b", "B moved", None), item("d", "D", None)] });
  let (_, done) = service.call(Method::PUT, "/sources/plans/schedules", Some(set)).await;
  assert_eq!(done, json!({ "written": 2, "unchanged": 1, "removed": 1 }));

  let (_, page) = service.get(&format!("/organization/schedules?{SPAN}")).await;
  let mut held = pairs(&page);
  held.sort();
  assert_eq!(held, ["plans:a", "plans:b", "plans:d", "work:x"], "another source's schedules stand");

  let twice = json!({ "items": [item("e", "E", None), item("e", "E", None)] });
  let (status, _) = service.call(Method::PUT, "/sources/plans/schedules", Some(twice)).await;
  assert_eq!(status, 400);
  assert!(service.stand.live("schedule:plans:e").is_empty(), "a refused set writes nothing");

  let (_, done) = service
    .call(Method::PUT, "/sources/plans/schedules", Some(json!({ "items": [] })))
    .await;
  assert_eq!(done["removed"], 3);
}

#[tokio::test]
async fn the_organization_reads_every_schedule_and_an_account_its_own() {
  let service = Service::fresh().await;
  let set = json!({ "items": [
    item("mine", "Mine", Some("2")),
    item("theirs", "Theirs", Some("3")),
    item("nobodys", "Nobody's", None),
  ] });
  service.call(Method::PUT, "/sources/plans/schedules", Some(set)).await;
  let late = schedule("Later", "2027-01-05T09:00:00", "2027-01-05T10:00:00", Some("2"));
  service.call(Method::PUT, "/sources/plans/schedules/later", Some(late)).await;

  let (status, organization) = service
    .console(Method::GET, &format!("/organization/schedules?{SPAN}"), "2", "member", None)
    .await;
  assert_eq!(status, 200, "{organization}");
  assert_eq!(organization["items"].as_array().unwrap().len(), 3, "the span holds three");
  assert!(
    organization["items"]
      .as_array()
      .unwrap()
      .iter()
      .all(|one| one["realm"] == "organization")
  );

  let (_, mine) = service
    .console(Method::GET, &format!("/mine/schedules?{SPAN}"), "2", "member", None)
    .await;
  assert_eq!(pairs(&mine), ["plans:mine"]);
  assert_eq!(mine["items"][0]["realm"], "mine");
  assert_eq!(
    service.stand.peers.lock().unwrap().brains.len(),
    1,
    "an account's brain is never written"
  );
}

#[tokio::test]
async fn a_source_is_hidden_for_one_account_in_one_realm() {
  let service = Service::fresh().await;
  service
    .call(Method::PUT, "/sources/plans/schedules/a", Some(item("a", "A", None)))
    .await;

  let (status, hidden) = service
    .console(Method::PATCH, "/mine/sources/plans", "2", "member", Some(json!({ "hidden": true })))
    .await;
  assert_eq!((status, &hidden["hidden"]), (200, &json!(true)), "{hidden}");
  let read = async |realm: &str, key: &str| {
    service.console(Method::GET, &format!("/{realm}/sources"), key, "member", None).await.1["items"][0]["hidden"].clone()
  };
  assert_eq!(read("mine", "2").await, true);
  assert_eq!(read("organization", "2").await, false);
  assert_eq!(read("mine", "3").await, false);

  let (status, named) = service
    .call(Method::PUT, "/sources/plans", Some(json!({ "name": "Plans", "color": "warning" })))
    .await;
  assert_eq!(status, 200, "{named}");
  assert_eq!((named["name"].as_str(), named["color"].as_str()), (Some("Plans"), Some("warning")));
  assert_eq!(read("mine", "2").await, true, "naming a source keeps every account's mark");

  let (status, _) = service
    .console(Method::PATCH, "/mine/sources/elsewhere", "2", "member", Some(json!({ "hidden": true })))
    .await;
  assert_eq!(status, 404);
}

#[tokio::test]
async fn the_writes_are_the_service_tokens_and_hold_their_shape() {
  let service = Service::fresh().await;
  let path = "/sources/plans/schedules/a";
  let good = schedule("A", "2026-11-20T09:00:00", "2026-11-20T10:00:00", None);

  let (status, refusal) =
    service.console(Method::PUT, path, "1", "administrator", Some(good.clone())).await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("forbidden")));

  let refused = |field: &str, value: Value| {
    let mut write = good.clone();
    write[field] = value;
    write
  };
  for (write, code) in [
    (refused("link", json!("javascript:alert(1)")), "bad-request"),
    (refused("link", json!("//elsewhere.example/x")), "bad-request"),
    (refused("link", json!("/\\elsewhere.example/x")), "bad-request"),
    (refused("title", json!("two\nlines")), "bad-request"),
    (refused("assignee", json!("k".repeat(201))), "bad-request"),
    (refused("end", json!("2026-11-20T08:00:00")), "end-before-start"),
  ] {
    let (_, refusal) = service.call(Method::PUT, path, Some(write.clone())).await;
    assert_eq!(refusal["code"].as_str(), Some(code), "{write}");
  }
  let (status, _) = service.call(Method::PUT, "/sources/no%20good/schedules/a", Some(good.clone())).await;
  assert_eq!(status, 400);
  let (status, _) = service
    .call(Method::PUT, path, Some(refused("link", json!("https://plans.example/a"))))
    .await;
  assert_eq!(status, 201, "an https address opens");

  let (_, refusal) = service
    .get("/organization/schedules?start=2026-01-01T00:00:00&end=2026-12-01T00:00:00")
    .await;
  assert_eq!(refusal["code"], "span-too-long");
}
