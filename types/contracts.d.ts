// TypeScript declarations for the capability contracts in the runtime MVP.
//
// These describe the *shape* of each capability. They are not security
// boundaries: types are erased at runtime, and providers validate every
// argument themselves (PROPOSAL.md §6.2).
//
// An application declares which globals it receives in its own
// `globals.d.ts`, using the grant names from its manifest:
//
//   declare const console: Console;   // "console": { "contract": "console@1" }
//   declare const notes: Files;       // "notes":   { "contract": "files@1" }

/** `console@1`: the standard console methods. Output goes wherever the host sends it. */
interface Console {
  log(...data: unknown[]): void;
  info(...data: unknown[]): void;
  warn(...data: unknown[]): void;
  error(...data: unknown[]): void;
  debug(...data: unknown[]): void;
}

/**
 * `files@1`: text files inside the directory the host chose for this grant.
 *
 * Paths are relative to that directory. Paths that try to leave it, such as
 * `../x` or `/etc/passwd`, reject with `permission-denied`.
 */
interface Files {
  readText(path: string): Promise<string>;
  writeText(path: string, contents: string): Promise<void>;
  /** Lists a directory. Subdirectory names end in `/`. Defaults to the root. */
  list(path?: string): Promise<string[]>;
  remove(path: string): Promise<void>;
}

type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

/** `kv@1`: a key-value store of JSON values. */
interface Kv {
  /** Resolves with `null` when the key does not exist. */
  get<T extends JsonValue = JsonValue>(key: string): Promise<T | null>;
  set(key: string, value: JsonValue): Promise<void>;
  /** Resolves with whether the key existed. */
  delete(key: string): Promise<boolean>;
  keys(prefix?: string): Promise<string[]>;
}

/**
 * The error a capability operation rejects with.
 *
 * This is a type only; there is no `CapabilityError` global. Check
 * `err.name === "CapabilityError"` and then `err.kind`.
 */
interface CapabilityError extends Error {
  name: "CapabilityError";
  kind:
    | "permission-denied"
    | "invalid-argument"
    | "not-found"
    | "unavailable"
    | "internal";
}
