//! Two interchangeable implementations of `kv@1`.
//!
//! - `memory-kv`: lives in memory and disappears when the host exits.
//! - `json-file-kv`: persists to a JSON file. Option `path` (required).
//!
//! Applications cannot tell which one they received. Switching between them
//! is a change to the host config, not to the application (PROPOSAL.md §2.5,
//! Milestone 4).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use cells_runtime::CallFuture;
use cells_runtime::CapabilityError;
use cells_runtime::Provider;
use cells_runtime::Value;

use super::ProviderContext;

/// Keys are limited so a misbehaving application cannot grow one entry
/// without bound.
const MAX_KEY_BYTES: usize = 1024;
const MAX_VALUE_BYTES: usize = 1024 * 1024;

pub fn build_memory(ctx: &ProviderContext) -> Result<Arc<dyn Provider>, String> {
  super::only_options(ctx, &[])?;
  Ok(Arc::new(Kv {
    entries: Mutex::new(BTreeMap::new()),
    file: None,
  }))
}

pub fn build_json_file(ctx: &ProviderContext) -> Result<Arc<dyn Provider>, String> {
  super::only_options(ctx, &["path"])?;
  let path = ctx.base_dir.join(super::string_option(ctx, "path")?);
  let entries = match std::fs::read_to_string(&path) {
    Ok(text) => serde_json::from_str(&text)
      .map_err(|e| format!("{} is not a valid store: {e}", path.display()))?,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
    Err(e) => return Err(format!("could not read {}: {e}", path.display())),
  };
  Ok(Arc::new(Kv {
    entries: Mutex::new(entries),
    file: Some(path),
  }))
}

struct Kv {
  entries: Mutex<BTreeMap<String, Value>>,
  /// Where changes are saved, for `json-file-kv`.
  file: Option<PathBuf>,
}

impl Kv {
  fn call(&self, method: &str, args: &[Value]) -> Result<Value, CapabilityError> {
    let mut entries = self.entries.lock().unwrap();
    match method {
      "get" => {
        let key = key_arg(args)?;
        Ok(entries.get(&key).cloned().unwrap_or(Value::Null))
      }
      "set" => {
        let key = key_arg(args)?;
        let value = args.get(1).cloned().unwrap_or(Value::Null);
        if serde_json::to_vec(&value).map(|v| v.len()).unwrap_or(usize::MAX) > MAX_VALUE_BYTES {
          return Err(CapabilityError::invalid_argument("value is larger than 1 MiB"));
        }
        entries.insert(key, value);
        self.save(&entries)?;
        Ok(Value::Null)
      }
      "delete" => {
        let key = key_arg(args)?;
        let existed = entries.remove(&key).is_some();
        self.save(&entries)?;
        Ok(Value::Bool(existed))
      }
      "keys" => {
        let prefix = super::optional_string_arg(args, 0, "prefix")?.unwrap_or_default();
        Ok(Value::Array(
          entries
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|k| Value::String(k.clone()))
            .collect(),
        ))
      }
      _ => Err(CapabilityError::invalid_argument(format!("kv@1 has no method `{method}`"))),
    }
  }

  fn save(&self, entries: &BTreeMap<String, Value>) -> Result<(), CapabilityError> {
    let Some(path) = &self.file else {
      return Ok(());
    };
    let text = serde_json::to_string_pretty(entries)
      .map_err(|_| CapabilityError::internal("could not serialize the store"))?;
    std::fs::write(path, text).map_err(|_| CapabilityError::unavailable("could not save the store"))
  }
}

fn key_arg(args: &[Value]) -> Result<String, CapabilityError> {
  let key = super::string_arg(args, 0, "key")?;
  if key.is_empty() || key.len() > MAX_KEY_BYTES {
    return Err(CapabilityError::invalid_argument("`key` must be 1 to 1024 bytes long"));
  }
  Ok(key)
}

impl Provider for Kv {
  fn describe(&self) -> String {
    match &self.file {
      Some(path) => format!("json-file-kv (file: {})", path.display()),
      None => "memory-kv (cleared when the host exits)".into(),
    }
  }

  fn invoke(&self, method: &str, args: Vec<Value>) -> CallFuture {
    // Operations are quick, so they run inline. The JSON-file variant does a
    // small blocking write; a production provider would not.
    let result = self.call(method, &args);
    Box::pin(async move { result })
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn stores_json_values() {
    let kv = Kv {
      entries: Mutex::new(BTreeMap::new()),
      file: None,
    };
    kv.call("set", &[json!("a:1"), json!({ "x": [1, 2] })]).unwrap();
    kv.call("set", &[json!("b"), json!(true)]).unwrap();
    assert_eq!(kv.call("get", &[json!("a:1")]).unwrap(), json!({ "x": [1, 2] }));
    assert_eq!(kv.call("get", &[json!("missing")]).unwrap(), Value::Null);
    assert_eq!(kv.call("keys", &[json!("a:")]).unwrap(), json!(["a:1"]));
    assert_eq!(kv.call("delete", &[json!("b")]).unwrap(), json!(true));
    assert_eq!(kv.call("delete", &[json!("b")]).unwrap(), json!(false));
    assert!(kv.call("get", &[json!(1)]).is_err());
  }

  #[test]
  fn json_file_store_persists() {
    let tmp = tempfile::tempdir().unwrap();
    let options = serde_json::Map::from_iter([("path".to_string(), json!("store.json"))]);
    let ctx = ProviderContext {
      app_name: "test",
      base_dir: tmp.path(),
      options: &options,
    };
    let first = build_json_file(&ctx).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
    rt.block_on(first.invoke("set", vec![json!("k"), json!(42)])).unwrap();
    let second = build_json_file(&ctx).unwrap();
    assert_eq!(rt.block_on(second.invoke("get", vec![json!("k")])).unwrap(), json!(42));
  }
}
