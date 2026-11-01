# Cells

Cells is a capability-oriented application platform. Applications are written in TypeScript and run in **cells**: isolated environments that can only affect the outside world through capabilities their host explicitly grants. See [PROPOSAL.md](PROPOSAL.md) for the full design.

This repository currently contains the **runtime MVP** (PROPOSAL.md §8.2–§8.4).

## Try it

You need a recent stable Rust toolchain (tested with 1.97). The first build downloads a prebuilt V8 library and takes a few minutes.

```bash
cargo run -p cells-cli -- run demos/hello
```

Then work through the other demos. [demos/README.md](demos/README.md) explains what each one shows.

## Layout

| Path | What it is |
|---|---|
| `crates/cells-runtime` | The runtime library: cells, the module loader, capability dispatch, and the in-memory grant table. |
| `crates/cells-cli` | `cells`, a command-line test host. It reads an app's manifest and a host config, supplies providers, and runs the app. |
| `demos/` | Small, heavily commented applications. |
| `types/` | TypeScript declarations for the baseline globals and the MVP's capability contracts. |
| `docs/deferred.md` | Temporary decisions that must change at a later stage. Each has matching `TODO(<stage>)` comments in the code. |

## How it works

An application has a **manifest** (`cell.json`) that names the capabilities it needs:

```json
"capabilities": {
  "console": { "contract": "console@1", "required": true },
  "notes":   { "contract": "files@1",   "required": true },
  "cache":   { "contract": "kv@1",      "required": false }
}
```

Each key becomes a global in the application. The key `notes` becomes `await notes.readText("a.md")`.

The host decides what each name actually gets. In the CLI host, that's a **host config** (`host.json`):

```json
"grants": {
  "console": { "provider": "stdout" },
  "notes":   { "provider": "local-files", "root": "data/notes" },
  "cache":   { "provider": "memory-kv", "decision": "ask" }
}
```

At run time:

1. The runtime starts a cell: a fresh V8 isolate on its own thread. There is no `Deno`, `process`, `fetch`, or filesystem access.
2. It installs the baseline globals (timers, `URL`, `TextEncoder`, ...) and one global per declared capability. Each capability global only wraps an opaque grant handle.
3. A call like `notes.readText(...)` sends the handle, method, and arguments to the **dispatcher** on a separate thread. The dispatcher looks up the handle in the calling cell's own grants and rejects the call unless that grant is valid.
4. The **provider** behind the grant performs the operation and enforces the grant's scope. For example, it refuses paths outside `data/notes`.

Grants can be `granted`, `denied`, or `unresolved`. An unresolved grant is decided by the host the first time it's used, for example by asking the user. Denied capabilities are still present as globals, but their calls reject with an error whose `kind` is `"permission-denied"`.

## Status and limitations

This is Stage 1 of the isolation plan (PROPOSAL.md §7.8). It removes ambient authority and checks every capability call, but every cell runs in the host's process. It is **not** a security boundary against malicious code: a vulnerability in V8 or in host code affects every cell.

Other current limitations:

- Only TypeScript, JavaScript, and JSON modules from the application's own directory can be imported. There are no npm or remote imports.
- Types are stripped, not checked. Use an editor or `tsc` for type checking; each demo has a `tsconfig.json`.
- `TextDecoder` is UTF-8 only and does not stream. `structuredClone` does not support `transfer`.
- Heap limits cover the V8 heap but not `ArrayBuffer` memory.

`docs/deferred.md` lists the decisions that will change in later stages.

## Tests

```bash
cargo test
```

The suite includes an adversarial set of tests (`crates/cells-runtime/tests/escape.rs`). It tries to reach runtime internals, host files, and authority the cell was not granted. It also checks that the global scope contains only reviewed names.
