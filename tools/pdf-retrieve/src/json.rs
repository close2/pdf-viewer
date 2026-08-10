//! Just enough JSON to write a report, and none to read one.
//!
//! Why this is here rather than a dependency: the output of this tool is a *fixed* shape it
//! writes and never parses, so what a serialisation crate would buy is derive macros for six
//! kinds of value. `doc/third-party-data.md`'s rule is that a dependency is taken for something
//! hard or something dangerous to get wrong; escaping a string is the second of those and is
//! twenty lines, and RFC 8259 section 7 states exactly which characters need it.
//!
//! The one rule that is easy to get wrong and is therefore stated here: **every code point below
//! U+0020 must be escaped**, and a PDF's readback contains them — a form feed between two pages
//! of a content stream is `\u{c}` and appears in real documents.

use std::fmt::Write as _;

/// One JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`, which this tool writes for an entry the document does not state.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A signed integer. Every number this tool prints is a count, an index or a page.
    Integer(i64),
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
        from.map_or(Self::Null, |value| Self::Text(value.into()))
    }

    /// An integer value from a count, saturating rather than wrapping.
    ///
    /// A `usize` past `i64::MAX` is not a page number or a byte offset of anything this program
    /// can open, so the saturation is unreachable — and it is here rather than a cast because
    /// `arithmetic_side_effects` is denied in this workspace and a silent wrap in a report is
    /// exactly the kind of quiet wrongness that lint exists for.
    #[must_use]
    pub fn count(from: usize) -> Self {
        Self::Integer(i64::try_from(from).unwrap_or(i64::MAX))
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

    /// The three escapes a PDF's readback actually produces, and the shape of the whole.
    ///
    /// A form feed is the one worth naming: it is a legal character in a content stream's text
    /// and JSON forbids it raw, so a writer that escaped only quotes and backslashes would
    /// produce a report no parser accepts — on a real document rather than on a hostile one.
    #[test]
    fn a_readbacks_control_characters_survive_the_writer() {
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
