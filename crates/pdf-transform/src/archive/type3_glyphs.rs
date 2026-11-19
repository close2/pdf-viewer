//! ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 for a Type 3 font: a code
//! a content stream shows that its font describes no glyph for is given a glyph procedure that
//! paints nothing, at the width the font already states for it (`doc/adr/2918`).
//!
//! # What the clauses say
//!
//! The embedding subclause asks an embedded font to define every glyph a file references for
//! rendering. A Type 3 font's glyph descriptions are its `/CharProcs` streams, and ISO 32000-2
//! §9.6.4 states the two steps a code takes to one: the encoding names the glyph, and
//!
//! > b) Look up the glyph name in the font's CharProcs dictionary to obtain a stream object
//! > containing a glyph description. If the name is not present as a key in CharProcs , no glyph
//! > shall be painted.
//!
//! §9.6.5.3 makes the mapping "entirely defined by its Encoding entry", so a code the
//! `/Differences` array names no glyph for reaches none either. Both cases are therefore defined
//! by the base standard: nothing is painted, and §9.2.4 still advances the text position by the
//! font's `/Widths` entry.
//!
//! # Why the edit moves no mark
//!
//! The procedure given is `w 0 d0` and nothing after it: Table 111's `d0` states the glyph's
//! width and permits the description to paint nothing, and `w` is the font's own `/Widths` entry,
//! which is what §9.2.4 already lays the page out by, and for a code outside `/FirstChar` to
//! `/LastChar` the 0 Table 110 gives it — so ISO 19005-4 section 6.2.10.5's width is met in the
//! same edit. The page draws exactly what §9.6.4 b) defines, before and after; this
//! is ADR 2500's empty Type 3 glyph, given to a font the file already holds. It is no loss,
//! because nothing the producer wrote is removed: the encoding gains names and `/CharProcs` gains
//! entries, and every existing entry stays as it was.
//!
//! # The names
//!
//! A code its encoding already names keeps that name, and the name is given the procedure, so
//! what the code extracts to is unchanged. A code with no name is given one in `/Differences`:
//! `empty` for the first width the font needs, then `empty.1`, `empty.2`, … — the first that
//! neither the encoding nor `/CharProcs` already uses. The standard defines no name for this
//! (§9.6.4's names mean nothing outside the font), so the spelling is a choice, and the one made
//! is a name §9.10.2's methods find nothing for in the Adobe Glyph List, as the code had none
//! before; it is the name `super::reader_resources`' empty font uses.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_archive::survey::Survey;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::decision::Because;
use super::prepare::Spare;

/// The requirement this answers for a Type 3 font.
pub(super) const SITE: &str = "fonts/embedded-programs-define-every-glyph-shown";

/// The stem of every name this gives a code its encoding names no glyph for.
const EMPTY: &str = "empty";

/// Why a Type 3 font's undescribed code is not given a glyph.
const NOT_DESCRIBED: &str = "a Type 3 font in this file shows a code its encoding names no \
     glyph for, or names a glyph its CharProcs does not describe, and ISO 19005 requires every \
     glyph a file renders to be defined. ISO 32000-2 \u{a7}9.6.4 paints nothing for such a code, so \
     this conversion gives it a glyph procedure that paints nothing at the font's own width; here \
     the font's encoding, CharProcs or Widths could not be read as \u{a7}9.6.4 states them, the \
     font is written directly in a resource dictionary, its shown codes were more than the survey \
     kept, two shown codes sharing one undescribed name have two widths, or no object number was \
     free for the procedure";

/// Why a font another Type 3 rewrite also writes keeps the refusal: both write its `/Encoding`
/// and `/CharProcs`, and one would undo the other.
pub(super) const ALSO_ANOTHER_REWRITE: &str = "a Type 3 font in this file shows a code it \
     describes no glyph for, and this conversion gives such a code a glyph procedure that paints \
     nothing at the font's width; the same font also has its .notdef glyph renamed or a glyph \
     procedure's width restated, each of which writes the font's Encoding and CharProcs too, and \
     this version does not write the two edits together";

/// What a conversion writes for the row: per font, its `/Encoding` and `/CharProcs`; and the
/// procedures, one per width.
#[derive(Debug, Default)]
pub(super) struct Described {
    /// Per font object, in the source's numbering, the `/Encoding` and `/CharProcs` to write.
    pub(super) at: BTreeMap<ObjectId, (Object, Object)>,
    /// The procedures painting nothing, at spare object numbers.
    pub(super) written: BTreeMap<ObjectId, Object>,
}

impl Described {
    /// The fonts written, for another Type 3 rewrite to keep clear of.
    pub(super) fn fonts(&self) -> impl Iterator<Item = &ObjectId> {
        self.at.keys()
    }
}

/// Every Type 3 font the requirement names, given a procedure for each code it shows and does
/// not describe.
///
/// # Errors
///
/// [`super::decision::THE_GLYPH_IS_NOT_IN_THE_PROGRAM`] behind ADR 0816's fence where a font the
/// requirement names is not a Type 3 one, so that the row's other answer is tried; and
/// [`NOT_DESCRIBED`] where a Type 3 font cannot be given its glyphs.
pub(super) fn describe(
    document: &Document,
    input: &pdf_archive::Report,
    survey: Option<&Survey>,
    spare: &mut Spare,
) -> Result<Described, Because> {
    let refused = Because::NotBuiltYet(NOT_DESCRIBED);
    let (named, elsewhere) = super::sites::objects_named(input, &[SITE])?;
    if elsewhere {
        return Err(refused);
    }
    let survey = survey.ok_or(refused)?;
    let mut out = Described::default();
    let mut procedures_at: BTreeMap<String, ObjectId> = BTreeMap::new();
    for id in named {
        let resolved = document.get(id);
        let font = super::sites::dictionary_of(&resolved).ok_or(refused)?;
        if document
            .get_key(font, "Subtype")
            .as_name()
            .map(Name::as_bytes)
            != Some(&b"Type3"[..])
        {
            return Err(Because::TheFence(
                super::decision::THE_GLYPH_IS_NOT_IN_THE_PROGRAM,
            ));
        }
        let shown = shown_codes(survey, id).ok_or(refused)?;
        let (encoding, procedures) =
            given_glyphs(document, font, &shown, spare, &mut procedures_at, &mut out)
                .ok_or(refused)?;
        out.at.insert(id, (encoding, procedures));
    }
    Ok(out)
}

/// Every code a content stream rendered in the font `id`, or `None` where the survey kept only
/// part of one selection's strings.
///
/// A font shared by several resource dictionaries is selected once for each, so the codes are
/// the union over its selections.
fn shown_codes(survey: &Survey, id: ObjectId) -> Option<BTreeSet<u8>> {
    let mut codes = BTreeSet::new();
    for used in survey.fonts() {
        if used.id != Some(id) || !used.rendered {
            continue;
        }
        if !used.shown_complete {
            return None;
        }
        // A Type 3 font is a simple font, so each byte a string shows is one code (§9.6.4).
        codes.extend(used.shown.keys().flatten().copied());
    }
    Some(codes)
}

/// The `/Encoding` and `/CharProcs` one font is to state, every shown code it did not describe
/// given a procedure at its `/Widths` entry; `None` where §9.6.4's entries cannot be read, a
/// width is not stated, or a name would need two widths.
fn given_glyphs(
    document: &Document,
    font: &Dictionary,
    shown: &BTreeSet<u8>,
    spare: &mut Spare,
    procedures_at: &mut BTreeMap<String, ObjectId>,
    out: &mut Described,
) -> Option<(Object, Object)> {
    let Object::Dictionary(mut encoding) = document.get_key(font, "Encoding") else {
        return None;
    };
    let mut differences: Vec<Object> = match document.get_key(&encoding, "Differences") {
        Object::Array(items) => items.iter().map(|item| document.resolve(item)).collect(),
        Object::Null => Vec::new(),
        _ => return None,
    };
    let mut procedures = document.get_key(font, "CharProcs").as_dict().cloned()?;
    let first = document.get_key(font, "FirstChar").as_integer()?;
    let widths = document.get_key(font, "Widths");
    let widths = widths.as_array()?;
    let names: BTreeMap<i64, String> = pdf_archive::type3_encoding(document, font)
        .into_iter()
        .collect();
    // The width each name is given, in §7.3.3's spelling, so that a name two codes share is given
    // one only where they state the same number; and the fresh name each width's unnamed codes
    // are given.
    let mut given: BTreeMap<String, String> = BTreeMap::new();
    let mut fresh: BTreeMap<String, String> = BTreeMap::new();
    let mut appended: Vec<Object> = Vec::new();
    for &byte in shown {
        let code = i64::from(byte);
        // Table 110: "For character codes outside the range FirstChar to LastChar, the width
        // shall be 0", which is the width this reader lays such a code out by (ADR 3110).
        let width = match code
            .checked_sub(first)
            .and_then(|at| usize::try_from(at).ok())
            .and_then(|at| widths.get(at))
        {
            Some(width) => document.resolve(width).as_number()?,
            None => 0.0,
        };
        let spelled = super::type3_widths::spelled(width);
        let name = if let Some(name) = names.get(&code) {
            let described = procedures
                .get(name.as_str())
                .is_some_and(|at| matches!(document.resolve(at), Object::Stream(_)));
            if described {
                continue;
            }
            name.clone()
        } else {
            let name = if let Some(name) = fresh.get(&spelled) {
                name.clone()
            } else {
                let name = unused_name(&procedures, &differences, &given)?;
                fresh.insert(spelled.clone(), name.clone());
                name
            };
            appended.push(Object::Integer(code));
            appended.push(Object::Name(Name::new(name.as_bytes())));
            name
        };
        match given.get(&name) {
            Some(stated) if *stated != spelled => return None,
            Some(_) => continue,
            None => {}
        }
        given.insert(name.clone(), spelled.clone());
        let at = if let Some(at) = procedures_at.get(&spelled) {
            *at
        } else {
            let at = spare.take(document)?;
            out.written.insert(at, painting_nothing(&spelled));
            procedures_at.insert(spelled, at);
            at
        };
        procedures.insert(Name::new(name.as_bytes()), Object::Reference(at));
    }
    // Each appended pair is a code with no name before it, so no code is named twice (§9.6.5.1).
    differences.extend(appended);
    encoding.insert(Name::new(&b"Differences"[..]), Object::Array(differences));
    Some((Object::Dictionary(encoding), Object::Dictionary(procedures)))
}

/// The first of `empty`, `empty.1`, `empty.2`, … that neither `/CharProcs`, the `/Differences`
/// array nor a name already given uses.
fn unused_name(
    procedures: &Dictionary,
    differences: &[Object],
    given: &BTreeMap<String, String>,
) -> Option<String> {
    let taken = |candidate: &str| {
        procedures.get(candidate).is_some()
            || given.contains_key(candidate)
            || differences.iter().any(|item| {
                item.as_name()
                    .is_some_and(|name| name.as_bytes() == candidate.as_bytes())
            })
    };
    // One more candidate than there are names in the three places, so one of them is free.
    let bound = procedures
        .len()
        .saturating_add(differences.len())
        .saturating_add(given.len());
    std::iter::once(EMPTY.to_owned())
        .chain((1..=bound).map(|n| format!("{EMPTY}.{n}")))
        .find(|candidate| !taken(candidate))
}

/// A glyph procedure stating `width` and painting nothing (ISO 32000-2 §9.6.4, Table 111's `d0`).
fn painting_nothing(width: &str) -> Object {
    let data = format!("{width} 0 d0\n").into_bytes();
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}
