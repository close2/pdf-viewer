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

use std::collections::BTreeMap;

use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use pdf_archive::Outcome;

use super::super::decision::Because;
use super::super::prepare::Spare;
use super::super::sites::{dictionary_of, loaded_as_stated, objects_named};

/// The requirements a font's restated `cmap` answers.
pub(in crate::archive) const ANSWERED: [&str; 4] = [
    "fonts/non-symbolic-truetype-differences-are-listed-names",
    "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
    "fonts/non-symbolic-truetype-program-maps-every-code",
    "fonts/symbolic-truetype-program-has-a-usable-cmap",
];

/// The three rows whose own rewrites name a font's encoding, which this restatement answers for
/// a font whose own preparation refused (`super::super::decision::Answer::OrCmapRestated`).
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
     proof, or the checker named only the first places of a requirement this file fails in more \
     places than it keeps, so the fonts past them are not known";

/// The fonts whose program is restated, and the objects that restate them.
#[derive(Debug, Default)]
pub(in crate::archive) struct CmapRestated {
    /// The descriptor each font dictionary now names, by font dictionary.
    pub(in crate::archive) descriptors: BTreeMap<ObjectId, ObjectId>,
    /// The descriptors and programs this conversion adds.
    written: BTreeMap<ObjectId, Object>,
}

impl CmapRestated {
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
        if self.descriptors.is_empty() {
            return None;
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
        Some(format!(
            "ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6: the program of {} \
             rendered TrueType font(s) now carries the Macintosh Roman (1, 0) and Microsoft \
             Symbol (3, 0) cmap subtables mapping each code to the glyph this reader drew, the \
             font is the symbolic font ISO 32000-2 section 9.6.5.4 describes and states no \
             Encoding, so no mark moved and no content stream changed: fonts, in the source's object numbering, {}{tail}",
            self.descriptors.len(),
            named.join(", ")
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

/// Every TrueType font the four `cmap` requirements name, restated, and with `standard_route_too`
/// the fonts the three encoding requirements name as well.
///
/// **The copies are bounded twice.** The checker keeps the first places of each requirement and
/// counts the rest (`pdf_archive`'s `Findings`), so the fonts named are at most the kept places
/// of the requirements asked, and the programs they copy together may not pass
/// [`MAX_COPIED_BYTES`], decoded. A requirement whose places were cut short leaves fonts unnamed
/// that the output would then still fail on, so it is refused here with the sentence that says
/// so instead of by the conversion's last check.
///
/// ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6.
pub(in crate::archive) fn restated(
    document: &Document,
    input: &pdf_archive::Report,
    spare: &mut Spare,
    programs: &BTreeMap<ObjectId, Object>,
    standard_route_too: bool,
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
        .copied()
        .collect();
    let (named, elsewhere) = objects_named(input, &rows);
    let cut_short = input.failures().any(|judgement| {
        rows.contains(&judgement.id)
            && matches!(&judgement.outcome,
                Outcome::Failed { places, total } if *total > places.len())
    });
    if elsewhere || cut_short {
        return Err(refused);
    }
    let mut found = CmapRestated::default();
    let mut copied = 0_usize;
    for id in named {
        let resolved = document.get(id);
        let dict = dictionary_of(&resolved).ok_or(refused)?;
        if restate_one(
            document,
            id,
            dict,
            (spare, programs, &mut copied),
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
    Ok(found)
}

/// One font: the new descriptor and program, proved, and the objects that hold them.
fn restate_one(
    document: &Document,
    id: ObjectId,
    dict: &Dictionary,
    (spare, programs, copied): (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    found: &mut CmapRestated,
) -> Option<()> {
    if document.get_key(dict, "Subtype").as_name()?.as_bytes() != b"TrueType" {
        return None;
    }
    let before = loaded_as_stated(document, dict)?;
    let reached = codes_reached(&before);
    let descriptor_at = dict.get("FontDescriptor")?.as_reference()?;
    let resolved = document.get(descriptor_at);
    let descriptor = dictionary_of(&resolved)?;
    let program_at = descriptor.get("FontFile2")?.as_reference()?;
    // The program the conversion writes for this font is the one it writes for the original, so
    // a restatement of its advances comes along into the copy.
    let Object::Stream(stream) = programs
        .get(&program_at)
        .cloned()
        .unwrap_or_else(|| document.get(program_at))
    else {
        return None;
    };
    let program = document.decoded_stream_data(&stream)?;
    *copied = copied.saturating_add(program.len());
    if *copied > MAX_COPIED_BYTES {
        return None;
    }
    let restated =
        pdf_font::embedding::with_tables(&program, &[(*b"cmap", cmap_table(&reached)?)])?;
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
    if after.is_substituted() || !on_the_same_glyphs(&reached, &after) {
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
    Some(())
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

/// Glyph 0, which every TrueType program keeps for a character it has no glyph for.
const NOTDEF: u16 = 0;

/// Whether the font as written reaches, for every code, the glyph the font as read reached, by
/// its program's own subtables and not by a mapping of the reader's choosing.
///
/// A code the font as read left on glyph 0 is held to no more than still not reaching another
/// glyph.
fn on_the_same_glyphs(reached: &BTreeMap<u8, u16>, after: &LoadedFont) -> bool {
    (0..=u8::MAX).all(|byte| {
        let code = Code::single_byte(byte);
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
fn format_4(entries: &BTreeMap<u16, u16>) -> Option<Vec<u8>> {
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
