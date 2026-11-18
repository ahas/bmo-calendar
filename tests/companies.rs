//! Companies: each company's organization realm read by its own accounts alone and by the administrator, who
//! writes nothing in another company's, a caller in no company reading nothing, the companies a caller reads, the account a tool session's holder acts
//! for, and the one move of the organization's calendars and events into their companies.

use bmo_calendar::store::Store;
use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{MEMBER_SUPERVISOR, NO_ACCOUNT_SUPERVISOR, Service, labels, local_in, scratch, stand};

const SPAN: &str = "start=2026-03-01T00:00:00&end=2026-03-31T00:00:00";

fn event(title: &str) -> Value {
  json!({ "calendarId": "organization", "title": title, "start": "2026-03-03T09:00:00", "end": "2026-03-03T10:00:00" })
}

#[tokio::test]
async fn another_company_reads_no_organization_calendar_or_event_of_the_first() {
  let service = Service::fresh().await;
  service.console(Method::GET, "/organization/calendars", "2", "member", None).await;
  let (status, made) = service
    .console(Method::POST, "/organization/events", "2", "member", Some(event("Ours")))
    .await;
  assert_eq!(status, 201, "{made}");
  assert_eq!(made["company"], "10");
  let path = format!("/organization/events/{}", made["id"].as_str().unwrap());

  let (_, calendars) = service.console(Method::GET, "/organization/calendars", "4", "member", None).await;
  assert_eq!(calendars["items"].as_array().unwrap().len(), 1, "its own company's calendar alone");
  assert_eq!(calendars["items"][0]["company"], "20");
  let (_, events) = service
    .console(Method::GET, &format!("/organization/events?{SPAN}"), "4", "member", None)
    .await;
  assert!(events["items"].as_array().unwrap().is_empty(), "{events}");
  assert_eq!(service.console(Method::GET, &path, "4", "member", None).await.0, 404);
  let named = format!("{path}?company=10");
  assert_eq!(
    service.console(Method::GET, &named, "4", "member", None).await.0,
    404,
    "naming it reads nothing"
  );
  let (status, _) = service
    .console(Method::PATCH, &named, "4", "member", Some(json!({ "title": "Theirs" })))
    .await;
  assert_eq!(status, 404);

  let (_, same) = service
    .console(Method::GET, &format!("/organization/events?{SPAN}"), "3", "member", None)
    .await;
  assert_eq!(same["items"].as_array().unwrap().len(), 1, "a unit under the company reads it");
  let (status, read) = service.console(Method::GET, &named, "1", "administrator", None).await;
  assert_eq!(
    (status, read["title"].as_str()),
    (200, Some("Ours")),
    "the administrator reads every company"
  );

  let stream = |key: &'static str| {
    let service = &service;
    async move {
      let headers = [("Owner-Key", key), ("Owner-Role", "member"), ("accept", "application/json")];
      let (_, page) = service
        .ask(
          Method::GET,
          "/events?kinds=event.changed",
          Some(crate::support::CONSOLE),
          &headers,
          None,
        )
        .await;
      page["items"].as_array().unwrap().len()
    }
  };
  assert_eq!(stream("2").await, 1);
  assert_eq!(stream("4").await, 0, "another company is given none of its events");
}

#[tokio::test]
async fn the_administrator_reads_another_companys_realm_and_writes_nothing_in_it() {
  let service = Service::fresh().await;
  service.console(Method::GET, "/organization/calendars", "4", "member", None).await;
  let (_, theirs) = service
    .console(Method::POST, "/organization/events", "4", "member", Some(event("Theirs")))
    .await;
  let path = format!("/organization/events/{}?company=20", theirs["id"].as_str().unwrap());

  let (status, read) = service.console(Method::GET, &path, "1", "administrator", None).await;
  assert_eq!((status, read["title"].as_str()), (200, Some("Theirs")));
  let calendar = json!({ "name": "Board", "color": "error" });
  let room = json!({ "name": "Room Z", "kind": "meeting room", "seats": 4 });
  let booking = json!({ "resourceId": "room-z", "title": "Ours", "start": "2026-03-04T10:00:00",
    "end": "2026-03-04T11:00:00" });
  let writes = [
    (Method::PATCH, path.clone(), Some(json!({ "title": "Ours" }))),
    (Method::DELETE, path.clone(), None),
    (Method::POST, "/organization/events?company=20".to_owned(), Some(event("Ours"))),
    (Method::POST, "/organization/calendars?company=20".to_owned(), Some(calendar.clone())),
    (
      Method::PATCH,
      "/organization/calendars/organization?company=20".to_owned(),
      Some(json!({ "hidden": true })),
    ),
    (Method::POST, "/organization/resources?company=20".to_owned(), Some(room)),
    (Method::POST, "/organization/bookings?company=20".to_owned(), Some(booking)),
  ];
  for (method, path, body) in writes {
    let (status, refusal) = service.console(method.clone(), &path, "1", "administrator", body).await;
    assert_eq!((status, refusal["code"].as_str()), (403, Some("forbidden")), "{method} {path}");
  }
  let key = format!("event:{}", theirs["id"].as_str().unwrap());
  assert_eq!(service.stand.lineage(&key).len(), 1, "the event stands as it was written");

  let (status, _) = service
    .console(
      Method::POST,
      "/organization/calendars?company=platform",
      "1",
      "administrator",
      Some(calendar),
    )
    .await;
  assert_eq!(status, 201, "the platform's own realm is his to write");
}

#[tokio::test]
async fn a_caller_in_no_company_reads_nothing_and_writes_nothing() {
  let service = Service::fresh().await;
  let (status, calendars) =
    service.console(Method::GET, "/organization/calendars", "5", "member", None).await;
  assert_eq!((status, calendars["items"].as_array().unwrap().len()), (200, 0));
  let (status, refusal) = service
    .console(Method::POST, "/organization/events", "5", "member", Some(event("None")))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (422, Some("no-company")));
  let (status, _) = service.console(Method::GET, "/mine/calendars", "5", "member", None).await;
  assert_eq!(status, 200, "its own realm stands as it did");
}

#[tokio::test]
async fn the_companies_a_caller_reads() {
  let service = Service::fresh().await;
  let read = async |key: &str, role: &str| {
    let (status, page) = service.console(Method::GET, "/organization/companies", key, role, None).await;
    assert_eq!(status, 200, "{page}");
    page["items"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| {
        let id = one["id"].as_str().unwrap();
        format!("{id} own:{} writes:{} manages:{}", one["own"], one["writes"], one["manages"])
      })
      .collect::<Vec<_>>()
  };
  assert_eq!(
    read("2", "member").await,
    ["10 own:true writes:true manages:true"],
    "a head of the company manages it"
  );
  assert_eq!(read("3", "member").await, ["10 own:true writes:true manages:false"]);
  assert_eq!(read("5", "member").await, Vec::<String>::new());
  assert_eq!(
    read("1", "administrator").await,
    [
      "10 own:true writes:true manages:true",
      "20 own:false writes:false manages:false",
      "platform own:false writes:true manages:false"
    ]
  );
  let (status, _) = service.console(Method::GET, "/mine/companies", "2", "member", None).await;
  assert_eq!(status, 404, "the companies stand in the organization realm");
}

#[tokio::test]
async fn a_tool_sessions_holder_acts_for_the_account_agency_names() {
  let service = Service::fresh().await;
  let key_of = async |holder: &str| {
    let (status, page) = service.get(&format!("/mine/calendars?holderName={holder}")).await;
    assert_eq!(status, 200, "{page}");
    let brains = service.stand.peers.lock().unwrap().brains.clone();
    brains.last().unwrap()["accountKey"].as_str().unwrap().to_owned()
  };
  assert_eq!(key_of(MEMBER_SUPERVISOR).await, "account:2", "a supervisor's owner");
  assert_eq!(key_of("role-agent").await, "account:3", "a role agent's owner");
  assert_eq!(
    key_of(NO_ACCOUNT_SUPERVISOR).await,
    "account:1",
    "a supervisor of no account acts for the owner"
  );

  service.get(&format!("/organization/calendars?holderName={MEMBER_SUPERVISOR}")).await;
  let (status, made) = service
    .post(&format!("/organization/events?holderName={MEMBER_SUPERVISOR}"), event("Asked"))
    .await;
  assert_eq!(
    (status, made["creator"].as_str(), made["company"].as_str()),
    (201, Some("2"), Some("10"))
  );
}

/// A memory written straight into the platform's brain, as the organization's realm held it before companies.
fn seed(stand: &crate::support::Stand, brain: i64, kind: &str, id: &str, text: &str, labels: Value) {
  stand.write(
    brain,
    json!({ "key": format!("{kind}:{id}"), "type": kind, "text": text, "actorName": "calendar", "labels": labels }),
  );
}

fn calendar_labels(creator: &str) -> Value {
  json!({ "color": "info", "realm": "organization", "writers": "creator", "tell": "creator", "hidden": "",
    "creator": creator, "createdAt": "1" })
}

fn event_labels(calendar: &str, creator: &str, start: &str, end: &str) -> Value {
  json!({ "calendar": calendar, "start": start, "end": end, "allDay": "false", "alarm": "5", "repeat": "none",
    "creator": creator, "realm": "organization", "createdAt": "1", "skip": "" })
}

#[tokio::test]
async fn the_organizations_calendars_and_events_move_once_into_their_creators_companies() {
  let stand = stand().await;
  let folder = scratch();
  let platform = stand.brain("organization");
  seed(
    &stand,
    platform,
    "calendar",
    "organization",
    "Organization calendar",
    calendar_labels("1"),
  );
  seed(&stand, platform, "calendar", "shared", "Shared", calendar_labels("1"));
  // Each event's alarm came due two minutes ago: a moved event written after its moment is told unless it was.
  let (start, end) = (local_in(3), local_in(30));
  seed(
    &stand,
    platform,
    "event",
    "ours",
    "Ours",
    event_labels("organization", "2", &start, &end),
  );
  seed(
    &stand,
    platform,
    "event",
    "theirs",
    "Theirs",
    event_labels("organization", "4", &start, &end),
  );
  seed(&stand, platform, "event", "loose", "Loose", event_labels("shared", "5", &start, &end));
  let store_path = folder.join("data/calendar/calendar.db");
  Store::open(&store_path).unwrap().mark_told(platform, "ours", &start, 1, 1).unwrap();

  let service = Service::start(&stand, folder.clone()).await;
  let brain_of = |key: &str| {
    stand
      .peers
      .lock()
      .unwrap()
      .brains
      .iter()
      .find(|brain| brain["accountKey"] == key)
      .unwrap()["id"]
      .clone()
  };
  let (first, second) = (brain_of("company:10"), brain_of("company:20"));
  let live_in =
    |key: &str| stand.live(key).iter().map(|memory| memory["brainId"].clone()).collect::<Vec<_>>();
  assert_eq!(live_in("event:ours"), std::slice::from_ref(&first));
  assert_eq!(live_in("event:theirs"), std::slice::from_ref(&second));
  assert_eq!(
    live_in("event:loose"),
    [json!(platform)],
    "a creator in no company leaves it with the platform"
  );
  let mut calendars = live_in("calendar:organization");
  calendars.sort_by_key(Value::to_string);
  assert_eq!(
    calendars,
    [first.clone(), second.clone()],
    "the calendar stands under each of its events"
  );
  let mut shared = live_in("calendar:shared");
  shared.sort_by_key(Value::to_string);
  assert_eq!(
    shared,
    [json!(platform), first.clone()],
    "a calendar goes to its creator's company, and stays while one of its events stays"
  );
  assert_eq!(labels(&stand.live("event:ours")[0])["creator"], "2", "every label goes with it");

  let store = Store::open(&store_path).unwrap();
  assert!(
    store.is_told(first.as_i64().unwrap(), "ours", &start).unwrap(),
    "its told marks go with it"
  );

  let record = stand.live("migration:company-realms");
  assert_eq!(record.len(), 1);
  let kept = labels(&record[0]);
  assert_eq!((kept["movedEvents"].as_str(), kept["stayedEvents"].as_str()), ("2", "1"));
  assert_eq!((kept["movedCalendars"].as_str(), kept["stayedCalendars"].as_str()), ("1", "1"));

  service.post("/beats", json!({})).await;
  let kinds: Vec<String> = stand
    .instructions()
    .iter()
    .map(|one| one["kind"].as_str().unwrap_or("").to_owned())
    .collect();
  assert!(kinds.iter().any(|kind| kind.starts_with("calendar:theirs:")), "{kinds:?}");
  assert!(
    !kinds.iter().any(|kind| kind.starts_with("calendar:ours:")),
    "nothing is told twice: {kinds:?}"
  );

  let (stand, folder) = service.stop().await;
  let written = stand.peers.lock().unwrap().memories.len();
  let again = Service::start(&stand, folder).await;
  assert_eq!(stand.peers.lock().unwrap().memories.len(), written, "the move runs once");
  again.stop().await;
}
