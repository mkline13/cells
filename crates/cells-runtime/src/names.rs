//! Validation of grant names, which become capability globals (§6.1).

/// Names defined by web standards. A grant may use one of these only if its
/// contract implements that standard.
const WEB_STANDARD_NAMES: &[(&str, Option<&str>)] = &[
  ("console", Some("console@1")),
  ("fetch", None),
  ("crypto", None),
  ("performance", None),
  ("navigator", None),
  ("location", None),
  ("localStorage", None),
  ("sessionStorage", None),
  ("indexedDB", None),
  ("caches", None),
  ("WebSocket", None),
  ("XMLHttpRequest", None),
  ("window", None),
  ("self", None),
  ("document", None),
];

/// JavaScript reserved words, which cannot be used as bare identifiers.
const RESERVED_WORDS: &[&str] = &[
  "await", "break", "case", "catch", "class", "const", "continue", "debugger",
  "default", "delete", "do", "else", "enum", "export", "extends", "false",
  "finally", "for", "function", "if", "implements", "import", "in",
  "instanceof", "interface", "let", "new", "null", "package", "private",
  "protected", "public", "return", "static", "super", "switch", "this",
  "throw", "true", "try", "typeof", "var", "void", "while", "with", "yield",
  "arguments", "eval", "undefined", "NaN", "Infinity", "globalThis",
];

/// Checks a grant name before a cell starts.
///
/// This catches names that are not usable as identifiers and web-standard
/// names used by the wrong contract. Collisions with ECMAScript built-ins and
/// baseline globals are caught when the cell's globals are built, because the
/// bootstrap script checks each name against the real global object.
pub(crate) fn validate_grant_name(name: &str, contract: &str) -> Result<(), String> {
  let mut chars = name.chars();
  let valid_identifier = chars
    .next()
    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
  if !valid_identifier || RESERVED_WORDS.contains(&name) {
    return Err(format!(
      "grant name {name:?} is not a valid JavaScript identifier"
    ));
  }
  if let Some((_, allowed)) = WEB_STANDARD_NAMES.iter().find(|(n, _)| *n == name)
    && *allowed != Some(contract)
  {
    return Err(match allowed {
      Some(expected) => format!(
        "grant name {name:?} is a web standard name and may only be used for {expected}"
      ),
      None => format!(
        "grant name {name:?} is a web standard name and no contract implements that standard yet"
      ),
    });
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::validate_grant_name;

  #[test]
  fn accepts_identifiers() {
    assert!(validate_grant_name("notes", "files@1").is_ok());
    assert!(validate_grant_name("_cache2", "kv@1").is_ok());
    assert!(validate_grant_name("console", "console@1").is_ok());
  }

  #[test]
  fn rejects_bad_names() {
    assert!(validate_grant_name("", "kv@1").is_err());
    assert!(validate_grant_name("2fast", "kv@1").is_err());
    assert!(validate_grant_name("my-files", "files@1").is_err());
    assert!(validate_grant_name("class", "kv@1").is_err());
    assert!(validate_grant_name("console", "files@1").is_err());
    assert!(validate_grant_name("fetch", "kv@1").is_err());
  }
}
