//! Comparing a quotation in a doc comment against the standard's own words.
//!
//! `CLAUDE.md` principle 5 says quotation marks mean verbatim, and until this module existed
//! nothing checked it: of five quotations sampled by hand, three were paraphrases wearing
//! quotation marks. A paraphrase is often the clearer thing to write — what it must not do
//! is claim to be the standard's sentence, because a reader checking the code against the
//! clause then checks it against a summary someone made of the clause.
//!
//! # What "verbatim" is allowed to ignore
//!
//! The comparison is against a *Markdown conversion* of a PDF, read by a Rust doc comment,
//! so three differences are noise rather than signal and are normalised away:
//!
//! - **Layout.** Line breaks, indentation and runs of spaces carry nothing; the conversion
//!   reflows and a doc comment wraps at 96 columns. Whitespace is collapsed to one space.
//! - **Markup.** The conversion writes emphasis as `*` and escapes `_`; a doc comment writes
//!   `` `post` `` so that rustdoc renders a table name as code. Both are stripped, from both
//!   sides, so the comparison is of words.
//! - **Typography.** The conversion carries curly quotes and non-breaking spaces where a
//!   doc comment is typed with the ASCII forms — and a **quotation mark of any shape is
//!   dropped outright**, for the reason [`normalise`] gives with its witness.
//!
//! The conversion also inlines page images as base64 `![Image]` lines, which are megabytes
//! of noise no quotation can match; they are dropped.
//!
//! # Elision
//!
//! A quotation may drop the middle of a sentence with `…`, which is honest and often
//! necessary — the standard's sentences are long. The fragments either side must then occur
//! **in order**, so an elision cannot join two clauses' worth of text into a sentence the
//! standard does not contain.
//!
//! # Where a quotation *begins*
//!
//! [`quoted_spans`] is the other half of the same rule and the one nothing used to have: what
//! [`normalise`] compares, this finds. `CLAUDE.md` makes a pair of quotation marks a claim to
//! be verbatim wherever it is written, so a `"` … `"` **or** a `'` … `'` in a ledger note, a
//! doc comment or one of this project's own documents is a quotation whether or not any gate
//! reads it.

/// Reduces text to the words a quotation is compared by.
///
/// Applied to both sides, so it can only ever make the comparison *coarser* — it cannot
/// make a paraphrase pass unless the paraphrase differs in nothing but layout and markup.
///
/// # A quotation mark is markup, and the conversion proves it
///
/// Every shape of quotation mark — `'`, `"` and the four curly forms — is dropped rather than
/// folded onto one of them, which puts it in the same category as the `*` and the `` ` ``
/// above. The witness is §14.8.6, where the standard sets one glyph and `doc/md/` writes it
/// two ways on two pages: §14.8.6.1's namespace name comes out as `"http://iso.org/pdf2/ssn"`
/// and §14.8.6.3's as `' http://www.w3.org/1998/Math/MathML '`, from `“…”` in both places —
/// a different mark **and** a space inserted inside it. `pdftotext -layout` over the PDF in
/// `doc/` is what settles that, and a comparison that kept either the mark or the space would
/// call a verbatim quotation of §14.8.6.3 absent from the standard.
///
/// What it costs is a quotation that differs from the standard in nothing but its punctuation
/// marks and its apostrophes, which is a price this comparison already pays for emphasis.
///
/// It does **not** repair the conversion's other spacing habit — `14.8.6.3 , ' Other
/// namespaces '` has a space before the comma as well — because that one needs the whitespace
/// taken out altogether, which is [`crate::prose::folded`]'s coarsening and not a gate's.
#[must_use]
pub fn normalise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        // A page image, inlined as base64 by the conversion. Megabytes, and never words.
        if line.trim_start().starts_with("![Image]") {
            continue;
        }
        for character in line.chars() {
            match character {
                // Markdown emphasis, inline code, the conversion's backslash escapes — and
                // every quotation mark, straight or curly, for the reason above.
                '*' | '`' | '_' | '\\' | '#' | '\'' | '"' | '\u{2018}' | '\u{2019}'
                | '\u{201c}' | '\u{201d}' => {}
                character if character.is_whitespace() => out.push(' '),
                character => out.push(character),
            }
        }
        out.push(' ');
    }

    let mut collapsed = String::with_capacity(out.len());
    let mut in_space = true;
    for character in out.chars() {
        if character == ' ' {
            if !in_space {
                collapsed.push(' ');
            }
            in_space = true;
        } else {
            collapsed.push(character);
            in_space = false;
        }
    }
    collapsed.trim().to_owned()
}

/// Whether `quotation` occurs in already-[`normalise`]d `haystack`.
///
/// The quotation is normalised here, and split on the ellipsis so that its fragments are
/// required in order rather than as one string.
#[must_use]
pub fn occurs_in(haystack: &str, quotation: &str) -> bool {
    let quotation = normalise(quotation);
    let mut rest = haystack;
    for fragment in quotation
        .split(['\u{2026}'])
        .flat_map(|part| part.split("..."))
    {
        let fragment = fragment.trim();
        if fragment.is_empty() {
            continue;
        }
        let Some(position) = rest.find(fragment) else {
            return false;
        };
        let after = position.saturating_add(fragment.len());
        rest = rest.get(after..).unwrap_or_default();
    }
    true
}

/// The shortest span between two marks that is treated as a quotation at all.
///
/// A pair of quotation marks round three words is usually a term being named — "soft mask",
/// "the `post` table" — rather than a sentence being quoted, and the standard says four-word
/// phrases like "shall be the same" on many pages. Four is where a span starts carrying enough
/// of a sentence to be checkable.
pub const MIN_WORDS: usize = 4;

/// Which pair of marks delimits a quotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// `"` … `"`, straight or curly.
    Double,
    /// `'` … `'`, straight or curly — the population no sweep could see until this module
    /// collected it.
    Single,
}

/// Every quoted span of a piece of prose, in the order it makes them.
///
/// # The single-quoted half, and why it took until now
///
/// `CLAUDE.md` binds *quotation marks*, not one shape of them, and this project writes both:
/// `doc/conformance/ledger.toml` quotes the standard in single quotes wherever the note
/// already sits inside a TOML string that would have to escape a `"`. Three sweeps read the
/// double-quoted spans and none of them could see the others — §12.7.5.2.2's stale quotation
/// was in single quotes and was found through the source rather than by any instrument
/// (ADR 0254, `doc/todo/48`'s third owed item).
///
/// The reason it was left is real and is what the rule below is for: **an apostrophe is the
/// same character as a closing single quote**, so a scanner that pairs every `'` makes an
/// opening mark of every possessive. So a `'` opens a span only where nothing or a space or a
/// bracket precedes it, and closes one only where nothing or a space or ordinary punctuation
/// follows it. `Table 89's` and `don't` fail the first test; `elements'` and `processors'`
/// fail it too, because a plural possessive's mark also has a letter before it. A curly `‘`
/// is unambiguous and needs neither test.
///
/// The noise shape this rule cannot reason its way out of is the standard's own vocabulary:
/// §9.4.3 names two text-showing operators `'` and `"`, so a note listing them opens a span on
/// an operator. [`closing_single`] is what bounds the damage that does.
///
/// A mark that never closes is dropped rather than run to the end of the text: nothing closed
/// it, so there is no span. Both curly shapes are the same marks — these documents are typed
/// with both — and a span shorter than [`MIN_WORDS`] is not collected.
#[must_use]
pub fn quoted_spans(text: &str) -> Vec<(Mark, String)> {
    let characters: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut at = 0usize;
    while let Some(&character) = characters.get(at) {
        let opening = at.saturating_add(1);
        if is_double(character) {
            let Some(close) = (opening..characters.len())
                .find(|index| characters.get(*index).copied().is_some_and(is_double))
            else {
                break;
            };
            collect(&mut found, Mark::Double, &characters, opening, close);
            at = close.saturating_add(1);
            continue;
        }
        let before = at.checked_sub(1).and_then(|back| characters.get(back));
        if opens_single(character, before.copied())
            && let Some(close) = closing_single(&characters, opening)
        {
            collect(&mut found, Mark::Single, &characters, opening, close);
            at = close.saturating_add(1);
            continue;
        }
        at = opening;
    }
    found
}

/// Where the single-quoted span opened at `from` closes, if it does.
///
/// **A double quotation mark ends the search rather than being passed over**, and that is what
/// keeps a doubtful opener cheap: a `'` this rule reads as opening a quotation — §9.4.3's
/// operator, most often — would otherwise run to the next apostrophe with a space after it and
/// swallow every double-quoted span in between, taking real quotations out of the sweep to
/// report one that is not. What the bound costs is a single-quoted quotation that carries one
/// of the standard's own `(see 9.8, "Font descriptors")` cross-references, and it costs it
/// cheaply: the inner span is then collected as a quotation in its own right, so nothing goes
/// unread either way.
fn closing_single(characters: &[char], from: usize) -> Option<usize> {
    for index in from..characters.len() {
        let character = characters.get(index).copied()?;
        if is_double(character) {
            return None;
        }
        if is_single(character) && closes_single(characters.get(index.saturating_add(1)).copied()) {
            return Some(index);
        }
    }
    None
}

/// Takes the span between two marks, where it is long enough to be a quotation.
fn collect(
    found: &mut Vec<(Mark, String)>,
    mark: Mark,
    characters: &[char],
    from: usize,
    to: usize,
) {
    let span: String = characters
        .get(from..to)
        .unwrap_or_default()
        .iter()
        .collect();
    if span.split_whitespace().count() >= MIN_WORDS {
        found.push((mark, span));
    }
}

/// A double quotation mark of either shape.
fn is_double(character: char) -> bool {
    matches!(character, '"' | '\u{201c}' | '\u{201d}')
}

/// A single quotation mark of either shape — which is also the apostrophe.
fn is_single(character: char) -> bool {
    matches!(character, '\'' | '\u{2018}' | '\u{2019}')
}

/// Whether a single mark opens a quotation, given the character before it.
///
/// A backtick and an asterisk are deliberately **not** openers: these documents write
/// `` `Tree::role`'s `` and `**the standard**'s` all the time, and admitting them would make
/// an opening mark of the commonest possessive in the tree.
fn opens_single(character: char, before: Option<char>) -> bool {
    if character == '\u{2018}' {
        return true;
    }
    character == '\''
        && before.is_none_or(|before| {
            before.is_whitespace()
                || matches!(
                    before,
                    '(' | '[' | '{' | '"' | '\u{201c}' | '\u{2014}' | '\u{2013}'
                )
        })
}

/// Whether a single mark closes a quotation, given the character after it.
fn closes_single(after: Option<char>) -> bool {
    after.is_none_or(|after| {
        after.is_whitespace()
            || matches!(
                after,
                '.' | ','
                    | ';'
                    | ':'
                    | '!'
                    | '?'
                    | ')'
                    | ']'
                    | '}'
                    | '"'
                    | '\u{201d}'
                    | '\u{2014}'
                    | '\u{2013}'
            )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_markup_and_typography_are_noise() {
        let standard = normalise(
            "shall be looked up in the font\nprogram's *post* table (see 9.6.5.4, \u{201c}Encodings\u{201d})",
        );
        assert!(occurs_in(
            &standard,
            "looked up in the font program's `post` table"
        ));
    }

    #[test]
    fn a_paraphrase_does_not_pass() {
        let standard = normalise("shall represent the concentrations of these process colourants");
        assert!(!occurs_in(
            &standard,
            "concentrations of process colourants"
        ));
    }

    #[test]
    fn an_elision_requires_its_fragments_in_order() {
        let standard = normalise(
            "aligning the darkest colour of the source with the darkest colour of the display",
        );
        assert!(occurs_in(
            &standard,
            "aligning the darkest colour \u{2026} with the darkest colour"
        ));
        assert!(!occurs_in(
            &standard,
            "with the darkest colour \u{2026} aligning the darkest colour"
        ));
    }

    #[test]
    fn an_inlined_page_image_is_not_searchable_text() {
        let standard = normalise("before\n![Image](data:image/png;base64,AAAABBBB)\nafter");
        assert_eq!(standard, "before after");
    }

    /// §14.8.6.3's witness, as `doc/md/` writes it: the standard's `“…”` comes out as a single
    /// quote with a space inside it, and the space goes with the mark once the mark is gone.
    #[test]
    fn the_conversions_two_spellings_of_one_quotation_mark_are_the_same_mark() {
        let standard = normalise(
            "as would be identified by the NS entry in a namespace dictionary, shall have the \
             value:\n\n' http://www.w3.org/1998/Math/MathML '\n\nNOTE 1 MathML is the only",
        );
        assert!(occurs_in(
            &standard,
            "shall have the value: \u{201c}http://www.w3.org/1998/Math/MathML\u{201d}"
        ));
    }

    #[test]
    fn a_span_of_three_words_is_a_term_rather_than_a_quotation() {
        assert!(quoted_spans("the \"soft mask\" entry").is_empty());
        assert_eq!(
            quoted_spans("the \u{201c}four words go here\u{201d} entry"),
            vec![(Mark::Double, "four words go here".to_owned())]
        );
    }

    /// The whole reason the single-quoted half was left unread for eleven hundred spans.
    #[test]
    fn a_possessive_does_not_open_a_quotation() {
        assert!(quoted_spans("`Tree::role`'s walk is what the clause states").is_empty());
        assert!(quoted_spans("the standard's own words about what a reader owes").is_empty());
        assert!(quoted_spans("PDF processors' handling of a flatness tolerance").is_empty());
    }

    #[test]
    fn a_single_quoted_span_is_a_quotation() {
        assert_eq!(
            quoted_spans("the clause's own 'high-order overflow shall be ignored', read again"),
            vec![(
                Mark::Single,
                "high-order overflow shall be ignored".to_owned()
            )]
        );
    }

    /// A quotation that carries an apostrophe inside it: the inner mark has a letter after it,
    /// so it cannot be the closing one.
    #[test]
    fn an_apostrophe_inside_a_single_quoted_span_does_not_close_it() {
        assert_eq!(
            quoted_spans("Table 227 says 'the field's value shall be ignored' here"),
            vec![(
                Mark::Single,
                "the field's value shall be ignored".to_owned()
            )]
        );
    }

    #[test]
    fn a_mark_that_never_closes_opens_nothing() {
        assert!(quoted_spans("a note that opens \"and never closes it again").is_empty());
    }

    /// §9.4.3's operator names are quotation marks, and a note that lists them must not lose
    /// the quotations after it.
    #[test]
    fn a_doubtful_single_mark_does_not_swallow_the_quotation_after_it() {
        assert_eq!(
            quoted_spans(
                "Table 103 names the operator ' and the row goes on to say \
                 \"a sentence of the standard's own\" about it"
            ),
            vec![(Mark::Double, "a sentence of the standard's own".to_owned())]
        );
    }

    /// A single-quoted span inside a double-quoted one belongs to the quotation that encloses
    /// it, and is not reported twice.
    #[test]
    fn a_nested_span_is_the_outer_quotation() {
        assert_eq!(
            quoted_spans("a text annotation is \"a 'sticky note' attached to a point\" here"),
            vec![(
                Mark::Double,
                "a 'sticky note' attached to a point".to_owned()
            )]
        );
    }
}
