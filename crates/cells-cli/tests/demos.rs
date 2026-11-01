//! Runs every demo through the `cells` binary, so the demos keep working.
//!
//! Each demo is copied to a temporary directory first, so runs do not leave
//! data behind in the repository.

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

fn demos_dir() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demos")
}

fn copy_dir(from: &Path, to: &Path) {
  std::fs::create_dir_all(to).unwrap();
  for entry in std::fs::read_dir(from).unwrap() {
    let entry = entry.unwrap();
    let target = to.join(entry.file_name());
    if entry.file_type().unwrap().is_dir() {
      if entry.file_name() != "data" {
        copy_dir(&entry.path(), &target);
      }
    } else {
      std::fs::copy(entry.path(), target).unwrap();
    }
  }
}

/// Copies a demo into a temporary directory and returns its path there.
fn demo(name: &str) -> (tempfile::TempDir, PathBuf) {
  let tmp = tempfile::tempdir().unwrap();
  let dir = tmp.path().join(name);
  copy_dir(&demos_dir().join(name), &dir);
  (tmp, dir)
}

fn cells(args: &[&str], dir: &Path, stdin: &str) -> Output {
  let mut child = Command::new(env!("CARGO_BIN_EXE_cells"))
    .args(args)
    .arg(dir)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
  child.wait_with_output().unwrap()
}

fn text(output: &Output) -> String {
  format!(
    "{}{}",
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  )
}

#[test]
fn hello() {
  let (_tmp, dir) = demo("hello");
  let out = cells(&["run"], &dir, "");
  let log = text(&out);
  assert!(out.status.success(), "{log}");
  assert!(log.contains("[hello] hello, world!"), "{log}");
  assert!(log.contains("[hello] this module is cell:///main.ts"), "{log}");
  assert!(log.contains("[hello] done"), "{log}");
}

#[test]
fn notes_asks_before_using_the_cache() {
  let (_tmp, dir) = demo("notes");

  let out = cells(&["run"], &dir, "y\n");
  let log = text(&out);
  assert!(out.status.success(), "{log}");
  assert!(log.contains("wants to use `cache`"), "{log}");
  assert!(log.contains("this is run #1 according to `cache`"), "{log}");
  assert!(log.contains("wrote note-001.md"), "{log}");
  assert!(log.contains("reading ../../host.json was refused: permission-denied"), "{log}");
  assert!(log.contains("writing through `templates` was refused: permission-denied"), "{log}");
  assert!(!log.contains("UNEXPECTED"), "{log}");

  // Declining is handled by the application.
  let out = cells(&["run"], &dir, "n\n");
  let log = text(&out);
  assert!(out.status.success(), "{log}");
  assert!(log.contains("runs are not being counted"), "{log}");
  assert!(log.contains("wrote note-002.md"), "{log}");
}

#[test]
fn notes_with_a_persistent_cache() {
  let (_tmp, dir) = demo("notes");
  let host = dir.join("host-persistent.json");
  let host = host.to_str().unwrap();
  for run in 1..=2 {
    let out = cells(&["run", "--host", host], &dir, "");
    let log = text(&out);
    assert!(out.status.success(), "{log}");
    assert!(log.contains(&format!("this is run #{run} according to `cache`")), "{log}");
  }
}

#[test]
fn notes_locked_down() {
  let (_tmp, dir) = demo("notes");
  let host = dir.join("host-locked-down.json");
  let out = cells(&["run", "--host", host.to_str().unwrap()], &dir, "");
  let log = text(&out);
  assert!(out.status.success(), "{log}");
  assert!(log.contains("`templates` is not available"), "{log}");
  assert!(log.contains("runs are not being counted"), "{log}");
  assert!(!log.contains("wants to use"), "{log}");
}

#[test]
fn notes_inspect() {
  let (_tmp, dir) = demo("notes");
  let out = cells(&["inspect"], &dir, "");
  let log = text(&out);
  assert!(out.status.success(), "{log}");
  assert!(log.contains("lib/notes.ts"), "{log}");
  assert!(log.contains("ask on first use"), "{log}");
  assert!(log.contains("read-only"), "{log}");
}

#[test]
fn sandbox_blocks_everything() {
  let (tmp, dir) = demo("sandbox");
  // The file the demo tries to import from outside its directory.
  std::fs::copy(demos_dir().join("sandbox-secret.ts"), tmp.path().join("sandbox-secret.ts")).unwrap();
  let out = cells(&["run"], &dir, "");
  let log = text(&out);
  assert!(out.status.success(), "{log}");
  assert!(log.contains("all attempts were blocked"), "{log}");
  assert!(!log.contains("LEAKED"), "{log}");
}

#[test]
fn runaway_is_terminated() {
  let (_tmp, dir) = demo("runaway");
  let out = cells(&["run", "--timeout", "0.5"], &dir, "");
  let log = text(&out);
  assert!(!out.status.success(), "{log}");
  assert!(log.contains("runaway terminated"), "{log}");
}

#[test]
fn memory_hog_runs_out_of_memory() {
  let (_tmp, dir) = demo("memory-hog");
  let out = cells(&["run"], &dir, "");
  let log = text(&out);
  assert!(!out.status.success(), "{log}");
  assert!(log.contains("terminated: out of memory"), "{log}");
}

#[test]
fn missing_required_grants_refuse_to_start() {
  let (_tmp, dir) = demo("notes");
  std::fs::write(dir.join("host.json"), r#"{ "grants": { "console": { "provider": "stdout" } } }"#).unwrap();
  let out = cells(&["run"], &dir, "");
  let log = text(&out);
  assert!(!out.status.success(), "{log}");
  assert!(log.contains("required capability `notes` is not configured"), "{log}");
}
