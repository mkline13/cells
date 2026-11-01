# Demos

Each demo is a small application with three parts:

- `cell.json`: the **manifest**. It's the application's own description of who it is and which capabilities it needs.
- `host.json`: the **host config**. It's the host's decision about what each capability actually gets. It lives next to the demo for convenience, but conceptually it belongs to the host, not the app.
- `main.ts`: the code. The comments at the top of each file explain what to look for.

Run any demo from the repository root:

```bash
cargo run -p cells-cli -- run demos/<name>
```

See what a demo asks for and what the host would grant, without running it:

```bash
cargo run -p cells-cli -- inspect demos/<name>
```

## Suggested order

| Demo | What it shows |
|---|---|
| [`hello`](hello/main.ts) | The basics. A cell gets a `console` capability and the baseline globals, and nothing else. Shows relative imports, top-level await, and timers. |
| [`notes`](notes/main.ts) | How grants shape behavior without the app changing. It has two grants of the same `files@1` contract with different scopes, one of them read-only. It also has an optional `cache` that is decided on first use. Try all three host configs; the commands are at the top of `main.ts`. |
| [`sandbox`](sandbox/main.ts) | Tries every obvious way out of the cell and reports each one. Every line should say `blocked`. |
| [`runaway`](runaway/main.ts) | An infinite loop the host stops from outside. Run it with `--timeout 2`, or press Ctrl-C. |
| [`memory-hog`](memory-hog/main.ts) | A cell that exhausts its 32 MiB heap. It is stopped without taking the host down. |

## Debugging in VS Code

Cells expose V8's debugger over the Chrome DevTools Protocol, the same protocol Node uses. VS Code's built-in JavaScript debugger can attach to them, with breakpoints, stepping, and the Variables panel, all in the TypeScript source.

1. Open the repository folder in VS Code.
2. Set a breakpoint in a demo's `.ts` file, or add a `debugger;` statement.
3. In **Run and Debug**, choose **Debug a demo** and press F5. Pick a demo.

VS Code builds `cells`, runs the demo with `--inspect-brk`, and attaches. The configuration is in [`.vscode/launch.json`](../.vscode/launch.json). Program output, and prompts like the notes demo's "Allow? [y/N]", appear in the integrated terminal.

To debug from the command line instead, start the cell yourself:

```bash
cargo run -p cells-cli -- run demos/notes --inspect-brk
```

Then attach with **Attach to a running cell** in VS Code, or open `chrome://inspect` in Chrome.

- `--inspect` starts the debugger endpoint but lets the cell run immediately. It suits long-running cells.
- `--inspect-brk` waits for a debugger, then pauses on the first statement that runs. Imported modules run before the module that imports them, so in `hello` that first statement is in `greeting.ts`.
- Both accept a port, such as `--inspect=9230`. The default is 9229.

The endpoint only listens on `127.0.0.1` and refuses requests whose `Host` header names anything else. A debugger can read and change anything in the cell, so it is a host feature: the application cannot turn it on.

## Editor support

Each demo has a `tsconfig.json` and a `globals.d.ts` that declares its capability globals. The shared declarations are in [`types/`](../types). To type-check a demo:

```bash
cd demos/notes && npx -p typescript tsc -p .
```

The runtime itself strips types without checking them.

## Writing your own

1. Copy `demos/hello` to a new directory.
2. Declare the capabilities you need in `cell.json`. The available contracts are `console@1`, `files@1`, and `kv@1`; their methods are listed in [`types/contracts.d.ts`](../types/contracts.d.ts).
3. Grant them in `host.json`. The available providers are:

| Provider | Contract | Options |
|---|---|---|
| `stdout` | `console@1` | none |
| `local-files` | `files@1` | `root` (required), `readOnly` |
| `memory-kv` | `kv@1` | none |
| `json-file-kv` | `kv@1` | `path` (required) |

Every grant also accepts `"decision"`: `"grant"` (the default), `"deny"`, or `"ask"`. `"ask"` is only allowed for optional capabilities.
