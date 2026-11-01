# Deferred Work

Temporary implementation decisions that must change at a later stage. Each entry here has matching `TODO(<stage>)` comments in the code, so `grep -rn "TODO(worker-processes)"` finds every place affected by that stage.

## Stage tags

| Tag | Stage | Proposal reference |
|---|---|---|
| `supervisor` | Supervisor and host API | §8.5 |
| `desktop-host` | Dashboard, the first desktop host | §8.6 |
| `composition` | Applications acting as capability providers | §8.5, §9.3 |
| `worker-processes` | Cells in separate worker processes (isolation Stage 2) | §7.8 |
| `os-isolation` | Operating-system isolation (isolation Stage 3) | §7.8 |
| `server-host` | Server host | §8.8 |

## Entries

### In-memory grant table in the runtime

- **Tag:** `supervisor`
- **Now:** The runtime ships a simple in-memory implementation of the dispatch interface so it can run without the supervisor.
- **Later:** The supervisor implements the dispatch interface and owns the grant table. The runtime's implementation remains only as a test and embedding convenience.

### Heap-limit termination as the only memory defense

- **Tag:** `worker-processes`
- **Now:** Each cell has a V8 heap limit, and the runtime terminates a cell that approaches it. `ArrayBuffer` memory outside the V8 heap needs its own allocation limit, and a fault in V8 or the host still affects every cell in the process.
- **Later:** Cells run in separate worker processes, with operating-system resource limits added at `os-isolation`.

### Dispatch messages travel over in-process channels

- **Tag:** `worker-processes`
- **Now:** Capability operations send messages to the supervisor's runtime over in-process channels.
- **Later:** The same messages cross the process boundary through IPC. The message format should not change.

### Fire-and-forget messages are dropped when the channel is full

- **Tag:** `server-host`
- **Now:** Operations such as `console.log` drop and count messages rather than block the cell.
- **Later:** Revisit for persistent server execution, where silently lost log output may be unacceptable.

### Ambient time and randomness

- **Tag:** `os-isolation`
- **Now:** `Date` and `Math.random` are available to every cell.
- **Later:** Reassess under the multi-user threat model, where ambient clocks enable timing side channels.
