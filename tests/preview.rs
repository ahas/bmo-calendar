//! The service over the stand-ins with a company's rooms and bookings in it, kept serving for a screen check:
//! `cargo test --test main preview -- --ignored --nocapture` prints the address and serves until it is
//! stopped. It is ignored in every ordinary run; README.md, 「Screen check」, says how the console bundle is
//! shown over it.

use chrono::{Datelike, Duration, Local, NaiveTime};
use serde_json::{Value, json};

use crate::support::Service;

/// A local datetime on a day this many days from today, at an hour and minute.
fn at(days: i64, hour: u32, minute: u32) -> String {
  let day = Local::now().date_naive() + Duration::days(days);
  day
    .and_time(NaiveTime::from_hms_opt(hour, minute, 0).unwrap())
    .format("%Y-%m-%dT%H:%M:%S")
    .to_string()
}

/// The days from today to this week's Monday and to its other days.
fn this_week(weekday: i64) -> i64 {
  weekday - i64::from(Local::now().date_naive().weekday().num_days_from_monday())
}

async fn seed(service: &Service, path: &str, body: Value) {
  let (status, answer) = service.post(path, body).await;
  assert!(status == 200 || status == 201, "{path}: {status} {answer}");
}

#[tokio::test]
#[ignore = "serves until it is stopped, for a screen check"]
async fn preview() {
  let service = Service::fresh().await;
  service.get("/organization/calendars?account=1").await;
  for account in ["1", "2", "3"] {
    service.get(&format!("/mine/calendars?account={account}")).await;
  }
  let rooms = [
    ("room-a", "Room A", "meeting room", "Floor 2", 4, false, vec![]),
    ("room-b", "Room B", "meeting room", "Floor 2", 8, false, vec![]),
    ("room-c", "Room C", "meeting room", "Floor 3", 12, true, vec!["3"]),
    ("room-d", "Room D", "meeting room", "Floor 5", 48, true, vec!["3"]),
    ("reception-a", "Reception A", "reception room", "Floor 1", 6, false, vec![]),
  ];
  for (id, name, kind, place, seats, approval, keepers) in rooms {
    let room = json!({ "id": id, "name": name, "kind": kind, "place": place, "seats": seats,
      "approval": approval, "keepers": keepers });
    seed(&service, "/organization/resources?account=1", room).await;
  }
  let booking = |id: &str, resource: &str, title: &str, start: String, end: String| json!({ "id": id, "resourceId": resource, "title": title, "start": start, "end": end });
  let monday = this_week(0);
  seed(
    &service,
    "/organization/bookings?account=2",
    booking("b-1", "room-a", "Weekly planning", at(monday, 9, 0), at(monday, 10, 0)),
  )
  .await;
  let mut weekly = booking("b-2", "room-b", "Design review", at(monday + 1, 14, 0), at(monday + 1, 15, 30));
  weekly["repeat"] = json!("weekly");
  seed(&service, "/organization/bookings?account=2", weekly).await;
  seed(
    &service,
    "/organization/bookings?account=3",
    booking("b-3", "room-a", "Interview", at(monday + 2, 11, 0), at(monday + 2, 12, 0)),
  )
  .await;
  seed(
    &service,
    "/organization/bookings?account=2",
    booking("b-4", "room-c", "Quarterly results", at(monday + 3, 10, 0), at(monday + 3, 12, 0)),
  )
  .await;
  seed(
    &service,
    "/organization/bookings?account=1",
    booking("b-5", "room-d", "All hands", at(monday + 4, 16, 0), at(monday + 4, 17, 0)),
  )
  .await;
  let mut all_day = booking("b-6", "reception-a", "Visitors", at(monday + 2, 0, 0), at(monday + 3, 0, 0));
  all_day["allDay"] = json!(true);
  seed(&service, "/organization/bookings?account=2", all_day).await;
  seed(
    &service,
    "/organization/events?account=1",
    json!({ "calendarId": "organization", "title": "Company dinner",
    "start": at(monday + 3, 18, 0), "end": at(monday + 3, 20, 0) }),
  )
  .await;

  println!(
    "preview: the service answers at {} (service token service-token, console token console-token)",
    service.address
  );
  std::future::pending::<()>().await;
}
