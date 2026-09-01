//! The page-range grammar every verb shares — RFC 0002 section 4.2, stated once.
//!
//! One grammar rather than a tool's, because prior art is genuinely split: qpdf filters parity by
//! *position* and spells the last page `z`, pdftk and cpdf filter by *page number*, mutool spells
//! the last page `N`. The RFC chose, and this module is that choice:
//!
//! | form | meaning |
//! |---|---|
//! | `5` | page 5, counted from 1 |
//! | `3-7` | inclusive range; `7-3` is the same pages reversed |
//! | `1-end` | `end` is the last page |
//! | `r1`, `r3-r1` | counted from the end: `r1` is the last page |
//! | `a,b,c` | concatenation, order significant, duplicates allowed |
//! | `x3-4` | exclusion from the running selection |
//! | `3-7:odd`, `:even` | filter the range by **page-number** parity |
//! | `@iv`, `@{A-3}` | the page whose §12.4.2 label is `iv` / `A-3`; braces when the label contains `,`, `-` or `:` |
//!
//! Two departures from prior art are deliberate and are the RFC's, not this module's: parity is
//! the page *number's*, because duplex printing is the use people have and position parity is
//! recoverable by composing selections while the reverse is not; and a label is addressed in the
//! grammar rather than by a mode switch, because no surveyed tool can say "the page labelled iv"
//! and this tree already reads §12.4.2.
//!
//! # Parsing and resolving are two steps, on purpose
//!
//! A [`Selection`] is pure data — RFC 0002 section 5's plan "has no paths inside", and it has no
//! document inside either. [`Selection::resolve`] is where the document enters: a page count and
//! a label lookup, both supplied by the caller, so that the same selection can be resolved against
//! any source and a plan can be built before a file is open. An unmatched label, a page past the
//! end and `r0` are all resolution errors naming what was asked for, never a silent clamp.
//!
//! A document whose label sequence produces duplicates resolves `@x` to the **first** match —
//! stated in `--help` as a choice (RFC 0002 section 4.2).

use std::fmt;
use std::str::FromStr;

/// A page selection as the user wrote it: the items of the grammar, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The comma-separated items, each an inclusion or an exclusion.
    items: Vec<Item>,
}

/// One comma-separated item.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Item {
    /// `x` prefix: remove these pages from the running selection instead of appending them.
    exclude: bool,
    /// Where the item starts.
    from: Point,
    /// Where it ends, for a range; `None` for a single page.
    to: Option<Point>,
    /// `:odd` / `:even`, a filter on the page number.
    parity: Option<Parity>,
}

/// One end of a range.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Point {
    /// A page number counted from 1.
    Page(usize),
    /// A page counted from the end: `r1` is the last page.
    FromEnd(usize),
    /// The last page, spelled `end`.
    End,
    /// The page whose §12.4.2 label this is.
    Label(String),
}

/// A parity filter on page numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parity {
    /// Odd page numbers, counted from 1.
    Odd,
    /// Even page numbers.
    Even,
}

/// A selection that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Nothing at all, or an empty item between two commas.
    #[error("empty page selection")]
    Empty,
    /// A term that is none of the grammar's four forms.
    #[error("not a page, `rN`, `end` or `@label`: {0:?}")]
    Term(String),
    /// A page number of zero, which no page has.
    #[error("page numbers count from 1, so {0:?} names no page")]
    Zero(String),
    /// `:something` that is neither `odd` nor `even`.
    #[error("not a parity filter (`:odd` or `:even`): {0:?}")]
    Parity(String),
    /// `@{` with no closing brace.
    #[error("a braced label is not closed: {0:?}")]
    UnclosedLabel(String),
    /// An `@` with nothing after it.
    #[error("`@` names no label")]
    EmptyLabel,
}

/// A selection that reads but does not resolve against this document.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveError {
    /// A page number past the end.
    #[error("page {page} is past the end: this document has {count}")]
    PastTheEnd {
        /// The page asked for, counted from 1.
        page: usize,
        /// How many pages the document has.
        count: usize,
    },
    /// `rN` with `N` larger than the document.
    #[error("r{from_end} counts back past the first page: this document has {count}")]
    BeforeTheStart {
        /// The distance from the end that was asked for.
        from_end: usize,
        /// How many pages the document has.
        count: usize,
    },
    /// A label no page carries.
    #[error("no page is labelled {label:?}")]
    NoSuchLabel {
        /// The label asked for.
        label: String,
    },
    /// A document with no pages, against which nothing resolves.
    #[error("the document has no pages")]
    NoPages,
}

impl Selection {
    /// Every page, first to last — what a verb takes when `--pages` is not given.
    #[must_use]
    pub fn all() -> Self {
        Self {
            items: vec![Item {
                exclude: false,
                from: Point::Page(1),
                to: Some(Point::End),
                parity: None,
            }],
        }
    }

    /// Resolves the selection to zero-based page indices, in selection order.
    ///
    /// `count` is how many pages the document has; `label` answers §12.4.2's label of a
    /// zero-based index where the document states one. Duplicates are kept — `1,1` is two
    /// pages, as the grammar says — and an exclusion removes every occurrence of its pages from
    /// whatever has been selected so far.
    ///
    /// # Errors
    ///
    /// A [`ResolveError`] naming the page, distance or label the document does not have. A
    /// selection that resolves to no pages is not an error: it is an empty answer.
    pub fn resolve(
        &self,
        count: usize,
        label: impl Fn(usize) -> Option<String>,
    ) -> Result<Vec<usize>, ResolveError> {
        if count == 0 {
            return Err(ResolveError::NoPages);
        }
        let mut selected: Vec<usize> = Vec::new();
        for item in &self.items {
            let pages = item.pages(count, &label)?;
            if item.exclude {
                selected.retain(|page| !pages.contains(page));
            } else {
                selected.extend(pages);
            }
        }
        Ok(selected)
    }
}

impl Item {
    /// The zero-based pages this item names, in its order, after its parity filter.
    fn pages(
        &self,
        count: usize,
        label: &impl Fn(usize) -> Option<String>,
    ) -> Result<Vec<usize>, ResolveError> {
        let from = self.from.resolve(count, label)?;
        let to = match &self.to {
            Some(point) => point.resolve(count, label)?,
            None => from,
        };
        // Inclusive in both directions: `7-3` is `3-7` reversed, which the grammar states.
        let range: Box<dyn Iterator<Item = usize>> = if from <= to {
            Box::new(from..=to)
        } else {
            Box::new((to..=from).rev())
        };
        Ok(range
            .filter(|&index| {
                // Parity is the page *number's* — one-based — never the index's.
                let number = index.saturating_add(1);
                match self.parity {
                    None => true,
                    Some(Parity::Odd) => number % 2 == 1,
                    Some(Parity::Even) => number % 2 == 0,
                }
            })
            .collect())
    }
}

impl Point {
    /// The zero-based index this point names in a document of `count` pages.
    fn resolve(
        &self,
        count: usize,
        label: &impl Fn(usize) -> Option<String>,
    ) -> Result<usize, ResolveError> {
        match self {
            Self::Page(page) => {
                if *page > count {
                    return Err(ResolveError::PastTheEnd { page: *page, count });
                }
                // `page >= 1` is the parser's guarantee.
                Ok(page.saturating_sub(1))
            }
            Self::FromEnd(from_end) => {
                count
                    .checked_sub(*from_end)
                    .ok_or(ResolveError::BeforeTheStart {
                        from_end: *from_end,
                        count,
                    })
            }
            Self::End => Ok(count.saturating_sub(1)),
            Self::Label(wanted) => (0..count)
                .find(|&index| label(index).as_deref() == Some(wanted.as_str()))
                .ok_or_else(|| ResolveError::NoSuchLabel {
                    label: wanted.clone(),
                }),
        }
    }
}

impl FromStr for Selection {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut items = Vec::new();
        for piece in split_items(text)? {
            items.push(parse_item(&piece)?);
        }
        if items.is_empty() {
            return Err(ParseError::Empty);
        }
        Ok(Self { items })
    }
}

/// Splits on commas that are not inside a braced label.
fn split_items(text: &str) -> Result<Vec<String>, ParseError> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            ',' => {
                if current.trim().is_empty() {
                    return Err(ParseError::Empty);
                }
                items.push(std::mem::take(&mut current));
            }
            '{' => {
                current.push('{');
                let mut closed = false;
                for inner in chars.by_ref() {
                    current.push(inner);
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err(ParseError::UnclosedLabel(text.to_owned()));
                }
            }
            other => current.push(other),
        }
    }
    if current.trim().is_empty() {
        return Err(ParseError::Empty);
    }
    items.push(current);
    Ok(items)
}

/// One item: `[x]term[-term][:odd|:even]`.
fn parse_item(piece: &str) -> Result<Item, ParseError> {
    let piece = piece.trim();
    let (exclude, rest) = match piece.strip_prefix('x') {
        // `x` followed by a digit, `r`, `e` or `@` is the exclusion prefix. There is no page
        // spelling that starts with `x`, so the prefix is unambiguous.
        Some(rest) if !rest.is_empty() => (true, rest),
        _ => (false, piece),
    };
    let (body, parity) = match split_outside_braces(rest, ':') {
        Some((body, filter)) => {
            let parity = match filter {
                "odd" => Parity::Odd,
                "even" => Parity::Even,
                other => return Err(ParseError::Parity(other.to_owned())),
            };
            (body, Some(parity))
        }
        None => (rest, None),
    };
    let (from, to) = match split_outside_braces(body, '-') {
        Some((from, to)) => (parse_point(from)?, Some(parse_point(to)?)),
        None => (parse_point(body)?, None),
    };
    Ok(Item {
        exclude,
        from,
        to,
        parity,
    })
}

/// Splits `text` at the first `separator` that is outside `{…}`, or `None` if there is none.
fn split_outside_braces(text: &str, separator: char) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (at, character) in text.char_indices() {
        match character {
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.saturating_sub(1),
            found if found == separator && depth == 0 => {
                let after = at.saturating_add(separator.len_utf8());
                return Some((&text[..at], &text[after..]));
            }
            _ => {}
        }
    }
    None
}

/// One term: digits, `rN`, `end`, `@label` or `@{label}`.
fn parse_point(term: &str) -> Result<Point, ParseError> {
    let term = term.trim();
    if term == "end" {
        return Ok(Point::End);
    }
    if let Some(label) = term.strip_prefix('@') {
        if label.is_empty() {
            return Err(ParseError::EmptyLabel);
        }
        let label = match label.strip_prefix('{') {
            Some(braced) => braced
                .strip_suffix('}')
                .ok_or_else(|| ParseError::UnclosedLabel(term.to_owned()))?,
            None => label,
        };
        if label.is_empty() {
            return Err(ParseError::EmptyLabel);
        }
        return Ok(Point::Label(label.to_owned()));
    }
    if let Some(digits) = term.strip_prefix('r') {
        let from_end = parse_number(digits, term)?;
        return Ok(Point::FromEnd(from_end));
    }
    parse_number(term, term).map(Point::Page)
}

/// A page number or a distance from the end: decimal digits, at least 1.
fn parse_number(digits: &str, whole: &str) -> Result<usize, ParseError> {
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseError::Term(whole.to_owned()));
    }
    let number: usize = digits
        .parse()
        .map_err(|_overflow| ParseError::Term(whole.to_owned()))?;
    if number == 0 {
        return Err(ParseError::Zero(whole.to_owned()));
    }
    Ok(number)
}

impl fmt::Display for Selection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (at, item) in self.items.iter().enumerate() {
            if at > 0 {
                formatter.write_str(",")?;
            }
            if item.exclude {
                formatter.write_str("x")?;
            }
            write!(formatter, "{}", item.from)?;
            if let Some(to) = &item.to {
                write!(formatter, "-{to}")?;
            }
            match item.parity {
                Some(Parity::Odd) => formatter.write_str(":odd")?,
                Some(Parity::Even) => formatter.write_str(":even")?,
                None => {}
            }
        }
        Ok(())
    }
}

impl fmt::Display for Point {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Page(page) => write!(formatter, "{page}"),
            Self::FromEnd(from_end) => write!(formatter, "r{from_end}"),
            Self::End => formatter.write_str("end"),
            Self::Label(label) => {
                if label.contains([',', '-', ':', '{', '}']) {
                    write!(formatter, "@{{{label}}}")
                } else {
                    write!(formatter, "@{label}")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ParseError, ResolveError, Selection};

    /// Ten pages whose labels are `i`–`iv` then `1`–`6`, the front-matter shape §12.4.2's own
    /// example describes.
    fn ten(index: usize) -> Option<String> {
        const ROMAN: [&str; 4] = ["i", "ii", "iii", "iv"];
        ROMAN
            .get(index)
            .map(|label| (*label).to_owned())
            .or_else(|| index.checked_sub(3).map(|n| n.to_string()))
    }

    fn resolve(text: &str) -> Result<Vec<usize>, ResolveError> {
        text.parse::<Selection>()
            .unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
            .resolve(10, ten)
    }

    /// Every row of RFC 0002 section 4.2's table, with the answer the row states.
    #[test]
    fn every_form_in_the_table() {
        assert_eq!(resolve("5"), Ok(vec![4]));
        assert_eq!(resolve("3-7"), Ok(vec![2, 3, 4, 5, 6]));
        assert_eq!(resolve("7-3"), Ok(vec![6, 5, 4, 3, 2]));
        assert_eq!(resolve("8-end"), Ok(vec![7, 8, 9]));
        assert_eq!(resolve("r1"), Ok(vec![9]));
        assert_eq!(resolve("r3-r1"), Ok(vec![7, 8, 9]));
        assert_eq!(resolve("1,1,3"), Ok(vec![0, 0, 2]));
        assert_eq!(resolve("1-6,x3-4"), Ok(vec![0, 1, 4, 5]));
        assert_eq!(resolve("3-7:odd"), Ok(vec![2, 4, 6]));
        assert_eq!(resolve("1-end:even"), Ok(vec![1, 3, 5, 7, 9]));
        assert_eq!(resolve("@iv"), Ok(vec![3]));
        assert_eq!(resolve("@{iv}"), Ok(vec![3]));
        assert_eq!(resolve("@ii-@2"), Ok(vec![1, 2, 3, 4, 5]));
    }

    /// Parity is the page number's, not the position's: the first page of `2-5:odd` is page 3,
    /// which is the departure from qpdf the RFC states.
    #[test]
    fn parity_is_by_page_number_not_position() {
        assert_eq!(resolve("2-5:odd"), Ok(vec![2, 4]));
        assert_eq!(resolve("2-5:even"), Ok(vec![1, 3]));
    }

    /// An exclusion removes every occurrence, and applies to what came before it only.
    #[test]
    fn exclusion_removes_every_occurrence_so_far() {
        assert_eq!(resolve("2,2,3,x2,2"), Ok(vec![2, 1]));
    }

    /// The label whose text contains a grammar character is addressed in braces.
    #[test]
    fn a_braced_label_may_contain_the_separators() {
        let selection: Selection = "@{A-3},@{a,b},@{x:y}".parse().expect("parses");
        let labels = |index: usize| ["A-3", "a,b", "x:y"].get(index).map(|s| (*s).to_owned());
        assert_eq!(selection.resolve(3, labels), Ok(vec![0, 1, 2]));
        assert_eq!(selection.to_string(), "@{A-3},@{a,b},@{x:y}");
    }

    /// Resolution errors name what was asked for.
    #[test]
    fn resolution_errors_name_the_request() {
        assert_eq!(
            resolve("11"),
            Err(ResolveError::PastTheEnd {
                page: 11,
                count: 10
            })
        );
        assert_eq!(
            resolve("r11"),
            Err(ResolveError::BeforeTheStart {
                from_end: 11,
                count: 10
            })
        );
        assert_eq!(
            resolve("@v"),
            Err(ResolveError::NoSuchLabel {
                label: "v".to_owned()
            })
        );
        assert_eq!(
            Selection::all().resolve(0, |_| None),
            Err(ResolveError::NoPages)
        );
    }

    /// A duplicate label resolves to its first page, as `--help` states.
    #[test]
    fn a_duplicated_label_resolves_to_its_first_page() {
        let selection: Selection = "@1".parse().expect("parses");
        assert_eq!(selection.resolve(4, |_| Some("1".to_owned())), Ok(vec![0]));
    }

    /// Parse errors, each the grammar's own.
    #[test]
    fn parse_errors() {
        assert_eq!("".parse::<Selection>(), Err(ParseError::Empty));
        assert_eq!("1,,2".parse::<Selection>(), Err(ParseError::Empty));
        assert_eq!(
            "0".parse::<Selection>(),
            Err(ParseError::Zero("0".to_owned()))
        );
        assert_eq!(
            "r0".parse::<Selection>(),
            Err(ParseError::Zero("r0".to_owned()))
        );
        assert_eq!(
            "abc".parse::<Selection>(),
            Err(ParseError::Term("abc".to_owned()))
        );
        assert_eq!(
            "1-3:prime".parse::<Selection>(),
            Err(ParseError::Parity("prime".to_owned()))
        );
        assert_eq!("@".parse::<Selection>(), Err(ParseError::EmptyLabel));
        assert_eq!(
            "@{iv".parse::<Selection>(),
            Err(ParseError::UnclosedLabel("@{iv".to_owned()))
        );
    }

    /// `Display` writes what `FromStr` reads, so a plan can be shown as the user would type it.
    #[test]
    fn display_round_trips() {
        for text in [
            "5", "3-7", "1-end", "r3-r1", "1,1,3", "1-6,x3-4", "3-7:odd", "@iv-@ix",
        ] {
            let selection: Selection = text.parse().expect("parses");
            assert_eq!(selection.to_string(), text);
        }
        assert_eq!(Selection::all().to_string(), "1-end");
    }
}
