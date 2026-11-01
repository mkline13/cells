//! The controlled module loader (PROPOSAL.md §7.3, §7.4).
//!
//! Applications import modules with native ES module syntax. Every module is
//! identified by a virtual URL under `cell:///`, which maps to a path inside
//! the application's source directory:
//!
//! ```text
//! cell:///main.ts          → <source root>/main.ts
//! cell:///lib/greeting.ts  → <source root>/lib/greeting.ts
//! ```
//!
//! Two layers keep imports inside the source directory:
//!
//! 1. Resolution only accepts `cell:` URLs. Relative specifiers are resolved
//!    against the importing module's URL, and URL resolution cannot climb
//!    above `cell:///`, so `../../etc/passwd` becomes `cell:///etc/passwd`.
//!    Absolute URLs with any other scheme (`file:`, `https:`, `ext:`, ...)
//!    and bare specifiers (`lodash`) are rejected.
//! 2. Files are read through a `cap_std::fs::Dir` opened at the source root.
//!    It refuses paths that escape the directory, including through
//!    symlinks.
//!
//! Virtual URLs also keep host paths out of stack traces and error messages.

use std::path::Path;

use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceMapOption;
use deno_core::ModuleLoadOptions;
use deno_core::ModuleLoadReferrer;
use deno_core::ModuleLoadResponse;
use deno_core::ModuleLoader;
use deno_core::ModuleSource;
use deno_core::ModuleSourceCode;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;
use deno_core::ResolutionKind;
use deno_core::error::ModuleLoaderError;
use deno_core::resolve_import;
use deno_error::JsErrorBox;

/// The scheme of every application module URL.
pub(crate) const CELL_SCHEME: &str = "cell";

/// Builds the URL of a module from its path relative to the source root.
pub(crate) fn module_url(relative_path: &str) -> Result<ModuleSpecifier, String> {
  let relative_path = relative_path.trim_start_matches("./");
  let url = ModuleSpecifier::parse(&format!("{CELL_SCHEME}:///"))
    .and_then(|root| root.join(relative_path))
    .map_err(|e| format!("invalid entry point {relative_path:?}: {e}"))?;
  if url.scheme() != CELL_SCHEME || url.path() == "/" {
    return Err(format!("invalid entry point {relative_path:?}"));
  }
  Ok(url)
}

pub(crate) struct CellModuleLoader {
  root: cap_std::fs::Dir,
}

impl CellModuleLoader {
  pub(crate) fn open(source_root: &Path) -> std::io::Result<Self> {
    // This is the only ambient filesystem access the runtime performs on an
    // application's behalf, and it is confined to the source directory.
    let root =
      cap_std::fs::Dir::open_ambient_dir(source_root, cap_std::ambient_authority())?;
    Ok(Self { root })
  }

  fn load_source(&self, url: &ModuleSpecifier) -> Result<ModuleSource, ModuleLoaderError> {
    let path = relative_path(url)?;
    let media_type = MediaType::from_specifier(url);
    let (module_type, transpile) = match media_type {
      MediaType::JavaScript | MediaType::Mjs => (ModuleType::JavaScript, false),
      MediaType::TypeScript | MediaType::Mts => (ModuleType::JavaScript, true),
      MediaType::Json => (ModuleType::Json, false),
      _ => {
        return Err(JsErrorBox::type_error(format!(
          "Cannot import {url}: only .ts, .mts, .js, .mjs, and .json modules are supported"
        )));
      }
    };

    let text = self.root.read_to_string(&path).map_err(|e| {
      // Do not include the host path in the message.
      let reason = match e.kind() {
        std::io::ErrorKind::NotFound => "module not found".to_string(),
        std::io::ErrorKind::PermissionDenied => {
          "path is outside the application's source directory".to_string()
        }
        _ => e.kind().to_string(),
      };
      JsErrorBox::type_error(format!("Cannot import {url}: {reason}"))
    })?;

    let code = if transpile {
      self.transpile(url, text, media_type)?
    } else {
      text
    };
    Ok(ModuleSource::new(
      module_type,
      ModuleSourceCode::String(code.into()),
      url,
      None,
    ))
  }

  /// Strips TypeScript syntax. Types are not checked (§7.3).
  ///
  /// The output carries an inline source map whose sources are the original
  /// `cell:///` URLs, with the TypeScript text embedded. Stack traces and the
  /// debugger use it to point at TypeScript lines.
  fn transpile(
    &self,
    url: &ModuleSpecifier,
    text: String,
    media_type: MediaType,
  ) -> Result<String, ModuleLoaderError> {
    let parsed = deno_ast::parse_module(ParseParams {
      specifier: url.clone(),
      text: text.into(),
      media_type,
      capture_tokens: false,
      scope_analysis: false,
      maybe_syntax: None,
    })
    .map_err(JsErrorBox::from_err)?;
    let emitted = parsed
      .transpile(
        &deno_ast::TranspileOptions {
          imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
          decorators: deno_ast::DecoratorsTranspileOption::Ecma,
          ..Default::default()
        },
        &deno_ast::TranspileModuleOptions { module_kind: None },
        &deno_ast::EmitOptions {
          // Inline, so both deno_core (for stack traces) and the
          // debugger (to show TypeScript) find it in the emitted code.
          source_map: SourceMapOption::Inline,
          inline_sources: true,
          ..Default::default()
        },
      )
      .map_err(JsErrorBox::from_err)?
      .into_source();
    Ok(emitted.text)
  }
}

/// Converts a `cell:///a/b.ts` URL to the relative path `a/b.ts`.
fn relative_path(url: &ModuleSpecifier) -> Result<String, ModuleLoaderError> {
  if url.scheme() != CELL_SCHEME || url.has_host() {
    return Err(JsErrorBox::type_error(format!(
      "Cannot import {url}: applications may only import their own modules"
    )));
  }
  let path = percent_decode(url.path().trim_start_matches('/'));
  if path.is_empty() || path.split('/').any(|seg| seg == ".." || seg == ".") {
    return Err(JsErrorBox::type_error(format!("Cannot import {url}: invalid path")));
  }
  Ok(path)
}

fn percent_decode(input: &str) -> String {
  let bytes = input.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'%'
      && i + 2 < bytes.len()
      && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
    {
      out.push(h * 16 + l);
      i += 3;
      continue;
    }
    out.push(bytes[i]);
    i += 1;
  }
  String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
  (b as char).to_digit(16).map(|d| d as u8)
}

impl ModuleLoader for CellModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    referrer: &str,
    _kind: ResolutionKind,
  ) -> Result<ModuleSpecifier, ModuleLoaderError> {
    // `resolve_import` rejects bare specifiers and resolves relative ones
    // against the referrer. This applies equally to static imports, dynamic
    // `import()`, and `import.meta.resolve`.
    let url = resolve_import(specifier, referrer).map_err(JsErrorBox::from_err)?;
    if url.scheme() != CELL_SCHEME || url.has_host() {
      return Err(JsErrorBox::type_error(format!(
        "Cannot import {specifier:?}: applications may only import their own modules"
      )));
    }
    Ok(url)
  }

  fn load(
    &self,
    url: &ModuleSpecifier,
    _referrer: Option<&ModuleLoadReferrer>,
    _options: ModuleLoadOptions,
  ) -> ModuleLoadResponse {
    ModuleLoadResponse::Sync(self.load_source(url))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn relative_imports_cannot_climb_above_the_root() {
    let main = module_url("main.ts").unwrap();
    let url = resolve_import("../../etc/passwd", main.as_str()).unwrap();
    assert_eq!(url.as_str(), "cell:///etc/passwd");
  }

  #[test]
  fn entry_points_map_to_cell_urls() {
    assert_eq!(module_url("main.ts").unwrap().as_str(), "cell:///main.ts");
    assert_eq!(module_url("./src/main.ts").unwrap().as_str(), "cell:///src/main.ts");
    assert!(module_url("").is_err());
  }

  #[test]
  fn paths_are_decoded() {
    let url = ModuleSpecifier::parse("cell:///my%20dir/a.ts").unwrap();
    assert_eq!(relative_path(&url).unwrap(), "my dir/a.ts");
    // The URL parser already removes `%2e%2e` segments...
    let url = ModuleSpecifier::parse("cell:///a/%2e%2e/b.ts").unwrap();
    assert_eq!(relative_path(&url).unwrap(), "b.ts");
    // ...but an encoded slash only becomes `..` after decoding.
    let url = ModuleSpecifier::parse("cell:///..%2Fsecret.ts").unwrap();
    assert!(relative_path(&url).is_err());
  }
}
