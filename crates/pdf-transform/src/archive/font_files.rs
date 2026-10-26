//! A font program filed under the key, and for a `/FontFile3` the `/Subtype`, of its format.
//!
//! ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 require a font program
//! embedded as ISO 32000 section 9.9 defines it. ISO 32000-2 §9.9's Table 124 files each format
//! under a descriptor key by the dictionaries that may carry it, and §9.9's Table 125 makes a
//! `/FontFile3` name the format:
//!
//! > A name specifying the format of the embedded font program. The name shall be Type1C for Type
//! > 1 compact fonts, CIDFontType0C for Type 0 compact CIDFonts, or OpenType for OpenType fonts.
//!
//! A producer can file a program under the wrong key or name and cannot write the wrong leading
//! bytes, so where the bytes settle a format a row admits under the font's dictionary, the program
//! is refiled there and nothing else changes: it is carried byte for byte, and this reader already
//! chose its reader by those bytes (`pdf_font::program`). A row under the same key is taken first,
//! which restates the name alone; failing that, a `/FontFile3` row, which moves the stream to that
//! key and drops the three lengths a compact program does not state. Moving a program the other
//! way would need lengths this conversion does not compute, and where no row admits the pairing —
//! a `glyf` program under a `CIDFontType0` dictionary is the corpus's case — no filing is true of
//! it, and making one true would change the dictionary's subtype and with it how a CID selects a
//! glyph (§9.7.4.2), so the document is refused by that sentence (ADR 2307).
//!
//! **One such change selects the same glyphs, and is made** (ADR 2326). A CFF-flavoured
//! OpenType program under a `CIDFontType2` dictionary is in no row, and Table 124 admits it under
//! `CIDFontType0`. §9.7.4.1's Table 115 says what an `Identity` map does under the first:
//!
//! > If the value of CIDToGIDMap is a name, it shall be Identity , indicating that the mapping
//! > between CIDs and glyph indices is the identity mapping.
//!
//! and §9.7.4.2 what the second does with a CFF whose Top DICT does not use `CIDFont` operators:
//!
//! > The CIDs shall be used directly as GID values, and the glyph procedure shall be retrieved
//! > using the CharStrings INDEX
//!
//! So where the map is `Identity` and the program is keyed by name, every CID selects the glyph
//! index it selected before, and the dictionary becomes `CIDFontType0`, its `/CIDToGIDMap` — an
//! entry Table 115 gives a Type 2 `CIDFont` alone — removed, and its program moved to the
//! `/FontFile3` `/OpenType` row. **A map stated as a stream** sends CID *c* to the glyph index in
//! its bytes 2*c* and 2*c* + 1, so the CIDs change to the glyph indices they selected: the `Type0`
//! font's encoding and the widths are re-keyed the same way ([`glyph_indices`], ADR 2342). No map,
//! or a CID-keyed program, which §9.7.4.2 sends through the charset, selects glyphs nothing says
//! the source selected, and is refused.
//!
//! **A font collection is no row's**: the TrueType Reference Manual's font file opens with its own
//! font directory, whose scaler type names the one font's outlines, and a `ttcf` header names
//! none. The face this reader draws, the one the descriptor's `/FontName` names (ADR 0141), is
//! written out of it as a program of its own and filed by its own format; a collection none of
//! whose faces carries that name is refused by a sentence of its own (ADR 2343).

mod glyph_indices;

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::{Outcome, Table124Row};
use pdf_font::embedding::Keying;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::Spare;

/// The requirement a refiled program answers.
const REQUIREMENT: &str = "fonts/font-programs-filed-by-their-format";

/// Why a program no row admits keeps its refusal.
const NO_ROW: &str = "a font program is in a format ISO 32000-2's Table 124 admits under no key \
     in the dictionary that names it - a glyf program under a CIDFontType0 dictionary, for one. No \
     filing is true of it, and making one true would change the font dictionary's subtype and with \
     it how a CID selects a glyph (section 9.7.4.2), which is a reinterpretation this conversion \
     does not make";

/// Why a program whose format the bytes do not settle keeps its refusal.
const NO_FORMAT: &str = "a FontFile3 states no Subtype Table 125 names, and its program's bytes \
     are in no format this reader can name, so there is no name to give it";

/// Why a program Table 124 files only under a key this conversion does not move into keeps its
/// refusal.
const NOT_INTO_THAT_KEY: &str = "a font program is filed under a key its format is not, and the \
     key Table 124 gives it is FontFile or FontFile2, whose stream states lengths of the program's \
     own parts that this conversion does not compute";

/// Why a finding list the validator shortened keeps its refusal.
const NOT_ALL_NAMED: &str = "more font programs are misfiled than the validator's report names, \
     so refiling the ones it named would leave the others failing";

/// Why a program no font dictionary in the file names keeps its refusal.
const NO_DICTIONARY: &str = "a font program the validator named is reached through a font \
     dictionary, or a descriptor written inside one, that this conversion's walk of the file's \
     objects does not edit, so it is not refiled";

/// Why a CFF-flavoured program under a `CIDFontType2` whose CIDs would select other glyphs under
/// `CIDFontType0` keeps its refusal.
const NOT_THE_SAME_GLYPHS: &str = "a CFF-flavoured OpenType program is under a CIDFontType2 \
     dictionary, and Table 124 admits it only under CIDFontType0. Under that subtype section \
     9.7.4.2 uses a CID directly as the glyph index of a name-keyed program and sends it through \
     the charset of a CID-keyed one; here the dictionary states no CIDToGIDMap, or the program is \
     CID-keyed, so nothing says which glyph each CID selected before";

/// Why a font collection keeps its refusal.
const A_COLLECTION: &str = "a font program is a font collection, a ttcf header over several \
     fonts' directories. Table 124 files a TrueType font program as the TrueType Reference Manual \
     describes one, a single font whose file opens with its own font directory, and an OpenType \
     program likewise as one font, so no key holds it. The face the descriptor's FontName names \
     would be written out as a program of its own (doc/adr/2343), and here no face of the \
     collection carries that name, or the dictionaries naming the program name different faces";

/// Why a program whose glyphs cannot be counted keeps its refusal.
const NOT_COUNTED: &str = "a CFF-flavoured OpenType program under a CIDFontType2 dictionary \
     states no glyph count this reader can read, so no CID can be shown to be one of its glyph \
     indices";

/// Where each misfiled program goes.
#[derive(Debug, Default)]
pub(super) struct Refiled {
    /// The row each program stream is filed under, by the stream's object.
    pub(super) at: BTreeMap<ObjectId, Table124Row>,
    /// The key each descriptor whose program moves to `/FontFile3` names it by now.
    pub(super) moved: BTreeMap<ObjectId, &'static str>,
    /// The program streams that move key, which lose the lengths a compact program does not state.
    pub(super) moved_programs: BTreeSet<ObjectId>,
    /// The `CIDFontType2` dictionaries that become `CIDFontType0`, each losing its
    /// `/CIDToGIDMap` (ADR 2326).
    pub(super) retyped: BTreeSet<ObjectId>,
    /// Of those, the ones whose map was a stream, with the widths and encodings that re-key their
    /// CIDs to glyph indices (ADR 2342).
    pub(super) rekeyed: BTreeMap<ObjectId, glyph_indices::Rekeyed>,
    /// For each `Type0` font whose `/Encoding` is re-keyed, the `CMap` stream it names now.
    pub(super) encodings: BTreeMap<ObjectId, ObjectId>,
    /// The program streams that held a font collection, each now the one face its descriptor
    /// names, as a whole stream (ADR 2343).
    pub(super) faces: BTreeMap<ObjectId, Object>,
}

impl Refiled {
    /// The `CMap` streams the re-keyed encodings name, for the conversion to add.
    pub(super) fn written(&self) -> impl Iterator<Item = (&ObjectId, &Object)> {
        self.rekeyed
            .values()
            .flat_map(|rekeyed| rekeyed.written.iter())
    }
}

/// How a `CIDFontType2` dictionary maps its CIDs to glyph indices (Table 115).
#[derive(Clone, Copy)]
enum CidMap {
    /// The name `Identity`.
    Identity,
    /// A stream.
    Stream,
    /// No entry, or one that is neither.
    Other,
}

/// One dictionary naming a program: its `/Subtype`, its own object and its descriptor's where
/// each is one, and the key the descriptor files the program under.
struct Holder {
    /// The font dictionary's `/Subtype`, the descendant's for a composite font.
    subtype: String,
    /// The font dictionary's own object, `None` where it is written inside another.
    font: Option<ObjectId>,
    /// How the dictionary's `/CIDToGIDMap` maps CIDs to glyph indices.
    map: CidMap,
    /// The descriptor's `/FontName`, which names the face of a collection (ADR 0141).
    font_name: Option<String>,
    /// The descriptor's own object, `None` where it is written inside the font dictionary.
    descriptor: Option<ObjectId>,
    /// The key the program is under now.
    key: &'static str,
}

/// How a dictionary becomes `CIDFontType0` to take a row, where it does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Retyping {
    /// It keeps its subtype.
    None,
    /// Its CIDs are already the glyph indices they select (ADR 2326).
    SameCids,
    /// Its CIDs change to the glyph indices they selected (ADR 2342).
    GlyphIndices,
}

/// The rows a program takes under `holder`, and whether its dictionary becomes `CIDFontType0`
/// to take them.
///
/// Table 124's rows under the dictionary's own subtype where there are any; for a `CIDFontType2`
/// over a CFF-flavoured OpenType program, which has none, `CIDFontType0`'s rows where each CID
/// can be shown to select the glyph it selected (the module's comment), and the refusal naming
/// why not where it cannot.
fn rows_for(holder: &Holder, program: &[u8]) -> Result<(Vec<Table124Row>, Retyping), Because> {
    let rows = pdf_archive::table_124_rows(&holder.subtype, program)
        .ok_or(Because::NotBuiltYet(NO_FORMAT))?;
    let tables = pdf_font::embedding::sfnt_tables(program);
    let wrapped_cff = tables
        .as_ref()
        .is_some_and(|tables| tables.carries(b"CFF "));
    if !rows.is_empty() || holder.subtype != "CIDFontType2" || !wrapped_cff {
        return Ok((rows, Retyping::None));
    }
    let by_name = tables.and_then(|tables| tables.cff) == Some(Keying::ByName);
    let retyping = match (holder.map, by_name) {
        (CidMap::Identity, true) => Retyping::SameCids,
        (CidMap::Stream, true) => Retyping::GlyphIndices,
        _ => return Err(Because::NotBuiltYet(NOT_THE_SAME_GLYPHS)),
    };
    let rows = pdf_archive::table_124_rows("CIDFontType0", program)
        .ok_or(Because::NotBuiltYet(NO_FORMAT))?;
    Ok((rows, retyping))
}

/// The face of a collection the dictionaries naming it all name, written out as one program.
fn face_of(naming: &[Holder], collection: &[u8]) -> Result<Vec<u8>, Because> {
    let mut names = naming.iter().map(|holder| holder.font_name.as_deref());
    let first = names.next().flatten();
    let name = first
        .filter(|first| names.all(|name| name == Some(*first)))
        .ok_or(Because::NotBuiltYet(A_COLLECTION))?;
    pdf_font::embedding::collection_face(collection, name).ok_or(Because::NotBuiltYet(A_COLLECTION))
}

/// Where every program the requirement failed at is refiled, or why none is.
///
/// **Nothing is judged here**: `pdf_archive::table_124_rows` is the requirement's own reading of
/// Table 124, asked again of the same bytes, so a filing this writes is one that row admits.
pub(super) fn refiled(
    document: &Document,
    input: &pdf_archive::Report,
    spare: &mut Spare,
) -> Result<Refiled, Because> {
    let mut named = BTreeSet::new();
    for judgement in input
        .failures()
        .filter(|judgement| judgement.id == REQUIREMENT)
    {
        let Outcome::Failed { places, total } = &judgement.outcome else {
            continue;
        };
        if places.len() != *total {
            return Err(Because::NotBuiltYet(NOT_ALL_NAMED));
        }
        for finding in places {
            named.insert(
                finding
                    .place
                    .object
                    .ok_or(Because::NotBuiltYet(NO_DICTIONARY))?,
            );
        }
    }
    let holders = dictionaries_naming_each_program(document);
    let mut out = Refiled::default();
    for id in named {
        let Object::Stream(stream) = document.get(id) else {
            return Err(Because::NotBuiltYet(NO_DICTIONARY));
        };
        let program = document
            .decoded_stream_data(&stream)
            .ok_or(Because::NotBuiltYet(NO_FORMAT))?;
        let naming = holders
            .get(&id)
            .filter(|naming| !naming.is_empty())
            .ok_or(Because::NotBuiltYet(NO_DICTIONARY))?;
        // A collection is filed as the face written out of it, so the row is that face's.
        let face = if program.starts_with(b"ttcf") {
            Some(face_of(naming, &program)?)
        } else {
            None
        };
        let program = face.as_deref().unwrap_or(&program);
        // One program under two dictionaries is one stream, so every dictionary naming it has to
        // be given the same filing.
        let mut chosen: Option<Table124Row> = None;
        for holder in naming {
            let (rows, retyping) = rows_for(holder, program)?;
            if retyping != Retyping::None {
                let font = holder.font.ok_or(Because::NotBuiltYet(NO_DICTIONARY))?;
                out.retyped.insert(font);
                if retyping == Retyping::GlyphIndices && !out.rekeyed.contains_key(&font) {
                    let glyphs = pdf_font::embedding::glyph_count(
                        program,
                        pdf_font::substitute::Format::Sfnt,
                    )
                    .ok_or(Because::NotBuiltYet(NOT_COUNTED))?;
                    let rekeyed = glyph_indices::rekey(document, font, glyphs, spare)?;
                    out.encodings.extend(rekeyed.encodings.iter());
                    out.rekeyed.insert(font, rekeyed);
                }
            }
            let row = rows
                .iter()
                .find(|row| row.key == holder.key)
                .or_else(|| rows.iter().find(|row| row.key == "FontFile3"))
                .copied();
            let row = match (row, rows.is_empty()) {
                (Some(row), _) => row,
                (None, true) => return Err(Because::NotBuiltYet(NO_ROW)),
                (None, false) => return Err(Because::NotBuiltYet(NOT_INTO_THAT_KEY)),
            };
            if chosen.is_some_and(|chosen| chosen != row) {
                return Err(Because::NotBuiltYet(NO_ROW));
            }
            chosen = Some(row);
            if row.key != holder.key {
                let descriptor = holder
                    .descriptor
                    .ok_or(Because::NotBuiltYet(NO_DICTIONARY))?;
                out.moved.insert(descriptor, holder.key);
                out.moved_programs.insert(id);
            }
        }
        if let Some(row) = chosen {
            out.at.insert(id, row);
            // The face's stream is written whole: its bytes, its name where the row states one,
            // and the `/Length1` Table 125 asks of a `/FontFile2` alone.
            if let Some(face) = &face {
                let mut dict = stream.dict.clone();
                if let Some(subtype) = row.subtype {
                    dict.insert(
                        Name::new(&b"Subtype"[..]),
                        Object::Name(Name::new(subtype.as_bytes())),
                    );
                }
                let written =
                    super::fonts::program_stream(Some(&dict), face, row.key == "FontFile2")
                        .ok_or(Because::NotBuiltYet(A_COLLECTION))?;
                out.faces.insert(id, written);
            }
        }
    }
    Ok(out)
}

/// For every program stream the file's font descriptors name, each dictionary naming it — a
/// composite font's descendant rather than the `Type0` dictionary, which §9.7.6.2 gives no
/// descriptor.
fn dictionaries_naming_each_program(document: &Document) -> BTreeMap<ObjectId, Vec<Holder>> {
    let mut out: BTreeMap<ObjectId, Vec<Holder>> = BTreeMap::new();
    let mut note = |font: &Dictionary, at: Option<ObjectId>| {
        let Some(subtype) = name(document, font, "Subtype") else {
            return;
        };
        let map = match document.get_key(font, "CIDToGIDMap") {
            Object::Name(name) if name.as_bytes() == b"Identity" => CidMap::Identity,
            Object::Stream(_) => CidMap::Stream,
            _ => CidMap::Other,
        };
        let stated = font.get("FontDescriptor");
        let descriptor_at = stated.and_then(Object::as_reference);
        let descriptor = document.get_key(font, "FontDescriptor");
        let Some(descriptor) = descriptor.as_dict() else {
            return;
        };
        let font_name = name(document, descriptor, "FontName");
        for key in ["FontFile", "FontFile2", "FontFile3"] {
            if let Some(program) = descriptor.get(key).and_then(Object::as_reference) {
                out.entry(program).or_default().push(Holder {
                    subtype: subtype.clone(),
                    font: at,
                    map,
                    font_name: font_name.clone(),
                    descriptor: descriptor_at,
                    key,
                });
            }
        }
    };
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let object = document.get(id);
        let Some(font) = object.as_dict() else {
            continue;
        };
        if name(document, font, "Subtype").as_deref() == Some("Type0") {
            let descendants = document.get_key(font, "DescendantFonts");
            for descendant in descendants.as_array().into_iter().flatten() {
                let at = descendant.as_reference();
                let descendant = document.resolve(descendant);
                if let Some(descendant) = descendant.as_dict() {
                    note(descendant, at);
                }
            }
        } else if font.get("FontDescriptor").is_some() {
            note(font, Some(id));
        }
    }
    out
}

/// A name-valued entry, as text.
fn name(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    document
        .get_key(dict, key)
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use pdf_archive::{Flavour, Target};
    use pdf_syntax::Document;
    use pdf_syntax::object::Object;

    use super::{A_COLLECTION, Because, NO_ROW, NOT_THE_SAME_GLYPHS, Spare, refiled};

    /// Object numbers past every fixture's own.
    fn spare() -> Spare {
        Spare { next: 100 }
    }

    /// A one-page document showing one font of `dictionary` whose `/FontFile3` states `/Type1C`
    /// over an sfnt naming `glyf` and the five tables Table 124's `OpenType` row asks beside it.
    fn a_misnamed_glyf_program_under(dictionary: &str) -> Document {
        let tags: [&[u8; 4]; 7] = [
            b"cmap", b"glyf", b"head", b"hhea", b"hmtx", b"loca", b"maxp",
        ];
        let end = 12_u32 + 16 * 7;
        let mut program = vec![0, 1, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0];
        for tag in tags {
            program.extend_from_slice(tag);
            program.extend_from_slice(&[0; 4]);
            program.extend_from_slice(&end.to_be_bytes());
            program.extend_from_slice(&[0; 4]);
        }
        a_program_under(dictionary, "FontFile3", "/Subtype /Type1C", &program)
    }

    /// An sfnt holding a name-keyed `CFF ` table — the serif face this program ships, a bare CFF
    /// whose Top DICT uses no `CIDFont` operators — an empty `cmap`, and the version 0.5 `maxp`
    /// a CFF-flavoured program states its glyph count in.
    fn a_name_keyed_open_type_program() -> Vec<u8> {
        let (cff, format) = pdf_font::standard::face(pdf_font::substitute::Request {
            family: pdf_font::substitute::Family::Serif,
            bold: false,
            italic: false,
            standard: true,
        });
        let glyphs = pdf_font::embedding::glyph_count(cff, format).expect("the face counts");
        let mut maxp = vec![0, 0, 0x50, 0];
        maxp.extend_from_slice(&glyphs.to_be_bytes());
        let start = 12_u32 + 16 * 3;
        let length = u32::try_from(cff.len()).expect("the shipped face is under 4 GiB");
        let padded = length.next_multiple_of(4);
        let mut program = b"OTTO".to_vec();
        program.extend_from_slice(&[0, 3, 0, 0, 0, 0, 0, 0]);
        for (tag, at, length) in [
            (b"CFF ", start, length),
            (b"cmap", start.saturating_add(padded), 0),
            (b"maxp", start.saturating_add(padded), 6),
        ] {
            program.extend_from_slice(tag);
            program.extend_from_slice(&[0; 4]);
            program.extend_from_slice(&at.to_be_bytes());
            program.extend_from_slice(&length.to_be_bytes());
        }
        program.extend_from_slice(cff);
        program.resize(program.len().next_multiple_of(4), 0);
        program.extend_from_slice(&maxp);
        program
    }

    /// A one-page document showing one font of `dictionary` whose descriptor files `program` under
    /// `key`, its stream dictionary stating `stated` besides.
    fn a_program_under(dictionary: &str, key: &str, stated: &str, program: &[u8]) -> Document {
        a_program_beside(dictionary, key, stated, program, "/F", None)
    }

    /// The same, with the descriptor's `/FontName` given and, where `extra` is one, an object 9
    /// written beside the rest.
    fn a_program_beside(
        dictionary: &str,
        key: &str,
        stated: &str,
        program: &[u8],
        font_name: &str,
        extra: Option<String>,
    ) -> Document {
        let hex = program.iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02X}");
            hex
        });
        let data = format!("{hex}>");
        // A composite font's descendant is an object of its own, object 8, as a producer writes
        // one; `dictionary` names its subtype and any entries beside it.
        let descendant = dictionary.starts_with("CIDFontType").then(|| {
            format!(
                "<< /Type /Font /Subtype /{dictionary} /BaseFont /F /CIDSystemInfo << /Registry \
                 (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 6 0 R >>"
            )
        });
        let font = if descendant.is_some() {
            "<< /Type /Font /Subtype /Type0 /BaseFont /F /Encoding /Identity-H /DescendantFonts \
             [8 0 R] >>"
                .to_owned()
        } else {
            format!(
                "<< /Type /Font /Subtype /{dictionary} /BaseFont /F /FirstChar 104 /LastChar 105 \
                 /Widths [500 500] /FontDescriptor 6 0 R >>"
            )
        };
        let content = "BT /F1 12 Tf (hi) Tj ET";
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 \
             0 R >> >> /Contents 4 0 R >>"
                .to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
            font,
            "<< /Type /FontDescriptor /FontName {name} /Flags 32 /FontBBox [0 0 1 1] \
             /ItalicAngle 0 /Ascent 1 /Descent 0 /CapHeight 1 /StemV 1 /{key} 7 0 R >>"
                .replace("{key}", key)
                .replace("{name}", font_name),
            format!(
                "<< {stated} /Length1 9 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{data}\n\
                 endstream",
                data.len()
            ),
        ];
        objects.extend(descendant);
        objects.extend(extra);
        let size = objects.len().saturating_add(1);
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let at = out.len();
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture opens")
    }

    #[test]
    fn a_misnamed_program_is_given_the_name_of_its_format() {
        // Table 124's `OpenType` row admits a `glyf` program under a TrueType dictionary, so the
        // stream's `Type1C` becomes `OpenType` under the key it is already filed by (ADR 2307).
        let document = a_misnamed_glyf_program_under("TrueType");
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        let refiled = refiled(&document, &input, &mut spare()).expect("a row admits it");
        assert_eq!(
            refiled
                .at
                .values()
                .map(|row| (row.key, row.subtype))
                .collect::<Vec<_>>(),
            vec![("FontFile3", Some("OpenType"))]
        );
    }

    #[test]
    fn a_compact_program_under_the_type_1_key_moves_to_font_file3() {
        // A bare CFF's header under a `CIDFontType0` dictionary is Table 124's `CIDFontType0C` row,
        // which only `/FontFile3` carries: the descriptor's key moves, and the stream loses the
        // lengths §9.9 says a compact program shall not state.
        let document = a_program_under("CIDFontType0", "FontFile", "", &[1, 0, 4, 1, 0, 0, 0, 0]);
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        let refiled = refiled(&document, &input, &mut spare()).expect("a row admits it");
        assert_eq!(
            refiled.moved.values().copied().collect::<Vec<_>>(),
            vec!["FontFile"]
        );
        assert_eq!(refiled.moved_programs.len(), 1);
        assert_eq!(
            refiled
                .at
                .values()
                .map(|row| (row.key, row.subtype))
                .collect::<Vec<_>>(),
            vec![("FontFile3", Some("CIDFontType0C"))]
        );
    }

    #[test]
    fn a_program_no_row_admits_keeps_its_refusal() {
        // No `FontFile3` row admits a `glyf` program under a `CIDFontType0` dictionary.
        let document = a_misnamed_glyf_program_under("CIDFontType0");
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        assert!(
            matches!(refiled(&document, &input, &mut spare()), Err(Because::NotBuiltYet(why)) if why == NO_ROW)
        );
    }

    #[test]
    fn an_identity_mapped_cid_font_type2_over_a_name_keyed_cff_becomes_cid_font_type0() {
        // Table 115's Identity map gives each CID its own number as the glyph index, and §9.7.4.2
        // uses a CID "directly" as the glyph index of a name-keyed CFF under CIDFontType0, so the
        // dictionary is retyped, its program moved to Table 124's `OpenType` row (ADR 2326).
        let document = a_program_under(
            "CIDFontType2 /CIDToGIDMap /Identity",
            "FontFile2",
            "",
            &a_name_keyed_open_type_program(),
        );
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        let refiled =
            refiled(&document, &input, &mut spare()).expect("the same glyphs under CIDFontType0");
        assert_eq!(
            refiled
                .retyped
                .iter()
                .map(|id| id.number)
                .collect::<Vec<_>>(),
            vec![8]
        );
        assert_eq!(
            refiled.moved.values().copied().collect::<Vec<_>>(),
            vec!["FontFile2"]
        );
        assert_eq!(
            refiled
                .at
                .values()
                .map(|row| (row.key, row.subtype))
                .collect::<Vec<_>>(),
            vec![("FontFile3", Some("OpenType"))]
        );
    }

    #[test]
    fn a_cid_font_type2_whose_cids_are_not_its_glyph_indices_keeps_its_refusal() {
        // No Identity map: under CIDFontType0 each CID would select the glyph of its own number,
        // which nothing says it selected before.
        let document = a_program_under(
            "CIDFontType2",
            "FontFile2",
            "",
            &a_name_keyed_open_type_program(),
        );
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        assert!(matches!(
            refiled(&document, &input, &mut spare()),
            Err(Because::NotBuiltYet(why)) if why == NOT_THE_SAME_GLYPHS
        ));
    }

    /// A `/CIDToGIDMap` stream sending CID 0x6869, the fixture's one code `hi` under
    /// `Identity-H`, to glyph 3 and every other CID to glyph 0, written as object 9.
    fn a_map_sending_hi_to_glyph_three() -> String {
        let mut hex = "0000".repeat(0x6869);
        hex.push_str("0003>");
        format!(
            "<< /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream",
            hex.len()
        )
    }

    #[test]
    fn a_stream_mapped_cid_font_type2_over_a_name_keyed_cff_is_re_keyed_to_its_glyph_indices() {
        // Table 115's stream sends CID 0x6869 to glyph 3, and §9.7.4.2 makes a name-keyed CFF's
        // CID its glyph index under CIDFontType0, so the encoding sends code 0x6869 to CID 3 and
        // `/W` states glyph 3's width where it stated CID 0x6869's (ADR 2342).
        let document = a_program_beside(
            "CIDFontType2 /CIDToGIDMap 9 0 R /W [26729 [321]]",
            "FontFile2",
            "",
            &a_name_keyed_open_type_program(),
            "/F",
            Some(a_map_sending_hi_to_glyph_three()),
        );
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        let refiled = refiled(&document, &input, &mut spare()).expect("re-keyed");
        let descendant = pdf_syntax::object::ObjectId::new(8, 0);
        assert!(refiled.retyped.contains(&descendant));
        let rekeyed = refiled
            .rekeyed
            .get(&descendant)
            .expect("its CIDs are re-keyed");
        // Every other code drew glyph 0 at `/DW`'s default of 1000 (Table 115).
        assert_eq!(
            format!("{:?}", rekeyed.widths),
            "[Integer(0), Array([Integer(1000)]), Integer(3), Array([Integer(321)])]"
        );
        let cmap = refiled
            .encodings
            .get(&pdf_syntax::object::ObjectId::new(5, 0))
            .expect("the Type0 font names a CMap");
        let Some(Object::Stream(stream)) = rekeyed.written.get(cmap) else {
            panic!("the CMap is written");
        };
        let program = document.decoded_stream_data(stream).expect("it decodes");
        let program = String::from_utf8_lossy(&program);
        assert!(program.contains("1 begincidrange\n<6869> <6869> 3\nendcidrange"));
        assert!(program.contains("<0000> <FFFF> 0\nendnotdefrange"));
    }

    #[test]
    fn a_re_keyed_font_whose_descriptor_states_cid_set_keeps_its_refusal() {
        // `/CIDSet`'s bits are CIDs, which the re-keying changes, and it is not re-keyed.
        let document = a_program_beside(
            "CIDFontType2 /CIDToGIDMap 9 0 R",
            "FontFile2",
            "",
            &a_name_keyed_open_type_program(),
            // Any stream stands for the set: its presence is what refuses.
            "/F /CIDSet 9 0 R",
            Some(a_map_sending_hi_to_glyph_three()),
        );
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        assert!(matches!(
            refiled(&document, &input, &mut spare()),
            Err(Because::NotBuiltYet(why)) if why.contains("CIDSet")
        ));
    }

    /// A `ttcf` collection of the given faces, each with its own copy of its tables.
    fn a_collection_of(faces: &[&[u8]]) -> Vec<u8> {
        let count = u32::try_from(faces.len()).expect("a few faces");
        let mut out = b"ttcf".to_vec();
        out.extend_from_slice(&[0, 1, 0, 0]);
        out.extend_from_slice(&count.to_be_bytes());
        let mut at = faces.len().saturating_mul(4).saturating_add(12);
        for face in faces {
            out.extend_from_slice(&u32::try_from(at).expect("small").to_be_bytes());
            at = at.saturating_add(face.len().next_multiple_of(4));
        }
        for face in faces {
            let shift = u32::try_from(out.len()).expect("small");
            let mut face = face.to_vec();
            let tables = usize::from(u16::from_be_bytes([face[4], face[5]]));
            for index in 0..tables {
                let field = index.saturating_mul(16).saturating_add(20);
                let offset = face
                    .get_mut(field..field.saturating_add(4))
                    .expect("a record");
                let moved = u32::from_be_bytes((&*offset).try_into().expect("four bytes"))
                    .saturating_add(shift);
                offset.copy_from_slice(&moved.to_be_bytes());
            }
            out.extend_from_slice(&face);
            out.resize(out.len().next_multiple_of(4), 0);
        }
        out
    }

    /// Two of the shipped Liberation faces: regular, then bold.
    fn two_sans_faces() -> [&'static [u8]; 2] {
        let face = |bold| {
            pdf_font::standard::face(pdf_font::substitute::Request {
                family: pdf_font::substitute::Family::SansSerif,
                bold,
                italic: false,
                standard: true,
            })
            .0
        };
        [face(false), face(true)]
    }

    #[test]
    fn a_font_collection_is_written_out_as_the_face_its_descriptor_names() {
        // Table 124's `FontFile2` row asks one TrueType font, and the descriptor names the second
        // face of two by its PostScript name behind a subset tag (ADR 0141), so that face is
        // written out, its tables the face's own and no other's (ADR 2343).
        let [regular, bold] = two_sans_faces();
        let document = a_program_beside(
            "TrueType",
            "FontFile2",
            "",
            &a_collection_of(&[regular, bold]),
            "/ABCDEF+LiberationSans-Bold",
            None,
        );
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        let refiled = refiled(&document, &input, &mut spare()).expect("the face is written out");
        let Some(Object::Stream(stream)) = refiled.faces.values().next() else {
            panic!("the program is a face now");
        };
        let face = document.decoded_stream_data(stream).expect("it decodes");
        assert_eq!(
            stream.dict.get("Length1").and_then(Object::as_integer),
            i64::try_from(face.len()).ok()
        );
        assert_eq!(
            pdf_font::embedding::with_tables(&face, &[]),
            pdf_font::embedding::with_tables(bold, &[]),
            "the face is the bold one, table for table"
        );
    }

    #[test]
    fn a_font_collection_none_of_whose_faces_the_descriptor_names_keeps_its_refusal() {
        // Face zero is what this reader draws when no name matches, and it is a choice the
        // document did not make, so it is not written into the archive (ADR 2343).
        let [regular, bold] = two_sans_faces();
        let document = a_program_beside(
            "TrueType",
            "FontFile2",
            "",
            &a_collection_of(&[regular, bold]),
            "/Helvetica",
            None,
        );
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        assert!(matches!(
            refiled(&document, &input, &mut spare()),
            Err(Because::NotBuiltYet(why)) if why == A_COLLECTION
        ));
    }

    #[test]
    fn a_font_collection_is_in_no_row_and_is_refused_by_name() {
        // A `ttcf` header opens no font directory the TrueType Reference Manual describes, so the
        // validator files it under no key and the conversion names the collection (ADR 2327).
        let mut program = b"ttcf".to_vec();
        program.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 16]);
        let document = a_program_under("TrueType", "FontFile2", "", &program);
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        assert!(
            input
                .failures()
                .any(|judgement| judgement.id == "fonts/font-programs-filed-by-their-format")
        );
        assert!(matches!(
            refiled(&document, &input, &mut spare()),
            Err(Because::NotBuiltYet(why)) if why == A_COLLECTION
        ));
    }
}
