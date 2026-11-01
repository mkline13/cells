// TypeScript declarations for the baseline globals every cell receives
// (PROPOSAL.md §7.5). These reach nothing outside the cell, so they are not
// capabilities.
//
// Use these with `"lib": ["ES2024"]` and no DOM library: the DOM library
// declares many globals (fetch, localStorage, document, ...) that do not
// exist in a cell.

// Extra arguments are passed to the callback.
declare function setTimeout(callback: (...args: any[]) => void, delay?: number, ...args: any[]): number;
declare function setInterval(callback: (...args: any[]) => void, delay?: number, ...args: any[]): number;
declare function clearTimeout(id: number | undefined): void;
declare function clearInterval(id: number | undefined): void;
declare function queueMicrotask(callback: () => void): void;

declare function structuredClone<T>(value: T): T;
declare function atob(data: string): string;
declare function btoa(data: string): string;

declare class TextEncoder {
  readonly encoding: "utf-8";
  encode(input?: string): Uint8Array;
  encodeInto(input: string, destination: Uint8Array): { read: number; written: number };
}

/** UTF-8 only. Streaming is not supported. */
declare class TextDecoder {
  constructor(label?: string, options?: { fatal?: boolean; ignoreBOM?: boolean });
  readonly encoding: "utf-8";
  readonly fatal: boolean;
  readonly ignoreBOM: boolean;
  decode(input?: ArrayBuffer | ArrayBufferView): string;
}

declare class URLSearchParams implements Iterable<[string, string]> {
  constructor(init?: string | Iterable<[string, string]> | Record<string, string> | URLSearchParams);
  readonly size: number;
  append(name: string, value: string): void;
  delete(name: string, value?: string): void;
  get(name: string): string | null;
  getAll(name: string): string[];
  has(name: string, value?: string): boolean;
  set(name: string, value: string): void;
  sort(): void;
  forEach(callback: (value: string, key: string, parent: URLSearchParams) => void): void;
  entries(): IterableIterator<[string, string]>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;
  [Symbol.iterator](): IterableIterator<[string, string]>;
  toString(): string;
}

declare class URL {
  constructor(url: string | URL, base?: string | URL);
  static canParse(url: string | URL, base?: string | URL): boolean;
  static parse(url: string | URL, base?: string | URL): URL | null;
  href: string;
  readonly origin: string;
  protocol: string;
  username: string;
  password: string;
  host: string;
  hostname: string;
  port: string;
  pathname: string;
  search: string;
  hash: string;
  readonly searchParams: URLSearchParams;
  toString(): string;
  toJSON(): string;
}

interface ImportMeta {
  /** This module's URL, such as `cell:///main.ts`. */
  url: string;
  resolve(specifier: string): string;
}
