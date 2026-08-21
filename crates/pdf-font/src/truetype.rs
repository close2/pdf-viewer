//! ISO 32000-2 §9.6.5.4: a simple font's character codes through a `TrueType` or `OpenType`
//! program's `cmap`.
//!
//! The subclause's shape is easy to lose and [`truetype_code_table`] carries it: a `cmap` is
//! not indexed by character code, so each of the rules turns the code into whatever the
//! subtable it names *is* indexed by. §9.6.5.2's other half — a code reaching a glyph by
//! *name* — is [`crate::name_keyed`]'s, and the tables the PDF encoding itself supplies are
//! [`crate::glyph_names`]'s.
//!
//! The container these tables sit in is [`crate::sfnt`]'s, and this module's tests cover both:
//! they are written against fonts assembled table by table, and the assembler serves the
//! repairs and the encoding rules alike.

use std::borrow::Cow;
use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object};
use skrifa::raw::TableProvider;
use skrifa::raw::tables::cmap::{Cmap, CmapSubtable, PlatformId};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::cff::CodeToGlyph;
use crate::encoding;
use crate::glyph_names::{GlyphNames, encoding_names, no_names};
use crate::loading::{CodeTable, FontError};
use crate::name_keyed::NameKeyed;

/// The three `cmap` subtables ISO 32000-2 §9.6.5.4 distinguishes.
///
/// It names them by their platform and encoding IDs, and the whole of its algorithm turns
/// on which of them a font carries — so they are found once, by ID, rather than through a
/// "best subtable" heuristic. `skrifa`'s own `Charmap` picks the most comprehensive
/// *Unicode* subtable, which is the right choice for laying out text and the wrong one
/// here: a (1, 0) Macintosh subtable is not a Unicode mapping, so `Charmap` does not
/// consider it at all, and a font whose only subtable is that one maps nothing.
///
/// That is not a corner case. `issue20504.pdf` embeds four `TrueType` subsets and every one
/// of them carries a single (1, 0) format 6 subtable — which is exactly what §9.6.5.4's
/// third guideline tells a producer to emit.
struct Subtables<'a> {
    /// (3, 0), Microsoft Symbol: codes are looked up as they are written.
    symbol: Option<CmapSubtable<'a>>,
    /// (3, 1), Microsoft Unicode: codes reach it as characters, through glyph names.
    unicode: Option<CmapSubtable<'a>>,
    /// (1, 0), Macintosh Roman: codes reach it as Mac OS Roman codes.
    macintosh: Option<CmapSubtable<'a>>,
    /// The whole table, for the last-resort mapping that asks every subtable in turn.
    ///
    /// A font may carry a subtable §9.6.5.4 names none of — `issue5501.pdf`'s only one is
    /// (0, 0), Unicode 1.0 — and there is nowhere else left to ask by the time that
    /// matters.
    all: Option<Cmap<'a>>,
}

impl<'a> Subtables<'a> {
    fn read(font: &FontRef<'a>) -> Self {
        let mut found = Self {
            symbol: None,
            unicode: None,
            macintosh: None,
            all: None,
        };
        let Ok(cmap) = font.cmap() else {
            return found;
        };
        found.all = Some(cmap.clone());
        for record in cmap.encoding_records() {
            let Ok(subtable) = record.subtable(cmap.offset_data()) else {
                continue;
            };
            // The first subtable of each kind wins; a font listing two is malformed, and
            // taking the earlier one at least makes the choice reproducible.
            let slot = match (record.platform_id(), record.encoding_id()) {
                (PlatformId::Windows, 0) => &mut found.symbol,
                (PlatformId::Windows, 1) => &mut found.unicode,
                (PlatformId::Macintosh, 0) => &mut found.macintosh,
                _ => continue,
            };
            if slot.is_none() {
                *slot = Some(subtable);
            }
        }
        found
    }
}

/// Resolves a simple font's character codes to glyphs in a `TrueType` or `OpenType` program.
///
/// This is ISO 32000-2 §9.6.5.4, whose shape is easy to lose: a `cmap` is *not* indexed by
/// character code. Each of its subtables is indexed by something else — a Unicode
/// character, a Mac OS Roman code, a two-byte symbol code — and the subclause is a set of
/// rules for turning a PDF character code into whichever of those the font happens to
/// carry. Handing the code straight to the font's Unicode subtable is right only by
/// coincidence, for ASCII, in a font that has one.
///
/// The rules, in the order the subclause gives them:
///
/// - **The font's own codes.** When the font descriptor sets the symbolic flag, or the
///   dictionary has no `/Encoding` at all, the PDF encoding says nothing: a (3, 0) subtable
///   is addressed by the code with the high byte of its range prepended, and failing that a
///   (1, 0) subtable is addressed by the single byte.
/// - **Through a glyph name.** Otherwise the code selects a glyph *name* — from the base
///   encoding, updated by `/Differences`, with anything still undefined filled from
///   `StandardEncoding` — and the name is carried to a (3, 1) subtable through the Adobe
///   Glyph List, or to a (1, 0) subtable through Mac OS Roman.
/// - **The `post` table.** "In any of these cases, if the glyph name cannot be mapped as
///   specified, the glyph name shall be looked up in the font program's `post` table."
///   This is what reaches a subsetter's `/gid2436`, which no encoding and no character set
///   knows but the font itself may name.
///
/// # The two tiers below the specification's own, and why they are last
///
/// §9.6.5.4 closes with "if a character cannot be mapped in any of the ways described
/// previously, a PDF processor may supply a mapping of its choosing". Two are supplied, and
/// each is narrower than the code it replaced.
///
/// The first offers the code to every subtable the font has, in the font's own order,
/// which is what this crate did for every font before the algorithm above existed. It still
/// earns its place twice over: a symbolic font carrying only a (3, 1) subtable — common,
/// and contrary to the guidelines — reaches no rule above and its codes really are ASCII;
/// and `issue5501.pdf`'s subset carries its byte-to-glyph map in a (0, 0) subtable, which
/// §9.6.5.4 does not mention at all and which is nonetheless the only correct answer for
/// that font.
///
/// The second treats the code as a glyph index, and applies **only to a font with no
/// readable `cmap` at all**. That restriction is the point: the old code fell through to it
/// per *code*, so a font with a perfectly good `cmap` that simply did not cover a code drew
/// glyph number `code` instead — a wrong glyph, confidently, in place of nothing. A
/// document using a simple font is required to embed a `cmap` (§9.9.1), so a program
/// without one is malformed, and a subset really is often ordered by code.
pub(crate) fn truetype_code_table(
    document: &Document,
    dict: &Dictionary,
    descriptor: Option<&Dictionary>,
    data: &[u8],
    name: &str,
) -> Result<(CodeTable, GlyphNames), FontError> {
    let font = FontRef::new(data).map_err(|e| FontError::Malformed {
        name: name.to_owned(),
        detail: e.to_string(),
    })?;
    let subtables = Subtables::read(&font);
    // §9.6.5.4's last resort is "the font program's `post` table (if one is present)" — and a
    // CFF-based OpenType keeps its glyph names somewhere else. `issue215.pdf` embeds an `OTTO`
    // whose `post` is version 3.0, which by definition holds no names at all, while its `CFF `
    // charset names every one of them; §9.6.2.1's NOTE 1 is the sentence that makes those the
    // same structure, so the charset is read here as what the clause asks for. Built once per
    // font rather than per code, because it inverts the whole charset.
    let charset = font
        .table_data(skrifa::Tag::new(b"CFF "))
        .and_then(|table| CodeToGlyph::read(table.as_bytes()).ok())
        .and_then(|read| match read {
            CodeToGlyph::Named(named) => Some(named),
            CodeToGlyph::Keyed { .. } => None,
        });
    let symbolic = descriptor.is_some_and(|d| is_symbolic(document, d));
    // §9.6.5.4 fills undefined entries from StandardEncoding, which `encoding_names` does
    // by starting there — but only for a font whose names are Latin at all. A symbolic
    // font's are not, and it takes the first route below rather than this one.
    let names = match encoding_names(document, dict, name, None, !symbolic) {
        Ok(names) => names,
        // §9.6.5.4 again: when the symbolic flag is set the `/Encoding` entry "is ignored".
        // So an entry naming an encoding this crate has no table for — `issue5701.pdf`
        // writes `/Encoding /Identity-H` on a simple `TrueType` font, which is not a base
        // encoding at all — is not a font this crate cannot read. It is an entry the
        // specification tells us not to read.
        Err(FontError::UnsupportedEncoding { .. }) if symbolic => no_names(),
        Err(other) => return Err(other),
    };

    let mut table: CodeTable = [None; 256];

    // "When the font has no Encoding entry, or the font descriptor's Symbolic flag is set
    // (in which case the Encoding entry is ignored)".
    let unencoded = matches!(document.get_key(dict, "Encoding"), Object::Null);
    if symbolic || unencoded {
        for (code, slot) in table.iter_mut().enumerate() {
            let Ok(code) = u32::try_from(code) else {
                continue;
            };
            *slot = symbol_glyph(&subtables, code);
        }
    }

    for (code, slot) in table.iter_mut().enumerate() {
        if slot.is_some() {
            continue;
        }
        let glyph_name = names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty());
        *slot = glyph_name
            .and_then(|glyph_name| named_glyph(&font, &subtables, charset.as_ref(), glyph_name));
    }

    // The two tiers the specification leaves to the processor; see the note above.
    for (code, slot) in table.iter_mut().enumerate() {
        if slot.is_some() {
            continue;
        }
        let Ok(code) = u32::try_from(code) else {
            continue;
        };
        *slot = as_character(&subtables, code).or_else(|| {
            subtables
                .all
                .is_none()
                .then(|| u16::try_from(code).ok())
                .flatten()
        });
    }

    if table.iter().all(Option::is_none) {
        // The font loaded and the encoding resolved, and between them they addressed not
        // one glyph. Reporting beats rendering an entirely blank page in silence.
        return Err(FontError::Malformed {
            name: name.to_owned(),
            detail: "no character code maps to a glyph".to_owned(),
        });
    }

    Ok((table, names))
}

/// Every glyph a font's Unicode subtable names, keyed by glyph.
///
/// The first character wins where several map to one glyph, which happens in every subset that
/// unifies a letter with its presentation form. Deterministic because `Charmap::mappings`
/// walks the subtable in code order.
pub(crate) fn invert_charmap(font: &FontRef<'_>) -> BTreeMap<u16, char> {
    let mut out = BTreeMap::new();
    for (code, glyph) in font.charmap().mappings() {
        let (Some(character), Ok(glyph)) = (char::from_u32(code), u16::try_from(glyph.to_u32()))
        else {
            continue;
        };
        out.entry(glyph).or_insert(character);
    }
    out
}

/// A code's glyph through the font's own codes: the (3, 0) subtable, then the (1, 0) one.
///
/// §9.6.5.4 says a (3, 0) subtable's codes lie in one of four ranges — `0x0000`, `0xF000`,
/// `0xF100` or `0xF200` — and that each byte from the string is prepended with the high
/// byte of *the* range the font uses. Which range that is is not recorded anywhere in the
/// font, so the four are tried in the order the subclause lists them. A subtable holding
/// two of them at once would be malformed; one holding none maps nothing, which is the
/// answer either way.
fn symbol_glyph(subtables: &Subtables<'_>, code: u32) -> Option<u16> {
    if let Some(symbol) = subtables.symbol.as_ref() {
        return [0x0000, 0xF000, 0xF100, 0xF200]
            .into_iter()
            .find_map(|high: u32| symbol.map_codepoint(high.saturating_add(code)))
            .and_then(narrow_glyph);
    }
    subtables
        .macintosh
        .as_ref()?
        .map_codepoint(code)
        .and_then(narrow_glyph)
}

/// A glyph name's glyph: through the (3, 1) subtable, the (1, 0) subtable, or `post`.
fn named_glyph(
    font: &FontRef<'_>,
    subtables: &Subtables<'_>,
    charset: Option<&NameKeyed>,
    glyph_name: &str,
) -> Option<u16> {
    let through_character = |name: &str| {
        if let Some(unicode) = subtables.unicode.as_ref() {
            read_fonts::ps::agl::name_to_char(name)
                .and_then(|character| unicode.map_codepoint(character))
        } else if let Some(macintosh) = subtables.macintosh.as_ref() {
            encoding::mac_os_roman_code(name)
                .and_then(|code| macintosh.map_codepoint(u32::from(code)))
        } else {
            None
        }
    };

    // §9.6.5.4 sends the glyph name "to a Unicode value by consulting the Adobe Glyph List
    // and Adobe Glyph List for New Fonts", and then:
    //
    // > In any of these cases, if the glyph name cannot be mapped as specified, the glyph
    // > name shall be looked up in the font program's "post" table (if one is present) and
    // > the associated glyph description shall be used.
    //
    // Those are two *lists*, and **no entry in either contains a FULL STOP**. A name like
    // `o.sc` is therefore one the lists do not map, and the sentence above is what applies
    // to it. What `read_fonts::ps::agl::name_to_char` implements is the wider *Adobe Glyph
    // List Specification*, whose algorithm for an unlisted name strips everything after the
    // first period — so it answers `o` for `o.sc`, which is a real letter and hides the
    // clause's own next step behind it.
    //
    // `issue215.pdf` is the witness and it settles the reading three times over: its
    // `/Differences` name eleven small-capital variants, `o.sc` through `n.sc`; its `post`
    // table names all eleven; its `/ToUnicode` maps them to U+F76F and neighbours, the
    // private-use block Adobe assigns to small capitals — so the *producer* says small
    // capitals. We drew `openmagazin` in lower case where four references draw small caps.
    let unlisted = glyph_name.contains('.');
    let listed_route = (!unlisted).then(|| through_character(glyph_name)).flatten();

    listed_route
        .and_then(narrow_glyph)
        .or_else(|| post_glyph(font, glyph_name))
        .or_else(|| charset.and_then(|charset| charset.by_name.get(glyph_name).copied()))
        // Last, and only for a name the lists do not hold: the specification's algorithmic
        // form. A font with no `post` entry for `o.sc` states nothing better than "an o",
        // and drawing one beats drawing nothing — but it is a recovery rather than the
        // clause's route, which is why it sits below `post` rather than above it.
        .or_else(|| {
            unlisted
                .then(|| through_character(glyph_name))
                .flatten()
                .and_then(narrow_glyph)
        })
}

/// A glyph name's glyph, from the font program's own `post` table.
///
/// Searched rather than indexed: `post` maps a glyph to its name, and this needs the
/// inverse. A simple font has at most 256 codes and this runs once per font at load time,
/// so the linear scan costs less than the map it would otherwise build — and the names it
/// is asked for are usually the ones no other route knew, so the scan usually runs to the
/// end and finds nothing.
fn post_glyph(font: &FontRef<'_>, glyph_name: &str) -> Option<u16> {
    let post = font.post().ok()?;
    (0..u16::try_from(post.num_names()).unwrap_or(u16::MAX)).find(|glyph| {
        post.glyph_name(skrifa::raw::types::GlyphId16::new(*glyph)) == Some(glyph_name)
    })
}

/// A code's glyph by treating the code itself as a character, in any subtable the font has.
///
/// The mapping of this processor's choosing, for a font that reaches none of §9.6.5.4's
/// rules. `Cmap::map_codepoint` asks every subtable in the order the font lists them, which
/// is what reaches the ones the subclause does not name. The private-use variant is where
/// symbolic `TrueType` fonts conventionally put their glyphs.
fn as_character(subtables: &Subtables<'_>, code: u32) -> Option<u16> {
    let cmap = subtables.all.as_ref()?;
    cmap.map_codepoint(code)
        .or_else(|| cmap.map_codepoint(0xF000_u32.saturating_add(code)))
        .and_then(narrow_glyph)
}

/// Narrows a glyph identifier to the `u16` a simple font's tables hold.
///
/// A glyph index beyond `u16` cannot appear in a `TrueType` font — `maxp` states the count
/// as a `u16` — so this discards nothing a well-formed font can produce.
fn narrow_glyph(glyph: GlyphId) -> Option<u16> {
    u16::try_from(glyph.to_u32()).ok()
}

/// Returns whether a font descriptor sets the symbolic flag.
///
/// Bit 3 of `/Flags`, counting from one. A symbolic font's character set is outside the
/// standard Latin set, so the encoding built into the font program describes it and a
/// Latin base encoding does not.
fn is_symbolic(document: &Document, descriptor: &Dictionary) -> bool {
    /// Bit 3, counting from one as the specification does.
    const SYMBOLIC: i64 = 1 << 2;

    document
        .get_key(descriptor, "Flags")
        .as_integer()
        .is_some_and(|flags| flags & SYMBOLIC != 0)
}

#[cfg(test)]
mod truetype_encoding_tests {
    use super::{Subtables, as_character, named_glyph, post_glyph, symbol_glyph};
    use crate::sfnt::{
        be32, glyph_length, repaired_loca_extent, repaired_loca_format, repaired_loca_order,
        sfnt_tables,
    };
    use skrifa::{FontRef, MetadataProvider};

    /// The glyph index every fixture below maps its one covered code to.
    ///
    /// Deliberately not equal to any code used here: a route that quietly fell back to
    /// treating the code as a glyph index would otherwise pass.
    const GLYPH: u16 = 7;

    /// Assembles an sfnt file from tables, which is all `FontRef` needs to read one.
    fn sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
        let count = u16::try_from(tables.len()).expect("a handful of tables");
        out.extend_from_slice(&count.to_be_bytes());
        // Binary-search hints, which nothing here reads and a well-formed file still has.
        out.extend_from_slice(&0_u16.to_be_bytes());
        out.extend_from_slice(&0_u16.to_be_bytes());
        out.extend_from_slice(&0_u16.to_be_bytes());

        let directory = 12_usize.saturating_add(16_usize.saturating_mul(tables.len()));
        let mut offset = directory;
        let mut body = Vec::new();
        for (tag, data) in tables {
            out.extend_from_slice(tag);
            out.extend_from_slice(&0_u32.to_be_bytes());
            out.extend_from_slice(&u32::try_from(offset).expect("small file").to_be_bytes());
            out.extend_from_slice(
                &u32::try_from(data.len())
                    .expect("small table")
                    .to_be_bytes(),
            );
            body.extend_from_slice(data);
            // Every table starts on a four-byte boundary.
            while body.len() % 4 != 0 {
                body.push(0);
            }
            offset = directory.saturating_add(body.len());
        }
        out.extend_from_slice(&body);
        out
    }

    /// A `head`, a `maxp`, a `loca` and a `glyf`, enough for `repaired_loca_format`.
    ///
    /// `stated` is what the `head` table claims `indexToLocFormat` is; the tables themselves
    /// are always built in the *long* form, so a fixture is well-formed exactly when it
    /// states 1.
    fn loca_fixture(stated: u16, glyphs: u16) -> Vec<([u8; 4], Vec<u8>)> {
        let entries = usize::from(glyphs).saturating_add(1);
        // Two bytes of glyph data apiece, so the last offset is a number nothing else is.
        let glyf = vec![0u8; entries.saturating_sub(1).saturating_mul(2)];
        let mut loca = Vec::new();
        for index in 0..entries {
            let at = u32::try_from(index.saturating_mul(2)).expect("small");
            loca.extend_from_slice(&at.to_be_bytes());
        }
        let mut head = vec![0u8; 54];
        head.splice(50..52, stated.to_be_bytes());
        let mut maxp = vec![0u8; 6];
        maxp.splice(4..6, glyphs.to_be_bytes());
        vec![
            (*b"head", head),
            (*b"maxp", maxp),
            (*b"loca", loca),
            (*b"glyf", glyf),
        ]
    }

    /// A byte-swapped `indexToLocFormat` is repaired from the file's own two statements.
    ///
    /// `issue2537r.pdf` states 0x0100, which is 1 in the wrong byte order and is neither of
    /// the two values ISO/IEC 14496-22 defines. What decides the repair is not another
    /// reader's behaviour but the file: `loca`'s last entry is `glyf`'s length under one
    /// reading and not the other, and `loca`'s own length is `4 × (n + 1)` rather than
    /// `2 × (n + 1)`. Both agree, so the answer is derived twice over.
    #[test]
    fn a_byte_swapped_loca_format_is_repaired_from_the_fonts_own_lengths() {
        let broken = sfnt(&loca_fixture(0x0100, 8));
        let repaired = repaired_loca_format(&broken).expect("the file says which format it is");

        let head = 12 + 16 * 4;
        assert_eq!(&repaired[head + 50..head + 52], &1_u16.to_be_bytes());
        assert_eq!(
            repaired.len(),
            broken.len(),
            "two bytes change, nothing else"
        );
    }

    /// A `loca` whose offsets descend is rebuilt from the glyphs' own bytes.
    ///
    /// Three glyphs, laid out in `glyf` in the order 2, 0, 1, with a `loca` that names each
    /// where it actually is. Two of the three pairs then descend, which is `issue11131_reduced`'s
    /// shape: `read-fonts` reads a negative length and refuses the glyph. After the repair the
    /// offsets ascend, every glyph's bytes are still its own, and the order in `glyf` is the
    /// glyph order.
    ///
    /// The glyphs are simple ones with no contours — a ten-byte header apiece — because what is
    /// under test is the ordering, and `a_composite_glyphs_length_is_its_component_loop` covers
    /// the reading of an entry that is not the simplest possible.
    #[test]
    fn a_loca_whose_offsets_descend_is_rebuilt_in_glyph_order() {
        // Three ten-byte simple glyphs, each with `numberOfContours` 0 and a distinguishing
        // bounding box, laid out in `glyf` as glyph 2, then 0, then 1.
        let glyph = |mark: u8| {
            let mut bytes = vec![0u8; 10];
            bytes[3] = mark;
            bytes
        };
        let mut glyf = Vec::new();
        glyf.extend_from_slice(&glyph(2));
        glyf.extend_from_slice(&glyph(0));
        glyf.extend_from_slice(&glyph(1));
        // Where each glyph *is*, in glyph order: 10, 20, 0 — and then the table's length.
        let mut loca = Vec::new();
        for at in [10u32, 20, 0, 30] {
            loca.extend_from_slice(&at.to_be_bytes());
        }
        let mut head = vec![0u8; 54];
        head.splice(50..52, 1_u16.to_be_bytes());
        let mut maxp = vec![0u8; 6];
        maxp.splice(4..6, 3_u16.to_be_bytes());
        let broken = sfnt(&[
            (*b"head", head),
            (*b"maxp", maxp),
            (*b"loca", loca),
            (*b"glyf", glyf),
        ]);

        let repaired = repaired_loca_order(&broken).expect("the offsets descend");
        let tables = sfnt_tables(&repaired).expect("a directory");
        let (loca_at, _) = tables[b"loca".as_slice()];
        let (glyf_at, _) = tables[b"glyf".as_slice()];
        let offsets: Vec<u32> = (0..4)
            .map(|index| be32(&repaired, loca_at + index * 4).expect("in range"))
            .collect();
        assert_eq!(offsets, vec![0, 10, 20, 30], "ascending, in glyph order");
        for (index, mark) in [0u8, 1, 2].into_iter().enumerate() {
            let at = glyf_at + index * 10 + 3;
            assert_eq!(repaired[at], mark, "glyph {index} keeps its own bytes");
        }
    }

    /// An empty glyph stays empty, even where the table around it is scrambled.
    ///
    /// The glyph table's own standard writes a glyph with no outline by giving it and its
    /// successor the same offset, and that statement is self-consistent whatever the rest of
    /// the table does — unlike a *descending* pair, which is what the repair exists to overrule.
    /// Reading such a glyph's length from its own bytes hands it whichever entry happens to
    /// begin there, which is a real glyph and was a real defect: `issue7074_reduced.pdf` drew
    /// `Our|2015|Graduates`, a narrow mark where each space belongs, because its `loca` runs
    /// 0, 108, 0, 108, 108, … and the entry at 108 is glyph 4.
    #[test]
    fn a_glyph_whose_offset_repeats_is_empty_and_stays_empty() {
        let glyph = |mark: u8| {
            let mut bytes = vec![0u8; 10];
            bytes[3] = mark;
            bytes
        };
        let mut glyf = Vec::new();
        glyf.extend_from_slice(&glyph(1));
        glyf.extend_from_slice(&glyph(0));
        // Glyph 0 is at 10; glyph 1 is *empty*, stated by repeating glyph 2's offset; glyph 2 is
        // at 0. The pair (10, 0) is what makes the table descend and the repair run at all.
        let mut loca = Vec::new();
        for at in [10u32, 0, 0, 20] {
            loca.extend_from_slice(&at.to_be_bytes());
        }
        let mut head = vec![0u8; 54];
        head.splice(50..52, 1_u16.to_be_bytes());
        let mut maxp = vec![0u8; 6];
        maxp.splice(4..6, 3_u16.to_be_bytes());
        let repaired = repaired_loca_order(&sfnt(&[
            (*b"head", head),
            (*b"maxp", maxp),
            (*b"loca", loca),
            (*b"glyf", glyf),
        ]))
        .expect("the offsets descend");

        let tables = sfnt_tables(&repaired).expect("a directory");
        let (loca_at, _) = tables[b"loca".as_slice()];
        let (glyf_at, _) = tables[b"glyf".as_slice()];
        let offsets: Vec<u32> = (0..4)
            .map(|index| be32(&repaired, loca_at + index * 4).expect("in range"))
            .collect();
        assert_eq!(
            offsets,
            vec![0, 10, 10, 20],
            "glyph 1 keeps a length of zero rather than taking the entry at its offset"
        );
        assert_eq!(repaired[glyf_at + 3], 0, "glyph 0 keeps its own bytes");
        assert_eq!(repaired[glyf_at + 10 + 3], 1, "and so does glyph 2");
    }

    /// A font whose directory overlaps itself is refused rather than repaired.
    ///
    /// Both crashers `fuzz/fuzz_targets/sfnt.rs` produced in its first minute, as the smallest
    /// fonts that reach them. A table directory is bytes a document supplied and both repairs
    /// *write* at offsets computed from it, so the two structural rules they rely on have to be
    /// checked rather than assumed: no table begins inside the directory, and no tag is named
    /// twice.
    ///
    /// The first: a `head` pointing at the directory turned "correct `indexToLocFormat`" into
    /// two bytes written over another table's *tag*. The second: with a tag repeated,
    /// `sfnt_tables` keeps the last entry and `rewritten_sfnt` patches the first, so the repair
    /// wrote one and read the other — and `repaired_font_program` found work to do on its own
    /// output, without end.
    #[test]
    fn a_directory_that_overlaps_or_repeats_itself_is_refused() {
        let sound = || {
            let mut head = vec![0u8; 54];
            head.splice(50..52, 1_u16.to_be_bytes());
            let mut maxp = vec![0u8; 6];
            maxp.splice(4..6, 2_u16.to_be_bytes());
            let mut loca = Vec::new();
            for at in [10u32, 0, 20] {
                loca.extend_from_slice(&at.to_be_bytes());
            }
            let glyf = vec![0u8; 20];
            vec![
                (*b"head", head),
                (*b"maxp", maxp),
                (*b"loca", loca),
                (*b"glyf", glyf),
            ]
        };
        // The fixture as built is repairable, which is what makes the two refusals below mean
        // something rather than pass for want of a repair to make.
        assert!(repaired_loca_order(&sfnt(&sound())).is_some());

        // `head` moved on top of the directory: 12 + 4 x 16 = 76 bytes of it, and 8 is inside.
        let mut overlapping = sfnt(&sound());
        let entry = (0..4)
            .map(|index| 12 + index * 16)
            .find(|at| &overlapping[*at..*at + 4] == b"head")
            .expect("the fixture names head");
        overlapping[entry + 8..entry + 12].copy_from_slice(&8_u32.to_be_bytes());
        assert_eq!(
            repaired_loca_order(&overlapping),
            None,
            "a table inside the directory is a font this cannot reason about"
        );

        // A second `glyf` entry, which is the shape that made the repair non-idempotent.
        let mut repeated = sound();
        repeated.push((*b"glyf", vec![0u8; 20]));
        assert_eq!(
            repaired_loca_order(&sfnt(&repeated)),
            None,
            "one tag names one table"
        );
    }

    /// A composite glyph's length is its component loop, not a fixed size.
    ///
    /// One component with `ARG_1_AND_2_ARE_WORDS` and `WE_HAVE_A_TWO_BY_TWO` set and
    /// `MORE_COMPONENTS` clear: 10 bytes of header, 2 of flags, 2 of glyph index, 4 of
    /// arguments and 8 of transform.
    #[test]
    fn a_composite_glyphs_length_is_its_component_loop() {
        let mut glyf = vec![0xFF, 0xFF]; // numberOfContours = -1
        glyf.extend_from_slice(&[0u8; 8]); // the rest of the header
        glyf.extend_from_slice(&0x0081_u16.to_be_bytes()); // words, two-by-two, no more
        glyf.extend_from_slice(&0_u16.to_be_bytes()); // the component's glyph index
        glyf.extend_from_slice(&[0u8; 4]); // two word arguments
        glyf.extend_from_slice(&[0u8; 8]); // the 2x2 transform
        assert_eq!(glyph_length(&glyf, 0), Some(26));
    }

    /// A font that states a legal format is left alone, whichever of the two it states.

    #[test]
    fn a_font_stating_a_legal_loca_format_is_not_touched() {
        for stated in [0, 1] {
            assert_eq!(repaired_loca_format(&sfnt(&loca_fixture(stated, 8))), None);
        }
        // And a font whose offsets ascend leaves `repaired_loca_order` on its first pass, which
        // is every well-formed font and is what makes the repair cost nothing on the common path.
        assert_eq!(repaired_loca_order(&sfnt(&loca_fixture(1, 8))), None);
    }

    /// Overwrites one table record's stated `length`, leaving the table's bytes where they are.
    ///
    /// The fixture builder writes a record that agrees with its data, which is the one thing
    /// [`repaired_loca_extent`]'s subject does not: `3867363.pdf`'s `loca` record understates
    /// its own table.
    fn understate(font: &mut [u8], tag: [u8; 4], length: u32) {
        let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
        let at = (0..count)
            .map(|index| 12_usize.saturating_add(index.saturating_mul(16)))
            .find(|at| font.get(*at..at.saturating_add(4)) == Some(tag.as_slice()))
            .expect("the fixture has this table");
        let field = at.saturating_add(12);
        font[field..field.saturating_add(4)].copy_from_slice(&length.to_be_bytes());
    }

    /// A `loca` record too short for `numGlyphs + 1` entries is corrected from the table itself.
    ///
    /// `3867363.pdf`'s 3254-glyph subset states 6510 bytes of `loca` where 3255 long offsets
    /// need 13 020, and the bytes there are a whole table — ascending, ending exactly at
    /// `glyf`'s length. `skrifa` reads the record, finds it too short for `numGlyphs`, and
    /// produces no outline for any glyph at all, so the page draws blank. The fixture is that
    /// shape at fixture scale: nine long entries, a record claiming four of them.
    #[test]
    fn a_loca_record_short_of_its_own_entries_is_corrected() {
        let mut broken = sfnt(&loca_fixture(1, 8));
        understate(&mut broken, *b"loca", 18);
        let repaired =
            repaired_loca_extent(&broken).expect("the table is whole; the number is not");

        let record = 12 + 16 * 2 + 12;
        assert_eq!(&repaired[record..record + 4], &36_u32.to_be_bytes());
        assert_eq!(
            repaired.len(),
            broken.len(),
            "four bytes change, nothing else"
        );
    }

    /// The two twins that keep the correction from being a guess.
    ///
    /// A record that already holds its entries is left alone — every well-formed font — and a
    /// short record whose extended read is *not* a `loca` is refused rather than believed. The
    /// second is the whole safety of reading past a stated length: what is accepted has to
    /// ascend and has to end at `glyf`'s length, which arbitrary bytes do not.
    #[test]
    fn a_loca_extent_is_corrected_only_where_the_table_proves_itself() {
        assert_eq!(repaired_loca_extent(&sfnt(&loca_fixture(1, 8))), None);

        let mut tables = loca_fixture(1, 8);
        // The terminator no longer names `glyf`'s length, so these bytes are not that table.
        let last = tables[2].1.len() - 4;
        tables[2].1[last..].copy_from_slice(&99_u32.to_be_bytes());
        let mut broken = sfnt(&tables);
        understate(&mut broken, *b"loca", 18);
        assert_eq!(repaired_loca_extent(&broken), None);
    }

    /// A font whose lengths agree with *neither* reading keeps its own answer.
    ///
    /// The point of the two tests is that this repair can fail: a `loca` that is neither
    /// `2 × (n + 1)` nor `4 × (n + 1)` bytes long is broken in a way this cannot name, and
    /// inventing a format for it would be exactly the guess the derivation exists to avoid.
    #[test]
    fn a_font_neither_reading_explains_is_left_to_skrifa() {
        let mut tables = loca_fixture(0x0100, 8);
        // One byte short of either form.
        tables[2].1.pop();
        assert_eq!(repaired_loca_format(&sfnt(&tables)), None);
    }

    /// A `cmap` with one subtable, in format 6, under the platform and encoding IDs given.
    ///
    /// Format 6 throughout, for every platform, so that what differs between the fixtures
    /// is only the identity §9.6.5.4 selects on. A subtable format is a storage detail and
    /// `read-fonts` reads all the common ones; the platform and encoding IDs are the
    /// subclause's whole subject.
    fn cmap(platform: u16, encoding: u16, first_code: u16, glyphs: &[u16]) -> Vec<u8> {
        let mut subtable = Vec::new();
        subtable.extend_from_slice(&6_u16.to_be_bytes());
        let length = 10_usize.saturating_add(2_usize.saturating_mul(glyphs.len()));
        subtable.extend_from_slice(&u16::try_from(length).expect("short").to_be_bytes());
        subtable.extend_from_slice(&0_u16.to_be_bytes());
        subtable.extend_from_slice(&first_code.to_be_bytes());
        subtable.extend_from_slice(&u16::try_from(glyphs.len()).expect("few").to_be_bytes());
        for glyph in glyphs {
            subtable.extend_from_slice(&glyph.to_be_bytes());
        }

        let mut table = Vec::new();
        table.extend_from_slice(&0_u16.to_be_bytes());
        table.extend_from_slice(&1_u16.to_be_bytes());
        table.extend_from_slice(&platform.to_be_bytes());
        table.extend_from_slice(&encoding.to_be_bytes());
        table.extend_from_slice(&12_u32.to_be_bytes());
        table.extend_from_slice(&subtable);
        table
    }

    /// A version 2.0 `post` table naming one glyph, and leaving every other `.notdef`.
    fn post(glyph: u16, name: &str) -> Vec<u8> {
        /// How many names the format reserves before a font's own begin.
        const MACINTOSH_NAMES: u16 = 258;

        let count = glyph.saturating_add(1);
        let mut table = vec![0_u8; 32];
        table.splice(0..4, 0x0002_0000_u32.to_be_bytes());
        table.extend_from_slice(&count.to_be_bytes());
        for index in 0..count {
            let entry = if index == glyph { MACINTOSH_NAMES } else { 0 };
            table.extend_from_slice(&entry.to_be_bytes());
        }
        table.push(u8::try_from(name.len()).expect("a short name"));
        table.extend_from_slice(name.as_bytes());
        table
    }

    /// The premise of the whole algorithm, asserted rather than assumed.
    ///
    /// `skrifa`'s `Charmap` selects the best *Unicode* subtable, and a (1, 0) Macintosh one
    /// is not a Unicode mapping, so it selects nothing. Handing it a character code —
    /// which is what this crate used to do for every `TrueType` font — therefore reaches no
    /// glyph at all in a font shaped the way §9.6.5.4's own guidelines ask for. If this
    /// test ever fails, `skrifa` has changed and the reasoning in `Subtables` needs
    /// re-reading, not the code.
    #[test]
    fn a_unicode_charmap_cannot_see_a_macintosh_subtable() {
        let data = sfnt(&[(*b"cmap", cmap(1, 0, 33, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("the fixture is a readable sfnt");

        assert_eq!(font.charmap().map(33_u32), None);
        assert_eq!(symbol_glyph(&Subtables::read(&font), 33), Some(GLYPH));
    }

    /// "Otherwise, if the font contains a (1, 0) subtable, single bytes from the string
    /// shall be used to look up the associated glyph descriptions from the subtable."
    #[test]
    fn a_macintosh_subtable_is_addressed_by_the_byte_itself() {
        let data = sfnt(&[(*b"cmap", cmap(1, 0, 33, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(symbol_glyph(&subtables, 33), Some(GLYPH));
        assert_eq!(symbol_glyph(&subtables, 34), None);
    }

    /// "If the font contains a (3, 0) subtable, the range of character codes shall be one
    /// of these: 0x0000 - 0x00FF, 0xF000 - 0xF0FF, 0xF100 - 0xF1FF, or 0xF200 - 0xF2FF.
    /// Depending on the range of codes, each byte from the string shall be prepended with
    /// the high byte of the range."
    #[test]
    fn a_symbol_subtable_is_addressed_through_the_high_byte_of_its_range() {
        for high in [0x0000_u16, 0xF000, 0xF100, 0xF200] {
            let data = sfnt(&[(*b"cmap", cmap(3, 0, high | 0x41, &[GLYPH]))]);
            let font = FontRef::new(&data).expect("readable");

            assert_eq!(
                symbol_glyph(&Subtables::read(&font), 0x41),
                Some(GLYPH),
                "a (3, 0) subtable in the {high:#06x} range was not found"
            );
        }
    }

    /// "A character code shall be first mapped to a glyph name … the glyph name shall then
    /// be mapped to a Unicode value by consulting the Adobe Glyph List … finally, the
    /// Unicode value shall be mapped to a glyph description according to the (3, 1)
    /// subtable."
    #[test]
    fn a_unicode_subtable_is_reached_through_the_adobe_glyph_list() {
        // U+00E9, which the Adobe Glyph List spells `eacute`.
        let data = sfnt(&[(*b"cmap", cmap(3, 1, 0x00E9, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(named_glyph(&font, &subtables, None, "eacute"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "egrave"), None);
    }

    /// "The glyph name shall then be mapped back to a character code according to the
    /// standard Roman encoding used on Mac OS."
    ///
    /// The name and the code are chosen where Mac OS Roman and every other encoding
    /// disagree: `eacute` is code 142 there and 233 in `WinAnsiEncoding`. A route reaching
    /// this subtable with the wrong encoding's code finds nothing, rather than finding a
    /// plausible wrong glyph — which is why the fixture covers only the one code.
    #[test]
    fn a_macintosh_subtable_is_reached_through_mac_os_roman() {
        let data = sfnt(&[(*b"cmap", cmap(1, 0, 142, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(named_glyph(&font, &subtables, None, "eacute"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "adieresis"), None);
    }

    /// "In any of these cases, if the glyph name cannot be mapped as specified, the glyph
    /// name shall be looked up in the font program's `post` table."
    ///
    /// `gid2436` is the shape that motivated keeping unrecognised `/Differences` names at
    /// all: a subsetter's convention for naming a glyph by index, which no encoding and no
    /// character set knows, and which the font itself may nonetheless carry.
    #[test]
    fn a_name_no_encoding_knows_is_found_in_the_post_table() {
        let data = sfnt(&[
            (*b"cmap", cmap(3, 1, 0x00E9, &[1])),
            (*b"post", post(GLYPH, "gid2436")),
        ]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(post_glyph(&font, "gid2436"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "gid2436"), Some(GLYPH));
        assert_eq!(named_glyph(&font, &subtables, None, "gid9999"), None);
    }

    /// A suffixed name is not one the Adobe Glyph List holds, so the `post` table decides.
    ///
    /// §9.6.5.4 sends a glyph name "to a Unicode value by consulting the Adobe Glyph List and
    /// Adobe Glyph List for New Fonts", and then to the `post` table "if the glyph name cannot
    /// be mapped as specified". Those are two *lists*, and neither holds an entry containing a
    /// FULL STOP — but the wider Adobe Glyph List *Specification* carries an algorithm for
    /// unlisted names that strips everything after the first period, which `read_fonts`
    /// implements and which answers `o` for `o.sc`. That answer is a real letter, so it hid
    /// the clause's own next step behind it and `issue215.pdf` drew `openmagazin` in lower
    /// case where four references draw small capitals — which the file itself confirms, by
    /// mapping those codes to U+F76F and its neighbours, the private-use block Adobe assigns
    /// to small capitals.
    ///
    /// The fixture puts both glyphs in reach so that a wrong order is a wrong answer rather
    /// than a missing one: the (3, 1) subtable holds `o` at glyph 1 and the `post` table names
    /// glyph 7 `o.sc`.
    #[test]
    fn a_suffixed_glyph_name_reaches_the_program_rather_than_its_base_letter() {
        let data = sfnt(&[
            (*b"cmap", cmap(3, 1, 0x006F, &[1])),
            (*b"post", post(GLYPH, "o.sc")),
        ]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(named_glyph(&font, &subtables, None, "o.sc"), Some(GLYPH));
        // And the unsuffixed name still takes the clause's first route.
        assert_eq!(named_glyph(&font, &subtables, None, "o"), Some(1));
        // A suffixed name the program does not carry falls back to the base letter, which
        // is a recovery rather than the clause's route — better than drawing nothing, and
        // last precisely because it is not what the subclause says.
        assert_eq!(named_glyph(&font, &subtables, None, "o.alt"), Some(1));
    }

    /// The mapping of this processor's choosing reaches a subtable §9.6.5.4 never names.
    ///
    /// `issue5501.pdf` carries its whole byte-to-glyph mapping in a (0, 0) subtable, which
    /// the subclause does not mention and which is the only correct answer for that font.
    #[test]
    fn the_last_resort_asks_every_subtable_including_unnamed_ones() {
        let data = sfnt(&[(*b"cmap", cmap(0, 0, 4, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");
        let subtables = Subtables::read(&font);

        assert_eq!(symbol_glyph(&subtables, 4), None, "not a (3, 0) or (1, 0)");
        assert_eq!(as_character(&subtables, 4), Some(GLYPH));
    }

    /// A symbolic font's private-use convention, which predates this algorithm here.
    #[test]
    fn the_last_resort_also_tries_the_private_use_area() {
        let data = sfnt(&[(*b"cmap", cmap(3, 1, 0xF041, &[GLYPH]))]);
        let font = FontRef::new(&data).expect("readable");

        assert_eq!(as_character(&Subtables::read(&font), 0x41), Some(GLYPH));
    }

    /// A font with no `cmap` at all is the only one whose codes may be glyph indices.
    #[test]
    fn only_a_font_without_a_cmap_has_no_subtables_to_ask() {
        let with = sfnt(&[(*b"cmap", cmap(1, 0, 33, &[GLYPH]))]);
        let without = sfnt(&[(*b"post", post(GLYPH, "gid2436"))]);

        let font = FontRef::new(&with).expect("readable");
        assert!(Subtables::read(&font).all.is_some());
        let font = FontRef::new(&without).expect("readable");
        assert!(Subtables::read(&font).all.is_none());
    }
}
