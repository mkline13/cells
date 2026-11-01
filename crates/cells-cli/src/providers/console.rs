//! `stdout`: implements `console@1` by writing to the terminal.

use std::io::Write;
use std::sync::Arc;

use cells_runtime::CallFuture;
use cells_runtime::CapabilityError;
use cells_runtime::Provider;
use cells_runtime::Value;

use super::ProviderContext;

pub fn build(ctx: &ProviderContext) -> Result<Arc<dyn Provider>, String> {
  super::only_options(ctx, &[])?;
  Ok(Arc::new(StdoutConsole {
    tag: ctx.app_name.to_string(),
  }))
}

struct StdoutConsole {
  /// Prefixed to every line, so output from different cells can be told
  /// apart.
  tag: String,
}

impl Provider for StdoutConsole {
  fn describe(&self) -> String {
    format!("stdout (lines tagged [{}])", self.tag)
  }

  fn invoke(&self, method: &str, _args: Vec<Value>) -> CallFuture {
    let message = format!("console@1 has no method `{method}`");
    Box::pin(async move { Err(CapabilityError::invalid_argument(message)) })
  }

  fn notify(&self, method: &str, args: Vec<Value>) {
    // console@1 sends write(level, text). Fire-and-forget calls cannot report
    // errors, so malformed calls are ignored.
    if method != "write" {
      return;
    }
    let (Some(Value::String(level)), Some(Value::String(text))) = (args.first(), args.get(1)) else {
      return;
    };
    let label = match level.as_str() {
      "log" | "info" | "debug" => "",
      "warn" => "warning: ",
      "error" => "error: ",
      _ => return,
    };
    let mut out: Box<dyn Write> = if label.is_empty() {
      Box::new(std::io::stdout().lock())
    } else {
      Box::new(std::io::stderr().lock())
    };
    for line in text.split('\n') {
      let _ = writeln!(out, "[{}] {label}{line}", self.tag);
    }
  }
}
