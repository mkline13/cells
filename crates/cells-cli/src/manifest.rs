//! Application manifests (`cell.json`).
//!
//! The manifest is the application's side of the story: who it is, where it
//! starts, and which capabilities it expects (PROPOSAL.md §5.2, §6.3). It
//! never grants anything by itself.
//!
//! ```json
//! {
//!   "name": "notes",
//!   "version": "0.1.0",
//!   "entry": "main.ts",
//!   "capabilities": {
//!     "console": { "contract": "console@1", "required": true },
//!     "notes":   { "contract": "files@1",   "required": true },
//!     "cache":   { "contract": "kv@1",      "required": false }
//!   }
//! }
//! ```
//!
//! Each key under `capabilities` is a grant name, which becomes the
//! capability's global in the application.
// TODO(supervisor): Manifest parsing and validation move into the supervisor.
// See docs/deferred.md.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
  pub name: String,
  pub version: String,
  pub entry: String,
  #[serde(default)]
  pub capabilities: BTreeMap<String, CapabilityDecl>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDecl {
  /// The contract and version, for example `files@1`.
  pub contract: String,
  /// Required capabilities must be granted before the cell starts. Optional
  /// ones may be granted at start, on first use, or not at all (§5.3, §5.5).
  pub required: bool,
}

pub const MANIFEST_FILE: &str = "cell.json";

impl Manifest {
  pub fn load(app_dir: &Path) -> Result<Self, String> {
    let path = app_dir.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(&path)
      .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
  }
}
