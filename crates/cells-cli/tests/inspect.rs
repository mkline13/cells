//! The debugger endpoint started by `cells run --inspect` and
//! `--inspect-brk`. The protocol itself is tested in
//! crates/cells-runtime/tests/inspector.rs.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

fn demo(name: &str) -> std::path::PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demos").join(name)
}

fn http_get(port: u16, path: &str, host: &str) -> String {
  let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
  write!(stream, "GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
  let mut response = String::new();
  stream.read_to_string(&mut response).unwrap();
  response
}

#[test]
fn inspect_brk_waits_and_serves_the_target() {
  // Port 0 picks a free port; the URL printed on stderr says which.
  let mut child = Command::new(env!("CARGO_BIN_EXE_cells"))
    .args(["run"])
    .arg(demo("hello"))
    .arg("--inspect-brk=0")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  let mut stderr = BufReader::new(child.stderr.take().unwrap());
  let url = loop {
    let mut line = String::new();
    assert!(stderr.read_line(&mut line).unwrap() > 0, "cells exited early");
    if let Some(url) = line.trim().strip_prefix("[cells] debugger listening on ") {
      break url.to_string();
    }
  };
  let address = url.strip_prefix("ws://").unwrap();
  let (host, id) = address.split_once('/').unwrap();
  let port: u16 = host.rsplit_once(':').unwrap().1.parse().unwrap();

  let list = http_get(port, "/json/list", host);
  assert!(list.starts_with("HTTP/1.1 200"), "{list}");
  assert!(list.contains(&format!("\"webSocketDebuggerUrl\":\"{url}\"")), "{list}");
  assert!(list.contains("\"url\":\"cell:///main.ts\""), "{list}");

  let version = http_get(port, "/json/version", &format!("localhost:{port}"));
  assert!(version.contains("Protocol-Version"), "{version}");

  // Requests that do not name the loopback interface are refused, which
  // blocks DNS rebinding from web pages.
  let rebound = http_get(port, "/json/list", &format!("evil.example:{port}"));
  assert!(rebound.starts_with("HTTP/1.1 403"), "{rebound}");
  assert!(!rebound.contains(id));

  // The cell is still waiting for a debugger: it has printed nothing.
  let mut stdout = child.stdout.take().unwrap();
  child.kill().unwrap();
  child.wait().unwrap();
  let mut printed = String::new();
  stdout.read_to_string(&mut printed).unwrap();
  assert!(printed.is_empty(), "{printed}");
}

#[test]
fn inspect_without_a_debugger_runs_normally() {
  let out = Command::new(env!("CARGO_BIN_EXE_cells"))
    .args(["run"])
    .arg(demo("hello"))
    .arg("--inspect=0")
    .output()
    .unwrap();
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(out.status.success(), "{stderr}");
  assert!(stderr.contains("debugger listening on ws://127.0.0.1:"), "{stderr}");
  assert!(String::from_utf8_lossy(&out.stdout).contains("[hello] done"));
}

#[test]
fn rejects_both_inspect_flags() {
  let out = Command::new(env!("CARGO_BIN_EXE_cells"))
    .args(["run"])
    .arg(demo("hello"))
    .args(["--inspect", "--inspect-brk"])
    .output()
    .unwrap();
  assert!(!out.status.success());
  assert!(String::from_utf8_lossy(&out.stderr).contains("only one of"));
}
