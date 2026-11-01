//! Capability providers supplied by this host.
//!
//! Providers are host-specific (PROPOSAL.md §1.2): a desktop host and a
//! server host would each supply their own. Each provider instance is created
//! for one grant and holds that grant's scope.

mod console;
mod files;
mod kv;

use std::path::Path;
use std::sync::Arc;

use cells_runtime::Provider;
use cells_runtime::Value;

/// What a provider factory may use to build an instance.
pub struct ProviderContext<'a> {
  /// The application's name, used to tag output.
  pub app_name: &'a str,
  /// The directory relative paths in the host config are resolved against.
  pub base_dir: &'a Path,
  /// Provider-specific options from the host config.
  pub options: &'a serde_json::Map<String, Value>,
}

/// A provider this host knows how to build.
pub struct ProviderKind {
  pub name: &'static str,
  /// The contract its instances implement.
  pub contract: &'static str,
  pub build: fn(&ProviderContext) -> Result<Arc<dyn Provider>, String>,
}

/// Every provider available in this host.
pub const PROVIDERS: &[ProviderKind] = &[
  ProviderKind {
    name: "stdout",
    contract: "console@1",
    build: console::build,
  },
  ProviderKind {
    name: "local-files",
    contract: "files@1",
    build: files::build,
  },
  ProviderKind {
    name: "memory-kv",
    contract: "kv@1",
    build: kv::build_memory,
  },
  ProviderKind {
    name: "json-file-kv",
    contract: "kv@1",
    build: kv::build_json_file,
  },
];

pub fn find(name: &str) -> Option<&'static ProviderKind> {
  PROVIDERS.iter().find(|p| p.name == name)
}

// --- Helpers for validating untrusted arguments --------------------------------

use cells_runtime::CapabilityError;

/// Reads a required string argument.
pub(crate) fn string_arg(args: &[Value], index: usize, name: &str) -> Result<String, CapabilityError> {
  match args.get(index) {
    Some(Value::String(s)) => Ok(s.clone()),
    _ => Err(CapabilityError::invalid_argument(format!("`{name}` must be a string"))),
  }
}

/// Reads an optional string argument.
pub(crate) fn optional_string_arg(
  args: &[Value],
  index: usize,
  name: &str,
) -> Result<Option<String>, CapabilityError> {
  match args.get(index) {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(s)) => Ok(Some(s.clone())),
    _ => Err(CapabilityError::invalid_argument(format!("`{name}` must be a string"))),
  }
}

/// Reads a string option from the host config.
pub(crate) fn string_option(ctx: &ProviderContext, key: &str) -> Result<String, String> {
  match ctx.options.get(key) {
    Some(Value::String(s)) => Ok(s.clone()),
    Some(_) => Err(format!("option `{key}` must be a string")),
    None => Err(format!("missing option `{key}`")),
  }
}

/// Reads a boolean option from the host config.
pub(crate) fn bool_option(ctx: &ProviderContext, key: &str) -> Result<bool, String> {
  match ctx.options.get(key) {
    Some(Value::Bool(b)) => Ok(*b),
    Some(_) => Err(format!("option `{key}` must be true or false")),
    None => Ok(false),
  }
}

/// Rejects options a provider does not understand, so typos are caught.
pub(crate) fn only_options(ctx: &ProviderContext, allowed: &[&str]) -> Result<(), String> {
  for key in ctx.options.keys() {
    if !allowed.contains(&key.as_str()) {
      return Err(format!("unknown option `{key}`"));
    }
  }
  Ok(())
}
