//! Rust operations reachable from a cell's JavaScript.
//!
//! Only the bootstrap script can call these: it captures them, then removes
//! `Deno` and `__bootstrap` from the global object before application code
//! loads (PROPOSAL.md §7.5).
//!
//! Capability operations are generic. `op_cell_invoke` and `op_cell_notify`
//! carry a grant handle, a method name, and arguments to the dispatcher. They
//! perform no authorization: calling them directly with any handle provides
//! no more authority than the cell's grants, because the dispatcher looks up
//! the handle together with this cell's identity.
//!
//! The remaining operations implement baseline web APIs (`URL`,
//! `TextDecoder`) and reach nothing outside the cell.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use deno_core::OpState;
use deno_core::op2;
use deno_core::url::Url;
use deno_core::url::quirks;
use deno_error::JsErrorBox;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use tokio::sync::mpsc;

use crate::dispatch::Call;
use crate::dispatch::Dispatcher;
use crate::error::CapabilityError;
use crate::ids::CellId;
use crate::ids::GrantHandle;

/// Per-cell state available to operations.
pub(crate) struct CellOpState {
  pub cell: CellId,
  pub dispatcher: Arc<dyn Dispatcher>,
  /// The shared runtime the dispatcher and providers run on.
  pub supervisor: tokio::runtime::Handle,
  /// Fire-and-forget calls, in order, to a forwarding task on the supervisor
  /// runtime. Bounded: when full, calls are dropped and counted rather than
  /// blocking the cell (§7.1).
  pub notify_tx: mpsc::Sender<Call>,
  pub dropped_notifications: Arc<AtomicU64>,
}

/// Capacity of each cell's fire-and-forget channel.
// TODO(server-host): Revisit dropping messages for persistent server
// execution, where lost log output may be unacceptable. See docs/deferred.md.
pub(crate) const NOTIFY_CHANNEL_CAPACITY: usize = 4096;

/// Built-in `deno_core` operations that must not be reachable even through
/// deno_core's own JavaScript (§7.5). They are replaced with operations that
/// throw.
///
/// - `op_panic` aborts the host process.
/// - `op_print` writes directly to stdout/stderr.
/// - `op_eval_context` and `op_compile_function` compile code under a
///   caller-chosen filename.
/// - `op_import_sync*` load modules outside the normal import path.
/// - `op_set_format_exception_callback` replaces host error formatting.
const DISABLED_BUILTIN_OPS: &[&str] = &[
  "op_panic",
  "op_print",
  "op_eval_context",
  "op_compile_function",
  "op_import_sync",
  "op_import_sync_with_source",
  "op_set_format_exception_callback",
];

deno_core::extension!(
  cells,
  ops = [
    op_cell_invoke,
    op_cell_notify,
    op_cell_url_parse,
    op_cell_url_set,
    op_cell_decode_utf8,
  ],
  middleware = |op| {
    if DISABLED_BUILTIN_OPS.contains(&op.name) {
      op.disable()
    } else {
      op
    }
  },
);

/// The JSON shape returned to the bootstrap script for each invocation.
fn outcome(result: Result<Value, CapabilityError>) -> Value {
  match result {
    Ok(value) => json!({ "ok": true, "value": value }),
    Err(err) => json!({ "ok": false, "kind": err.kind, "message": err.message }),
  }
}

/// Sends an awaited capability call to the dispatcher.
#[op2]
#[serde]
async fn op_cell_invoke(
  state: Rc<RefCell<OpState>>,
  #[smi] handle: u32,
  #[string] method: String,
  #[serde] args: Vec<serde_json::Value>,
) -> serde_json::Value {
  let (cell, dispatcher, supervisor) = {
    let state = state.borrow();
    let s = state.borrow::<CellOpState>();
    (s.cell, s.dispatcher.clone(), s.supervisor.clone())
  };
  let call = Call {
    cell,
    handle: GrantHandle::from_raw(handle),
    method,
    args,
  };
  // The dispatcher runs on the supervisor's runtime, not on this cell's
  // thread. Awaiting the task's handle is the reply channel.
  let task = supervisor.spawn(async move { dispatcher.invoke(call).await });
  let result = task
    .await
    .unwrap_or_else(|_| Err(CapabilityError::internal("the provider failed")));
  outcome(result)
}

/// Queues a fire-and-forget capability call.
#[op2]
fn op_cell_notify(
  state: &mut OpState,
  #[smi] handle: u32,
  #[string] method: String,
  #[serde] args: Vec<serde_json::Value>,
) {
  let s = state.borrow::<CellOpState>();
  let call = Call {
    cell: s.cell,
    handle: GrantHandle::from_raw(handle),
    method,
    args,
  };
  if s.notify_tx.try_send(call).is_err() {
    s.dropped_notifications.fetch_add(1, Ordering::Relaxed);
  }
}

/// The components of a parsed URL, as the WHATWG URL standard defines them.
#[derive(Serialize)]
struct UrlParts {
  href: String,
  origin: String,
  protocol: String,
  username: String,
  password: String,
  host: String,
  hostname: String,
  port: String,
  pathname: String,
  search: String,
  hash: String,
}

impl UrlParts {
  fn of(url: &Url) -> Self {
    Self {
      href: quirks::href(url).to_string(),
      origin: quirks::origin(url),
      protocol: quirks::protocol(url).to_string(),
      username: quirks::username(url).to_string(),
      password: quirks::password(url).to_string(),
      host: quirks::host(url).to_string(),
      hostname: quirks::hostname(url).to_string(),
      port: quirks::port(url).to_string(),
      pathname: quirks::pathname(url).to_string(),
      search: quirks::search(url).to_string(),
      hash: quirks::hash(url).to_string(),
    }
  }
}

/// Parses a URL for the baseline `URL` class.
#[op2]
#[serde]
fn op_cell_url_parse(
  #[string] href: String,
  #[serde] base: serde_json::Value,
) -> Result<UrlParts, JsErrorBox> {
  let parsed = match base {
    serde_json::Value::String(base) => Url::parse(&base).and_then(|b| b.join(&href)),
    _ => Url::parse(&href),
  };
  parsed
    .map(|url| UrlParts::of(&url))
    .map_err(|_| JsErrorBox::type_error(format!("Invalid URL: {href:?}")))
}

/// Applies a `URL` setter, such as `url.pathname = "/x"`.
#[op2]
#[serde]
fn op_cell_url_set(
  #[string] href: String,
  #[string] field: String,
  #[string] value: String,
) -> Result<UrlParts, JsErrorBox> {
  let mut url =
    Url::parse(&href).map_err(|_| JsErrorBox::type_error("Invalid URL"))?;
  // As in the standard, setters that receive an unusable value leave the
  // URL unchanged, except `href`, which throws.
  match field.as_str() {
    "href" => {
      url = Url::parse(&value)
        .map_err(|_| JsErrorBox::type_error(format!("Invalid URL: {value:?}")))?;
    }
    "protocol" => {
      let _ = quirks::set_protocol(&mut url, &value);
    }
    "username" => {
      let _ = quirks::set_username(&mut url, &value);
    }
    "password" => {
      let _ = quirks::set_password(&mut url, &value);
    }
    "host" => {
      let _ = quirks::set_host(&mut url, &value);
    }
    "hostname" => {
      let _ = quirks::set_hostname(&mut url, &value);
    }
    "port" => {
      let _ = quirks::set_port(&mut url, &value);
    }
    "pathname" => quirks::set_pathname(&mut url, &value),
    "search" => quirks::set_search(&mut url, &value),
    "hash" => quirks::set_hash(&mut url, &value),
    _ => return Err(JsErrorBox::type_error(format!("Unknown URL field {field:?}"))),
  }
  Ok(UrlParts::of(&url))
}

/// Decodes UTF-8 for the baseline `TextDecoder` class.
#[op2]
#[string]
fn op_cell_decode_utf8(
  #[buffer] bytes: &[u8],
  fatal: bool,
  ignore_bom: bool,
) -> Result<String, JsErrorBox> {
  let bytes = match bytes {
    [0xEF, 0xBB, 0xBF, rest @ ..] if !ignore_bom => rest,
    _ => bytes,
  };
  if fatal {
    std::str::from_utf8(bytes)
      .map(str::to_string)
      .map_err(|_| JsErrorBox::type_error("The encoded data was not valid UTF-8"))
  } else {
    Ok(String::from_utf8_lossy(bytes).into_owned())
  }
}
