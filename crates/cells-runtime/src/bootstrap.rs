//! Generates the bootstrap script that builds a cell's global scope.

use std::collections::BTreeMap;

use serde_json::json;

use crate::cell::CellGrant;

const BOOTSTRAP_JS: &str = include_str!("js/bootstrap.js");

/// Returns a classic script that calls `bootstrap(grants, factories)`.
///
/// The grant list is serialized as JSON, so application-chosen names cannot
/// inject code. Factories are trusted host code supplied by contracts.
pub(crate) fn script(grants: &[CellGrant]) -> String {
  let grant_list: Vec<_> = grants
    .iter()
    .map(|g| {
      json!({
        "name": g.name,
        "contract": g.contract.id(),
        "handle": g.handle.get(),
        "methods": g.contract.methods(),
      })
    })
    .collect();

  let mut factories = BTreeMap::new();
  for grant in grants {
    if let Some(source) = grant.contract.js_factory() {
      factories.insert(grant.contract.id().to_string(), source);
    }
  }
  let factories = factories
    .into_iter()
    .map(|(id, source)| {
      let key = serde_json::to_string(&id).expect("string serializes");
      format!("{key}: ({source})")
    })
    .collect::<Vec<_>>()
    .join(",\n");

  let grants_json = serde_json::to_string(&grant_list).expect("grants serialize");
  format!("{BOOTSTRAP_JS}({grants_json}, {{\n{factories}\n}});\n")
}
