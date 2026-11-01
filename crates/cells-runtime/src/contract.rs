//! Capability contracts.
//!
//! A contract is the agreed shape of a capability: its identifier and
//! version (`files@1`), its methods, and whether each method returns a
//! result (PROPOSAL.md §6.2, §6.3). It says nothing about how the capability
//! is implemented; that is the provider's job.

use std::fmt;
use std::sync::Arc;

use serde::Serialize;

/// Whether a method returns a result to the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MethodKind {
  /// Asynchronous: returns a `Promise` that resolves with the result or
  /// rejects with a capability error.
  Invoke,
  /// Fire-and-forget: returns nothing and reports no errors (§6.6).
  Notify,
}

/// One method of a contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Method {
  pub name: String,
  pub kind: MethodKind,
}

/// A capability contract.
#[derive(Clone, Debug)]
pub struct Contract {
  id: String,
  methods: Vec<Method>,
  js_factory: Option<Arc<str>>,
}

/// An invalid contract definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError(String);

impl fmt::Display for ContractError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl std::error::Error for ContractError {}

impl Contract {
  /// Creates a contract with no methods.
  ///
  /// `id` must have the form `name@version`, for example `files@1`.
  pub fn new(id: impl Into<String>) -> Result<Self, ContractError> {
    let id = id.into();
    let valid = match id.split_once('@') {
      Some((name, version)) => {
        let mut chars = name.chars();
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
          && chars.all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || "-_.".contains(c)
          })
          && !version.is_empty()
          && version.chars().all(|c| c.is_ascii_digit())
      }
      None => false,
    };
    if !valid {
      return Err(ContractError(format!(
        "invalid contract id {id:?}: expected the form name@version, such as files@1"
      )));
    }
    Ok(Self {
      id,
      methods: Vec::new(),
      js_factory: None,
    })
  }

  /// Adds an asynchronous method.
  pub fn invoke(mut self, name: &str) -> Self {
    self.methods.push(Method {
      name: name.to_string(),
      kind: MethodKind::Invoke,
    });
    self
  }

  /// Adds a fire-and-forget method.
  pub fn notify(mut self, name: &str) -> Self {
    self.methods.push(Method {
      name: name.to_string(),
      kind: MethodKind::Notify,
    });
    self
  }

  /// Supplies custom JavaScript that builds the application-facing object.
  ///
  /// By default, each method becomes a function that forwards its arguments
  /// unchanged. A factory lets a contract present a different shape to
  /// applications. The `console@1` contract uses one to format its arguments
  /// into text inside the cell, then sends a single `write` message.
  ///
  /// `source` must be a JavaScript function expression with the signature
  /// `(invoke, notify) => object`, where `invoke(method, args)` returns a
  /// promise and `notify(method, args)` returns nothing. The factory only
  /// ever receives functions bound to its own grant handle, so it cannot
  /// reach any other grant.
  ///
  /// This is trusted host code: it runs before application code, while
  /// internal runtime objects are still reachable.
  pub fn with_js_factory(mut self, source: impl Into<Arc<str>>) -> Self {
    self.js_factory = Some(source.into());
    self
  }

  /// The contract identifier, for example `files@1`.
  pub fn id(&self) -> &str {
    &self.id
  }

  /// The contract's methods.
  pub fn methods(&self) -> &[Method] {
    &self.methods
  }

  /// Looks up a method by name.
  pub fn method(&self, name: &str) -> Option<&Method> {
    self.methods.iter().find(|m| m.name == name)
  }

  pub(crate) fn js_factory(&self) -> Option<&str> {
    self.js_factory.as_deref()
  }
}

/// Contracts used by the runtime MVP and its demo host.
///
/// TypeScript declarations for these are in `types/contracts.d.ts`.
pub mod standard {
  use super::Contract;

  /// `console@1`: the standard `console` methods.
  ///
  /// Arguments are formatted into a string inside the cell. The provider
  /// receives a single fire-and-forget `write(level, text)` call, where
  /// `level` is one of `log`, `info`, `warn`, `error`, or `debug`.
  pub fn console() -> Contract {
    Contract::new("console@1")
      .expect("valid contract id")
      .notify("write")
      .with_js_factory(include_str!("js/console_factory.js"))
  }

  /// `files@1`: text files within the grant's directory.
  ///
  /// - `readText(path): Promise<string>`
  /// - `writeText(path, contents): Promise<void>`
  /// - `list(path = "."): Promise<string[]>` (directories end in `/`)
  /// - `remove(path): Promise<void>`
  pub fn files() -> Contract {
    Contract::new("files@1")
      .expect("valid contract id")
      .invoke("readText")
      .invoke("writeText")
      .invoke("list")
      .invoke("remove")
  }

  /// `kv@1`: a key-value store of JSON values.
  ///
  /// - `get(key): Promise<unknown | null>`
  /// - `set(key, value): Promise<void>`
  /// - `delete(key): Promise<boolean>`
  /// - `keys(prefix = ""): Promise<string[]>`
  pub fn kv() -> Contract {
    Contract::new("kv@1")
      .expect("valid contract id")
      .invoke("get")
      .invoke("set")
      .invoke("delete")
      .invoke("keys")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn contract_ids_must_have_a_version() {
    assert!(Contract::new("files@1").is_ok());
    assert!(Contract::new("my-files.v2@10").is_ok());
    assert!(Contract::new("files").is_err());
    assert!(Contract::new("files@").is_err());
    assert!(Contract::new("Files@1").is_err());
    assert!(Contract::new("files@one").is_err());
  }
}
