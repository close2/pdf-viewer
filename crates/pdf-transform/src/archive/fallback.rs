//! "This target, or that one where this one refuses": the record a conversion keeps when its
//! first target refused and the second was written (ADR 2498).
//!
//! The verb takes the two as one request (`--to 2b --or-else-to 4`), converts to the first, and
//! only where that conversion wrote no file converts to the second. This module is the half that
//! stays in the report: which target was asked for first, and the refusals that sent the document
//! on, so the person reading the report of a part 4 file learns that part 2 was wanted and what
//! stopped it. The file itself says which part it is the way every output does, in the
//! identification schema's `pdfaid:part` (ISO 19005-2 section 6.6.4; ISO 19005-4 section 6.7.3).
//! **Set by the face that ran both conversions**, like [`super::Conversion::configuration`]: a
//! plan names one target, so the library's `apply` never knows a second attempt happened.

use pdf_archive::Target;

use crate::Declined;
use crate::json::Value;

/// The first target of a request that fell back, and why it refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fallback {
    /// The target the request named first, whose conversion wrote no file.
    pub requested: Target,
    /// What that conversion refused over, as its own report stated it.
    pub refused: Vec<Declined>,
}

impl Fallback {
    /// The sentence the rendered report opens with, then one line per refusal.
    ///
    /// `written` is the target of the conversion this record sits in, which may itself have
    /// refused: the report says so below.
    #[must_use]
    pub fn render(&self, written: Target) -> String {
        use std::fmt::Write as _;
        let mut out = format!(
            "{requested} was asked for first and refused ({count} reason(s)); the same request \
             named {written} next, and this report is of that conversion\n",
            requested = self.requested,
            count = self.refused.len(),
        );
        for declined in &self.refused {
            let _ = writeln!(out, "  {} refused: {}", self.requested, declined.subject);
        }
        out
    }

    /// The record as RFC 0002 section 4.5's JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::Object(vec![
            (
                "requested".to_owned(),
                Value::text(self.requested.to_string()),
            ),
            (
                "refused".to_owned(),
                Value::Array(
                    self.refused
                        .iter()
                        .map(|declined| {
                            Value::Object(vec![
                                ("subject".to_owned(), Value::text(declined.subject.clone())),
                                ("detail".to_owned(), Value::text(declined.detail.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}
