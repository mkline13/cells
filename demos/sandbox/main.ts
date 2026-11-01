// sandbox: tries every obvious way out of the cell, and reports each one.
//
// Run it with:
//
//   cargo run -p cells-cli -- run demos/sandbox
//
// This application is granted exactly one capability: `console`. Each check
// below tries to get something more. Every line should say "blocked".
//
// The same attempts, and more, run as automated tests in
// crates/cells-runtime/tests/escape.rs.
//
// Keep in mind what this does and does not show. The runtime removes ambient
// authority and checks every capability call, but cells still share one
// operating-system process. A bug in V8 itself could break out. Stronger
// isolation comes in later stages (PROPOSAL.md §7.8).

// Marks this file as an ES module, which TypeScript requires for top-level
// await. Every file the runtime loads is a module either way.
export {};

let leaks = 0;

function report(attempt: string, blocked: boolean, detail = "") {
  if (!blocked) leaks += 1;
  console.log(`${blocked ? "blocked" : "LEAKED "}  ${attempt}${detail ? ` (${detail})` : ""}`);
}

const g = globalThis as Record<string, unknown>;

// --- 1. Runtime internals ---------------------------------------------------------
// deno_core exposes its operations through `Deno.core.ops`. The runtime's
// bootstrap script removes `Deno` and `__bootstrap` before this code runs.

report("reach Deno.core.ops", typeof g.Deno === "undefined");
report("reach __bootstrap", typeof g.__bootstrap === "undefined");
report("find them through eval", eval("typeof Deno") === "undefined");

// --- 2. Ambient APIs from other runtimes ----------------------------------------------
// None of these exist. The network, environment, and processes are only
// reachable through capabilities, and this app was granted none of them.

for (const name of ["fetch", "process", "require", "Bun", "WebSocket", "localStorage"]) {
  report(`use ${name}`, !(name in g));
}

// --- 3. Capabilities that were never granted ---------------------------------------------
// A capability the manifest does not declare is simply absent.

report("use an undeclared `files` capability", typeof g.files === "undefined");

// --- 4. Imports that leave the application ------------------------------------------------
// The module loader only serves files inside this directory, as cell:/// URLs.
// `../sandbox-secret.ts` exists on disk, one directory up.

const imports = [
  "../sandbox-secret.ts",
  "file:///etc/passwd",
  "https://example.com/module.ts",
  "ext:core/ops",
  "node:fs",
];
for (const specifier of imports) {
  try {
    await import(specifier);
    report(`import ${specifier}`, false);
  } catch (err) {
    report(`import ${specifier}`, true, (err as Error).message.split("\n")[0]);
  }
}

// --- 5. Forging a capability ------------------------------------------------------------------
// A capability's authority lives in the host's grant table, not in the
// JavaScript object. Building a lookalike object does nothing, and the real
// global cannot be replaced.

try {
  g.console = { log: () => {} };
  report("replace the console global", false);
} catch {
  report("replace the console global", true, "it is read-only");
}

try {
  (console as unknown as Record<string, unknown>).log = () => {};
  report("modify the console object", false);
} catch {
  report("modify the console object", true, "it is frozen");
}

// --- 6. Host details ---------------------------------------------------------------------------
// Stack traces and module URLs use cell:/// paths, so the host's directory
// layout is not revealed.

const stack = new Error("probe").stack ?? "";
report("learn the host path from a stack trace", stack.includes("cell:///") && !stack.includes("file:"));

console.log(leaks === 0 ? "all attempts were blocked" : `${leaks} attempt(s) leaked!`);
