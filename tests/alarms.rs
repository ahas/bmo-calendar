//! Alarms: a due one told once through the receiver, none twice across a restart, a late one with its
//! lateness, a cancelled one never, and each told in the room its account reaches.

use bmo_calendar::model::{epoch_of, now_ms, parse_local};
use serde_json::{Value, json};

use crate::support::{MEMBER_ROOM, OWNER_ROOM, Service, local_in, scratch, stand};

async fn due_event(service: &Service, account: &str, title: &str) -> Value {
  service.get(&format!("/mine/calendars?account={account}")).await;
  let body = json!({
    "calendarId": "mine",
    "title": title,
    "start": local_in(2),
    "end": local_in(32),
    "alarm": "5",
  });
  let (status, event) = service.post(&format!("/mine/events?account={account}"), body).await;
  assert_eq!(status, 201, "{event}");
  event
}

#[tokio::test]
async fn a_due_alarm_is_told_once_and_none_twice_across_a_restart() {
  let service = Service::fresh().await;
  let event = due_event(&service, "1", "Dentist").await;
  let id = event["id"].as_str().unwrap();

  let (status, beat) = service.post("/beats", json!({})).await;
  assert_eq!(status, 200, "{beat}");
  let told = beat["told"].as_array().unwrap();
  assert_eq!(told.len(), 1, "{beat}");
  assert_eq!(told[0]["eventId"], id);
  assert_eq!(told[0]["occurrence"], event["start"]);
  assert_eq!(told[0]["lateMs"], 0);
  assert_eq!(told[0]["rooms"], json!([OWNER_ROOM]));

  let instructions = service.stand.instructions();
  assert_eq!(instructions.len(), 1);
  let instruction = &instructions[0];
  assert_eq!(instruction["room"], OWNER_ROOM);
  assert_eq!(instruction["sort"], "calendar");
  assert_eq!(instruction["cause"], "reply");
  assert_eq!(instruction["summary"], "Dentist");
  assert_eq!(instruction["kind"], format!("calendar:{id}:{}", event["start"].as_str().unwrap()));
  let order = instruction["said"]["order"].as_str().unwrap();
  assert!(order.starts_with("CALENDAR ALARM."), "the turn opens on its marker: {order}");
  assert!(order.contains("\"Dentist\""), "the event's title: {order}");
  assert!(
    order.contains(&event["start"].as_str().unwrap().replace('T', " ")),
    "the event's start: {order}"
  );
  assert!(order.contains("a personal calendar"), "its realm: {order}");
  assert!(order.contains("It was asked to be reminded of."), "{order}");
  assert!(order.contains("It is for the owner."), "whom it is for: {order}");
  assert!(!order.contains("late"), "{order}");
  for sentence in ["Remind", "in your own words", "tell the owner", "so it reaches them"] {
    assert!(!order.contains(sentence), "the turn holds no instruction ({sentence}): {order}");
  }

  let (_, again) = service.post("/beats", json!({})).await;
  assert_eq!(again["told"], json!([]));
  let (_, read) = service.get(&format!("/mine/events/{id}?account=1")).await;
  assert_eq!(read["told"], event["start"]);

  let (stand, folder) = service.stop().await;
  let service = Service::start(&stand, folder).await;
  let (_, after) = service.post("/beats", json!({})).await;
  assert_eq!(after["told"], json!([]));
  assert_eq!(service.stand.instructions().len(), 1, "a restart tells nothing twice");
}

#[tokio::test]
async fn an_alarm_that_came_due_while_the_service_was_down_is_told_late() {
  let stand = stand().await;
  let folder = scratch();
  let now = now_ms();
  let start = local_in(-20);
  let brain = stand.brain("account:1");
  stand.peers.lock().unwrap().written_at = Some(now - 3 * 3_600_000);
  stand.write(
    brain,
    json!({
      "key": "event:missed",
      "type": "event",
      "text": "Standup",
      "validFrom": epoch_of(parse_local(&start).unwrap()),
      "actorName": "tester",
      "labels": { "calendar": "mine", "start": start, "end": local_in(10), "allDay": "false", "alarm": "0",
        "repeat": "none", "creator": "1", "realm": "mine", "createdAt": "1", "skip": "" },
    }),
  );
  let settings = crate::support::settings(&stand, &folder);
  bmo_calendar::store::Store::open(&settings.store.path)
    .unwrap()
    .set_last_beat(now - 2 * 3_600_000)
    .unwrap();

  let service = Service::start(&stand, folder).await;
  service.until(async |service| !service.stand.instructions().is_empty()).await;
  let order = service.stand.instructions()[0]["said"]["order"].as_str().unwrap().to_owned();
  assert!(order.starts_with("CALENDAR ALARM."), "the turn opens on its marker: {order}");
  assert!(order.contains("Standup"), "the event's title: {order}");
  assert!(order.contains("minutes late"), "how late it is: {order}");
  assert!(!order.contains("Remind"), "the turn holds no instruction: {order}");

  let from = now - 3_600_000;
  let (_, alarms) =
    service.get(&format!("/mine/alarms?account=1&from={from}&to={}", now + 3_600_000)).await;
  assert_eq!(alarms["items"][0]["eventId"], "missed");
  assert_eq!(alarms["items"][0]["told"], true);
  let (_, beat) = service.post("/beats", json!({})).await;
  assert_eq!(beat["told"], json!([]));
}

#[tokio::test]
async fn a_cancelled_event_is_told_nothing() {
  let service = Service::fresh().await;
  let event = due_event(&service, "1", "Called off").await;
  service
    .delete(&format!("/mine/events/{}?account=1", event["id"].as_str().unwrap()))
    .await;

  let (_, beat) = service.post("/beats", json!({})).await;
  assert_eq!(beat["told"], json!([]));
  assert!(service.stand.instructions().is_empty());
}

#[tokio::test]
async fn each_account_is_told_in_the_room_it_reaches() {
  let service = Service::fresh().await;
  due_event(&service, "2", "Member with a room").await;
  due_event(&service, "3", "Member without one").await;

  let (_, beat) = service.post("/beats", json!({})).await;
  assert_eq!(beat["told"].as_array().unwrap().len(), 2, "{beat}");
  let instructions = service.stand.instructions();
  let in_room = |room: i64| instructions.iter().find(|one| one["room"] == room).cloned().unwrap();

  let member = in_room(MEMBER_ROOM);
  assert_eq!(member["summary"], "Member with a room");
  let order = member["said"]["order"].as_str().unwrap();
  assert!(order.starts_with("CALENDAR ALARM."), "the turn opens on its marker: {order}");
  assert!(order.contains("Member with a room"), "the event's title: {order}");
  assert!(order.contains("It is for member-a (account 2)."), "whom it is for: {order}");
  assert!(!order.contains("Remind"), "the turn holds no instruction: {order}");
  let relayed = in_room(OWNER_ROOM);
  assert_eq!(relayed["summary"], "Member without one");
  let order = relayed["said"]["order"].as_str().unwrap();
  assert!(order.starts_with("CALENDAR ALARM."), "the turn opens on its marker: {order}");
  assert!(order.contains("Member without one"), "the event's title: {order}");
  assert!(
    order.contains("It is for member-b (account 3), who has no room of their own here."),
    "whom it is for, as data: {order}"
  );
  for sentence in ["Remind", "in your own words", "tell the owner", "so it reaches them"] {
    assert!(!order.contains(sentence), "the turn holds no instruction ({sentence}): {order}");
  }
}

#[tokio::test]
async fn a_member_with_no_registered_room_is_told_in_their_own_bmo_room() {
  let service = Service::fresh().await;
  {
    let mut peers = service.stand.peers.lock().unwrap();
    // Member 2's registered sender room comes before the BMO room kept for them.
    peers.account_rooms.insert("2".to_owned(), vec![210]);
    peers.private_rooms.insert(210, 2);
    // Member 3's rooms: another account's private room, a shared room and two of their own BMO's.
    peers.account_rooms.insert("3".to_owned(), vec![730, 720, 710, 705]);
    peers.private_rooms.extend([(720, 3), (710, 3), (705, 2)]);
  }
  due_event(&service, "2", "Member with a room").await;
  due_event(&service, "3", "Member with a BMO").await;

  let (_, beat) = service.post("/beats", json!({})).await;
  assert_eq!(beat["told"].as_array().unwrap().len(), 2, "{beat}");
  let rooms: Vec<(i64, String)> = service
    .stand
    .instructions()
    .iter()
    .map(|one| (one["room"].as_i64().unwrap(), one["summary"].as_str().unwrap().to_owned()))
    .collect();
  assert!(rooms.contains(&(MEMBER_ROOM, "Member with a room".to_owned())), "{rooms:?}");
  assert!(rooms.contains(&(710, "Member with a BMO".to_owned())), "{rooms:?}");
  let own = service.stand.instructions().into_iter().find(|one| one["room"] == 710).unwrap();
  let order = own["said"]["order"].as_str().unwrap();
  assert!(
    order.contains("It is for member-b (account 3)."),
    "told in a room of their own: {order}"
  );
}

#[tokio::test]
async fn the_organizations_tell_reaches_everyone_it_names() {
  let service = Service::fresh().await;
  for account in ["2", "3"] {
    service.get(&format!("/mine/calendars?account={account}")).await;
  }
  service.get("/organization/calendars?account=1").await;
  service
    .patch("/organization/calendars/organization?account=1", json!({ "tell": "everyone" }))
    .await;
  let body = json!({ "calendarId": "organization", "title": "All hands", "start": local_in(2), "end": local_in(30), "alarm": "5" });
  let (status, _) = service.post("/organization/events?account=1", body).await;
  assert_eq!(status, 201);

  let (_, beat) = service.post("/beats", json!({})).await;
  let rooms: Vec<i64> =
    beat["told"][0]["rooms"].as_array().unwrap().iter().filter_map(Value::as_i64).collect();
  assert_eq!(rooms, [MEMBER_ROOM, OWNER_ROOM]);
  let instructions = service.stand.instructions();
  assert_eq!(instructions.len(), 2, "one room is told once");
}

#[tokio::test]
async fn an_alarm_is_told_in_the_voice_of_the_supervisor_that_set_it() {
  let service = Service::fresh().await;
  service.get("/mine/calendars?account=1").await;
  let body = json!({
    "calendarId": "mine",
    "title": "Call the bank",
    "start": local_in(2),
    "end": local_in(32),
    "alarm": "5",
  });
  let (status, set) = service.post("/mine/events?account=1&holderName=kakia-chat", body).await;
  assert_eq!(status, 201, "{set}");
  assert_eq!(set["setBy"], "kakia-chat");
  due_event(&service, "1", "Dentist").await;

  service.post("/beats", json!({})).await;

  let instructions = service.stand.instructions();
  let voice = |title: &str| {
    instructions
      .iter()
      .find(|one| one["summary"] == title)
      .map(|one| one["agentName"].clone())
      .unwrap()
  };
  assert_eq!(voice("Call the bank"), json!("kakia-chat"));
  assert_eq!(voice("Dentist"), Value::Null, "one set from a screen is the room's default's to tell");
}
