//! A code of an embedded Compact Font Format program that reaches `.notdef`, or no glyph, given a
//! glyph of its own by writing the program.
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator to
//! reference the `.notdef` glyph, and ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section
//! 6.2.10.4.1 require the embedded program to define every glyph a page shows. A simple font whose
//! `/FontFile3` is a bare CFF (`/Type1C`) fails both where a shown code's name is one the program's
//! charset does not hold, or where the encoding in force names nothing, or `.notdef`, for it —
//! under `WinAnsiEncoding` the line feed, code 10, is the commonest. ISO 32000-2 §9.6.5.2 is the
//! route such a code takes:
//!
//! > The Differences array maps codes to the names of glyph descriptions that exist in the font
//! > program, regardless of whether those glyphs are referenced by the font's built -in encoding or
//! > by the encoding specified in the BaseEncoding entry.
//!
//! **What this reader drew is what the archive keeps** (`doc/questions/A694`, item 3): a code on
//! glyph 0 drew the program's `.notdef` outline, so it is given a copy of that charstring; a code
//! that reached no glyph drew nothing, so it is given a glyph that draws nothing. Each glyph's
//! advance is the width the dictionary states for its codes, which ISO 19005-2 section 6.2.11.5
//! holds the program to, so codes of two widths take two glyphs (`doc/adr/2543` section 2).
//!
//! **A code keeps the name the file gave it.** Where the encoding names a glyph the program lacks,
//! the new glyph carries that name and the font dictionary is not touched, so §9.10.2's glyph-name
//! method reads the character the producer wrote. Where the encoding names nothing or `.notdef`,
//! the glyph is named `notdef`, `notdef.1`, … — outside the Adobe Glyph List, so the code is given
//! no character it did not have (`doc/questions/A48`'s condition) — and the font's `/Encoding` is
//! restated with a `/Differences` entry naming it. The program is written by
//! `pdf_font::cff::with_glyphs_given`, and the font is loaded again before anything is kept:
//! every code draws the outline it drew before, each code given a glyph reaches that glyph, and
//! that glyph's advance agrees with the code's width (`doc/adr/2578`).

use std::collections::BTreeMap;

use pdf_font::cff::{GivenOutline, GlyphToGive};
use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::prepare::Spare;
use super::super::sites::{dictionary_of, loaded_as_stated};

/// The objects one simple CFF font's glyphs are given by.
#[derive(Debug)]
pub(in crate::archive) struct CffGlyphsGiven {
    /// The descriptor the font names from now on.
    pub(in crate::archive) descriptor: ObjectId,
    /// The `/Encoding` the font states from now on, where a code needed a name of its own.
    pub(in crate::archive) encoding: Option<Object>,
    /// The new objects: the program and the descriptor.
    pub(in crate::archive) written: Vec<(ObjectId, Object)>,
}

/// Whether a font dictionary is a simple `Type1` font whose descriptor embeds a bare CFF program
/// as `/FontFile3` of `/Subtype /Type1C` — the font [`given`] can carry.
pub(in crate::archive) fn is_simple_cff(document: &Document, dict: &Dictionary) -> bool {
    program_of(document, dict).is_some()
}

/// The font's descriptor and the reference to its program, for a font [`is_simple_cff`] admits.
fn program_of(document: &Document, dict: &Dictionary) -> Option<(Dictionary, ObjectId)> {
    if document.get_key(dict, "Subtype").as_name()?.as_bytes() != b"Type1" {
        return None;
    }
    let resolved = document.resolve(dict.get("FontDescriptor")?);
    let descriptor = dictionary_of(&resolved)?.clone();
    let at = descriptor.get("FontFile3")?.as_reference()?;
    let Object::Stream(stream) = document.get(at) else {
        return None;
    };
    (document
        .get_key(&stream.dict, "Subtype")
        .as_name()?
        .as_bytes()
        == b"Type1C")
        .then_some((descriptor, at))
}

/// What one shown code is given: the outline it is to draw, the name it is reached by (`None`
/// for a fresh one the encoding is restated to name), and its advance in the program's units.
type Asked = (GivenOutline, Option<String>, i64);

/// What one shown code needs.
enum Needs {
    /// Nothing: it reaches a glyph other than `.notdef`.
    Reached,
    /// A glyph of its own.
    Given(Asked),
}

/// The glyphs a simple CFF font's shown codes on `.notdef`, or on no glyph, are given, with
/// `shown` the strings the pages show in it, proved; `None` where the font is not one
/// [`is_simple_cff`] admits, a code's width does not fit the program's units within ISO 19005-2
/// section 6.2.11.5's tolerance, codes of one name state two widths, the encoding cannot be
/// restated, the program cannot be written, or the font fails the proof.
///
/// `copied` is the running count of program bytes the conversion copies, which the caller holds
/// to `budget`; it grows by this program's length before anything is copied.
pub(in crate::archive) fn given<'a>(
    document: &Document,
    dict: &Dictionary,
    shown: impl Iterator<Item = &'a [u8]>,
    (spare, programs, copied): (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    budget: usize,
) -> Option<CffGlyphsGiven> {
    let (descriptor, program_at) = program_of(document, dict)?;
    let before = loaded_as_stated(document, dict)?;
    if before.is_substituted() {
        return None;
    }
    // A simple font's code is one byte (§9.6.5), so at most 256 distinct codes are read here
    // however many strings the pages show.
    let mut codes = [false; 256];
    for text in shown {
        for byte in text {
            codes[usize::from(*byte)] = true;
        }
    }
    let Object::Stream(stream) = programs
        .get(&program_at)
        .cloned()
        .unwrap_or_else(|| document.get(program_at))
    else {
        return None;
    };
    let program = document.decoded_stream_data(&stream)?;
    *copied = copied.saturating_add(program.len());
    if *copied > budget {
        return None;
    }
    let units = pdf_font::cff::units_per_em(&program).ok()?;

    // One glyph per name, or per outline and width for the codes a fresh name is written for.
    let mut asked: BTreeMap<(bool, Option<String>, i64), Vec<u8>> = BTreeMap::new();
    for byte in (0..=u8::MAX).filter(|byte| codes[usize::from(*byte)]) {
        let code = Code::single_byte(byte);
        let Needs::Given((draws, name, advance)) = asked_for(&before, code, units)? else {
            continue;
        };
        let key = (draws == GivenOutline::NotdefsOwn, name, advance);
        asked.entry(key).or_default().push(byte);
    }
    if asked.is_empty() {
        return None;
    }
    // A name reaches one glyph, so codes of one name at two widths cannot both be answered.
    let mut names = std::collections::BTreeSet::new();
    if asked
        .keys()
        .filter_map(|(_, name, _)| name.as_ref())
        .any(|name| !names.insert(name))
    {
        return None;
    }
    let glyphs: Vec<GlyphToGive> = asked
        .keys()
        .map(|(copy, name, advance)| GlyphToGive {
            draws: if *copy {
                GivenOutline::NotdefsOwn
            } else {
                GivenOutline::Nothing
            },
            name: name.clone(),
            advance: *advance,
        })
        .collect();
    let (rebuilt, added) = pdf_font::cff::with_glyphs_given(&program, &glyphs).ok()?;

    let mut fresh: BTreeMap<u8, String> = BTreeMap::new();
    let mut given: BTreeMap<u8, u16> = BTreeMap::new();
    for (((_, name, _), bytes), glyph) in asked.iter().zip(&added) {
        for byte in bytes {
            given.insert(*byte, glyph.glyph);
            if name.is_none() {
                fresh.insert(*byte, glyph.name.clone());
            }
        }
    }
    let encoding = if fresh.is_empty() {
        None
    } else {
        Some(encoding_restated(document, dict, &fresh)?)
    };
    let new_program = super::program_stream(Some(&stream.dict), &rebuilt, false)?;
    let mut new_descriptor = descriptor;
    with_names_listed(document, &mut new_descriptor, &added);

    // The proof reads the font as the conversion will write it, from objects held in memory.
    let mut proof_descriptor = new_descriptor.clone();
    proof_descriptor.insert(Name::new(&b"FontFile3"[..]), new_program.clone());
    let mut candidate = dict.clone();
    candidate.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Dictionary(proof_descriptor),
    );
    if let Some(encoding) = &encoding {
        candidate.insert(Name::new(&b"Encoding"[..]), encoding.clone());
    }
    let after = LoadedFont::load(document, &candidate, "").ok()?;
    if !proved(&before, &after, &given) {
        return None;
    }

    let program_id = spare.take(document)?;
    let descriptor_id = spare.take(document)?;
    new_descriptor.insert(Name::new(&b"FontFile3"[..]), Object::Reference(program_id));
    Some(CffGlyphsGiven {
        descriptor: descriptor_id,
        encoding,
        written: vec![
            (program_id, new_program),
            (descriptor_id, Object::Dictionary(new_descriptor)),
        ],
    })
}

/// The descriptor's `/CharSet`, where it states one, with the added glyphs' names appended:
/// ISO 19005-2 section 6.2.11.4.2 holds a stated `CharSet` to list every glyph the program holds.
fn with_names_listed(
    document: &Document,
    descriptor: &mut Dictionary,
    added: &[pdf_font::cff::AddedGlyph],
) {
    let Some(Object::String(set)) = descriptor.get("CharSet").map(|set| document.resolve(set))
    else {
        return;
    };
    let mut listed = set.to_vec();
    for glyph in added {
        listed.push(b'/');
        listed.extend_from_slice(glyph.name.as_bytes());
    }
    descriptor.insert(Name::new(&b"CharSet"[..]), Object::String(listed.into()));
}

/// What `code` is to be given, and `None` where it cannot be given one: it drew a mark without reaching a glyph,
/// or its width is not a whole number of the program's units within the tolerance.
fn asked_for(font: &LoadedFont, code: Code, units: f32) -> Option<Needs> {
    let glyph = font.glyph_index(code);
    let draws = match glyph {
        Some(pdf_font::NOTDEF_GLYPH) => GivenOutline::NotdefsOwn,
        Some(_) => return Some(Needs::Reached),
        None if drawn(font, code).is_empty() => GivenOutline::Nothing,
        None => return None,
    };
    let exact = font.advance(code) * units;
    let whole = exact.round();
    // The tolerance both parts state between a dictionary's width and its program's.
    if (whole - exact).abs() / units > super::CONSISTENT || !(0.0..=32_767.0).contains(&whole) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an advance in font units, rounded and checked to lie within 0..=32 767 above"
    )]
    let advance = whole as i64;
    let name = font
        .selected_glyph_name(code)
        .filter(|name| *name != ".notdef")
        .map(str::to_owned);
    Some(Needs::Given((draws, name, advance)))
}

/// The outline this reader draws for `code`, as its path commands; empty where it draws none.
fn drawn(font: &LoadedFont, code: Code) -> Vec<pdf_render::PathCommand> {
    font.outline(code)
        .map(|outline| outline.commands().to_vec())
        .unwrap_or_default()
}

/// Whether the font as written draws, for every one of the 256 codes, the outline the font as read
/// drew; reaches, for each code of `given`, the glyph given it at an advance within the tolerance
/// of its width; and reaches, for every other code, the glyph it reached before.
fn proved(before: &LoadedFont, after: &LoadedFont, given: &BTreeMap<u8, u16>) -> bool {
    !after.is_substituted()
        && (0..=u8::MAX).all(|byte| {
            let code = Code::single_byte(byte);
            let glyph = match given.get(&byte) {
                Some(glyph) => {
                    let advance = after.program_advance(code).is_some_and(|program| {
                        (program - after.advance(code)).abs() <= super::CONSISTENT
                    });
                    advance && after.glyph_index(code) == Some(*glyph)
                }
                // A code no page shows may share a name with one given a glyph — `WinAnsiEncoding`
                // names `space` at 32 and at 160 — and so come to reach that glyph where it
                // reached none; what it draws is held below like every other code's.
                None => {
                    after.glyph_index(code) == before.glyph_index(code)
                        || before.glyph_index(code).is_none()
                            && after
                                .glyph_index(code)
                                .is_some_and(|glyph| given.values().any(|own| *own == glyph))
                }
            };
            glyph
                && drawn(after, code) == drawn(before, code)
                && (after.advance(code) - before.advance(code)).abs() <= f32::EPSILON
        })
}

/// The font's `/Encoding` with each code of `fresh` named by its new glyph's name.
///
/// A predefined encoding becomes the `/BaseEncoding` of a dictionary (Table 112 admits
/// `MacRomanEncoding`, `MacExpertEncoding` and `WinAnsiEncoding` there); an absent one becomes a
/// dictionary with no base, which Table 112 reads against the embedded program's built-in
/// encoding exactly as the absent entry was read; a dictionary keeps its base and has its
/// `/Differences` restated, every code it named keeping its name unless `fresh` names it. `None`
/// for an encoding stated any other way.
fn encoding_restated(
    document: &Document,
    dict: &Dictionary,
    fresh: &BTreeMap<u8, String>,
) -> Option<Object> {
    let mut out = Dictionary::new();
    out.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Encoding"[..])),
    );
    let mut names: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
    match dict
        .get("Encoding")
        .map(|encoding| document.resolve(encoding))
    {
        None => {}
        Some(Object::Name(base)) => {
            if !matches!(
                base.as_bytes(),
                b"MacRomanEncoding" | b"MacExpertEncoding" | b"WinAnsiEncoding"
            ) {
                return None;
            }
            out.insert(Name::new(&b"BaseEncoding"[..]), Object::Name(base));
        }
        Some(Object::Dictionary(stated)) => {
            if let Some(base) = stated.get("BaseEncoding") {
                out.insert(Name::new(&b"BaseEncoding"[..]), document.resolve(base));
            }
            if let Some(differences) = stated.get("Differences") {
                let differences = document.resolve(differences);
                let mut code: Option<i64> = None;
                for item in differences.as_array()? {
                    match document.resolve(item) {
                        Object::Integer(at) => code = Some(at),
                        Object::Name(name) => {
                            let at = code?;
                            names.insert(u8::try_from(at).ok()?, name.as_bytes().to_vec());
                            code = Some(at.saturating_add(1));
                        }
                        _ => return None,
                    }
                }
            }
        }
        Some(_) => return None,
    }
    for (code, name) in fresh {
        names.insert(*code, name.as_bytes().to_vec());
    }
    // One run per stretch of consecutive codes, which is the array's own form (§9.6.5.1).
    let mut differences = Vec::new();
    let mut next: Option<u8> = None;
    for (code, name) in &names {
        if next != Some(*code) {
            differences.push(Object::Integer(i64::from(*code)));
        }
        differences.push(Object::Name(Name::new(name.as_slice())));
        next = code.checked_add(1);
    }
    out.insert(Name::new(&b"Differences"[..]), Object::Array(differences));
    Some(Object::Dictionary(out))
}
