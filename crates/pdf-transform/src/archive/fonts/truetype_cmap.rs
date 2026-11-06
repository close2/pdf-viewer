//! A rendered TrueType font whose codes reach their glyphs by the route ISO 19005 admits, because
//! its program's `cmap` now says where this reader found each of them.
//!
//! ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 hold a rendered TrueType font's
//! embedded program and its encoding to ISO 32000-2 §9.6.5.4's rules, and a font whose program
//! lacks the subtable its encoding needs, or whose `Differences` array names a glyph outside the
//! Adobe Glyph List, fails them. The producer's file leaves such a code to a mapping this reader
//! chooses (the same subclause's licence: "a PDF processor may supply a mapping of its
//! choosing"), and ISO 19005 withdraws that licence. What the reader drew is the state the
//! archive keeps (`doc/questions/A694`, item 3, under `doc/questions/A48`), and the subclause's
//! symbolic route states it without a name anywhere:
//!
//! > When the font has no Encoding entry, or the font descriptor's Symbolic flag is set (in which
//! > case the Encoding entry is ignored), this shall occur:
//!
//! and a (3, 0) subtable is then read with the high byte of one of its ranges prepended to each
//! byte of the string, which for the range 0xF000 to 0xF0FF is the prefix used below.
//!
//! The font becomes the symbolic font §9.6.5.4 describes: its descriptor states the Symbolic flag
//! and not the Nonsymbolic one, its dictionary states no `/Encoding`, and its program carries the
//! two subtables ISO 19005 names for it, the Macintosh Roman (1, 0) one mapping each code to the
//! glyph the reader drew and the Microsoft Symbol (3, 0) one mapping 0xF000 plus the code to the
//! same glyph. ISO 32000-2 §9.6.5.4's NOTE 1 says that "a TrueType/OpenType font program to be
//! used in a PDF file could need modification to conform to these guidelines"; the glyph
//! outlines, the metrics and every other table are the producer's bytes.
//!
//! **Nothing is written that was not proved**: the font is loaded again with the new dictionary,
//! descriptor and program, and each of the 256 codes must reach the glyph it reached before and
//! must no longer need a mapping of the reader's choosing. A font failing the proof is refused
//! with a sentence that says so.

use std::collections::{BTreeMap, BTreeSet};

use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use pdf_archive::survey::Survey;

use super::super::decision::Because;
use super::super::prepare::Spare;
use super::super::sites::{dictionary_of, loaded_as_stated, objects_named};
use super::Substitutes;
use super::{cff_notdef_glyph, cid_cff_notdef_glyph, cid_notdef_glyph, notdef_glyph};

/// The requirements a font's restated `cmap` answers.
pub(in crate::archive) const ANSWERED: [&str; 4] = [
    "fonts/non-symbolic-truetype-differences-are-listed-names",
    "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
    "fonts/non-symbolic-truetype-program-maps-every-code",
    "fonts/symbolic-truetype-program-has-a-usable-cmap",
];

/// The three rows whose own rewrites name a font's encoding, which this restatement answers for
/// a font whose own preparation refused (`super::super::decision::Answer::OrStated`).
pub(in crate::archive) const STANDARD_ROUTE_ROWS: [&str; 3] = [
    "fonts/truetype-codes-reach-glyphs-by-the-standard-route",
    "fonts/symbolic-truetype-states-no-encoding",
    "fonts/non-symbolic-truetype-uses-a-standard-encoding",
];

/// The most font program, decoded, one conversion copies: 32 MiB.
///
/// Each restated font carries a program of its own, because its `cmap` differs, so the memory and
/// the time a conversion spends grow with the sum of the programs it copies and not with the
/// number of fonts. A measured document of 32 fonts sharing one 17 MB program took 429 MiB at its
/// peak and 119 s, against 140 MiB and 57 s without the restatement (ADR 2520 section 5); the
/// budget admits one such program twice, which is what a CJK face shared by a few fonts needs,
/// and refuses the rest with [`OVER_BUDGET`].
pub(in crate::archive) const MAX_COPIED_BYTES: usize = 32 << 20;

/// Why a file whose programs together pass [`MAX_COPIED_BYTES`] keeps the refusal.
const OVER_BUDGET: &str = "a rendered TrueType font in this file breaks ISO 19005-2 section \
     6.2.11.6's rules for the program's cmap or the encoding's names, and stating the route in \
     the program means copying each font's program with a cmap of its own; the programs of \
     the fonts named together pass the 32 MiB of font program one conversion copies, so none \
     is copied";

/// Why a font whose program cannot be restated keeps the refusal.
const NOT_RESTATED: &str = "a rendered TrueType font in this file breaks ISO 19005-2 section \
     6.2.11.6's rules for the program's cmap or the encoding's names, and this conversion states \
     the route this reader took in the program itself: the Macintosh Roman (1, 0) and Microsoft \
     Symbol (3, 0) subtables, mapping each code to the glyph this reader drew, with the font \
     made the symbolic one ISO 32000-2 \u{a7}9.6.5.4 describes. It does that only where it can \
     prove the font loads again with every code on the glyph it reached before and on no \
     mapping of the reader's choosing. This font is not an embedded TrueType program under a \
     descriptor of its own, or the program could not be rewritten, or the font failed the \
     proof";

/// The two `.notdef` rows a restated program answers by a glyph of its own for each code that
/// reached glyph 0 (`doc/adr/2543`).
pub(in crate::archive) const NOTDEF_ROWS: [&str; 2] = [
    "fonts/no-notdef-glyph-shown",
    "fonts/embedded-programs-define-every-glyph-shown",
];

/// Why a `.notdef` row stays refused where the restatement answered the rest.
pub(in crate::archive) const NOTDEF_NOT_GIVEN: &str = "this file shows a character code that \
     reaches the .notdef glyph of the program a font embeds, and this conversion gives such a \
     code a glyph of its own whose outline and advance are copied from the program's .notdef \
     glyph, reached through a restated cmap, or for a CIDFontType2 through a restated \
     CIDToGIDMap; in a simple font's CFF program the code is given a copy of the .notdef \
     charstring, or a glyph that draws nothing where it reached no glyph, under the name its \
     encoding gives it or a new one the encoding is restated to name, and in a simple font's \
     Type 1 program the same, its CharStrings dictionary decrypted, grown and encrypted again; \
     in a CIDFontType0's CID-keyed CFF program a CID the charset does not hold is given a copy \
     of the .notdef charstring and added to the charset. It does that only for a TrueType, \
     simple CFF, Type 1 or CID-keyed CFF program it can extend and prove, and only where every \
     font the \
     requirement names has one: a font here is another kind, reaches no glyph at all for its \
     code, or could not be extended";

/// Whether every font the [`NOTDEF_ROWS`] name has a program the copy reaches — a simple
/// TrueType font, a `Type0` font over a `CIDFontType2` ([`cid_notdef_glyph::is_cid_truetype`]),
/// or a simple font over a bare CFF program ([`cff_notdef_glyph::is_simple_cff`]) or a Type 1 program ([`cff_notdef_glyph::is_simple_type1`]) — and every place they name is an object: the condition
/// for the rows to join [`restated`], which refuses whole on a font it cannot carry.
pub(in crate::archive) fn notdef_fonts_have_truetype_programs(
    document: &Document,
    input: &pdf_archive::Report,
) -> bool {
    let Ok((named, elsewhere)) = objects_named(input, &NOTDEF_ROWS) else {
        return false;
    };
    !elsewhere
        && !named.is_empty()
        && named.into_iter().all(|id| {
            let resolved = document.get(id);
            dictionary_of(&resolved)
                .is_some_and(|dict| super::notdef_format::extends(document, dict))
        })
}

/// The fonts whose program is restated, and the objects that restate them.
#[derive(Debug, Default)]
pub(in crate::archive) struct CmapRestated {
    /// The descriptor each font dictionary now names, by font dictionary.
    pub(in crate::archive) descriptors: BTreeMap<ObjectId, ObjectId>,
    /// The fonts whose program was given a glyph for the codes that reached `.notdef`, a subset
    /// of [`Self::descriptors`]'s keys.
    pub(in crate::archive) notdef_given: BTreeSet<ObjectId>,
    /// The new descriptor and `/CIDToGIDMap` each `CIDFontType2` descendant names, by
    /// descendant, for the `Type0` fonts of [`Self::notdef_given`] (`doc/adr/2556`).
    pub(in crate::archive) descendants: BTreeMap<ObjectId, (ObjectId, ObjectId)>,
    /// The new descriptor, and the restated `/Encoding` where a code needed a name of its own,
    /// each simple font over a CFF or a Type 1 program of [`Self::notdef_given`] names, by font
    /// dictionary (`doc/adr/2578`, `doc/adr/2620`).
    pub(in crate::archive) cff: BTreeMap<ObjectId, (ObjectId, Option<Object>)>,
    /// The fonts of [`Self::cff`] whose program is a Type 1 program (`doc/adr/2620`).
    pub(in crate::archive) type1: BTreeSet<ObjectId>,
    /// The new descriptor each `CIDFontType0` descendant over a CID-keyed CFF program names, by
    /// descendant, for the `Type0` fonts of [`Self::notdef_given`] (`doc/adr/2598`).
    pub(in crate::archive) cid_cff: BTreeMap<ObjectId, ObjectId>,
    /// The embedded `CMap` each `Type0` font of [`Self::notdef_given`] that showed CID 0 names as
    /// its `/Encoding`, by font dictionary, with the fresh CID it sends that code to
    /// (`doc/adr/2622`).
    pub(in crate::archive) fresh_cids: BTreeMap<ObjectId, (ObjectId, u16)>,
    /// The `/W` each descendant of a font of [`Self::fresh_cids`] states from now on, by
    /// descendant, with the fresh CID at CID 0's width.
    pub(in crate::archive) widths: BTreeMap<ObjectId, Object>,
    /// Whether every font the [`NOTDEF_ROWS`] name is in [`Self::notdef_given`], which is when
    /// those rows are answered.
    pub(in crate::archive) notdef_answered: bool,
    /// The descriptors and programs this conversion adds.
    written: BTreeMap<ObjectId, Object>,
}

impl CmapRestated {
    /// The sentence the history states for the CID-keyed CFF programs given glyphs, or `None`
    /// where there is none (`doc/adr/2598`).
    fn cid_cff_sentence(&self) -> Option<String> {
        if self.cid_cff.is_empty() {
            return None;
        }
        Some(format!(
            "ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.8 and ISO 19005-4 sections 6.2.10.4.1 \
                 and 6.2.10.9: the CID-keyed CFF program of {} CIDFontType0 font(s) gained a glyph \
                 for each shown CID its charset did not hold, whose outline is the program's own \
                 .notdef glyph's, at the width the font states for the CID, with the CID added to \
                 the charset, so no mark moved and no content stream changed: descendant fonts, in \
                 the source's object numbering, {}",
            self.cid_cff.len(),
            self.cid_cff
                .keys()
                .take(MOST_NAMED)
                .map(|id| format!("{} {} R", id.number, id.generation))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// The new descriptors and programs, for the conversion to add.
    pub(in crate::archive) fn written(&self) -> impl Iterator<Item = (&ObjectId, &Object)> {
        self.written.iter()
    }

    /// The `xmpMM:History` parameters recording the fonts restated, naming the clause.
    ///
    /// `None` where no font was. `doc/questions/A48` allows a construction of this kind on
    /// condition that the file's own history names the clause it answers, and the entry names
    /// the font objects, up to [`MOST_NAMED`], so a reader of the archive can find them.
    pub(in crate::archive) fn history(&self) -> Option<String> {
        let glyphs = self.glyphs_history();
        let Some(fresh) = self.fresh_cid_sentence() else {
            return glyphs;
        };
        Some(match glyphs {
            Some(glyphs) => format!("{glyphs}; {fresh}"),
            None => fresh,
        })
    }

    /// The sentence the history states for the fonts whose code of CID 0 was sent to a fresh
    /// CID, or `None` where there is none (`doc/adr/2622`).
    fn fresh_cid_sentence(&self) -> Option<String> {
        if self.fresh_cids.is_empty() {
            return None;
        }
        Some(format!(
            "ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9, with ISO 32000-2 \
             section 9.7.4.2's CID 0: the Identity-H Encoding of {} Type0 font(s) is restated as \
             an embedded CMap sending each code to the CID Identity-H gave it except code <0000>, \
             which goes to a fresh CID given the glyph CID 0 drew at CID 0's width, so no mark \
             moved and no content stream changed: fonts, in the source's object numbering, with \
             the fresh CID, {}",
            self.fresh_cids.len(),
            self.fresh_cids
                .iter()
                .take(MOST_NAMED)
                .map(|(id, (_, cid))| format!("{} {} R to CID {cid}", id.number, id.generation))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// The sentence for the simple fonts whose CFF or Type 1 program gained glyphs, one clause
    /// per format; empty where none did.
    fn simple_programs_sentence(&self) -> String {
        [("CFF", false), ("Type 1", true)]
            .into_iter()
            .filter_map(|(program, type1)| {
                let fonts: Vec<_> = self
                    .cff
                    .keys()
                    .filter(|id| self.type1.contains(id) == type1)
                    .collect();
                (!fonts.is_empty()).then(|| {
                    format!(
                        "ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.8 and ISO 19005-4 sections \
                     6.2.10.4.1 and 6.2.10.9: the {program} program of {} simple font(s) gained a \
                     glyph for each code that reached .notdef, whose outline is the program's own \
                     .notdef glyph's, and for each code that reached no glyph, which draws \
                     nothing, each at the width the font dictionary states, so no mark moved and \
                     no content stream changed: fonts, in the source's object numbering, {}",
                        fonts.len(),
                        fonts
                            .iter()
                            .take(MOST_NAMED)
                            .map(|id| format!("{} {} R", id.number, id.generation))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// The `xmpMM:History` parameters recording the programs given glyphs and the `cmap`s
    /// restated; `None` where no font was.
    fn glyphs_history(&self) -> Option<String> {
        if self.descriptors.is_empty()
            && self.descendants.is_empty()
            && self.cff.is_empty()
            && self.cid_cff.is_empty()
        {
            return None;
        }
        let cff = self.simple_programs_sentence();
        let cff = match self.cid_cff_sentence() {
            None => cff,
            Some(cid_cff) if cff.is_empty() => cid_cff,
            Some(cid_cff) => format!("{cff}; {cid_cff}"),
        };
        if self.descriptors.is_empty() && self.descendants.is_empty() {
            return Some(cff);
        }
        let composite = if self.descendants.is_empty() {
            String::new()
        } else {
            format!(
                "ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9: the program of {} \
                 CIDFontType2 font(s) gained one glyph for each width whose outline and advance \
                 are its own .notdef glyph's, and the CIDToGIDMap is restated as a stream sending \
                 each CID that reached .notdef to the glyph for its width, so no mark moved and no \
                 content stream changed: descendant fonts, in the source's object numbering, {}",
                self.descendants.len(),
                self.descendants
                    .keys()
                    .take(MOST_NAMED)
                    .map(|id| format!("{} {} R", id.number, id.generation))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        // The CFF fonts' sentence follows whatever the TrueType ones said.
        let cff = if cff.is_empty() {
            cff
        } else {
            format!("; {cff}")
        };
        if self.descriptors.is_empty() {
            return Some(format!("{composite}{cff}"));
        }
        let named: Vec<String> = self
            .descriptors
            .keys()
            .take(MOST_NAMED)
            .map(|id| format!("{} {} R", id.number, id.generation))
            .collect();
        let rest = self.descriptors.len().saturating_sub(named.len());
        let tail = if rest == 0 {
            String::new()
        } else {
            format!(", and {rest} more")
        };
        let simple_given = self
            .notdef_given
            .len()
            .saturating_sub(self.descendants.len())
            .saturating_sub(self.cff.len());
        let given = if simple_given == 0 {
            String::new()
        } else {
            format!(
                "; for ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9, the \
                 program of {simple_given} of them gained one glyph whose outline and advance are its own \
                 .notdef glyph's, and each code that reached .notdef now reaches that glyph"
            )
        };
        Some(format!(
            "ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6: the program of {} \
             rendered TrueType font(s) now carries the Macintosh Roman (1, 0) and Microsoft \
             Symbol (3, 0) cmap subtables mapping each code to the glyph this reader drew, the \
             font is the symbolic font ISO 32000-2 section 9.6.5.4 describes and states no \
             Encoding{given}, so no mark moved and no content stream changed: fonts, in the source's object numbering, {}{tail}{}{composite}{cff}",
            self.descriptors.len(),
            named.join(", "),
            if composite.is_empty() { "" } else { "; " }
        ))
    }
}

/// How many fonts the history entry names before it counts the rest.
const MOST_NAMED: usize = 20;

/// Table 121's bit position 3, the Symbolic flag.
const SYMBOLIC: i64 = 1 << 2;

/// Table 121's bit position 6, the Nonsymbolic flag.
const NONSYMBOLIC: i64 = 1 << 5;

/// The first code of the Microsoft Symbol subtable's range, ISO 32000-2 §9.6.5.4's 0xF000.
const SYMBOL_PREFIX: u16 = 0xF000;

/// Every TrueType font the four `cmap` requirements name, restated, with `standard_route_too`
/// the fonts the three encoding requirements name as well, and with `notdef_too` the fonts the
/// two [`NOTDEF_ROWS`] name, each given a glyph for the codes that reached `.notdef`.
///
/// **The fonts are the requirements' whole population, and the copies are bounded twice.** The
/// fonts are every object the requirements' findings named ([`objects_named`]), not the places a
/// report keeps, which are a prefix past thirty-two (`doc/adr/2602`); that set is itself bounded
/// at [`pdf_archive::MAX_NAMED`] objects and refused by name past it. The programs the fonts copy
/// together may not pass [`MAX_COPIED_BYTES`], decoded.
///
/// ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6.
pub(in crate::archive) fn restated(
    document: &Document,
    input: &pdf_archive::Report,
    spare: &mut Spare,
    programs: &BTreeMap<ObjectId, Object>,
    (standard_route_too, notdef_too): (bool, bool),
    survey: Option<&Survey>,
    substitutes: Option<&Substitutes>,
) -> Result<CmapRestated, Because> {
    let refused = Because::NotBuiltYet(NOT_RESTATED);
    let rows: Vec<&str> = ANSWERED
        .iter()
        .chain(
            standard_route_too
                .then_some(&STANDARD_ROUTE_ROWS)
                .into_iter()
                .flatten(),
        )
        .chain(notdef_too.then_some(&NOTDEF_ROWS).into_iter().flatten())
        .copied()
        .collect();
    let (named, elsewhere) = objects_named(input, &rows)?;
    let notdef: BTreeSet<ObjectId> = if notdef_too {
        objects_named(input, &NOTDEF_ROWS)?.0
    } else {
        BTreeSet::new()
    };
    if elsewhere {
        return Err(refused);
    }
    let mut found = CmapRestated::default();
    let mut copied = 0_usize;
    for id in named {
        let resolved = document.get(id);
        let dict = dictionary_of(&resolved).ok_or(refused)?;
        // A `Type0` font reaches this only by the `.notdef` rows, and its glyphs are given by
        // CID: the strings it shows are the survey's, complete, since a code left out of them
        // could be one on glyph 0 that nothing gave a glyph (`doc/adr/2556`).
        if cid_notdef_glyph::is_cid_truetype(document, dict) {
            let not_given = Because::NotBuiltYet(cid_notdef_glyph::CID_NOT_GIVEN);
            let survey = survey.filter(|_| notdef.contains(&id)).ok_or(not_given)?;
            let used: Vec<_> = survey.fonts().filter(|used| used.id == Some(id)).collect();
            if used.is_empty() || used.iter().any(|used| !used.shown_complete) {
                return Err(not_given);
            }
            let shown = used
                .iter()
                .flat_map(|used| used.shown.keys().map(Vec::as_slice));
            let cid_0_shown = Because::NotBuiltYet(cid_notdef_glyph::CID_0_SHOWN);
            if cid_notdef_glyph::falls_back_to_cid_0(document, dict, shown.clone()) {
                return Err(cid_0_shown);
            }
            let Some(given) = cid_notdef_glyph::given(
                document,
                dict,
                shown.clone(),
                (spare, programs, &mut copied),
                MAX_COPIED_BYTES,
            ) else {
                return Err(if copied > MAX_COPIED_BYTES {
                    Because::NotBuiltYet(OVER_BUDGET)
                } else if cid_notdef_glyph::shows_cid_0(document, dict, shown) {
                    cid_0_shown
                } else {
                    not_given
                });
            };
            // Two `Type0` fonts over one descendant would each restate its one map.
            if found.descendants.contains_key(&given.descendant) {
                return Err(not_given);
            }
            found
                .descendants
                .insert(given.descendant, (given.descriptor, given.map));
            if let Some(fresh) = given.fresh {
                found.fresh_cids.insert(id, (fresh.cmap, fresh.cid));
                found.widths.insert(given.descendant, fresh.widths);
            }
            found.written.extend(given.written);
            found.notdef_given.insert(id);
            continue;
        }
        if cff_routed(
            document,
            (id, dict),
            (&notdef, survey),
            (spare, programs, &mut copied),
            &mut found,
        )? {
            continue;
        }
        if restate_one(
            document,
            (id, notdef.contains(&id)),
            dict,
            (spare, programs, &mut copied),
            substitutes,
            &mut found,
        )
        .is_none()
        {
            return Err(if copied > MAX_COPIED_BYTES {
                Because::NotBuiltYet(OVER_BUDGET)
            } else {
                refused
            });
        }
    }
    found.notdef_answered = notdef_too && notdef.is_subset(&found.notdef_given);
    Ok(found)
}

/// A font over a CFF or a simple font's Type 1 program, given its glyphs by writing the program
/// into `found` (`doc/adr/2620` for the Type 1 one), and `true`;
/// `false` for a font over any other program. A simple font over a bare CFF program and a `Type0`
/// font over a CID-keyed one reach this only by the `.notdef` rows: the first has its codes given
/// glyphs (`doc/adr/2578`), the second its CIDs, by writing the program's charset
/// (`doc/adr/2598`).
fn cff_routed(
    document: &Document,
    (id, dict): (ObjectId, &Dictionary),
    (notdef, survey): (&BTreeSet<ObjectId>, Option<&Survey>),
    held: (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    found: &mut CmapRestated,
) -> Result<bool, Because> {
    if cid_cff_notdef_glyph::is_cid_cff(document, dict) {
        cid_cff_given(document, (id, dict), (notdef, survey), held, found)?;
        return Ok(true);
    }
    if cff_notdef_glyph::is_simple_cff(document, dict)
        || cff_notdef_glyph::is_simple_type1(document, dict)
    {
        let survey = survey.filter(|_| notdef.contains(&id));
        cff_given(document, (id, dict), survey, held, found)?;
        return Ok(true);
    }
    Ok(false)
}

/// One `Type0` font over a CID-keyed CFF program named by the `.notdef` rows, given its glyphs
/// from the survey's shown strings, complete, into `found` (`doc/adr/2598`), together with every
/// other font of `notdef` over the same descendant, whose program and charset they share; a font
/// whose group an earlier one already answered is only counted.
fn cid_cff_given(
    document: &Document,
    (id, dict): (ObjectId, &Dictionary),
    (notdef, survey): (&BTreeSet<ObjectId>, Option<&Survey>),
    (spare, programs, copied): (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    found: &mut CmapRestated,
) -> Result<(), Because> {
    let refused = Because::NotBuiltYet(cid_cff_notdef_glyph::CID_CFF_NOT_GIVEN);
    let survey = survey.filter(|_| notdef.contains(&id)).ok_or(refused)?;
    let descendant = cid_cff_notdef_glyph::descendant_id(document, dict).ok_or(refused)?;
    if found.cid_cff.contains_key(&descendant) {
        found.notdef_given.insert(id);
        return Ok(());
    }
    let mut group: Vec<(ObjectId, Dictionary)> = Vec::new();
    for other in notdef {
        let resolved = document.get(*other);
        if let Some(other_dict) = dictionary_of(&resolved)
            && cid_cff_notdef_glyph::descendant_id(document, other_dict) == Some(descendant)
        {
            group.push((*other, other_dict.clone()));
        }
    }
    let mut fonts: Vec<(&Dictionary, Vec<&[u8]>)> = Vec::with_capacity(group.len());
    // The members whose code of CID 0 the encoding sends to a fresh CID (`doc/adr/2622`).
    let mut showing_zero: Vec<ObjectId> = Vec::new();
    for (member, member_dict) in &group {
        let used: Vec<_> = survey
            .fonts()
            .filter(|used| used.id == Some(*member))
            .collect();
        if used.is_empty() || used.iter().any(|used| !used.shown_complete) {
            return Err(refused);
        }
        let shown: Vec<&[u8]> = used
            .iter()
            .flat_map(|used| used.shown.keys().map(Vec::as_slice))
            .collect();
        if cid_cff_notdef_glyph::shows_own_cid_0(document, member_dict, shown.iter().copied()) {
            showing_zero.push(*member);
        }
        fonts.push((member_dict, shown));
    }
    let Some(given) = cid_cff_notdef_glyph::given(
        document,
        &fonts,
        (spare, programs, copied),
        MAX_COPIED_BYTES,
    ) else {
        return Err(if *copied > MAX_COPIED_BYTES {
            Because::NotBuiltYet(OVER_BUDGET)
        } else if showing_zero.is_empty() {
            refused
        } else {
            Because::NotBuiltYet(cid_cff_notdef_glyph::CID_CFF_0_SHOWN)
        });
    };
    found.cid_cff.insert(given.descendant, given.descriptor);
    if let Some(fresh) = given.fresh {
        for member in showing_zero {
            found.fresh_cids.insert(member, (fresh.cmap, fresh.cid));
        }
        found.widths.insert(given.descendant, fresh.widths);
    }
    found.written.extend(given.written);
    found.notdef_given.insert(id);
    Ok(())
}

/// One simple CFF font named by the `.notdef` rows, given its glyphs from the survey's shown
/// strings, complete, into `found` (`doc/adr/2578`).
fn cff_given(
    document: &Document,
    (id, dict): (ObjectId, &Dictionary),
    survey: Option<&Survey>,
    (spare, programs, copied): (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    found: &mut CmapRestated,
) -> Result<(), Because> {
    let refused = Because::NotBuiltYet(NOTDEF_NOT_GIVEN);
    let used: Vec<_> = survey
        .ok_or(refused)?
        .fonts()
        .filter(|used| used.id == Some(id))
        .collect();
    if used.is_empty() || used.iter().any(|used| !used.shown_complete) {
        return Err(refused);
    }
    let shown = used
        .iter()
        .flat_map(|used| used.shown.keys().map(Vec::as_slice));
    let Some(given) = cff_notdef_glyph::given(
        document,
        dict,
        shown,
        (spare, programs, copied),
        MAX_COPIED_BYTES,
    ) else {
        return Err(if *copied > MAX_COPIED_BYTES {
            Because::NotBuiltYet(OVER_BUDGET)
        } else {
            refused
        });
    };
    found.cff.insert(id, (given.descriptor, given.encoding));
    if cff_notdef_glyph::is_simple_type1(document, dict) {
        found.type1.insert(id);
    }
    found.written.extend(given.written);
    found.notdef_given.insert(id);
    Ok(())
}

/// The codes grouped by the advance the font gives them, in the program's units.
///
/// `None` where an advance falls outside what a 16-bit unit count holds.
fn codes_by_advance(font: &LoadedFont, codes: &[u8], units: f32) -> Option<BTreeMap<u16, Vec<u8>>> {
    let mut by_advance: BTreeMap<u16, Vec<u8>> = BTreeMap::new();
    for code in codes {
        let advance = (font.advance(Code::single_byte(*code)) * units).round();
        if !(0.0..=f32::from(u16::MAX)).contains(&advance) {
            return None;
        }
        // Within the range just checked, so the conversion neither truncates nor wraps.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the advance was checked to lie in 0..=u16::MAX and is a whole number"
        )]
        let advance = advance as u16;
        by_advance.entry(advance).or_default().push(*code);
    }
    Some(by_advance)
}

/// One font: the new descriptor and program, proved, and the objects that hold them.
fn restate_one(
    document: &Document,
    (id, notdef): (ObjectId, bool),
    dict: &Dictionary,
    (spare, programs, copied): (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    substitutes: Option<&Substitutes>,
    found: &mut CmapRestated,
) -> Option<()> {
    if document.get_key(dict, "Subtype").as_name()?.as_bytes() != b"TrueType" {
        return None;
    }
    // A font the substitution makes a `Type1` one over a CFF face is no longer a TrueType font in
    // the output, and §9.6.5.4's route is not the one it is read by (`doc/adr/2744`).
    if substitutes
        .and_then(|substitutes| substitutes.described.get(&id))
        .and_then(|entries| entries.get("Subtype"))
        .and_then(Object::as_name)
        .is_some_and(|subtype| subtype.as_bytes() != b"TrueType")
    {
        return Some(());
    }
    let source = dict;
    let written = as_written(document, (id, dict), programs, substitutes)?;
    let (dict, descriptor, stream) = (&written.dict, &written.descriptor, &written.program);
    let (before, reached, on_notdef) = codes_drawn(document, source, &written, notdef)?;
    let program = document.decoded_stream_data(stream)?;
    *copied = copied.saturating_add(program.len());
    if *copied > MAX_COPIED_BYTES {
        return None;
    }
    // A font named by a `.notdef` row with no code on glyph 0 is restated as the other rows ask
    // and given no glyph: its shown code reaches no glyph at all, and a copy of `.notdef` would
    // draw a mark the reader did not. It is left out of `notdef_given`, so its row stays refused.
    let mut tables = Vec::new();
    let mut map = reached.clone();
    let mut given: BTreeMap<u8, u16> = BTreeMap::new();
    if !on_notdef.is_empty() {
        // One glyph for each width the dictionary gives these codes, in the program's units,
        // because the program's advance for a glyph is held to the width of every code that
        // reaches it (ISO 19005-2 section 6.2.11.5).
        let units = f32::from(notdef_glyph::units_per_em(&program)?);
        let by_advance = codes_by_advance(&before, &on_notdef, units)?;
        let advances: Vec<u16> = by_advance.keys().copied().collect();
        let glyphs = notdef_glyph::with_notdef_copied(&program, &advances)?;
        for (codes, glyph) in by_advance.values().zip(&glyphs.glyphs) {
            for code in codes {
                map.insert(*code, *glyph);
                given.insert(*code, *glyph);
            }
        }
        tables.extend(glyphs.tables);
    }
    tables.push((*b"cmap", cmap_table(&map)?));
    let restated = pdf_font::embedding::with_tables(&program, &tables)?;
    // Each new glyph is proved to be glyph 0's own outline in the program as written.
    let notdef_outline = notdef_glyph::outline(&program, NOTDEF);
    if given
        .values()
        .any(|glyph| notdef_glyph::outline(&restated, *glyph) != notdef_outline)
    {
        return None;
    }
    let new_program = super::program_stream(Some(&stream.dict), &restated, true)?;

    let mut new_descriptor = descriptor.clone();
    let flags = document
        .get_key(descriptor, "Flags")
        .as_integer()
        .unwrap_or(0);
    new_descriptor.insert(
        Name::new(&b"Flags"[..]),
        Object::Integer((flags | SYMBOLIC) & !NONSYMBOLIC),
    );

    // The proof reads the font as the conversion will write it, from objects held in memory.
    let mut proof_descriptor = new_descriptor.clone();
    proof_descriptor.insert(Name::new(&b"FontFile2"[..]), new_program.clone());
    let mut candidate = dict.clone();
    candidate.remove("Encoding");
    candidate.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Dictionary(proof_descriptor),
    );
    let after = LoadedFont::load(document, &candidate, "").ok()?;
    if after.is_substituted() || !on_the_same_glyphs(&reached, &given, &after) {
        return None;
    }
    // And the program's advance for each is the width the dictionary gives its code, which is
    // what the reader advanced by (ISO 19005-2 section 6.2.11.5's thousandth of an em).
    if given.keys().any(|code| {
        let code = Code::single_byte(*code);
        after
            .program_advance(code)
            .is_none_or(|program| (program - after.advance(code)).abs() > super::CONSISTENT)
    }) {
        return None;
    }

    let program_id = spare.take(document)?;
    let descriptor_id = spare.take(document)?;
    new_descriptor.insert(Name::new(&b"FontFile2"[..]), Object::Reference(program_id));
    found.written.insert(program_id, new_program);
    found
        .written
        .insert(descriptor_id, Object::Dictionary(new_descriptor));
    found.descriptors.insert(id, descriptor_id);
    if !given.is_empty() {
        found.notdef_given.insert(id);
    }
    Some(())
}

/// One font as this conversion writes it before its `cmap` is restated: the dictionary with the
/// entries the substitution adds, its descriptor with the program the substitution embeds, and
/// that program's stream.
struct AsWritten {
    /// The font dictionary, with the entries [`Substitutes`] adds to it.
    dict: Dictionary,
    /// The descriptor the dictionary names, with the program entry the substitution writes.
    descriptor: Dictionary,
    /// The `/FontFile2` stream the descriptor names in the output.
    program: std::sync::Arc<pdf_syntax::Stream>,
    /// The face the substitution embeds as that program, as it writes it, where the program is
    /// one this conversion embeds rather than the file's own.
    embedded_face: Option<std::sync::Arc<pdf_syntax::Stream>>,
}

/// The font the conversion writes for `dict`, where it carries a `/FontFile2` program.
///
/// **A font the file did not embed is restated over the face the conversion embeds for it**
/// (`doc/adr/2744`). [`Substitutes`] writes a program into the font's descriptor, and the output is
/// then a TrueType font with a program whose `cmap` is held to ISO 19005-2 section 6.2.11.6 like
/// any other, so the restatement is made over that program. The descriptor is read where the file or the substitution writes it; the restated one is written
/// as an object of its own either way, which ISO 32000-2 §9.6.3 and Table 109 require.
fn as_written(
    document: &Document,
    (id, dict): (ObjectId, &Dictionary),
    programs: &BTreeMap<ObjectId, Object>,
    substitutes: Option<&Substitutes>,
) -> Option<AsWritten> {
    let mut dict = dict.clone();
    if let Some(entries) = substitutes.and_then(|substitutes| substitutes.described.get(&id)) {
        for (key, value) in entries.iter() {
            dict.insert(key.clone(), value.clone());
        }
    }
    let named = dict.get("FontDescriptor")?;
    let added = named
        .as_reference()
        .and_then(|at| substitutes?.written.get(&at));
    let resolved = document.resolve(named);
    let mut descriptor = dictionary_of(added.unwrap_or(&resolved))?.clone();
    if let Some(embedding) = named.as_reference().and_then(|at| substitutes?.at.get(&at)) {
        embedding.write_into(&mut descriptor);
    }
    let program_at = descriptor.get("FontFile2")?.as_reference()?;
    let embedded_face =
        match substitutes.and_then(|substitutes| substitutes.written.get(&program_at)) {
            Some(Object::Stream(face)) => Some(std::sync::Arc::clone(face)),
            Some(_) => return None,
            None => None,
        };
    // The program the conversion writes for this font is the one it writes for the original, so
    // a restatement of its advances comes along into the copy.
    let Object::Stream(program) = programs
        .get(&program_at)
        .or_else(|| substitutes?.written.get(&program_at))
        .cloned()
        .unwrap_or_else(|| document.get(program_at))
    else {
        return None;
    };
    Some(AsWritten {
        dict,
        descriptor,
        program,
        embedded_face,
    })
}

/// The font a reader of the output loads before its `cmap` is restated, the glyph each code is
/// restated to, and the codes on glyph 0 that a `.notdef` row asks a glyph for.
///
/// A font the file did not embed is drawn from a face this program chose, and the output
/// draws the face this program ships for it, which the substitution embeds. The codes are
/// restated from that face's own code table, the one this reader draws the face by and the
/// substitution proves its embedding with, each glyph held to having the same outline in the
/// program embedded, which may be the face with its advances restated or glyphs added; a code
/// the table leaves without a glyph keeps the one the embedded program gives it, which is the
/// substitution's own (`doc/adr/2744`).
fn codes_drawn(
    document: &Document,
    source: &Dictionary,
    written: &AsWritten,
    notdef: bool,
) -> Option<(LoadedFont, BTreeMap<u8, u16>, Vec<u8>)> {
    let Some(embedded) = &written.embedded_face else {
        let before = loaded_as_stated(document, source)?;
        let reached = codes_reached(&before);
        let on_notdef = if notdef {
            codes_on_notdef(&before)
        } else {
            Vec::new()
        };
        return Some((before, reached, on_notdef));
    };
    let drawn = LoadedFont::load(document, source, "").ok()?;
    if !drawn.is_substituted() {
        return None;
    }
    let face = document.decoded_stream_data(embedded)?;
    let shipped = pdf_font::standard::shipped_face(document, source, "").ok()?;
    let as_embedded = embedded_as_written(document, written, embedded)?;
    let mut reached = codes_reached(&as_embedded);
    for byte in 0..=u8::MAX {
        if let Some(glyph) = shipped.glyph(byte).filter(|glyph| *glyph != NOTDEF) {
            let outline = notdef_glyph::outline(shipped.program, glyph);
            if outline.is_none() || notdef_glyph::outline(&face, glyph) != outline {
                return None;
            }
            reached.insert(byte, glyph);
        }
    }
    let on_notdef = if notdef {
        codes_on_notdef(&as_embedded)
            .into_iter()
            .filter(|byte| !reached.contains_key(byte))
            .collect()
    } else {
        Vec::new()
    };
    Some((as_embedded, reached, on_notdef))
}

/// The font as the substitution writes it, its program held in memory, as a reader of the
/// output would load it before the `cmap` is restated.
fn embedded_as_written(
    document: &Document,
    written: &AsWritten,
    embedded: &std::sync::Arc<pdf_syntax::Stream>,
) -> Option<LoadedFont> {
    let mut descriptor = written.descriptor.clone();
    descriptor.insert(
        Name::new(&b"FontFile2"[..]),
        Object::Stream(std::sync::Arc::clone(embedded)),
    );
    let mut dict = written.dict.clone();
    dict.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Dictionary(descriptor),
    );
    LoadedFont::load(document, &dict, "")
        .ok()
        .filter(|font| !font.is_substituted())
}

/// The glyph each of a simple font's 256 codes reached, for the codes that reached one.
///
/// A code on glyph 0 is left out. A `cmap` lookup that answers glyph 0 answers that the character
/// has no glyph (the Manual's `.notdef`), so no subtable states the route to it, and ISO 19005's
/// `.notdef` rule is the requirement that owns such a code (`fonts/no-notdef-glyph-shown`).
fn codes_reached(font: &LoadedFont) -> BTreeMap<u8, u16> {
    (0..=u8::MAX)
        .filter_map(|byte| {
            font.glyph_index(Code::single_byte(byte))
                .filter(|glyph| *glyph != NOTDEF)
                .map(|glyph| (byte, glyph))
        })
        .collect()
}

/// The codes of a simple font that reached glyph 0, which this reader drew as the program's
/// `.notdef` outline.
///
/// A code that reached no glyph at all is not one of them: the reader drew nothing for it.
fn codes_on_notdef(font: &LoadedFont) -> Vec<u8> {
    (0..=u8::MAX)
        .filter(|byte| font.glyph_index(Code::single_byte(*byte)) == Some(NOTDEF))
        .collect()
}

/// Glyph 0, which every TrueType program keeps for a character it has no glyph for.
const NOTDEF: u16 = 0;

/// Whether the font as written reaches, for every code, the glyph the font as read reached, by
/// its program's own subtables and not by a mapping of the reader's choosing.
///
/// A code the font as read left on glyph 0 is held to no more than still not reaching another
/// glyph, unless it is one of `given`, which must reach the glyph given it by the program's own
/// subtables.
fn on_the_same_glyphs(
    reached: &BTreeMap<u8, u16>,
    given: &BTreeMap<u8, u16>,
    after: &LoadedFont,
) -> bool {
    (0..=u8::MAX).all(|byte| {
        let code = Code::single_byte(byte);
        if let Some(glyph) = given.get(&byte) {
            return after.glyph_index(code) == Some(*glyph)
                && !after.glyph_needed_a_reader_chosen_mapping(code);
        }
        match reached.get(&byte) {
            Some(glyph) => {
                after.glyph_index(code) == Some(*glyph)
                    && !after.glyph_needed_a_reader_chosen_mapping(code)
            }
            None => after.glyph_index(code).is_none_or(|glyph| glyph == NOTDEF),
        }
    })
}

/// A `cmap` table of the Macintosh Roman (1, 0) subtable, which maps each code to its glyph, and
/// the Microsoft Symbol (3, 0) one, which maps the code with 0xF000 added.
///
/// The Manual's table opens with a version and a count, then one record a subtable (platform,
/// encoding, offset from the table's start) in ascending platform and encoding order.
fn cmap_table(reached: &BTreeMap<u8, u16>) -> Option<Vec<u8>> {
    if reached.is_empty() {
        return None;
    }
    let roman = format_6(reached);
    let symbol = format_4(
        &reached
            .iter()
            .map(|(byte, glyph)| (SYMBOL_PREFIX | u16::from(*byte), *glyph))
            .collect(),
    )?;
    let first = FIRST_SUBTABLE;
    let second = first.checked_add(u32::try_from(roman.len()).ok()?)?;
    let mut out = Vec::with_capacity(
        roman
            .len()
            .saturating_add(symbol.len())
            .saturating_add(usize::try_from(FIRST_SUBTABLE).ok()?),
    );
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&2u16.to_be_bytes());
    for (platform, encoding, offset) in [(1u16, 0u16, first), (3, 0, second)] {
        out.extend_from_slice(&platform.to_be_bytes());
        out.extend_from_slice(&encoding.to_be_bytes());
        out.extend_from_slice(&offset.to_be_bytes());
    }
    out.extend_from_slice(&roman);
    out.extend_from_slice(&symbol);
    Some(out)
}

/// Where the first subtable starts: after the table's version and count, and its two records of
/// platform, encoding and offset.
const FIRST_SUBTABLE: u32 = 4 + 2 * 8;

/// Format 6, the trimmed table: the 256 codes from 0, a glyph index each, 0 where none.
fn format_6(reached: &BTreeMap<u8, u16>) -> Vec<u8> {
    let mut out = Vec::with_capacity(10 + 2 * 256);
    out.extend_from_slice(&6u16.to_be_bytes());
    out.extend_from_slice(&(10u16 + 2 * 256).to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&256u16.to_be_bytes());
    for byte in 0..=u8::MAX {
        out.extend_from_slice(&reached.get(&byte).copied().unwrap_or(0).to_be_bytes());
    }
    out
}

/// Format 4, segment mapping to delta values, with one segment for each run of consecutive
/// characters, whose glyph indices are listed in the glyph index array, and the closing segment
/// 0xFFFF the format requires.
pub(in crate::archive) fn format_4(entries: &BTreeMap<u16, u16>) -> Option<Vec<u8>> {
    let mut runs: Vec<(u16, u16, usize)> = Vec::new();
    let mut glyphs: Vec<u16> = Vec::new();
    for (&character, &glyph) in entries {
        match runs.last_mut() {
            Some((_, end, _)) if end.checked_add(1) == Some(character) => *end = character,
            _ => runs.push((character, character, glyphs.len())),
        }
        glyphs.push(glyph);
    }
    let segments = runs.len().checked_add(1)?;
    let segment_count = u16::try_from(segments).ok()?;
    let doubled = segment_count.checked_mul(2)?;
    let selector = u16::try_from(segments.ilog2()).ok()?;
    let search_range = 2u16.checked_mul(1u16.checked_shl(u32::from(selector))?)?;
    let length = u16::try_from(
        16usize
            .checked_add(segments.checked_mul(8)?)?
            .checked_add(glyphs.len().checked_mul(2)?)?,
    )
    .ok()?;
    let range_shift = doubled.checked_sub(search_range)?;
    let mut out = Vec::with_capacity(usize::from(length));
    for word in [4, length, 0, doubled, search_range, selector, range_shift] {
        out.extend_from_slice(&word.to_be_bytes());
    }
    for (_, end, _) in &runs {
        out.extend_from_slice(&end.to_be_bytes());
    }
    out.extend_from_slice(&0xFFFFu16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    for (start, _, _) in &runs {
        out.extend_from_slice(&start.to_be_bytes());
    }
    out.extend_from_slice(&0xFFFFu16.to_be_bytes());
    for _ in &runs {
        out.extend_from_slice(&0u16.to_be_bytes());
    }
    out.extend_from_slice(&1u16.to_be_bytes());
    // An idRangeOffset counts from its own position to the glyph index array, so it is the
    // distance to the end of the offsets plus the run's place in the array.
    for (index, (_, _, first)) in runs.iter().enumerate() {
        let to_the_array = u16::try_from(segments.checked_sub(index)?)
            .ok()?
            .checked_mul(2)?;
        let into_the_array = u16::try_from(*first).ok()?.checked_mul(2)?;
        out.extend_from_slice(&to_the_array.checked_add(into_the_array)?.to_be_bytes());
    }
    out.extend_from_slice(&0u16.to_be_bytes());
    for glyph in &glyphs {
        out.extend_from_slice(&glyph.to_be_bytes());
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{cmap_table, format_4};
    use std::collections::BTreeMap;

    /// The glyph a format 4 subtable gives a character, read the way the Manual defines it.
    fn lookup(table: &[u8], character: u16) -> Option<u16> {
        let word = |at: usize| u16::from_be_bytes([table[at], table[at.saturating_add(1)]]);
        let segments = usize::from(word(6) / 2);
        let after = |base: usize, count: usize| base.saturating_add(count.saturating_mul(2));
        let ends = 14;
        let starts = after(ends, segments).saturating_add(2);
        let deltas = after(starts, segments);
        let offsets = after(deltas, segments);
        (0..segments).find_map(|index| {
            let start = word(after(starts, index));
            if word(after(ends, index)) < character || start > character {
                return None;
            }
            let range = word(after(offsets, index));
            let glyph = if range == 0 {
                character
            } else {
                word(
                    after(offsets, index)
                        .saturating_add(usize::from(range))
                        .saturating_add(
                            usize::from(character.saturating_sub(start)).saturating_mul(2),
                        ),
                )
            };
            Some(glyph.wrapping_add(word(after(deltas, index))))
        })
    }

    #[test]
    fn a_format_4_subtable_gives_back_each_character_s_glyph() {
        let entries: BTreeMap<u16, u16> =
            [(0xF020, 3), (0xF021, 900), (0xF041, 7), (0xF0FF, 65535)]
                .into_iter()
                .collect();
        let table = format_4(&entries).expect("the entries fit");
        assert_eq!(
            usize::from(u16::from_be_bytes([table[2], table[3]])),
            table.len()
        );
        for (character, glyph) in &entries {
            assert_eq!(lookup(&table, *character), Some(*glyph), "{character:#x}");
        }
        assert_eq!(lookup(&table, 0xF022), None);
    }

    #[test]
    fn the_table_holds_the_two_subtables_in_the_order_the_manual_sorts_them() {
        let reached: BTreeMap<u8, u16> = [(65, 36), (66, 37)].into_iter().collect();
        let table = cmap_table(&reached).expect("two codes fit");
        assert_eq!(&table[..4], &[0, 0, 0, 2]);
        assert_eq!(
            &table[4..8],
            &[0, 1, 0, 0],
            "the Macintosh Roman record first"
        );
        assert_eq!(
            &table[12..16],
            &[0, 3, 0, 0],
            "then the Microsoft Symbol one"
        );
        assert!(
            cmap_table(&BTreeMap::new()).is_none(),
            "no code reached, no table"
        );
    }
}
