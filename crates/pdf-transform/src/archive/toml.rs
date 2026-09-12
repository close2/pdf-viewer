//! A reader for the small, fixed subset of TOML a remedy configuration is written in.
//!
//! # Why not a TOML crate
//!
//! The same argument `tools/conformance/src/toml_subset.rs` makes for the ledger, one crate over:
//! a configuration file is a data file this project documents and this project reads, and the
//! subset it needs is small enough to state in one place — a comment, a blank line, a table
//! header whose segments may be quoted (`[site."fonts/x".target."4f"]`), and a key holding a
//! string, a list of strings, a boolean, or an inline table of strings. `Cargo.toml` says why this
//! crate carries no serialisation dependency, and a parser for the whole of TOML would be one.
//!
//! **This is not a TOML parser.** It accepts the subset and *rejects* the rest, by line, naming
//! what it expected: an integer, a multi-line string, a nested inline table or an array of tables
//! fails to read rather than being misread. That is the only property that makes a restricted
//! reader safe to build on, and the day a configuration needs one of those the answer is a real
//! parser rather than another special case here.
//!
//! The ledger's reader is not reused because it reads a different subset — `[[name]]` headers
//! and bare keys only — and because a product crate depending on a tool crate would put the
//! dependency arrow the wrong way round.

use std::fmt;

/// A value a configuration may hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    /// A basic string: `remedy = "discard"`.
    Text(String),
    /// A single-line array of basic strings: `media-type = ["application/xml", "text/xml"]`.
    List(Vec<String>),
    /// `true` or `false`.
    Bool(bool),
    /// A single-line inline table whose values are basic strings:
    /// `media-types = { ".xml" = "application/xml" }`.
    Map(Vec<(String, String)>),
}

impl Value {
    /// The string, if this is one.
    #[must_use]
    pub(crate) fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The strings, taking a lone string as a list of one.
    #[must_use]
    pub(crate) fn as_list(&self) -> Option<Vec<&str>> {
        match self {
            Self::Text(text) => Some(vec![text.as_str()]),
            Self::List(items) => Some(items.iter().map(String::as_str).collect()),
            _ => None,
        }
    }

    /// The boolean, if this is one.
    #[must_use]
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The inline table's entries, if this is one.
    #[must_use]
    pub(crate) fn as_map(&self) -> Option<&[(String, String)]> {
        match self {
            Self::Map(entries) => Some(entries),
            _ => None,
        }
    }

    /// What kind of value this is, for an error naming the kind expected.
    #[must_use]
    pub(crate) const fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "a string",
            Self::List(_) => "a list of strings",
            Self::Bool(_) => "a boolean",
            Self::Map(_) => "an inline table",
        }
    }
}

/// One `key = value` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The key, with any quotation marks removed.
    pub(crate) key: String,
    /// The value.
    pub(crate) value: Value,
    /// The 1-based line it sits on.
    pub(crate) line: usize,
}

/// One `[a.b."c"]` table, with its keys in the order they were written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Table {
    /// The header's segments, each with any quotation marks removed.
    pub(crate) path: Vec<String>,
    /// The 1-based line the header sits on.
    pub(crate) line: usize,
    /// The keys, in file order. A key may not repeat within a table.
    pub(crate) entries: Vec<Entry>,
}

impl Table {
    /// The value of a key, if the table has it.
    #[must_use]
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| &entry.value)
    }
}

/// Why a file is not in the subset.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {expected}")]
pub(crate) struct TomlError {
    /// The 1-based line the reader stopped on.
    pub(crate) line: usize,
    /// What the reader expected to find there.
    pub(crate) expected: String,
}

impl TomlError {
    fn at(line: usize, expected: impl Into<String>) -> Self {
        Self {
            line,
            expected: expected.into(),
        }
    }
}

/// Reads the subset.
///
/// # Errors
///
/// On anything the subset does not cover, naming the line and what was expected. A key before
/// the first table header, a repeated key in one table, a repeated table header and an
/// unterminated string are all errors: a configuration is written by a person, and anything
/// surprising in it is that person's edit going wrong, which they would rather be told about.
pub(crate) fn parse(text: &str) -> Result<Vec<Table>, TomlError> {
    let mut tables: Vec<Table> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index.saturating_add(1);
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('[') {
            if rest.starts_with('[') {
                return Err(TomlError::at(
                    line,
                    "a `[table]` header; an array of tables `[[…]]` is outside the subset this \
                     reader accepts",
                ));
            }
            let (inside, after) = rest
                .split_once(']')
                .ok_or_else(|| TomlError::at(line, "a `[table]` header closed on the same line"))?;
            if !after.trim().is_empty() && !after.trim().starts_with('#') {
                return Err(TomlError::at(
                    line,
                    "nothing after a table header but a comment",
                ));
            }
            let path = header_path(inside, line)?;
            if tables.iter().any(|table| table.path == path) {
                return Err(TomlError::at(
                    line,
                    format!(
                        "a table stated once; `[{}]` was already opened",
                        inside.trim()
                    ),
                ));
            }
            tables.push(Table {
                path,
                line,
                entries: Vec::new(),
            });
            continue;
        }
        let (key, rest) = split_key(trimmed, line)?;
        let value = parse_value(rest, line)?;
        let Some(table) = tables.last_mut() else {
            return Err(TomlError::at(
                line,
                "a `[table]` header before the first key; a configuration has no top-level keys",
            ));
        };
        if table.entries.iter().any(|entry| entry.key == key) {
            return Err(TomlError::at(
                line,
                format!("a key stated once; `{key}` repeats"),
            ));
        }
        table.entries.push(Entry { key, value, line });
    }
    Ok(tables)
}

/// The segments of a header, `a.b."c d"`, each bare or quoted.
fn header_path(inside: &str, line: usize) -> Result<Vec<String>, TomlError> {
    let mut path = Vec::new();
    let mut rest = inside.trim();
    if rest.is_empty() {
        return Err(TomlError::at(
            line,
            "a table header with at least one segment",
        ));
    }
    loop {
        rest = rest.trim_start();
        let (segment, after) = if rest.starts_with('"') {
            let (text, after) = quoted(rest, line)?;
            (text, after)
        } else {
            let end = rest.find('.').unwrap_or(rest.len());
            let bare = rest.get(..end).unwrap_or_default().trim();
            if !is_bare_key(bare) {
                return Err(TomlError::at(
                    line,
                    format!(
                        "a bare header segment of letters, digits, `_` and `-`, or a quoted \
                         one; {bare:?} is neither"
                    ),
                ));
            }
            (bare.to_owned(), rest.get(end..).unwrap_or_default())
        };
        path.push(segment);
        let after = after.trim_start();
        if after.is_empty() {
            return Ok(path);
        }
        let Some(next) = after.strip_prefix('.') else {
            return Err(TomlError::at(
                line,
                "a `.` between header segments, or the end of the header",
            ));
        };
        rest = next;
    }
}

/// `key = rest`, the key bare or quoted.
fn split_key(text: &str, line: usize) -> Result<(String, &str), TomlError> {
    let (key, after) = if text.starts_with('"') {
        quoted(text, line)?
    } else {
        let end = text.find(['=', ' ', '\t']).unwrap_or(text.len());
        let bare = text.get(..end).unwrap_or_default();
        if !is_bare_key(bare) {
            return Err(TomlError::at(
                line,
                "`key = value`, a `[table]` header, a `#` comment or a blank line",
            ));
        }
        (bare.to_owned(), text.get(end..).unwrap_or_default())
    };
    let after = after.trim_start();
    let Some(rest) = after.strip_prefix('=') else {
        return Err(TomlError::at(line, format!("`=` after the key `{key}`")));
    };
    Ok((key, rest.trim()))
}

/// Whether a bare key or header segment is spelled as TOML permits.
fn is_bare_key(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// One value, with nothing but a comment after it.
fn parse_value(text: &str, line: usize) -> Result<Value, TomlError> {
    let (value, rest) = value_prefix(text, line)?;
    let rest = rest.trim();
    if !rest.is_empty() && !rest.starts_with('#') {
        return Err(TomlError::at(
            line,
            format!("nothing after the value but a comment; found {rest:?}"),
        ));
    }
    Ok(value)
}

/// One value at the start of `text`, and what follows it.
fn value_prefix(text: &str, line: usize) -> Result<(Value, &str), TomlError> {
    if text.starts_with('"') {
        let (string, rest) = quoted(text, line)?;
        return Ok((Value::Text(string), rest));
    }
    if let Some(rest) = text.strip_prefix("true") {
        return Ok((Value::Bool(true), rest));
    }
    if let Some(rest) = text.strip_prefix("false") {
        return Ok((Value::Bool(false), rest));
    }
    if let Some(rest) = text.strip_prefix('[') {
        return list(rest, line);
    }
    if let Some(rest) = text.strip_prefix('{') {
        return map(rest, line);
    }
    Err(TomlError::at(
        line,
        "a value that is a \"string\", a [\"list\", \"of\", \"strings\"], true, false, or an \
         inline table { \"key\" = \"value\" }; integers, dates and multi-line strings are outside \
         the subset this reader accepts",
    ))
}

/// The rest of a `[…]` list of strings.
fn list(mut rest: &str, line: usize) -> Result<(Value, &str), TomlError> {
    let mut items = Vec::new();
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix(']') {
            return Ok((Value::List(items), after));
        }
        if !rest.starts_with('"') {
            return Err(TomlError::at(
                line,
                "a list holding only \"strings\", closed on the same line",
            ));
        }
        let (item, after) = quoted(rest, line)?;
        items.push(item);
        rest = after.trim_start();
        if let Some(after) = rest.strip_prefix(',') {
            rest = after;
        } else if !rest.starts_with(']') {
            return Err(TomlError::at(line, "`,` or `]` after a list item"));
        }
    }
}

/// The rest of a `{…}` inline table of strings.
fn map(mut rest: &str, line: usize) -> Result<(Value, &str), TomlError> {
    let mut entries: Vec<(String, String)> = Vec::new();
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix('}') {
            return Ok((Value::Map(entries), after));
        }
        let (key, after) = split_key(rest, line)?;
        if entries.iter().any(|(seen, _)| *seen == key) {
            return Err(TomlError::at(
                line,
                format!("a key stated once in an inline table; `{key}` repeats"),
            ));
        }
        if !after.starts_with('"') {
            return Err(TomlError::at(
                line,
                "an inline table holding only \"string\" values, closed on the same line",
            ));
        }
        let (value, after) = quoted(after, line)?;
        entries.push((key, value));
        rest = after.trim_start();
        if let Some(after) = rest.strip_prefix(',') {
            rest = after;
        } else if !rest.starts_with('}') {
            return Err(TomlError::at(
                line,
                "`,` or `}` after an inline table entry",
            ));
        }
    }
}

/// A basic string starting at `text`'s opening quotation mark, decoded, and what follows it.
///
/// The escapes are TOML's for a basic string — `\"`, `\\`, `\n`, `\t`, `\r`, `\uXXXX` and
/// `\UXXXXXXXX`; anything else after a backslash is an error rather than a guess.
fn quoted(text: &str, line: usize) -> Result<(String, &str), TomlError> {
    let mut out = String::new();
    let mut chars = text.char_indices();
    let Some((_, '"')) = chars.next() else {
        return Err(TomlError::at(line, "an opening quotation mark"));
    };
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                let after = text.get(at.saturating_add(1)..).unwrap_or_default();
                return Ok((out, after));
            }
            '\\' => {
                let Some((_, escaped)) = chars.next() else {
                    break;
                };
                match escaped {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'u' | 'U' => {
                        let width = if escaped == 'u' { 4 } else { 8 };
                        let mut digits = String::new();
                        for _ in 0..width {
                            match chars.next() {
                                Some((_, digit)) if digit.is_ascii_hexdigit() => {
                                    digits.push(digit);
                                }
                                _ => {
                                    return Err(TomlError::at(
                                        line,
                                        format!("{width} hexadecimal digits after `\\{escaped}`"),
                                    ));
                                }
                            }
                        }
                        let code = u32::from_str_radix(&digits, 16).map_err(|_| {
                            TomlError::at(line, "a hexadecimal escape this reader can decode")
                        })?;
                        out.push(char::from_u32(code).ok_or_else(|| {
                            TomlError::at(line, format!("a Unicode scalar value, not U+{digits}"))
                        })?);
                    }
                    other => {
                        return Err(TomlError::at(
                            line,
                            format!(
                                "an escape this reader knows (\\\" \\\\ \\n \\t \\r \\u \\U), \
                                 not `\\{other}`"
                            ),
                        ));
                    }
                }
            }
            other => out.push(other),
        }
    }
    Err(TomlError::at(
        line,
        "a closing quotation mark on the same line",
    ))
}

impl fmt::Display for Value {
    /// The value as it would be written, for a report that quotes a configuration back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "{text:?}"),
            Self::List(items) => {
                f.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item:?}")?;
                }
                f.write_str("]")
            }
            Self::Bool(value) => write!(f, "{value}"),
            Self::Map(entries) => {
                f.write_str("{ ")?;
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key:?} = {value:?}")?;
                }
                f.write_str(" }")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_header_and_every_value_kind_read_back() {
        let text = "\
# a comment
[profile]
name = \"as-if-printed\"   # trailing comment
default = \"stop\"

[site.\"embedded-files/x\".target.\"4f\"]
remedy = \"preserve\"
prefer = [\"attach-unchanged\", \"append-pages\"]
fresh-packet = true
media-types = { \".xml\" = \"application/xml\", \".csv\" = \"text/csv\" }
\"quoted key\" = \"v\\u00e9\"
";
        let tables = parse(text).expect("the subset reads");
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].path, vec!["profile".to_owned()]);
        assert_eq!(
            tables[0].get("name").and_then(Value::as_text),
            Some("as-if-printed")
        );
        assert_eq!(
            tables[1].path,
            vec![
                "site".to_owned(),
                "embedded-files/x".to_owned(),
                "target".to_owned(),
                "4f".to_owned()
            ]
        );
        assert_eq!(
            tables[1].get("prefer").and_then(Value::as_list),
            Some(vec!["attach-unchanged", "append-pages"])
        );
        assert_eq!(
            tables[1].get("fresh-packet").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            tables[1].get("media-types").and_then(Value::as_map),
            Some(
                &[
                    (".xml".to_owned(), "application/xml".to_owned()),
                    (".csv".to_owned(), "text/csv".to_owned())
                ][..]
            )
        );
        assert_eq!(
            tables[1].get("quoted key").and_then(Value::as_text),
            Some("vé")
        );
    }

    #[test]
    fn what_is_outside_the_subset_is_refused_by_line() {
        for (text, line, word) in [
            ("[[site]]\n", 1, "array of tables"),
            ("key = \"x\"\n", 1, "before the first key"),
            ("[a]\nkey = 1\n", 2, "integers"),
            ("[a]\nkey = \"open\n", 2, "closing quotation mark"),
            ("[a]\nkey = \"x\"\nkey = \"y\"\n", 3, "repeats"),
            ("[a]\n[a]\n", 2, "already opened"),
            ("[a]\nk = [\"x\" \"y\"]\n", 2, "`,` or `]`"),
            ("[a]\nk = { \"x\" = 1 }\n", 2, "only \"string\" values"),
            ("[a]\nk = \"\\q\"\n", 2, "escape"),
            ("[a]\nk = \"x\" trailing\n", 2, "nothing after the value"),
        ] {
            let error = parse(text).expect_err(text);
            assert_eq!(error.line, line, "{text:?}");
            assert!(
                error.expected.contains(word),
                "{text:?}: {}",
                error.expected
            );
        }
    }
}
