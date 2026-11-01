// A module imported by main.ts as "./greeting.ts".
//
// Modules are ordinary ES modules. Each cell has its own module state, so
// two cells running this application would not share `count`.

export interface Greeting {
  text: string;
  count: number;
}

let count = 0;

export function greet(name: string): Greeting {
  count += 1;
  return { text: `hello, ${name}!`, count };
}
