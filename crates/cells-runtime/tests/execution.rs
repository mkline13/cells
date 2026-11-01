//! Milestone 1 and 3: TypeScript execution, modules, errors, and cell
//! lifecycle.

mod common;

use std::time::Duration;
use std::time::Instant;

use cells_runtime::CellExit;
use common::Harness;

#[test]
fn runs_typescript_with_relative_imports_and_top_level_await() {
  let h = Harness::new();
  h.file(
    "main.ts",
    r#"
      import { greet, type Greeting } from "./lib/greeting.ts";
      import data from "./data.json" with { type: "json" };

      const g: Greeting = await Promise.resolve(greet("world"));
      console.log(g.text, data.answer);
    "#,
  );
  h.file(
    "lib/greeting.ts",
    r#"
      export interface Greeting { text: string }
      export function greet(name: string): Greeting {
        return { text: `hello, ${name}` };
      }
    "#,
  );
  h.file("data.json", r#"{ "answer": 42 }"#);
  assert_eq!(h.run_ok(), ["hello, world 42"]);
}

#[test]
fn reports_uncaught_exceptions_with_typescript_locations() {
  let h = Harness::new();
  h.file(
    "main.ts",
    "type Unused = { a: string };\n\nfunction fail(): never {\n  throw new Error(\"boom\");\n}\nfail();\n",
  );
  let CellExit::Failed(message) = h.run().exit else {
    panic!("expected failure");
  };
  assert!(message.contains("boom"), "{message}");
  // Source maps point at the original TypeScript line, not the emitted one.
  assert!(message.contains("cell:///main.ts:4"), "{message}");
}

#[test]
fn reports_unhandled_rejections() {
  let h = Harness::new();
  h.file("main.ts", "Promise.reject(new Error(\"nobody caught me\"));");
  let CellExit::Failed(message) = h.run().exit else {
    panic!("expected failure");
  };
  assert!(message.contains("nobody caught me"), "{message}");
}

#[test]
fn reports_syntax_errors() {
  let h = Harness::new();
  h.file("main.ts", "const = ;");
  assert!(matches!(h.run().exit, CellExit::Failed(_)));
}

#[test]
fn terminates_infinite_loops() {
  let h = Harness::new();
  h.file("main.ts", "console.log('spinning'); while (true) {}");
  let cell = h.spawn();
  std::thread::sleep(Duration::from_millis(200));
  cell.terminate();
  let report = cell.join();
  assert_eq!(report.exit, CellExit::Terminated);
  assert_eq!(h.output(), ["spinning"]);
}

#[test]
fn terminates_idle_cells() {
  // The cell is waiting on a timer, so no JavaScript is running for V8 to
  // interrupt. Termination must still stop it promptly.
  let h = Harness::new();
  h.file("main.ts", "setTimeout(() => console.log('too late'), 60_000);");
  let cell = h.spawn();
  std::thread::sleep(Duration::from_millis(100));
  let started = Instant::now();
  cell.terminate();
  assert_eq!(cell.join().exit, CellExit::Terminated);
  assert!(started.elapsed() < Duration::from_secs(5));
  assert!(h.output().is_empty());
}

#[test]
fn stops_cells_that_exhaust_their_heap() {
  let mut h = Harness::new();
  h.heap_limit = Some(32 * 1024 * 1024);
  h.file(
    "main.ts",
    "const hoard: string[][] = []; while (true) { hoard.push(new Array(100_000).fill('x')); }",
  );
  assert_eq!(h.run().exit, CellExit::OutOfMemory);

  // The process survives, and other cells still run.
  let other = Harness::new();
  other.file("main.ts", "console.log('still alive');");
  assert_eq!(other.run_ok(), ["still alive"]);
}

#[test]
fn cells_have_separate_global_state() {
  let a = Harness::new();
  a.file("main.ts", "(globalThis as any).shared = 'from a'; console.log('a set it');");
  let b = Harness::new();
  b.file(
    "main.ts",
    "await new Promise((r) => setTimeout(r, 50)); console.log(String((globalThis as any).shared));",
  );
  let (ca, cb) = (a.spawn(), b.spawn());
  assert_eq!(ca.join().exit, CellExit::Completed);
  assert_eq!(cb.join().exit, CellExit::Completed);
  assert_eq!(a.output(), ["a set it"]);
  assert_eq!(b.output(), ["undefined"]);
}

#[test]
fn rejects_invalid_grant_names_before_starting() {
  let mut h = Harness::without_console();
  h.grant(
    "fetch",
    cells_runtime::standard::kv(),
    cells_runtime::GrantState::Denied,
  );
  h.file("main.ts", "");
  assert!(h.engine.spawn(h.config()).is_err());
}

#[test]
fn rejects_grant_names_that_collide_with_globals() {
  let mut h = Harness::without_console();
  h.grant("Array", cells_runtime::standard::kv(), cells_runtime::GrantState::Denied);
  h.file("main.ts", "");
  let err = h.engine.spawn(h.config()).err().expect("spawn fails");
  assert!(err.to_string().contains("collides"), "{err}");
}

#[test]
fn console_output_is_complete_and_ordered_when_join_returns() {
  let h = Harness::new();
  h.file("main.ts", "for (let i = 0; i < 500; i++) console.log(String(i));");
  let report = h.run();
  assert_eq!(report.exit, CellExit::Completed);
  assert_eq!(report.dropped_notifications, 0);
  let expected: Vec<String> = (0..500).map(|i| i.to_string()).collect();
  assert_eq!(h.output(), expected);
}
