//! The platform administrator's deletion of an account or a company for good (`docs/api.md`, 「Removing an account
//! or a company」): the count changes nothing and names what the removal then takes; the account's own realm goes
//! with its brain, another account's stays, a company's realm goes only for the company itself, a second removal
//! takes nothing, and a member's console call is refused.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{COMPANY, Service};

fn lunch() -> Value {
  json!({ "calendarId": "mine", "title": "Lunch", "start": "2026-03-02T12:00:00", "end": "2026-03-02T13:00:00" })
}

fn brain_keys(service: &Service) -> Vec<String> {
  let peers = service.stand.peers.lock().unwrap();
  peers
    .brains
    .iter()
    .filter_map(|brain| brain["accountKey"].as_str().map(str::to_owned))
    .collect()
}

#[tokio::test]
async fn an_accounts_own_realm_goes_whole_and_nothing_else_does() {
  let service = Service::fresh().await;
  for account in ["2", "4"] {
    service.console(Method::GET, "/mine/calendars", account, "member", None).await;
    let (status, _) = service.console(Method::POST, "/mine/events", account, "member", Some(lunch())).await;
    assert_eq!(status, 201);
  }

  let (status, counted) =
    service.console(Method::GET, "/accounts/2/removal", "1", "administrator", None).await;
  assert_eq!((status, &counted), (200, &json!({ "counts": { "calendars": 1, "events": 1 } })));
  assert!(brain_keys(&service).contains(&"account:2".to_owned()), "a count changes nothing");

  let (status, removed) = service.console(Method::DELETE, "/accounts/2", "1", "administrator", None).await;
  assert_eq!((status, &removed["removed"]), (200, &counted["counts"]));
  let keys = brain_keys(&service);
  assert!(!keys.contains(&"account:2".to_owned()), "the account's realm is gone: {keys:?}");
  assert!(keys.contains(&"account:4".to_owned()), "another account's realm stays");

  let (status, again) = service.delete("/accounts/2").await;
  assert_eq!((status, again), (200, json!({ "removed": { "calendars": 0, "events": 0 } })));
}

#[tokio::test]
async fn a_companys_realm_goes_only_for_the_company_itself() {
  let service = Service::fresh().await;
  let (status, _) = service.console(Method::GET, "/organization/calendars", "2", "member", None).await;
  assert_eq!(status, 200);
  let zeros = json!({ "calendars": 0, "events": 0, "schedules": 0, "resources": 0, "bookings": 0 });

  let (status, below) = service.get(&format!("/org-units/99/removal?company={COMPANY}")).await;
  assert_eq!((status, &below["counts"]), (200, &zeros), "a unit below a company keeps nothing here");

  let path = format!("/org-units/{COMPANY}?company={COMPANY}");
  let (status, removed) = service.delete(&path).await;
  assert_eq!(status, 200, "{removed}");
  assert_eq!(removed["removed"]["calendars"], 1);
  assert!(!brain_keys(&service).contains(&format!("company:{COMPANY}")));
  let (status, again) = service.delete(&path).await;
  assert_eq!((status, &again["removed"]), (200, &zeros));
}

#[tokio::test]
async fn a_removal_is_the_administrators_alone_and_names_an_id() {
  let service = Service::fresh().await;
  let (status, _) = service.console(Method::DELETE, "/accounts/2", "2", "member", None).await;
  assert_eq!(status, 403);
  let (status, _) = service.delete("/accounts/two").await;
  assert_eq!(status, 400);
}
