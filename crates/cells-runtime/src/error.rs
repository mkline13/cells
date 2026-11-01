//! Errors returned by capability operations.

use std::fmt;

use serde::Serialize;

/// The category of a capability error.
///
/// Applications see this as the `kind` property of the rejected error, for
/// example `err.kind === "permission-denied"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorKind {
  /// The cell does not hold a valid grant for this capability, or the
  /// operation falls outside the grant's scope.
  PermissionDenied,
  /// The arguments were malformed or the method does not exist.
  InvalidArgument,
  /// The requested resource does not exist.
  NotFound,
  /// The provider cannot currently perform the operation.
  Unavailable,
  /// Something went wrong inside the host.
  Internal,
}

impl ErrorKind {
  /// The name applications see in `err.kind`.
  pub fn as_str(self) -> &'static str {
    match self {
      ErrorKind::PermissionDenied => "permission-denied",
      ErrorKind::InvalidArgument => "invalid-argument",
      ErrorKind::NotFound => "not-found",
      ErrorKind::Unavailable => "unavailable",
      ErrorKind::Internal => "internal",
    }
  }
}

/// An error returned to the application by a capability operation.
///
/// The message is shown to application code, so it must not contain host
/// details the application should not learn, such as absolute host paths or
/// credentials.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CapabilityError {
  pub kind: ErrorKind,
  pub message: String,
}

impl CapabilityError {
  pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
    Self {
      kind,
      message: message.into(),
    }
  }

  pub fn permission_denied(message: impl Into<String>) -> Self {
    Self::new(ErrorKind::PermissionDenied, message)
  }

  pub fn invalid_argument(message: impl Into<String>) -> Self {
    Self::new(ErrorKind::InvalidArgument, message)
  }

  pub fn not_found(message: impl Into<String>) -> Self {
    Self::new(ErrorKind::NotFound, message)
  }

  pub fn unavailable(message: impl Into<String>) -> Self {
    Self::new(ErrorKind::Unavailable, message)
  }

  pub fn internal(message: impl Into<String>) -> Self {
    Self::new(ErrorKind::Internal, message)
  }
}

impl fmt::Display for CapabilityError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.kind.as_str(), self.message)
  }
}

impl std::error::Error for CapabilityError {}
