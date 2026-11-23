//! The units the calendar calls (`docs/api.md`, 「Calls the calendar makes」): the brain, the receiver, the
//! account service, agency, the node service and the Teams page service, each found afresh for every call from the settings
//! or the data root's files, and called with no clock on the answer. A call whose connection is refused is asked
//! again two seconds later.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::model::Memory;
use crate::settings::{Peer, Settings};

/// How long a refused connection waits before it is asked again.
pub const AGAIN: Duration = Duration::from_secs(2);

/// The agent key every realm's brain is kept under.
pub const AGENT_KEY: &str = "calendar";

/// How many members a page of the brain's is asked for.
const PAGE: &str = "500";

/// One unit as a call reaches it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
  pub name: &'static str,
  pub address: String,
  pub token: String,
}

/// Why a call came to nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallFault {
  /// The unit answered a failure: its status and the body's code and sentence.
  Answered {
    status: u16,
    code: String,
    error: String,
  },
  /// The unit could not be found or its answer not read.
  Broken(String),
}

impl CallFault {
  pub fn status(&self) -> Option<u16> {
    match self {
      Self::Answered { status, .. } => Some(*status),
      Self::Broken(_) => None,
    }
  }
}

impl std::fmt::Display for CallFault {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Answered {
        status,
        code,
        error,
      } => write!(formatter, "{error} ({status} {code})"),
      Self::Broken(why) => formatter.write_str(why),
    }
  }
}

impl From<CallFault> for ApiError {
  fn from(fault: CallFault) -> Self {
    ApiError::upstream(fault.to_string())
  }
}

/// An address under a listen setting, `127.0.0.1:13105` read as `http://127.0.0.1:13105`.
pub fn address(listen: &str) -> String {
  let listen = listen.trim().trim_end_matches('/');
  if listen.contains("://") {
    listen.to_owned()
  } else if let Some(port) = listen.strip_prefix("0.0.0.0:") {
    format!("http://127.0.0.1:{port}")
  } else {
    format!("http://{listen}")
  }
}

/// The `KEY=value` lines of an env file, none where it does not stand.
fn env_file(path: &Path) -> BTreeMap<String, String> {
  std::fs::read_to_string(path)
    .map(|text| {
      text
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().trim_matches('"').to_owned()))
        .collect()
    })
    .unwrap_or_default()
}

fn toml_file(path: &Path) -> toml::Table {
  std::fs::read_to_string(path)
    .ok()
    .and_then(|text| text.parse().ok())
    .unwrap_or_default()
}

fn named(name: &'static str, peer: &Peer) -> Option<Reach> {
  (!peer.address.is_empty()).then(|| Reach {
    name,
    address: address(&peer.address),
    token: peer.token.clone(),
  })
}

fn found(name: &'static str, address: Option<String>, token: Option<String>) -> Result<Reach, CallFault> {
  match (address, token) {
    (Some(address), Some(token)) => Ok(Reach {
      name,
      address,
      token,
    }),
    _ => Err(CallFault::Broken(format!("the {name}'s address or token is set nowhere"))),
  }
}

/// The brain: the settings', else `BRAIN_ADDRESS` and `BRAIN_TOKEN` of the data root's `env/brain.env`.
pub fn brain(settings: &Settings) -> Result<Reach, CallFault> {
  if let Some(reach) = named("brain", &settings.brain) {
    return Ok(reach);
  }
  let file = env_file(&settings.data_root().join("env/brain.env"));
  found(
    "brain",
    file.get("BRAIN_ADDRESS").map(|one| address(one)),
    file.get("BRAIN_TOKEN").cloned(),
  )
}

/// The receiver: the settings', else `listen` of `env/receiver.toml` and the token of `env/receiver.token`.
pub fn receiver(settings: &Settings) -> Result<Reach, CallFault> {
  if let Some(reach) = named("receiver", &settings.receiver) {
    return Ok(reach);
  }
  let root = settings.data_root();
  let listen = toml_file(&root.join("env/receiver.toml"))
    .get("listen")
    .and_then(|v| v.as_str())
    .map(address);
  found("receiver", listen, token_file(&root.join("env/receiver.token")))
}

/// A token kept alone in a file, its surrounding whitespace dropped; none where the file holds none.
fn token_file(path: &Path) -> Option<String> {
  let token = std::fs::read_to_string(path).ok()?.trim().to_owned();
  (!token.is_empty()).then_some(token)
}

/// The node service, with the calendar's own caller token: the settings', else `listen` of `env/node.toml` and
/// the token of `env/calendar-node.token`.
pub fn node(settings: &Settings) -> Result<Reach, CallFault> {
  if let Some(reach) = named("node service", &settings.node) {
    return Ok(reach);
  }
  let root = settings.data_root();
  let listen = toml_file(&root.join("env/node.toml"))
    .get("listen")
    .and_then(|v| v.as_str())
    .map(address);
  found("node service", listen, token_file(&root.join("env/calendar-node.token")))
}

/// The Teams page service's port the data root's `env/ms-teams.env` stands for where it names none.
const TEAMS_PORT: &str = "3240";

/// The Teams page service, with the calendar's own page token caller token: the settings', else `BMO_MS_TEAMS_PORT` of
/// `env/ms-teams.env` on the loopback and the token of `env/ms-teams-calendar-caller.token`.
pub fn teams(settings: &Settings) -> Result<Reach, CallFault> {
  if let Some(reach) = named("Teams page service", &settings.teams) {
    return Ok(reach);
  }
  let root = settings.data_root();
  let file = env_file(&root.join("env/ms-teams.env"));
  let port = file.get("BMO_MS_TEAMS_PORT").map_or(TEAMS_PORT, String::as_str);
  found(
    "Teams page service",
    Some(format!("http://127.0.0.1:{port}")),
    token_file(&root.join("env/ms-teams-calendar-caller.token")),
  )
}

/// The account service: the settings', else `[server] listen` and `token` of `env/web.toml`.
pub fn accounts(settings: &Settings) -> Result<Reach, CallFault> {
  if let Some(reach) = named("account service", &settings.accounts) {
    return Ok(reach);
  }
  let file = toml_file(&settings.data_root().join("env/web.toml"));
  let server = file.get("server").and_then(|server| server.as_table());
  let text =
    |key: &str| server.and_then(|server| server.get(key)).and_then(|v| v.as_str()).map(str::to_owned);
  found("account service", text("listen").map(|listen| address(&listen)), text("token"))
}

/// Agency's control port the data root's `env/agency.env` stands for where it names none.
const AGENCY_PORT: &str = "3220";

/// Agency: the settings', else the control port of the data root's `env/agency.env` on the loopback and its
/// token, `AGENCY_CONTROL_TOKEN` or the file `AGENCY_CONTROL_TOKEN_FILE` names.
pub fn agency(settings: &Settings) -> Result<Reach, CallFault> {
  if let Some(reach) = named("agency", &settings.agency) {
    return Ok(reach);
  }
  let file = env_file(&settings.data_root().join("env/agency.env"));
  let port = file.get("AGENCY_CONTROL_PORT").map_or(AGENCY_PORT, String::as_str);
  let token = file.get("AGENCY_CONTROL_TOKEN").cloned().or_else(|| {
    let path = file.get("AGENCY_CONTROL_TOKEN_FILE")?;
    let token = std::fs::read_to_string(path).ok()?.trim().to_owned();
    (!token.is_empty()).then_some(token)
  });
  found("agency", Some(format!("http://127.0.0.1:{port}")), token)
}

/// A path segment, percent-encoded.
fn segment(text: &str) -> String {
  let mut url =
    reqwest::Url::parse("http://segment/").unwrap_or_else(|_| unreachable!("a fixed address reads"));
  if let Ok(mut segments) = url.path_segments_mut() {
    segments.push(text);
  }
  url.path().trim_start_matches('/').to_owned()
}

/// The one HTTP client every call shares, never through a proxy.
pub fn http() -> reqwest::Client {
  reqwest::Client::builder().no_proxy().build().unwrap_or_default()
}

/// One request sent, asked again two seconds after a refused connection.
pub async fn send(build: impl Fn() -> RequestBuilder, name: &str) -> Result<Response, CallFault> {
  loop {
    match build().send().await {
      Ok(response) => return Ok(response),
      Err(why) if why.is_connect() => tokio::time::sleep(AGAIN).await,
      Err(why) => return Err(CallFault::Broken(format!("the {name} could not be reached: {why}"))),
    }
  }
}

/// A unit's answer read as JSON, or its failure.
pub async fn answered<T: DeserializeOwned>(response: Response, name: &str) -> Result<T, CallFault> {
  let status = response.status();
  let text = response
    .text()
    .await
    .map_err(|why| CallFault::Broken(format!("the {name}'s answer: {why}")))?;
  if !status.is_success() {
    let said: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    return Err(CallFault::Answered {
      status: status.as_u16(),
      code: said["code"].as_str().unwrap_or("error").to_owned(),
      error: said["error"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("the {name} answered {status}")),
    });
  }
  let text = if text.trim().is_empty() { "null" } else { text.as_str() };
  serde_json::from_str(text)
    .map_err(|why| CallFault::Broken(format!("the {name}'s answer does not read: {why}")))
}

/// One call of a unit's route, with its query and its body, its answer read as JSON.
pub async fn call<T: DeserializeOwned>(
  http: &reqwest::Client,
  reach: &Reach,
  method: Method,
  path: &str,
  query: &[(&str, String)],
  body: Option<&Value>,
) -> Result<T, CallFault> {
  let url = format!("{}{path}", reach.address);
  let response = send(
    || {
      let mut request = http.request(method.clone(), &url).bearer_auth(&reach.token).query(query);
      if let Some(body) = body {
        request = request.json(body);
      }
      request
    },
    reach.name,
  )
  .await?;
  answered(response, reach.name).await
}

/// A brain as the brain service answers it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrainRow {
  pub id: i64,
  pub account_key: String,
}

#[derive(Deserialize)]
struct Paged<T> {
  items: Vec<T>,
  more: bool,
}

/// The brain service's routes the calendar calls.
pub struct BrainCalls<'a> {
  pub http: &'a reqwest::Client,
  pub reach: Reach,
}

impl BrainCalls<'_> {
  /// Every brain kept under the calendar's agent key.
  pub async fn calendar_brains(&self) -> Result<Vec<BrainRow>, CallFault> {
    let mut brains = Vec::new();
    let mut after: Option<i64> = None;
    loop {
      let mut query = vec![("agentKey", AGENT_KEY.to_owned()), ("limit", PAGE.to_owned())];
      if let Some(after) = after {
        query.push(("after", after.to_string()));
      }
      let page: Paged<BrainRow> =
        call(self.http, &self.reach, Method::GET, "/brains", &query, None).await?;
      after = page.items.last().map(|brain| brain.id);
      let more = page.more && after.is_some();
      brains.extend(page.items);
      if !more {
        return Ok(brains);
      }
    }
  }

  /// The brain of an account key, where one stands.
  pub async fn find(&self, account_key: &str) -> Result<Option<BrainRow>, CallFault> {
    let query = [("accountKey", account_key.to_owned()), ("agentKey", AGENT_KEY.to_owned())];
    let page: Paged<BrainRow> = call(self.http, &self.reach, Method::GET, "/brains", &query, None).await?;
    Ok(page.items.into_iter().next())
  }

  /// The brain of an account key, made where none stands.
  pub async fn find_or_make(&self, account_key: &str) -> Result<BrainRow, CallFault> {
    if let Some(brain) = self.find(account_key).await? {
      return Ok(brain);
    }
    let body = json!({ "accountKey": account_key, "agentKey": AGENT_KEY, "name": AGENT_KEY });
    match call(self.http, &self.reach, Method::POST, "/brains", &[], Some(&body)).await {
      Ok(brain) => Ok(brain),
      Err(fault) if fault.status() == Some(409) => self
        .find(account_key)
        .await?
        .ok_or_else(|| CallFault::Broken("the brain said it stands and then did not".into())),
      Err(fault) => Err(fault),
    }
  }

  /// Every live memory of a type in a brain, by `validFrom`.
  pub async fn memories(&self, brain: i64, kind: &str) -> Result<Vec<Memory>, CallFault> {
    let path = format!("/brains/{brain}/memories");
    let mut memories = Vec::new();
    let mut after: Option<i64> = None;
    loop {
      let mut query = vec![
        ("type", kind.to_owned()),
        ("status", "active".to_owned()),
        ("order", "validFrom".to_owned()),
        ("limit", PAGE.to_owned()),
      ];
      if let Some(after) = after {
        query.push(("after", after.to_string()));
      }
      let page: Paged<Memory> = call(self.http, &self.reach, Method::GET, &path, &query, None).await?;
      after = page.items.last().map(|memory| memory.id);
      let more = page.more && after.is_some();
      memories.extend(page.items);
      if !more {
        return Ok(memories);
      }
    }
  }

  /// A first version written.
  pub async fn write(&self, brain: i64, body: &Value) -> Result<Memory, CallFault> {
    call(
      self.http,
      &self.reach,
      Method::POST,
      &format!("/brains/{brain}/memories"),
      &[],
      Some(body),
    )
    .await
  }

  /// A new version written over a live memory.
  pub async fn version(&self, brain: i64, memory: i64, body: &Value) -> Result<Memory, CallFault> {
    let path = format!("/brains/{brain}/memories/{memory}/versions");
    call(self.http, &self.reach, Method::POST, &path, &[], Some(body)).await
  }

  /// A memory moved to `done`, with the reason.
  pub async fn done(
    &self,
    brain: i64,
    memory: i64,
    actor: &str,
    reason: &str,
  ) -> Result<Memory, CallFault> {
    let body = json!({ "status": "done", "actorName": actor, "reason": reason });
    let path = format!("/brains/{brain}/memories/{memory}");
    call(self.http, &self.reach, Method::PATCH, &path, &[], Some(&body)).await
  }

  /// The brain's event stream of one brain's memories, from after `after` or from now.
  pub async fn stream(&self, brain: i64, after: Option<i64>) -> Result<Response, CallFault> {
    let url = format!("{}/events", self.reach.address);
    let mut query =
      vec![("kinds", "memory.written,memory.moved".to_owned()), ("scope", format!("brain:{brain}"))];
    if let Some(after) = after {
      query.push(("after", after.to_string()));
    }
    let response = send(
      || {
        self
          .http
          .get(&url)
          .bearer_auth(&self.reach.token)
          .query(&query)
          .header("accept", "text/event-stream")
      },
      "brain",
    )
    .await?;
    if response.status() == StatusCode::OK {
      return Ok(response);
    }
    answered::<Value>(response, "brain").await?;
    Err(CallFault::Broken("the brain's stream answered no stream".into()))
  }
}

/// The receiver's owner: the owner as a sender, and the private room.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
  pub sender_id: i64,
  pub room: i64,
}

/// A user of the account service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
  pub name: Option<String>,
  pub senders: Vec<i64>,
}

/// A user id, which the account service may write as a number or a string.
fn key_of(value: &Value) -> Option<String> {
  match value {
    Value::String(text) if !text.is_empty() => Some(text.clone()),
    Value::Number(number) => Some(number.to_string()),
    _ => None,
  }
}

/// The receiver's owner.
pub async fn owner(http: &reqwest::Client, settings: &Settings) -> Result<Owner, CallFault> {
  call(http, &receiver(settings)?, Method::GET, "/owner", &[], None).await
}

/// What the receiver says of one room it holds: whether it is registered, and whose private room it is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
  #[serde(default)]
  pub registered: bool,
  #[serde(default)]
  pub private_of: Option<i64>,
}

impl Room {
  /// Whether this is the private room of the account under this key.
  pub fn is_private_to(&self, account: &str) -> bool {
    self.private_of.is_some_and(|private_of| private_of.to_string() == account)
  }
}

/// One room the receiver holds, where it holds one under this number.
pub async fn room(
  http: &reqwest::Client,
  settings: &Settings,
  room: i64,
) -> Result<Option<Room>, CallFault> {
  match call::<Room>(http, &receiver(settings)?, Method::GET, &format!("/rooms/{room}"), &[], None).await {
    Ok(room) => Ok(Some(room)),
    Err(fault) if fault.status() == Some(404) => Ok(None),
    Err(fault) => Err(fault),
  }
}

/// The numbers of the rooms the receiver lists for a console account, its private rooms among them.
pub async fn account_rooms(
  http: &reqwest::Client,
  settings: &Settings,
  account: &str,
) -> Result<Vec<i64>, CallFault> {
  #[derive(Deserialize)]
  struct Rooms {
    #[serde(default)]
    rooms: Vec<i64>,
  }
  let path = format!("/accounts/{account}/rooms");
  match call::<Rooms>(http, &receiver(settings)?, Method::GET, &path, &[], None).await {
    Ok(listed) => Ok(listed.rooms),
    Err(fault) if fault.status() == Some(404) => Ok(Vec::new()),
    Err(fault) => Err(fault),
  }
}

/// An instruction queued on the receiver.
pub async fn instruct(
  http: &reqwest::Client,
  settings: &Settings,
  body: &Value,
) -> Result<Value, CallFault> {
  call(http, &receiver(settings)?, Method::POST, "/instructions", &[], Some(body)).await
}

/// The account a sender is linked to, where one is.
pub async fn sender_user(
  http: &reqwest::Client,
  settings: &Settings,
  sender: i64,
) -> Result<Option<String>, CallFault> {
  let path = format!("/senders/{sender}/user");
  match call::<Value>(http, &accounts(settings)?, Method::GET, &path, &[], None).await {
    Ok(answer) => Ok(key_of(&answer["user"])),
    Err(fault) if fault.status() == Some(404) => Ok(None),
    Err(fault) => Err(fault),
  }
}

/// A user's name and senders, where the user stands.
pub async fn user(
  http: &reqwest::Client,
  settings: &Settings,
  user: &str,
) -> Result<Option<User>, CallFault> {
  let path = format!("/users/{user}");
  match call::<Value>(http, &accounts(settings)?, Method::GET, &path, &[], None).await {
    Ok(answer) => Ok(Some(User {
      name: answer["name"].as_str().map(str::to_owned),
      senders: answer["senders"]
        .as_array()
        .map(|all| all.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default(),
    })),
    Err(fault) if fault.status() == Some(404) => Ok(None),
    Err(fault) => Err(fault),
  }
}

/// The owner's account: the account the account service links to the receiver's owner sender.
pub async fn owner_account(
  http: &reqwest::Client,
  settings: &Settings,
) -> Result<Option<String>, CallFault> {
  let owner = owner(http, settings).await?;
  sender_user(http, settings, owner.sender_id).await
}

/// The account a tool session's holder acts for, as agency names its owner: a supervisor's `ownerAccount`, else
/// a role agent's; none where agency names none.
pub async fn holder_account(
  http: &reqwest::Client,
  settings: &Settings,
  holder: &str,
) -> Result<Option<String>, CallFault> {
  let reach = agency(settings)?;
  let path = format!("/supervisors/{}", segment(holder));
  match call::<Value>(http, &reach, Method::GET, &path, &[], None).await {
    Ok(supervisor) => return Ok(key_of(&supervisor["ownerAccount"])),
    Err(fault) if fault.status() == Some(404) => {}
    Err(fault) => return Err(fault),
  }
  let agents: Value = call(http, &reach, Method::GET, "/role-agents", &[], None).await?;
  let Some(items) = agents["items"].as_array() else {
    return Err(CallFault::Broken("agency's role agents do not read".into()));
  };
  Ok(
    items
      .iter()
      .find(|agent| agent["name"].as_str() == Some(holder))
      .and_then(|agent| key_of(&agent["ownerAccount"])),
  )
}

/// An account's node as agency's record names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonNode {
  pub node: String,
  /// Whether the administrator declared the node shared for the account.
  pub shared: bool,
}

/// The node agency's record names for an account, where one does.
pub async fn person_node(
  http: &reqwest::Client,
  settings: &Settings,
  account: &str,
) -> Result<Option<PersonNode>, CallFault> {
  let records: Value = call(http, &agency(settings)?, Method::GET, "/person-nodes", &[], None).await?;
  let Some(items) = records["items"].as_array() else {
    return Err(CallFault::Broken("agency's records of people's nodes do not read".into()));
  };
  Ok(
    items
      .iter()
      .find(|record| key_of(&record["ownerAccount"]).as_deref() == Some(account))
      .and_then(|record| {
        Some(PersonNode {
          node: record["node"].as_str().filter(|node| !node.is_empty())?.to_owned(),
          shared: record["shared"].as_bool().unwrap_or(false),
        })
      }),
  )
}

/// One command run on a person's own node as the calendar, for their account, answered with its end.
pub async fn node_run(
  http: &reqwest::Client,
  settings: &Settings,
  node_name: &str,
  body: &Value,
) -> Result<Value, CallFault> {
  let path = format!("/nodes/{}/runs", segment(node_name));
  call(http, &node(settings)?, Method::POST, &path, &[], Some(body)).await
}

/// A Microsoft Graph access token read from the account's own Teams page, holding one of the scopes named.
pub async fn page_token(
  http: &reqwest::Client,
  settings: &Settings,
  account: &str,
  scopes: &[&str],
) -> Result<String, CallFault> {
  let number: u64 = account.parse().map_err(|_| {
    CallFault::Broken(format!("account {account} is no number the Teams page service reads"))
  })?;
  let body = json!({ "account": number, "scopes": scopes });
  let answer: Value = call(http, &teams(settings)?, Method::POST, "/page-tokens", &[], Some(&body)).await?;
  answer["token"]
    .as_str()
    .filter(|token| !token.is_empty())
    .map(str::to_owned)
    .ok_or_else(|| CallFault::Broken("the Teams page service answered no token".into()))
}

/// A company as the account service names it: its unit id and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanyRef {
  pub id: String,
  pub name: String,
}

/// The company an account stands in, where it stands in one.
pub async fn company_of(
  http: &reqwest::Client,
  settings: &Settings,
  user: &str,
) -> Result<Option<CompanyRef>, CallFault> {
  let path = format!("/users/{}/unit", segment(user));
  match call::<Value>(http, &accounts(settings)?, Method::GET, &path, &[], None).await {
    Ok(answer) => Ok(key_of(&answer["company"]["id"]).map(|id| CompanyRef {
      name: answer["company"]["name"].as_str().map_or_else(|| id.clone(), str::to_owned),
      id,
    })),
    Err(fault) if fault.status() == Some(404) => Ok(None),
    Err(fault) => Err(fault),
  }
}

/// Every company the account service holds, asked as the administrator who calls.
pub async fn companies(
  http: &reqwest::Client,
  settings: &Settings,
  owner_key: &str,
) -> Result<Vec<CompanyRef>, CallFault> {
  let reach = accounts(settings)?;
  let url = format!("{}/units", reach.address);
  let response = send(
    || {
      http
        .get(&url)
        .bearer_auth(&reach.token)
        .header("Owner-Key", owner_key)
        .header("Owner-Role", "administrator")
    },
    reach.name,
  )
  .await?;
  let answer: Value = answered(response, reach.name).await?;
  Ok(
    answer["units"]
      .as_array()
      .map(Vec::as_slice)
      .unwrap_or_default()
      .iter()
      .filter(|unit| unit["parent"].is_null())
      .filter_map(|unit| {
        let id = key_of(&unit["id"])?;
        Some(CompanyRef {
          name: unit["name"].as_str().map_or_else(|| id.clone(), str::to_owned),
          id,
        })
      })
      .collect(),
  )
}

/// Whether an account heads a unit, or one above it. The account service takes the unit as a number and the
/// account key as a string.
pub async fn heads(
  http: &reqwest::Client,
  settings: &Settings,
  unit: &str,
  user: &str,
) -> Result<bool, CallFault> {
  let unit = unit.parse::<i64>().map_or_else(|_| json!(unit), |number| json!(number));
  let body = json!({ "unit": unit, "user": user });
  match call::<Value>(http, &accounts(settings)?, Method::POST, "/checks/unit", &[], Some(&body)).await {
    Ok(answer) => Ok(answer["allowed"].as_bool().unwrap_or(false)),
    Err(fault) if fault.status() == Some(404) => Ok(false),
    Err(fault) => Err(fault),
  }
}
