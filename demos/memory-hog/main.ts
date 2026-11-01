// memory-hog: a cell that tries to use all the memory it can.
//
// Run it with:
//
//   cargo run -p cells-cli -- run demos/memory-hog
//
// host.json gives this cell a 32 MiB heap ("heapLimitMb": 32). When the cell
// approaches that limit, the runtime terminates it and reports
// "out of memory". Without that protection, V8 would abort the entire host
// process, taking every other cell with it (PROPOSAL.md §7.7).
//
// The limit covers the V8 heap. Memory behind ArrayBuffers is allocated
// outside it and is not limited yet; that is tracked in docs/deferred.md.

console.log("allocating until the heap limit is reached...");

const hoard: string[][] = [];
for (let round = 1; ; round++) {
  hoard.push(new Array(250_000).fill(`round ${round}`));
  if (round % 5 === 0) {
    console.log(`holding ${round} arrays of 250,000 entries`);
  }
}
