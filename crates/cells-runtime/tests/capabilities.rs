//! Milestone 2: capability injection, dispatch, and grant states.

mod common;

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use cells_runtime::GrantState;
use cells_runtime::Provider;
use cells_runtime::Resolve;
use cells_runtime::ResolveFuture;
use cells_runtime::ResolveRequest;
use common::Echo;
use common::Harness;
use common::echo_contract;

#[test]
fn granted_capabilities_dispatch_to_providers() {
  let mut h = Harness::new();
  h.grant("echo", echo_contract(), GrantState::Granted(Arc::new(Echo)));
  h.file(
    "main.ts",
    r#"
      const reply = await (globalThis as any).echo.say("hi", { n: 1 });
      console.log(JSON.stringify(reply));
    "#,
  );
  assert_eq!(h.run_ok(), [r#"{"method":"say","args":["hi",{"n":1}]}"#]);
}

#[test]
fn console_formats_values_inside_the_cell() {
  let h = Harness::new();
  h.file(
    "main.ts",
    r#"
      console.log("text", 1, true, null, undefined, [1, "a"], { b: { c: 2 } });
      console.warn(new Map([["k", 1]]));
      const circular: any = {}; circular.self = circular;
      console.error(circular);
    "#,
  );
  assert_eq!(
    h.run_ok(),
    [
      r#"text 1 true null undefined [ 1, "a" ] { b: { c: 2 } }"#,
      r#"warn: Map(1) { "k" => 1 }"#,
      "error: { self: [Circular] }",
    ]
  );
}

#[test]
fn denied_capabilities_are_present_but_reject() {
  let mut h = Harness::new();
  h.grant("echo", echo_contract(), GrantState::Denied);
  h.file(
    "main.ts",
    r#"
      const echo = (globalThis as any).echo;
      console.log(typeof echo);
      try {
        await echo.say("hi");
      } catch (err: any) {
        console.log(err.name, err.kind);
      }
    "#,
  );
  assert_eq!(h.run_ok(), ["object", "CapabilityError permission-denied"]);
}

#[test]
fn undeclared_capabilities_are_absent() {
  let h = Harness::new();
  h.file("main.ts", "console.log(typeof (globalThis as any).echo);");
  assert_eq!(h.run_ok(), ["undefined"]);
}

#[test]
fn several_grants_of_one_contract_are_independent() {
  struct Tagged(&'static str);
  impl Provider for Tagged {
    fn describe(&self) -> String {
      self.0.into()
    }
    fn invoke(&self, _: &str, _: Vec<serde_json::Value>) -> cells_runtime::CallFuture {
      let tag = self.0;
      Box::pin(async move { Ok(serde_json::json!(tag)) })
    }
  }
  let mut h = Harness::new();
  h.grant("left", echo_contract(), GrantState::Granted(Arc::new(Tagged("L"))));
  h.grant("right", echo_contract(), GrantState::Granted(Arc::new(Tagged("R"))));
  h.file(
    "main.ts",
    "const g = globalThis as any; console.log(await g.left.say(), await g.right.say());",
  );
  assert_eq!(h.run_ok(), ["L R"]);
}

struct Approve {
  asked: AtomicUsize,
}

impl Resolve for Approve {
  fn resolve(&self, request: ResolveRequest) -> ResolveFuture {
    assert_eq!(request.grant_name, "echo");
    self.asked.fetch_add(1, Ordering::SeqCst);
    Box::pin(async {
      let provider: Arc<dyn Provider> = Arc::new(Echo);
      Some(provider)
    })
  }
}

#[test]
fn unresolved_grants_are_decided_on_first_use() {
  let resolver = Arc::new(Approve {
    asked: AtomicUsize::new(0),
  });
  let mut h = Harness::new();
  h.grant("echo", echo_contract(), GrantState::Unresolved(resolver.clone()));
  h.file(
    "main.ts",
    r#"
      const echo = (globalThis as any).echo;
      console.log("before");
      const [a, b] = await Promise.all([echo.say(1), echo.say(2)]);
      console.log(a.args[0], b.args[0]);
    "#,
  );
  assert_eq!(h.run_ok(), ["before", "1 2"]);
  assert_eq!(resolver.asked.load(Ordering::SeqCst), 1);
}

#[test]
fn revoked_grants_reject_later_calls() {
  let mut h = Harness::new();
  let handle = h.grant("echo", echo_contract(), GrantState::Granted(Arc::new(Echo)));
  h.file(
    "main.ts",
    r#"
      const echo = (globalThis as any).echo;
      await echo.say("first");
      console.log("first ok");
      // Give the host time to revoke the grant.
      await new Promise((r) => setTimeout(r, 300));
      try { await echo.say("second"); } catch (err: any) { console.log(err.kind); }
    "#,
  );
  let cell = h.spawn();
  std::thread::sleep(std::time::Duration::from_millis(150));
  assert!(h.table.revoke(h.cell, handle));
  assert_eq!(cell.join().exit, cells_runtime::CellExit::Completed);
  assert_eq!(h.output(), ["first ok", "permission-denied"]);
}

#[test]
fn capability_errors_from_providers_reach_the_application() {
  struct Missing;
  impl Provider for Missing {
    fn describe(&self) -> String {
      "missing".into()
    }
    fn invoke(&self, _: &str, _: Vec<serde_json::Value>) -> cells_runtime::CallFuture {
      Box::pin(async { Err(cells_runtime::CapabilityError::not_found("no such thing")) })
    }
  }
  let mut h = Harness::new();
  h.grant("echo", echo_contract(), GrantState::Granted(Arc::new(Missing)));
  h.file(
    "main.ts",
    "try { await (globalThis as any).echo.say(); } catch (e: any) { console.log(e.kind, e.message); }",
  );
  assert_eq!(h.run_ok(), ["not-found no such thing"]);
}

#[test]
fn unknown_methods_are_rejected_by_the_dispatcher() {
  let mut h = Harness::new();
  h.grant("echo", echo_contract(), GrantState::Granted(Arc::new(Echo)));
  h.file(
    "main.ts",
    r#"
      // The capability object is frozen, so this adds nothing...
      try { (globalThis as any).echo.extra = () => {}; } catch { console.log("frozen"); }
      console.log(typeof (globalThis as any).echo.extra);
    "#,
  );
  assert_eq!(h.run_ok(), ["frozen", "undefined"]);
}
