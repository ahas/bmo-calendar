//! The console's reach: the guard of every route (401, 403), the console face's manifest read through bmo-ui's
//! own reading of the contract on the release the client vendors, the face wired to the calendar's declaration
//! and credential, a 404 where no bundle is built, and the events a reader is given. The face's own rules are
//! bmo-console-face's tests'.

use std::path::{Path, PathBuf};

use bmo_calendar::console::DECLARATION;
use bmo_calendar::events::KINDS;
use bmo_console_face::read_manifest;
use reqwest::Method;
use serde_json::{Value, json};

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
async fn no_face_is_answered_where_no_bundle_is_built() {
  let service = Service::fresh().await;
  assert_eq!(service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await.0, 404);
}

#[tokio::test]
async fn a_bundle_built_without_its_layer_is_not_served() {
  let folder = scratch();
  build(&folder, None);
  let service = Service::start(&stand().await, folder).await;
  assert_eq!(service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await.0, 404);
}

#[test]
fn the_declaration_names_a_notice_the_service_publishes() {
  let declared: Value = serde_json::from_str(DECLARATION).unwrap();
  for kind in declared["pushKinds"].as_array().unwrap() {
    assert!(KINDS.contains(&kind.as_str().unwrap()), "{kind}");
  }
}

#[tokio::test]
async fn the_served_manifest_is_read_by_the_contract_on_the_vendored_release() {
  let layer = installed_layer();
  let folder = scratch();
  build(&folder, Some(&layer));
  let service = Service::start(&stand().await, folder).await;

  let (status, served) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!(status, 200, "{served}");
  let read = read_manifest(&served, &vendored_release()).unwrap_or_else(|why| panic!("{why}"));
  assert_eq!(read["layer"], layer.as_str());
  assert_eq!(read["name"], "calendar");

  // The face is guarded by the console's credential, not the service's own token.
  assert_eq!(fetch(&service, "/console-manifest", TOKEN, &[]).await.0, 401);

  // The words are read for no reader, and name the screen in every locale they hold.
  let (status, _, words) = fetch(&service, read["words"].as_str().unwrap(), CONSOLE, &[]).await;
  assert_eq!(status, 200);
  let words: Value = serde_json::from_str(&words).unwrap();
  for locale in ["en", "ko"] {
    assert!(words[locale]["calendar"]["title"].is_string(), "{locale}: {words}");
  }

  // The module and the stylesheet are a reader's, the owner key among what the declaration asks.
  let module = read["bundle"].as_str().unwrap();
  let stylesheet = read["stylesheet"].as_str().unwrap();
  assert_eq!(fetch(&service, module, CONSOLE, &[("Owner-Role", "member")]).await.0, 403);
  let reader = [("Owner-Key", "2"), ("Owner-Role", "member")];
  assert_eq!(fetch(&service, module, CONSOLE, &reader).await.2, MODULE);
  assert_eq!(fetch(&service, stylesheet, CONSOLE, &reader).await.2, STYLESHEET);
}

const MODULE: &str = "export default {};\n";
const STYLESHEET: &str = ".calendar {}\n";

/// A built bundle in the folder the service reads, its layer file written where one is given.
fn build(folder: &Path, layer: Option<&str>) {
  let bundle = folder.join("bundle");
  std::fs::create_dir_all(&bundle).unwrap();
  std::fs::write(bundle.join("bundle.js"), MODULE).unwrap();
  std::fs::write(bundle.join("bundle.css"), STYLESHEET).unwrap();
  let words =
    json!({ "en": { "calendar": { "title": "Calendar" } }, "ko": { "calendar": { "title": "Calendar" } } });
  std::fs::write(bundle.join("words.json"), words.to_string()).unwrap();
  if let Some(layer) = layer {
    std::fs::write(bundle.join("layer.json"), json!({ "layer": layer }).to_string()).unwrap();
  }
}

/// The client's folder.
const CLIENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/client");

/// The release of bmo-ui the client has installed, as its own layer file names it.
fn installed_layer() -> String {
  let path = Path::new(CLIENT).join("node_modules/bmo-ui/shared/layer.json");
  let file: Value =
    serde_json::from_slice(&std::fs::read(path).expect("the client's packages are installed")).unwrap();
  file["layer"].as_str().unwrap().to_owned()
}

/// The bmo-ui release file the client vendors, as its package manifest names it.
fn vendored_release() -> PathBuf {
  let manifest: Value =
    serde_json::from_slice(&std::fs::read(Path::new(CLIENT).join("package.json")).unwrap()).unwrap();
  let taken = manifest["devDependencies"]["bmo-ui"].as_str().unwrap();
  Path::new(CLIENT).join(taken.strip_prefix("file:").expect("bmo-ui is taken as a file"))
}

/// One GET under `bearer` with `headers`: its status, its cache line and its body.
async fn fetch(
  service: &Service,
  path: &str,
  bearer: &str,
  headers: &[(&str, &str)],
) -> (u16, String, String) {
  let mut request = reqwest::Client::new().get(format!("{}{path}", service.address)).bearer_auth(bearer);
  for (name, value) in headers {
    request = request.header(*name, *value);
  }
  let response = request.send().await.unwrap();
  let status = response.status().as_u16();
  let kept = response
    .headers()
    .get("cache-control")
    .and_then(|value| value.to_str().ok())
    .unwrap_or("")
    .to_owned();
  (status, kept, response.text().await.unwrap())
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
