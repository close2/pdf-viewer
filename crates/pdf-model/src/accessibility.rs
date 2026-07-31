//! ISO 32000-2 §14.9's accessibility support: what a page *says*, as against what it shows.
//!
//! §14.9.1 states the purpose plainly — "many computer users with visual impairments use
//! screen readers to read documents aloud" — and then names four facilities. Three of them
//! substitute or annotate text and are read here; the fourth, §14.9.6's pronunciation hints,
//! the clause itself excuses ("A PDF processor is not required to process pronunciation
//! hints").
//!
//! # Three entries, three different rules, and the differences are the whole subject
//!
//! | entry | clause | what it is | word break |
//! |---|---|---|---|
//! | `/ActualText` | §14.9.4 | a *character* substitution | none between consecutive ones |
//! | `/Alt` | §14.9.3 | a description of an item that does not translate into text | one between consecutive ones |
//! | `/E` | §14.9.5 | an abbreviation's expansion | one on each side |
//!
//! §14.9.4's NOTE 2 draws the first distinction itself — the treatment of `/ActualText` as a
//! character replacement "is different from the treatment of Alt, which is treated as a whole
//! word or phrase substitution". That is why `/ActualText` belongs to *extraction* — it is
//! what a person copying the page should get — and is applied in `content.rs` directly to
//! [`crate::Interpretation::text`], while `/Alt` and `/E` belong to *vocalisation* and are
//! recorded here as spans over that same string. Copying a ligature should give `fi`; copying
//! a photograph should give nothing, and speaking it should give its description.
//!
//! # Where each entry may appear
//!
//! Each of the three may sit on a `Span` marked-content sequence's property list or on a
//! structure element (§14.9.3, §14.9.4, §14.9.5 each list both), and `/Lang` may additionally
//! sit on the document catalog. Both routes reach a content stream: a `BDC` operand carries
//! the property list directly, and a `/MCID` in one names the sequence's structure element
//! through §14.7.5.4's parent tree. `content.rs` asks the property list first, because it is
//! the more specific statement — attached to this sequence rather than to the element the
//! sequence belongs to.
//!
//! # What this module does not decide
//!
//! Nothing here reaches a pixel. §14.1's opening sentence covers the whole clause: "[t]he
//! features described in this clause do not affect the final appearance of a document."

use std::ops::Range;

/// What §14.9 says about one marked-content sequence, over the text that sequence produced.
///
/// Built by the interpreter for every sequence stating any of the three entries, and for no
/// other — a page whose producer tagged nothing carries an empty list and pays one `Option`
/// test per `BDC`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Described {
    /// Where the sequence's text sits in [`crate::Interpretation::text`], in bytes.
    ///
    /// Ranges nest exactly as the sequences do, because a `BDC` and its `EMC` bracket both.
    pub range: Range<usize>,
    /// §14.9.3's `/Alt`: "human-readable text that could, for example, be vocalised by a
    /// text-to-speech engine".
    pub alt: Option<String>,
    /// §14.9.5's `/E`: the expansion of an abbreviation or acronym.
    pub expansion: Option<String>,
    /// §14.9.2's `/Lang`, a BCP 47 language tag, in force for this sequence and what it
    /// encloses.
    ///
    /// Already resolved through §14.9.2.3's hierarchy as far as *this* sequence states it:
    /// the property list's own entry, or failing that the language its structure element
    /// inherits. The enclosing sequences and the document's default are applied by
    /// [`speech`], because they are properties of the surrounding text rather than of this
    /// sequence.
    pub language: Option<String>,
}

impl Described {
    /// The phrase that replaces this sequence's text when it is spoken, if any.
    ///
    /// **`/Alt` wins over `/E` where a file states both, and that is a choice rather than a
    /// reading.** §14.9.3 makes `/Alt` "a complete (or whole) word or phrase substitution for
    /// the current element" and §14.9.5 makes `/E` "a word or phrase substitution for the
    /// tagged text"; both replace, the clause states no precedence between them, and a
    /// sequence carrying both has said two things about one span. `/Alt` is preferred because
    /// it describes the *item* while `/E` expands the *text* it contains, so where an item is
    /// described the description subsumes what its text spells out.
    #[must_use]
    fn phrase(&self) -> Option<&str> {
        self.alt.as_deref().or(self.expansion.as_deref())
    }
}

/// One run of text as a text-to-speech engine would receive it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spoken {
    /// The text to speak.
    pub text: String,
    /// The language it is in — §14.9.2.2's identifier, or `None` where the document states
    /// none anywhere in the hierarchy.
    ///
    /// The clause admits one more answer this type does not distinguish: "the empty text
    /// string, to indicate that the language is unknown". An empty `/Lang` is read as no
    /// statement, which is what it means.
    pub language: Option<String>,
}

/// §14.9's vocalised form of a page: its text with the descriptions and expansions applied.
///
/// `text` is [`crate::Interpretation::text`], `described` the spans the interpreter recorded
/// over it, and `default` the document catalog's `/Lang`, which §14.9.2.3 makes "the default
/// natural language for all text in the document".
///
/// Runs are merged while the language does not change, so a page stating one language is one
/// run and an untagged page is one run with whatever the catalog said.
///
/// # The rules, and where each comes from
///
/// - A sequence with `/Alt` or `/E` **replaces** the text it encloses, nested sequences and
///   all: both entries are phrase substitutions for their whole span.
/// - A word break is inserted on each side of such a phrase. §14.9.5 states it for `/E` — it
///   "shall be treated as if a word break separates it from any surrounding text" — and
///   §14.9.3 states the consecutive case for `/Alt`: "[i]f each of two (or more) elements in
///   a sequence have an Alt entry in their dictionaries, they shall be treated as if a word
///   break is present between them." Inserting on both sides satisfies both, and inserts
///   nothing where the text already breaks.
/// - `/ActualText` appears nowhere here because it is already in `text`, and §14.9.4's rule
///   that consecutive ones carry no word break between them is what plain concatenation does.
/// - The innermost stated `/Lang` wins, which is §14.9.2.3's hierarchy: the catalog's default,
///   overridden by a structure element's, overridden by a nested element's or sequence's. The
///   clause's own EXAMPLE 3 is the case that looks like an exception and is not — structured
///   content inside a `Span` of another language takes the structure element's, because it is
///   the inner statement.
#[must_use]
pub fn speech(text: &str, described: &[Described], default: Option<&str>) -> Vec<Spoken> {
    // Outermost first, so that a sequence is met before what it encloses. Equal starts order
    // the longer span first for the same reason.
    let mut items: Vec<&Described> = described.iter().collect();
    items.sort_by(|left, right| {
        left.range
            .start
            .cmp(&right.range.start)
            .then(right.range.end.cmp(&left.range.end))
    });

    let mut runs = Vec::new();
    emit(text, &items, 0, text.len(), default, 0, &mut runs);
    runs
}

/// Deepest nesting of marked-content sequences whose entries are applied.
///
/// The interpreter puts no bound on how deeply a content stream may nest `BDC` … `EMC` — a
/// section costs an operator and `MAX_OPERATIONS` bounds those — so a file may state a million
/// nested sequences, each with a `/Lang`, and this walk descends one level per sequence.
/// Legitimate tagging is a handful of levels; a document past this bound has its remaining
/// nesting flattened into the language in force at the bound, which loses an override no real
/// file states and costs nothing that can be seen.
const MAX_NESTING: usize = 64;

/// Emits `text[from..to]` with the sequences in `items`, which all lie within it.
///
/// `items` is ordered outermost-first, so its first entry is a sequence at this level and
/// everything up to that entry's end is nested inside it.
fn emit(
    text: &str,
    items: &[&Described],
    from: usize,
    to: usize,
    language: Option<&str>,
    depth: usize,
    runs: &mut Vec<Spoken>,
) {
    let mut cursor = from;
    let mut index = 0usize;
    while let Some(item) = items.get(index) {
        let (start, end) = (item.range.start, item.range.end);
        // A range outside this span, or inverted by a malformed nesting, governs nothing here.
        if start < cursor || end > to || start > end {
            index = index.saturating_add(1);
            continue;
        }
        // Everything after this sequence and before its end is nested inside it.
        let mut past = index.saturating_add(1);
        while items.get(past).is_some_and(|inner| inner.range.start < end) {
            past = past.saturating_add(1);
        }

        push(text.get(cursor..start).unwrap_or_default(), language, runs);
        let inner = item.language.as_deref().or(language);
        if let Some(phrase) = item.phrase() {
            push_break(runs);
            push(phrase, inner, runs);
            push_break(runs);
        } else if depth < MAX_NESTING {
            emit(
                text,
                items.get(index.saturating_add(1)..past).unwrap_or_default(),
                start,
                end,
                inner,
                depth.saturating_add(1),
                runs,
            );
        } else {
            push(text.get(start..end).unwrap_or_default(), inner, runs);
        }
        cursor = end;
        index = past;
    }
    push(text.get(cursor..to).unwrap_or_default(), language, runs);
}

/// Appends text to the last run, or starts a new one when the language changes.
fn push(text: &str, language: Option<&str>, runs: &mut Vec<Spoken>) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = runs.last_mut()
        && last.language.as_deref() == language
    {
        last.text.push_str(text);
        return;
    }
    runs.push(Spoken {
        text: text.to_owned(),
        language: language.map(str::to_owned),
    });
}

/// Inserts the word break §14.9.3 and §14.9.5 ask for, where there is not one already.
///
/// A space rather than any other separator because the clause says "word break" and gives no
/// character for it; what matters to the consumer is that the phrase does not run into its
/// neighbour. Nothing is inserted at the start of the output, where there is no neighbour.
fn push_break(runs: &mut [Spoken]) {
    let Some(last) = runs.last_mut() else {
        return;
    };
    if last.text.ends_with(char::is_whitespace) {
        return;
    }
    last.text.push(' ');
}

#[cfg(test)]
mod tests {
    use super::{Described, Spoken, speech};

    /// Builds a span with only an `/Alt`.
    fn alt(range: std::ops::Range<usize>, text: &str) -> Described {
        Described {
            range,
            alt: Some(text.to_owned()),
            expansion: None,
            language: None,
        }
    }

    /// The clause's own §14.9.5 example: two abbreviations, each expanded in place.
    ///
    /// `BT /Span <</E (Doctor)>> BDC (Dr.) Tj EMC (Healwell works at 123 Industrial ) Tj
    /// /Span <</E (Drive)>> BDC (Dr.) Tj EMC ET` — so the drawn text is
    /// `Dr.Healwell works at 123 Industrial Dr.` and what is spoken expands both.
    #[test]
    fn an_expansion_replaces_the_abbreviation_it_tags() {
        let text = "Dr.Healwell works at 123 Industrial Dr.";
        let described = vec![
            Described {
                range: 0..3,
                alt: None,
                expansion: Some("Doctor".to_owned()),
                language: None,
            },
            Described {
                range: 36..39,
                alt: None,
                expansion: Some("Drive".to_owned()),
                language: None,
            },
        ];
        let spoken = speech(text, &described, None);
        assert_eq!(spoken.len(), 1, "one language throughout");
        assert_eq!(
            spoken.first().map(|run| run.text.as_str()),
            Some("Doctor Healwell works at 123 Industrial Drive "),
            "§14.9.5: the expansion is separated from surrounding text by a word break"
        );
    }

    /// §14.9.3: consecutive alternate descriptions are separated by a word break.
    ///
    /// The two spans are adjacent in the drawn text — two glyphs with no space between them —
    /// and the clause requires that they be "treated as if a word break is present between
    /// them", which no character in the page's own text supplies.
    #[test]
    fn consecutive_alternate_descriptions_are_separated() {
        let text = "AB";
        let described = vec![alt(0..1, "six-point star"), alt(1..2, "arrow")];
        let spoken = speech(text, &described, None);
        assert_eq!(
            spoken.first().map(|run| run.text.as_str()),
            Some("six-point star arrow ")
        );
    }

    /// A description replaces everything its sequence encloses, nested sequences included.
    ///
    /// §14.9.3 makes `/Alt` "a complete (or whole) word or phrase substitution for the current
    /// element", so an inner expansion inside a described figure is not also spoken.
    #[test]
    fn a_description_replaces_what_it_encloses() {
        let text = "xAy";
        let described = vec![
            alt(0..3, "a chart"),
            Described {
                range: 1..2,
                alt: None,
                expansion: Some("ampere".to_owned()),
                language: None,
            },
        ];
        let spoken = speech(text, &described, None);
        assert_eq!(
            spoken.first().map(|run| run.text.as_str()),
            Some("a chart ")
        );
    }

    /// §14.9.2.3's EXAMPLE 1: the catalog's language, overridden inside one sequence.
    #[test]
    fn a_sequence_overrides_the_documents_language() {
        let text = "See you later, or as Arnold would say, Hasta la vista.";
        let described = vec![Described {
            range: 39..54,
            alt: None,
            expansion: None,
            language: Some("es-MX".to_owned()),
        }];
        let spoken = speech(text, &described, Some("en-US"));
        assert_eq!(
            spoken,
            vec![
                Spoken {
                    text: "See you later, or as Arnold would say, ".to_owned(),
                    language: Some("en-US".to_owned()),
                },
                Spoken {
                    text: "Hasta la vista.".to_owned(),
                    language: Some("es-MX".to_owned()),
                },
            ]
        );
    }

    /// §14.9.2.3's EXAMPLE 3: structured content inside a `Span` takes the element's language.
    ///
    /// The outer sequence states Mexican Spanish and the inner one — whose language came from
    /// its structure element — states U.S. English, and the clause's rule that "the structure
    /// element's language specification shall take precedence" is the innermost statement
    /// winning rather than a special case.
    #[test]
    fn the_innermost_language_wins_and_the_outer_one_resumes() {
        let text = "Hasta la vista, as Arnold would say.!";
        let described = vec![
            Described {
                range: 0..37,
                alt: None,
                expansion: None,
                language: Some("es-MX".to_owned()),
            },
            Described {
                range: 16..36,
                alt: None,
                expansion: None,
                language: Some("en-US".to_owned()),
            },
        ];
        let spoken = speech(text, &described, None);
        assert_eq!(
            spoken
                .iter()
                .map(|run| (run.text.as_str(), run.language.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("Hasta la vista, ", Some("es-MX")),
                ("as Arnold would say.", Some("en-US")),
                ("!", Some("es-MX")),
            ]
        );
    }

    /// A page that tags nothing is one run in the document's language.
    #[test]
    fn an_untagged_page_is_one_run() {
        let spoken = speech("plain text", &[], Some("de"));
        assert_eq!(
            spoken,
            vec![Spoken {
                text: "plain text".to_owned(),
                language: Some("de".to_owned()),
            }]
        );
    }

    /// A range past the end of the text is ignored rather than panicking.
    ///
    /// The interpreter cannot produce one, but `Described` is public and its ranges come from
    /// a document; `text.get` answering `None` is what keeps a malformed span from indexing.
    #[test]
    fn a_range_outside_the_text_governs_nothing() {
        let spoken = speech("abc", &[alt(2..99, "gone")], None);
        assert_eq!(spoken.first().map(|run| run.text.as_str()), Some("abc"));
    }
}
