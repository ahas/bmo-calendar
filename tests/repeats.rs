//! Repeating events: one row per occurrence in a span, a skipped month's day left out, and one occurrence
//! changed or cancelled on its own.

use serde_json::{Value, json};

use crate::support::{Service, labels};

async fn series(service: &Service, start: &str, end: &str, repeat: &str) -> String {
  let body =
    json!({ "calendarId": "mine", "title": "Series", "start": start, "end": end, "repeat": repeat });
  let (status, event) = service.post("/mine/events?account=2", body).await;
  assert_eq!(status, 201, "{event}");
  event["id"].as_str().unwrap().to_owned()
}

fn starts(page: &Value) -> Vec<String> {
  page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|row| row["start"].as_str().unwrap().to_owned())
    .collect()
}

#[tokio::test]
async fn a_repeating_event_answers_each_occurrence_in_the_span() {
  let service = Service::fresh().await;
  service.get("/mine/calendars?account=2").await;
  let weekly = series(&service, "2026-01-05T09:00:00", "2026-01-05T10:00:00", "weekly").await;

  let (_, page) = service
    .get("/mine/events?account=2&start=2026-01-10T00:00:00&end=2026-02-01T00:00:00")
    .await;
  assert_eq!(starts(&page), ["2026-01-12T09:00:00", "2026-01-19T09:00:00", "2026-01-26T09:00:00"]);
  let row = &page["items"][0];
  assert_eq!(row["id"], weekly.as_str());
  assert_eq!(row["occurrence"], "2026-01-12T09:00:00");
  assert_eq!(row["end"], "2026-01-12T10:00:00");

  let (_, page) = service
    .get("/mine/events?account=2&start=2026-01-12T09:30:00&end=2026-01-12T09:45:00")
    .await;
  assert_eq!(starts(&page), ["2026-01-12T09:00:00"], "an occurrence under way overlaps the span");
}

#[tokio::test]
async fn a_monthly_event_skips_a_month_without_its_day() {
  let service = Service::fresh().await;
  service.get("/mine/calendars?account=2").await;
  series(&service, "2026-01-31T18:00:00", "2026-01-31T19:00:00", "monthly").await;

  let (_, page) = service
    .get("/mine/events?account=2&start=2026-01-01T00:00:00&end=2026-04-01T00:00:00")
    .await;
  assert_eq!(starts(&page), ["2026-01-31T18:00:00", "2026-03-31T18:00:00"]);
}

#[tokio::test]
async fn one_occurrence_is_cancelled_or_changed_on_its_own() {
  let service = Service::fresh().await;
  service.get("/mine/calendars?account=2").await;
  let daily = series(&service, "2026-02-01T07:00:00", "2026-02-01T07:30:00", "daily").await;
  let span = "/mine/events?account=2&start=2026-02-01T00:00:00&end=2026-02-05T00:00:00";

  let (status, _) = service
    .delete(&format!("/mine/events/{daily}?account=2&occurrence=2026-02-02T07:00:00"))
    .await;
  assert_eq!(status, 204);
  assert_eq!(labels(&service.stand.live(&format!("event:{daily}"))[0])["skip"], "2026-02-02T07:00:00");

  let (status, single) = service
    .patch(
      &format!("/mine/events/{daily}?account=2"),
      json!({ "occurrence": "2026-02-03T07:00:00", "title": "Later", "start": "2026-02-03T08:00:00", "end": "2026-02-03T08:30:00" }),
    )
    .await;
  assert_eq!(status, 200, "{single}");
  assert_ne!(single["id"], daily.as_str());
  assert_eq!(single["repeat"], "none");
  assert_eq!(single["title"], "Later");

  let (_, page) = service.get(span).await;
  assert_eq!(starts(&page), ["2026-02-01T07:00:00", "2026-02-03T08:00:00", "2026-02-04T07:00:00"]);
  assert_eq!(
    labels(&service.stand.live(&format!("event:{daily}"))[0])["skip"],
    "2026-02-02T07:00:00,2026-02-03T07:00:00"
  );

  let (status, refusal) = service
    .delete(&format!("/mine/events/{daily}?account=2&occurrence=2026-02-02T07:00:00"))
    .await;
  assert_eq!((status, refusal["code"].as_str()), (422, Some("not-an-occurrence")));
  let (status, _) = service
    .delete(&format!("/mine/events/{daily}?account=2&occurrence=2026-02-04T07:05:00"))
    .await;
  assert_eq!(status, 422);
}
