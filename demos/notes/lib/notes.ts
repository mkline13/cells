// Helpers for the notes demo.
//
// These functions take capabilities as parameters instead of reaching for
// the globals. That keeps them testable and makes their authority obvious
// from their signatures: `render` needs nothing, `countRun` needs a store,
// and `nextNoteNumber` needs a directory.

/** Whether an error is a capability error saying the grant is not allowed. */
export function isDenied(err: unknown): boolean {
  const e = err as Partial<CapabilityError>;
  return e?.name === "CapabilityError" && e.kind === "permission-denied";
}

/** Fills `{{name}}` placeholders. */
export function render(template: string, values: Record<string, unknown>): string {
  return template.replace(/\{\{(\w+)\}\}/g, (_, key) => String(values[key] ?? ""));
}

/** Returns the number for the next note, based on the notes already written. */
export async function nextNoteNumber(dir: Files): Promise<number> {
  const existing = (await dir.list()).filter((n) => n.startsWith("note-"));
  return existing.length + 1;
}

/**
 * Counts how many times the app has run, using the key-value store.
 *
 * Returns `null` if the host denied the store. Whether the count survives
 * between runs depends on which provider the host chose, not on this code.
 */
export async function countRun(store: Kv): Promise<number | null> {
  try {
    const runs = ((await store.get<number>("runs")) ?? 0) + 1;
    await store.set("runs", runs);
    return runs;
  } catch (err) {
    if (!isDenied(err)) throw err;
    return null;
  }
}
