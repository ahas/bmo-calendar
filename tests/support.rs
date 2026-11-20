//! What the tests share: stand-ins of the brain, the receiver, the account service, agency, the node service,
//! bmo-ms-teams and Microsoft Graph on one loopback port, the service started over them in this process, and
//! calls of its routes.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as Sent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, patch, post};
use bmo_calendar::settings::Settings;
use chrono::{Duration as Span, Local, NaiveDateTime};
use futures_util::{StreamExt, stream};
use serde_json::{Value, json};
use tokio::sync::{broadcast, oneshot};

/// The stand-ins' one token.
pub const PEER_TOKEN: &str = "peer-token";
/// The service token.
pub const TOKEN: &str = "service-token";
/// The console's credential.
pub const CONSOLE: &str = "console-token";

/// The owner as the receiver and the account service know it.
pub const OWNER_SENDER: i64 = 100;
pub const OWNER_ROOM: i64 = 500;
pub const OWNER_ACCOUNT: &str = "1";
/// A member with a registered room of their own.
pub const MEMBER_WITH_ROOM: &str = "2";
pub const MEMBER_ROOM: i64 = 200;
/// A member with no room the receiver holds.
pub const MEMBER_WITHOUT_ROOM: &str = "3";
/// A member of another company.
pub const OTHER_MEMBER: &str = "4";
/// An account standing in no company.
pub const NO_COMPANY: &str = "5";
/// The company of the owner and the first members.
pub const COMPANY: &str = "10";
/// A supervisor agency names member 2 the owner of, and one of no account.
pub const MEMBER_SUPERVISOR: &str = "supervisor-b";
pub const NO_ACCOUNT_SUPERVISOR: &str = "supervisor-none";
/// The calendar's own caller token at the node service and bmo-ms-teams.
pub const CALLER_TOKEN: &str = "calendar-caller-token";
/// The node the settings name for the deployment's own account, and the account that node and the Teams page
/// know him by.
pub const OWNER_NODE: &str = "owner-pc";
pub const OWNER_NODE_ACCOUNT: &str = "9001";
/// The Teams page token the stand-in hands out.
pub const PAGE_TOKEN: &str = "page-token";

/// What the stand-ins hold and were asked.
#[derive(Default)]
pub struct Peers {
  pub brains: Vec<Value>,
  pub memories: Vec<Value>,
  pub events: Vec<Value>,
  pub instructions: Vec<Value>,
  /// The rooms the receiver lists for an account, and the account each private room among them is kept for.
  pub account_rooms: BTreeMap<String, Vec<i64>>,
  pub private_rooms: BTreeMap<i64, i64>,
  pub next: i64,
  /// A `createdAt` the next memory written takes in place of now.
  pub written_at: Option<i64>,
  /// Agency's records of people's nodes.
  pub person_nodes: Vec<Value>,
  /// What the command a run carries answers, before its base64; the stdout of a run as it stands where none.
  pub node_day: Option<Value>,
  /// The runs asked: the node, the body and the bearer.
  pub runs: Vec<(String, Value, String)>,
  /// Whether the Teams page holds a token.
  pub page_token: bool,
  /// The page tokens asked: the body and the bearer.
  pub page_token_asks: Vec<(Value, String)>,
  /// The events Graph holds for the day, its first one on a page of its own.
  pub graph_events: Vec<Value>,
  /// Graph's pages asked: the query, the `Prefer` header and the bearer.
  pub graph_asks: Vec<(BTreeMap<String, String>, String, String)>,
}

/// The stand-ins and the address they answer at.
#[derive(Clone)]
pub struct Stand {
  pub peers: Arc<Mutex<Peers>>,
  pub live: broadcast::Sender<Value>,
  pub address: String,
}

fn now() -> i64 {
  bmo_calendar::model::now_ms()
}

fn page(items: Vec<Value>) -> Json<Value> {
  Json(json!({ "items": items, "more": false }))
}

fn refused(status: StatusCode, code: &str) -> Response {
  (status, Json(json!({ "code": code, "error": code }))).into_response()
}

impl Stand {
  fn publish(&self, peers: &mut Peers, kind: &str, brain: i64, data: Value) {
    peers.next += 1;
    let event = json!({ "id": peers.next, "kind": kind, "at": now(), "data": data, "scope": format!("brain:{brain}") });
    peers.events.push(event.clone());
    let _ = self.live.send(event);
  }

  /// A memory written straight into a brain, as another writer would.
  pub fn write(&self, brain: i64, mut body: Value) -> Value {
    let mut peers = self.peers.lock().unwrap();
    peers.next += 1;
    let id = peers.next;
    let created_at = peers.written_at.take().unwrap_or_else(now);
    let memory = json!({
      "id": id,
      "brainId": brain,
      "key": body["key"].take(),
      "type": body["type"].take(),
      "text": body["text"].take(),
      "status": "active",
      "lineageId": id,
      "validFrom": body["validFrom"].as_i64().unwrap_or(created_at),
      "labels": body["labels"].take(),
      "actorName": body["actorName"].take(),
      "createdAt": created_at,
    });
    peers.memories.push(memory.clone());
    self.publish(&mut peers, "memory.written", brain, memory.clone());
    memory
  }

  /// The brain kept under an account key, made where none stands.
  pub fn brain(&self, account_key: &str) -> i64 {
    let mut peers = self.peers.lock().unwrap();
    if let Some(brain) = peers.brains.iter().find(|brain| brain["accountKey"] == account_key) {
      return brain["id"].as_i64().unwrap();
    }
    peers.next += 1;
    let id = peers.next;
    peers
      .brains
      .push(json!({ "id": id, "accountKey": account_key, "agentKey": "calendar", "name": "calendar" }));
    id
  }

  /// The live memories of a brain whose key is this.
  pub fn live(&self, key: &str) -> Vec<Value> {
    let peers = self.peers.lock().unwrap();
    peers
      .memories
      .iter()
      .filter(|memory| memory["key"] == key && memory["status"] == "active")
      .cloned()
      .collect()
  }

  /// Every memory, of every status, whose key is this.
  pub fn lineage(&self, key: &str) -> Vec<Value> {
    let peers = self.peers.lock().unwrap();
    peers.memories.iter().filter(|memory| memory["key"] == key).cloned().collect()
  }

  pub fn instructions(&self) -> Vec<Value> {
    self.peers.lock().unwrap().instructions.clone()
  }
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct BrainsQuery {
  account_key: Option<String>,
  agent_key: Option<String>,
}

async fn list_brains(State(stand): State<Stand>, Query(query): Query<BrainsQuery>) -> Json<Value> {
  let peers = stand.peers.lock().unwrap();
  page(
    peers
      .brains
      .iter()
      .filter(|brain| query.account_key.as_ref().is_none_or(|key| brain["accountKey"] == key.as_str()))
      .filter(|brain| query.agent_key.as_ref().is_none_or(|key| brain["agentKey"] == key.as_str()))
      .cloned()
      .collect(),
  )
}

async fn make_brain(State(stand): State<Stand>, Json(body): Json<Value>) -> Response {
  let key = body["accountKey"].as_str().unwrap_or_default().to_owned();
  if stand.peers.lock().unwrap().brains.iter().any(|brain| brain["accountKey"] == key) {
    return refused(StatusCode::CONFLICT, "conflict");
  }
  let id = stand.brain(&key);
  let brain = stand
    .peers
    .lock()
    .unwrap()
    .brains
    .iter()
    .find(|brain| brain["id"] == id)
    .cloned()
    .unwrap();
  (StatusCode::CREATED, Json(brain)).into_response()
}

#[derive(serde::Deserialize, Default)]
struct MemoriesQuery {
  #[serde(rename = "type")]
  kind: Option<String>,
  status: Option<String>,
}

async fn list_memories(
  State(stand): State<Stand>,
  Path(brain): Path<i64>,
  Query(query): Query<MemoriesQuery>,
) -> Json<Value> {
  let peers = stand.peers.lock().unwrap();
  page(
    peers
      .memories
      .iter()
      .filter(|memory| memory["brainId"] == brain)
      .filter(|memory| query.kind.as_ref().is_none_or(|kind| memory["type"] == kind.as_str()))
      .filter(|memory| query.status.as_ref().is_none_or(|status| memory["status"] == status.as_str()))
      .cloned()
      .collect(),
  )
}

async fn write_memory(
  State(stand): State<Stand>,
  Path(brain): Path<i64>,
  Json(body): Json<Value>,
) -> Response {
  let key = body["key"].as_str().unwrap_or_default().to_owned();
  let taken = {
    let peers = stand.peers.lock().unwrap();
    peers
      .memories
      .iter()
      .any(|memory| memory["brainId"] == brain && memory["key"] == key && memory["status"] == "active")
  };
  if taken {
    return refused(StatusCode::CONFLICT, "conflict");
  }
  (StatusCode::CREATED, Json(stand.write(brain, body))).into_response()
}

async fn write_version(
  State(stand): State<Stand>,
  Path((brain, id)): Path<(i64, i64)>,
  Json(body): Json<Value>,
) -> Response {
  let mut peers = stand.peers.lock().unwrap();
  let Some(at) = peers
    .memories
    .iter()
    .position(|memory| memory["id"] == id && memory["status"] == "active")
  else {
    return refused(StatusCode::CONFLICT, "conflict");
  };
  peers.next += 1;
  let new_id = peers.next;
  let mut old = peers.memories[at].clone();
  let mut new = old.clone();
  old["status"] = json!("superseded");
  old["supersededBy"] = json!(new_id);
  peers.memories[at] = old.clone();
  new["id"] = json!(new_id);
  new["createdAt"] = json!(now());
  for field in ["text", "validFrom", "labels", "actorName"] {
    if !body[field].is_null() {
      new[field] = body[field].clone();
    }
  }
  peers.memories.push(new.clone());
  stand.publish(&mut peers, "memory.moved", brain, old);
  stand.publish(&mut peers, "memory.written", brain, new.clone());
  (StatusCode::CREATED, Json(new)).into_response()
}

async fn move_memory(
  State(stand): State<Stand>,
  Path((brain, id)): Path<(i64, i64)>,
  Json(body): Json<Value>,
) -> Response {
  let mut peers = stand.peers.lock().unwrap();
  let Some(at) = peers
    .memories
    .iter()
    .position(|memory| memory["id"] == id && memory["status"] == "active")
  else {
    return refused(StatusCode::CONFLICT, "conflict");
  };
  peers.memories[at]["status"] = body["status"].clone();
  peers.memories[at]["reason"] = body["reason"].clone();
  let moved = peers.memories[at].clone();
  stand.publish(&mut peers, "memory.moved", brain, moved.clone());
  Json(moved).into_response()
}

#[derive(serde::Deserialize, Default)]
struct EventsQuery {
  scope: Option<String>,
  after: Option<i64>,
}

async fn brain_events(State(stand): State<Stand>, Query(query): Query<EventsQuery>) -> Response {
  let live = stand.live.subscribe();
  let scope = query.scope.clone();
  let backlog: Vec<Value> = match query.after {
    Some(after) => stand
      .peers
      .lock()
      .unwrap()
      .events
      .iter()
      .filter(|event| event["id"].as_i64().unwrap_or(0) > after)
      .cloned()
      .collect(),
    None => Vec::new(),
  };
  let live = stream::unfold(live, |mut live| async move {
    loop {
      match live.recv().await {
        Ok(event) => return Some((event, live)),
        Err(broadcast::error::RecvError::Lagged(_)) => continue,
        Err(_) => return None,
      }
    }
  });
  let events = stream::iter(backlog)
    .chain(live)
    .filter(move |event| {
      std::future::ready(scope.as_ref().is_none_or(|scope| event["scope"] == scope.as_str()))
    })
    .map(|event| {
      Ok::<_, Infallible>(
        Sent::default()
          .event(event["kind"].as_str().unwrap_or(""))
          .id(event["id"].to_string())
          .data(event.to_string()),
      )
    });
  Sse::new(events).keep_alive(KeepAlive::default()).into_response()
}

async fn owner() -> Json<Value> {
  Json(json!({ "senderId": OWNER_SENDER, "room": OWNER_ROOM }))
}

async fn room(State(stand): State<Stand>, Path(room): Path<i64>) -> Json<Value> {
  let private_of = stand.peers.lock().unwrap().private_rooms.get(&room).copied();
  Json(
    json!({ "id": room, "registered": room == MEMBER_ROOM || room == OWNER_ROOM, "privateOf": private_of }),
  )
}

async fn account_rooms(State(stand): State<Stand>, Path(account): Path<String>) -> Json<Value> {
  let rooms = stand.peers.lock().unwrap().account_rooms.get(&account).cloned().unwrap_or_default();
  Json(json!({ "rooms": rooms }))
}

async fn instruct(State(stand): State<Stand>, Json(body): Json<Value>) -> Response {
  let mut peers = stand.peers.lock().unwrap();
  peers.instructions.push(body);
  let id = peers.instructions.len();
  (StatusCode::CREATED, Json(json!({ "id": id }))).into_response()
}

async fn sender_user(Path(sender): Path<i64>) -> Response {
  match sender {
    OWNER_SENDER => Json(json!({ "user": 1 })).into_response(),
    _ => refused(StatusCode::NOT_FOUND, "not-found"),
  }
}

async fn user(Path(user): Path<String>) -> Response {
  let (name, senders) = match user.as_str() {
    OWNER_ACCOUNT => ("owner", vec![OWNER_SENDER]),
    MEMBER_WITH_ROOM => ("member-a", vec![MEMBER_ROOM]),
    MEMBER_WITHOUT_ROOM => ("member-b", vec![300]),
    OTHER_MEMBER => ("member-c", vec![400]),
    NO_COMPANY => ("member-d", vec![600]),
    _ => return refused(StatusCode::NOT_FOUND, "not-found"),
  };
  Json(json!({ "user": user, "name": name, "administrator": user == OWNER_ACCOUNT, "senders": senders }))
    .into_response()
}

/// The unit and company an account stands in: the owner and the first members in one company, member 4 in
/// another, member 5 in none.
async fn user_unit(Path(user): Path<String>) -> Response {
  let (unit, company) = match user.as_str() {
    OWNER_ACCOUNT | MEMBER_WITH_ROOM => {
      (json!({ "id": 10, "name": "Company A" }), json!({ "id": 10, "name": "Company A" }))
    }
    MEMBER_WITHOUT_ROOM => {
      (json!({ "id": 11, "name": "Team A" }), json!({ "id": 10, "name": "Company A" }))
    }
    OTHER_MEMBER => (json!({ "id": 20, "name": "Company B" }), json!({ "id": 20, "name": "Company B" })),
    _ => return refused(StatusCode::NOT_FOUND, "not-found"),
  };
  Json(json!({ "user": user, "unit": unit, "company": company, "path": [company, unit] })).into_response()
}

async fn units(headers: axum::http::HeaderMap) -> Response {
  if headers.get("owner-role").and_then(|role| role.to_str().ok()) != Some("administrator") {
    return refused(StatusCode::FORBIDDEN, "forbidden");
  }
  Json(json!({ "units": [
    { "id": 10, "name": "Company A", "heads": [], "members": [] },
    { "id": 20, "name": "Company B", "heads": [], "members": [] },
    { "id": 11, "name": "Team A", "parent": 10, "heads": [], "members": [] },
  ] }))
  .into_response()
}

/// Member 2 heads company 10. The ask is held to the account service's shape, `unit` a number and `user` a
/// string, and any other is refused 400 `bad-request` as the account service refuses it.
async fn unit_check(Json(body): Json<Value>) -> Response {
  let (Some(unit), Some(user)) = (body["unit"].as_i64(), body["user"].as_str()) else {
    return refused(StatusCode::BAD_REQUEST, "bad-request");
  };
  let allowed = unit == 10 && user == "2";
  Json(json!({ "allowed": allowed, "grant": format!("organization:{unit}#manage") })).into_response()
}

async fn supervisor(Path(name): Path<String>) -> Response {
  match name.as_str() {
    MEMBER_SUPERVISOR => Json(json!({ "name": name, "ownerAccount": 2 })).into_response(),
    NO_ACCOUNT_SUPERVISOR => Json(json!({ "name": name, "ownerAccount": null })).into_response(),
    _ => refused(StatusCode::NOT_FOUND, "not-found"),
  }
}

async fn role_agents() -> Json<Value> {
  Json(json!({ "items": [{ "name": "role-agent", "role": "voyager", "ownerAccount": 3 }], "more": false }))
}

fn bearer_of(headers: &axum::http::HeaderMap) -> String {
  headers
    .get("authorization")
    .and_then(|value| value.to_str().ok())
    .and_then(|value| value.strip_prefix("Bearer "))
    .unwrap_or("")
    .to_owned()
}

async fn person_nodes(State(stand): State<Stand>) -> Json<Value> {
  page(stand.peers.lock().unwrap().person_nodes.clone())
}

/// A run, answered by the command's answer in base64 on its last line, as the PowerShell command writes it.
async fn node_run(
  State(stand): State<Stand>,
  Path(node): Path<String>,
  headers: axum::http::HeaderMap,
  Json(body): Json<Value>,
) -> Response {
  use base64::Engine;
  let mut peers = stand.peers.lock().unwrap();
  peers.runs.push((node, body, bearer_of(&headers)));
  let Some(day) = peers.node_day.clone() else {
    return Json(json!({ "exitCode": 1, "stdout": "", "stderr": "The term is not recognized." }))
      .into_response();
  };
  let encoded = base64::engine::general_purpose::STANDARD.encode(day.to_string());
  Json(json!({ "node": "n", "shell": "powershell", "exitCode": 0, "signal": null, "stdout": format!("{encoded}\r\n"),
    "stderr": "", "stdoutCut": false, "stderrCut": false }))
  .into_response()
}

async fn page_tokens(
  State(stand): State<Stand>,
  headers: axum::http::HeaderMap,
  Json(body): Json<Value>,
) -> Response {
  let mut peers = stand.peers.lock().unwrap();
  peers.page_token_asks.push((body, bearer_of(&headers)));
  if !peers.page_token {
    return (
      StatusCode::SERVICE_UNAVAILABLE,
      Json(json!({ "code": "not-ready", "error": "The page asks for a sign-in." })),
    )
      .into_response();
  }
  Json(json!({ "token": PAGE_TOKEN, "scopes": "Calendars.ReadWrite", "expiresAt": now() + 3_600_000, "node": "n" }))
    .into_response()
}

/// Graph's calendar view: the first event on a page of its own, the rest on the page its next link names.
async fn calendar_view(
  State(stand): State<Stand>,
  Query(query): Query<BTreeMap<String, String>>,
  headers: axum::http::HeaderMap,
) -> Response {
  let mut peers = stand.peers.lock().unwrap();
  let prefer = headers.get("prefer").and_then(|value| value.to_str().ok()).unwrap_or("").to_owned();
  peers.graph_asks.push((query.clone(), prefer, bearer_of(&headers)));
  if bearer_of(&headers) != PAGE_TOKEN {
    return (
      StatusCode::UNAUTHORIZED,
      Json(json!({ "error": { "code": "InvalidAuthenticationToken", "message": "Bad token." } })),
    )
      .into_response();
  }
  let events = peers.graph_events.clone();
  if query.contains_key("second") {
    return Json(json!({ "value": events.get(1..).unwrap_or_default() })).into_response();
  }
  let next = format!("{}/me/calendarView?second=1", stand.address);
  Json(json!({ "value": events.get(..1).unwrap_or_default(), "@odata.nextLink": next })).into_response()
}

/// The stand-ins, answering on a fresh loopback port.
pub async fn stand() -> Stand {
  let (live, _) = broadcast::channel(1000);
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let stand = Stand {
    peers: Arc::new(Mutex::new(Peers::default())),
    live,
    address: format!("http://{}", listener.local_addr().unwrap()),
  };
  let router = Router::new()
    .route("/brains", get(list_brains).post(make_brain))
    .route("/brains/{brain}/memories", get(list_memories).post(write_memory))
    .route("/brains/{brain}/memories/{id}", patch(move_memory))
    .route("/brains/{brain}/memories/{id}/versions", post(write_version))
    .route("/events", get(brain_events))
    .route("/owner", get(owner))
    .route("/rooms/{room}", get(room))
    .route("/accounts/{account}/rooms", get(account_rooms))
    .route("/instructions", post(instruct))
    .route("/senders/{sender}/user", get(sender_user))
    .route("/users/{user}", get(user))
    .route("/users/{user}/unit", get(user_unit))
    .route("/units", get(units))
    .route("/checks/unit", post(unit_check))
    .route("/supervisors/{name}", get(supervisor))
    .route("/role-agents", get(role_agents))
    .route("/person-nodes", get(person_nodes))
    .route("/nodes/{node}/runs", post(node_run))
    .route("/page-tokens", post(page_tokens))
    .route("/me/calendarView", get(calendar_view))
    .with_state(stand.clone());
  tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
  stand
}

/// A fresh folder of its own under the system's scratch folder.
pub fn scratch() -> PathBuf {
  static COUNT: AtomicU64 = AtomicU64::new(0);
  let folder = std::env::temp_dir().join(format!(
    "bmo-calendar-test-{}-{}-{}",
    std::process::id(),
    now(),
    COUNT.fetch_add(1, Ordering::Relaxed)
  ));
  std::fs::create_dir_all(&folder).unwrap();
  folder
}

/// The service running over the stand-ins.
pub struct Service {
  pub address: String,
  pub folder: PathBuf,
  pub stand: Stand,
  stop: Option<oneshot::Sender<()>>,
  ended: Option<tokio::task::JoinHandle<Result<(), String>>>,
}

/// Settings over the stand-ins, the store and the console bundle in `folder`.
pub fn settings(stand: &Stand, folder: &std::path::Path) -> Settings {
  let local = format!(
    "[server]\nlisten = \"127.0.0.1:0\"\ntoken = \"{TOKEN}\"\ndata_root = \"{root}\"\n\
     [console]\ntoken = \"{CONSOLE}\"\nfolder = \"{bundle}\"\n\
     [store]\npath = \"{store}\"\n\
     [brain]\naddress = \"{peer}\"\ntoken = \"{PEER_TOKEN}\"\n\
     [receiver]\naddress = \"{peer}\"\ntoken = \"{PEER_TOKEN}\"\n\
     [accounts]\naddress = \"{peer}\"\ntoken = \"{PEER_TOKEN}\"\n\
     [agency]\naddress = \"{peer}\"\ntoken = \"{PEER_TOKEN}\"\n\
     [node]\naddress = \"{peer}\"\ntoken = \"{CALLER_TOKEN}\"\n\
     [teams]\naddress = \"{peer}\"\ntoken = \"{CALLER_TOKEN}\"\n\
     [outlook]\nowner_node = \"{OWNER_NODE}\"\nowner_account = \"{OWNER_NODE_ACCOUNT}\"\n\
     graph_address = \"{peer}\"\n",
    root = folder.join("data").display(),
    bundle = folder.join("bundle").display(),
    store = folder.join("data/calendar/calendar.db").display(),
    peer = stand.address,
  );
  std::fs::write(folder.join("settings.local.toml"), local).unwrap();
  Settings::read(folder, Some(Default::default())).unwrap()
}

impl Service {
  /// The service started over the stand-ins in `folder`, once it answers ready.
  pub async fn start(stand: &Stand, folder: PathBuf) -> Self {
    let settings = settings(stand, &folder);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = oneshot::channel();
    let ended = tokio::spawn(bmo_calendar::serve_on(settings, listener, async move {
      let _ = stopped.await;
    }));
    let service = Self {
      address,
      folder,
      stand: stand.clone(),
      stop: Some(stop),
      ended: Some(ended),
    };
    for _ in 0..500 {
      let health = service.get("/health").await;
      if health.1["ready"] == true {
        return service;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the service never answered ready");
  }

  /// The service over fresh stand-ins in a fresh folder.
  pub async fn fresh() -> Self {
    Self::start(&stand().await, scratch()).await
  }

  /// Stops the service and waits for it to end.
  pub async fn stop(mut self) -> (Stand, PathBuf) {
    if let Some(stop) = self.stop.take() {
      let _ = stop.send(());
    }
    if let Some(ended) = self.ended.take() {
      ended.await.unwrap().unwrap();
    }
    (self.stand.clone(), self.folder.clone())
  }

  /// One call with the given bearer and headers.
  pub async fn ask(
    &self,
    method: reqwest::Method,
    path: &str,
    bearer: Option<&str>,
    headers: &[(&str, &str)],
    body: Option<Value>,
  ) -> (u16, Value) {
    let mut request = reqwest::Client::new().request(method, format!("{}{path}", self.address));
    if let Some(bearer) = bearer {
      request = request.bearer_auth(bearer);
    }
    for (name, value) in headers {
      request = request.header(*name, *value);
    }
    if let Some(body) = body {
      request = request.json(&body);
    }
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
  }

  /// A call with the service token.
  pub async fn call(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> (u16, Value) {
    self.ask(method, path, Some(TOKEN), &[], body).await
  }

  pub async fn get(&self, path: &str) -> (u16, Value) {
    self.call(reqwest::Method::GET, path, None).await
  }

  pub async fn post(&self, path: &str, body: Value) -> (u16, Value) {
    self.call(reqwest::Method::POST, path, Some(body)).await
  }

  pub async fn patch(&self, path: &str, body: Value) -> (u16, Value) {
    self.call(reqwest::Method::PATCH, path, Some(body)).await
  }

  pub async fn delete(&self, path: &str) -> (u16, Value) {
    self.call(reqwest::Method::DELETE, path, None).await
  }

  /// A call through the console with a reader's key and role.
  pub async fn console(
    &self,
    method: reqwest::Method,
    path: &str,
    key: &str,
    role: &str,
    body: Option<Value>,
  ) -> (u16, Value) {
    self
      .ask(method, path, Some(CONSOLE), &[("Owner-Key", key), ("Owner-Role", role)], body)
      .await
  }

  /// Waits until a condition on the service's answers holds.
  pub async fn until(&self, mut holds: impl AsyncFnMut(&Self) -> bool) {
    for _ in 0..500 {
      if holds(self).await {
        return;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the condition never held");
  }
}

/// A local datetime this far from now, to the second.
pub fn local_in(minutes: i64) -> String {
  let at: NaiveDateTime = Local::now().naive_local() + Span::minutes(minutes);
  at.format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// The set of a page's ids.
pub fn ids(page: &Value) -> BTreeSet<String> {
  page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["id"].as_str().unwrap().to_owned())
    .collect()
}

/// The labels of a memory, as a map.
pub fn labels(memory: &Value) -> BTreeMap<String, String> {
  serde_json::from_value(memory["labels"].clone()).unwrap()
}
