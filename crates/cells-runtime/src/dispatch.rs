//! The dispatch interface between cells and the trusted side of the host.

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::error::CapabilityError;
use crate::ids::CellId;
use crate::ids::GrantHandle;

/// One capability operation sent by a cell.
///
/// This is the message that crosses from the cell to the dispatcher. It is
/// plain data so that the same message can travel over IPC once cells run in
/// worker processes.
#[derive(Clone, Debug)]
pub struct Call {
  /// The cell that made the call. Set by the runtime, never by application
  /// code.
  pub cell: CellId,
  /// The grant handle wrapped by the capability global the application used.
  /// Supplied by application code, so it must be treated as untrusted.
  pub handle: GrantHandle,
  /// The method name. Untrusted.
  pub method: String,
  /// The method arguments. Untrusted.
  pub args: Vec<Value>,
}

/// The result of a capability operation.
pub type CallFuture =
  Pin<Box<dyn Future<Output = Result<Value, CapabilityError>> + Send + 'static>>;

/// Receives capability operations from cells, authorizes them, and routes
/// them to providers.
///
/// This is the security boundary of the runtime (PROPOSAL.md §7.5). An
/// implementation must look up `call.handle` **together with** `call.cell`
/// and reject the call unless that cell currently holds a valid grant behind
/// that handle.
///
/// The runtime calls these methods on the shared supervisor runtime, never on
/// a cell's thread.
///
/// [`crate::GrantTable`] is the in-memory implementation shipped with the
/// runtime.
// TODO(supervisor): The supervisor implements this trait and owns the grant
// table. See docs/deferred.md.
// TODO(worker-processes): Cells reach the dispatcher over IPC instead of
// in-process channels; this trait stays on the supervisor's side.
pub trait Dispatcher: Send + Sync + 'static {
  /// Handles an operation whose result the application awaits.
  fn invoke(&self, call: Call) -> CallFuture;

  /// Handles a fire-and-forget operation, such as `console.log`. These return
  /// nothing and report no errors to the application (§6.6).
  fn notify(&self, call: Call);
}

/// A host-supplied implementation of a capability contract.
///
/// A provider instance is created per grant and holds that grant's scope, for
/// example the directory a filesystem grant is restricted to. It only
/// receives calls the dispatcher has already authorized, but its arguments
/// are still untrusted: it must validate them and enforce its scope (§6.2).
pub trait Provider: Send + Sync + 'static {
  /// A short description of the provider and its scope, for inspection.
  /// Example: `local-files (root: ./data)`.
  fn describe(&self) -> String;

  /// Performs an operation whose result the application awaits.
  fn invoke(&self, method: &str, args: Vec<Value>) -> CallFuture;

  /// Performs a fire-and-forget operation. The default ignores it.
  fn notify(&self, method: &str, args: Vec<Value>) {
    let _ = (method, args);
  }
}
