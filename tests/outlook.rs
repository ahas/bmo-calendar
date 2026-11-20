//! A person's Outlook day (`docs/api.md`, 「A person's Outlook day」): read on the holder's own node, from the
//! running Outlook or else from Graph with the Teams page token; refused for a node not the account's own; and a
//! title read as the work-code form.

use bmo_calendar::title::parts;
use bmo_calendar::wire::TitleParts;
use serde_json::{Value, json};

use crate::support::{CALLER_TOKEN, CONSOLE, OWNER_NODE, OWNER_NODE_ACCOUNT, PAGE_TOKEN, Service, stand};

const DAY: &str = "/outlook/day?holderName=supervisor-b&date=2026-03-04";

/// The day as the command on the node answers it, Outlook running with the events given or not running.
fn node_day(events: Option<Value>) -> Value {
  json!({
    "date": "2026-03-04", "zone": "Korea Standard Time",
    "startUtc": "2026-03-03T15:00:00Z", "endUtc": "2026-03-04T15:00:00Z",
    "outlook": if events.is_some() { "running" } else { "not-running" },
    "why": "", "events": events.unwrap_or_else(|| json!([])),
  })
}

/// A service whose stand-in agency names member 2's own node, `member-pc`, the command answering `day`.
async fn service_for_member(day: Value) -> Service {
  let stand = stand().await;
  {
    let mut peers = stand.peers.lock().unwrap();
    peers.person_nodes =
      vec![json!({ "ownerAccount": "2", "node": "member-pc", "shared": false, "setAt": 1 })];
    peers.node_day = Some(day);
  }
  Service::start(&stand, crate::support::scratch()).await
}

#[tokio::test]
async fn the_running_outlook_is_read_on_the_holders_own_node() {
  let events = json!([{ "title": "1010211I [Platform] Weekly review", "start": "2026-03-04T09:00:00",
    "end": "2026-03-04T10:00:00", "allDay": false, "organizer": "Kim", "attendees": 3, "room": "Room 7" }]);
  let service = service_for_member(node_day(Some(events))).await;
  let (status, day) = service.get(DAY).await;
  assert_eq!(status, 200, "{day}");
  assert_eq!((day["road"].as_str(), day["node"].as_str()), (Some("outlook"), Some("member-pc")));
  assert_eq!(day["zone"], "Korea Standard Time");
  assert_eq!(
    day["items"],
    json!([{ "title": "1010211I [Platform] Weekly review", "start": "2026-03-04T09:00:00",
      "end": "2026-03-04T10:00:00", "allDay": false, "organizer": "Kim", "attendees": 3, "room": "Room 7",
      "titleParts": { "levels": ["1", "01", "02", "11", "I"], "group": "Platform", "task": "Weekly review" } }])
  );

  let peers = service.stand.peers.lock().unwrap();
  let (node, body, bearer) = &peers.runs[0];
  assert_eq!((node.as_str(), bearer.as_str()), ("member-pc", CALLER_TOKEN));
  assert_eq!((body["account"].as_str(), body["shell"].as_str()), (Some("2"), Some("powershell")));
  assert_eq!(body["env"], json!({ "BMO_CALENDAR_DAY": "2026-03-04" }));
  assert!(peers.page_token_asks.is_empty(), "no token is asked where Outlook was read");
}

#[tokio::test]
async fn graph_reads_the_day_where_outlook_is_not_running() {
  let service = service_for_member(node_day(None)).await;
  {
    let mut peers = service.stand.peers.lock().unwrap();
    peers.page_token = true;
    let organizer = json!({ "emailAddress": { "name": "Kim", "address": "kim-address" } });
    peers.graph_events = vec![
      json!({ "subject": "Plan", "isAllDay": false, "isCancelled": false, "organizer": organizer,
        "start": { "dateTime": "2026-03-04T09:00:00.0000000" }, "end": { "dateTime": "2026-03-04T09:30:00.0000000" },
        "location": { "displayName": "Room 7" },
        "attendees": [
          { "type": "required", "emailAddress": { "address": "KIM-ADDRESS" } },
          { "type": "required", "emailAddress": { "address": "lee-address" } },
          { "type": "optional", "emailAddress": { "address": "park-address" } },
          { "type": "resource", "emailAddress": { "address": "room7-address" } },
        ] }),
      json!({ "subject": "Dropped", "isCancelled": true, "start": { "dateTime": "2026-03-04T11:00:00" },
        "end": { "dateTime": "2026-03-04T12:00:00" } }),
      json!({ "subject": "Focus", "isAllDay": false, "isCancelled": false,
        "start": { "dateTime": "2026-03-04T13:00:00.0000000" }, "end": { "dateTime": "2026-03-04T14:00:00.0000000" } }),
    ];
  }
  let (status, day) = service.get(DAY).await;
  assert_eq!(status, 200, "{day}");
  assert_eq!(day["road"], "graph");
  let items = day["items"].as_array().unwrap();
  let seen: Vec<(&str, &str, u64, &str, &str)> = items
    .iter()
    .map(|item| {
      (
        item["title"].as_str().unwrap(),
        item["start"].as_str().unwrap(),
        item["attendees"].as_u64().unwrap(),
        item["organizer"].as_str().unwrap(),
        item["room"].as_str().unwrap(),
      )
    })
    .collect();
  assert_eq!(
    seen,
    vec![
      ("Plan", "2026-03-04T09:00:00", 2, "Kim", "Room 7"),
      ("Focus", "2026-03-04T13:00:00", 0, "", "")
    ],
    "the cancelled meeting dropped, the organizer and the room not counted, both pages read"
  );

  let peers = service.stand.peers.lock().unwrap();
  let (asked, bearer) = &peers.page_token_asks[0];
  assert_eq!((asked["account"].as_u64(), bearer.as_str()), (Some(2), CALLER_TOKEN));
  assert!(asked["scopes"].as_array().unwrap().contains(&json!("Calendars.Read")));
  let (query, prefer, bearer) = &peers.graph_asks[0];
  assert_eq!(
    (query["startDateTime"].as_str(), query["endDateTime"].as_str()),
    ("2026-03-03T15:00:00Z", "2026-03-04T15:00:00Z")
  );
  assert!(!query["$select"].contains("body"), "no body is read");
  assert_eq!(
    (prefer.as_str(), bearer.as_str()),
    ("outlook.timezone=\"Korea Standard Time\"", PAGE_TOKEN)
  );
  assert_eq!(peers.graph_asks.len(), 2);
}

#[tokio::test]
async fn a_day_with_no_road_or_no_node_of_its_own_is_refused() {
  let service = service_for_member(node_day(None)).await;
  let (status, refused) = service.get(DAY).await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-road")), "{refused}");
  assert!(refused["error"].as_str().unwrap().contains("Outlook is not running"), "{refused}");

  service.stand.peers.lock().unwrap().person_nodes[0]["shared"] = json!(true);
  let (status, refused) = service.get(DAY).await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-node")), "{refused}");
  let (status, refused) = service.get("/outlook/day?holderName=role-agent").await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-node")), "{refused}");
  assert_eq!(service.stand.peers.lock().unwrap().runs.len(), 1, "no other node is tried");

  let (status, refused) = service.get("/outlook/day").await;
  assert_eq!((status, refused["code"].as_str()), (422, Some("no-account")));
  let (status, _) = service.get("/outlook/day?holderName=supervisor-b&date=2026-3-4").await;
  assert_eq!(status, 400);
  let (status, _) = service
    .ask(
      reqwest::Method::GET,
      DAY,
      Some(CONSOLE),
      &[("Owner-Key", "2"), ("Owner-Role", "member")],
      None,
    )
    .await;
  assert_eq!(status, 403, "the console reads no one's Outlook day");
}

#[tokio::test]
async fn the_owners_day_is_read_on_his_node_under_the_account_it_knows_him_by() {
  let service = service_for_member(node_day(None)).await;
  {
    let mut peers = service.stand.peers.lock().unwrap();
    peers.person_nodes = vec![
      json!({ "ownerAccount": OWNER_NODE_ACCOUNT, "node": OWNER_NODE, "shared": true, "setAt": 1 }),
      json!({ "ownerAccount": "2", "node": OWNER_NODE, "shared": true, "setAt": 1 }),
    ];
    peers.page_token = true;
  }
  let (status, day) = service.get("/outlook/day?holderName=someone-of-the-owner").await;
  assert_eq!(status, 200, "{day}");
  assert_eq!((day["node"].as_str(), day["road"].as_str()), (Some(OWNER_NODE), Some("graph")));
  {
    let peers = service.stand.peers.lock().unwrap();
    assert_eq!(peers.runs[0].1["account"], OWNER_NODE_ACCOUNT, "the account his node knows him by");
    assert_eq!(peers.page_token_asks[0].0["account"].to_string(), OWNER_NODE_ACCOUNT, "a number");
    assert!(peers.runs[0].1.get("env").is_none(), "no day named is the PC's today");
  }

  let (status, refused) = service.get(DAY).await;
  assert_eq!(
    (status, refused["code"].as_str()),
    (422, Some("no-node")),
    "another account sharing his node"
  );
  assert_eq!(service.stand.peers.lock().unwrap().runs.len(), 1);
}

#[test]
fn a_title_is_read_as_the_work_code_form() {
  let read = |levels: &[&str], group: Option<&str>, task: &str| TitleParts {
    levels: levels.iter().map(|level| (*level).to_owned()).collect(),
    group: group.map(str::to_owned),
    task: task.to_owned(),
  };
  assert_eq!(parts("6010000N Leave"), read(&["6", "01", "00", "00", "N"], None, "Leave"));
  assert_eq!(
    parts("1010211I_A_B [Robots] Field test"),
    read(&["1", "01", "02", "11", "I", "A", "B"], Some("Robots"), "Field test")
  );
  assert_eq!(parts("101021I Short code"), read(&[], None, "101021I Short code"));
  assert_eq!(parts("Lunch"), read(&[], None, "Lunch"));
}
