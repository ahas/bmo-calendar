//! The service as it runs: its settings, the cache of each realm's brain kept by the brain's event stream, the
//! told marks, the events it publishes, and whether it has read the brain yet.

use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use serde_json::Value;

use crate::cache::{Cache, Changed, Held, RealmKey};
use crate::error::ApiError;
use crate::events::{Events, Scope};
use crate::model::Memory;
use crate::peers::{self, AGAIN, BrainCalls, BrainRow, CallFault};
use crate::settings::Settings;
use crate::store::Store;
use crate::wire::{
  BookingChanged, CalendarChanged, EventChanged, ResourceChanged, ScheduleChanged, SourceChanged,
};

/// The service's state, shared by every route and task.
pub struct App {
  pub settings: Settings,
  pub http: reqwest::Client,
  pub cache: Mutex<Cache>,
  pub store: Store,
  pub events: Events,
  /// Null once the brain has been read; why not before.
  pub unready: Mutex<Option<String>>,
  /// One realm found or made at a time, so two calls never make one brain twice.
  pub realm_lock: tokio::sync::Mutex<()>,
  /// One beat at a time.
  pub beat_lock: tokio::sync::Mutex<()>,
  /// One write of sources and schedules at a time, so two sets of a source never interleave.
  pub schedule_lock: tokio::sync::Mutex<()>,
  /// The one writer of bookings: each write is checked against the cache and written to the brain before the
  /// next is checked, so two bookings that meet never both stand.
  pub booking_lock: tokio::sync::Mutex<()>,
}

impl App {
  pub fn new(settings: Settings, store: Store) -> Arc<Self> {
    Arc::new(Self {
      settings,
      http: peers::http(),
      cache: Mutex::new(Cache::default()),
      store,
      events: Events::new(),
      unready: Mutex::new(Some("the brain has not been read yet".to_owned())),
      realm_lock: tokio::sync::Mutex::new(()),
      beat_lock: tokio::sync::Mutex::new(()),
      schedule_lock: tokio::sync::Mutex::new(()),
      booking_lock: tokio::sync::Mutex::new(()),
    })
  }

  /// The brain's routes, as the settings reach it now.
  pub fn brain(&self) -> Result<BrainCalls<'_>, CallFault> {
    Ok(BrainCalls {
      http: &self.http,
      reach: peers::brain(&self.settings)?,
    })
  }

  /// Why the service is not ready, or none.
  pub fn unready(&self) -> Option<String> {
    self.unready.lock().ok().and_then(|reason| reason.clone())
  }

  fn set_unready(&self, reason: Option<String>) {
    if let Ok(mut unready) = self.unready.lock() {
      *unready = reason;
    }
  }

  /// Reads every realm's brain until it has read them all once and moved the organization's calendars and
  /// events into their companies, asking again two seconds after a failure.
  pub async fn read_brain(self: &Arc<Self>) {
    loop {
      let done = match self.read_all().await {
        Ok(()) => crate::companies::migrate(self).await.map_err(|why| why.error),
        Err(fault) => Err(fault.to_string()),
      };
      match done {
        Ok(()) => {
          self.set_unready(None);
          return;
        }
        Err(why) => {
          eprintln!("bmo-calendar: the brain could not be read: {why}");
          self.set_unready(Some(format!("the brain could not be read: {why}")));
          tokio::time::sleep(AGAIN).await;
        }
      }
    }
  }

  async fn read_all(self: &Arc<Self>) -> Result<(), CallFault> {
    let brains = self.brain()?.calendar_brains().await?;
    let _held = self.realm_lock.lock().await;
    for brain in brains {
      let Some(realm) = RealmKey::of_account_key(&brain.account_key) else {
        continue;
      };
      if self.held(&realm).is_none() {
        self.hold(realm, &brain).await?;
      }
    }
    Ok(())
  }

  /// The brain a realm is held in, where it is held.
  pub fn held(&self, realm: &RealmKey) -> Option<i64> {
    self.cache.lock().ok()?.realms.get(realm).map(|held| held.brain)
  }

  /// A realm's brain: the one held, else the one the brain service holds, else, where `make` is set, one made.
  /// Answers none where the realm has no brain and none is to be made; a call with no company has none, and
  /// is refused `no-company` where one would be made.
  pub async fn realm(self: &Arc<Self>, realm: &RealmKey, make: bool) -> Result<Option<i64>, ApiError> {
    if let Some(reason) = self.unready() {
      return Err(ApiError::not_ready(reason));
    }
    self.realm_brain(realm, make).await
  }

  /// [`App::realm`], whether or not the service is ready: the move into companies reads and makes brains
  /// before it is.
  pub async fn realm_brain(
    self: &Arc<Self>,
    realm: &RealmKey,
    make: bool,
  ) -> Result<Option<i64>, ApiError> {
    let Some(account_key) = realm.account_key() else {
      return if make { Err(ApiError::no_company()) } else { Ok(None) };
    };
    if let Some(brain) = self.held(realm) {
      return Ok(Some(brain));
    }
    let _held = self.realm_lock.lock().await;
    if let Some(brain) = self.held(realm) {
      return Ok(Some(brain));
    }
    let calls = self.brain()?;
    let found = if make {
      Some(calls.find_or_make(&account_key).await?)
    } else {
      calls.find(&account_key).await?
    };
    match found {
      Some(brain) => {
        self.hold(realm.clone(), &brain).await?;
        Ok(Some(brain.id))
      }
      None => Ok(None),
    }
  }

  /// A brain held for its realm: its stream opened, then its memories read, then the stream followed.
  async fn hold(self: &Arc<Self>, realm: RealmKey, brain: &BrainRow) -> Result<(), CallFault> {
    let stream = self.brain()?.stream(brain.id, None).await?;
    if let Ok(mut cache) = self.cache.lock() {
      cache.realms.insert(realm.clone(), Held::new(brain.id));
    }
    if let Err(fault) = self.read_realm(&realm, brain.id).await {
      if let Ok(mut cache) = self.cache.lock() {
        cache.realms.remove(&realm);
      }
      return Err(fault);
    }
    let app = Arc::clone(self);
    let id = brain.id;
    tokio::spawn(async move { app.follow(realm, id, stream).await });
    Ok(())
  }

  /// A realm's live memories of every type it holds read whole and taken in.
  async fn read_realm(&self, realm: &RealmKey, brain: i64) -> Result<(), CallFault> {
    let calls = self.brain()?;
    let mut memories = Vec::new();
    for kind in realm.kinds() {
      memories.extend(calls.memories(brain, kind).await?);
    }
    for memory in &memories {
      self.take_in(realm, memory);
    }
    Ok(())
  }

  /// A memory taken into the cache, and what changed published to those who read it.
  pub fn take_in(&self, realm: &RealmKey, memory: &Memory) {
    let changed = match self.cache.lock() {
      Ok(mut cache) => cache.apply(realm, memory),
      Err(_) => None,
    };
    let scope = scope_of(realm);
    let company = realm.company();
    let company_word = company.clone().unwrap_or_default();
    match changed {
      Some(Changed::Calendar(calendar_id)) => self.events.publish(
        "calendar.changed",
        CalendarChanged {
          realm: realm.realm(),
          company,
          calendar_id,
        },
        scope,
      ),
      Some(Changed::Event(event_id)) => self.events.publish(
        "event.changed",
        EventChanged {
          realm: realm.realm(),
          company,
          event_id,
          occurrence: None,
        },
        scope,
      ),
      Some(Changed::Source(source_id)) => {
        self.events.publish("source.changed", SourceChanged { source_id }, Scope::Everyone)
      }
      Some(Changed::Schedule(source_id, item_id)) => self.events.publish(
        "schedule.changed",
        ScheduleChanged {
          company: company_word,
          source_id,
          item_id,
        },
        scope,
      ),
      Some(Changed::Resource(resource_id)) => self.events.publish(
        "resource.changed",
        ResourceChanged {
          company: company_word,
          resource_id,
        },
        scope,
      ),
      Some(Changed::Booking(booking_id, booker)) => self.events.publish(
        "booking.changed",
        BookingChanged {
          company: company_word,
          booking_id,
          booker,
        },
        scope,
      ),
      None => {}
    }
  }

  /// The brain's stream of one brain followed for the life of the service: each memory written or moved taken
  /// in; a broken stream opened again from its last event two seconds later; a resume the brain no longer
  /// holds read whole again.
  async fn follow(self: Arc<Self>, realm: RealmKey, brain: i64, first: reqwest::Response) {
    let mut response = Some(first);
    let mut last: Option<i64> = None;
    let stopped = self.events.stopped();
    tokio::pin!(stopped);
    loop {
      if let Some(open) = response.take() {
        tokio::select! {
          () = &mut stopped => return,
          () = self.read_stream(&realm, open, &mut last) => {}
        }
      }
      tokio::select! {
        () = &mut stopped => return,
        () = tokio::time::sleep(AGAIN) => {}
      }
      let opened = match self.brain() {
        Ok(calls) => calls.stream(brain, last).await,
        Err(fault) => Err(fault),
      };
      match opened {
        Ok(open) => {
          if last.is_none() && self.read_realm(&realm, brain).await.is_err() {
            continue;
          }
          response = Some(open);
        }
        Err(fault) if fault.status() == Some(410) => {
          last = None;
          if let Ok(calls) = self.brain()
            && let Ok(open) = calls.stream(brain, None).await
            && self.read_realm(&realm, brain).await.is_ok()
          {
            response = Some(open);
          }
        }
        Err(fault) => eprintln!("bmo-calendar: the brain's stream of brain {brain}: {fault}"),
      }
    }
  }

  /// One open stream read to its end.
  async fn read_stream(&self, realm: &RealmKey, response: reqwest::Response, last: &mut Option<i64>) {
    let mut bytes = response.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = bytes.next().await {
      let Ok(chunk) = chunk else {
        return;
      };
      buffer.push_str(&String::from_utf8_lossy(&chunk).replace('\r', ""));
      while let Some(end) = buffer.find("\n\n") {
        let frame: String = buffer.drain(..end + 2).collect();
        let mut data = String::new();
        for line in frame.lines() {
          if let Some(value) = line.strip_prefix("data:") {
            data.push_str(value.trim_start());
          } else if let Some(value) = line.strip_prefix("id:")
            && let Ok(id) = value.trim().parse()
          {
            *last = Some(id);
          }
        }
        if data.is_empty() {
          continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(&data) else {
          continue;
        };
        if let Some(id) = event["id"].as_i64() {
          *last = Some(id);
        }
        if let Ok(memory) = serde_json::from_value::<Memory>(event["data"].clone()) {
          self.take_in(realm, &memory);
        }
      }
    }
  }
}

/// Who a realm's events are given to: an account's own, or a company's readers.
pub fn scope_of(realm: &RealmKey) -> Scope {
  match realm {
    RealmKey::Mine(account) => Scope::Account(account.clone()),
    _ => Scope::Company(realm.company().unwrap_or_default()),
  }
}
