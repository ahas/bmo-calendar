//! The unit's own store (`docs/design.md`, 「Alarms」): the told marks, per realm, and the moment of the last
//! beat. Nothing else is kept here; every calendar and event is the brain's.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS told (
  brain_id INTEGER NOT NULL,
  event_id TEXT NOT NULL,
  occurrence TEXT NOT NULL,
  at INTEGER NOT NULL,
  told_at INTEGER NOT NULL,
  PRIMARY KEY (brain_id, event_id, occurrence)
);
CREATE TABLE IF NOT EXISTS beats (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  at INTEGER NOT NULL
);
";

/// The store, one SQLite file.
#[derive(Clone)]
pub struct Store {
  connection: Arc<Mutex<Connection>>,
}

impl Store {
  /// The store at `path`, its folder and its tables made where they do not stand.
  pub fn open(path: &Path) -> Result<Self, String> {
    if let Some(folder) = path.parent().filter(|folder| !folder.as_os_str().is_empty()) {
      std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    }
    let connection = Connection::open(path).map_err(|why| format!("{}: {why}", path.display()))?;
    connection.execute_batch(SCHEMA).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(Self {
      connection: Arc::new(Mutex::new(connection)),
    })
  }

  fn with<T>(&self, work: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T, String> {
    let connection = self.connection.lock().map_err(|_| "the store's lock is poisoned".to_owned())?;
    work(&connection).map_err(|why| format!("the store: {why}"))
  }

  /// Whether an occurrence's alarm has been told.
  pub fn is_told(&self, brain: i64, event: &str, occurrence: &str) -> Result<bool, String> {
    self.with(|connection| {
      connection
        .query_row(
          "SELECT 1 FROM told WHERE brain_id = ?1 AND event_id = ?2 AND occurrence = ?3",
          params![brain, event, occurrence],
          |_| Ok(()),
        )
        .optional()
        .map(|found| found.is_some())
    })
  }

  /// The mark that an occurrence's alarm, due at `at`, was told at `told_at`.
  pub fn mark_told(
    &self,
    brain: i64,
    event: &str,
    occurrence: &str,
    at: i64,
    told_at: i64,
  ) -> Result<(), String> {
    self.with(|connection| {
      connection
        .execute(
          "INSERT OR IGNORE INTO told (brain_id, event_id, occurrence, at, told_at) VALUES (?1, ?2, ?3, ?4, ?5)",
          params![brain, event, occurrence, at, told_at],
        )
        .map(drop)
    })
  }

  /// The occurrence of an event last told, where one was.
  pub fn last_told(&self, brain: i64, event: &str) -> Result<Option<String>, String> {
    self.with(|connection| {
      connection
        .query_row(
          "SELECT occurrence FROM told WHERE brain_id = ?1 AND event_id = ?2 ORDER BY told_at DESC, occurrence DESC \
           LIMIT 1",
          params![brain, event],
          |row| row.get(0),
        )
        .optional()
    })
  }

  /// Every told mark of an event kept for one brain kept for another as well, as the event moves there.
  pub fn copy_told(&self, from: i64, to: i64, event: &str) -> Result<(), String> {
    self.with(|connection| {
      connection
        .execute(
          "INSERT OR IGNORE INTO told (brain_id, event_id, occurrence, at, told_at) \
           SELECT ?2, event_id, occurrence, at, told_at FROM told WHERE brain_id = ?1 AND event_id = ?3",
          params![from, to, event],
        )
        .map(drop)
    })
  }

  /// The moment of the last beat, where one ran.
  pub fn last_beat(&self) -> Result<Option<i64>, String> {
    self.with(|connection| {
      connection
        .query_row("SELECT at FROM beats WHERE id = 1", [], |row| row.get(0))
        .optional()
    })
  }

  /// The moment of the beat just run.
  pub fn set_last_beat(&self, at: i64) -> Result<(), String> {
    self.with(|connection| {
      connection
        .execute(
          "INSERT INTO beats (id, at) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET at = ?1",
          params![at],
        )
        .map(drop)
    })
  }
}
