//! The adversarial test suite (PROPOSAL.md §7.5): application code tries to
//! reach operations, built-ins, host files, and authority it was not granted.
//!
//! Every test here should keep passing with every change to the runtime.

mod common;

use std::sync::Arc;

use cells_runtime::GrantState;
use common::Echo;
use common::Harness;
use common::echo_contract;

#[test]
fn deno_core_internals_are_unreachable() {
  let h = Harness::new();
  h.file(
    "main.ts",
    r#"
      const g = globalThis as any;
      console.log(typeof g.Deno, typeof g.__bootstrap);
      console.log(Object.getOwnPropertyNames(globalThis).some((n) => n.includes("Deno")));
    "#,
  );
  assert_eq!(h.run_ok(), ["undefined undefined", "false"]);
}

#[test]
fn console_is_absent_unless_granted() {
  let h = Harness::without_console();
  // With no console, report through a thrown error instead.
  h.file("main.ts", "throw new Error(`console is ${typeof console}`);");
  let cells_runtime::CellExit::Failed(message) = h.run().exit else {
    panic!("expected failure");
  };
  assert!(message.contains("console is undefined"), "{message}");
}

#[test]
fn the_global_object_contains_only_expected_names() {
  // This list is the cell's complete global scope besides capabilities.
  // A new entry here is a change to what every application can reach, and
  // must be reviewed as one.
  let h = Harness::new();
  h.file(
    "main.ts",
    "console.log(Object.getOwnPropertyNames(globalThis).sort().join(','));",
  );
  let output = h.run_ok();
  let names: Vec<&str> = output[0].split(',').collect();

  let ecmascript = [
    "AggregateError", "Array", "ArrayBuffer", "AsyncDisposableStack",
    "Atomics", "BigInt", "DisposableStack", "SuppressedError",
    "BigInt64Array", "BigUint64Array", "Boolean", "DataView", "Date", "Error",
    "EvalError", "FinalizationRegistry", "Float16Array", "Float32Array",
    "Float64Array", "Function", "Infinity", "Int16Array", "Int32Array",
    "Int8Array", "Intl", "Iterator", "JSON", "Map", "Math", "NaN", "Number",
    "Object", "Promise", "Proxy", "RangeError", "ReferenceError", "Reflect",
    "RegExp", "Set", "SharedArrayBuffer", "String", "Symbol", "SyntaxError",
    "TypeError", "URIError", "Uint16Array", "Uint32Array", "Uint8Array",
    "Uint8ClampedArray", "WeakMap", "WeakRef", "WeakSet", "WebAssembly",
    "decodeURI", "decodeURIComponent", "encodeURI", "encodeURIComponent",
    "escape", "eval", "globalThis", "isFinite", "isNaN", "parseFloat",
    "parseInt", "undefined", "unescape",
    // `Temporal.Now` reads the clock, like `Date.now()`. Ambient time is
    // accepted under the initial threat model (PROPOSAL.md §7.5).
    "Temporal",
  ];
  let baseline = [
    "TextDecoder", "TextEncoder", "URL", "URLSearchParams", "atob", "btoa",
    "clearInterval", "clearTimeout", "queueMicrotask", "setInterval",
    "setTimeout", "structuredClone",
  ];
  let capabilities = ["console"];

  let unexpected: Vec<&str> = names
    .iter()
    .copied()
    .filter(|n| !ecmascript.contains(n) && !baseline.contains(n) && !capabilities.contains(n))
    .collect();
  // V8 upgrades add new ECMAScript built-ins. They fail this test on purpose,
  // so each one is reviewed before it is added to the list above.
  assert!(unexpected.is_empty(), "unexpected globals: {unexpected:?}");
  for name in baseline {
    assert!(names.contains(&name), "missing baseline global {name}");
  }
}

#[test]
fn imports_cannot_leave_the_source_directory() {
  let h = Harness::new();
  // A file next to the application directory that it must not be able to
  // import.
  let outside = h.path().parent().unwrap().join(format!(
    "outside-{}.ts",
    h.path().file_name().unwrap().to_string_lossy()
  ));
  std::fs::write(&outside, "export const secret = 'leaked';").unwrap();
  let outside_name = outside.file_name().unwrap().to_string_lossy().to_string();
  h.file(
    "main.ts",
    &format!(
      r#"
        const attempts = [
          "../{outside_name}",
          "../../../../../../{outside_name}",
          "/{outside_name}",
          "file://{}",
          "file:///etc/passwd",
          "https://example.com/mod.ts",
          "ext:core/ops",
          "ext:core/mod.js",
          "node:fs",
          "lodash",
          "./link.ts",
        ];
        for (const specifier of attempts) {{
          try {{
            const mod = await import(specifier);
            console.log("LOADED", specifier, JSON.stringify(mod));
          }} catch {{
            console.log("blocked");
          }}
        }}
      "#,
      outside.display()
    ),
  );
  // A symlink inside the application directory that points outside it.
  #[cfg(unix)]
  std::os::unix::fs::symlink(&outside, h.path().join("link.ts")).unwrap();
  #[cfg(not(unix))]
  h.file("link.ts", "throw new Error('no symlinks on this platform');");

  let output = h.run_ok();
  let _ = std::fs::remove_file(&outside);
  assert_eq!(output.len(), 11);
  assert!(output.iter().all(|line| line == "blocked"), "{output:#?}");
}

#[test]
fn import_meta_resolve_cannot_produce_host_urls() {
  let h = Harness::new();
  h.file(
    "main.ts",
    r#"
      console.log(import.meta.url);
      console.log(import.meta.resolve("./other.ts"));
      try { import.meta.resolve("file:///etc/passwd"); console.log("resolved"); }
      catch { console.log("blocked"); }
    "#,
  );
  assert_eq!(h.run_ok(), ["cell:///main.ts", "cell:///other.ts", "blocked"]);
}

#[test]
fn stack_traces_do_not_reveal_host_paths() {
  let h = Harness::new();
  h.file("main.ts", "console.log(new Error('where am I').stack);");
  let output = h.run_ok().join("\n");
  let host_path = h.path().to_string_lossy().to_string();
  assert!(!output.contains(&host_path), "{output}");
  assert!(output.contains("cell:///main.ts"), "{output}");
}

#[test]
fn forged_capability_objects_carry_no_authority() {
  let mut h = Harness::new();
  // `secret` exists in the host, but this cell's grant for it is denied.
  h.grant("secret", echo_contract(), GrantState::Denied);
  h.file(
    "main.ts",
    r#"
      const g = globalThis as any;
      // Copying the methods of a real capability does not copy authority it
      // lacks, and objects that merely look like capabilities do nothing.
      const forged = { say: g.secret.say };
      for (const attempt of [() => forged.say("x"), () => g.secret.say.call({}, "x")]) {
        try { await attempt(); console.log("ALLOWED"); }
        catch (err: any) { console.log(err.kind); }
      }
      // The global cannot be replaced.
      try { g.secret = { say: () => "fake" }; } catch { console.log("read-only"); }
    "#,
  );
  assert_eq!(
    h.run_ok(),
    ["permission-denied", "permission-denied", "read-only"]
  );
}

#[test]
fn handles_from_other_cells_carry_no_authority() {
  // Cell A holds a grant. Cell B holds nothing. Even if B learned A's handle
  // number, B's calls are looked up against B's own (empty) grants. This is
  // checked directly against the dispatcher because application code has no
  // way to name a handle at all.
  let mut a = Harness::new();
  let handle = a.grant("echo", echo_contract(), GrantState::Granted(Arc::new(Echo)));
  let b_cell = cells_runtime::CellId::next();
  let call = cells_runtime::Call {
    cell: b_cell,
    handle,
    method: "say".into(),
    args: vec![],
  };
  use cells_runtime::Dispatcher;
  let result = common::supervisor().block_on(a.table.invoke(call));
  assert_eq!(result.unwrap_err().kind, cells_runtime::ErrorKind::PermissionDenied);
}

#[test]
fn eval_runs_inside_the_cell_with_no_extra_authority() {
  let h = Harness::new();
  h.file(
    "main.ts",
    r#"
      console.log(eval("typeof Deno"), new Function("return typeof __bootstrap")());
      console.log(eval("1 + 1"));
    "#,
  );
  assert_eq!(h.run_ok(), ["undefined undefined", "2"]);
}

#[test]
fn modifying_built_ins_does_not_break_capability_dispatch() {
  // Application code can change shared prototypes. That only affects the
  // cell itself, and the bootstrap script captured what it needs earlier.
  let mut h = Harness::new();
  h.grant("echo", echo_contract(), GrantState::Granted(Arc::new(Echo)));
  h.file(
    "main.ts",
    r#"
      (Array as any).isArray = () => false;
      Object.freeze = ((o: any) => o) as any;
      const reply = await (globalThis as any).echo.say("still works");
      console.log(reply.args[0]);
    "#,
  );
  assert_eq!(h.run_ok(), ["still works"]);
}
