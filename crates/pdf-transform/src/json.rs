//! Just enough JSON to write a report, and none to read one.
//!
//! The same module, and the same argument, as `tools/pdf-retrieve/src/json.rs`: the report is a
//! *fixed* shape this crate writes and never parses, so what a serialisation crate would buy is
//! derive macros for six kinds of value, and `doc/third-party-data.md`'s rule is that a
//! dependency is taken for something hard or something dangerous to get wrong. Escaping is the
//! second of those and it is twenty lines, stated by RFC 8259 section 7. It is a second copy
//! rather than a dependency on `pdf-retrieve` because a shipped crate does not depend on an
//! instrument (RFC 0002 section 5, ADR 0800), and the copy is small enough to read whole.
//!
//! **Every code point below U+0020 is escaped**: a page label or an attachment's description is
//! the document's text and may carry any of them.

use std::fmt::Write as _;

/// One JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`, written for an entry the document does not state.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A signed integer: a count, an index, a page, a byte size.
    Integer(i64),
    /// A real number: a measurement rather than a count.
    ///
    /// RFC 8259 section 6 admits one and gives it no range or precision, so what is written is
    /// Rust's own shortest representation that reads back as the same `f64` — which makes the
    /// report deterministic, since the same measurement is the same digits on every run. A value
    /// JSON has no spelling for (an infinity, a NaN) is written as `null` rather than as a token
    /// no parser accepts; nothing in this report can produce one, and a number that cannot be
    /// written is better said to be absent than to break the document around it.
    Number(f64),
    /// A string, escaped on the way out.
    Text(String),
    /// An array, in the order given.
    Array(Vec<Value>),
    /// An object, in the order given — insertion order rather than sorted, because a report
    /// reads better with its identifying fields first and JSON objects are unordered anyway.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// A string value, from anything that is one.
    #[must_use]
    pub fn text(from: impl Into<String>) -> Self {
        Self::Text(from.into())
    }

    /// A string value where there is one, `null` where there is not.
    #[must_use]
    pub fn optional(from: Option<impl Into<String>>) -> Self {
        from.map_or(Self::Null, Self::text)
    }

    /// An integer value from a count, saturating rather than wrapping.
    ///
    /// A `usize` past `i64::MAX` is not a page number or a byte size of anything this program
    /// can open, so the saturation is unreachable — and it is here rather than a cast because
    /// `arithmetic_side_effects` is denied in this workspace.
    #[must_use]
    pub fn count(from: usize) -> Self {
        Self::Integer(i64::try_from(from).unwrap_or(i64::MAX))
    }

    /// An integer value from a byte count, saturating the same way.
    #[must_use]
    pub fn bytes(from: u64) -> Self {
        Self::Integer(i64::try_from(from).unwrap_or(i64::MAX))
    }

    /// A count where there is one.
    #[must_use]
    pub fn optional_count(from: Option<usize>) -> Self {
        from.map_or(Self::Null, Self::count)
    }

    /// The value as JSON, indented for a person and parseable by a program.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    /// One value at one indentation level.
    fn write(&self, out: &mut String, depth: usize) {
        let pad = |out: &mut String, depth: usize| {
            for _ in 0..depth {
                out.push_str("  ");
            }
        };
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Integer(value) => {
                let _ = write!(out, "{value}");
            }
            Self::Number(value) if !value.is_finite() => out.push_str("null"),
            Self::Number(value) => {
                let _ = write!(out, "{value:?}");
            }
            Self::Text(value) => escape(value, out),
            Self::Array(items) if items.is_empty() => out.push_str("[]"),
            Self::Array(items) => {
                out.push_str("[\n");
                for (at, item) in items.iter().enumerate() {
                    pad(out, depth.saturating_add(1));
                    item.write(out, depth.saturating_add(1));
                    if at.saturating_add(1) < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                pad(out, depth);
                out.push(']');
            }
            Self::Object(fields) if fields.is_empty() => out.push_str("{}"),
            Self::Object(fields) => {
                out.push_str("{\n");
                for (at, (key, value)) in fields.iter().enumerate() {
                    pad(out, depth.saturating_add(1));
                    escape(key, out);
                    out.push_str(": ");
                    value.write(out, depth.saturating_add(1));
                    if at.saturating_add(1) < fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                pad(out, depth);
                out.push('}');
            }
        }
    }
}

/// One string, quoted and escaped as RFC 8259 section 7 requires.
fn escape(text: &str, out: &mut String) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            control if control < ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(control));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::Value;

    /// The escapes a document's text can force, and the shape of the whole.
    #[test]
    fn control_characters_survive_the_writer() {
        let value = Value::Object(vec![
            ("text".to_owned(), Value::text("a\u{c}b\"c\\d\u{1}")),
            ("page".to_owned(), Value::count(339)),
            ("stated".to_owned(), Value::optional(None::<String>)),
            ("items".to_owned(), Value::Array(vec![Value::Bool(true)])),
        ]);
        assert_eq!(
            value.render(),
            "{\n  \"text\": \"a\\fb\\\"c\\\\d\\u0001\",\n  \"page\": 339,\n  \"stated\": null,\n  \
             \"items\": [\n    true\n  ]\n}\n"
        );
    }
}
