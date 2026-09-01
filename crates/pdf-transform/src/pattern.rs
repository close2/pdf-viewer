//! Output-name patterns — RFC 0002 section 4.3 — and the sanitisation its section 8 names as a hazard.
//!
//! Multi-file output names are printf-style patterns, following pdftk `burst`, `pdfseparate` and
//! mutool: `-o 'page-%d.png'`. What each escape means:
//!
//! | escape | expands to |
//! |---|---|
//! | `%d` | the ordinal, counted from 1, zero-padded to the width of the count |
//! | `%03d` | the ordinal at an explicit width |
//! | `%p` | the first source page number of the piece, counted from 1 |
//! | `%l` | that page's §12.4.2 label, sanitised; the page number where the document states none |
//! | `%t` | a title — an outline item's for `split --at-bookmarks`, an embedded file's name for `attachments` — sanitised |
//! | `%%` | a literal `%` |
//!
//! **A pattern without `%d` when more than one file would be written is a usage error, not a
//! silent overwrite** — pdfseparate's rule, and [`Pattern::distinguishes`] is what a verb asks
//! before it writes anything.
//!
//! # Why `%l` and `%t` are sanitised, and how
//!
//! A page label and an outline title are the *document's* text, so a pattern-expanded name is
//! attacker-influenced: a title of `../../.bashrc` must not escape the output directory (RFC
//! 0002 section 8). [`sanitise`] replaces every path separator, every control byte and every byte that
//! is not valid in a file name on the platforms this program runs on with `_`, and a text that
//! comes out empty becomes `_` so it still names something. The rule is stated in `--help`, and
//! the report names every name that was changed by it.

use std::fmt::{self, Write as _};
use std::str::FromStr;

/// A parsed output-name pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    /// The pieces, literal text and escapes alternating in whatever order the user wrote.
    pieces: Vec<Piece>,
}

/// One piece of a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    /// Text copied as it is.
    Literal(String),
    /// `%d` or `%0Nd`.
    Ordinal {
        /// An explicit width, or `None` for the width of the count.
        width: Option<usize>,
    },
    /// `%p`.
    FirstPage,
    /// `%l`.
    Label,
    /// `%t`.
    Title,
}

/// A pattern that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    /// `%` followed by something that is not an escape.
    #[error("not an output-name escape (`%d`, `%03d`, `%p`, `%l`, `%t`, `%%`): {0:?}")]
    Escape(String),
    /// An empty pattern names nothing.
    #[error("the output name is empty")]
    Empty,
}

/// What a pattern's escapes are filled from, for one output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fill<'a> {
    /// The output's ordinal, counted from 1.
    pub ordinal: usize,
    /// How many outputs there are, which decides `%d`'s default width.
    pub count: usize,
    /// The first source page of this output, counted from 1, where the output comes from a page.
    pub page: Option<usize>,
    /// That page's §12.4.2 label, where the document states one.
    pub label: Option<&'a str>,
    /// The title this output has, where the verb gives it one.
    pub title: Option<&'a str>,
}

/// An expanded name, and whether sanitisation changed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expanded {
    /// The name to open.
    pub name: String,
    /// `true` where a `%l` or `%t` had a byte replaced, so the report can say so.
    pub sanitised: bool,
}

impl Pattern {
    /// Whether the pattern can name `count` outputs distinctly: one output needs nothing, more
    /// need `%d`.
    ///
    /// `%p` and `%l` are deliberately not enough: a selection may name one page twice, and two
    /// pages may carry one label, so only the ordinal is distinct by construction.
    #[must_use]
    pub fn distinguishes(&self, count: usize) -> bool {
        count <= 1
            || self
                .pieces
                .iter()
                .any(|piece| matches!(piece, Piece::Ordinal { .. }))
    }

    /// Whether the pattern uses `%p` or `%l`, which only a page-derived output can fill.
    #[must_use]
    pub fn names_a_page(&self) -> bool {
        self.pieces
            .iter()
            .any(|piece| matches!(piece, Piece::FirstPage | Piece::Label))
    }

    /// Whether the pattern uses `%t`.
    #[must_use]
    pub fn names_a_title(&self) -> bool {
        self.pieces
            .iter()
            .any(|piece| matches!(piece, Piece::Title))
    }

    /// The name for one output.
    ///
    /// An escape the fill cannot answer — `%p` for an output that is not a page's, `%t` where
    /// the verb has no title — expands to its ordinal, so a name is always produced; a verb
    /// checks [`Self::names_a_page`] and [`Self::names_a_title`] up front and refuses the
    /// pattern where that matters.
    #[must_use]
    pub fn expand(&self, fill: &Fill<'_>) -> Expanded {
        let mut name = String::new();
        let mut sanitised = false;
        for piece in &self.pieces {
            match piece {
                Piece::Literal(text) => name.push_str(text),
                Piece::Ordinal { width } => {
                    let width = width.unwrap_or_else(|| digits(fill.count));
                    // Writing into a `String` cannot fail.
                    let _ = write!(name, "{:0width$}", fill.ordinal);
                }
                Piece::FirstPage => {
                    name.push_str(&fill.page.unwrap_or(fill.ordinal).to_string());
                }
                Piece::Label => match fill.label {
                    Some(label) => {
                        let clean = sanitise(label);
                        sanitised |= clean != label;
                        name.push_str(&clean);
                    }
                    None => name.push_str(&fill.page.unwrap_or(fill.ordinal).to_string()),
                },
                Piece::Title => match fill.title {
                    Some(title) => {
                        let clean = sanitise(title);
                        sanitised |= clean != title;
                        name.push_str(&clean);
                    }
                    None => name.push_str(&fill.ordinal.to_string()),
                },
            }
        }
        Expanded { name, sanitised }
    }
}

/// How many decimal digits `count` has, at least 1.
fn digits(count: usize) -> usize {
    let mut digits: usize = 1;
    let mut rest = count / 10;
    while rest > 0 {
        digits = digits.saturating_add(1);
        rest /= 10;
    }
    digits
}

/// A document's text made safe as one file-name component.
///
/// Replaced with `_`: `/` and `\` (both, whatever the platform, because an output written on one
/// may be read on another), every control character including U+007F, and the characters
/// Windows forbids in a name (`<>:"|?*`) so a name this program writes on Linux is one it could
/// have written anywhere. A component that comes out empty is `_`. Nothing else is touched:
/// spaces, Unicode and a leading `.` are the document's business.
#[must_use]
pub fn sanitise(text: &str) -> String {
    let clean: String = text
        .chars()
        .map(|character| match character {
            '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            control if control.is_control() => '_',
            other => other,
        })
        .collect();
    if clean.is_empty() {
        "_".to_owned()
    } else {
        clean
    }
}

impl FromStr for Pattern {
    type Err = PatternError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() {
            return Err(PatternError::Empty);
        }
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut chars = text.chars().peekable();
        while let Some(character) = chars.next() {
            if character != '%' {
                literal.push(character);
                continue;
            }
            // An escape. Collect an optional `0N` width, then the letter.
            let mut width_digits = String::new();
            while let Some(digit) = chars.peek().copied().filter(char::is_ascii_digit) {
                width_digits.push(digit);
                chars.next();
            }
            let letter = chars.next();
            let piece = match (letter, width_digits.as_str()) {
                (Some('%'), "") => {
                    literal.push('%');
                    continue;
                }
                (Some('d'), "") => Piece::Ordinal { width: None },
                (Some('d'), digits) => Piece::Ordinal {
                    width: Some(
                        digits
                            .parse()
                            .map_err(|_overflow| PatternError::Escape(format!("%{digits}d")))?,
                    ),
                },
                (Some('p'), "") => Piece::FirstPage,
                (Some('l'), "") => Piece::Label,
                (Some('t'), "") => Piece::Title,
                (found, digits) => {
                    let mut shown = format!("%{digits}");
                    if let Some(found) = found {
                        shown.push(found);
                    }
                    return Err(PatternError::Escape(shown));
                }
            };
            if !literal.is_empty() {
                pieces.push(Piece::Literal(std::mem::take(&mut literal)));
            }
            pieces.push(piece);
        }
        if !literal.is_empty() {
            pieces.push(Piece::Literal(literal));
        }
        Ok(Self { pieces })
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for piece in &self.pieces {
            match piece {
                Piece::Literal(text) => formatter.write_str(&text.replace('%', "%%"))?,
                Piece::Ordinal { width: None } => formatter.write_str("%d")?,
                Piece::Ordinal { width: Some(width) } => write!(formatter, "%0{width}d")?,
                Piece::FirstPage => formatter.write_str("%p")?,
                Piece::Label => formatter.write_str("%l")?,
                Piece::Title => formatter.write_str("%t")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Fill, Pattern, PatternError, sanitise};

    fn fill(ordinal: usize, count: usize, label: Option<&str>) -> Fill<'_> {
        Fill {
            ordinal,
            count,
            page: Some(ordinal.saturating_add(6)),
            label,
            title: None,
        }
    }

    /// `%d` pads to the width of the count, and `%03d` to what it says.
    #[test]
    fn the_ordinal_pads_to_the_count() {
        let pattern: Pattern = "page-%d.png".parse().expect("parses");
        assert_eq!(pattern.expand(&fill(3, 9, None)).name, "page-3.png");
        assert_eq!(pattern.expand(&fill(3, 10, None)).name, "page-03.png");
        assert_eq!(pattern.expand(&fill(3, 1234, None)).name, "page-0003.png");
        let wide: Pattern = "p%05d".parse().expect("parses");
        assert_eq!(wide.expand(&fill(3, 9, None)).name, "p00003");
    }

    /// `%p` is the source page, `%l` its label, `%%` a percent.
    #[test]
    fn page_label_and_percent() {
        let pattern: Pattern = "%p-%l-100%%.png".parse().expect("parses");
        assert_eq!(
            pattern.expand(&fill(1, 1, Some("iv"))).name,
            "7-iv-100%.png"
        );
        assert_eq!(pattern.expand(&fill(1, 1, None)).name, "7-7-100%.png");
    }

    /// The hazard RFC 0002 section 8 names, by name.
    #[test]
    fn a_label_cannot_escape_the_directory() {
        let pattern: Pattern = "out/%l.png".parse().expect("parses");
        let expanded = pattern.expand(&fill(1, 1, Some("../../.bashrc")));
        assert_eq!(expanded.name, "out/.._.._.bashrc.png");
        assert!(expanded.sanitised);
        assert_eq!(sanitise("a\u{0}b\tc"), "a_b_c");
        assert_eq!(sanitise(""), "_");
        assert_eq!(sanitise("C:\\x|y"), "C__x_y");
        assert_eq!(sanitise("plain name.txt"), "plain name.txt");
    }

    /// pdfseparate's rule: more than one output needs `%d`.
    #[test]
    fn more_than_one_output_needs_the_ordinal() {
        let without: Pattern = "page-%p.png".parse().expect("parses");
        assert!(without.distinguishes(1));
        assert!(!without.distinguishes(2));
        let with: Pattern = "page-%d.png".parse().expect("parses");
        assert!(with.distinguishes(2));
    }

    /// Bad escapes are named.
    #[test]
    fn bad_escapes() {
        assert_eq!(
            "a%q".parse::<Pattern>(),
            Err(PatternError::Escape("%q".to_owned()))
        );
        assert_eq!(
            "a%".parse::<Pattern>(),
            Err(PatternError::Escape("%".to_owned()))
        );
        assert_eq!(
            "a%03p".parse::<Pattern>(),
            Err(PatternError::Escape("%03p".to_owned()))
        );
        assert_eq!("".parse::<Pattern>(), Err(PatternError::Empty));
    }

    /// Display writes what parses.
    #[test]
    fn display_round_trips() {
        for text in ["page-%d.png", "%03d-%p-%l-%t", "100%%"] {
            let pattern: Pattern = text.parse().expect("parses");
            assert_eq!(pattern.to_string(), text);
        }
    }
}
