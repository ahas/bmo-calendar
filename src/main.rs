//! The `bmo-calendar` command: `serve` answers the routes until it is told to stop; `health` ends 0 only where
//! the service its settings name answers ready; `openapi` prints the OpenAPI document; `tool-env` writes the env
//! file the tool reads the service's address and token from.

use std::path::PathBuf;
use std::process::ExitCode;

use bmo_calendar::openapi::openapi_json;
use bmo_calendar::settings::Settings;
use bmo_calendar::tool_env::{Written, write_env};
use bmo_calendar::wire::Health;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bmo-calendar", version = bmo_calendar::VERSION, about = "The calendar service and its command.")]
struct Command {
  #[command(subcommand)]
  verb: Verb,
}

#[derive(Subcommand)]
enum Verb {
  /// Reads the brain, beats once a minute and answers the routes until the process is told to stop.
  Serve,
  /// Whether the service the settings name answers ready: exit code 0 where it does, 1 where not.
  Health,
  /// Prints the OpenAPI document.
  Openapi,
  /// Writes the env file the tool reads the service's address and token from, under the data root.
  ToolEnv {
    /// The data root; the settings' where none is given.
    #[arg(long, value_name = "dir")]
    data_root: Option<PathBuf>,
  },
}

async fn health(settings: &Settings) -> Result<(), String> {
  let base = settings.base_address();
  let response = reqwest::Client::builder()
    .no_proxy()
    .build()
    .map_err(|why| format!("no HTTP client: {why}"))?
    .get(format!("{base}/health"))
    .bearer_auth(&settings.server.token)
    .send()
    .await
    .map_err(|why| format!("{base}: {why}"))?;
  let status = response.status();
  let health: Health = response.json().await.map_err(|why| format!("{base} answered {status}: {why}"))?;
  if health.ready {
    Ok(())
  } else {
    Err(format!(
      "the service is not ready: {}",
      health.reason.as_deref().unwrap_or("it gave no reason")
    ))
  }
}

#[tokio::main]
async fn main() -> ExitCode {
  let Command { verb } = Command::parse();
  let ended = match verb {
    Verb::Openapi => {
      print!("{}", openapi_json());
      Ok(())
    }
    verb => match Settings::load() {
      Err(why) => Err(why),
      Ok(settings) => match verb {
        Verb::Serve => bmo_calendar::serve(settings).await,
        Verb::Health => health(&settings).await,
        Verb::ToolEnv { data_root } => {
          let root = data_root.unwrap_or_else(|| settings.data_root());
          write_env(&settings, &root).map(|written| match written {
            Written::Written(path) => println!("written {}", path.display()),
            Written::Unchanged(path) => println!("unchanged {}", path.display()),
          })
        }
        Verb::Openapi => Ok(()),
      },
    },
  };
  match ended {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("bmo-calendar: {why}");
      ExitCode::FAILURE
    }
  }
}
