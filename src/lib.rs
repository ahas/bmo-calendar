//! The calendar service: dated events kept as memories in the brain (the brain the master, this unit the
//! subordinate), their alarms told to the supervisor through the receiver, and the console face that shows
//! them (`docs/design.md`, `docs/api.md`).

pub mod alarms;
pub mod api;
pub mod app;
pub mod booking;
pub mod bookings;
pub mod cache;
pub mod companies;
pub mod console;
pub mod error;
pub mod events;
pub mod model;
pub mod openapi;
pub mod peers;
pub mod realm;
pub mod schedules;
pub mod settings;
pub mod store;
pub mod tool_env;
pub mod wire;

use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;

use crate::api::{Served, router};
use crate::app::App;
use crate::settings::Settings;
use crate::store::Store;

/// How often the beat runs.
pub const BEAT: Duration = Duration::from_secs(60);

/// Opens the store, reads the brain, beats once a minute, and answers the routes on the settings' address until
/// the process is told to stop, the open event streams ended at once.
pub async fn serve(settings: Settings) -> Result<(), String> {
  let listener = TcpListener::bind(settings.server.listen)
    .await
    .map_err(|why| format!("{} could not be listened on: {why}", settings.server.listen))?;
  serve_on(settings, listener, stopped()).await
}

/// The service on a listener already bound, until `stop` resolves.
pub async fn serve_on(
  settings: Settings,
  listener: TcpListener,
  stop: impl Future<Output = ()> + Send + 'static,
) -> Result<(), String> {
  let store = Store::open(&settings.store.path)?;
  // An empty console credential reaches nothing, so no face stands to be reached.
  let face = if settings.console.token.is_empty() {
    eprintln!("bmo-calendar: the console face is not served: no console credential is set");
    None
  } else {
    match console::face(&settings.console.folder, &settings.console.token) {
      Ok(face) => Some(face),
      Err(why) => {
        eprintln!("bmo-calendar: the console face is not served: {why}");
        None
      }
    }
  };
  let app = App::new(settings, store);
  let reading = Arc::clone(&app);
  let beating = tokio::spawn(async move {
    reading.read_brain().await;
    let mut beat = tokio::time::interval(BEAT);
    loop {
      beat.tick().await;
      if let Err(why) = alarms::beat(&reading).await {
        eprintln!("bmo-calendar: the beat failed: {}", why.error);
      }
    }
  });
  let stopping = Arc::clone(&app);
  let served = axum::serve(listener, router(Served { app, face }))
    .with_graceful_shutdown(async move {
      stop.await;
      stopping.events.stop();
    })
    .await
    .map_err(|why| why.to_string());
  beating.abort();
  served
}

/// Resolves once the process is told to stop: an interrupt, or the service manager's terminate.
async fn stopped() {
  let interrupt = async {
    let _ = tokio::signal::ctrl_c().await;
  };
  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      Ok(mut signal) => {
        signal.recv().await;
      }
      Err(_) => std::future::pending().await,
    }
  };
  tokio::select! {
    () = interrupt => {}
    () = terminate => {}
  }
}
