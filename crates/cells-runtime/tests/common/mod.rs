//! Shared helpers for runtime integration tests.

#![allow(dead_code, reason = "not every test uses every helper")]

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use cells_runtime::CallFuture;
use cells_runtime::CapabilityError;
use cells_runtime::CellConfig;
use cells_runtime::CellGrant;
use cells_runtime::CellHandle;
use cells_runtime::CellId;
use cells_runtime::CellReport;
use cells_runtime::Contract;
use cells_runtime::Engine;
use cells_runtime::GrantState;
use cells_runtime::GrantTable;
use cells_runtime::Provider;
use cells_runtime::Value;
use cells_runtime::standard;

/// The shared supervisor runtime for all tests in one binary.
pub fn supervisor() -> &'static tokio::runtime::Runtime {
  static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
  RT.get_or_init(|| {
    tokio::runtime::Builder::new_multi_thread()
      .worker_threads(2)
      .enable_all()
      .build()
      .unwrap()
  })
}

/// Captures console output.
#[derive(Default)]
pub struct Recorder {
  pub lines: Mutex<Vec<String>>,
}

impl Provider for Recorder {
  fn describe(&self) -> String {
    "recorder".into()
  }
  fn invoke(&self, _method: &str, _args: Vec<Value>) -> CallFuture {
    Box::pin(async { Err(CapabilityError::invalid_argument("no such method")) })
  }
  fn notify(&self, _method: &str, args: Vec<Value>) {
    // console@1 sends write(level, text).
    let level = args.first().and_then(Value::as_str).unwrap_or("?");
    let text = args.get(1).and_then(Value::as_str).unwrap_or("");
    let line = if level == "log" {
      text.to_string()
    } else {
      format!("{level}: {text}")
    };
    self.lines.lock().unwrap().push(line);
  }
}

/// Answers `echo@1.say(...)` with its arguments.
pub struct Echo;

impl Provider for Echo {
  fn describe(&self) -> String {
    "echo".into()
  }
  fn invoke(&self, method: &str, args: Vec<Value>) -> CallFuture {
    let method = method.to_string();
    Box::pin(async move { Ok(serde_json::json!({ "method": method, "args": args })) })
  }
}

pub fn echo_contract() -> Contract {
  Contract::new("echo@1").unwrap().invoke("say")
}

/// A temporary application directory plus the pieces needed to run it.
pub struct Harness {
  pub dir: tempfile::TempDir,
  pub table: Arc<GrantTable>,
  pub engine: Engine,
  pub console: Arc<Recorder>,
  pub cell: CellId,
  pub grants: Vec<CellGrant>,
  pub heap_limit: Option<usize>,
}

impl Harness {
  /// A harness whose cell has a `console` grant.
  pub fn new() -> Self {
    let mut harness = Self::without_console();
    let console = harness.console.clone();
    harness.grant("console", standard::console(), GrantState::Granted(console));
    harness
  }

  /// A harness whose cell has no grants at all.
  pub fn without_console() -> Self {
    let table = GrantTable::new();
    let engine = Engine::new(table.clone(), supervisor().handle().clone());
    Self {
      dir: tempfile::tempdir().unwrap(),
      table,
      engine,
      console: Arc::new(Recorder::default()),
      cell: CellId::next(),
      grants: Vec::new(),
      heap_limit: None,
    }
  }

  pub fn file(&self, path: &str, contents: &str) -> &Self {
    let full = self.dir.path().join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, contents).unwrap();
    self
  }

  pub fn grant(&mut self, name: &str, contract: Contract, state: GrantState) -> cells_runtime::GrantHandle {
    let handle = self.table.register(self.cell, name, contract.clone(), state);
    self.grants.push(CellGrant {
      name: name.into(),
      contract,
      handle,
    });
    handle
  }

  pub fn config(&self) -> CellConfig {
    let mut config = CellConfig::new(self.cell, "test", self.dir.path(), "main.ts");
    config.grants = self.grants.clone();
    if let Some(limit) = self.heap_limit {
      config.heap_limit = limit;
    }
    config
  }

  pub fn spawn(&self) -> CellHandle {
    self.engine.spawn(self.config()).expect("cell starts")
  }

  /// Runs `main.ts` to completion.
  pub fn run(&self) -> CellReport {
    self.spawn().join()
  }

  /// Runs `main.ts` and returns its console output.
  pub fn run_ok(&self) -> Vec<String> {
    let report = self.run();
    assert_eq!(report.exit, cells_runtime::CellExit::Completed, "output: {:?}", self.output());
    self.output()
  }

  pub fn output(&self) -> Vec<String> {
    self.console.lines.lock().unwrap().clone()
  }

  pub fn path(&self) -> &Path {
    self.dir.path()
  }
}
