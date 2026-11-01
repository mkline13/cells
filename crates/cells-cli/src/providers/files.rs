//! `local-files`: implements `files@1` with a directory on the host.
//!
//! Options:
//! - `root` (required): the directory the grant is restricted to.
//! - `readOnly` (default `false`): reject `writeText` and `remove`.
//!
//! Every path is resolved relative to a `cap_std::fs::Dir` opened at the
//! root, which refuses paths that escape it, including through `..` and
//! symlinks (PROPOSAL.md §7.4). The application never learns the host path
//! of the root.

use std::io::ErrorKind as IoErrorKind;
use std::path::Path;
use std::sync::Arc;

use cap_std::fs::Dir;
use cells_runtime::CallFuture;
use cells_runtime::CapabilityError;
use cells_runtime::Provider;
use cells_runtime::Value;

use super::ProviderContext;

pub fn build(ctx: &ProviderContext) -> Result<Arc<dyn Provider>, String> {
  super::only_options(ctx, &["root", "readOnly"])?;
  let root = string_option_path(ctx)?;
  let read_only = super::bool_option(ctx, "readOnly")?;
  std::fs::create_dir_all(&root)
    .map_err(|e| format!("could not create {}: {e}", root.display()))?;
  let dir = Dir::open_ambient_dir(&root, cap_std::ambient_authority())
    .map_err(|e| format!("could not open {}: {e}", root.display()))?;
  Ok(Arc::new(LocalFiles {
    dir: Arc::new(dir),
    shown_root: root.display().to_string(),
    read_only,
  }))
}

fn string_option_path(ctx: &ProviderContext) -> Result<std::path::PathBuf, String> {
  let root = super::string_option(ctx, "root")?;
  Ok(ctx.base_dir.join(root))
}

struct LocalFiles {
  dir: Arc<Dir>,
  /// Shown by `cells inspect`. Never sent to the application.
  shown_root: String,
  read_only: bool,
}

impl Provider for LocalFiles {
  fn describe(&self) -> String {
    let mode = if self.read_only { "read-only" } else { "read-write" };
    format!("local-files (root: {}, {mode})", self.shown_root)
  }

  fn invoke(&self, method: &str, args: Vec<Value>) -> CallFuture {
    let dir = self.dir.clone();
    let method = method.to_string();
    let read_only = self.read_only;
    Box::pin(async move {
      // cap-std is synchronous, so file operations run on a blocking thread.
      tokio::task::spawn_blocking(move || call(&dir, &method, &args, read_only))
        .await
        .unwrap_or_else(|_| Err(CapabilityError::internal("the filesystem operation failed")))
    })
  }
}

fn call(dir: &Dir, method: &str, args: &[Value], read_only: bool) -> Result<Value, CapabilityError> {
  match method {
    "readText" => {
      let path = path_arg(args, 0)?;
      let text = dir.read_to_string(&path).map_err(|e| io_error(e, &path))?;
      Ok(Value::String(text))
    }
    "writeText" => {
      if read_only {
        return Err(CapabilityError::permission_denied("this directory is read-only"));
      }
      let path = path_arg(args, 0)?;
      let contents = super::string_arg(args, 1, "contents")?;
      if let Some(parent) = Path::new(&path).parent()
        && !parent.as_os_str().is_empty()
      {
        dir.create_dir_all(parent).map_err(|e| io_error(e, &path))?;
      }
      dir.write(&path, contents).map_err(|e| io_error(e, &path))?;
      Ok(Value::Null)
    }
    "list" => {
      let path = super::optional_string_arg(args, 0, "path")?.unwrap_or_else(|| ".".into());
      check_path(&path)?;
      let mut names = Vec::new();
      for entry in dir.read_dir(&path).map_err(|e| io_error(e, &path))? {
        let entry = entry.map_err(|e| io_error(e, &path))?;
        let mut name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
          name.push('/');
        }
        names.push(Value::String(name));
      }
      names.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
      Ok(Value::Array(names))
    }
    "remove" => {
      if read_only {
        return Err(CapabilityError::permission_denied("this directory is read-only"));
      }
      let path = path_arg(args, 0)?;
      dir.remove_file(&path).map_err(|e| io_error(e, &path))?;
      Ok(Value::Null)
    }
    _ => Err(CapabilityError::invalid_argument(format!("files@1 has no method `{method}`"))),
  }
}

fn path_arg(args: &[Value], index: usize) -> Result<String, CapabilityError> {
  let path = super::string_arg(args, index, "path")?;
  check_path(&path)?;
  Ok(path)
}

/// Gives clear errors for paths that are obviously outside the grant. cap-std
/// enforces the boundary regardless; this only improves the message.
fn check_path(path: &str) -> Result<(), CapabilityError> {
  if path.is_empty() {
    return Err(CapabilityError::invalid_argument("`path` must not be empty"));
  }
  if Path::new(path).is_absolute() || path.split(['/', '\\']).any(|seg| seg == "..") {
    return Err(CapabilityError::permission_denied(format!(
      "{path:?} is outside the granted directory"
    )));
  }
  Ok(())
}

/// Maps an I/O error to a capability error without revealing host paths.
fn io_error(err: std::io::Error, path: &str) -> CapabilityError {
  match err.kind() {
    IoErrorKind::NotFound => CapabilityError::not_found(format!("{path:?} does not exist")),
    // cap-std reports attempts to escape the root, including through
    // symlinks, as permission errors.
    IoErrorKind::PermissionDenied => {
      CapabilityError::permission_denied(format!("{path:?} is outside the granted directory"))
    }
    IoErrorKind::IsADirectory | IoErrorKind::NotADirectory | IoErrorKind::InvalidInput => {
      CapabilityError::invalid_argument(format!("{path:?}: {}", err.kind()))
    }
    _ => CapabilityError::internal(format!("{path:?}: {}", err.kind())),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn dir() -> (tempfile::TempDir, Dir) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = Dir::open_ambient_dir(tmp.path(), cap_std::ambient_authority()).unwrap();
    (tmp, dir)
  }

  fn s(v: &str) -> Value {
    Value::String(v.into())
  }

  #[test]
  fn reads_writes_lists_and_removes() {
    let (_tmp, dir) = dir();
    call(&dir, "writeText", &[s("a/b.txt"), s("hi")], false).unwrap();
    assert_eq!(call(&dir, "readText", &[s("a/b.txt")], false).unwrap(), s("hi"));
    assert_eq!(call(&dir, "list", &[], false).unwrap(), serde_json::json!(["a/"]));
    call(&dir, "remove", &[s("a/b.txt")], false).unwrap();
    let err = call(&dir, "readText", &[s("a/b.txt")], false).unwrap_err();
    assert_eq!(err.kind, cells_runtime::ErrorKind::NotFound);
  }

  #[test]
  fn refuses_to_leave_the_root() {
    let (tmp, dir) = dir();
    for path in ["../x", "/etc/passwd", "a/../../x"] {
      let err = call(&dir, "readText", &[s(path)], false).unwrap_err();
      assert_eq!(err.kind, cells_runtime::ErrorKind::PermissionDenied, "{path}");
    }
    #[cfg(unix)]
    {
      std::os::unix::fs::symlink("/etc", tmp.path().join("escape")).unwrap();
      let err = call(&dir, "readText", &[s("escape/hostname")], false).unwrap_err();
      assert_eq!(err.kind, cells_runtime::ErrorKind::PermissionDenied);
    }
    let _ = tmp;
  }

  #[test]
  fn read_only_grants_reject_writes() {
    let (_tmp, dir) = dir();
    let err = call(&dir, "writeText", &[s("a.txt"), s("x")], true).unwrap_err();
    assert_eq!(err.kind, cells_runtime::ErrorKind::PermissionDenied);
  }

  #[test]
  fn validates_arguments() {
    let (_tmp, dir) = dir();
    let err = call(&dir, "readText", &[Value::from(5)], false).unwrap_err();
    assert_eq!(err.kind, cells_runtime::ErrorKind::InvalidArgument);
    let err = call(&dir, "writeText", &[s("a.txt")], false).unwrap_err();
    assert_eq!(err.kind, cells_runtime::ErrorKind::InvalidArgument);
  }
}
