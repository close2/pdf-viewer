//! The standard's own clause index, read from the Markdown conversion in `doc/md/`.
//!
//! Every citation in this tree names a clause number, and until this module existed nothing
//! could say whether that number was one the standard has. Two of the thirty-six distinct
//! numbers cited turned out not to be — `§8.9.6.5` and `§11.4.5.6`, both copied from comment
//! to comment and into two reports — which is the whole argument for building the index
//! rather than trusting the citations.
//!
//! # What a clause's *span* is
//!
//! A quotation is checked against the text of the clause it cites, and a clause's text
//! includes its subclauses: a sentence cited as `§8.9.6` may live in `§8.9.6.2`. So a
//! heading's span runs from the heading to the next numeric heading that is **not** a
//! descendant of it.
//!
//! Two properties of the conversion shape that rule, and both were measured rather than
//! assumed:
//!
//! - **Not every `##` heading is a clause.** The conversion promotes `EXAMPLE 2`, `NOTE 9`
//!   and similar run-in headings to the same level, so a span delimited by *any* heading
//!   would end at the first example inside the clause. Only numeric headings delimit.
//! - **The annexes and the corrigenda are not in clause order.** `## A.1 General` precedes
//!   `## Annex A`, and the EC3 corrigendum at the end of the file carries renumbered
//!   subclauses of `§14.8` twenty thousand lines after `§14.13`. So `Annex` also ends a
//!   span, or the last subclause of clause 14 would swallow every annex.
//!
//! A number may occur more than once — `14.8.4.7.3` does, once in the body and once in the
//! corrigendum that renumbers it — so the index keeps every occurrence and a quotation may
//! match in any of them.

use std::fmt;
use std::ops::Range;
use std::path::Path;
use std::str::FromStr;

use crate::quote;

/// A clause number, as the standard writes it: `8.9.6.2`.
///
/// Ordered by component rather than by text, so `§8.9` precedes `§8.10`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClauseNumber(Vec<u16>);

impl ClauseNumber {
    /// The components, most significant first: `8.9.6.2` gives `[8, 9, 6, 2]`.
    #[must_use]
    pub fn components(&self) -> &[u16] {
        &self.0
    }

    /// The top-level clause this number belongs to: `8.9.6.2` gives `8`.
    #[must_use]
    pub fn clause(&self) -> u16 {
        // A `ClauseNumber` is only ever built through `FromStr`, which rejects an empty
        // component list, so there is always a first component.
        self.0.first().copied().unwrap_or_default()
    }

    /// How many components the number has: `8.9.6.2` is at depth 4.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    /// Whether `other` is a strictly deeper number beginning with this one.
    ///
    /// `8.9.6` is an ancestor of `8.9.6.2`; nothing is an ancestor of itself, and `8.9.6` is
    /// not an ancestor of `8.9.61`, because the comparison is by component.
    #[must_use]
    pub fn is_ancestor_of(&self, other: &Self) -> bool {
        other.0.len() > self.0.len() && other.0.starts_with(&self.0)
    }
}

impl fmt::Display for ClauseNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, component) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(".")?;
            }
            write!(f, "{component}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ClauseNumber {
    /// The number itself, because a failure naming `ClauseNumber([8, 9, 6, 5])` is harder to
    /// read than one naming `§8.9.6.5`, and this type appears mostly in test failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "§{self}")
    }
}

/// Why a string is not a clause number.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClauseNumberError {
    /// The string held no components at all.
    #[error("a clause number cannot be empty")]
    Empty,
    /// A component was not a decimal number, or did not fit in a `u16`.
    #[error("`{text}` is not a clause number: `{component}` is not a decimal number")]
    NotNumeric {
        /// The whole string that was being parsed.
        text: String,
        /// The component that failed.
        component: String,
    },
}

impl FromStr for ClauseNumber {
    type Err = ClauseNumberError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() {
            return Err(ClauseNumberError::Empty);
        }
        let mut components = Vec::new();
        for component in text.split('.') {
            let parsed = component
                .parse::<u16>()
                .map_err(|_| ClauseNumberError::NotNumeric {
                    text: text.to_owned(),
                    component: component.to_owned(),
                })?;
            components.push(parsed);
        }
        Ok(Self(components))
    }
}

/// One numbered heading of the standard, with the text that belongs to it.
#[derive(Debug, Clone)]
pub struct Heading {
    /// The clause number the heading carries.
    pub number: ClauseNumber,
    /// The heading's title, with the number removed: `Stencil masking`.
    pub title: String,
    /// The 1-based line of `doc/md/` the heading sits on, for error messages.
    pub line: usize,
    /// The byte range of this clause's text, subclauses included.
    pub span: Range<usize>,
}

/// Every numbered heading of the standard, in the order the document states them.
#[derive(Debug)]
pub struct ClauseIndex {
    text: String,
    headings: Vec<Heading>,
}

/// Why the clause index could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ClauseIndexError {
    /// The Markdown conversion could not be read.
    #[error("cannot read the standard at {path}: {source}")]
    Unreadable {
        /// The path that was tried.
        path: String,
        /// The underlying failure.
        source: std::io::Error,
    },
    /// The file held no numbered headings, so it is not the standard.
    #[error("{path} holds no numbered headings; it is not the standard's conversion")]
    NotTheStandard {
        /// The path that was read.
        path: String,
    },
}

impl ClauseIndex {
    /// Reads the standard's Markdown conversion and indexes its numbered headings.
    ///
    /// # Errors
    ///
    /// If the file cannot be read, or holds no numbered heading at all — which means the
    /// path is wrong or the conversion has changed shape, and either way every check built
    /// on it would silently pass.
    pub fn read(path: &Path) -> Result<Self, ClauseIndexError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| ClauseIndexError::Unreadable {
                path: path.display().to_string(),
                source,
            })?;
        let index = Self::parse(text);
        if index.headings.is_empty() {
            return Err(ClauseIndexError::NotTheStandard {
                path: path.display().to_string(),
            });
        }
        Ok(index)
    }

    /// Indexes the numbered headings of an already-loaded conversion.
    #[must_use]
    pub fn parse(text: String) -> Self {
        struct Raw {
            number: Option<ClauseNumber>,
            title: String,
            line: usize,
            start: usize,
            ends_a_span: bool,
        }

        let mut raw: Vec<Raw> = Vec::new();
        let mut offset = 0usize;
        for (index, line) in text.split_inclusive('\n').enumerate() {
            let start = offset;
            offset = offset.saturating_add(line.len());
            let Some(heading) = line.trim_start().strip_prefix("## ") else {
                continue;
            };
            let heading = heading.trim();
            let (first, rest) = match heading.split_once(char::is_whitespace) {
                Some((first, rest)) => (first, rest.trim()),
                None => (heading, ""),
            };
            let number = first.trim_end_matches('.').parse::<ClauseNumber>().ok();
            // An annex heading ends the preceding clause's span. See the module comment:
            // the annexes and the corrigendum are not in clause-number order, so nothing
            // numeric delimits the last subclause of clause 14.
            let ends_a_span = number.is_some() || heading.starts_with("Annex ");
            raw.push(Raw {
                number,
                title: rest.to_owned(),
                line: index.saturating_add(1),
                start,
                ends_a_span,
            });
        }

        let mut headings = Vec::new();
        for (position, entry) in raw.iter().enumerate() {
            let Some(number) = entry.number.clone() else {
                continue;
            };
            let end = raw
                .iter()
                .skip(position.saturating_add(1))
                .find(|later| {
                    later.ends_a_span
                        && later
                            .number
                            .as_ref()
                            .is_none_or(|later| !number.is_ancestor_of(later))
                })
                .map_or(text.len(), |later| later.start);
            headings.push(Heading {
                number,
                title: entry.title.clone(),
                line: entry.line,
                span: entry.start..end,
            });
        }

        Self { text, headings }
    }

    /// Every numbered heading, in document order.
    #[must_use]
    pub fn headings(&self) -> &[Heading] {
        &self.headings
    }

    /// Whether the standard has this clause.
    #[must_use]
    pub fn contains(&self, number: &ClauseNumber) -> bool {
        self.headings
            .iter()
            .any(|heading| &heading.number == number)
    }

    /// The title of a clause, taken from its first occurrence.
    #[must_use]
    pub fn title(&self, number: &ClauseNumber) -> Option<&str> {
        self.headings
            .iter()
            .find(|heading| &heading.number == number)
            .map(|heading| heading.title.as_str())
    }

    /// The distinct subclause numbers of one top-level clause, in document order.
    ///
    /// The clause heading itself is excluded: `§8` is a container, and the ledger's rows are
    /// the requirements underneath it.
    #[must_use]
    pub fn subclauses_of(&self, clause: u16) -> Vec<ClauseNumber> {
        let mut seen: Vec<ClauseNumber> = Vec::new();
        for heading in &self.headings {
            if heading.number.clause() != clause || heading.number.depth() < 2 {
                continue;
            }
            if !seen.contains(&heading.number) {
                seen.push(heading.number.clone());
            }
        }
        seen
    }

    /// Whether `quotation` occurs verbatim in the text of `number`, subclauses included.
    ///
    /// Compared through [`quote::normalise`], and satisfied if *any* occurrence of the
    /// clause number holds it — `14.8.4.7.3` is stated twice, and the corrigendum's text is
    /// as normative as the body's.
    #[must_use]
    pub fn holds_quotation(&self, number: &ClauseNumber, quotation: &str) -> bool {
        self.headings
            .iter()
            .filter(|heading| &heading.number == number)
            .any(|heading| {
                let text = self.text.get(heading.span.clone()).unwrap_or_default();
                quote::occurs_in(&quote::normalise(text), quotation)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(text: &str) -> ClauseNumber {
        text.parse().unwrap()
    }

    #[test]
    fn a_clause_number_orders_by_component_not_by_text() {
        assert!(number("8.9") < number("8.10"));
        assert!(number("8.9.6.2") > number("8.9.6"));
        assert_eq!(number("8.9.6.2").to_string(), "8.9.6.2");
    }

    #[test]
    fn ancestry_is_by_component_so_8_9_6_is_not_an_ancestor_of_8_9_61() {
        assert!(number("8.9.6").is_ancestor_of(&number("8.9.6.2")));
        assert!(!number("8.9.6").is_ancestor_of(&number("8.9.61")));
        assert!(!number("8.9.6").is_ancestor_of(&number("8.9.6")));
    }

    #[test]
    fn a_clause_number_must_be_numeric() {
        assert_eq!("".parse::<ClauseNumber>(), Err(ClauseNumberError::Empty));
        assert!("A.1".parse::<ClauseNumber>().is_err());
        assert!("8.9.".parse::<ClauseNumber>().is_err());
    }

    /// A clause's span must reach into its subclauses, or a sentence cited at the family's
    /// number could never be found — which is how §9.6.5 and §8.9.6 are cited here.
    #[test]
    fn a_span_covers_the_subclauses_but_stops_at_the_next_sibling() {
        let index = ClauseIndex::parse(
            "## 8.9.6 Masked images\nintro\n\n## 8.9.6.2 Stencil masking\nstencil text\n\n\
             ## 8.9.7 Something else\nother text\n"
                .to_owned(),
        );
        assert!(index.holds_quotation(&number("8.9.6"), "stencil text"));
        assert!(!index.holds_quotation(&number("8.9.6"), "other text"));
        assert!(!index.holds_quotation(&number("8.9.6.2"), "intro"));
    }

    /// The conversion promotes run-in headings such as `EXAMPLE 2` to `##`, so a span
    /// delimited by any heading would stop at the first example inside the clause.
    #[test]
    fn a_non_numeric_heading_does_not_end_a_span() {
        let index = ClauseIndex::parse(
            "## 7.4.9 RunLengthDecode\nbefore\n\n## EXAMPLE 2\nafter the example\n".to_owned(),
        );
        assert!(index.holds_quotation(&number("7.4.9"), "after the example"));
    }

    /// The last subclause of clause 14 is followed by the annexes, which carry no number
    /// this parser recognises, and then by a corrigendum that renumbers parts of §14.8.
    #[test]
    fn an_annex_heading_ends_a_span() {
        let index = ClauseIndex::parse(
            "## 14.13.10 Associated file examples\nbody\n\n## Annex A\nannex text\n".to_owned(),
        );
        assert!(index.holds_quotation(&number("14.13.10"), "body"));
        assert!(!index.holds_quotation(&number("14.13.10"), "annex text"));
    }

    /// `14.8.4.7.3` is stated twice: once in the body and once in the corrigendum that
    /// renumbers it. A quotation from either is a quotation from the standard.
    #[test]
    fn a_number_stated_twice_is_searched_in_both_places() {
        let index = ClauseIndex::parse(
            "## 14.8.4.7.3 Ruby and warichu elements\nruby text\n\n## Annex A\nx\n\n\
             ## 14.8.4.7.3 Link elements\nlink text\n"
                .to_owned(),
        );
        assert!(index.holds_quotation(&number("14.8.4.7.3"), "ruby text"));
        assert!(index.holds_quotation(&number("14.8.4.7.3"), "link text"));
    }

    #[test]
    fn subclauses_exclude_the_clause_heading_itself_and_deduplicate() {
        let index = ClauseIndex::parse(
            "## 8 Graphics\nx\n\n## 8.1 General\ny\n\n## 8.1 General\nz\n".to_owned(),
        );
        assert_eq!(index.subclauses_of(8), vec![number("8.1")]);
    }
}
