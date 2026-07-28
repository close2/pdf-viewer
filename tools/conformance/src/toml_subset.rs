//! A reader and writer for the small, fixed subset of TOML the ledger is written in.
//!
//! # Why not a TOML crate
//!
//! The ledger is a data file this project generates and this project reads, and the
//! conformance gate is the last thing that should stop running because a dependency did not
//! build. The subset it needs is four constructs — a comment, a blank line, an
//! array-of-tables header, and a key holding a string or a list of strings — and reading
//! exactly those is under two hundred lines with the grammar stated in one place.
//!
//! The cost is written down rather than hidden: **this is not a TOML parser**. It accepts a
//! subset and *rejects* the rest, by line, with a message naming what it expected. Valid
//! TOML that steps outside the subset — an inline table, a multi-line string, an integer —
//! fails to read rather than being misread, which is the only property that makes a
//! restricted reader safe to build. If the ledger ever needs those, the answer is a real
//! parser, not another special case here.

use std::fmt;

/// A value the ledger may hold: a string, or a list of strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A basic string: `status = "implemented"`.
    Text(String),
    /// A single-line array of basic strings: `code = ["a.rs", "b.rs"]`.
    List(Vec<String>),
}

impl Value {
    /// The string, if this is one.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::List(_) => None,
        }
    }

    /// The values, taking a lone string as a list of one — the ledger's `code` and `test`
    /// keys are written both ways and mean the same thing.
    #[must_use]
    pub fn as_list(&self) -> Vec<&str> {
        match self {
            Self::Text(text) => vec![text.as_str()],
            Self::List(items) => items.iter().map(String::as_str).collect(),
        }
    }
}

/// One `[[name]]` table, with its keys in the order they were written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The header's name.
    pub name: String,
    /// The 1-based line the header sits on.
    pub line: usize,
    /// The keys, in file order. A key may not repeat.
    pub entries: Vec<(String, Value)>,
}

impl Table {
    /// The value of a key, if the table has it.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }
}

/// Why a file is not in the subset.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {expected}")]
pub struct TomlError {
    /// The 1-based line the reader stopped on.
    pub line: usize,
    /// What the reader expected to find there.
    pub expected: String,
}

/// Reads the subset.
///
/// # Errors
///
/// On anything the subset does not cover, naming the line and what was expected. A key
/// before the first table header, a repeated key and an unterminated string are all errors:
/// the ledger is generated, so anything surprising in it is a person's edit going wrong.
pub fn parse(text: &str) -> Result<Vec<Table>, TomlError> {
    let mut tables: Vec<Table> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index.saturating_add(1);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("[[") {
            let Some(name) = rest.strip_suffix("]]") else {
                return Err(TomlError {
                    line: line_number,
                    expected: "an array-of-tables header, `[[name]]`".to_owned(),
                });
            };
            tables.push(Table {
                name: name.trim().to_owned(),
                line: line_number,
                entries: Vec::new(),
            });
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(TomlError {
                line: line_number,
                expected: "`key = value`, a `[[name]]` header, a `#` comment or a blank line"
                    .to_owned(),
            });
        };
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(TomlError {
                line: line_number,
                expected: "a bare key of letters, digits and underscores".to_owned(),
            });
        }
        let value = read_value(value.trim(), line_number)?;
        let Some(table) = tables.last_mut() else {
            return Err(TomlError {
                line: line_number,
                expected: "a `[[name]]` header before the first key".to_owned(),
            });
        };
        if table.entries.iter().any(|(name, _)| name == key) {
            return Err(TomlError {
                line: line_number,
                expected: format!("no second `{key}` in this table"),
            });
        }
        table.entries.push((key.to_owned(), value));
    }
    Ok(tables)
}

fn read_value(text: &str, line: usize) -> Result<Value, TomlError> {
    if let Some(inner) = text.strip_prefix('[') {
        let Some(inner) = inner.strip_suffix(']') else {
            return Err(TomlError {
                line,
                expected: "a single-line array, `[\"a\", \"b\"]`".to_owned(),
            });
        };
        let mut items = Vec::new();
        let mut rest = inner.trim();
        while !rest.is_empty() {
            let (item, after) = read_string(rest, line)?;
            items.push(item);
            rest = after.trim_start();
            if let Some(after_comma) = rest.strip_prefix(',') {
                rest = after_comma.trim_start();
            } else if !rest.is_empty() {
                return Err(TomlError {
                    line,
                    expected: "a comma between array items".to_owned(),
                });
            }
        }
        return Ok(Value::List(items));
    }
    let (text, rest) = read_string(text, line)?;
    if !rest.trim().is_empty() {
        return Err(TomlError {
            line,
            expected: "nothing after the value".to_owned(),
        });
    }
    Ok(Value::Text(text))
}

/// Reads one basic string, returning it and whatever follows the closing quote.
fn read_string(text: &str, line: usize) -> Result<(String, &str), TomlError> {
    let expected = |what: &str| TomlError {
        line,
        expected: what.to_owned(),
    };
    let mut characters = text.char_indices();
    match characters.next() {
        Some((_, '"')) => {}
        _ => return Err(expected("a basic string in double quotes")),
    }
    let mut value = String::new();
    while let Some((position, character)) = characters.next() {
        match character {
            '"' => {
                let after = position.saturating_add(1);
                return Ok((value, text.get(after..).unwrap_or_default()));
            }
            '\\' => {
                let Some((_, escape)) = characters.next() else {
                    return Err(expected("an escape after a backslash"));
                };
                match escape {
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    _ => return Err(expected("one of the escapes `\\\\`, `\\\"`, `\\n`, `\\t`")),
                }
            }
            control if control.is_control() => {
                return Err(expected("no control character inside a string"));
            }
            character => value.push(character),
        }
    }
    Err(expected("a closing double quote"))
}

/// Writes one basic string, escaping exactly what [`parse`] can read back.
pub(crate) fn write_string(out: &mut String, value: &str) -> fmt::Result {
    use fmt::Write as _;

    out.push('"');
    for character in value.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            // The reader rejects raw control characters, so writing one would produce a
            // file this program cannot read back. Nothing in the ledger has one; if
            // something ever does, it must be seen rather than silently mangled.
            control if control.is_control() => {
                write!(out, "\\u{:04X}", u32::from(control))?;
            }
            character => out.push(character),
        }
    }
    out.push('"');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_shape_the_ledger_is_written_in() {
        let tables = parse(
            "# a comment\n\n[[clause]]\nclause = \"8.9.6.2\"\nstatus = \"reported\"\n\
             code = [\"a.rs\", \"b.rs\"]\n\n[[clause]]\nclause = \"8.9.6.3\"\n",
        )
        .unwrap();
        assert_eq!(tables.len(), 2);
        let first = tables.first().unwrap();
        assert_eq!(first.name, "clause");
        assert_eq!(first.get("status").unwrap().as_text(), Some("reported"));
        assert_eq!(first.get("code").unwrap().as_list(), vec!["a.rs", "b.rs"]);
        assert_eq!(first.get("note"), None);
    }

    /// A lone string is a list of one, because `code = "x.rs"` and `code = ["x.rs"]` say
    /// the same thing and a ledger written by hand will use both.
    #[test]
    fn a_string_reads_as_a_list_of_one() {
        let tables = parse("[[clause]]\ncode = \"x.rs\"\n").unwrap();
        assert_eq!(
            tables.first().unwrap().get("code").unwrap().as_list(),
            vec!["x.rs"]
        );
    }

    #[test]
    fn what_the_subset_does_not_cover_is_rejected_rather_than_misread() {
        // An inline table, an integer, a bare value, a key with no table, a repeated key.
        for (text, line) in [
            ("[[clause]]\nx = { a = \"b\" }\n", 2),
            ("[[clause]]\nx = 3\n", 2),
            ("[[clause]]\nx = unquoted\n", 2),
            ("clause = \"8\"\n", 1),
            ("[[clause]]\nx = \"a\"\nx = \"b\"\n", 3),
            ("[[clause]]\nx = \"unterminated\n", 2),
            ("[clause]\n", 1),
        ] {
            let error = parse(text).unwrap_err();
            assert_eq!(error.line, line, "for {text:?}");
        }
    }

    #[test]
    fn a_string_survives_the_round_trip_it_needs() {
        let awkward = "a \"quoted\" \\ backslash\nand a newline";
        let mut written = String::from("[[clause]]\nnote = ");
        write_string(&mut written, awkward).unwrap();
        written.push('\n');
        let tables = parse(&written).unwrap();
        assert_eq!(
            tables.first().unwrap().get("note").unwrap().as_text(),
            Some(awkward)
        );
    }
}
