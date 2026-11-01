// Builds a cell's global scope before any application code runs
// (PROPOSAL.md §7.5).
//
// This is trusted host code. It runs while `Deno.core` is still reachable,
// captures the few internal functions it needs, and then removes `Deno` and
// `__bootstrap` from the global object. Application code can never reach
// them afterwards.
//
// Steps:
//   1. Remove deno_core's `console`, so `console` exists only if granted.
//   2. Install the baseline web APIs, which reach nothing outside the cell.
//   3. Install one global per declared capability. Each wraps a grant handle.
//   4. Remove `Deno` and `__bootstrap`.
//
// The Rust side calls this function with:
//   grants:    [{ name, contract, handle, methods: [{ name, kind }] }]
//   factories: { [contractId]: (invoke, notify) => object }
(function bootstrap(grants, factories) {
  "use strict";

  const core = globalThis.Deno.core;
  const {
    op_cell_invoke,
    op_cell_notify,
    op_cell_url_parse,
    op_cell_url_set,
    op_cell_decode_utf8,
  } = core.ops;
  const { createTimer, cancelTimer, encode, structuredClone: coreClone } = core;

  // Captured so later changes to built-in prototypes by application code
  // cannot change how this script's functions behave.
  const ObjectDefineProperty = Object.defineProperty;
  const ObjectFreeze = Object.freeze;
  const ObjectHasOwn = Object.hasOwn;
  const ArrayIsArray = Array.isArray;
  const MapCtor = Map;
  const ErrorCtor = Error;
  const TypeErrorCtor = TypeError;
  const SymbolIterator = Symbol.iterator;
  const StringFromCharCode = String.fromCharCode;

  // Defines a global the way web standards define theirs: writable,
  // configurable, and not enumerable.
  function defineBaseline(name, value) {
    ObjectDefineProperty(globalThis, name, {
      value,
      writable: true,
      enumerable: false,
      configurable: true,
    });
  }

  // ---------------------------------------------------------------------------
  // 1. Remove deno_core's console.
  // ---------------------------------------------------------------------------

  delete globalThis.console;

  // ---------------------------------------------------------------------------
  // 2. Baseline web APIs (§7.5 "Baseline globals").
  // ---------------------------------------------------------------------------

  // --- Timers -----------------------------------------------------------------
  // Timers run on the cell's own event loop. When a cell is terminated its
  // isolate is destroyed, which cancels every pending timer.

  let nextTimerId = 1;
  const activeTimers = new MapCtor();

  function startTimer(callback, delay, args, repeat) {
    if (typeof callback !== "function") {
      throw new TypeErrorCtor("The callback must be a function");
    }
    const id = nextTimerId++;
    const timer = createTimer(
      () => {
        if (!repeat) activeTimers.delete(id);
        callback(...args);
      },
      Number(delay) || 0,
      undefined,
      repeat,
      true,
    );
    activeTimers.set(id, timer);
    return id;
  }

  function clearTimer(id) {
    const timer = activeTimers.get(id);
    if (timer !== undefined) {
      activeTimers.delete(id);
      cancelTimer(timer);
    }
  }

  defineBaseline("setTimeout", (cb, delay = 0, ...args) => startTimer(cb, delay, args, false));
  defineBaseline("setInterval", (cb, delay = 0, ...args) => startTimer(cb, delay, args, true));
  defineBaseline("clearTimeout", (id) => clearTimer(id));
  defineBaseline("clearInterval", (id) => clearTimer(id));

  // --- structuredClone ----------------------------------------------------------

  defineBaseline("structuredClone", function structuredClone(value, options) {
    if (options !== undefined && options.transfer !== undefined) {
      throw new TypeErrorCtor("structuredClone: transfer is not supported");
    }
    return coreClone(value);
  });

  // --- TextEncoder / TextDecoder (UTF-8 only) ---------------------------------

  class TextEncoder {
    get encoding() {
      return "utf-8";
    }
    encode(input = "") {
      return encode(String(input));
    }
    encodeInto(input, destination) {
      const bytes = encode(String(input));
      const written = Math.min(bytes.length, destination.length);
      destination.set(bytes.subarray(0, written));
      // `read` counts UTF-16 code units; this is exact only when everything
      // fits, which covers the common case.
      return { read: written === bytes.length ? String(input).length : 0, written };
    }
  }

  const UTF8_LABELS = ["utf-8", "utf8", "unicode-1-1-utf-8"];

  class TextDecoder {
    #fatal;
    #ignoreBOM;
    constructor(label = "utf-8", options = {}) {
      const normalized = String(label).trim().toLowerCase();
      if (!UTF8_LABELS.includes(normalized)) {
        throw new RangeError(`The encoding "${label}" is not supported`);
      }
      this.#fatal = Boolean(options.fatal);
      this.#ignoreBOM = Boolean(options.ignoreBOM);
    }
    get encoding() {
      return "utf-8";
    }
    get fatal() {
      return this.#fatal;
    }
    get ignoreBOM() {
      return this.#ignoreBOM;
    }
    decode(input = new Uint8Array(0), options = {}) {
      if (options.stream) {
        throw new TypeErrorCtor("TextDecoder: streaming is not supported");
      }
      let bytes;
      if (input instanceof ArrayBuffer || input instanceof SharedArrayBuffer) {
        bytes = new Uint8Array(input);
      } else if (ArrayBuffer.isView(input)) {
        bytes = new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
      } else {
        throw new TypeErrorCtor("TextDecoder: input must be a BufferSource");
      }
      return op_cell_decode_utf8(bytes, this.#fatal, this.#ignoreBOM);
    }
  }

  defineBaseline("TextEncoder", TextEncoder);
  defineBaseline("TextDecoder", TextDecoder);

  // --- atob / btoa ----------------------------------------------------------------

  const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

  defineBaseline("btoa", function btoa(data) {
    data = String(data);
    let out = "";
    for (let i = 0; i < data.length; i += 3) {
      const codes = [data.charCodeAt(i), data.charCodeAt(i + 1), data.charCodeAt(i + 2)];
      for (const c of codes) {
        if (c > 0xff) {
          throw new ErrorCtor("btoa: the string contains characters outside of Latin-1");
        }
      }
      const [a, b, c] = codes;
      const n = (a << 16) | ((b || 0) << 8) | (c || 0);
      out += B64[(n >> 18) & 63] + B64[(n >> 12) & 63];
      out += i + 1 < data.length ? B64[(n >> 6) & 63] : "=";
      out += i + 2 < data.length ? B64[n & 63] : "=";
    }
    return out;
  });

  defineBaseline("atob", function atob(data) {
    data = String(data).replace(/[\t\n\f\r ]/g, "");
    if (data.length % 4 === 0) data = data.replace(/==?$/, "");
    if (data.length % 4 === 1 || /[^A-Za-z0-9+/]/.test(data)) {
      throw new ErrorCtor("atob: the string is not correctly encoded");
    }
    let out = "";
    let buffer = 0;
    let bits = 0;
    for (const ch of data) {
      buffer = (buffer << 6) | B64.indexOf(ch);
      bits += 6;
      if (bits >= 8) {
        bits -= 8;
        out += StringFromCharCode((buffer >> bits) & 0xff);
      }
    }
    return out;
  });

  // --- URLSearchParams --------------------------------------------------------------

  // Encodes like `application/x-www-form-urlencoded`.
  function formEncode(s) {
    return encodeURIComponent(s)
      .replace(/[!'()~]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase())
      .replace(/%20/g, "+");
  }

  function formDecode(s) {
    const replaced = s.replace(/\+/g, " ");
    try {
      return decodeURIComponent(replaced);
    } catch {
      return replaced;
    }
  }

  function parseQuery(query) {
    const list = [];
    for (const part of query.split("&")) {
      if (part === "") continue;
      const eq = part.indexOf("=");
      const name = eq === -1 ? part : part.slice(0, eq);
      const value = eq === -1 ? "" : part.slice(eq + 1);
      list.push([formDecode(name), formDecode(value)]);
    }
    return list;
  }

  // Lets a URL observe changes to its `searchParams`.
  const searchParamsOwner = new WeakMap();

  class URLSearchParams {
    #list = [];

    constructor(init = "") {
      if (init instanceof URLSearchParams) {
        this.#list = init.#list.map(([k, v]) => [k, v]);
      } else if (typeof init === "object" && init !== null) {
        if (typeof init[SymbolIterator] === "function") {
          for (const pair of init) {
            const entry = [...pair];
            if (entry.length !== 2) {
              throw new TypeErrorCtor("Each pair must have exactly two items");
            }
            this.#list.push([String(entry[0]), String(entry[1])]);
          }
        } else {
          for (const key of Object.keys(init)) {
            this.#list.push([key, String(init[key])]);
          }
        }
      } else {
        const query = String(init);
        this.#list = parseQuery(query.startsWith("?") ? query.slice(1) : query);
      }
    }

    #update() {
      const owner = searchParamsOwner.get(this);
      if (owner) owner(this.toString());
    }

    // Used by URL when its query changes.
    static _replace(params, query) {
      params.#list = parseQuery(query.startsWith("?") ? query.slice(1) : query);
    }

    get size() {
      return this.#list.length;
    }
    append(name, value) {
      this.#list.push([String(name), String(value)]);
      this.#update();
    }
    delete(name, value) {
      name = String(name);
      this.#list = this.#list.filter(([k, v]) =>
        k !== name || (value !== undefined && v !== String(value))
      );
      this.#update();
    }
    get(name) {
      name = String(name);
      const entry = this.#list.find(([k]) => k === name);
      return entry ? entry[1] : null;
    }
    getAll(name) {
      name = String(name);
      return this.#list.filter(([k]) => k === name).map(([, v]) => v);
    }
    has(name, value) {
      name = String(name);
      return this.#list.some(([k, v]) =>
        k === name && (value === undefined || v === String(value))
      );
    }
    set(name, value) {
      name = String(name);
      value = String(value);
      const index = this.#list.findIndex(([k]) => k === name);
      if (index === -1) {
        this.#list.push([name, value]);
      } else {
        this.#list[index][1] = value;
        this.#list = this.#list.filter(([k], i) => k !== name || i === index);
      }
      this.#update();
    }
    sort() {
      // Stable, by UTF-16 code units, as the standard requires.
      this.#list.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
      this.#update();
    }
    forEach(callback, thisArg) {
      for (const [k, v] of this.#list) callback.call(thisArg, v, k, this);
    }
    *entries() {
      for (const [k, v] of this.#list) yield [k, v];
    }
    *keys() {
      for (const [k] of this.#list) yield k;
    }
    *values() {
      for (const [, v] of this.#list) yield v;
    }
    [SymbolIterator]() {
      return this.entries();
    }
    toString() {
      return this.#list.map(([k, v]) => `${formEncode(k)}=${formEncode(v)}`).join("&");
    }
  }

  const replaceSearchParams = URLSearchParams._replace;
  delete URLSearchParams._replace;

  // --- URL ------------------------------------------------------------------------------
  // Parsing and setters are implemented in Rust by the `url` crate, which
  // follows the WHATWG URL standard.

  class URL {
    #parts;
    #searchParams = null;

    constructor(url, base) {
      this.#parts = op_cell_url_parse(String(url), base === undefined ? null : String(base));
    }

    static canParse(url, base) {
      try {
        new URL(url, base);
        return true;
      } catch {
        return false;
      }
    }

    static parse(url, base) {
      try {
        return new URL(url, base);
      } catch {
        return null;
      }
    }

    #set(field, value) {
      this.#parts = op_cell_url_set(this.#parts.href, field, String(value));
      if (this.#searchParams !== null) {
        replaceSearchParams(this.#searchParams, this.#parts.search);
      }
    }

    get href() { return this.#parts.href; }
    set href(v) { this.#set("href", v); }
    get origin() { return this.#parts.origin; }
    get protocol() { return this.#parts.protocol; }
    set protocol(v) { this.#set("protocol", v); }
    get username() { return this.#parts.username; }
    set username(v) { this.#set("username", v); }
    get password() { return this.#parts.password; }
    set password(v) { this.#set("password", v); }
    get host() { return this.#parts.host; }
    set host(v) { this.#set("host", v); }
    get hostname() { return this.#parts.hostname; }
    set hostname(v) { this.#set("hostname", v); }
    get port() { return this.#parts.port; }
    set port(v) { this.#set("port", v); }
    get pathname() { return this.#parts.pathname; }
    set pathname(v) { this.#set("pathname", v); }
    get search() { return this.#parts.search; }
    set search(v) { this.#set("search", v); }
    get hash() { return this.#parts.hash; }
    set hash(v) { this.#set("hash", v); }

    get searchParams() {
      if (this.#searchParams === null) {
        this.#searchParams = new URLSearchParams(this.#parts.search);
        searchParamsOwner.set(this.#searchParams, (query) => {
          this.#parts = op_cell_url_set(this.#parts.href, "search", query);
        });
      }
      return this.#searchParams;
    }

    toString() {
      return this.href;
    }
    toJSON() {
      return this.href;
    }
  }

  defineBaseline("URLSearchParams", URLSearchParams);
  defineBaseline("URL", URL);

  // `queueMicrotask` is installed by deno_core and is kept as-is.

  // ---------------------------------------------------------------------------
  // 3. Capability globals (§6.1).
  // ---------------------------------------------------------------------------

  // Errors from capability operations. Applications check `err.kind`, for
  // example `err.kind === "permission-denied"`.
  function capabilityError(kind, message) {
    const err = new ErrorCtor(message);
    err.name = "CapabilityError";
    err.kind = kind;
    return err;
  }

  function buildCapability(grant) {
    // Both functions are bound to this grant's handle. Neither carries
    // authority: the dispatcher checks the handle against this cell's grants.
    const handle = grant.handle;
    async function invoke(method, args) {
      if (!ArrayIsArray(args)) throw new TypeErrorCtor("args must be an array");
      const result = await op_cell_invoke(handle, method, args);
      if (result.ok) return result.value;
      throw capabilityError(result.kind, result.message);
    }
    function notify(method, args) {
      if (!ArrayIsArray(args)) throw new TypeErrorCtor("args must be an array");
      op_cell_notify(handle, method, args);
    }

    const factory = factories[grant.contract];
    if (factory !== undefined) return factory(invoke, notify);

    const capability = {};
    for (const method of grant.methods) {
      const name = method.name;
      capability[name] = method.kind === "notify"
        ? (...args) => { notify(name, args); }
        : (...args) => invoke(name, args);
    }
    return capability;
  }

  for (const grant of grants) {
    // Catches collisions with ECMAScript built-ins, baseline globals, and
    // anything inherited by the global object (§6.1).
    if (grant.name in globalThis || ObjectHasOwn(globalThis, grant.name)) {
      throw new TypeErrorCtor(
        `Grant name "${grant.name}" collides with an existing global`,
      );
    }
    const capability = ObjectFreeze(buildCapability(grant));
    // Not writable or configurable, so a capability cannot be replaced by
    // accident. Replacing one would not grant anything anyway.
    ObjectDefineProperty(globalThis, grant.name, {
      value: capability,
      writable: false,
      enumerable: false,
      configurable: false,
    });
  }

  // ---------------------------------------------------------------------------
  // 4. Remove deno_core's internals from the global object.
  // ---------------------------------------------------------------------------

  delete globalThis.Deno;
  delete globalThis.__bootstrap;
})
