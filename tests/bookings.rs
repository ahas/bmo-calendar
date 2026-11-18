//! Resources and bookings: resources kept by the administrator and the company's heads, two bookings of one
//! resource never meeting, a booking that waits for a keeper's approval and the tellings around it, a repeating
//! booking with one occurrence cancelled, and another company reading none of it.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{MEMBER_SUPERVISOR, Service};

/// Room A, eight seats, booked at once; Room B, twenty seats, kept by member 3 and booked on approval.
async fn with_rooms() -> Service {
  let service = Service::fresh().await;
  let rooms = [
    json!({ "id": "room-a", "name": "Room A", "kind": "meeting room", "place": "Floor 2", "seats": 8 }),
    json!({ "id": "room-b", "name": "Room B", "kind": "meeting room", "place": "Floor 3", "seats": 20,
      "approval": true, "keepers": ["3"] }),
  ];
  for room in rooms {
    let (status, made) = service
      .console(Method::POST, "/organization/resources", "1", "administrator", Some(room))
      .await;
    assert_eq!(status, 201, "{made}");
  }
  service
}

fn booking(resource: &str, start: &str, end: &str) -> Value {
  json!({ "resourceId": resource, "title": "Planning", "start": start, "end": end })
}

async fn book(service: &Service, key: &str, body: Value) -> (u16, Value) {
  service.console(Method::POST, "/organization/bookings", key, "member", Some(body)).await
}

/// The kinds of the instructions the receiver was handed.
fn told(service: &Service) -> Vec<String> {
  service
    .stand
    .instructions()
    .iter()
    .map(|one| one["kind"].as_str().unwrap_or("").to_owned())
    .collect()
}

#[tokio::test]
async fn resources_are_kept_by_the_administrator_and_the_companys_heads() {
  let service = with_rooms().await;
  let room = json!({ "name": "Room C", "kind": "reception room", "seats": 6 });
  let (status, refusal) = service
    .console(Method::POST, "/organization/resources", "3", "member", Some(room.clone()))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("not-a-writer")));
  let (status, made) = service
    .console(Method::POST, "/organization/resources", "2", "member", Some(room))
    .await;
  assert_eq!((status, made["company"].as_str()), (201, Some("10")), "{made}");

  let outsider =
    json!({ "name": "Room D", "kind": "meeting room", "seats": 4, "approval": true, "keepers": ["4"] });
  let (status, refusal) = service
    .console(Method::POST, "/organization/resources", "1", "administrator", Some(outsider))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (422, Some("not-in-company")));
  let keeperless = json!({ "name": "Room D", "kind": "meeting room", "seats": 4, "approval": true });
  let (status, _) = service
    .console(Method::POST, "/organization/resources", "1", "administrator", Some(keeperless))
    .await;
  assert_eq!(status, 400, "a resource that waits for approval has a keeper");

  let (_, page) = service.console(Method::GET, "/organization/resources", "3", "member", None).await;
  let names: Vec<&str> = page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| one["name"].as_str().unwrap())
    .collect();
  assert_eq!(names, ["Room A", "Room B", "Room C"], "by kind, place and name");

  let (status, _) = service
    .console(
      Method::PATCH,
      "/organization/resources/room-a",
      "3",
      "member",
      Some(json!({ "seats": 10 })),
    )
    .await;
  assert_eq!(status, 403);
  let (status, changed) = service
    .console(
      Method::PATCH,
      "/organization/resources/room-a",
      "2",
      "member",
      Some(json!({ "seats": 10 })),
    )
    .await;
  assert_eq!((status, changed["seats"].as_u64()), (200, Some(10)));
}

#[tokio::test]
async fn two_bookings_of_one_resource_never_meet() {
  let service = with_rooms().await;
  let (status, first) =
    book(&service, "2", booking("room-a", "2026-03-02T10:00:00", "2026-03-02T11:00:00")).await;
  assert_eq!(
    (status, first["state"].as_str(), first["booker"].as_str()),
    (201, Some("booked"), Some("2"))
  );

  let (status, refusal) =
    book(&service, "3", booking("room-a", "2026-03-02T10:30:00", "2026-03-02T11:30:00")).await;
  assert_eq!((status, refusal["code"].as_str()), (409, Some("overlap")), "{refusal}");
  let said = refusal["error"].as_str().unwrap();
  assert!(
    said.contains(first["id"].as_str().unwrap()) && said.contains("\"Planning\""),
    "it names the one it meets: {said}"
  );
  let (status, _) =
    book(&service, "3", booking("room-a", "2026-03-02T11:00:00", "2026-03-02T12:00:00")).await;
  assert_eq!(status, 201, "one may start where another ends");
  let (status, _) =
    book(&service, "3", booking("room-b", "2026-03-02T10:30:00", "2026-03-02T11:30:00")).await;
  assert_eq!(status, 201, "another resource is another span");

  let path = format!("/organization/bookings/{}", first["id"].as_str().unwrap());
  let (status, refusal) = service
    .console(Method::PATCH, &path, "2", "member", Some(json!({ "end": "2026-03-02T11:30:00" })))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (409, Some("overlap")), "a change is held to it too");
  let (status, refusal) = service
    .console(Method::PATCH, &path, "4", "member", Some(json!({ "title": "Theirs" })))
    .await;
  assert_eq!(status, 404, "{refusal}");
  let (status, refusal) = service.console(Method::DELETE, &path, "3", "member", None).await;
  assert_eq!(
    (status, refusal["code"].as_str()),
    (403, Some("not-a-writer")),
    "neither its booker nor a keeper"
  );

  let free = "/organization/resources/free?start=2026-03-02T10:00:00&end=2026-03-02T11:00:00&seats=8";
  let (_, page) = service.console(Method::GET, free, "3", "member", None).await;
  let ids: Vec<&str> = page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| one["id"].as_str().unwrap())
    .collect();
  assert!(ids.is_empty(), "room A is booked and room B requested then: {ids:?}");
  let mut all_day = booking("room-a", "2026-03-07T09:00:00", "2026-03-07T10:00:00");
  all_day["allDay"] = json!(true);
  let (status, whole) = book(&service, "2", all_day).await;
  assert_eq!(
    (status, whole["start"].as_str(), whole["end"].as_str()),
    (201, Some("2026-03-07T00:00:00"), Some("2026-03-08T00:00:00")),
    "an all-day booking holds its days whole"
  );
  let (status, _) =
    book(&service, "3", booking("room-a", "2026-03-07T18:00:00", "2026-03-07T19:00:00")).await;
  assert_eq!(status, 409);
  let free = "/organization/resources/free?start=2026-03-03T10:00:00&end=2026-03-03T11:00:00&seats=10";
  let (_, page) = service.console(Method::GET, free, "3", "member", None).await;
  let ids: Vec<&str> = page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| one["id"].as_str().unwrap())
    .collect();
  assert_eq!(ids, ["room-b"], "room A has too few seats");
}

#[tokio::test]
async fn a_booking_waits_for_a_keepers_approval_and_its_booker_is_told() {
  let service = with_rooms().await;
  let (status, asked) =
    book(&service, "2", booking("room-b", "2026-03-04T10:00:00", "2026-03-04T11:00:00")).await;
  assert_eq!((status, asked["state"].as_str()), (201, Some("requested")), "{asked}");
  let id = asked["id"].as_str().unwrap().to_owned();
  service
    .until(async |service| {
      told(service).iter().any(|kind| kind.starts_with(&format!("booking:{id}:requested:")))
    })
    .await;
  let request = service
    .stand
    .instructions()
    .into_iter()
    .find(|one| one["kind"].as_str().unwrap().contains(":requested:"))
    .unwrap();
  let order = request["said"]["order"].as_str().unwrap();
  assert!(order.starts_with("CALENDAR BOOKING REQUEST."), "{order}");
  assert!(order.contains("member-b (account 3)"), "the keeper is told: {order}");

  let (_, waiting) = service
    .console(Method::GET, "/organization/bookings/requests", "3", "member", None)
    .await;
  assert_eq!(waiting["items"][0]["id"], id.as_str());
  let (_, none) = service
    .console(Method::GET, "/organization/bookings/requests", "2", "member", None)
    .await;
  assert!(none["items"].as_array().unwrap().is_empty(), "the booker keeps no such resource");

  let decision = format!("/organization/bookings/{id}/decision");
  let (status, refusal) = service
    .console(Method::POST, &decision, "2", "member", Some(json!({ "approve": true })))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("not-a-keeper")));
  let (status, approved) = service
    .console(Method::POST, &decision, "3", "member", Some(json!({ "approve": true })))
    .await;
  assert_eq!(
    (status, approved["state"].as_str(), approved["decidedBy"].as_str()),
    (200, Some("booked"), Some("3"))
  );
  let (status, refusal) = service
    .console(Method::POST, &decision, "3", "member", Some(json!({ "approve": true })))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (409, Some("not-requested")));
  let decisions = async |key: &str| {
    let headers = [("Owner-Key", key), ("Owner-Role", "member"), ("accept", "application/json")];
    let (_, page) = service
      .ask(
        Method::GET,
        "/events?kinds=booking.decided",
        Some(crate::support::CONSOLE),
        &headers,
        None,
      )
      .await;
    page["items"].as_array().unwrap().len()
  };
  assert_eq!(decisions("2").await, 1, "the booker is given the decision");
  assert_eq!(decisions("3").await, 0, "no one else is");
  service
    .until(async |service| {
      told(service).iter().any(|kind| kind.starts_with(&format!("booking:{id}:decided:")))
    })
    .await;
  let decided = service
    .stand
    .instructions()
    .into_iter()
    .find(|one| one["kind"].as_str().unwrap().contains(":decided:"))
    .unwrap();
  assert_eq!(decided["room"], crate::support::MEMBER_ROOM, "the booker is told in their own room");
  assert!(decided["said"]["order"].as_str().unwrap().contains("was approved"));

  let (status, again) =
    book(&service, "2", booking("room-b", "2026-03-05T10:00:00", "2026-03-05T11:00:00")).await;
  assert_eq!(status, 201);
  let again_id = again["id"].as_str().unwrap().to_owned();
  let (_, refused) = service
    .console(
      Method::POST,
      &format!("/organization/bookings/{again_id}/decision"),
      "3",
      "member",
      Some(json!({ "approve": false, "reason": "Closed that day" })),
    )
    .await;
  assert_eq!(
    (refused["state"].as_str(), refused["reason"].as_str()),
    (Some("refused"), Some("Closed that day"))
  );
  service
    .until(async |service| service.stand.live(&format!("booking:{again_id}")).is_empty())
    .await;
  let refusal = service
    .stand
    .instructions()
    .into_iter()
    .find(|one| one["kind"].as_str().unwrap().starts_with(&format!("booking:{again_id}:decided:")))
    .unwrap();
  assert!(refusal["said"]["order"].as_str().unwrap().contains("was refused: \"Closed that day\""));
  let (status, _) =
    book(&service, "3", booking("room-b", "2026-03-05T10:00:00", "2026-03-05T11:00:00")).await;
  assert_eq!(status, 201, "a refused booking holds no span");

  let tellings = told(&service).len();
  service.post("/beats", json!({})).await;
  assert_eq!(told(&service).len(), tellings, "each is told once");
}

#[tokio::test]
async fn a_repeating_booking_with_one_occurrence_cancelled() {
  let service = with_rooms().await;
  let mut weekly = booking("room-a", "2026-03-02T09:00:00", "2026-03-02T10:00:00");
  weekly["repeat"] = json!("weekly");
  let (status, made) = book(&service, "2", weekly).await;
  assert_eq!(status, 201, "{made}");
  let path = format!("/organization/bookings/{}", made["id"].as_str().unwrap());

  let (status, _) = service
    .console(Method::DELETE, &format!("{path}?occurrence=2026-03-09T09:00:00"), "2", "member", None)
    .await;
  assert_eq!(status, 204);
  let (status, refusal) = service
    .console(Method::DELETE, &format!("{path}?occurrence=2026-03-10T09:00:00"), "2", "member", None)
    .await;
  assert_eq!((status, refusal["code"].as_str()), (422, Some("not-an-occurrence")));

  let span = "/mine/bookings?start=2026-03-01T00:00:00&end=2026-04-01T00:00:00";
  let (_, mine) = service.console(Method::GET, span, "2", "member", None).await;
  let starts: Vec<&str> = mine["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| one["start"].as_str().unwrap())
    .collect();
  assert_eq!(
    starts,
    [
      "2026-03-02T09:00:00",
      "2026-03-16T09:00:00",
      "2026-03-23T09:00:00",
      "2026-03-30T09:00:00"
    ]
  );
  assert!(mine["items"].as_array().unwrap().iter().all(|one| one["realm"] == "mine"));

  let (status, _) =
    book(&service, "3", booking("room-a", "2026-03-09T09:00:00", "2026-03-09T10:00:00")).await;
  assert_eq!(status, 201, "the cancelled occurrence is free");
  let (status, refusal) =
    book(&service, "3", booking("room-a", "2027-06-14T09:30:00", "2027-06-14T10:30:00")).await;
  assert_eq!(
    (status, refusal["code"].as_str()),
    (409, Some("overlap")),
    "the series holds its later weeks"
  );

  let (status, moved) = service
    .console(Method::PATCH, &path, "2", "member", Some(json!({ "occurrence": "2026-03-16T09:00:00", "start": "2026-03-16T13:00:00", "end": "2026-03-16T14:00:00" })))
    .await;
  assert_eq!((status, moved["repeat"].as_str()), (200, Some("none")), "{moved}");
  let (_, mine) = service.console(Method::GET, span, "2", "member", None).await;
  let starts: Vec<&str> = mine["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| one["start"].as_str().unwrap())
    .collect();
  assert_eq!(
    starts,
    [
      "2026-03-02T09:00:00",
      "2026-03-16T13:00:00",
      "2026-03-23T09:00:00",
      "2026-03-30T09:00:00"
    ]
  );
}

#[tokio::test]
async fn another_company_reads_no_resource_or_booking_of_the_first() {
  let service = with_rooms().await;
  let (_, made) =
    book(&service, "2", booking("room-a", "2026-03-02T10:00:00", "2026-03-02T11:00:00")).await;
  let path = format!("/organization/bookings/{}", made["id"].as_str().unwrap());

  let (_, resources) = service.console(Method::GET, "/organization/resources", "4", "member", None).await;
  assert!(resources["items"].as_array().unwrap().is_empty());
  let span = "/organization/bookings?start=2026-03-01T00:00:00&end=2026-03-31T00:00:00";
  let (_, bookings) = service.console(Method::GET, span, "4", "member", None).await;
  assert!(bookings["items"].as_array().unwrap().is_empty());
  assert_eq!(service.console(Method::GET, &path, "4", "member", None).await.0, 404);
  assert_eq!(
    service.console(Method::GET, &format!("{path}?company=10"), "4", "member", None).await.0,
    404
  );
  let (status, refusal) =
    book(&service, "4", booking("room-a", "2026-03-03T10:00:00", "2026-03-03T11:00:00")).await;
  assert_eq!(
    (status, refusal["code"].as_str()),
    (422, Some("unknown-resource")),
    "its own company holds no such room"
  );
  let (status, read) = service
    .console(Method::GET, &format!("{path}?company=10"), "1", "administrator", None)
    .await;
  assert_eq!((status, read["booker"].as_str()), (200, Some("2")));
  let elsewhere = booking("room-a", "2026-03-04T10:00:00", "2026-03-04T11:00:00");
  let (status, refusal) = service.post("/organization/bookings?account=2&company=20", elsewhere).await;
  assert_eq!(
    (status, refusal["code"].as_str()),
    (403, Some("not-a-writer")),
    "a booking is made in its booker's own company"
  );
}

#[tokio::test]
async fn a_supervisor_books_as_the_person_it_serves() {
  let service = with_rooms().await;
  let (status, made) = service
    .post(
      &format!("/organization/bookings?holderName={MEMBER_SUPERVISOR}"),
      booking("room-a", "2026-03-06T10:00:00", "2026-03-06T11:00:00"),
    )
    .await;
  assert_eq!(
    (status, made["booker"].as_str(), made["company"].as_str()),
    (201, Some("2"), Some("10")),
    "{made}"
  );
  let (status, _) = service
    .delete(&format!(
      "/organization/bookings/{}?holderName={MEMBER_SUPERVISOR}",
      made["id"].as_str().unwrap()
    ))
    .await;
  assert_eq!(status, 204);
}
