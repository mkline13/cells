//! An in-memory grant table that implements [`Dispatcher`].

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use serde::Serialize;

use crate::contract::Contract;
use crate::contract::MethodKind;
use crate::dispatch::Call;
use crate::dispatch::CallFuture;
use crate::dispatch::Dispatcher;
use crate::dispatch::Provider;
use crate::error::CapabilityError;
use crate::ids::CellId;
use crate::ids::GrantHandle;

/// The future returned by [`Resolve::resolve`]: a provider if the host grants
/// the request, or `None` if it denies it.
pub type ResolveFuture =
  Pin<Box<dyn Future<Output = Option<Arc<dyn Provider>>> + Send + 'static>>;

/// Host policy for a grant that is decided on first use (PROPOSAL.md §5.5).
///
/// A desktop host might ask the user. The resolver runs at most once per
/// grant; concurrent calls wait for its answer.
pub trait Resolve: Send + Sync + 'static {
  fn resolve(&self, request: ResolveRequest) -> ResolveFuture;
}

/// What the host is being asked to decide.
#[derive(Clone, Debug)]
pub struct ResolveRequest {
  pub cell: CellId,
  pub grant_name: String,
  pub contract: String,
}

/// The state a grant starts in when it is registered.
pub enum GrantState {
  /// Operations are dispatched to this provider.
  Granted(Arc<dyn Provider>),
  /// Operations are rejected with `permission-denied`.
  Denied,
  /// The host decides when the grant is first used.
  Unresolved(Arc<dyn Resolve>),
}

/// The current status of a grant, for inspection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantStatus {
  Granted,
  Denied,
  Unresolved,
  Revoked,
}

/// A description of one grant, for inspection.
#[derive(Clone, Debug, Serialize)]
pub struct GrantInfo {
  pub handle: GrantHandle,
  pub name: String,
  pub contract: String,
  pub status: GrantStatus,
  /// The provider's description, when one is known.
  pub provider: Option<String>,
}

enum SlotState {
  Granted(Arc<dyn Provider>),
  Denied,
  Unresolved(Arc<dyn Resolve>),
  Revoked,
}

struct Slot {
  name: String,
  contract: Contract,
  state: Mutex<SlotState>,
  /// Serializes resolution so the resolver runs at most once.
  resolving: tokio::sync::Mutex<()>,
}

impl Slot {
  /// Returns the provider if the grant may be used right now, resolving it
  /// first if needed.
  async fn provider(
    &self,
    cell: CellId,
  ) -> Result<Arc<dyn Provider>, CapabilityError> {
    if let Some(result) = self.settled() {
      return result;
    }
    let _guard = self.resolving.lock().await;
    // Another call may have resolved the grant while this one waited.
    let resolver = {
      let state = self.state.lock().unwrap();
      match &*state {
        SlotState::Unresolved(resolver) => resolver.clone(),
        _ => {
          drop(state);
          return self.settled().expect("grant is settled");
        }
      }
    };
    let decision = resolver
      .resolve(ResolveRequest {
        cell,
        grant_name: self.name.clone(),
        contract: self.contract.id().to_string(),
      })
      .await;
    let mut state = self.state.lock().unwrap();
    // A revocation during resolution wins.
    if matches!(*state, SlotState::Unresolved(_)) {
      *state = match decision {
        Some(provider) => SlotState::Granted(provider),
        None => SlotState::Denied,
      };
    }
    drop(state);
    self.settled().expect("grant is settled")
  }

  /// The outcome for a grant that is not waiting on a decision.
  fn settled(&self) -> Option<Result<Arc<dyn Provider>, CapabilityError>> {
    match &*self.state.lock().unwrap() {
      SlotState::Granted(provider) => Some(Ok(provider.clone())),
      SlotState::Denied => Some(Err(CapabilityError::permission_denied(
        format!("`{}` was not granted", self.name),
      ))),
      SlotState::Revoked => Some(Err(CapabilityError::permission_denied(
        format!("`{}` has been revoked", self.name),
      ))),
      SlotState::Unresolved(_) => None,
    }
  }

  fn info(&self, handle: GrantHandle) -> GrantInfo {
    let (status, provider) = match &*self.state.lock().unwrap() {
      SlotState::Granted(p) => (GrantStatus::Granted, Some(p.describe())),
      SlotState::Denied => (GrantStatus::Denied, None),
      SlotState::Unresolved(_) => (GrantStatus::Unresolved, None),
      SlotState::Revoked => (GrantStatus::Revoked, None),
    };
    GrantInfo {
      handle,
      name: self.name.clone(),
      contract: self.contract.id().to_string(),
      status,
      provider,
    }
  }
}

#[derive(Default)]
struct CellGrants {
  next_handle: u32,
  slots: BTreeMap<GrantHandle, Arc<Slot>>,
}

/// A simple, in-memory record of which grants each cell holds.
///
/// It checks every call against the calling cell's own grants, then
/// dispatches it to the grant's provider. This lets the runtime run without
/// the supervisor, for tests, embedding, and the MVP host.
// TODO(supervisor): The supervisor owns grants and implements `Dispatcher`.
// This type remains as a test and embedding convenience. See docs/deferred.md.
#[derive(Default)]
pub struct GrantTable {
  cells: Mutex<HashMap<CellId, CellGrants>>,
}

impl GrantTable {
  pub fn new() -> Arc<Self> {
    Arc::new(Self::default())
  }

  /// Records a grant for `cell` and returns the handle that identifies it.
  ///
  /// `name` is the application's name for the grant, which becomes its
  /// global (§6.1).
  pub fn register(
    &self,
    cell: CellId,
    name: impl Into<String>,
    contract: Contract,
    state: GrantState,
  ) -> GrantHandle {
    let mut cells = self.cells.lock().unwrap();
    let grants = cells.entry(cell).or_default();
    grants.next_handle += 1;
    let handle = GrantHandle::from_raw(grants.next_handle);
    let state = match state {
      GrantState::Granted(p) => SlotState::Granted(p),
      GrantState::Denied => SlotState::Denied,
      GrantState::Unresolved(r) => SlotState::Unresolved(r),
    };
    grants.slots.insert(
      handle,
      Arc::new(Slot {
        name: name.into(),
        contract,
        state: Mutex::new(state),
        resolving: tokio::sync::Mutex::new(()),
      }),
    );
    handle
  }

  /// Withdraws a grant. Later operations that use it are rejected (§6.7).
  /// Returns `false` if the cell has no such grant.
  pub fn revoke(&self, cell: CellId, handle: GrantHandle) -> bool {
    match self.slot(cell, handle) {
      Some(slot) => {
        *slot.state.lock().unwrap() = SlotState::Revoked;
        true
      }
      None => false,
    }
  }

  /// Removes every grant held by `cell`, for example after it exits.
  pub fn remove_cell(&self, cell: CellId) {
    self.cells.lock().unwrap().remove(&cell);
  }

  /// Describes every grant held by `cell`, in registration order.
  pub fn inspect(&self, cell: CellId) -> Vec<GrantInfo> {
    let cells = self.cells.lock().unwrap();
    cells
      .get(&cell)
      .map(|g| g.slots.iter().map(|(h, s)| s.info(*h)).collect())
      .unwrap_or_default()
  }

  fn slot(&self, cell: CellId, handle: GrantHandle) -> Option<Arc<Slot>> {
    // The lookup is keyed by the calling cell: a handle issued to another
    // cell is not found here.
    let cells = self.cells.lock().unwrap();
    cells.get(&cell)?.slots.get(&handle).cloned()
  }

  /// Finds the grant and checks that the method exists with the right kind.
  fn authorize(
    &self,
    call: &Call,
    kind: MethodKind,
  ) -> Result<Arc<Slot>, CapabilityError> {
    let slot = self.slot(call.cell, call.handle).ok_or_else(|| {
      CapabilityError::permission_denied("no such grant for this cell")
    })?;
    match slot.contract.method(&call.method) {
      Some(m) if m.kind == kind => Ok(slot),
      _ => Err(CapabilityError::invalid_argument(format!(
        "{} has no method `{}`",
        slot.contract.id(),
        call.method
      ))),
    }
  }
}

impl Dispatcher for GrantTable {
  fn invoke(&self, call: Call) -> CallFuture {
    let slot = match self.authorize(&call, MethodKind::Invoke) {
      Ok(slot) => slot,
      Err(err) => return Box::pin(async move { Err(err) }),
    };
    Box::pin(async move {
      let provider = slot.provider(call.cell).await?;
      provider.invoke(&call.method, call.args).await
    })
  }

  fn notify(&self, call: Call) {
    // Fire-and-forget operations report no errors, so a call that is not
    // authorized is dropped. An unresolved grant is not resolved by a
    // notification: asking the user for permission because of a log line
    // would be surprising.
    let Ok(slot) = self.authorize(&call, MethodKind::Notify) else {
      return;
    };
    if let Some(Ok(provider)) = slot.settled() {
      provider.notify(&call.method, call.args);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::AtomicUsize;
  use std::sync::atomic::Ordering;

  use serde_json::json;

  use super::*;
  use crate::error::ErrorKind;

  struct Echo;

  impl Provider for Echo {
    fn describe(&self) -> String {
      "echo".into()
    }
    fn invoke(&self, method: &str, args: Vec<serde_json::Value>) -> CallFuture {
      let reply = json!({ "method": method, "args": args });
      Box::pin(async move { Ok(reply) })
    }
  }

  struct CountingResolver {
    calls: AtomicUsize,
    allow: bool,
  }

  impl Resolve for CountingResolver {
    fn resolve(&self, _request: ResolveRequest) -> ResolveFuture {
      self.calls.fetch_add(1, Ordering::SeqCst);
      let decision: Option<Arc<dyn Provider>> =
        if self.allow { Some(Arc::new(Echo)) } else { None };
      Box::pin(async move { decision })
    }
  }

  fn contract() -> Contract {
    Contract::new("echo@1").unwrap().invoke("say").notify("shout")
  }

  fn call(cell: CellId, handle: GrantHandle, method: &str) -> Call {
    Call {
      cell,
      handle,
      method: method.into(),
      args: vec![json!("hi")],
    }
  }

  #[tokio::test]
  async fn granted_calls_reach_the_provider() {
    let table = GrantTable::new();
    let cell = CellId::next();
    let h = table.register(cell, "echo", contract(), GrantState::Granted(Arc::new(Echo)));
    let result = table.invoke(call(cell, h, "say")).await.unwrap();
    assert_eq!(result, json!({ "method": "say", "args": ["hi"] }));
  }

  #[tokio::test]
  async fn handles_are_scoped_to_their_cell() {
    let table = GrantTable::new();
    let owner = CellId::next();
    let other = CellId::next();
    let h = table.register(owner, "echo", contract(), GrantState::Granted(Arc::new(Echo)));
    // `other` holds no grants, so the same handle number means nothing.
    let err = table.invoke(call(other, h, "say")).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::PermissionDenied);
  }

  #[tokio::test]
  async fn forged_handles_are_rejected() {
    let table = GrantTable::new();
    let cell = CellId::next();
    table.register(cell, "echo", contract(), GrantState::Granted(Arc::new(Echo)));
    let err = table
      .invoke(call(cell, GrantHandle::from_raw(999), "say"))
      .await
      .unwrap_err();
    assert_eq!(err.kind, ErrorKind::PermissionDenied);
  }

  #[tokio::test]
  async fn unknown_methods_never_reach_the_provider() {
    let table = GrantTable::new();
    let cell = CellId::next();
    let h = table.register(cell, "echo", contract(), GrantState::Granted(Arc::new(Echo)));
    let err = table.invoke(call(cell, h, "launchMissiles")).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidArgument);
    // A notify method cannot be invoked, and vice versa.
    let err = table.invoke(call(cell, h, "shout")).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidArgument);
  }

  #[tokio::test]
  async fn denied_and_revoked_grants_are_rejected() {
    let table = GrantTable::new();
    let cell = CellId::next();
    let denied = table.register(cell, "a", contract(), GrantState::Denied);
    let granted = table.register(cell, "b", contract(), GrantState::Granted(Arc::new(Echo)));

    let err = table.invoke(call(cell, denied, "say")).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::PermissionDenied);

    assert!(table.invoke(call(cell, granted, "say")).await.is_ok());
    assert!(table.revoke(cell, granted));
    let err = table.invoke(call(cell, granted, "say")).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::PermissionDenied);
    assert_eq!(table.inspect(cell)[1].status, GrantStatus::Revoked);
  }

  #[tokio::test]
  async fn unresolved_grants_are_resolved_once() {
    let table = GrantTable::new();
    let cell = CellId::next();
    let resolver = Arc::new(CountingResolver {
      calls: AtomicUsize::new(0),
      allow: true,
    });
    let h = table.register(cell, "echo", contract(), GrantState::Unresolved(resolver.clone()));
    assert_eq!(table.inspect(cell)[0].status, GrantStatus::Unresolved);

    let (a, b) = tokio::join!(
      table.invoke(call(cell, h, "say")),
      table.invoke(call(cell, h, "say")),
    );
    assert!(a.is_ok() && b.is_ok());
    assert!(table.invoke(call(cell, h, "say")).await.is_ok());
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
    assert_eq!(table.inspect(cell)[0].status, GrantStatus::Granted);
  }

  #[tokio::test]
  async fn unresolved_grants_can_be_denied() {
    let table = GrantTable::new();
    let cell = CellId::next();
    let resolver = Arc::new(CountingResolver {
      calls: AtomicUsize::new(0),
      allow: false,
    });
    let h = table.register(cell, "echo", contract(), GrantState::Unresolved(resolver.clone()));
    let err = table.invoke(call(cell, h, "say")).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::PermissionDenied);
    assert!(table.invoke(call(cell, h, "say")).await.is_err());
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
  }
}
