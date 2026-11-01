//! Creating, running, and terminating cells.

use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Once;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use deno_core::v8;
use tokio::sync::Notify;
use tokio::sync::mpsc;

use crate::bootstrap;
use crate::contract::Contract;
use crate::dispatch::Dispatcher;
use crate::ids::CellId;
use crate::ids::GrantHandle;
use crate::loader;
use crate::loader::CellModuleLoader;
use crate::names;
use crate::ops;
use crate::ops::CellOpState;

/// The default V8 heap limit for a cell: 128 MiB.
pub const DEFAULT_HEAP_LIMIT: usize = 128 * 1024 * 1024;

/// One capability global to install in a cell.
#[derive(Clone, Debug)]
pub struct CellGrant {
  /// The global's name, chosen by the application's manifest.
  pub name: String,
  /// The contract the global implements.
  pub contract: Contract,
  /// The handle the dispatcher issued for this grant.
  pub handle: GrantHandle,
}

/// Everything needed to start a cell.
#[derive(Clone, Debug)]
pub struct CellConfig {
  pub id: CellId,
  /// A human-readable name, used for the cell's thread.
  pub name: String,
  /// The application's source directory. Modules can only be imported from
  /// inside it.
  pub source_root: PathBuf,
  /// The entry module, relative to `source_root`, for example `main.ts`.
  pub entry: String,
  /// One global per capability the application declared, whether or not its
  /// grant has been decided yet (§5.5).
  pub grants: Vec<CellGrant>,
  /// The V8 heap limit in bytes. See [`DEFAULT_HEAP_LIMIT`].
  pub heap_limit: usize,
}

impl CellConfig {
  /// A configuration with no grants and the default heap limit.
  pub fn new(
    id: CellId,
    name: impl Into<String>,
    source_root: impl Into<PathBuf>,
    entry: impl Into<String>,
  ) -> Self {
    Self {
      id,
      name: name.into(),
      source_root: source_root.into(),
      entry: entry.into(),
      grants: Vec::new(),
      heap_limit: DEFAULT_HEAP_LIMIT,
    }
  }
}

/// How a cell finished.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellExit {
  /// The entry module evaluated and the event loop ran out of work.
  Completed,
  /// Application code threw an uncaught exception or an unhandled promise
  /// rejection, or a module failed to load. Contains the formatted error.
  Failed(String),
  /// The host terminated the cell.
  Terminated,
  /// The cell approached its heap limit and was terminated.
  OutOfMemory,
}

impl fmt::Display for CellExit {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CellExit::Completed => f.write_str("completed"),
      CellExit::Failed(err) => write!(f, "failed: {err}"),
      CellExit::Terminated => f.write_str("terminated"),
      CellExit::OutOfMemory => f.write_str("terminated: out of memory"),
    }
  }
}

/// The result of running a cell.
#[derive(Clone, Debug)]
pub struct CellReport {
  pub id: CellId,
  pub exit: CellExit,
  /// Fire-and-forget calls dropped because the channel was full.
  pub dropped_notifications: u64,
}

/// A cell could not be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnError(String);

impl fmt::Display for SpawnError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl std::error::Error for SpawnError {}

/// Creates and runs cells.
///
/// Each cell runs on its own thread, with its own V8 isolate and a
/// single-threaded asynchronous runtime driving its event loop. Capability
/// calls are sent to `dispatcher` on the shared `supervisor` runtime
/// (PROPOSAL.md §7.1).
pub struct Engine {
  dispatcher: Arc<dyn Dispatcher>,
  supervisor: tokio::runtime::Handle,
}

impl Engine {
  /// Creates an engine.
  ///
  /// Call this on the main thread before starting any cells: the first call
  /// initializes the V8 platform, which must happen once per process before
  /// isolates are created on other threads.
  ///
  /// `supervisor` is a handle to a multi-threaded tokio runtime on which the
  /// dispatcher and providers run.
  pub fn new(dispatcher: Arc<dyn Dispatcher>, supervisor: tokio::runtime::Handle) -> Self {
    static INIT: Once = Once::new();
    INIT.call_once(|| JsRuntime::init_platform(None));
    Self {
      dispatcher,
      supervisor,
    }
  }

  /// Starts a cell on a new thread.
  ///
  /// Returns once the cell's isolate exists, so the returned handle can
  /// always terminate it. Errors in application code are reported through
  /// [`CellHandle::join`], not here.
  pub fn spawn(&self, config: CellConfig) -> Result<CellHandle, SpawnError> {
    for grant in &config.grants {
      names::validate_grant_name(&grant.name, grant.contract.id()).map_err(SpawnError)?;
    }
    let main_url = loader::module_url(&config.entry).map_err(SpawnError)?;
    let loader = config.source_root.clone();

    let id = config.id;
    let terminated = Arc::new(AtomicBool::new(false));
    let wake = Arc::new(Notify::new());
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();

    let setup = CellSetup {
      config,
      main_url,
      loader,
      dispatcher: self.dispatcher.clone(),
      supervisor: self.supervisor.clone(),
      terminated: terminated.clone(),
      wake: wake.clone(),
    };
    let thread = std::thread::Builder::new()
      .name(format!("{id}"))
      .spawn(move || {
        let report = run_cell_thread(setup, ready_tx);
        let _ = done_tx.send(report);
      })
      .map_err(|e| SpawnError(format!("could not start a thread for {id}: {e}")))?;

    match ready_rx.recv() {
      Ok(Ok(isolate)) => Ok(CellHandle {
        id,
        isolate,
        terminated,
        wake,
        done: done_rx,
        thread: Some(thread),
      }),
      Ok(Err(message)) => {
        let _ = thread.join();
        Err(SpawnError(message))
      }
      Err(_) => {
        let _ = thread.join();
        Err(SpawnError(format!("{id} failed to start")))
      }
    }
  }
}

/// Controls a running cell.
pub struct CellHandle {
  id: CellId,
  isolate: v8::IsolateHandle,
  terminated: Arc<AtomicBool>,
  wake: Arc<Notify>,
  done: std::sync::mpsc::Receiver<CellReport>,
  thread: Option<JoinHandle<()>>,
}

impl CellHandle {
  pub fn id(&self) -> CellId {
    self.id
  }

  /// Stops the cell.
  ///
  /// This interrupts running JavaScript, including infinite loops, and also
  /// stops a cell that is idle waiting on timers or capability calls. Safe to
  /// call from any thread, more than once, or after the cell has finished.
  pub fn terminate(&self) {
    self.terminated.store(true, Ordering::SeqCst);
    self.isolate.terminate_execution();
    self.wake.notify_one();
  }

  /// A handle that can terminate the cell from another thread.
  pub fn terminator(&self) -> Terminator {
    Terminator {
      isolate: self.isolate.clone(),
      terminated: self.terminated.clone(),
      wake: self.wake.clone(),
    }
  }

  /// Waits for the cell to finish.
  pub fn join(mut self) -> CellReport {
    let report = self.done.recv().unwrap_or(CellReport {
      id: self.id,
      exit: CellExit::Failed("the cell's thread exited unexpectedly".into()),
      dropped_notifications: 0,
    });
    if let Some(thread) = self.thread.take() {
      let _ = thread.join();
    }
    report
  }
}

/// Terminates a cell from another thread. See [`CellHandle::terminate`].
#[derive(Clone)]
pub struct Terminator {
  isolate: v8::IsolateHandle,
  terminated: Arc<AtomicBool>,
  wake: Arc<Notify>,
}

impl Terminator {
  pub fn terminate(&self) {
    self.terminated.store(true, Ordering::SeqCst);
    self.isolate.terminate_execution();
    self.wake.notify_one();
  }
}

struct CellSetup {
  config: CellConfig,
  main_url: deno_core::ModuleSpecifier,
  loader: PathBuf,
  dispatcher: Arc<dyn Dispatcher>,
  supervisor: tokio::runtime::Handle,
  terminated: Arc<AtomicBool>,
  wake: Arc<Notify>,
}

type Ready = std::sync::mpsc::Sender<Result<v8::IsolateHandle, String>>;

fn run_cell_thread(setup: CellSetup, ready: Ready) -> CellReport {
  let id = setup.config.id;
  let dropped = Arc::new(AtomicU64::new(0));
  let report = |exit| CellReport {
    id,
    exit,
    dropped_notifications: 0,
  };

  let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
    Ok(rt) => rt,
    Err(e) => {
      let _ = ready.send(Err(format!("could not start the event loop for {id}: {e}")));
      return report(CellExit::Failed(e.to_string()));
    }
  };
  // Resolves once every fire-and-forget call this cell made has been handed
  // to the dispatcher, so console output is complete before `join` returns.
  let (drained_tx, drained_rx) = tokio::sync::oneshot::channel();
  let exit = rt.block_on(run_cell(setup, ready, dropped.clone(), drained_tx));
  // The cell's isolate, and with it the sending side of the channel, has
  // been dropped by now.
  let _ = drained_rx.blocking_recv();
  CellReport {
    id,
    exit,
    dropped_notifications: dropped.load(Ordering::Relaxed),
  }
}

async fn run_cell(
  setup: CellSetup,
  ready: Ready,
  dropped: Arc<AtomicU64>,
  drained: tokio::sync::oneshot::Sender<()>,
) -> CellExit {
  let CellSetup {
    config,
    main_url,
    loader,
    dispatcher,
    supervisor,
    terminated,
    wake,
  } = setup;
  let id = config.id;

  let loader = match CellModuleLoader::open(&loader) {
    Ok(loader) => loader,
    Err(e) => {
      let _ = ready.send(Err(format!(
        "could not open the source directory {}: {e}",
        config.source_root.display()
      )));
      return CellExit::Failed(e.to_string());
    }
  };

  let mut js = match JsRuntime::try_new(RuntimeOptions {
    module_loader: Some(Rc::new(loader)),
    extensions: vec![ops::cells::init()],
    create_params: Some(v8::CreateParams::default().heap_limits(0, config.heap_limit)),
    ..Default::default()
  }) {
    Ok(js) => js,
    Err(e) => {
      let _ = ready.send(Err(format!("could not create the isolate for {id}: {e}")));
      return CellExit::Failed(e.to_string());
    }
  };

  // Out-of-memory protection (§7.7). Without this, V8 aborts the whole
  // process when one cell exhausts its heap. The callback terminates this
  // cell and raises the limit slightly so V8 can unwind.
  // TODO(worker-processes): Heap limits do not cover ArrayBuffer memory or
  // faults in V8 itself; process isolation does. See docs/deferred.md.
  let out_of_memory = Arc::new(AtomicBool::new(false));
  let isolate = js.v8_isolate().thread_safe_handle();
  {
    let out_of_memory = out_of_memory.clone();
    let isolate = isolate.clone();
    let wake = wake.clone();
    js.add_near_heap_limit_callback(move |current, _initial| {
      out_of_memory.store(true, Ordering::SeqCst);
      isolate.terminate_execution();
      wake.notify_one();
      current + current / 4
    });
  }

  // Fire-and-forget calls are forwarded in order by a task on the supervisor
  // runtime. The task ends when this cell's sender is dropped.
  // TODO(worker-processes): This channel becomes the IPC link. See
  // docs/deferred.md.
  let (notify_tx, mut notify_rx) = mpsc::channel(ops::NOTIFY_CHANNEL_CAPACITY);
  {
    let dispatcher = dispatcher.clone();
    supervisor.spawn(async move {
      while let Some(call) = notify_rx.recv().await {
        dispatcher.notify(call);
      }
      let _ = drained.send(());
    });
  }

  js.op_state().borrow_mut().put(CellOpState {
    cell: id,
    dispatcher,
    supervisor,
    notify_tx,
    dropped_notifications: dropped,
  });

  if let Err(e) = js.execute_script("cell:bootstrap", bootstrap::script(&config.grants)) {
    let _ = ready.send(Err(format!("could not build the globals for {id}: {e}")));
    return CellExit::Failed(e.to_string());
  }

  // From here on, the host can terminate the cell.
  if ready.send(Ok(isolate)).is_err() {
    return CellExit::Terminated;
  }

  let classify = |result: Result<(), deno_core::error::CoreError>| {
    if out_of_memory.load(Ordering::SeqCst) {
      CellExit::OutOfMemory
    } else if terminated.load(Ordering::SeqCst) {
      CellExit::Terminated
    } else {
      match result {
        Ok(()) => CellExit::Completed,
        Err(e) => CellExit::Failed(e.to_string()),
      }
    }
  };

  let run = async {
    let module = js.load_main_es_module(&main_url).await?;
    let evaluation = js.mod_evaluate(module);
    js.run_event_loop(Default::default()).await?;
    evaluation.await
  };

  // `run` returns when the application finishes or V8 stops it. `wake`
  // fires on termination, which matters when the cell is idle and no
  // JavaScript is running for V8 to interrupt.
  tokio::select! {
    result = run => classify(result),
    _ = wake.notified() => classify(Ok(())),
  }
}
