//! The glyph name each of a simple font's 256 character codes selects.
//!
//! This is the half of ISO 32000-2 §9.6.5's mapping that the *document* determines: a base
//! encoding, named by `/Encoding` or by an encoding dictionary's `/BaseEncoding`, with a
//! `/Differences` array layered over it. What a name then reaches belongs to the font
//! program — [`crate::name_keyed`] for a charset, [`crate::truetype`] for a `cmap` — and the
//! encodings' own tables are [`crate::encoding`]'s.

use std::borrow::Cow;

use pdf_syntax::{Dictionary, Document, Object};

use crate::encoding::{self, BaseEncoding};
use crate::loading::FontError;

/// The glyph name each of a simple font's 256 character codes selects.
///
/// Borrowed for a name one of the specifications lists, which is nearly every name a real
/// document writes, and owned for one only the document's own font program carries — a
/// subsetter's `/gid2436`, say. Both have to be kept: an unrecognised name is *not* an
/// unencoded code, and the font's own `post` table or CFF charset may hold the glyph under
/// exactly that spelling. Dropping them was a defect; see [`apply_differences`].
pub(crate) type GlyphNames = Box<[Cow<'static, str>; 256]>;

/// A table of 256 unencoded codes, which every encoding starts from.
pub(crate) fn no_names() -> GlyphNames {
    Box::new(std::array::from_fn(|_| Cow::Borrowed("")))
}

/// The glyph name each character code selects, according to the PDF encoding alone.
///
/// This is the half of the mapping the *document* determines, shared by every route to a
/// glyph: a bare CFF resolves these names through its charset, a substitute resolves them
/// through the Adobe Glyph List, and text extraction falls back to them when a font has no
/// `/ToUnicode`. An empty name means the code is unencoded and the font's own encoding
/// applies.
pub(crate) fn encoding_names(
    document: &Document,
    dict: &Dictionary,
    name: &str,
    symbolic_font: Option<encoding::SymbolicEncoding>,
    fall_back_to_standard: bool,
) -> Result<GlyphNames, FontError> {
    let mut names = no_names();

    if let Some(symbolic) = symbolic_font {
        // The two symbolic standard-14 fonts have their own encoding and no Latin base.
        for (code, slot) in names.iter_mut().enumerate() {
            if let Ok(code) = u8::try_from(code) {
                *slot = Cow::Borrowed(symbolic.glyph_name(code));
            }
        }
    } else {
        let base = base_encoding(document, dict, name)?
            .or(fall_back_to_standard.then_some(BaseEncoding::Standard));
        if let Some(base) = base {
            for (code, slot) in names.iter_mut().enumerate() {
                if let Ok(code) = u8::try_from(code) {
                    *slot = Cow::Borrowed(base.glyph_name(code));
                }
            }
        }
    }

    apply_differences(document, dict, &mut names);
    Ok(names)
}

/// The names Table 109 permits `/Encoding` to hold, and Table 112 `/BaseEncoding`.
///
/// ISO 32000-2 §9.6.2.1, Table 109, of a font dictionary's `/Encoding`:
///
/// > The value of Encoding shall be either the name of a predefined encoding (
/// > MacRomanEncoding, MacExpertEncoding , or WinAnsiEncoding , as described in Annex D,
/// > "Character sets and encodings") or an encoding dictionary
///
/// `StandardEncoding` is not on the table's list and is accepted anyway: Annex D defines it,
/// §9.6.5.1 makes it the base a nonsymbolic font falls back to, and producers write it. That is
/// a deliberate extra rather than an oversight, and it is why this list exists separately from
/// [`BaseEncoding::by_name`] — the two answer different questions, *may a font say this* and
/// *does this crate have the table*. **`MacExpertEncoding` was the name where they differed and
/// is not any more**: Annex D.4's table is transcribed, so the two lists give the same four
/// answers today and `an_encoding_name_the_table_does_not_permit_is_no_encoding_at_all` asserts
/// it. They stay separate because a later edition can add a name to one and not the other, and
/// because the answer to the second question is what decides whether a font draws.
pub(crate) const PERMITTED_ENCODING_NAMES: [&[u8]; 4] = [
    b"StandardEncoding",
    b"MacRomanEncoding",
    b"MacExpertEncoding",
    b"WinAnsiEncoding",
];

/// Reads the base encoding a font dictionary names, if it names one.
///
/// # A name the table does not permit is not an encoding this font uses
///
/// Table 109 makes `/Encoding` **optional** and says what its absence means in the same cell —
/// it is "[a] specification of the font's character encoding **if different from its built-in
/// encoding**" — so a font that states nothing readable there has stated nothing, and the
/// built-in encoding stands. A name outside the four above is therefore treated as absent
/// rather than refused: refusing draws no text at all where the clause states which text to
/// draw, which is ADR 0106's rule about an optional entry erasing what a clause requires.
/// `bug859204.pdf` writes `/Encoding /NULL` on an embedded Type 1 program and lost its whole
/// page for it.
///
/// **The refusal below has no member today.** It fires where a name the table *permits* has no
/// table in this crate — `MacExpertEncoding` was that name until Annex D.4 was transcribed —
/// and it is kept rather than removed because the distinction is the load-bearing one: a name
/// the table permits carries a meaning a fallback would lose, and a name it does not permit
/// carries none. Six of the expert set's codes mean exactly what they mean in `WinAnsiEncoding`
/// — space, comma, hyphen, period, colon, semicolon — so a fallback would have got a document's
/// punctuation right and every letter wrong, which is why it was never taken.
fn base_encoding(
    document: &Document,
    dict: &Dictionary,
    name: &str,
) -> Result<Option<BaseEncoding>, FontError> {
    let encoding = document.get_key(dict, "Encoding");
    let named = encoding
        .as_name()
        .map(|value| value.as_bytes().to_vec())
        .or_else(|| {
            encoding
                .as_dict()
                .map(|d| document.get_key(d, "BaseEncoding"))
                .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec()))
        });

    match named {
        None => Ok(None),
        Some(named) if !PERMITTED_ENCODING_NAMES.contains(&named.as_slice()) => Ok(None),
        Some(named) => {
            BaseEncoding::by_name(&named)
                .map(Some)
                .ok_or_else(|| FontError::UnsupportedEncoding {
                    name: name.to_owned(),
                    encoding: String::from_utf8_lossy(&named).into_owned(),
                })
        }
    }
}

/// Layers an `/Encoding` dictionary's `/Differences` array over a table of glyph names.
///
/// # A name this crate does not recognise still names the code
///
/// An earlier version kept only names with a `'static` spelling and left the base
/// encoding's name in place for the rest, which meant a code the document had explicitly
/// reassigned silently kept its old meaning. `issue20504.pdf` writes
/// `/Differences [33 /gid2436 …]` — a subsetter's convention for naming a glyph by its
/// index, which §9.6.5 does not define but does not forbid either — and every one of its
/// four codes fell back to `StandardEncoding`, so a page of Chinese was drawn as `!"#$`
/// with nothing reported. §9.6.5.4's "any *undefined* entries in the table shall be filled
/// using `StandardEncoding`" is about codes the encoding never assigned, not about codes it
/// assigned to a name we happen not to know.
///
/// So every name is kept. A recognised one keeps its static spelling and costs no
/// allocation, which is the case for nearly every name a real document writes; a novel one
/// is owned, and reaches the font's own `post` table or CFF charset where it may well be
/// found.
fn apply_differences(document: &Document, dict: &Dictionary, names: &mut GlyphNames) {
    let encoding = document.get_key(dict, "Encoding");
    let Some(encoding) = encoding.as_dict() else {
        return;
    };
    let differences = document.get_key(encoding, "Differences");
    let Some(items) = differences.as_array() else {
        return;
    };

    let mut code: Option<usize> = None;
    for item in items {
        match document.resolve(item) {
            Object::Integer(value) => code = usize::try_from(value).ok(),
            Object::Name(glyph_name) => {
                let Some(at) = code else { continue };
                if let Some(slot) = names.get_mut(at) {
                    *slot = glyph_name_of(glyph_name.as_bytes());
                }
                code = at.checked_add(1);
            }
            _ => {}
        }
    }
}

/// Returns a `/Differences` name, borrowing the specifications' spelling where there is one.
///
/// Glyph names are ASCII by specification; a font that breaks that is malformed, not a
/// reason to lose the name, so the owned case is lossy rather than fallible.
fn glyph_name_of(name: &[u8]) -> Cow<'static, str> {
    match interned(name) {
        Some(known) => Cow::Borrowed(known),
        None => Cow::Owned(String::from_utf8_lossy(name).into_owned()),
    }
}

/// Returns the `'static` spelling of a glyph name, if it is one the specifications list.
///
/// Matching one avoids an allocation for the overwhelmingly common case of a name PDF or
/// CFF already defines.
fn interned(name: &[u8]) -> Option<&'static str> {
    let name = std::str::from_utf8(name).ok()?;
    skrifa::raw::ps::string::STANDARD_STRINGS
        .iter()
        .copied()
        .find(|known| *known == name)
        .or_else(|| {
            (0..=u8::MAX).find_map(|code| {
                [
                    BaseEncoding::WinAnsi.glyph_name(code),
                    BaseEncoding::MacRoman.glyph_name(code),
                    encoding::SymbolicEncoding::Symbol.glyph_name(code),
                    encoding::SymbolicEncoding::ZapfDingbats.glyph_name(code),
                ]
                .into_iter()
                .find(|known| *known == name)
            })
        })
}
