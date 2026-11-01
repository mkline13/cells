// runaway: a cell that never stops, and a host that stops it anyway.
//
// Run it with a timeout:
//
//   cargo run -p cells-cli -- run demos/runaway --timeout 2
//
// or without one, and press Ctrl-C:
//
//   cargo run -p cells-cli -- run demos/runaway
//
// The loop below never yields, so no JavaScript-level mechanism could
// interrupt it: no timer would ever fire, and no promise would ever settle.
// The host stops it from outside by asking V8 to terminate the isolate,
// which interrupts running code wherever it is (PROPOSAL.md §7.7).

console.log("starting a loop that never ends...");

let iterations = 0;
while (true) {
  iterations++;
  if (iterations % 200_000_000 === 0) {
    console.log(`still going: ${iterations / 1_000_000}M iterations`);
  }
}
