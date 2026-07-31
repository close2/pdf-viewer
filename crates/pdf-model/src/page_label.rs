//! ISO 32000-2 §12.4.2's page labels.
//!
//! A page has an *index*, which is its position and always runs from zero, and it may also
//! have a *label*, which is what a reader sees beside it. The clause keeps them apart in as
//! many words:
//!
//! > Page labels and page indices need not coincide: the indices shall be fixed, running
//! > consecutively through the document starting from 0 for the first page, but the labels
//! > may be specified in any way that is appropriate for the particular document.
//!
//! The document divides itself into *labelling ranges*, each starting at a page index and
//! running to the start of the next, and each stating a numbering style, a prefix and a first
//! value. The ranges live in the catalog's `/PageLabels`, which is §7.9.7's number tree.
//!
//! # Why this is here and not in `viewer-ui`
//!
//! A label is a string computed from the document and from nothing else — no window, no font,
//! no device. It is the one part of clause 12's navigation half with no user-interface
//! question in it, which is why `CLAUDE.md` names it in scope beside outlines and
//! destinations, and why it can be finished here while the panel that would show it does not
//! exist.

use pdf_syntax::{Dictionary, Document, Object, tree};

/// The numbering styles Table 161's `/S` defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    /// `D`: decimal Arabic numerals.
    Decimal,
    /// `R` and `r`: Roman numerals, upper and lower case.
    Roman { upper: bool },
    /// `A` and `a`: letters, "A to Z for the first 26 pages, AA to ZZ for the next 26".
    Letters { upper: bool },
}

impl Style {
    /// Reads Table 161's `/S`, or `None` for a range that states no style.
    ///
    /// There is no default, and the clause is explicit about what that means: "if no S entry
    /// is present, page labels shall consist solely of a label prefix with no numeric
    /// portion" — so every page of such a range carries the *same* label, which its own NOTE
    /// spells out with `Contents`.
    fn read(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"D" => Self::Decimal,
            b"R" => Self::Roman { upper: true },
            b"r" => Self::Roman { upper: false },
            b"A" => Self::Letters { upper: true },
            b"a" => Self::Letters { upper: false },
            _ => return None,
        })
    }

    /// Renders one number in this style.
    fn render(self, value: u32) -> String {
        match self {
            Self::Decimal => value.to_string(),
            Self::Roman { upper } => roman(value, upper),
            Self::Letters { upper } => letters(value, upper),
        }
    }
}

/// Table 161's letters: `A` to `Z`, then `AA` to `ZZ`, then `AAA` and so on.
///
/// **Not base 26.** The clause says "A to Z for the first 26 pages, AA to ZZ for the next 26",
/// so the twenty-seventh page is `AA` rather than `AB`: the letter is `(n − 1) mod 26` and the
/// *repeat count* is `⌈n ÷ 26⌉`. Base 26 would give `AA` at 27 by coincidence and `AB` at 28,
/// where the clause gives `BB`.
fn letters(value: u32, upper: bool) -> String {
    if value == 0 {
        return String::new();
    }
    let index = value.saturating_sub(1) % 26;
    let repeats = value.saturating_sub(1).saturating_div(26).saturating_add(1);
    let base = if upper { b'A' } else { b'a' };
    let letter = char::from(base.saturating_add(u8::try_from(index).unwrap_or(0)));
    std::iter::repeat_n(letter, usize::try_from(repeats).unwrap_or(1)).collect()
}

/// Roman numerals, in the subtractive form every reader expects.
///
/// The clause names the style and states no algorithm, so this is the conventional one:
/// the standard's own example runs `i, ii, iii, iv`, which the additive form would spell
/// `iiii`. Zero has no Roman numeral and produces nothing, which is the same answer
/// [`letters`] gives it.
fn roman(value: u32, upper: bool) -> String {
    /// The subtractive pairs, largest first, which is what makes `iv` rather than `iiii`.
    const NUMERALS: [(u32, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];

    let mut left = value;
    let mut out = String::new();
    for (amount, numeral) in NUMERALS {
        while left >= amount {
            out.push_str(numeral);
            left = left.saturating_sub(amount);
        }
    }
    if upper { out.to_uppercase() } else { out }
}

/// One labelling range: where it starts and what it says.
#[derive(Debug, Clone)]
struct Range {
    /// The page index this range begins at, which is its key in the number tree.
    first: i64,
    style: Option<Style>,
    prefix: String,
    /// Table 161's `/St`, "the value of the numeric portion for the first page label in the
    /// range … which shall be greater than or equal to 1. Default value: 1."
    start: u32,
}

/// A document's page labels, read once.
///
/// Built eagerly from the number tree and then answered by binary search, because a document
/// has as many ranges as it has numbering styles — a handful — and never as many as it has
/// pages. That is the one place §7.9.7's efficiency argument does not apply, and the reason
/// `tree::number_pairs` exists beside `tree::lookup`.
#[derive(Debug, Clone, Default)]
pub struct PageLabels {
    ranges: Vec<Range>,
}

impl PageLabels {
    /// Reads the catalog's `/PageLabels`, which is absent from most documents.
    ///
    /// An absent entry is not an error and not a defect: a document that states no labels has
    /// none, and [`Self::label`] then answers `None` for every page rather than inventing the
    /// index as a label. Deciding what to show for a page with no label is a viewer's
    /// question, and this crate does not answer viewers' questions.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        let entry = document.get_key(&catalog, "PageLabels");
        let Some(root) = entry.as_dict() else {
            return Self::default();
        };
        let mut ranges: Vec<Range> = tree::number_pairs(root, &|object| document.resolve(object))
            .into_iter()
            .filter_map(|(first, value)| {
                let dict = value.as_dict()?;
                Some(Range {
                    first,
                    style: document
                        .get_key(dict, "S")
                        .as_name()
                        .and_then(|name| Style::read(name.as_bytes())),
                    prefix: prefix(document, dict),
                    // "shall be greater than or equal to 1. Default value: 1." A file writing
                    // zero or a negative value has said something the clause forbids, and the
                    // default is the only value it names.
                    start: document
                        .get_key(dict, "St")
                        .as_integer()
                        .and_then(|value| u32::try_from(value).ok())
                        .filter(|value| *value >= 1)
                        .unwrap_or(1),
                })
            })
            .collect();
        // The clause requires the keys to be ascending; a file that wrote them otherwise is
        // sorted here rather than refused, because the ranges' *order* is the only thing that
        // decides which one a page falls in and nothing else in the file states it.
        ranges.sort_by_key(|range| range.first);
        Self { ranges }
    }

    /// The label for a page index, or `None` where the document states none.
    ///
    /// §12.4.2:
    ///
    /// > The tree shall include a value for page index 0.
    ///
    /// A file that omits it leaves every page before its first range unlabelled, which is what
    /// this returns — the alternative would be extending the first range backwards, which no
    /// sentence supports.
    #[must_use]
    pub fn label(&self, index: usize) -> Option<String> {
        let index = i64::try_from(index).ok()?;
        // The last range whose first page is at or before this one.
        let at = self.ranges.partition_point(|range| range.first <= index);
        let range = self.ranges.get(at.checked_sub(1)?)?;
        let mut label = range.prefix.clone();
        if let Some(style) = range.style {
            // "Pages within a range shall be numbered sequentially in ascending order",
            // starting at `/St`.
            let offset = u32::try_from(index.saturating_sub(range.first)).unwrap_or(0);
            label.push_str(&style.render(range.start.saturating_add(offset)));
        }
        Some(label)
    }

    /// Whether the document states any labels at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

/// Table 161's `/P`, "[t]he label prefix for page labels in this range".
///
/// A *text string*, so §7.9.2.2's three encodings apply and `pdf_syntax::text_string` is what
/// decodes them — the same function §12.7.4.3's field values use, which is why a prefix in
/// UTF-16BE works without anything here knowing that it is.
fn prefix(document: &Document, dict: &Dictionary) -> String {
    match document.get_key(dict, "P") {
        Object::String(bytes) => pdf_syntax::text_string(&bytes),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Style, letters, roman};

    /// The clause's own example: "a document with pages labelled i, ii, iii, iv, 1, 2, 3, A-8,
    /// A-9". The first four are what a *subtractive* Roman numeral gives; the additive form
    /// would spell the fourth `iiii`, so the example is what chooses the algorithm.
    #[test]
    fn roman_numerals_are_subtractive() {
        let lower = |n| roman(n, false);
        assert_eq!(
            [lower(1), lower(2), lower(3), lower(4)],
            ["i", "ii", "iii", "iv"]
        );
        assert_eq!(roman(1990, true), "MCMXC");
        assert_eq!(roman(0, true), "", "zero has no Roman numeral");
    }

    /// "A to Z for the first 26 pages, AA to ZZ for the next 26, and so on" — which is **not**
    /// base 26, and this is the assertion that separates them.
    #[test]
    fn letters_repeat_rather_than_carrying() {
        assert_eq!(letters(1, true), "A");
        assert_eq!(letters(26, true), "Z");
        assert_eq!(letters(27, true), "AA", "the next 26 begin at AA");
        // Base 26 would give `AB` here. The clause gives `BB`.
        assert_eq!(letters(28, true), "BB");
        assert_eq!(letters(52, true), "ZZ");
        assert_eq!(letters(53, false), "aaa");
    }

    /// Table 161 lists five style names and no default, so anything else is no style at all —
    /// and a range with no style labels every one of its pages identically, which the clause's
    /// own NOTE about a `Contents` prefix describes.
    #[test]
    fn only_the_five_named_styles_are_styles() {
        for name in [b"D".as_slice(), b"R", b"r", b"A", b"a"] {
            assert!(Style::read(name).is_some(), "{name:?}");
        }
        for name in [b"d".as_slice(), b"I", b"1", b"", b"Decimal"] {
            assert!(Style::read(name).is_none(), "{name:?}");
        }
    }
}
