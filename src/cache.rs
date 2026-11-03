//! What the calendar last read of the brain (`docs/design.md`, 「What a calendar and an event are in the
//! brain」): each realm's brain and its live calendars and events, by their own ids. It is rebuilt from the
//! brain at start and kept by the brain's event stream and the calendar's own writes.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{CALENDAR_TYPE, EVENT_TYPE, Memory};
use crate::wire::Realm;

/// A realm as a brain is keyed: the organization's, or one account's.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RealmKey {
  Mine(String),
  Organization,
}

impl RealmKey {
  /// The realm's word.
  pub fn realm(&self) -> Realm {
    match self {
      Self::Mine(_) => Realm::Mine,
      Self::Organization => Realm::Organization,
    }
  }

  /// The account whose realm it is, for a `mine` realm.
  pub fn account(&self) -> Option<&str> {
    match self {
      Self::Mine(account) => Some(account),
      Self::Organization => None,
    }
  }

  /// The brain's `accountKey`.
  pub fn account_key(&self) -> String {
    match self {
      Self::Mine(account) => format!("account:{account}"),
      Self::Organization => "organization".to_owned(),
    }
  }

  /// The realm a brain's `accountKey` names, where it names one.
  pub fn of_account_key(key: &str) -> Option<Self> {
    if key == "organization" {
      return Some(Self::Organization);
    }
    key
      .strip_prefix("account:")
      .filter(|account| !account.is_empty())
      .map(|account| Self::Mine(account.into()))
  }
}

/// One realm's brain and what it holds live.
#[derive(Clone, Debug)]
pub struct Held {
  pub brain: i64,
  pub calendars: BTreeMap<String, Memory>,
  pub events: BTreeMap<String, Memory>,
  /// The ids of the memories seen closed, so a late read of them does not bring them back.
  closed: HashSet<i64>,
}

impl Held {
  pub fn new(brain: i64) -> Self {
    Self {
      brain,
      calendars: BTreeMap::new(),
      events: BTreeMap::new(),
      closed: HashSet::new(),
    }
  }
}

/// What changed in the cache: a calendar or an event, by its own id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Changed {
  Calendar(String),
  Event(String),
}

/// Every realm held.
#[derive(Debug, Default)]
pub struct Cache {
  pub realms: HashMap<RealmKey, Held>,
}

impl Cache {
  /// The realm a brain is, where it is held.
  pub fn realm_of(&self, brain: i64) -> Option<RealmKey> {
    self.realms.iter().find(|(_, held)| held.brain == brain).map(|(realm, _)| realm.clone())
  }

  /// A memory read or written taken in: a live one stands at its own id unless a later one stands there, a
  /// closed one leaves where it stood. Answers what changed, where anything did.
  pub fn apply(&mut self, realm: &RealmKey, memory: &Memory) -> Option<Changed> {
    let held = self.realms.get_mut(realm)?;
    let id = memory.own_id()?.to_owned();
    let (map, changed) = match memory.kind.as_str() {
      CALENDAR_TYPE => (&mut held.calendars, Changed::Calendar(id.clone())),
      EVENT_TYPE => (&mut held.events, Changed::Event(id.clone())),
      _ => return None,
    };
    if memory.live() {
      if held.closed.contains(&memory.id) {
        return None;
      }
      if map.get(&id).is_some_and(|standing| standing.id >= memory.id) {
        return None;
      }
      map.insert(id, memory.clone());
      Some(changed)
    } else {
      held.closed.insert(memory.id);
      if map.get(&id).is_some_and(|standing| standing.id == memory.id) {
        map.remove(&id);
        return Some(changed);
      }
      None
    }
  }
}
