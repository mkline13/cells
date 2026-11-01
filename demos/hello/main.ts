// hello: the smallest useful application.
//
// Run it with:
//
//   cargo run -p cells-cli -- run demos/hello
//
// What happens when you run it:
//
// 1. The host (the `cells` command) reads `cell.json`, the application's
//    manifest. It declares one capability, named `console`, using the
//    `console@1` contract.
// 2. The host reads `host.json`, its own configuration. It decides to grant
//    `console` and back it with the `stdout` provider.
// 3. The runtime creates a *cell*: a fresh V8 isolate on its own thread. It
//    installs the baseline globals (timers, URL, TextEncoder, ...) and one
//    global per declared capability, then runs this file.
//
// There is no `Deno`, no `process`, no `require`, and no `fetch` here. The
// only way this code can affect the outside world is through `console`.

// Relative imports work. They can only reach files inside this application's
// directory, which the runtime exposes as `cell:///`.
import { greet, type Greeting } from "./greeting.ts";

// TypeScript syntax is stripped before the code runs. Types are not checked
// at runtime, so use your editor (or `tsc`) for that.
const greeting: Greeting = greet("world");

// `console` is a capability, not a built-in. If cell.json did not declare
// it, or host.json did not grant it, this line would throw a ReferenceError.
console.log(greeting.text);

// Every module has a `cell:///` URL. Host paths never appear in the cell.
console.log("this module is", import.meta.url);

// Top-level await works. Timers are part of the baseline: they reach nothing
// outside the cell.
const started = Date.now();
await new Promise((resolve) => setTimeout(resolve, 250));
console.log(`waited ${Date.now() - started}ms`);

// A few other baseline globals.
const url = new URL("/docs?page=2", "https://example.com");
console.log("URL parsing:", url.pathname, url.searchParams.get("page"));
console.log("UTF-8 bytes in ✓:", new TextEncoder().encode("✓").length);
console.log("structuredClone:", structuredClone({ nested: [1, 2, 3] }));

// When this module finishes and no timers or capability calls are pending,
// the cell's event loop runs out of work and the cell completes.
console.log("done");
