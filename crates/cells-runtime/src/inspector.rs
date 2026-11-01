//! Debugger access to a cell through V8's inspector.
//!
//! V8 includes a debugger that speaks the Chrome DevTools Protocol, the same
//! protocol VS Code and Chrome DevTools use. When a cell is started with
//! [`CellConfig::inspect`](crate::CellConfig::inspect) set, the runtime turns
//! it on and the host can open sessions with [`CellInspector::connect`].
//!
//! The runtime only provides the sessions. Exposing them, for example as a
//! WebSocket server, is up to the host, because a debugger can read and
//! change anything inside the cell. It is a host capability, never an
//! application one.

use deno_core::InspectorMsg;
use deno_core::InspectorSessionChannels;
use deno_core::InspectorSessionKind;
use deno_core::InspectorSessionProxy;
use deno_core::futures::StreamExt;
use deno_core::futures::channel::mpsc;

/// How a cell's debugger is set up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectMode {
  /// The cell starts running immediately. A debugger can attach at any time.
  Attach,
  /// The cell waits for a debugger to attach and resume it, then pauses on
  /// its first statement. Use this to debug code that runs at startup.
  ///
  /// While waiting, the cell cannot be terminated.
  // TODO(worker-processes): Waiting blocks the cell's thread, so
  // `CellHandle::terminate` cannot interrupt it. A worker process can simply
  // be killed. See docs/deferred.md.
  BreakOnStart,
}

/// Opens debugger sessions on one cell. Cheap to clone; usable from any
/// thread.
#[derive(Clone)]
pub struct CellInspector {
  new_sessions: mpsc::UnboundedSender<InspectorSessionProxy>,
}

impl CellInspector {
  pub(crate) fn new(new_sessions: mpsc::UnboundedSender<InspectorSessionProxy>) -> Self {
    Self { new_sessions }
  }

  /// Opens a new session. Returns `None` if the cell has exited.
  ///
  /// The cell picks up new sessions while its event loop runs, or while it
  /// is paused in the debugger or waiting for one.
  pub fn connect(&self) -> Option<InspectorSession> {
    let (to_frontend_tx, to_frontend_rx) = mpsc::unbounded::<InspectorMsg>();
    let (to_cell_tx, to_cell_rx) = mpsc::unbounded::<String>();
    let proxy = InspectorSessionProxy {
      channels: InspectorSessionChannels::Regular {
        tx: to_frontend_tx,
        rx: to_cell_rx,
      },
      // A debugger session does not keep the cell alive on its own.
      kind: InspectorSessionKind::NonBlocking {
        wait_for_disconnect: false,
      },
    };
    self.new_sessions.unbounded_send(proxy).ok()?;
    Some(InspectorSession {
      to_cell: to_cell_tx,
      from_cell: to_frontend_rx,
    })
  }
}

/// One debugger session: DevTools protocol messages, as JSON text, in both
/// directions.
pub struct InspectorSession {
  to_cell: mpsc::UnboundedSender<String>,
  from_cell: mpsc::UnboundedReceiver<InspectorMsg>,
}

impl InspectorSession {
  /// Sends a protocol message to the cell. Returns `false` if the session
  /// has closed.
  pub fn send(&self, message: String) -> bool {
    self.to_cell.unbounded_send(message).is_ok()
  }

  /// Waits for the next protocol message from the cell. Returns `None` when
  /// the session has closed, for example because the cell exited.
  pub async fn recv(&mut self) -> Option<String> {
    self.from_cell.next().await.map(|msg| msg.content)
  }

  /// Splits the session so sending and receiving can happen in different
  /// tasks.
  pub fn split(self) -> (InspectorSender, InspectorReceiver) {
    (
      InspectorSender { to_cell: self.to_cell },
      InspectorReceiver {
        from_cell: self.from_cell,
      },
    )
  }
}

/// The sending half of an [`InspectorSession`].
pub struct InspectorSender {
  to_cell: mpsc::UnboundedSender<String>,
}

impl InspectorSender {
  /// See [`InspectorSession::send`].
  pub fn send(&self, message: String) -> bool {
    self.to_cell.unbounded_send(message).is_ok()
  }
}

/// The receiving half of an [`InspectorSession`].
pub struct InspectorReceiver {
  from_cell: mpsc::UnboundedReceiver<InspectorMsg>,
}

impl InspectorReceiver {
  /// See [`InspectorSession::recv`].
  pub async fn recv(&mut self) -> Option<String> {
    self.from_cell.next().await.map(|msg| msg.content)
  }
}
