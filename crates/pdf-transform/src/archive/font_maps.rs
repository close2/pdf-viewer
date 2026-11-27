//! A font's maps that this reader read past, restated as it read them (ADR 3570).
//!
//! `fonts/font-maps-read-as-written` (ISO 19005-2 and ISO 19005-4 section 5.1) fails where the
//! reader's log holds one of three repairs `pdf_font` makes reading a font dictionary:
//!
//! - **a `/ToUnicode` that is not a stream** — ISO 32000-2 Table 109 and §9.10.3 give the entry
//!   as a stream holding a `CMap`, and this reader reads any other value as no map, so the entry
//!   is removed: the font then states no map, which is what was read;
//! - **an `/Encoding` or `/BaseEncoding` name the standard does not predefine** — Table 109 and
//!   Table 112 name the encodings an entry may name, and this reader reads any other name as no
//!   base encoding, the program's own standing (`pdf_font`'s `base_encoding`), so the name is
//!   removed;
//! - **a `/Differences` array with an element that is neither a code nor a glyph name in place**
//!   — §9.6.5.1 states the array's grammar, and this reader skips such an element, so the array
//!   is written again as the codes and names it assigned, in the order it assigned them;
//! - **a `/ToUnicode` `CMap` with entries that do not read** — §9.10.3 states the map's grammar,
//!   and this reader reads an entry outside it as no mapping, so the map is written again from the
//!   mappings it read, at the producer's object, and proved to answer every code as it answered
//!   ([`to_unicode`], ADR 3781).
//!
//! Each is checked before it is written: a simple font loaded from the dictionary as restated
//! draws every single-byte code's outline at the advance it drew from the dictionary as written,
//! and a Type 3 font read from it names every code's glyph as it did (ADR 3793). §9.6.4 selects a
//! Type 3 glyph by the name `/Encoding` gives the code, through `/CharProcs`, so the code to name
//! map unchanged is the glyph unchanged. Table 110 requires a Type 3 font's `/Encoding` and makes
//! it a dictionary, so a name this reader reads as no table is written as the dictionary that
//! assigns no code rather than removed.
//!
//! A font is reached as an object of its own and written directly inside another object, where
//! the rewrite edits it in its holder (ADR 3794).
//! What is lost is the producer's own words at those places, which no reader reads by ISO 32000-2
//! and some reader may read by a convention of its own — `/ToUnicode /Identity-H` read as the
//! identity map is the common one — so the restatement is the loss `font-maps-as-read`, and the
//! report names every font and entry. A damaged map's dropped entries are that loss too: each
//! is words of the producer's no reader reads by §9.10.3's UTF-16BE, and a reader with a
//! convention of its own may read one (an unpaired surrogate as U+FFFD is the common one).
//!
//! Refused by name ([`NOT_RESTATED`]): a font the proof does not hold for — a nonsymbolic
//! `TrueType` font's unknown `/Encoding` name is one, since this reader selects its glyphs by
//! another route once the entry is absent — and an encoding dictionary two fonts would read two
//! ways restated.
//!
//! **A nonsymbolic `TrueType` font naming the standard encoding is passed over** (ADR 3753): the
//! validator reports that name under `fonts/non-symbolic-truetype-uses-a-standard-encoding`
//! rather than this row, and `standard-truetype-encoding`'s rewrite restates it as one of the two
//! names ISO 19005 admits, proved code by code; removing it here would move the font onto the
//! route the proof above refuses.

use core::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use pdf_font::{Code, LoadedFont, PERMITTED_ENCODING_NAMES};
use pdf_model::type3::Type3Font;
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::decision::Because;

mod to_unicode;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const ROW: &str = "fonts/font-maps-read-as-written";

/// Why a font's maps are not restated as read.
pub(super) const NOT_RESTATED: &str = "a font dictionary states a ToUnicode that is not a \
     stream, an Encoding or BaseEncoding naming no encoding ISO 32000-2 predefines, or a \
     Differences array with an element out of its grammar, and this reader read past it (a \
     ToUnicode CMap whose entries do not all read is answered on its own terms); this \
     conversion restates such a font as it was read only where an encoding dictionary it \
     restates is read alike by every font naming it, and every code the font shows draws the \
     outline and the advance it drew, or for a Type 3 font names the glyph it named; this font is \
     not that";

/// The fonts and encoding dictionaries restated, and what each lost.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// Each dictionary written again, by its object: a font, or an encoding dictionary.
    pub(super) dictionaries: BTreeMap<ObjectId, Dictionary>,
    /// Each damaged `/ToUnicode` stream written again as read, by its own object.
    pub(super) streams: BTreeMap<ObjectId, Object>,
    /// The objects a font restated here is written directly inside, each font rewritten there by
    /// [`direct_as_read`] (ADR 3794).
    pub(super) holders: BTreeSet<ObjectId>,
    /// For the report, one line per entry restated.
    pub(super) entries: Vec<String>,
}

/// Every font the reader read a map of past, restated as read, or why one is not.
///
/// A font is reached where it is an object of its own stating `/Type /Font`, and where it is
/// written directly inside another object, found by its position as `pdf_archive`'s font
/// population finds it (`super::sites::fonts_inside_edited`).
///
/// # Errors
///
/// [`NOT_RESTATED`] where any font the log names is outside what the module comment lists.
pub(super) fn restated(document: &Document) -> Result<Restated, Because> {
    let mut restated = Restated::default();
    for number in document.xref().object_numbers() {
        let id = document.defined_id(number);
        let object = document.get(id);
        let Some(holder) = super::sites::dictionary_of(&object) else {
            continue;
        };
        if is_font(holder) {
            let label = format!("object {} {}", id.number, id.generation);
            restate_damaged_map(document, &label, holder, &mut restated)?;
            if let Some((written, lines)) = font_as_read(document, &label, holder, &mut restated)? {
                restated.entries.extend(lines);
                if written != *holder {
                    restated.dictionaries.insert(id, written);
                }
            }
        }
        restate_direct_fonts(document, id, holder, &mut restated)?;
    }
    // Every font naming a restated encoding dictionary is a font of this loop, so each was proved
    // drawing alike with it.
    Ok(restated)
}

/// The fonts written directly inside `holder`, restated as [`font_as_read`] restates a font of its
/// own; the holder is recorded where any of them changed, and the rewrite edits it there.
fn restate_direct_fonts(
    document: &Document,
    id: ObjectId,
    holder: &Dictionary,
    restated: &mut Restated,
) -> Result<(), Because> {
    let label = format!(
        "a font written directly inside object {} {}",
        id.number, id.generation
    );
    let found = RefCell::new((core::mem::take(restated), None::<Because>));
    let edit = |font: &Dictionary| {
        let (restated, failed) = &mut *found.borrow_mut();
        if failed.is_some() {
            return None;
        }
        let answer = restate_damaged_map(document, &label, font, restated)
            .and_then(|()| font_as_read(document, &label, font, restated));
        match answer {
            Ok(Some((written, lines))) => {
                restated.entries.extend(lines);
                (written != *font).then_some(written)
            }
            Ok(None) => None,
            Err(why) => {
                *failed = Some(why);
                None
            }
        }
    };
    let changed = super::sites::fonts_inside_edited(holder, &edit, &[]).is_some();
    let (taken, failed) = found.into_inner();
    *restated = taken;
    if let Some(why) = failed {
        return Err(why);
    }
    if changed {
        restated.holders.insert(id);
    }
    Ok(())
}

/// A font written directly inside a holder [`restated`] recorded, as it was restated there; `None`
/// where nothing in it was read past. The objects it names were restated on their own turn.
pub(super) fn direct_as_read(document: &Document, font: &Dictionary) -> Option<Dictionary> {
    let mut scratch = Restated::default();
    font_as_read(document, "", font, &mut scratch)
        .ok()
        .flatten()
        .map(|(written, _)| written)
}

/// Whether a dictionary states itself a font by Table 109's `/Type`.
fn is_font(dict: &Dictionary) -> bool {
    dict.get("Type")
        .and_then(Object::as_name)
        .map(Name::as_bytes)
        == Some(b"Font")
}

/// One font as this reader read its maps, and the report's lines, `label` naming it; `None`
/// where nothing in it was read past. An encoding dictionary of its own is recorded in
/// `restated` rather than in the font.
///
/// # Errors
///
/// [`NOT_RESTATED`] where the font does not draw alike restated, or an encoding dictionary two
/// fonts name would be restated two ways.
fn font_as_read(
    document: &Document,
    label: &str,
    font: &Dictionary,
    restated: &mut Restated,
) -> Result<Option<(Dictionary, Vec<String>)>, Because> {
    let refused = Because::NotBuiltYet(NOT_RESTATED);
    let mut written = font.clone();
    let mut lines = Vec::new();
    if let Some(stated) = font.get("ToUnicode")
        && document.resolve(stated).as_stream().is_none()
    {
        written.remove("ToUnicode");
        lines.push(format!(
            "{label}: ToUnicode {} removed",
            described(&document.resolve(stated))
        ));
    }
    let encoding_at = font.get("Encoding").and_then(Object::as_reference);
    let kind = Kind::of(document, font);
    let truetype = nonsymbolic_truetype(document, font);
    match document.resolve(font.get("Encoding").unwrap_or(&Object::Null)) {
        Object::Name(name) if kind == Kind::Simple && !predefined(&name, truetype) => {
            written.remove("Encoding");
            lines.push(format!(
                "{label}: Encoding /{} removed",
                String::from_utf8_lossy(name.as_bytes())
            ));
        }
        // Table 110 requires a Type 3 font's entry, and makes it a dictionary: a name this reader
        // reads as no table is written as the dictionary that assigns no code, which is what was
        // read (`pdf_model::type3`'s `encoding`).
        Object::Name(name) if kind == Kind::Type3 && !predefined(&name, false) => {
            let mut none = Dictionary::new();
            none.insert(
                Name::new(&b"Type"[..]),
                Object::Name(Name::new(&b"Encoding"[..])),
            );
            none.insert(Name::new(&b"Differences"[..]), Object::Array(Vec::new()));
            written.insert(Name::new(&b"Encoding"[..]), Object::Dictionary(none));
            lines.push(format!(
                "{label}: Encoding /{} written as an encoding dictionary assigning no code",
                String::from_utf8_lossy(name.as_bytes())
            ));
        }
        Object::Dictionary(encoding) => {
            if let Some(read) = encoding_as_read(document, &encoding, truetype, kind) {
                lines.push(format!("{label}: Encoding {}", read.1));
                match encoding_at {
                    Some(at) => {
                        if restated
                            .dictionaries
                            .get(&at)
                            .is_some_and(|before| *before != read.0)
                        {
                            return Err(refused);
                        }
                        restated.dictionaries.insert(at, read.0.clone());
                    }
                    None => {
                        written.insert(
                            Name::new(&b"Encoding"[..]),
                            Object::Dictionary(read.0.clone()),
                        );
                    }
                }
                if !draws_alike(document, font, &with_encoding(&written, read.0)) {
                    return Err(refused);
                }
            }
        }
        _ => {}
    }
    if lines.is_empty() {
        return Ok(None);
    }
    if !draws_alike(document, font, &written) {
        return Err(refused);
    }
    Ok(Some((written, lines)))
}

/// A `/ToUnicode` program as this reader read it: `data` itself where `read`, its reading, read
/// every entry, else the program [`restate_damaged_map`] writes at the stream's object. The one
/// route by which another rewrite of the map starts from what was read rather than from the
/// entries this reader read past (ADR 3794).
///
/// # Errors
///
/// [`to_unicode::NOT_WRITTEN`] where the damaged map is not written as read.
pub(super) fn map_as_read(
    data: &[u8],
    read: &pdf_font::tounicode::ToUnicode,
) -> Result<Vec<u8>, &'static str> {
    if read.damaged() {
        to_unicode::program_as_read(data, read)
    } else {
        Ok(data.to_vec())
    }
}

/// A font's `/ToUnicode` stream this reader read past entries of, written again as read at its own
/// object ([`to_unicode`]); a stream two fonts name is written once, its reading being the
/// stream's own.
///
/// # Errors
///
/// [`to_unicode::NOT_WRITTEN`] where the map's data does not decode or the rewrite is not proved.
fn restate_damaged_map(
    document: &Document,
    label: &str,
    font: &Dictionary,
    restated: &mut Restated,
) -> Result<(), Because> {
    let refused = Because::NotBuiltYet(to_unicode::NOT_WRITTEN);
    let Some(at) = font.get("ToUnicode").and_then(Object::as_reference) else {
        return Ok(());
    };
    let Object::Stream(stream) = document.get(at) else {
        return Ok(());
    };
    if restated.streams.contains_key(&at) {
        return Ok(());
    }
    let read = pdf_font::to_unicode(document, font);
    if !read.damaged() {
        return Ok(());
    }
    let data = document.decoded_stream_data(&stream).ok_or(refused)?;
    let written = to_unicode::written(&stream, &data, &read).map_err(Because::NotBuiltYet)?;
    let mut line = format!(
        "{label}: ToUnicode object {} {} written as read, {} entries read past dropped",
        at.number, at.generation, written.dropped_count
    );
    for dropped in &written.dropped {
        line.push_str("; ");
        line.push_str(dropped);
    }
    restated.entries.push(line);
    restated.streams.insert(at, written.stream);
    Ok(())
}

/// An encoding dictionary as this reader read it, and the line that says what changed; `None`
/// where nothing was read past. `truetype` is [`nonsymbolic_truetype`] of the font naming it, and
/// `kind` its [`Kind`].
fn encoding_as_read(
    document: &Document,
    encoding: &Dictionary,
    truetype: bool,
    kind: Kind,
) -> Option<(Dictionary, String)> {
    let mut written = encoding.clone();
    let mut changed = Vec::new();
    if let Object::Name(name) =
        document.resolve(encoding.get("BaseEncoding").unwrap_or(&Object::Null))
        && !predefined(&name, truetype)
    {
        written.remove("BaseEncoding");
        changed.push(format!(
            "BaseEncoding /{} removed",
            String::from_utf8_lossy(name.as_bytes())
        ));
    }
    match document.resolve(encoding.get("Differences").unwrap_or(&Object::Null)) {
        Object::Null => {}
        Object::Array(items) => {
            let read = differences_as_read(document, &items, kind == Kind::Type3);
            if read.len() != items.len()
                || read
                    .iter()
                    .zip(items.iter())
                    .any(|(left, right)| *left != document.resolve(right))
            {
                written.insert(Name::new(&b"Differences"[..]), Object::Array(read));
                changed.push("Differences written as the codes and names read".to_owned());
            }
        }
        other => {
            written.remove("Differences");
            changed.push(format!("Differences {} removed", described(&other)));
        }
    }
    (!changed.is_empty()).then(|| (written, changed.join(", ")))
}

/// A `/Differences` array as `pdf_font` reads it (§9.6.5.1): a code, then the names assigned to
/// it and the codes after it; an element that is neither, a negative code and a name before any
/// code are skipped, as the reader skips them. `byte_codes` reads as `pdf_model::type3` reads a
/// Type 3 font's array, where a code beyond one byte is read past as a negative one is, since
/// §9.6.4 selects a Type 3 glyph by a single-byte code.
fn differences_as_read(document: &Document, items: &[Object], byte_codes: bool) -> Vec<Object> {
    let mut read = Vec::with_capacity(items.len());
    let mut code: Option<i64> = None;
    let mut next_written: Option<i64> = None;
    for item in items {
        match document.resolve(item) {
            Object::Integer(value) if value >= 0 && (!byte_codes || value <= 255) => {
                code = Some(value);
            }
            Object::Integer(_) => code = None,
            Object::Name(name) => {
                let Some(at) = code else {
                    continue;
                };
                if next_written != Some(at) {
                    read.push(Object::Integer(at));
                }
                read.push(Object::Name(name));
                next_written = at.checked_add(1);
                code = next_written;
            }
            _ => {}
        }
    }
    read
}

/// `font` with `encoding` written in place of its `/Encoding`, for the proof alone.
fn with_encoding(font: &Dictionary, encoding: Dictionary) -> Dictionary {
    let mut font = font.clone();
    font.insert(Name::new(&b"Encoding"[..]), Object::Dictionary(encoding));
    font
}

/// Whether a simple font loaded from `after` draws every single-byte code's outline at the advance
/// it drew loaded from `before`, and a Type 3 font read from `after` names every code's glyph as
/// it did ([`type3_alike`]). A composite font's codes select CIDs through its `CMap`, which none
/// of these entries reaches, so only its `/ToUnicode` can change, and that draws nothing.
fn draws_alike(document: &Document, before: &Dictionary, after: &Dictionary) -> bool {
    let name = "the font restated as read";
    match Kind::of(document, before) {
        Kind::Simple => {}
        Kind::Type3 => return type3_alike(document, before, after),
        Kind::Other => return true,
    }
    let (Ok(drawn), Ok(restated)) = (
        LoadedFont::load(document, before, name),
        LoadedFont::load(document, after, name),
    ) else {
        // A font that does not load is drawn by the substitute either way; what it draws is the
        // embedding routes' to answer, and the two dictionaries name the same font.
        return LoadedFont::load(document, before, name).is_err()
            && LoadedFont::load(document, after, name).is_err();
    };
    (0..=u8::MAX).all(|byte| {
        let code = Code::single_byte(byte);
        // `Path` states no equality; its derived `Debug` writes every point's coordinates in
        // full, so two outlines that print alike are the same outline.
        let outline = |font: &LoadedFont| font.outline(code).map(|path| format!("{path:?}"));
        drawn.advance(code).to_bits() == restated.advance(code).to_bits()
            && outline(&drawn) == outline(&restated)
    })
}

/// Whether a Type 3 font read from `after` selects every single-byte code's glyph by the name it
/// selected it by read from `before`, or both are refused alike.
///
/// ISO 32000-2 §9.6.4 draws a Type 3 glyph by two lookups: the code in `/Encoding` gives a glyph
/// name, and the name in `/CharProcs` gives the glyph description. Only the first reads an entry
/// this module restates, and `/Widths` and `/FontMatrix` are written as they were, so a code
/// named alike draws the description and the advance it drew.
fn type3_alike(document: &Document, before: &Dictionary, after: &Dictionary) -> bool {
    let name = "the font restated as read";
    match (
        Type3Font::read(document, before, name),
        Type3Font::read(document, after, name),
    ) {
        (Ok(drawn), Ok(restated)) => {
            (0..=u32::from(u8::MAX)).all(|code| drawn.glyph_name(code) == restated.glyph_name(code))
        }
        (Err(drawn), Err(restated)) => drawn == restated,
        _ => false,
    }
}

/// The three kinds of font this module's entries mean something different for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A simple font with a program, whose `/Encoding` Table 109 gives the predefined names.
    Simple,
    /// A Type 3 font, whose `/Encoding` Table 110 requires and §9.6.5.3 makes its whole mapping.
    Type3,
    /// A composite font or a `CIDFont`, whose codes no entry here reaches.
    Other,
}

impl Kind {
    /// The kind `font`'s `/Subtype` states.
    fn of(document: &Document, font: &Dictionary) -> Self {
        match document.resolve(font.get("Subtype").unwrap_or(&Object::Null)) {
            Object::Name(name)
                if matches!(name.as_bytes(), b"Type1" | b"MMType1" | b"TrueType") =>
            {
                Self::Simple
            }
            Object::Name(name) if name.as_bytes() == b"Type3" => Self::Type3,
            _ => Self::Other,
        }
    }
}

/// Whether `name` is left as written: one of the encodings `pdf_font` reads by name, or the
/// standard encoding's on a nonsymbolic `TrueType` font, which `standard-truetype-encoding`
/// restates instead (ADR 3753).
fn predefined(name: &Name, truetype: bool) -> bool {
    PERMITTED_ENCODING_NAMES.contains(&name.as_bytes())
        || (truetype && name.as_bytes() == b"StandardEncoding")
}

/// Whether the font is a `TrueType` font whose descriptor states its flags and clears Table 121's
/// Symbolic flag: the population of the validator's row for a nonsymbolic `TrueType` font's
/// encoding, decided as that row decides it.
fn nonsymbolic_truetype(document: &Document, font: &Dictionary) -> bool {
    /// Table 121's bit position 3, counting from one as the specification does.
    const SYMBOLIC: u32 = 1 << 2;

    let is_truetype = matches!(
        document.resolve(font.get("Subtype").unwrap_or(&Object::Null)),
        Object::Name(name) if name.as_bytes() == b"TrueType"
    );
    let Object::Dictionary(descriptor) = document.get_key(font, "FontDescriptor") else {
        return false;
    };
    is_truetype
        && document
            .get_key(&descriptor, "Flags")
            .as_integer()
            .is_some()
        && pdf_font::descriptor_flags(document, &descriptor) & SYMBOLIC == 0
}

/// A value the report can name: a name as itself, anything else by its kind.
fn described(value: &Object) -> String {
    match value {
        Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        Object::Null => "null".to_owned(),
        Object::Integer(_) | Object::Real(_) => "a number".to_owned(),
        Object::String(_) => "a string".to_owned(),
        Object::Array(_) => "an array".to_owned(),
        Object::Dictionary(_) => "a dictionary".to_owned(),
        _ => "a value".to_owned(),
    }
}
