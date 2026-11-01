//! The debugger: a DevTools protocol session can pause a cell, read its
//! variables, and see TypeScript source maps.

mod common;

use std::time::Duration;

use cells_runtime::CellExit;
use cells_runtime::InspectMode;
use cells_runtime::InspectorSession;
use common::Harness;
use serde_json::Value;
use serde_json::json;

/// A tiny DevTools protocol client.
struct Client {
  session: InspectorSession,
  next_id: u64,
  /// Notifications received while waiting for something else.
  events: Vec<Value>,
}

impl Client {
  async fn call(&mut self, method: &str, params: Value) -> Value {
    self.next_id += 1;
    let id = self.next_id;
    let message = json!({ "id": id, "method": method, "params": params });
    assert!(self.session.send(message.to_string()), "session closed");
    loop {
      let msg = self.next().await;
      if msg["id"] == id {
        assert!(msg.get("error").is_none(), "{method} failed: {msg}");
        return msg["result"].clone();
      }
      self.events.push(msg);
    }
  }

  /// Waits for the notification `method`, including one already received.
  async fn event(&mut self, method: &str) -> Value {
    if let Some(i) = self.events.iter().position(|e| e["method"] == method) {
      return self.events.remove(i);
    }
    loop {
      let msg = self.next().await;
      if msg["method"] == method {
        return msg;
      }
      self.events.push(msg);
    }
  }

  async fn next(&mut self) -> Value {
    let text = tokio::time::timeout(Duration::from_secs(10), self.session.recv())
      .await
      .expect("timed out waiting for the inspector")
      .expect("session closed");
    serde_json::from_str(&text).unwrap()
  }
}

#[test]
fn a_debugger_can_pause_the_cell_and_read_variables() {
  let h = Harness::new();
  h.file(
    "main.ts",
    "const greeting: string = 'hi';\nfunction work(): number {\n  const secretNumber = 41 + 1;\n  debugger;\n  return secretNumber;\n}\nconsole.log(greeting, work());\n",
  );
  let mut config = h.config();
  config.inspect = Some(InspectMode::BreakOnStart);
  let cell = h.engine.spawn(config).unwrap();
  let inspector = cell.inspector().expect("inspector enabled");

  common::supervisor().block_on(async {
    let mut client = Client {
      session: inspector.connect().unwrap(),
      next_id: 0,
      events: Vec::new(),
    };
    client.call("Runtime.enable", json!({})).await;
    client.call("Debugger.enable", json!({})).await;
    client.call("Runtime.runIfWaitingForDebugger", json!({})).await;

    // Paused on the first statement of main.ts. Call frames identify their
    // script by id; the URL comes from the matching scriptParsed event.
    let paused = client.event("Debugger.paused").await;
    let parsed = loop {
      let event = client.event("Debugger.scriptParsed").await;
      if event["params"]["url"] == "cell:///main.ts" {
        break event;
      }
    };
    assert_eq!(
      paused["params"]["callFrames"][0]["location"]["scriptId"],
      parsed["params"]["scriptId"]
    );
    assert_eq!(paused["params"]["callFrames"][0]["location"]["lineNumber"], 0);

    // The module was compiled with an inline TypeScript source map.
    assert!(parsed["params"]["sourceMapURL"]
      .as_str()
      .unwrap()
      .starts_with("data:application/json"));

    // Run to the `debugger;` statement and read a local variable.
    client.call("Debugger.resume", json!({})).await;
    let paused = client.event("Debugger.paused").await;
    let frame = paused["params"]["callFrames"][0]["callFrameId"].clone();
    let result = client
      .call(
        "Debugger.evaluateOnCallFrame",
        json!({ "callFrameId": frame, "expression": "secretNumber" }),
      )
      .await;
    assert_eq!(result["result"]["value"], 42);

    // The Variables panel lists locals through the scope chain.
    let scope = paused["params"]["callFrames"][0]["scopeChain"][0]["object"]["objectId"].clone();
    let props = client
      .call("Runtime.getProperties", json!({ "objectId": scope, "ownProperties": true }))
      .await;
    let names: Vec<&str> = props["result"]
      .as_array()
      .unwrap()
      .iter()
      .filter_map(|p| p["name"].as_str())
      .collect();
    assert!(names.contains(&"secretNumber"), "{names:?}");

    client.call("Debugger.resume", json!({})).await;
  });

  assert_eq!(cell.join().exit, CellExit::Completed);
  assert_eq!(h.output(), ["hi 42"]);
}

#[test]
fn cells_without_inspect_have_no_inspector() {
  let h = Harness::new();
  h.file("main.ts", "");
  let cell = h.spawn();
  assert!(cell.inspector().is_none());
  assert_eq!(cell.join().exit, CellExit::Completed);
}
