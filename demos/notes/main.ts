// notes: writes a numbered note each time it runs.
//
// This demo shows how grants shape what an application can do, without the
// application changing at all. Run it with each host config:
//
//   cargo run -p cells-cli -- run demos/notes
//       `cache` is decided on first use: you are asked on the terminal.
//       Answer y and the run count is kept in memory, so it always says
//       run #1. Answer n and the app carries on without it.
//
//   cargo run -p cells-cli -- run demos/notes --host demos/notes/host-persistent.json
//       `cache` is granted up front and backed by a JSON file, so the run
//       count goes up each time. Same contract, same app code, different
//       provider.
//
//   cargo run -p cells-cli -- run demos/notes --host demos/notes/host-locked-down.json
//       `cache` is denied and `templates` is not configured at all. The app
//       still works, with less.
//
//   cargo run -p cells-cli -- inspect demos/notes
//       Shows what the app asks for and what the host would grant.
//
// The four capabilities (see cell.json):
//
//   notes      files@1    required   where notes are written
//   templates  files@1    optional   a second, read-only files grant
//   cache      kv@1       optional   counts how many times the app has run
//   console    console@1  required   output
//
// `notes` and `templates` use the same contract but are separate grants
// with separate scopes. Each name is its own global.

import { countRun, isDenied, nextNoteNumber, render } from "./lib/notes.ts";

// --- Pick the template --------------------------------------------------------

// `templates` is optional. If the host denied it, the global is still here,
// but the call rejects with a CapabilityError of kind "permission-denied".
let template: string;
try {
  template = await templates.readText("note.md");
  console.log("using the template from the `templates` grant");
} catch (err) {
  if (!isDenied(err)) throw err;
  template = "# Note {{number}}\n\nWritten at {{time}}.\n";
  console.log("`templates` is not available; using a built-in template");
}

// --- Count this run ------------------------------------------------------------

// With host.json, this is the first use of `cache`, so this is where the
// host asks you whether to allow it.
const runs = await countRun(cache);
console.log(
  runs === null
    ? "`cache` is not available, so runs are not being counted"
    : `this is run #${runs} according to \`cache\``,
);

// --- Pick the note number -----------------------------------------------------

const number = await nextNoteNumber(notes);

// --- Write the note -----------------------------------------------------------

const name = `note-${String(number).padStart(3, "0")}.md`;
await notes.writeText(name, render(template, { number, time: new Date().toISOString() }));
console.log(`wrote ${name}`);

const all = await notes.list();
console.log(`the notes directory now has ${all.length} file(s):`, all);

// --- Try to step outside the grants -------------------------------------------

// The `notes` grant is restricted to one directory. The provider refuses
// paths that leave it, whether through "..", an absolute path, or a symlink.
for (const path of ["../../host.json", "/etc/passwd"]) {
  try {
    await notes.readText(path);
    console.error(`UNEXPECTED: read ${path}`);
  } catch (err) {
    console.log(`reading ${path} was refused:`, (err as CapabilityError).kind);
  }
}

// The `templates` grant is read-only. Writing through it is refused by the
// provider, even though `writeText` exists on the object: the contract is
// the same, but the host gave this grant a narrower scope.
try {
  await templates.writeText("note.md", "overwritten!");
  console.error("UNEXPECTED: overwrote the template");
} catch (err) {
  console.log("writing through `templates` was refused:", (err as CapabilityError).kind);
}
