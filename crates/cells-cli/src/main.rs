//! `cells`: a command-line test host for the Cells runtime MVP.
//!
//! This is the simplest possible host (PROPOSAL.md §8.1): it reads an
//! application's manifest and a host config, decides the grants, and runs
//! the application in a cell.
//!
//! ```text
//! cells run <app-dir> [--host <host.json>] [--timeout <seconds>]
//! cells inspect <app-dir> [--host <host.json>]
//! ```
//!
//! `--host` defaults to `<app-dir>/host.json`.

mod host_config;
mod manifest;
mod policy;
mod providers;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use cells_runtime::CellConfig;
use cells_runtime::CellExit;
use cells_runtime::CellGrant;
use cells_runtime::CellId;
use cells_runtime::Engine;
use cells_runtime::GrantState;
use cells_runtime::GrantTable;

use host_config::HostConfig;
use manifest::Manifest;
use policy::AskOnTerminal;
use policy::Outcome;
use policy::PlannedGrant;

const USAGE: &str = "\
usage:
  cells run <app-dir> [--host <host.json>] [--timeout <seconds>]
  cells inspect <app-dir> [--host <host.json>]

--host defaults to <app-dir>/host.json.";

struct Args {
  command: String,
  app_dir: PathBuf,
  host: Option<PathBuf>,
  timeout: Option<Duration>,
}

fn parse_args() -> Result<Args, String> {
  let mut args = std::env::args().skip(1);
  let command = args.next().ok_or("missing command")?;
  if command == "--help" || command == "-h" {
    return Err(String::new());
  }
  let mut app_dir = None;
  let mut host = None;
  let mut timeout = None;
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--host" => host = Some(PathBuf::from(args.next().ok_or("--host needs a path")?)),
      "--timeout" => {
        let secs: f64 = args
          .next()
          .ok_or("--timeout needs a number of seconds")?
          .parse()
          .map_err(|_| "--timeout needs a number of seconds")?;
        timeout = Some(Duration::from_secs_f64(secs));
      }
      _ if arg.starts_with("--") => return Err(format!("unknown option {arg}")),
      _ if app_dir.is_none() => app_dir = Some(PathBuf::from(arg)),
      _ => return Err(format!("unexpected argument {arg}")),
    }
  }
  Ok(Args {
    command,
    app_dir: app_dir.ok_or("missing <app-dir>")?,
    host,
    timeout,
  })
}

fn main() -> ExitCode {
  let args = match parse_args() {
    Ok(args) => args,
    Err(message) => {
      if !message.is_empty() {
        eprintln!("error: {message}\n");
      }
      eprintln!("{USAGE}");
      return ExitCode::from(2);
    }
  };
  let result = match args.command.as_str() {
    "run" => run(&args),
    "inspect" => inspect(&args),
    other => Err(format!("unknown command {other:?}\n\n{USAGE}")),
  };
  match result {
    Ok(code) => code,
    Err(message) => {
      eprintln!("error: {message}");
      ExitCode::FAILURE
    }
  }
}

fn load(args: &Args) -> Result<(Manifest, HostConfig, Vec<PlannedGrant>), String> {
  let manifest = Manifest::load(&args.app_dir)?;
  let host_path = args
    .host
    .clone()
    .unwrap_or_else(|| args.app_dir.join("host.json"));
  let host = HostConfig::load(&host_path)?;
  let planned = policy::plan(&manifest, &host)?;
  Ok((manifest, host, planned))
}

/// `cells run`: decide grants, start the cell, and wait for it.
fn run(args: &Args) -> Result<ExitCode, String> {
  let (manifest, host, planned) = load(args)?;

  // The supervisor runtime: the dispatcher and providers run here, separate
  // from the cell's own thread (PROPOSAL.md §7.1).
  let supervisor = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(2)
    .enable_all()
    .build()
    .map_err(|e| format!("could not start the supervisor runtime: {e}"))?;

  let table = GrantTable::new();
  // Must happen on the main thread, before any cell starts.
  let engine = Engine::new(table.clone(), supervisor.handle().clone());

  let cell = CellId::next();
  let mut config = CellConfig::new(cell, &manifest.name, &args.app_dir, &manifest.entry);
  if let Some(mb) = host.heap_limit_mb {
    config.heap_limit = mb * 1024 * 1024;
  }
  for grant in planned {
    let state = match grant.outcome {
      Outcome::Grant(provider) => GrantState::Granted(provider),
      Outcome::Ask(provider) => GrantState::Unresolved(Arc::new(AskOnTerminal {
        app_name: manifest.name.clone(),
        provider,
      })),
      Outcome::Deny(_) => GrantState::Denied,
    };
    let handle = table.register(cell, &grant.name, grant.contract.clone(), state);
    config.grants.push(CellGrant {
      name: grant.name,
      contract: grant.contract,
      handle,
    });
  }

  eprintln!("[cells] starting {} {} as {cell}", manifest.name, manifest.version);
  let handle = engine.spawn(config).map_err(|e| e.to_string())?;

  // Ctrl-C and --timeout both terminate the cell.
  let terminator = handle.terminator();
  let timeout = args.timeout;
  supervisor.spawn(async move {
    let expired = async {
      match timeout {
        Some(t) => tokio::time::sleep(t).await,
        None => std::future::pending().await,
      }
    };
    tokio::select! {
      _ = tokio::signal::ctrl_c() => eprintln!("\n[cells] interrupted; terminating"),
      _ = expired => eprintln!("[cells] timeout reached; terminating"),
    }
    terminator.terminate();
  });

  let report = handle.join();
  table.remove_cell(cell);
  if report.dropped_notifications > 0 {
    eprintln!(
      "[cells] warning: {} console messages were dropped because the cell produced them too quickly",
      report.dropped_notifications
    );
  }
  eprintln!("[cells] {} {}", manifest.name, report.exit);
  Ok(match report.exit {
    CellExit::Completed => ExitCode::SUCCESS,
    _ => ExitCode::FAILURE,
  })
}

/// `cells inspect`: show the application's source, what it asks for, and
/// what this host would actually give it (Milestone 4).
fn inspect(args: &Args) -> Result<ExitCode, String> {
  let (manifest, _host, planned) = load(args)?;

  println!("Application  {} {}", manifest.name, manifest.version);
  println!("Entry point  {}", manifest.entry);
  println!();
  println!("Source files (importable as cell:///<path>)");
  let mut files = Vec::new();
  collect_sources(&args.app_dir, &args.app_dir, &mut files);
  files.sort();
  for file in files {
    println!("  {file}");
  }
  println!();
  println!("Capabilities");
  let width = planned.iter().map(|g| g.name.len()).max().unwrap_or(0);
  for grant in &planned {
    let requirement = if grant.required { "required" } else { "optional" };
    let (status, detail) = match &grant.outcome {
      Outcome::Grant(p) => ("granted", p.describe()),
      Outcome::Ask(p) => ("ask on first use", p.describe()),
      Outcome::Deny(reason) => ("denied", reason.clone()),
    };
    println!(
      "  {:width$}  {:10}  {requirement}  → {status}",
      grant.name,
      grant.contract.id(),
    );
    println!("  {:width$}  {}", "", detail);
  }
  println!();
  println!("Everything else (network, environment, processes, other files) is unavailable.");
  Ok(ExitCode::SUCCESS)
}

/// Lists module files under the application directory, skipping data
/// directories that providers write to.
fn collect_sources(root: &Path, dir: &Path, out: &mut Vec<String>) {
  let Ok(entries) = std::fs::read_dir(dir) else {
    return;
  };
  for entry in entries.flatten() {
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().into_owned();
    if path.is_dir() {
      if name != "data" && name != "node_modules" && !name.starts_with('.') {
        collect_sources(root, &path, out);
      }
      continue;
    }
    let is_module = ["ts", "mts", "js", "mjs"]
      .iter()
      .any(|ext| name.ends_with(&format!(".{ext}")))
      && !name.ends_with(".d.ts");
    if is_module {
      let relative = path.strip_prefix(root).unwrap_or(&path);
      out.push(relative.to_string_lossy().replace('\\', "/"));
    }
  }
}
