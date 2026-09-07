//! What a requirement found, and where.
//!
//! A verdict that says only *failed* sends its reader back into the document to discover
//! where — so every failure carries a witness: the object, the page, or the name that made it
//! one. That is the same discipline `tools/conformance` applies to this project's own claims,
//! and the reason `doc/rfc/0006` argues the validator is the larger part of the value.

use pdf_syntax::ObjectId;

/// Where in a document a requirement failed.
///
/// `None` in every field is a legitimate witness for a rule about the file as a whole — the
/// header's bytes, the trailer's keys — and says so rather than inventing a location.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Where {
    /// The object the failure is in, where it is in one.
    pub object: Option<ObjectId>,
    /// The zero-based page index, where the failure belongs to a page.
    pub page: Option<usize>,
    /// The name of the offending entry, filter, font or colourant.
    pub name: Option<String>,
}

impl Where {
    /// The whole file, for a rule about its shape rather than its contents.
    #[must_use]
    pub const fn file() -> Self {
        Self {
            object: None,
            page: None,
            name: None,
        }
    }

    /// One object.
    #[must_use]
    pub const fn object(id: ObjectId) -> Self {
        Self {
            object: Some(id),
            page: None,
            name: None,
        }
    }

    /// One page, by zero-based index.
    #[must_use]
    pub const fn page(index: usize) -> Self {
        Self {
            object: None,
            page: Some(index),
            name: None,
        }
    }

    /// The same place, with the name of what was wrong there.
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// One place a requirement was not met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Where it was found.
    pub place: Where,
    /// What was wrong, in one sentence a person can act on.
    pub what: String,
}

/// What one requirement's predicate found, and how much of it was kept.
///
/// # The bound, and why it is reported rather than applied in silence
///
/// A hostile or merely enormous document can fail one requirement on every object it has, and
/// a report holding a million findings is not a report. So a predicate's findings are capped —
/// and the cap is *announced*, because "this requirement failed 12 times" and "this requirement
/// failed at least 12 times and we stopped counting" are different facts about the document,
/// and only the second one tells the reader the list in front of them is a prefix.
#[derive(Debug, Clone, Default)]
pub struct Findings {
    kept: Vec<Finding>,
    seen: usize,
}

/// How many places one requirement reports before it starts counting instead.
const MAX_KEPT: usize = 32;

impl Findings {
    /// Records a place the requirement was not met.
    pub fn record(&mut self, place: Where, what: impl Into<String>) {
        self.seen = self.seen.saturating_add(1);
        if self.kept.len() < MAX_KEPT {
            self.kept.push(Finding {
                place,
                what: what.into(),
            });
        }
    }

    /// The places kept, in the order they were found.
    #[must_use]
    pub fn kept(&self) -> &[Finding] {
        &self.kept
    }

    /// How many places the requirement was not met, including those past the bound.
    #[must_use]
    pub const fn seen(&self) -> usize {
        self.seen
    }

    /// Whether the list is a prefix of what was found rather than the whole of it.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.seen > MAX_KEPT
    }

    /// Whether the requirement was met.
    #[must_use]
    pub const fn met(&self) -> bool {
        self.seen == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{Findings, Where};

    #[test]
    fn a_requirement_nothing_reported_is_met() {
        let findings = Findings::default();
        assert!(findings.met());
        assert!(!findings.truncated());
        assert_eq!(findings.seen(), 0);
    }

    #[test]
    fn past_the_bound_the_count_keeps_going_and_says_so() {
        let mut findings = Findings::default();
        for index in 0..100 {
            findings.record(Where::page(index), "wrong");
        }
        assert!(!findings.met());
        assert_eq!(
            findings.seen(),
            100,
            "the count is of what the document did"
        );
        assert_eq!(
            findings.kept().len(),
            32,
            "the list is of what a report can hold"
        );
        assert!(
            findings.truncated(),
            "and the difference between them is announced"
        );
    }
}
