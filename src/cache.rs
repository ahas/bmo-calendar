//! What the calendar last read of the brain (`docs/design.md`, 「What a calendar and an event are in the
//! brain」): each realm's brain and what it holds live, by their own ids. It is rebuilt from the brain at start
//! and kept by the brain's event stream and the calendar's own writes.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{
  BOOKING_TYPE, CALENDAR_TYPE, EVENT_TYPE, Memory, RESOURCE_TYPE, SCHEDULE_TYPE, SOURCE_TYPE,
};
pub use crate::realm::RealmKey;

/// One realm's brain and what it holds live.
#[derive(Clone, Debug)]
pub struct Held {
  pub brain: i64,
  pub calendars: BTreeMap<String, Memory>,
  pub events: BTreeMap<String, Memory>,
  pub sources: BTreeMap<String, Memory>,
  /// By their own ids, the source's and the item's joined by a colon.
  pub schedules: BTreeMap<String, Memory>,
  pub resources: BTreeMap<String, Memory>,
  pub bookings: BTreeMap<String, Memory>,
  /// The ids of the memories seen closed, so a late read of them does not bring them back.
  closed: HashSet<i64>,
}

impl Held {
  pub fn new(brain: i64) -> Self {
    Self {
      brain,
      calendars: BTreeMap::new(),
      events: BTreeMap::new(),
      sources: BTreeMap::new(),
      schedules: BTreeMap::new(),
      resources: BTreeMap::new(),
      bookings: BTreeMap::new(),
      closed: HashSet::new(),
    }
  }
}

/// What changed in the cache, by its own id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Changed {
  Calendar(String),
  Event(String),
  Source(String),
  /// A schedule: its source's id and its item's.
  Schedule(String, String),
  Resource(String),
  /// A booking: its id and its booker.
  Booking(String, String),
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
      SOURCE_TYPE => (&mut held.sources, Changed::Source(id.clone())),
      SCHEDULE_TYPE => (
        &mut held.schedules,
        Changed::Schedule(memory.label("source").to_owned(), memory.label("item").to_owned()),
      ),
      RESOURCE_TYPE => (&mut held.resources, Changed::Resource(id.clone())),
      BOOKING_TYPE => (&mut held.bookings, Changed::Booking(id.clone(), memory.label("booker").to_owned())),
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
