//! The fonts family of the decision table: the rows whose requirement identifier begins
//! `fonts/`, the sentences those rows quote, and the faces a font row's answer counts.
//! `doc/pdfa-blocking-issues.md` section 3 is the same family.
//!
//! Whichever of [`super::REMEDIES`], [`super::CONDITIONAL`], [`super::SHAPES`] and
//! [`super::REFUSED_BY_NAME`] this family has rows for reads them beside every other family's, and
//! the dispatch in [`super`] reads only those four tables, so where a row is written decides
//! nothing about its answer (`doc/adr/2634`).

use super::super::prepare::Prepared;
use super::super::rewrite::Rewrite;
use super::{Answer, Authorisations, Because, Conditional, Decision, Loss, Losses, Remedy, losing};

/// What embedding a substitute face asserts, which is `doc/questions/A47`'s condition on it.
///
/// `doc/adr/0927`: the sentence travels with the decision. What changes here is larger than what
/// any other `Stated` row changes and the argument for it is `doc/pdf-a-conversion-limits.md`
/// section 4.9's: a file that names a font and does not carry it **has no appearance of its
/// own** — every reader picks a face at display time and they pick different ones — so writing
/// one down settles a question the file left open rather than overriding an answer it gave.
const SUBSTITUTION_REINTERPRETS: &str = "a face this program ships, or the face of this \
     machine's the viewer drew it from, is now embedded for a font this file names and never \
     carried, so the shapes every reader draws for it are this program's choice rather than each \
     reader's. That is a change, and it is the one an archival \
     format asks for: the file had no appearance of its own before, because a non-embedded font \
     is drawn from whatever the machine opening it happens to have. Nothing moves — the glyph \
     widths the font dictionary states are untouched, and where one of the standard 14 stated \
     none the widths written are the ones it was already laid out by, so every line breaks and every word sits \
     where its producer put it — and the report names, per font, what was asked for, what was \
     embedded and whether the face's own advances were used or restated. Supply the real font \
     with --font, or ask for --no-substitute and the font is refused by name instead";

/// What naming a TrueType code's glyph in the encoding asserts (`doc/adr/1942`).
///
/// The glyph is unchanged — the preparation proves that — but the route to it was this reader's
/// choice and is now the file's statement: a reader that chose differently drew something else
/// before and draws this afterwards. ISO 32000-2 §9.6.5.4 licenses each reader's choice, and an
/// archival format exists to remove exactly that latitude.
const STANDARD_ROUTE_REINTERPRETS: &str = "a TrueType font's encoding now names the glyph \
     each code it shows is drawn with, where the file left the code to a mapping each reader \
     chooses for itself: the character the font's own ToUnicode states for the code, through \
     the font program's own Unicode cmap subtable, which is the glyph this renderer drew. A \
     reader that chose another mapping drew another glyph before and draws this one now; no \
     glyph changes for this renderer, the program is untouched, and the code's text is what \
     the ToUnicode already said";

/// What restating a TrueType font's `cmap` asserts (`doc/adr/2520`).
///
/// Every code draws the glyph this reader drew, which the preparation proves; what changes is
/// that the file now states the route to it, and with the route the font's kind: a font whose
/// descriptor said Nonsymbolic and whose encoding named its glyphs is a symbolic font whose
/// program maps the codes themselves.
/// What a `.notdef` code given a glyph of its own changes, for a reader other than this one.
const NOTDEF_GLYPH_REINTERPRETS: &str = "a code a TrueType font sent to its program's .notdef \
     glyph now reaches a glyph of its own whose outline and advance are copied from .notdef's, \
     by the restated cmap, or for a CIDFontType2 by the CIDToGIDMap restated as a stream. The \
     page draws the same marks for this renderer, and a reader that \
     drew nothing for .notdef, or a box of its own, draws the program's .notdef outline now. A \
     shown code a simple TrueType font sent to no glyph reaches a glyph at its width with no \
     outline, as this renderer drew no mark for it; a reader that drew the program's .notdef \
     for such a code draws nothing now";

const TRUETYPE_CMAP_REINTERPRETS: &str = "a rendered TrueType font is now the symbolic font ISO \
     32000-2 \u{a7}9.6.5.4 describes: its program's cmap maps each code to the glyph this \
     renderer drew for it and its dictionary states no Encoding, where the file left some codes \
     to a mapping each reader chooses for itself or named glyphs outside the Adobe Glyph List. \
     No glyph changes for this renderer; a reader that chose another mapping drew another glyph \
     before and draws this one now, and the encoding's glyph names no longer say which \
     character a code is, so the text a reader extracts comes from the font's ToUnicode where \
     it has one";

/// What writing `/CIDToGIDMap` `/Identity` asserts, which is `doc/adr/0948`'s condition on it.
const COLLECTION_RESTATED: &str = "an embedded CIDFont in this file named a character collection \
     its CMap did not, and now names the CMap's. ISO 32000-2 \u{a7}9.7.4.2 selects an embedded \
     program's glyphs through its CIDToGIDMap or its own charset, not through the collection, and \
     the font is restated only where this reader consults the collection for nothing - no CID to \
     Unicode table, vertical forms or glyph classes of either collection - so no glyph and no \
     character extracted changes. What changes is the CIDFont's statement of which collection \
     its CIDs are numbered in";

const CID_TO_GID_REINTERPRETS: &str = "an embedded Type 2 CIDFont in this file stated no \
     CIDToGIDMap, and now states Identity. ISO 32000-1's table gives Identity as that entry's own \
     default, and that edition is the one describing this file: ISO 19005-2 section 5.1 makes a \
     PDF/A-2 file one that adheres to ISO 32000-1, and a file labelled PDF 1.7 or earlier is one \
     ISO 32000-2 \u{a7}I.2 says ISO 32000-1 describes. So every reader of this file was already \
     mapping CIDs to glyphs that way and no glyph changes. What changes is that the file now says \
     so, which is what makes it readable the same way under ISO 32000-2 — where the entry is \
     required and has no default";

/// This family's rows of [`super::REMEDIES`].
pub(super) const REMEDIES: &[Remedy] = &[
    // ISO 19005-2 section 6.2.11.7.2, and Level U and above only: a CMap on every non-exempt
    // font, and usable values in the ones a file already has. Both are answered by the same
    // derivation, because a CMap written over a font's referenced codes states a value for each
    // of them — so a placeholder the producer left is replaced by the same act that supplies a
    // missing entry. Part 4 states the first rule as a `should` and its own row is the second
    // one, which is why the identifier is shared and the clause is not.
    Remedy {
        requirement: "fonts/to-unicode-present",
        answer: Answer::Mechanical(Rewrite::ToUnicode),
    },
    Remedy {
        requirement: "fonts/to-unicode-values-are-usable",
        answer: Answer::Mechanical(Rewrite::ToUnicode),
    },
    // ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1, the same sentence's "as
    // defined in" section 9.9: Table 124 files a program's format under a key and a name, and the
    // bytes settle the format, so the program is refiled and carried (ADR 2307).
    Remedy {
        requirement: "fonts/font-programs-filed-by-their-format",
        answer: Answer::Mechanical(Rewrite::FontProgramRefiled),
    },
    Remedy {
        requirement: "fonts/font-programs-embedded",
        answer: Answer::Stated(
            None,
            Rewrite::SubstituteFontProgram,
            SUBSTITUTION_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.11.5, ISO 19005-4 section 6.2.10.5: the widths the dictionary
    // states and the ones the program states agree. Section 4.9 sets out three ways to reach
    // that and ranks them; this is the second, and the third — restating `/Widths` — is the one
    // it calls **never**, because `/Widths` is what positions the glyphs.
    Remedy {
        requirement: "fonts/widths-array-covers-character-range",
        answer: Answer::Stated(
            None,
            Rewrite::FontWidthArrayRestated,
            "the short Widths array narrows LastChar to the codes it covers, or entries past \
             LastChar are removed from a long array. Codes outside the retained range use \
             MissingWidth, as this viewer already laid them out; the page's marks do not move",
        ),
    },
    // ISO 19005-2 section 6.2.11.2 through ISO 32000-1 Table 111, ISO 19005-4 section 6.2.10.2
    // through ISO 32000-2 Table 109: the three entries are written as the advances this reader
    // lays the glyphs by, so nothing on the page moves here; what the output states is the reading
    // a reader of the source had to choose, since a dictionary with a descriptor and no widths is
    // one the base standard does not admit (`doc/adr/2686`), and a font used only in mode 3 with
    // no descriptor is given one of the face substituted for it (`doc/adr/2688`).
    Remedy {
        requirement: "fonts/simple-font-states-its-widths",
        answer: Answer::Stated(
            None,
            Rewrite::FontWidthsStated,
            "a font that stated a FontDescriptor and not all of FirstChar, LastChar and Widths is \
             given the three, as the advances this viewer laid its glyphs out by: for one of the \
             standard 14 names its published metrics, otherwise the descriptor's MissingWidth. \
             The page's marks do not move here; a reader that laid the source out by its font \
             program instead would see the glyphs move where the program's widths differ. A font \
             used only in text rendering mode 3 that stated no FontDescriptor is given the three \
             and a descriptor of the face this viewer substitutes for it, with no program: it \
             paints nothing, and its text and the positions of its glyphs are the ones this \
             viewer read; a reader that laid the source out by another face would place the \
             invisible text differently",
        ),
    },
    Remedy {
        requirement: "fonts/widths-agree-with-the-program",
        answer: Answer::Mechanical(Rewrite::RestateFontMetrics),
    },
    // ISO 19005-4 section 6.2.10.5's third paragraph, which part 2 does not state: the same
    // agreement going down the page, between §9.7.4.3's `/DW2` and `/W2` and the program's
    // `vmtx`. The direction is the row above's and the argument for it is stronger — §9.2.4 is
    // an inference about which statement positions a glyph, while §9.9.1 says outright that the
    // "vhea" and "vmtx" tables "shall never be used by a PDF processor" and that `/DW2` and
    // `/W2` are the only way to specify vertical metrics in PDF. So restating the program is a
    // change no conforming reader can observe, and restating the dictionary would move every
    // glyph on a vertical line on the authority of a table nothing may read.
    Remedy {
        requirement: "fonts/vertical-metrics-agree-with-the-program",
        answer: Answer::Mechanical(Rewrite::RestateVerticalFontMetrics),
    },
    // ISO 19005-2 section 6.2.11.4.2, which part 4 does not state.
    Remedy {
        requirement: "fonts/charset-lists-every-glyph-in-the-program",
        answer: Answer::Mechanical(Rewrite::DescriptorSetRemoved),
    },
    Remedy {
        requirement: "fonts/cidset-lists-every-cid-in-the-program",
        answer: Answer::Mechanical(Rewrite::DescriptorSetRemoved),
    },
    // ISO 19005-2 section 6.2.11.3.1, ISO 19005-4 section 6.2.10.3.1: the CIDFont's collection
    // restated as its CMap's where this reader reads it for nothing, and refused by
    // `super::super::collections`'s sentence elsewhere (ADR 2717).
    Remedy {
        requirement: "fonts/cid-system-info-agrees-with-the-cmap",
        answer: Answer::Stated(None, Rewrite::CollectionRestated, COLLECTION_RESTATED),
    },
    // ISO 19005-2 section 6.2.11.3.2, ISO 19005-4 section 6.2.10.3.2 — written where ISO
    // 32000-1 describes the file, and `super::super::sites` refuses it elsewhere and says why (ADR 2246).
    Remedy {
        requirement: "fonts/cid-to-gid-map-present",
        answer: Answer::Stated(None, Rewrite::CidToGidIdentity, CID_TO_GID_REINTERPRETS),
    },
    // ISO 19005-2 section 6.2.11.2, ISO 19005-4 section 6.2.10.2, through ISO 32000-2 §9.6.2.1's
    // Table 109 and the three font tables beside it: each makes `/Type` required with the one
    // value `Font`, and a dictionary a `/Font` resource or a `/DescendantFonts` array names is a
    // font by where it stands, so the name written is the only one the entry may hold and says
    // what the file's own structure already says (`doc/adr/2608`).
    Remedy {
        requirement: "fonts/font-dictionary-states-type-font",
        answer: Answer::Mechanical(Rewrite::FontTypeStated),
    },
    // ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6 — the two rows of that
    // subclause whose subject is the font dictionary rather than the program. Each is mechanical
    // only where `super::super::sites` has proved the glyph every code reaches is unchanged, and refused
    // by that preparation where it is not.
    Remedy {
        requirement: "fonts/symbolic-truetype-states-no-encoding",
        answer: Answer::OrStated(
            &Answer::Mechanical(Rewrite::SymbolicTrueTypeEncodingRemoved),
            Rewrite::TrueTypeCmapRestated,
            TRUETYPE_CMAP_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "fonts/non-symbolic-truetype-uses-a-standard-encoding",
        answer: Answer::OrStated(
            &Answer::Mechanical(Rewrite::StandardTrueTypeEncoding),
            Rewrite::TrueTypeCmapRestated,
            TRUETYPE_CMAP_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.11.8, ISO 19005-4 section 6.2.10.9, for a Type 3 font: the glyph is
    // the producer's own procedure under the name `.notdef`, and renaming it moves nothing
    // (`doc/adr/1942`). A font with a program keeps ADR 0816's fence, which `super::super::sites` returns
    // where any font the requirement names is not a Type 3 one.
    //
    // A font with a TrueType program takes the second answer: the program gains a glyph whose
    // outline and advance are its own glyph 0's, and each code that reached `.notdef` reaches it
    // through the restated `cmap`, so the page draws what this reader drew and no text-showing
    // operator references `.notdef` (`doc/questions/A694` item 3, `doc/adr/2543`). Stated, as the
    // restated `cmap` is; refused by `super::super::fonts` for every other program.
    Remedy {
        requirement: "fonts/no-notdef-glyph-shown",
        answer: Answer::OrStated(
            &Answer::Mechanical(Rewrite::Type3NotdefRenamed),
            Rewrite::NotdefGlyphGiven,
            NOTDEF_GLYPH_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.11.4.1's last requirement for the same codes, in the embedded
    // program's half `pdf_archive` asks it of, by the same glyph (`doc/adr/2543`).
    Remedy {
        requirement: "fonts/embedded-programs-define-every-glyph-shown",
        answer: Answer::Stated(None, Rewrite::NotdefGlyphGiven, NOTDEF_GLYPH_REINTERPRETS),
    },
    // The same subclause's last paragraph: every code reaches its glyph by §9.6.5.4's own route.
    // Stated rather than mechanical, because the glyph is this reader's choice made the file's
    // statement; refused by `super::super::sites` wherever the file does not itself say which character
    // the code is, or the names cannot be proved to leave every shown code on its glyph.
    Remedy {
        requirement: "fonts/truetype-codes-reach-glyphs-by-the-standard-route",
        answer: Answer::OrStated(
            &Answer::Stated(
                None,
                Rewrite::StandardRouteNames,
                STANDARD_ROUTE_REINTERPRETS,
            ),
            Rewrite::TrueTypeCmapRestated,
            TRUETYPE_CMAP_REINTERPRETS,
        ),
    },
    // ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6, the four paragraphs that hold a
    // rendered TrueType font's program and its encoding to each other: the font becomes the
    // symbolic font §9.6.5.4 describes, its program's cmap stating the glyph this reader drew for
    // each code. Stated, because the route was the reader's choice and is now the file's
    // (`doc/questions/A694` item 3, `doc/adr/2520`); refused by `super::super::fonts` wherever it cannot
    // prove the font loads again with every code on the glyph it reached before.
    Remedy {
        requirement: "fonts/non-symbolic-truetype-differences-are-listed-names",
        answer: Answer::Stated(
            None,
            Rewrite::TrueTypeCmapRestated,
            TRUETYPE_CMAP_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
        answer: Answer::Stated(
            None,
            Rewrite::TrueTypeCmapRestated,
            TRUETYPE_CMAP_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "fonts/non-symbolic-truetype-program-maps-every-code",
        answer: Answer::Stated(
            None,
            Rewrite::TrueTypeCmapRestated,
            TRUETYPE_CMAP_REINTERPRETS,
        ),
    },
    Remedy {
        requirement: "fonts/symbolic-truetype-program-has-a-usable-cmap",
        answer: Answer::Stated(
            None,
            Rewrite::TrueTypeCmapRestated,
            TRUETYPE_CMAP_REINTERPRETS,
        ),
    },
];

/// This family's rows of [`super::CONDITIONAL`].
pub(super) const CONDITIONAL: &[(&str, Conditional)] = &[(
    "fonts/font-programs-embedded",
    Conditional::PerDocument(
        Losses::one(Loss::MachineFace)
            .with(Loss::SubstituteStretch)
            .with(Loss::GlyphsFromToUnicode)
            .with(Loss::LayoutFromThisViewer)
            .with(Loss::TextAsImage)
            .with(Loss::PageAsImage)
            .with(Loss::TaggingOfPhotographedPages)
            .with(Loss::OptionalContentOfPhotographedPages),
    ),
)];

/// This family's rows of [`super::REFUSED_BY_NAME`].
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &[
    // ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2 through §9.7.6.1's Table 119:
    // a font a text-showing operator runs with that has no `CIDFont` states nothing a rewrite
    // could restate, and building one is a substitution nothing here performs (ADR 2433).
    (
        "fonts/descendant-fonts-holds-one-cidfont",
        Because::NotBuiltYet(NO_CIDFONT_TO_DESCEND_TO),
    ),
    // ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6: the embedded program and the
    // encoding held to each other, which `REMEDIES` answers by stating the route this reader took
    // in the program itself (`doc/adr/2520`).
    // ISO 19005-2 section 6.2.11.3.3, ISO 19005-4 section 6.2.10.3.3.
    (
        "fonts/cmap-embedded-or-predefined",
        Because::TheFence(A_CMAP_IS_THE_ENCODING),
    ),
    (
        "fonts/cmap-uses-only-predefined-cmaps",
        Because::TheFence(A_CMAP_IS_THE_ENCODING),
    ),
    // ISO 19005-2 section 6.2.11.3.3, ISO 19005-4 section 6.2.10.3.3.
    (
        "fonts/embedded-cmap-states-its-own-write-mode",
        Because::NotBuiltYet(WRITE_MODE_DISAGREEMENT),
    ),
];

/// Why a Type 0 font that has no `CIDFont` is refused.
///
/// ISO 32000-2 §9.7.6.1's Table 119 makes `/DescendantFonts` the one-element array naming the
/// `CIDFont`, and §9.7.4.1 makes the `CIDFont` what carries the glyph selection, the widths and the
/// character collection. With none there is no statement for a rewrite to restate, only a
/// dictionary to build, which is `doc/pdf-a-conversion-limits.md` section 4.9's substitution
/// carried out for a font with nothing to substitute into (ADR 2433).
const NO_CIDFONT_TO_DESCEND_TO: &str = "a Type 0 font here has no CIDFont as its descendant - its \
     DescendantFonts is missing, is not the one-element array ISO 32000-2 \u{a7}9.7.6.1's Table \
     119 requires, or holds something other than a CIDFont dictionary, a null among them - so the \
     file states no glyph selection, widths or character collection for any code the font shows. \
     Building a CIDFont for it would be doc/pdf-a-conversion-limits.md section 4.9's substitution \
     carried out for a font that has no CIDFont dictionary to substitute into, and that is not \
     built";

/// Why a page that draws a glyph its own embedded program has not got is refused.
///
/// The mapping is fixed by a program the file carries, so the ways out are editing the content
/// stream to stop showing the code, which takes a mark off the page, or giving the program a glyph
/// for it. A copy of the program's own `.notdef` outline draws what this reader drew and invents
/// nothing, and `fonts::notdef_glyph` writes one into a TrueType program, simple (`doc/adr/2543`)
/// or under a `CIDFontType2` (`doc/adr/2556`); this is the sentence for every program it does not
/// reach.
pub(in crate::archive) const THE_GLYPH_IS_NOT_IN_THE_PROGRAM: &str = "this file shows a character code whose glyph its \
     own embedded font program does not define, so the code reaches the .notdef glyph — which \
     ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing \
     operator to reference in any rendering mode. The file carries the program, so the mapping \
     is the producer's and fixed. For a TrueType program this conversion gives such a code a \
     glyph of its own copied from the program's .notdef glyph, so the page draws what it drew — \
     through a restated cmap for a simple font, and through a CIDToGIDMap restated as a stream \
     for a CIDFontType2; the program here is another kind — a CFF or Type 1 program, whose \
     charstrings this version does not write — or a TrueType program this conversion could not \
     extend and prove. Where the font is *not* \
     embedded this is not a refusal at all, because doc/pdf-a-conversion-limits.md section 4.9 \
     then builds the program and chooses what each code maps to; supplying the intended font \
     with --font moves this document into that case";

/// Why a `CMap` this file neither embeds nor names from the predefined set is not written.
const A_CMAP_IS_THE_ENCODING: &str = "a CMap is what maps a composite font's character codes to \
     CIDs, so supplying one this file neither embeds nor takes from the predefined set means \
     writing the encoding its producer did not — every code would then select whatever this \
     program chose, which is doc/questions/A48's forbidden half. The same holds of an embedded \
     CMap that builds on one the base standard does not predefine: what it builds on is not here \
     to be carried, and inventing it invents the mapping";

/// Why a `CMap` stream and its program disagreeing about `/WMode` is not settled.
const WRITE_MODE_DISAGREEMENT: &str = "this CMap stream's WMode entry and the write mode the \
     CMap program itself states disagree, and between them they decide whether the text runs \
     down the page or across it. Making them agree means choosing which of the file's two \
     statements its producer meant, which changes either the writing direction a reader lays the \
     text out in or the program's own bytes. That is a question for the document's owner, and no \
     interface exists to ask it";

/// ISO 19005 section 6.2.11.4.1's embedded faces: a statement, or every loss the embedding
/// incurs — `machine-face` where the shipped face goes over a face of this machine's the viewer
/// draws instead (`doc/adr/2162`), `substitute-stretch` where it goes in without the fitting the
/// viewer draws it with (`doc/adr/2262`) — carried by the one decision, so a caller authorises
/// all of them or the font row stops.
pub(super) fn embedded_faces(
    reinterprets: &'static str,
    authorised: Authorisations,
    prepared: &Prepared,
) -> Decision {
    if let Some(because) = prepared.obstacle(Rewrite::SubstituteFontProgram) {
        return Decision::Refused(because);
    }
    let losses = prepared.substitutes.as_ref().map_or_else(
        |_| Losses::default(),
        super::super::fonts::Substitutes::losses,
    );
    if losses.is_empty() {
        return Decision::Stated {
            rewrite: Rewrite::SubstituteFontProgram,
            reinterprets,
        };
    }
    losing(losses, Rewrite::SubstituteFontProgram, authorised)
}
