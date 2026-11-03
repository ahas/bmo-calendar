//! The console's reach: the guard of every route (401, 403), the bundle's manifest, module, stylesheet and
//! words, a 404 where no bundle is built, and the events a reader is given.

use reqwest::Method;
use serde_json::json;

use crate::support::{CONSOLE, Service, TOKEN, scratch, stand};

#[tokio::test]
async fn every_route_is_guarded() {
  let service = Service::fresh().await;
  let get = async |bearer: Option<&str>, headers: &[(&str, &str)]| {
    let (status, answer) = service.ask(Method::GET, "/mine/calendars", bearer, headers, None).await;
    (status, answer["code"].as_str().unwrap_or("").to_owned())
  };

  assert_eq!(get(None, &[]).await, (401, "unauthorized".to_owned()));
  assert_eq!(get(Some("wrong"), &[]).await, (401, "unauthorized".to_owned()));
  assert_eq!(get(Some(CONSOLE), &[("Owner-Role", "member")]).await, (403, "forbidden".to_owned()));
  assert_eq!(get(Some(CONSOLE), &[("Owner-Key", "2")]).await, (403, "forbidden".to_owned()));
  assert_eq!(
    get(Some(CONSOLE), &[("Owner-Key", "2"), ("Owner-Role", "guest")]).await,
    (403, "forbidden".to_owned())
  );
  let long = "k".repeat(201);
  assert_eq!(
    get(Some(CONSOLE), &[("Owner-Key", &long), ("Owner-Role", "member")]).await,
    (403, "forbidden".to_owned())
  );
  assert_eq!(get(Some(CONSOLE), &[("Owner-Key", "2"), ("Owner-Role", "member")]).await.0, 200);

  let (status, refusal) = service
    .ask(
      Method::POST,
      "/beats",
      Some(CONSOLE),
      &[("Owner-Key", "1"), ("Owner-Role", "administrator")],
      None,
    )
    .await;
  assert_eq!((status, refusal["code"].as_str()), (403, Some("forbidden")));
  let (status, refusal) = service.get("/nowhere/at/all").await;
  assert_eq!((status, refusal["code"].as_str()), (404, Some("no-route")));
  let (status, health) = service.ask(Method::GET, "/health", None, &[], None).await;
  assert_eq!(status, 401, "{health}");
}

#[tokio::test]
async fn no_bundle_is_answered_where_none_is_built() {
  let service = Service::fresh().await;
  for path in ["/console-manifest", "/console/words", "/console/bundle-0000000000000000.js"] {
    let (status, refusal) = service.ask(Method::GET, path, Some(CONSOLE), &[], None).await;
    assert_eq!((status, refusal["code"].as_str()), (404, Some("not-found")), "{path}");
  }
}

#[tokio::test]
async fn a_built_bundle_is_answered_by_its_manifest() {
  let folder = scratch();
  let bundle = folder.join("bundle");
  std::fs::create_dir_all(&bundle).unwrap();
  std::fs::write(bundle.join("bundle.js"), "export default {};\n").unwrap();
  std::fs::write(bundle.join("bundle.css"), ".calendar {}\n").unwrap();
  std::fs::write(bundle.join("words.json"), r#"{"en":{"calendar":{"title":"Calendar"}}}"#).unwrap();
  let service = Service::start(&stand().await, folder).await;

  let (status, manifest) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!(status, 200);
  assert_eq!(manifest["name"], "calendar");
  assert_eq!(manifest["title"], "calendar.title");
  assert_eq!(manifest["icon"], "i-material-symbols-calendar-month-outline");
  assert_eq!(manifest["screenPath"], "/calendar");
  assert_eq!(manifest["serviceName"], "calendar");
  assert_eq!(manifest["apiPrefix"], "/");
  assert_eq!(manifest["role"], "member");
  assert_eq!(manifest["ownerKey"], true);
  assert_eq!(manifest["events"], true);
  assert_eq!(manifest["pushKinds"], json!(["alarm.told"]));
  assert_eq!(manifest["words"], "/console/words");

  let module = manifest["bundle"].as_str().unwrap();
  assert!(module.starts_with("/console/bundle-") && module.ends_with(".js"));
  let response = reqwest::Client::new()
    .get(format!("{}{module}", service.address))
    .bearer_auth(CONSOLE)
    .send()
    .await
    .unwrap();
  assert_eq!(response.status(), 200);
  assert_eq!(response.headers()["content-type"], "text/javascript");
  assert_eq!(response.headers()["cache-control"], "private, max-age=31536000, immutable");
  assert_eq!(response.text().await.unwrap(), "export default {};\n");

  let (_, words) = service.ask(Method::GET, "/console/words", Some(CONSOLE), &[], None).await;
  assert_eq!(words["en"]["calendar"]["title"], "Calendar");
}

#[tokio::test]
async fn a_reader_is_given_the_organizations_events_and_its_own() {
  let service = Service::fresh().await;
  for account in ["2", "3"] {
    service.console(Method::GET, "/mine/calendars", account, "member", None).await;
  }
  service.get("/organization/calendars").await;
  let read = async |key: &str| {
    let headers = [("Owner-Key", key), ("Owner-Role", "member"), ("accept", "application/json")];
    let (status, page) = service
      .ask(Method::GET, "/events?kinds=calendar.changed", Some(CONSOLE), &headers, None)
      .await;
    assert_eq!(status, 200, "{page}");
    page["items"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| one["data"]["calendarId"].as_str().unwrap().to_owned())
      .collect::<Vec<_>>()
  };

  assert_eq!(read("2").await, ["mine", "organization"]);
  assert_eq!(read("3").await, ["mine", "organization"]);
  let (_, all) = service
    .ask(Method::GET, "/events", Some(TOKEN), &[("accept", "application/json")], None)
    .await;
  assert_eq!(all["items"].as_array().unwrap().len(), 3);

  let (status, refusal) = service
    .ask(Method::GET, "/events?after=1", Some(TOKEN), &[("accept", "application/json")], None)
    .await;
  assert_eq!((status, refusal["code"].as_str()), (410, Some("gone")));
}
