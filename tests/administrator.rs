//! The platform administrator in another person's own realm: he reads it by naming the account, his writes
//! there are refused, and his read leaves nothing in it; a member's named account is not read.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{Service, ids};

const SPAN: &str = "start=2026-03-01T00:00:00&end=2026-03-31T00:00:00";

fn lunch() -> Value {
  json!({ "calendarId": "mine", "title": "Lunch", "start": "2026-03-02T12:00:00", "end": "2026-03-02T13:00:00" })
}

/// Every brain and every memory the stand-in holds, as they stand now.
fn standing(service: &Service) -> (Vec<Value>, Vec<Value>) {
  let peers = service.stand.peers.lock().unwrap();
  (peers.brains.clone(), peers.memories.clone())
}

#[tokio::test]
async fn the_administrator_reads_a_persons_own_realm_and_writes_nothing_there() {
  let service = Service::fresh().await;
  service.console(Method::GET, "/mine/calendars", "2", "member", None).await;
  let (_, own) = service.console(Method::POST, "/mine/events", "2", "member", Some(lunch())).await;
  let id = own["id"].as_str().unwrap().to_owned();
  let one = format!("/mine/events/{id}?account=2");
  let before = standing(&service);

  let read = async |path: &str| service.console(Method::GET, path, "1", "administrator", None).await;
  let (status, calendars) = read("/mine/calendars?account=2").await;
  assert_eq!((status, ids(&calendars)), (200, ["mine".to_owned()].into()));
  let (status, events) = read(&format!("/mine/events?account=2&{SPAN}")).await;
  assert_eq!((status, ids(&events)), (200, [id.clone()].into()));
  let (status, event) = read(&one).await;
  assert_eq!((status, event["title"].as_str()), (200, Some("Lunch")));

  let writes = [
    (Method::PATCH, one.clone(), Some(json!({ "title": "Dinner" }))),
    (Method::DELETE, one.clone(), None),
    (Method::POST, "/mine/events?account=2".to_owned(), Some(lunch())),
    (
      Method::PATCH,
      "/mine/calendars/mine?account=2".to_owned(),
      Some(json!({ "hidden": true })),
    ),
  ];
  for (method, path, body) in writes {
    let (status, refusal) = service.console(method.clone(), &path, "1", "administrator", body).await;
    assert_eq!((status, refusal["code"].as_str()), (403, Some("forbidden")), "{method} {path}");
  }
  assert_eq!(
    standing(&service),
    before,
    "the read and the refused writes leave the realm as it stood"
  );

  let (status, _) = service.console(Method::GET, &one, "3", "member", None).await;
  assert_eq!(status, 404, "a member's named account is not read: it reads its own realm");
}

#[tokio::test]
async fn the_administrators_read_of_a_realm_with_nothing_in_it_makes_nothing() {
  let service = Service::fresh().await;
  let before = standing(&service);
  for path in ["/mine/calendars?account=3", "/organization/calendars?company=20"] {
    let (status, page) = service.console(Method::GET, path, "1", "administrator", None).await;
    assert_eq!((status, page["items"].as_array().map(Vec::len)), (200, Some(0)), "{path}");
  }
  assert_eq!(standing(&service), before, "no brain, no first calendar");
}
