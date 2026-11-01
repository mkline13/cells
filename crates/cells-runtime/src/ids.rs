//! Identifiers for cells and grants.

use std::fmt;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use serde::Serialize;

/// Identifies one running cell.
///
/// Cell identifiers are unique within a process. They tag a cell's output and
/// key its grants, so a grant handle is only meaningful together with the
/// cell it was issued to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct CellId(u64);

impl CellId {
  /// Allocates a new, process-unique cell identifier.
  pub fn next() -> Self {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Self(NEXT.fetch_add(1, Ordering::Relaxed))
  }

  /// The numeric value of the identifier.
  pub fn get(self) -> u64 {
    self.0
  }
}

impl fmt::Display for CellId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "cell-{}", self.0)
  }
}

/// An opaque reference to one grant held by one cell.
///
/// Each capability global wraps a handle. Handles are only unique within a
/// cell: the dispatcher always looks them up together with the calling
/// cell's [`CellId`], so a forged handle, or one copied from another cell,
/// carries no authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct GrantHandle(u32);

impl GrantHandle {
  /// Creates a handle from its numeric value.
  pub fn from_raw(raw: u32) -> Self {
    Self(raw)
  }

  /// The numeric value of the handle.
  pub fn get(self) -> u32 {
    self.0
  }
}

impl fmt::Display for GrantHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "grant-{}", self.0)
  }
}
