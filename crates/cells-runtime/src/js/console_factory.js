// Builds the application-facing `console` object for the `console@1`
// contract. See `Contract::with_js_factory`.
//
// Formatting happens here, inside the cell, so the provider only ever
// receives plain text: `write(level, text)`.
(invoke, notify) => {
  const MAX_DEPTH = 4;

  function inspect(value, depth, seen) {
    switch (typeof value) {
      case "string":
        return depth === 0 ? value : JSON.stringify(value);
      case "number":
      case "boolean":
      case "undefined":
        return String(value);
      case "bigint":
        return `${value}n`;
      case "symbol":
        return value.toString();
      case "function":
        return `[Function${value.name ? ": " + value.name : " (anonymous)"}]`;
    }
    if (value === null) return "null";
    if (value instanceof Error) {
      return value.stack ?? `${value.name}: ${value.message}`;
    }
    if (seen.has(value)) return "[Circular]";
    if (depth >= MAX_DEPTH) return Array.isArray(value) ? "[Array]" : "[Object]";
    seen.add(value);
    try {
      const next = (v) => inspect(v, depth + 1, seen);
      if (Array.isArray(value)) {
        return `[ ${value.map(next).join(", ")} ]`;
      }
      if (value instanceof Map) {
        const entries = [...value].map(([k, v]) => `${next(k)} => ${next(v)}`);
        return `Map(${value.size}) { ${entries.join(", ")} }`;
      }
      if (value instanceof Set) {
        return `Set(${value.size}) { ${[...value].map(next).join(", ")} }`;
      }
      if (ArrayBuffer.isView(value)) {
        return `${value.constructor.name}(${value.length}) [ ${
          Array.prototype.slice.call(value, 0, 16).join(", ")
        }${value.length > 16 ? ", ..." : ""} ]`;
      }
      if (value instanceof Date) return value.toISOString();
      if (value instanceof URL) return `URL ${JSON.stringify(value.href)}`;
      const entries = Object.entries(value).map(([k, v]) => {
        const key = /^[A-Za-z_$][\w$]*$/.test(k) ? k : JSON.stringify(k);
        return `${key}: ${next(v)}`;
      });
      const name = value.constructor && value.constructor !== Object
        ? `${value.constructor.name} `
        : "";
      return entries.length ? `${name}{ ${entries.join(", ")} }` : `${name}{}`;
    } finally {
      seen.delete(value);
    }
  }

  function format(args) {
    return args.map((arg) => inspect(arg, 0, new Set())).join(" ");
  }

  const writer = (level) => (...args) => {
    notify("write", [level, format(args)]);
  };

  return {
    log: writer("log"),
    info: writer("info"),
    warn: writer("warn"),
    error: writer("error"),
    debug: writer("debug"),
  };
}
