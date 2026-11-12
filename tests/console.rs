//! The console's reach: the guard of every route (401, 403), the bundle's manifest read through bmo-ui's own
//! reading of the contract, its module, stylesheet and words to the console alone, a 404 where no bundle is
//! built, and the events a reader is given.

use std::path::Path;

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
async fn no_bundle_is_answered_where_none_is_built() {
  let service = Service::fresh().await;
  for path in ["/console-manifest", "/console/words", "/console/bundle-0000000000000000.js"] {
    let (status, refusal) = service.ask(Method::GET, path, Some(CONSOLE), &[], None).await;
    assert_eq!((status, refusal["code"].as_str()), (404, Some("not-found")), "{path}");
  }
}

#[tokio::test]
async fn a_bundle_built_without_its_layer_is_not_served() {
  let folder = scratch();
  build(&folder, None);
  let service = Service::start(&stand().await, folder).await;
  let (status, refusal) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!((status, refusal["code"].as_str()), (404, Some("not-found")));
}

#[tokio::test]
async fn a_built_bundle_is_answered_by_a_manifest_the_contract_reads() {
  let layer = installed_layer();
  let folder = scratch();
  build(&folder, Some(&layer));
  let service = Service::start(&stand().await, folder).await;

  let (status, served) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!(status, 200, "{served}");
  let read = read_by_contract(&served.to_string());
  assert_eq!(read["onThisLayer"], true, "{read}");
  let manifest = &read["manifest"];
  assert_eq!(manifest["layer"], layer.as_str());
  for (field, chosen) in [
    ("name", json!("calendar")),
    ("title", json!("calendar.title")),
    ("screenPath", json!("/calendar")),
    ("serviceName", json!("calendar")),
    ("apiPrefix", json!("/")),
    ("role", json!("member")),
    ("ownerKey", json!(true)),
    ("events", json!(true)),
    ("websocket", json!(false)),
    ("pushKinds", json!(["alarm.told"])),
    ("shareReads", json!([])),
  ] {
    assert_eq!(manifest[field], chosen, "{field}");
  }

  // The words name the screen in every locale they hold.
  let (status, words) = service
    .ask(Method::GET, manifest["words"].as_str().unwrap(), Some(CONSOLE), &[], None)
    .await;
  assert_eq!(status, 200);
  for locale in ["en", "ko"] {
    assert!(words[locale]["calendar"]["title"].is_string(), "{locale}: {words}");
  }

  let module = manifest["bundle"].as_str().unwrap();
  let stylesheet = manifest["stylesheet"].as_str().unwrap();
  assert!(module.starts_with("/console/bundle-") && module.ends_with(".js"), "{module}");
  let reader = [("Owner-Key", "2"), ("Owner-Role", "member")];
  let (status, kept, body) = fetch(&service, module, CONSOLE, &reader).await;
  assert_eq!(
    (status, kept.as_str(), body.as_str()),
    (200, "private, max-age=31536000, immutable", MODULE)
  );
  let (status, _, body) = fetch(&service, stylesheet, CONSOLE, &reader).await;
  assert_eq!((status, body.as_str()), (200, STYLESHEET));
}

#[tokio::test]
async fn the_bundles_routes_are_the_consoles_alone() {
  let folder = scratch();
  build(&folder, Some("0.1.0-rev.1"));
  let service = Service::start(&stand().await, folder).await;
  let (_, manifest) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  let module = manifest["bundle"].as_str().unwrap();
  let stylesheet = manifest["stylesheet"].as_str().unwrap();
  let administrator = [("Owner-Key", "1"), ("Owner-Role", "administrator")];

  // The service's own token is no console credential, on the four routes alike.
  for path in ["/console-manifest", "/console/words", module, stylesheet] {
    assert_eq!(fetch(&service, path, TOKEN, &administrator).await.0, 401, "{path}");
  }
  // The manifest and the words are read for no reader; the module and the stylesheet are a reader's.
  for path in ["/console-manifest", "/console/words"] {
    assert_eq!(fetch(&service, path, CONSOLE, &[]).await.0, 200, "{path}");
  }
  for path in [module, stylesheet] {
    assert_eq!(fetch(&service, path, CONSOLE, &[]).await.0, 403, "{path}");
    assert_eq!(fetch(&service, path, CONSOLE, &[("Owner-Key", "1")]).await.0, 403, "{path}");
    assert_eq!(fetch(&service, path, CONSOLE, &[("Owner-Role", "member")]).await.0, 403, "{path}");
    assert_eq!(fetch(&service, path, CONSOLE, &administrator).await.0, 200, "{path}");
  }
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

/// The release of bmo-ui the client has installed, as its own layer file names it.
fn installed_layer() -> String {
  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/client/node_modules/bmo-ui/shared/layer.json");
  let file: Value =
    serde_json::from_slice(&std::fs::read(path).expect("the client's packages are installed")).unwrap();
  file["layer"].as_str().unwrap().to_owned()
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

/// The manifest read through bmo-ui's own reading of the contract, by the client's script under `bun`.
fn read_by_contract(manifest: &str) -> Value {
  use std::io::Write as _;
  use std::process::{Command, Stdio};

  let mut reading = Command::new("bun")
    .args(["tests/read-manifest.ts"])
    .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/client"))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("bun runs in client/");
  reading
    .stdin
    .take()
    .expect("its input")
    .write_all(manifest.as_bytes())
    .expect("the manifest is handed over");
  let read = reading.wait_with_output().expect("the reading ends");
  assert!(
    read.status.success(),
    "the contract refuses the manifest: {}",
    String::from_utf8_lossy(&read.stderr)
  );
  serde_json::from_slice(&read.stdout).expect("the reading answers JSON")
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
