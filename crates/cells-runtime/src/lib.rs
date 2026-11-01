//! The Cells runtime.
//!
//! The runtime executes TypeScript applications in **cells**: isolated
//! execution environments, each with its own V8 isolate, its own module
//! state, and only the capabilities granted to it (PROPOSAL.md §1.2, §7).
//!
//! # How the pieces fit together
//!
//! ```text
//!  cell thread (one per cell)              supervisor runtime (shared)
//! ┌──────────────────────────────┐        ┌────────────────────────────┐
//! │ application code             │        │ Dispatcher (e.g. GrantTable)│
//! │   await notes.readText(..)   │        │   looks up (cell, handle)   │
//! │        │                     │  Call  │   checks the grant          │
//! │ capability global            ├───────►│        │                    │
//! │   wraps a GrantHandle        │        │        ▼                    │
//! │        │                     │◄───────┤ Provider (host-supplied)    │
//! │ op_cell_invoke (no authz)    │ result │   enforces the grant scope  │
//! └──────────────────────────────┘        └────────────────────────────┘
//! ```
//!
//! - A capability global in JavaScript is only a handle. It carries no
//!   authority (§7.5).
//! - Capability operations forward the handle, method name, and arguments to
//!   a [`Dispatcher`] running on a separate, shared asynchronous runtime. The
//!   operations themselves perform no authorization (§6.7).
//! - The dispatcher checks that the calling cell holds a valid grant, then
//!   calls the [`Provider`] that implements the capability. The provider
//!   enforces the grant's scope.
//!
//! [`GrantTable`] is a simple in-memory dispatcher, so the runtime can be used
//! without the supervisor. Hosts supply providers, contracts, and policy.
//!
//! # Security status
//!
//! This is the Stage 1 runtime (§7.8): every cell runs in the host process.
//! It removes ambient authority and checks every capability call, but it is
//! **not** a security boundary against malicious code.

mod bootstrap;
mod cell;
mod contract;
mod dispatch;
mod error;
mod grants;
mod ids;
mod inspector;
mod loader;
mod names;
mod ops;

pub use cell::CellConfig;
pub use cell::CellExit;
pub use cell::CellGrant;
pub use cell::CellHandle;
pub use cell::CellReport;
pub use cell::DEFAULT_HEAP_LIMIT;
pub use cell::Engine;
pub use cell::SpawnError;
pub use cell::Terminator;
pub use contract::Contract;
pub use contract::ContractError;
pub use contract::Method;
pub use contract::MethodKind;
pub use contract::standard;
pub use dispatch::Call;
pub use dispatch::CallFuture;
pub use dispatch::Dispatcher;
pub use dispatch::Provider;
pub use error::CapabilityError;
pub use error::ErrorKind;
pub use grants::GrantInfo;
pub use grants::GrantState;
pub use grants::GrantStatus;
pub use grants::GrantTable;
pub use grants::Resolve;
pub use grants::ResolveFuture;
pub use grants::ResolveRequest;
pub use ids::CellId;
pub use inspector::CellInspector;
pub use inspector::InspectMode;
pub use inspector::InspectorReceiver;
pub use inspector::InspectorSender;
pub use inspector::InspectorSession;
pub use ids::GrantHandle;

/// JSON values are the wire format for capability arguments and results.
///
/// They are serializable, which keeps the same messages usable across a
/// process boundary in Stage 2 (§8.5).
pub use serde_json::Value;
