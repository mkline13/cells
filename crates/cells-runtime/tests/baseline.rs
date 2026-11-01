//! The baseline globals every cell receives (PROPOSAL.md §7.5).

mod common;

use common::Harness;

fn run(source: &str) -> Vec<String> {
  let h = Harness::new();
  h.file("main.ts", source);
  h.run_ok()
}

#[test]
fn timers() {
  let output = run(
    r#"
      const order: string[] = [];
      setTimeout(() => order.push("b"), 20);
      setTimeout((x: string) => order.push(x), 5, "a");
      const cancelled = setTimeout(() => order.push("never"), 10);
      clearTimeout(cancelled);
      let ticks = 0;
      const interval = setInterval(() => { if (++ticks === 3) clearInterval(interval); }, 1);
      await new Promise((resolve) => setTimeout(resolve, 60));
      console.log(order.join(","), ticks);
    "#,
  );
  assert_eq!(output, ["a,b 3"]);
}

#[test]
fn text_encoding() {
  let output = run(
    r#"
      const bytes = new TextEncoder().encode("héllo ✓");
      console.log(bytes.length, new TextDecoder().decode(bytes));
      console.log(new TextDecoder().decode(new Uint8Array([0xEF, 0xBB, 0xBF, 0x68, 0x69])));
      try { new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array([0xff])); }
      catch (e: any) { console.log(e.name); }
      console.log(new TextDecoder().decode(new Uint8Array([0x61, 0xff])));
    "#,
  );
  assert_eq!(output, ["10 héllo ✓", "hi", "TypeError", "a\u{fffd}"]);
}

#[test]
fn base64() {
  let output = run(r#"console.log(btoa("hello"), atob("aGVsbG8="), atob(btoa("a")), btoa(""));"#);
  assert_eq!(output, ["aGVsbG8= hello a "]);
}

#[test]
fn structured_clone() {
  let output = run(
    r#"
      const original = { date: new Date(0), list: [1, 2], map: new Map([["k", "v"]]) };
      const copy = structuredClone(original);
      console.log(copy !== original, copy.list !== original.list, copy.map.get("k"), copy.date.getTime());
    "#,
  );
  assert_eq!(output, ["true true v 0"]);
}

#[test]
fn url() {
  let output = run(
    r#"
      const url = new URL("../b/c?x=1&y=two words#frag", "https://user:pw@example.com:8080/a/index.html");
      console.log(url.href);
      console.log(url.origin, url.host, url.pathname, url.search, url.hash);
      console.log(url.searchParams.get("y"));
      url.searchParams.append("z", "3");
      url.pathname = "/new";
      console.log(url.toString());
      console.log(URL.canParse("not a url"), URL.canParse("/path", "https://a.example"));
    "#,
  );
  assert_eq!(
    output,
    [
      "https://user:pw@example.com:8080/b/c?x=1&y=two%20words#frag",
      "https://example.com:8080 example.com:8080 /b/c ?x=1&y=two%20words #frag",
      "two words",
      "https://user:pw@example.com:8080/new?x=1&y=two+words&z=3#frag",
      "false true",
    ]
  );
}

#[test]
fn url_search_params() {
  let output = run(
    r#"
      const params = new URLSearchParams("?a=1&b=2&a=3");
      params.set("b", "x y");
      params.delete("missing");
      console.log(params.getAll("a").join("|"), params.has("b"), params.size);
      params.sort();
      console.log(params.toString());
      console.log([...new URLSearchParams({ k: "v" })].join(";"));
    "#,
  );
  assert_eq!(output, ["1|3 true 3", "a=1&a=3&b=x+y", "k,v"]);
}

#[test]
fn queue_microtask() {
  let output = run(
    r#"
      const order: number[] = [];
      queueMicrotask(() => order.push(2));
      order.push(1);
      await Promise.resolve();
      console.log(order.join(","));
    "#,
  );
  assert_eq!(output, ["1,2"]);
}
