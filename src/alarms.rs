//! The alarms (`docs/design.md`, 「Alarms」; `docs/api.md`, 「Alarms」): the moments of each event's alarm, and
//! the beat that tells the due ones through the receiver once each, keeping the told marks.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Duration, NaiveDateTime};
use serde_json::json;

use crate::app::App;
use crate::cache::RealmKey;
use crate::error::ApiError;
use crate::model::{
  Memory, epoch_of, event_of, local_of, local_text, now_ms, parse_local, skipped, starts_between,
};
use crate::peers;
use crate::wire::{AlarmTold, Beat, Event, Realm, Tell, ToldAlarm};

/// How late an alarm is before it is named late: past the minute a beat stands for.
const LATE_FROM_MS: i64 = 60_000;

/// One occurrence's alarm moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moment {
  pub occurrence: NaiveDateTime,
  pub start: NaiveDateTime,
  /// Epoch ms.
  pub at: i64,
}

/// The alarm moments of an event whose occurrences start in `[low, high)` (epoch ms of the start).
fn moments_by_start(memory: &Memory, low: i64, high: i64) -> Vec<Moment> {
  let Some(event) = event_of(memory, Realm::Mine, None) else {
    return Vec::new();
  };
  let (Some(minutes), Some(start)) = (event.alarm.minutes(), parse_local(&event.start)) else {
    return Vec::new();
  };
  let skip = skipped(memory);
  let low_local = local_of(low) - Duration::hours(3);
  let high_local = local_of(high) + Duration::hours(3);
  starts_between(start, event.repeat, &skip, low_local, high_local)
    .into_iter()
    .map(|occurrence| Moment {
      occurrence,
      start: occurrence,
      at: epoch_of(occurrence) - minutes * 60_000,
    })
    .filter(|moment| {
      let start_ms = moment.at + minutes * 60_000;
      start_ms >= low && start_ms < high
    })
    .collect()
}

/// The alarm moments of an event in `[from, to)` (epoch ms of the moment).
pub fn moments(memory: &Memory, from: i64, to: i64) -> Vec<Moment> {
  let Some(event) = event_of(memory, Realm::Mine, None) else {
    return Vec::new();
  };
  let Some(minutes) = event.alarm.minutes() else {
    return Vec::new();
  };
  let ahead = minutes * 60_000;
  moments_by_start(memory, from + ahead, to + ahead)
    .into_iter()
    .filter(|m| m.at >= from && m.at < to)
    .collect()
}

/// One alarm come due: its realm, its brain, its event and its moment.
struct Due {
  realm: RealmKey,
  brain: i64,
  event: Event,
  moment: Moment,
  late_ms: i64,
  tell: Tell,
  calendar_name: String,
}

/// One beat: every alarm due since the last beat, or written after its moment and not started yet, told once.
pub async fn beat(app: &Arc<App>) -> Result<Beat, ApiError> {
  let _one = app.beat_lock.lock().await;
  let now = now_ms();
  let since = app.store.last_beat().map_err(ApiError::failed)?.unwrap_or(now - LATE_FROM_MS);
  let mut due = Vec::new();
  {
    let cache = app.cache.lock().map_err(|_| ApiError::failed("the cache's lock is poisoned"))?;
    for (realm, held) in &cache.realms {
      for memory in held.events.values() {
        let Some(event) = event_of(memory, realm.realm(), None) else {
          continue;
        };
        let Some(minutes) = event.alarm.minutes() else {
          continue;
        };
        let calendar = held.calendars.get(&event.calendar_id);
        let tell = match realm {
          RealmKey::Mine(_) => Tell::Creator,
          RealmKey::Organization => calendar
            .and_then(|calendar| crate::model::calendar_of(calendar, Realm::Organization, None))
            .and_then(|calendar| calendar.tell)
            .unwrap_or(Tell::Creator),
        };
        let calendar_name = calendar.map(|calendar| calendar.text.clone()).unwrap_or_default();
        let low = since.min(now) - minutes * 60_000 - 1;
        for moment in moments(memory, low.min(now - minutes * 60_000), now + 1) {
          let start_ms = moment.at + minutes * 60_000;
          let after_last_beat = moment.at > since;
          let written_after = event.changed_at > moment.at && start_ms > now;
          if moment.at > now || !(after_last_beat || written_after) {
            continue;
          }
          let late = now - moment.at;
          due.push(Due {
            realm: realm.clone(),
            brain: held.brain,
            event: event.clone(),
            late_ms: if after_last_beat && late >= LATE_FROM_MS { late } else { 0 },
            moment,
            tell,
            calendar_name: calendar_name.clone(),
          });
        }
      }
    }
  }
  let mut told = Vec::new();
  // An alarm not told keeps the next beat looking back to it, so a failed telling is asked again.
  let mut untold: Option<i64> = None;
  for one in due {
    let occurrence = local_text(one.moment.occurrence);
    if app.store.is_told(one.brain, &one.event.id, &occurrence).map_err(ApiError::failed)? {
      continue;
    }
    match tell(app, &one, &occurrence).await {
      Ok(rooms) => {
        app
          .store
          .mark_told(one.brain, &one.event.id, &occurrence, one.moment.at, now)
          .map_err(ApiError::failed)?;
        app.events.publish(
          "alarm.told",
          AlarmTold {
            realm: one.realm.realm(),
            event_id: one.event.id.clone(),
            occurrence: occurrence.clone(),
            at: one.moment.at,
          },
          one.realm.account().map(str::to_owned),
        );
        told.push(ToldAlarm {
          realm: one.realm.realm(),
          event_id: one.event.id.clone(),
          occurrence,
          at: one.moment.at,
          late_ms: one.late_ms,
          rooms,
        });
      }
      Err(why) => {
        eprintln!("bmo-calendar: the alarm of event {} could not be told: {why}", one.event.id);
        untold = Some(untold.map_or(one.moment.at, |at| at.min(one.moment.at)));
      }
    }
  }
  let looked_to = untold.map_or(now, |at| (at - 1).min(now)).max(since.min(now));
  app.store.set_last_beat(looked_to).map_err(ApiError::failed)?;
  Ok(Beat { told })
}

/// Whom one alarm is told: the accounts, by key.
fn recipients(app: &App, one: &Due) -> Vec<String> {
  match &one.realm {
    RealmKey::Mine(account) => vec![account.clone()],
    RealmKey::Organization => match one.tell {
      Tell::Nobody => Vec::new(),
      Tell::Creator => vec![one.event.creator.clone()],
      Tell::Everyone => {
        let mut accounts = vec![String::new()];
        if let Ok(cache) = app.cache.lock() {
          accounts.extend(cache.realms.keys().filter_map(|realm| realm.account().map(str::to_owned)));
        }
        accounts
      }
    },
  }
}

/// Whom a room is told an alarm for: an account, its name where known, and whether it has a room of its own.
struct Listener {
  account: String,
  name: Option<String>,
  owner: bool,
  own_room: bool,
}

/// One alarm told in every room it reaches; answers the rooms.
async fn tell(app: &App, one: &Due, occurrence: &str) -> Result<Vec<i64>, String> {
  let accounts = recipients(app, one);
  if accounts.is_empty() {
    return Ok(Vec::new());
  }
  let owner = peers::owner(&app.http, &app.settings).await.map_err(|why| why.to_string())?;
  let owner_account = peers::sender_user(&app.http, &app.settings, owner.sender_id).await.ok().flatten();
  let mut rooms: BTreeMap<i64, Vec<Listener>> = BTreeMap::new();
  for account in accounts {
    let is_owner = account.is_empty() || owner_account.as_deref() == Some(account.as_str());
    if is_owner {
      let listeners = rooms.entry(owner.room).or_default();
      if !listeners.iter().any(|listener| listener.owner) {
        listeners.push(Listener {
          account,
          name: None,
          owner: true,
          own_room: true,
        });
      }
      continue;
    }
    let user = peers::user(&app.http, &app.settings, &account).await.ok().flatten();
    let mut room = None;
    for sender in user.as_ref().map(|user| user.senders.clone()).unwrap_or_default() {
      if sender != owner.room
        && peers::registered_room(&app.http, &app.settings, sender).await.unwrap_or(false)
      {
        room = Some(sender);
        break;
      }
    }
    let listener = Listener {
      account,
      name: user.and_then(|user| user.name),
      owner: false,
      own_room: room.is_some(),
    };
    let listeners = rooms.entry(room.unwrap_or(owner.room)).or_default();
    if !listeners.iter().any(|standing| standing.account == listener.account) {
      listeners.push(listener);
    }
  }
  let mut told = Vec::new();
  for (room, listeners) in rooms {
    let body = json!({
      "agentName": null,
      "room": room,
      "said": { "fact": null, "order": order(one, &listeners) },
      "sort": "calendar",
      "summary": one.event.title,
      "kind": format!("calendar:{}:{occurrence}", one.event.id),
      "cause": "reply",
      "events": [],
      "agentOnly": false,
      "ridesUntil": null,
      "via": null,
      "run": null,
    });
    peers::instruct(&app.http, &app.settings, &body).await.map_err(|why| why.to_string())?;
    told.push(room);
  }
  Ok(told)
}

/// The order a room's supervisor is handed, in English: a marker naming the alarm, the event, its start, its
/// calendar and realm, how late it is, and whom it is for. What to do with a quiet instruction stands in the
/// receiving session's own rules, never in the turn.
fn order(one: &Due, listeners: &[Listener]) -> String {
  let event = &one.event;
  let start = local_text(one.moment.start).replace('T', " ");
  let when = if event.all_day { format!("on {}", &start[..10]) } else { format!("at {start}") };
  let realm = match one.realm {
    RealmKey::Mine(_) => "a personal calendar",
    RealmKey::Organization => "the organization's calendar",
  };
  let calendar = if one.calendar_name.is_empty() {
    String::new()
  } else {
    format!(" \"{}\"", one.calendar_name)
  };
  let mut text = format!(
    "CALENDAR ALARM. \"{}\" starts {when} (local time), in{calendar}, {realm}. It was asked to be reminded of.",
    event.title
  );
  if one.late_ms > 0 {
    text.push_str(&format!(
      " This reminder is {} minutes late: the calendar was not running when it came due.",
      one.late_ms / 60_000
    ));
  }
  for listener in listeners {
    let who = match (&listener.name, listener.owner) {
      (_, true) => "the owner".to_owned(),
      (Some(name), false) => format!("{name} (account {})", listener.account),
      (None, false) => format!("the member with account {}", listener.account),
    };
    if listener.owner || listener.own_room {
      text.push_str(&format!(" It is for {who}."));
    } else {
      text.push_str(&format!(" It is for {who}, who has no room of their own here."));
    }
  }
  text
}
