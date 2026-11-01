//! Turns an application's requirements and the host's configuration into
//! grants (PROPOSAL.md §4.6: requirements, policy, enforcement).
//!
//! - The manifest says what the application needs.
//! - The host config says what this host permits and how.
//! - The resulting grant states are what the runtime enforces.

use std::sync::Arc;

use cells_runtime::Contract;
use cells_runtime::Provider;
use cells_runtime::Resolve;
use cells_runtime::ResolveFuture;
use cells_runtime::ResolveRequest;
use cells_runtime::standard;

use crate::host_config::Decision;
use crate::host_config::HostConfig;
use crate::manifest::Manifest;
use crate::providers;
use crate::providers::ProviderContext;

/// The contracts this host understands.
fn contract(id: &str) -> Option<Contract> {
  match id {
    "console@1" => Some(standard::console()),
    "files@1" => Some(standard::files()),
    "kv@1" => Some(standard::kv()),
    _ => None,
  }
}

/// The host's decision for one declared capability.
pub struct PlannedGrant {
  pub name: String,
  pub contract: Contract,
  pub required: bool,
  pub outcome: Outcome,
}

pub enum Outcome {
  Grant(Arc<dyn Provider>),
  /// Decided on first use by asking on the terminal.
  Ask(Arc<dyn Provider>),
  Deny(String),
}

/// Decides every capability in the manifest. Fails if a required capability
/// cannot be granted before the cell starts (§5.4).
pub fn plan(manifest: &Manifest, host: &HostConfig) -> Result<Vec<PlannedGrant>, String> {
  for name in host.grants.keys() {
    if !manifest.capabilities.contains_key(name) {
      return Err(format!(
        "the host config grants `{name}`, which {} does not declare",
        manifest.name
      ));
    }
  }

  let mut planned = Vec::new();
  for (name, decl) in &manifest.capabilities {
    let refuse = |reason: String| -> Result<Vec<PlannedGrant>, String> {
      Err(format!("cannot start {}: required capability `{name}` {reason}", manifest.name))
    };

    let Some(contract) = contract(&decl.contract) else {
      if decl.required {
        return refuse(format!("uses {}, which this host does not support", decl.contract));
      }
      // The host cannot provide it, so it is denied: the global is still
      // present, and its operations are rejected (§6.3). The runtime still
      // needs the contract's method list to build the global, so an
      // unsupported optional contract gets an empty one.
      let contract = Contract::new(&decl.contract).map_err(|e| e.to_string())?;
      planned.push(PlannedGrant {
        name: name.clone(),
        contract,
        required: false,
        outcome: Outcome::Deny(format!("this host does not support {}", decl.contract)),
      });
      continue;
    };

    let Some(config) = host.grants.get(name) else {
      if decl.required {
        return refuse("is not configured in the host config".into());
      }
      planned.push(PlannedGrant {
        name: name.clone(),
        contract,
        required: false,
        outcome: Outcome::Deny("not configured by the host".into()),
      });
      continue;
    };

    let kind = providers::find(&config.provider)
      .ok_or_else(|| format!("`{name}`: unknown provider `{}`", config.provider))?;
    if kind.contract != decl.contract {
      return Err(format!(
        "`{name}`: provider `{}` implements {}, but {} expects {}",
        kind.name, kind.contract, manifest.name, decl.contract
      ));
    }

    let outcome = match config.decision {
      Decision::Deny if decl.required => return refuse("is denied by the host config".into()),
      Decision::Ask if decl.required => {
        return refuse("must be granted before the cell starts, so it cannot use \"ask\"".into());
      }
      Decision::Deny => Outcome::Deny("denied by the host config".into()),
      decision => {
        let ctx = ProviderContext {
          app_name: &manifest.name,
          base_dir: &host.base_dir,
          options: &config.options,
        };
        let provider = (kind.build)(&ctx).map_err(|e| format!("`{name}` ({}): {e}", kind.name))?;
        if decision == Decision::Ask {
          Outcome::Ask(provider)
        } else {
          Outcome::Grant(provider)
        }
      }
    };
    planned.push(PlannedGrant {
      name: name.clone(),
      contract,
      required: decl.required,
      outcome,
    });
  }
  Ok(planned)
}

/// Asks on the terminal whether to grant a capability, the first time the
/// application uses it.
pub struct AskOnTerminal {
  pub app_name: String,
  pub provider: Arc<dyn Provider>,
}

/// Only one question is shown at a time.
static PROMPT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

impl Resolve for AskOnTerminal {
  fn resolve(&self, request: ResolveRequest) -> ResolveFuture {
    let question = format!(
      "[cells] {} wants to use `{}` ({} via {}). Allow? [y/N] ",
      self.app_name,
      request.grant_name,
      request.contract,
      self.provider.describe()
    );
    let provider = self.provider.clone();
    Box::pin(async move {
      let _guard = PROMPT_LOCK.lock().await;
      let allowed = tokio::task::spawn_blocking(move || {
        use std::io::Write;
        eprint!("{question}");
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        // End of input, such as a closed stdin, counts as "no".
        let read = std::io::stdin().read_line(&mut answer).is_ok();
        // A typed answer ends the line; a piped one does not.
        if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
          eprintln!("{}", answer.trim());
        }
        read && matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
      })
      .await
      .unwrap_or(false);
      allowed.then_some(provider)
    })
  }
}
