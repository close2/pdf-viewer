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
//!   is written again as the codes and names it assigned, in the order it assigned them.
//!
//! Each is checked before it is written: a simple font loaded from the dictionary as restated
//! draws every single-byte code's outline at the advance it drew from the dictionary as written.
//! What is lost is the producer's own words at those places, which no reader reads by ISO 32000-2
//! and some reader may read by a convention of its own — `/ToUnicode /Identity-H` read as the
//! identity map is the common one — so the restatement is the loss `font-maps-as-read`, and the
//! report names every font and entry.
//!
//! Refused by name ([`NOT_RESTATED`]): a font the proof does not hold for — a nonsymbolic
//! `TrueType` font's unknown `/Encoding` name is one, since this reader selects its glyphs by
//! another route once the entry is absent — and, through the output's own check, a font written
//! directly inside another object, which this pass does not reach.
//!
//! **A nonsymbolic `TrueType` font naming the standard encoding is passed over** (ADR 3753): the
//! validator reports that name under `fonts/non-symbolic-truetype-uses-a-standard-encoding`
//! rather than this row, and `standard-truetype-encoding`'s rewrite restates it as one of the two
//! names ISO 19005 admits, proved code by code; removing it here would move the font onto the
//! route the proof above refuses.

use std::collections::BTreeMap;

use pdf_font::{Code, LoadedFont, PERMITTED_ENCODING_NAMES};
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const ROW: &str = "fonts/font-maps-read-as-written";

/// Why a font's maps are not restated as read.
pub(super) const NOT_RESTATED: &str = "a font dictionary states a ToUnicode that is not a \
     stream, an Encoding or BaseEncoding naming no encoding ISO 32000-2 predefines, or a \
     Differences array with an element out of its grammar, and this reader read past it; this \
     conversion restates such a font as it was read only where the font is an object of its own, \
     an encoding dictionary it restates is read alike by every font naming it, and every code the \
     font shows draws the outline and the advance it drew; this font is not that";

/// The fonts and encoding dictionaries restated, and what each lost.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// Each dictionary written again, by its object: a font, or an encoding dictionary.
    pub(super) dictionaries: BTreeMap<ObjectId, Dictionary>,
    /// For the report, one line per entry restated.
    pub(super) entries: Vec<String>,
}

/// Every font the reader read a map of past, restated as read, or why one is not.
///
/// # Errors
///
/// [`NOT_RESTATED`] where any font the log names is outside what the module comment lists.
pub(super) fn restated(document: &Document) -> Result<Restated, Because> {
    let refused = Because::NotBuiltYet(NOT_RESTATED);
    let mut restated = Restated::default();
    for number in document.xref().object_numbers() {
        let id = document.defined_id(number);
        let Object::Dictionary(font) = document.get(id) else {
            continue;
        };
        if font
            .get("Type")
            .and_then(Object::as_name)
            .map(Name::as_bytes)
            != Some(b"Font")
        {
            continue;
        }
        let mut written = font.clone();
        let mut lines = Vec::new();
        if let Some(stated) = font.get("ToUnicode")
            && document.resolve(stated).as_stream().is_none()
        {
            written.remove("ToUnicode");
            lines.push(format!(
                "object {} {}: ToUnicode {} removed",
                id.number,
                id.generation,
                described(&document.resolve(stated))
            ));
        }
        let encoding_at = font.get("Encoding").and_then(Object::as_reference);
        let truetype = nonsymbolic_truetype(document, &font);
        match document.resolve(font.get("Encoding").unwrap_or(&Object::Null)) {
            Object::Name(name) if simple(document, &font) && !predefined(&name, truetype) => {
                written.remove("Encoding");
                lines.push(format!(
                    "object {} {}: Encoding /{} removed",
                    id.number,
                    id.generation,
                    String::from_utf8_lossy(name.as_bytes())
                ));
            }
            Object::Dictionary(encoding) => {
                if let Some(read) = encoding_as_read(document, &encoding, truetype) {
                    lines.push(format!(
                        "object {} {}: Encoding {}",
                        id.number, id.generation, read.1
                    ));
                    match encoding_at {
                        Some(at) => {
                            restated.dictionaries.insert(at, read.0.clone());
                        }
                        None => {
                            written.insert(
                                Name::new(&b"Encoding"[..]),
                                Object::Dictionary(read.0.clone()),
                            );
                        }
                    }
                    if !draws_alike(document, &font, &with_encoding(&written, read.0)) {
                        return Err(refused);
                    }
                }
            }
            _ => {}
        }
        if lines.is_empty() {
            continue;
        }
        if !draws_alike(document, &font, &written) {
            return Err(refused);
        }
        restated.entries.extend(lines);
        if written != font {
            restated.dictionaries.insert(id, written);
        }
    }
    // Every font naming a restated encoding dictionary is a font of this loop, so each was proved
    // drawing alike with it.
    Ok(restated)
}

/// An encoding dictionary as this reader read it, and the line that says what changed; `None`
/// where nothing was read past. `truetype` is [`nonsymbolic_truetype`] of the font naming it.
fn encoding_as_read(
    document: &Document,
    encoding: &Dictionary,
    truetype: bool,
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
            let read = differences_as_read(document, &items);
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
/// code are skipped, as the reader skips them.
fn differences_as_read(document: &Document, items: &[Object]) -> Vec<Object> {
    let mut read = Vec::with_capacity(items.len());
    let mut code: Option<i64> = None;
    let mut next_written: Option<i64> = None;
    for item in items {
        match document.resolve(item) {
            Object::Integer(value) if value >= 0 => code = Some(value),
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
/// it drew loaded from `before`. A composite font's codes select CIDs through its `CMap`, which
/// none of these entries reaches, so only its `/ToUnicode` can change, and that draws nothing.
fn draws_alike(document: &Document, before: &Dictionary, after: &Dictionary) -> bool {
    if !simple(document, before) {
        return true;
    }
    let name = "the font restated as read";
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

/// Whether the font is a simple font, whose `/Encoding` Table 109 gives the predefined names.
fn simple(document: &Document, font: &Dictionary) -> bool {
    matches!(
        document.resolve(font.get("Subtype").unwrap_or(&Object::Null)),
        Object::Name(name) if matches!(name.as_bytes(), b"Type1" | b"MMType1" | b"TrueType")
    )
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
