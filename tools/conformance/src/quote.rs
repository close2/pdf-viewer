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
//!   doc comment is typed with the ASCII forms.
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

/// Reduces text to the words a quotation is compared by.
///
/// Applied to both sides, so it can only ever make the comparison *coarser* — it cannot
/// make a paraphrase pass unless the paraphrase differs in nothing but layout and markup.
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
                // Markdown emphasis, inline code, and the conversion's backslash escapes.
                '*' | '`' | '_' | '\\' | '#' => {}
                '\u{2018}' | '\u{2019}' => out.push('\''),
                '\u{201c}' | '\u{201d}' => out.push('"'),
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
}
