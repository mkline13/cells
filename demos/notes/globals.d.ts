// The capability globals this application receives. The names match the
// keys under "capabilities" in cell.json.
//
// `templates` and `cache` are optional. They are always present as globals,
// but the host may deny them, in which case every call rejects with a
// CapabilityError whose kind is "permission-denied".
declare const console: Console;
declare const notes: Files;
declare const templates: Files;
declare const cache: Kv;
