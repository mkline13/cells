//! Host configuration (`host.json`).
//!
//! The host config is the host's side of the story: which provider backs
//! each of the application's grant names, what scope it has, and whether the
//! grant is decided now or when first used (PROPOSAL.md §3.10, §5.4, §5.5).
//!
//! ```json
//! {
//!   "heapLimitMb": 64,
//!   "grants": {
//!     "console": { "provider": "stdout" },
//!     "notes":   { "provider": "local-files", "root": "data/notes" },
//!     "cache":   { "provider": "memory-kv", "decision": "ask" }
//!   }
//! }
//! ```
//!
//! `decision` is one of:
//! - `grant` (default): granted before the cell starts.
//! - `deny`: present in the application, but every call is rejected.
//! - `ask`: decided on first use by asking on the terminal. Only allowed for
//!   optional capabilities.
//!
//! Relative paths are resolved against the directory containing the config.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HostConfig {
  #[serde(default)]
  pub heap_limit_mb: Option<usize>,
  #[serde(default)]
  pub grants: BTreeMap<String, GrantConfig>,
  /// The directory relative paths are resolved against.
  #[serde(skip)]
  pub base_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantConfig {
  pub provider: String,
  #[serde(default)]
  pub decision: Decision,
  /// Provider-specific settings, such as `root` or `readOnly`.
  #[serde(flatten)]
  pub options: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
  #[default]
  Grant,
  Deny,
  Ask,
}

impl HostConfig {
  pub fn load(path: &Path) -> Result<Self, String> {
    let text = std::fs::read_to_string(path)
      .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut config: HostConfig =
      serde_json::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    config.base_dir = path
      .parent()
      .map(Path::to_path_buf)
      .unwrap_or_else(|| PathBuf::from("."));
    Ok(config)
  }
}
