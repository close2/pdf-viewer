//! The content-stream fences of the decision table: the rows refused because the bytes their
//! requirement fails are inside a content stream the producer wrote, whatever the identifier's
//! prefix, and the gate [`super::decide`] keeps in front of the whole table for a syntax failure
//! inside one. `doc/pdfa-blocking-issues.md` section 5 is the same family.
//!
//! Whichever of [`super::REMEDIES`], [`super::CONDITIONAL`], [`super::SHAPES`] and
//! [`super::REFUSED_BY_NAME`] this family has rows for reads them beside every other family's, and
//! the dispatch in [`super`] reads only those four tables, so where a row is written decides
//! nothing about its answer (`doc/adr/2634`).

use pdf_archive::Judgement;
use pdf_archive::Outcome;

use super::super::prepare::Prepared;
use super::super::rewrite::Rewrite;
use super::{
    Answer, Authorisations, Because, Conditional, Decision, Loss, Losses, Remedy, answer_of,
};

/// The operator row, where a run this reader draws as a number is answered by writing the number.
const OPERATORS: &str = "graphics/only-operators-the-base-standard-defines";

/// This family's rows of [`super::REMEDIES`].
pub(super) const REMEDIES: &[Remedy] = &[
    // ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2, where the operator is a run ISO
    // 32000-2 §7.3.3 spells as no number and this reader draws a number for (`doc/adr/1004`):
    // the number written in its place, at the loss `content-as-drawn` (`doc/adr/2638`). Every
    // other operator keeps the fence's refusal, so [`run_written_as_drawn`] routes on what
    // `super::super::as_drawn` found, as [`CONDITIONAL`] says.
    Remedy {
        requirement: OPERATORS,
        answer: Answer::Mechanical(Rewrite::RunWrittenAsDrawn),
    },
];

/// This family's rows of [`super::CONDITIONAL`].
pub(super) const CONDITIONAL: &[(&str, Conditional)] = &[(
    OPERATORS,
    Conditional::PerDocument(Losses::one(Loss::ContentAsDrawn)),
)];

/// The operator row's answer: the number written in place of every run where the profile
/// authorised `content-as-drawn` and every operator the survey found is such a run, the reason
/// the rewrite could not be made where it could not, and the fence's refusal otherwise, which is
/// what a profile that authorised nothing meets.
pub(super) fn run_written_as_drawn(authorised: Authorisations, prepared: &Prepared) -> Decision {
    if !authorised.grants(Loss::ContentAsDrawn) {
        return Decision::Refused(Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS));
    }
    match &prepared.as_drawn {
        Ok(drawn) if drawn.answers_every_operator => {
            prepared.obstacle(Rewrite::RunWrittenAsDrawn).map_or_else(
                || {
                    answer_of(
                        Answer::Loses(Loss::ContentAsDrawn, Rewrite::RunWrittenAsDrawn),
                        authorised,
                        prepared,
                    )
                },
                Decision::Refused,
            )
        }
        // A document whose unlisted operators are not all runs, or hold none, is the fence's as
        // where the loss is not granted: the remedy has nothing to answer there.
        Ok(_) => Decision::Refused(Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS)),
        Err(because) if crate::archive::as_drawn::reached_no_run(*because) => {
            Decision::Refused(Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS))
        }
        Err(because) => Decision::Refused(*because),
    }
}

/// This family's rows of [`super::REFUSED_BY_NAME`].
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &[
    // ISO 19005-2 section 6.7.3.2 asks for space characters *inside show strings*, which is the
    // producer's page and ADR 0816's fence.
    (
        "logical-structure/word-boundaries",
        Because::TheFence(WORD_BOUNDARIES_ARE_ON_THE_PAGE),
    ),
    // ISO 19005-2 section 6.2.11.7.3's `/ActualText` is a marked-content property list or a
    // structure element's entry over a *span of a content stream*, so supplying one means
    // deciding where the span starts and ends inside the producer's page.
    (
        "fonts/actual-text-covers-private-use-characters",
        Because::TheFence(ACTUAL_TEXT_IS_ON_THE_PAGE),
    ),
    // The rules a content stream's own bytes fail, which this verb carries byte for byte.
    (
        "graphics/inline-image-interpolation-is-off",
        Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS),
    ),
    (
        "file-structure/inline-image-filters",
        Because::TheFence(CONTENT_STREAM_IS_THE_PRODUCERS),
    ),
    (
        "graphics/rendering-intent-operator-names-one-of-four",
        Because::TheFence(RENDERING_INTENT_OPERAND),
    ),
    // ISO 19005-4 section 6.2.10.5 and section 6.2.10.8: both are the producer's page.
    (
        "fonts/type3-glyph-procedures-state-their-width",
        Because::TheFence(TYPE3_WIDTH_IS_IN_THE_PROCEDURE),
    ),
    (
        "fonts/actual-text-states-no-private-use",
        Because::TheFence(ACTUAL_TEXT_STATES_THE_PRODUCERS_WORDS),
    ),
];

/// Why show strings with no spaces between words are not repaired.
const WORD_BOUNDARIES_ARE_ON_THE_PAGE: &str = "word boundaries are asked for inside the show \
     strings themselves, so supplying them means editing the producer's content stream and \
     re-deciding the glyph positioning that goes with it. ADR 0816's fence is where that stops. \
     PDF/A-2u and PDF/A-2b do not state this requirement";

/// Why a Private Use character's `/ActualText` is not supplied.
const ACTUAL_TEXT_IS_ON_THE_PAGE: &str = "an ActualText entry covers a span of a content stream, \
     so writing one means deciding where inside the producer's page that span begins and ends — \
     and its value means deciding what a character in the Private Use Area was for, which is the \
     evidence Level A exists to require rather than something to be manufactured";

/// Why a rule a content stream's own bytes fail is not repaired.
const CONTENT_STREAM_IS_THE_PRODUCERS: &str = "this requirement is failed by bytes inside a \
     content stream, which this verb carries from the source byte for byte. Meeting it means \
     editing the producer's page, and ADR 0816's fence is where that stops — \
     doc/pdf-a-conversion-limits.md section 5.2. The one exception doc/questions/A50 grants is a \
     closed list of operator spellings the standard itself documents as equivalent, applied only \
     where a deprecation rule names one, and this is not that";

/// Why an unrecognised `ri` operand is not restated, where an `/RI` entry's answer differs.
const RENDERING_INTENT_OPERAND: &str = "the rendering intent operator's operand names an intent \
     ISO 32000-2 does not define. §8.6.5.8 says what a processor does with such a name — it uses \
     RelativeColorimetric — so the answer is known, and the operand is inside a content stream \
     this verb carries byte for byte, which is ADR 0816's fence. The same name in a graphics \
     state's RI entry or an image dictionary's Intent entry is not refused for this reason: that \
     is an object, and doc/pdf-a-conversion-limits.md's own answer for it is a rewrite nobody \
     has built";

/// Why a Type 3 glyph procedure's width is not reconciled with the font dictionary's.
const TYPE3_WIDTH_IS_IN_THE_PROCEDURE: &str = "a Type 3 glyph procedure's d0 or d1 operands \
     disagree with the width its font dictionary states, and both halves are closed. The \
     operands are inside the glyph procedure, which is a content stream this verb carries byte \
     for byte; and §9.2.4 makes the dictionary's widths what positions every glyph on the line, \
     so doc/pdf-a-conversion-limits.md section 4.9 restates those never. Changing either moves \
     marks on the page";

/// Why an `/ActualText` holding a Private Use character is neither corrected nor removed.
const ACTUAL_TEXT_STATES_THE_PRODUCERS_WORDS: &str = "ISO 19005-4 section 6.2.10.8 forbids a \
     Private Use character inside an ActualText entry, and that entry is a marked-content \
     property list or a structure element's entry over a span of the producer's page. Changing \
     its value means deciding what the producer meant that span to say, which doc/questions/A48 \
     forbids; removing it takes away the only statement the file makes about what those glyphs \
     spell, which at Level A is the evidence the level exists to require";

/// The row whose repair ISO 32000-2 §7.3.4.3 states the result of.
pub(super) const ODD_HEXADECIMAL_DIGITS: &str = "file-structure/hexadecimal-string-digits";

/// Whether this document fails [`ODD_HEXADECIMAL_DIGITS`] inside a content stream.
///
/// Asked by [`super::super::prepare`] so that the repair is worked out for the documents that need it
/// and for no others: the same failure named at an object is answered by the whole-file rewrite,
/// which writes every string in §7.3.4.3's even-digit form anyway.
pub(in crate::archive) fn odd_hexadecimal_digits_on_a_page(input: &pdf_archive::Report) -> bool {
    input
        .failures()
        .any(|judgement| judgement.id == ODD_HEXADECIMAL_DIGITS && in_a_content_stream(judgement))
}

/// A syntax rule the whole-file rewrite cannot reach, because the syntax is on a page.
pub(super) const SYNTAX_INSIDE_CONTENT: &str = "this failure is inside a content stream, which this verb \
     carries byte for byte; correcting \
     it would edit the producer's page, and ADR 0816's fence is where that stops — \
     doc/pdf-a-conversion-limits.md section 5.2";

/// Whether any place the validator kept names a page rather than an object.
///
/// `pdf_archive` records an object's own syntax at `Where::object` and a content stream's at
/// `Where::page`, so this is the question "was it on a page" asked of the report rather than of
/// the document. The list is capped, so a document failing in both kinds past the cap could be
/// read the other way; stage 3's own verdict is what catches that, and it refuses to write a
/// file that does not conform.
pub(super) fn in_a_content_stream(judgement: &Judgement) -> bool {
    let Outcome::Failed { places, .. } = &judgement.outcome else {
        return false;
    };
    places
        .iter()
        .any(|finding| finding.place.page.is_some() && finding.place.object.is_none())
}
