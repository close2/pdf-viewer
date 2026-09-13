//! ISO 32000-2 §9.7's composite fonts: the `CMap` a Type0 font's `/Encoding` names, and the
//! `CIDFont`'s own route from a CID to a glyph.
//!
//! The two halves are §9.7.5's and §9.7.4.2's and they are independent — a `CMap` says
//! nothing about glyph indices and a `/CIDToGIDMap` says nothing about codes — so they are
//! resolved apart from each other, which is what stops the Identity case from being the only
//! one that works. The `CMap` format itself is [`crate::cmap`]'s and the predefined ones are
//! [`crate::predefined`]'s; what is here is what a font dictionary says about them.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object};
use skrifa::FontRef;

use crate::cff::CodeToGlyph;
use crate::cmap::CMap;
use crate::loading::FontError;
use crate::predefined;
use crate::program::Program;
use crate::tounicode::ToUnicode;

/// How a `CIDFont` turns a CID into a glyph index (ISO 32000-2 §9.7.4.2).
///
/// The clause gives one route per kind of embedded program, and the three arms below are
/// exactly those routes. Which one applies is decided by what the program *is* rather than by
/// the `/Subtype` name, because a `CIDFontType0` may arrive wrapped in an `OpenType`
/// container and glyph selection then still runs through the CFF's charset.
#[derive(Debug)]
pub(crate) enum CidToGlyph {
    /// The CID is the glyph index.
    ///
    /// Two of the clause's cases reach this. A `TrueType` program whose `/CIDToGIDMap` is
    /// `Identity` (§9.7.5.2):
    ///
    /// > the 2-byte CID values shall be identical glyph indices for the glyph descriptions in
    /// > the `TrueType` font program
    ///
    /// And a CFF program whose Top DICT carries no `CIDFont` operators (§9.7.4.2):
    ///
    /// > The CIDs shall be used directly as GID values
    Identity,
    /// A CID-keyed CFF's charset, inverted (§9.7.4.2).
    ///
    /// > The CIDs shall be used to determine the GID value for the glyph procedure using the
    /// > charset table in the CFF program.
    Charset(BTreeMap<u16, u16>),
    /// A `/CIDToGIDMap` stream (§9.7.4.1 Table 115).
    ///
    /// > the glyph index for a particular CID value c shall be a 2-byte value stored in bytes
    /// > 2 × 𝑐 and 2 × 𝑐 + 1 , where the first byte shall be the high-order byte.
    Stream(Arc<[u8]>),
}

impl CidToGlyph {
    /// The glyph index a CID selects, or `None` where the `CIDFont` has none for it.
    pub(crate) fn glyph(&self, cid: u32) -> Option<u16> {
        match self {
            // Glyph indices are 16 bits wide, so a larger CID cannot name one.
            Self::Identity => u16::try_from(cid).ok(),
            Self::Charset(by_cid) => by_cid.get(&u16::try_from(cid).ok()?).copied(),
            Self::Stream(bytes) => {
                let at = usize::try_from(cid).ok()?.checked_mul(2)?;
                let pair = bytes.get(at..at.checked_add(2)?)?;
                let (Some(&high), Some(&low)) = (pair.first(), pair.get(1)) else {
                    return None;
                };
                Some(u16::from_be_bytes([high, low]))
            }
        }
    }
}

/// Resolves a Type 0 font's `/Encoding` to the `CMap` that decodes its strings.
///
/// ISO 32000-2 §9.7.6.1 Table 119 gives the two forms:
///
/// > The name of a predefined `CMap`, or a stream containing a `CMap` that maps character codes
/// > to font numbers and CIDs.
///
/// Of the predefined names, Table 116's two Identity `CMap`s are built here and the rest are
/// *data* — the registered `CMap` files — which this binary has carried since the
/// hundred-and-fifty-sixth session (see [`crate::predefined`]). A name it does not carry is
/// still refused and reported rather than approximated: guessing at a `CMap` maps codes to the
/// wrong glyphs, which is plausible-looking wrong text and the worst kind of rendering error.
///
/// Vertical writing is *drawn*, and this comment said it was refused for eighty-five sessions
/// after it stopped being. §9.7.5.1 makes the mode a property of the `CMap` and it decides the
/// metrics:
///
/// > A `CMap` shall specify the writing mode … for any `CIDFont` with which the `CMap` is
/// > combined. The writing mode determines which metrics shall be used when glyphs are painted
/// > from that font.
///
/// §9.2.4 and §9.7.4.3 give those metrics as `/W2` and `/DW2`, which `Vertical::read` has read
/// since the thirty-sixth session — `vertical.pdf` sets two columns down the right edge of its
/// page, where before that it came out as one overlapping line across the top, reporting
/// nothing. What is refused here is a *predefined* `CMap`, horizontal or vertical alike, and
/// the only reason a name ending in `V` is refused is the data the paragraph above names.
pub(crate) fn composite_cmap(
    document: &Document,
    dict: &Dictionary,
    name: &str,
) -> Result<CMap, FontError> {
    let unsupported = |encoding: &str| FontError::UnsupportedEncoding {
        name: name.to_owned(),
        encoding: encoding.to_owned(),
    };

    let encoding = document.get_key(dict, "Encoding");
    let cmap = match &encoding {
        Object::Name(named) => match named.as_bytes() {
            b"Identity-H" => return Ok(CMap::identity()),
            // §9.7.5.2: the two Identity `CMap`s differ only in their writing mode, and
            // the vertical one carries `/WMode 1`, which `CMap::identity_vertical` states.
            b"Identity-V" => return Ok(CMap::identity_vertical()),
            other => {
                let named = String::from_utf8_lossy(other);
                return predefined::cmap(&named).ok_or_else(|| unsupported(&named));
            }
        },
        Object::Stream(_) => read_cmap(document, &encoding, name, 0)?,
        _ => return Err(unsupported("no /Encoding naming a CMap")),
    };

    // Table 118's `/WMode` and the file's own must agree — "The value of this entry shall be
    // the same as the value of WMode in the CMap file" — so a font is refused if *either*
    // asks for vertical writing rather than only the one a reader happens to consult.
    let dictionary_wmode = encoding
        .as_stream()
        .map(|stream| document.get_key(&stream.dict, "WMode"))
        .and_then(|value| value.as_integer())
        .unwrap_or(0);
    if i64::from(cmap.wmode()) != dictionary_wmode.clamp(0, 1) {
        return Err(unsupported(
            "a CMap whose /WMode disagrees with the one in its own stream",
        ));
    }

    // Without codespace ranges no code can be extracted at all (§9.7.6.2), and without
    // character mappings every code is `.notdef` (§9.7.6.3) — a page of empty boxes drawn in
    // silence. Both are refused rather than approximated, per the rule that unsupported input
    // stays loud. Every one of the corpus's fourteen embedded `CMap`s states both.
    if !cmap.has_codespace() {
        return Err(unsupported("a CMap stream with no codespace ranges"));
    }
    if !cmap.has_mappings() {
        return Err(unsupported("a CMap stream with no character mappings"));
    }
    Ok(cmap)
}

/// Reads one embedded `CMap` stream and the chain of `/UseCMap`s beneath it (§9.7.5.3).
///
/// Table 118 states what it is:
///
/// > The name of a predefined `CMap`, or a stream containing a `CMap`. If this entry is
/// > present, the referencing `CMap` shall specify only the character mappings that differ from
/// > the referenced `CMap`.
///
/// A stream is read and built upon; a *name* is a predefined `CMap`, which this binary carries
/// (see [`crate::predefined`]) and which is therefore resolved rather than refused. A name it
/// does not carry is still refused: the referencing map's mappings would be missing an unknown
/// share of their codes, which is worse than saying so.
///
/// §9.7.5.4 a) requires a `usecmap` operator inside the file to be named by `/UseCMap` as well:
///
/// > If the embedded `CMap` file contains a `usecmap` reference, the `CMap` indicated there
/// > shall also be identified by the `UseCMap` entry in the `CMap` stream dictionary.
///
/// **Because the clause requires the two to agree, the file's own statement is read where the
/// dictionary is silent** — on a conforming file it says the same thing, and on a file that
/// omitted the entry it is the only statement there is. The refusal is kept for the case its
/// reason actually describes: a name this binary does not carry, where what the map inherits
/// cannot be found and an unknown share of its codes would silently be missing.
///
/// No corpus document exercises any of this — none of the fourteen embedded `CMap`s references
/// another — so the tests are synthetic, which is trap 8's advice rather than an accident. The
/// `/ToUnicode` form is the one a document does exercise; see [`crate::loading::read_to_unicode`].
fn read_cmap(
    document: &Document,
    object: &Object,
    name: &str,
    depth: u32,
) -> Result<CMap, FontError> {
    /// Bounds the `/UseCMap` chain, which a document could otherwise make cyclic.
    const MAX_DEPTH: u32 = 4;

    let unsupported = |encoding: &str| FontError::UnsupportedEncoding {
        name: name.to_owned(),
        encoding: encoding.to_owned(),
    };

    if depth > MAX_DEPTH {
        return Err(unsupported("a /UseCMap chain deeper than four"));
    }
    let stream = object
        .as_stream()
        .ok_or_else(|| unsupported("an /Encoding that is neither a name nor a stream"))?;
    let data = document
        .decoded_stream_data(stream)
        // The stream is the font dictionary's `/Encoding`, so what failed is the file's own
        // entry rather than a font program; [`FontError::MalformedDictionary`] is the variant
        // that does not claim a program was read.
        .ok_or_else(|| FontError::MalformedDictionary {
            name: name.to_owned(),
            detail: "its /Encoding CMap stream could not be decoded (§9.7.5.3)".to_owned(),
        })?;

    let used = match document.get_key(&stream.dict, "UseCMap") {
        // §9.7.5.4 a) requires an in-file `usecmap` reference to be named by this entry as
        // well, so on a conforming file the two say the same thing and the file's own
        // statement is what a file that omitted the entry has left. Resolved rather than
        // refused where the name is one this binary carries; the refusal below still fires
        // where it is not, because then what the map inherits genuinely cannot be found.
        Object::Null => predefined::used_by(&data).and_then(|name| predefined::cmap(&name)),
        Object::Name(named) if named.as_bytes() == b"Identity-H" => Some(CMap::identity()),
        Object::Name(named) if named.as_bytes() == b"Identity-V" => Some(CMap::identity_vertical()),
        Object::Name(named) => {
            let name_used = String::from_utf8_lossy(named.as_bytes()).into_owned();
            Some(predefined::cmap(&name_used).ok_or_else(|| {
                unsupported(&format!("a CMap built on the predefined {name_used}"))
            })?)
        }
        referenced => Some(read_cmap(
            document,
            &referenced,
            name,
            depth.saturating_add(1),
        )?),
    };

    let cmap = CMap::parse(&data, used.as_ref());
    if cmap.references_another() && used.is_none() {
        return Err(unsupported(
            "a CMap whose usecmap reference is not named by /UseCMap (§9.7.5.4 a))",
        ));
    }
    Ok(cmap)
}

/// Resolves how a `CIDFont`'s CIDs reach glyph indices (§9.7.4.2).
///
/// Three routes, in the order the clauses put them.
///
/// **A CID-keyed CFF's charset first**, because §9.7.4.2 states that route outright and does
/// so for the program rather than for the `/Subtype` name — a `CIDFontType0` may arrive as a
/// CFF wrapped in an `OpenType` container, and the clause's two CFF cases are about what the
/// Top DICT contains.
///
/// **Then the dictionary's own `/CIDToGIDMap` stream**, whatever the `CIDFont`'s subtype.
/// Table 115 conditions the entry's *presence* on Type 2 — "Required for Type 2 `CIDFonts` with
/// embedded font programs" — and defines its meaning unconditionally: "A specification of the
/// mapping from CIDs to glyph indices." §9.7.4.2's other CFF sentence, that a program whose
/// Top DICT has no `CIDFont` operators uses "the CIDs … directly as GID values", describes what
/// such a program offers on its own; it cannot outrank a mapping the font dictionary states,
/// because that would make the stated mapping mean nothing.
///
/// **The identity last**, which is what remains when neither the program nor the dictionary
/// says otherwise.
///
/// The one corpus font that settles the middle rule is `issue7901.pdf`'s: a `CIDFontType0`
/// whose `/FontFile3` is an `OpenType` wrapper around a *name*-keyed CFF, carrying a
/// `/CIDToGIDMap` stream of 230 entries. Ignoring the stream there — the first reading of
/// Table 115 written here — drew "üãÍ†Ë œÍ†ÿ¨ Ì{«" where four renderers draw "The Free Software
/// Definition", because the producer's CIDs are not that CFF's glyph indices and the stream is
/// the only thing in the file that says what they are.
pub(crate) fn cid_to_glyph(
    document: &Document,
    descendant: &Dictionary,
    data: &[u8],
    program: Program,
    name: &str,
) -> Result<CidToGlyph, FontError> {
    // A CFF, bare or wrapped. `read` reports whether the Top DICT uses CIDFont operators,
    // which is precisely the distinction §9.7.4.2 draws between its two CFF cases.
    let cff = match program {
        Program::BareCff => Some(Cow::Borrowed(data)),
        Program::Sfnt => FontRef::new(data).ok().and_then(|font| {
            font.table_data(skrifa::Tag::new(b"CFF "))
                .map(|table| Cow::Owned(table.as_bytes().to_vec()))
        }),
        // §9.9's Table 124 gives a CIDFont two programs, `/FontFile2` and `/FontFile3`, and
        // a bare Type 1 is neither — so nothing is read out of it *here*, and the
        // `/CIDToGIDMap` route below decides, which for a CIDFontType0 is the identity.
        //
        // That identity is the clause's own answer rather than a guess, by two sentences
        // read together. §9.7.4.2, on a CFF whose Top DICT does not use CIDFont operators:
        //
        // > The CIDs shall be used directly as GID values, and the glyph procedure shall be
        // > retrieved using the CharStrings INDEX
        //
        // and §9.6.2.1's NOTE 1, on what a CFF *is*:
        //
        // > an alternative, more compact but functionally equivalent representation of a
        // > Type 1 font program
        //
        // A bare Type 1 program is a name-keyed program whose charstrings are in an order,
        // exactly as a non-CID-keyed CFF's are, so "used directly as GID values" transfers
        // without inventing anything. `issue11740_reduced.pdf` is the witness, and before
        // this it was substituted for and drawn through its own `/ToUnicode` — which
        // records the Windows-1251 *bytes* of its Russian text as Latin-1 code points, so
        // the page came out as mojibake with nothing reported.
        Program::Type1 => None,
    };
    if let Some(cff) = cff {
        // A bare CFF has already been read once, to decide it was one; a wrapped one is read
        // here for the first time. Either way a program this crate cannot parse is a font it
        // cannot draw, so the error is propagated rather than falling through to the identity.
        let read = CodeToGlyph::read(&cff).map_err(|e| FontError::Malformed {
            name: name.to_owned(),
            detail: e.to_string(),
        })?;
        if let CodeToGlyph::Keyed { by_cid } = read {
            return Ok(CidToGlyph::Charset(by_cid));
        }
    }

    Ok(match document.get_key(descendant, "CIDToGIDMap") {
        // Absent, or `Identity`: the CID is the glyph index. A producer that omits the entry
        // has said nothing to prefer over the identity — which is also what §9.7.5.2 describes
        // for `Identity-H` with an embedded `TrueType` program.
        Object::Null => CidToGlyph::Identity,
        Object::Name(map) if map == "Identity" => CidToGlyph::Identity,
        Object::Name(other) => {
            return Err(FontError::UnsupportedEncoding {
                name: name.to_owned(),
                encoding: format!(
                    "/CIDToGIDMap /{}, which Table 115 says shall be Identity",
                    String::from_utf8_lossy(other.as_bytes())
                ),
            });
        }
        stream => {
            /// A glyph index is 16 bits, so no CID above 65 535 can have one and a map longer
            /// than two bytes per such CID describes nothing.
            const MAX_MAP: usize = 2 * (1 << 16);

            // Both failures below are the descendant dictionary's own entry rather than a font
            // program: Table 115 types `/CIDToGIDMap` as a stream or a name, and a value that is
            // neither, or a stream that will not decode, leaves §9.7.4.2 with no map. Reporting
            // them as a program that "could not be parsed" would send a reader to the
            // `/FontFile2`, which is not the fault.
            let stream = stream
                .as_stream()
                .ok_or_else(|| FontError::MalformedDictionary {
                    name: name.to_owned(),
                    detail: "/CIDToGIDMap is neither a name nor a stream, which is all Table 115 \
                             types it as (§9.7.4.2)"
                        .to_owned(),
                })?;
            let bytes = document.decoded_stream_data(stream).ok_or_else(|| {
                FontError::MalformedDictionary {
                    name: name.to_owned(),
                    detail: "its /CIDToGIDMap stream could not be decoded (§9.7.4.2)".to_owned(),
                }
            })?;
            let kept = bytes.get(..bytes.len().min(MAX_MAP)).unwrap_or(&bytes);
            CidToGlyph::Stream(Arc::from(kept))
        }
    })
}

/// Reads a font's `/ToUnicode` `CMap`, which is absent more often than not.
/// ISO 32000-2 §9.10.2's third method, steps b) to d): the character collection's own table.
///
/// > b. Obtain the registry and ordering of the character collection used by the font's CMap
/// > (for example, Adobe and Japan1) from its CIDSystemInfo dictionary.
///
/// `None` where the descendant states no `/CIDSystemInfo`, or states one this binary carries no
/// table for — which is every registry but Adobe's, and `Identity` orderings, where the codes
/// are indices into a font nobody supplied and no table could say what they mean.
///
/// Keyed by **CID**, which is the caller's business: step (a) of the same method is the font's
/// own `CMap`, and `LoadedFont` applies it before asking this table.
pub(crate) fn collection_table(document: &Document, descendant: &Dictionary) -> Option<ToUnicode> {
    let (registry, ordering) = collection_names(document, descendant)?;
    predefined::cid_to_unicode(&registry, &ordering)
}

/// §9.7.3's registry and ordering, which between them name a character collection.
///
/// > A character collection shall be uniquely identified by the Registry , Ordering , and
/// > Supplement entries in the CIDSystemInfo dictionary
///
/// The supplement is deliberately not read, on the clause's own words two sentences later:
/// "This value shall not be used in determining compatibility between character collections."
///
/// One reader for the three questions that need it — what a CID *means* (§9.10.2 step b), which
/// script a substitute has to cover, and which of its glyphs are vertical forms — so that a font
/// cannot belong to one collection for one of them and another for the next. `None` where the
/// descendant states no `/CIDSystemInfo` or states one without both strings, which Table 115
/// makes a malformed descendant rather than a choice.
pub(crate) fn collection_names(
    document: &Document,
    descendant: &Dictionary,
) -> Option<(String, String)> {
    let info = document.get_key(descendant, "CIDSystemInfo");
    let info = info.as_dict()?;
    let text = |key: &str| {
        document
            .get_key(info, key)
            .as_string()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    };
    Some((text("Registry")?, text("Ordering")?))
}

/// Why [`collection_table`] answered `None`, in the file's own terms.
///
/// **One refusal carried four different facts about a file, and only the last of them is work
/// owed.** A substituted composite font with no `/ToUnicode` reaches §9.10.2's third method and
/// that method needs the collection's own table; when there is none, the message said
/// "neither a `/ToUnicode` nor a registered character collection" and stopped — so a file the
/// standard forbids outright, a file missing a required entry, a collection no table can exist
/// for, and a collection this binary happens not to carry all read alike. Trap 11's rule from
/// the other side: a report is only as good as the condition it fires on, and a report that
/// fires on four conditions has named none of them.
///
/// The four, in the order they are asked, sharpest first:
///
/// a) **The file states a combination §9.7.5.2 forbids.** With the program absent and the
///    `CMap` one of the two identity ones, the clause is about the *file*:
///
///    > The Identity-H and Identity-V CMaps shall not be used with a non-embedded font. Only
///    > standardized character sets may be used.
///
///    ADR 0433 read this population off the ink sweep by hand; this is that reading said by the
///    refusal itself.
/// b) **The descendant states no readable `/CIDSystemInfo`.** Table 115 makes it "( Required )",
///    so §9.10.2's step (b), quoted above [`collection_table`], has nothing to obtain.
/// c) **The collection's ordering is `Identity`.** §9.7.3 makes a character collection "an
///    ordered set of glyphs" whose order "shall determine the CID number for each glyph", so an
///    `Identity` ordering is the glyph order of the program the file did not embed. No table
///    could exist, and none is owed.
/// d) **This binary carries no table for the collection the file names**, which is the only one
///    of the four that is a gap in this reader. §9.7.5.2 requires four collections —
///    "A PDF processor shall support Adobe-CNS1-7, Adobe-GB1-5, Adobe-Japan1-7 and Adobe-KR-9
///    character collections" — and `predefined`'s own test asserts all four are carried, so what
///    reaches this is a collection beyond what the clause requires.
///
/// `encoding` is the Type 0 font's `/Encoding`, needed for (a) alone: the prohibition is stated
/// of the `CMap` rather than of the descendant. `font` is that same Type 0 dictionary, and
/// [`to_unicode_gap`] is what it is read for — the half-sentence every branch below ends with,
/// which is the **fifth** fact this refusal was carrying unseparated.
pub(crate) fn collection_gap(
    document: &Document,
    font: &Dictionary,
    descendant: &Dictionary,
    encoding: Option<&str>,
) -> String {
    let instead = to_unicode_gap(document, font);

    if let Some(name @ ("Identity-H" | "Identity-V")) = encoding {
        return format!(
            "the file states /Encoding /{name} over a descendant with no embedded program, \
             which §9.7.5.2 says shall not be used — a CID is then an index into a program \
             nobody supplied — and {instead}"
        );
    }

    let Some((registry, ordering)) = collection_names(document, descendant) else {
        return format!(
            "the descendant states no readable /CIDSystemInfo, which Table 115 makes required, \
             so §9.10.2 step (b) has no character collection to obtain — and {instead}"
        );
    };

    if ordering == "Identity" {
        return format!(
            "the descendant's character collection is {registry}-{ordering}, whose CIDs are the \
             glyph order of a program nobody supplied (§9.7.3), so no table can say what they \
             mean — and {instead}"
        );
    }

    format!(
        "this reader carries no CID-to-Unicode table for the character collection \
         {registry}-{ordering}, which is beyond the four §9.7.5.2 requires — and {instead}"
    )
}

/// Why §9.10.2's first method did not answer either, in the file's own terms.
///
/// **This is [`collection_gap`]'s fifth fact, and it was the half-sentence all four of the
/// others ended with.** Each of them said "it states no `/ToUnicode`", which is a claim about the
/// *file* rather than about this reader — and `issue11915.pdf` falsifies it. That document is a
/// font specimen: five lines naming five faces, each shown in a non-embedded `CIDFontType2` under
/// `/Encoding /Identity-H`, and each of its five Type 0 dictionaries states
///
/// ```text
/// /ToUnicode /Identity-H
/// ```
///
/// — a **name**, where §9.10.1 says of that entry that its
///
/// > value shall be a stream object containing a special kind of CMap file that maps character
/// > codes to Unicode values
///
/// and Table 119 types it `stream`. So the entry is there and is not the thing the clause names,
/// [`crate::loading::read_to_unicode`] rightly reads nothing out of it, and the refusal then
/// reported a file nobody has. ADR 0836's lesson one crate over and about a report rather than
/// about a decoder: **what a reader could not use is not the same statement as what a file did
/// not state.**
///
/// **The refusal is unchanged by any of this**, which is why this returns prose and not a
/// verdict. A name is not a stream; §9.10.3 defines a `/ToUnicode` `CMap` as the contents of one;
/// and no clause says what a name there would mean — Table 116's predefined `CMap`s map codes to
/// *CIDs*, so reading `/Identity-H` as "the code is already the Unicode value" is an invention,
/// however plausible the references make it look on a page whose codes happen to be UTF-16BE.
/// What the sentence buys is that the next reader of this message learns the file broke a second
/// `shall`, and that a round wanting to recover the construction knows where it is.
///
/// Three answers, and the third is not the name of a kind:
///
/// - **No entry.** The original sentence, now said only where it is true.
/// - **An entry that is not a stream.** The value's own kind, a name with its spelling, because
///   *which* name the producer wrote is the whole of what it was trying to say.
/// - **A stream that produced no mapping.** [`crate::loading::read_to_unicode`] answers empty for
///   a stream that does not decode and for a `CMap` that states nothing, and neither of those is
///   an absent entry either. **This bullet said "not because a corpus document reaches it" and the
///   golden corrected it in the same session**: `issue5801.pdf`, another of ADR 0433's eleven, has
///   a `/ToUnicode` that is a stream — and the stream is a copy of the *`Identity-H` CID* `CMap`,
///   all `begincidrange` and not one `beginbfchar`, so there is nothing for a `/ToUnicode` reader
///   to find in it. `examples/to_unicode_kind_census` counts **55 such streams over 27 documents**
///   against the sixteen names over ten, so the commoner of the two shapes is the one that was
///   almost written off. Trap 11 from both ends at once: a report that fires on a condition it
///   does not name, and a comment that asserted a population nobody had counted.
fn to_unicode_gap(document: &Document, font: &Dictionary) -> String {
    let object = document.get_key(font, "ToUnicode");
    let kind = match &object {
        Object::Null => {
            return "it states no /ToUnicode to read the codes by instead (§9.10.2)".to_owned();
        }
        Object::Stream(_) => {
            return "its /ToUnicode states no mapping to read the codes by instead (§9.10.2)"
                .to_owned();
        }
        Object::Name(name) => Cow::Owned(format!(
            "the name /{}",
            String::from_utf8_lossy(name.as_bytes())
        )),
        Object::Array(_) => Cow::Borrowed("an array"),
        Object::Dictionary(_) => Cow::Borrowed("a dictionary"),
        Object::String(_) => Cow::Borrowed("a string"),
        Object::Boolean(_) => Cow::Borrowed("a boolean"),
        Object::Integer(_) | Object::Real(_) => Cow::Borrowed("a number"),
        // `get_key` resolves indirect objects, and a dangling one answers `Null` rather than
        // itself, so this arm is the match's completeness rather than a file's.
        Object::Reference(_) => Cow::Borrowed("an unresolved reference"),
    };
    format!(
        "its /ToUnicode is {kind}, where §9.10.1 says the value \"shall be a stream object\" and \
         Table 119 types it a stream, so there is no CMap to read the codes by instead (§9.10.2)"
    )
}

#[cfg(test)]
mod tests {
    use super::composite_cmap;
    use crate::fixture::font_dictionary;

    /// **§7.3.5's `#` escapes are resolved before §9.7.5.2's name is looked up.**
    ///
    /// §7.3.5 makes the escape part of a name's *syntax* rather than a spelling variant — the
    /// clause's third rule is the NUMBER SIGN form — so `/Identity#2DH` and `/Identity-H` are
    /// one name, `2Dh` being the HYPHEN-MINUS. A reader that compared the raw bytes would find
    /// neither the horizontal identity `CMap` nor any predefined one.
    ///
    /// `hayro`'s issue 11 is that miss, and its symptom is worth recording because it is not
    /// the one you would guess: horizontal text came out laid down the page. §9.7.5.2 is why —
    /// the two identity `CMap`s differ only in their writing mode, so a name that is not
    /// recognised at all is one step from being taken for its twin, and the page comes out
    /// rotated rather than blank.
    ///
    /// This tree decodes the escape in `pdf_syntax`'s lexer, which is its only name reader, so
    /// the two spellings cannot diverge here. What is asserted is that they do not — including
    /// the writing mode, which is the half the symptom was about.
    #[test]
    fn an_escaped_cmap_name_is_the_name_it_spells() {
        for (spelling, expected_wmode) in [
            ("/Identity#2DH", 0),
            ("/Identity-H", 0),
            ("/Identity#2DV", 1),
            ("/Identity-V", 1),
            // Every character may be escaped, not only the ones that must be: `49h` is `I`
            // and `74h` is `t`, so this is the same name spelled the long way round.
            ("/#49dent#69ty#2DH", 0),
        ] {
            let (document, dict) = font_dictionary(&format!("/Encoding {spelling}"));
            let cmap = composite_cmap(&document, &dict, "T")
                .unwrap_or_else(|e| panic!("{spelling} is a name §9.7.5.2 defines, but: {e}"));
            assert_eq!(
                cmap.wmode(),
                expected_wmode,
                "{spelling} names the identity CMap whose writing mode is {expected_wmode}"
            );
        }
    }

    /// [`collection_gap`]'s four facts are four sentences, not one.
    ///
    /// The refusal they feed said "neither a `/ToUnicode` nor a registered character collection"
    /// for all four, so a file the standard forbids outright, a file missing a required entry, a
    /// collection no table could exist for, and a collection this binary happens not to carry
    /// read alike — and the corpus gate's classification of them was therefore a guess. Each row
    /// below asserts the phrase its own case is named by *and* that no other row's phrase
    /// appears, which is what makes this a test of the split rather than of the wording.
    ///
    /// Every row states no `/ToUnicode`, so every one of them ends in [`to_unicode_gap`]'s first
    /// answer — which is the sentence all four used to end in unconditionally, and is asserted
    /// here so that the split above stays a test of the *other* four facts.
    ///
    /// Calibrated (trap 13) by making the function return one constant: all four rows fail.
    #[test]
    fn four_facts_about_a_file_reach_four_different_refusals() {
        let cases = [
            (
                Some("Identity-H"),
                "/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>",
                "§9.7.5.2 says shall not be used",
            ),
            (None, "/Type /Font", "no readable /CIDSystemInfo"),
            (
                Some("90ms-RKSJ-H"),
                "/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>",
                "glyph order of a program nobody supplied",
            ),
            (
                Some("90ms-RKSJ-H"),
                "/CIDSystemInfo << /Registry (Adobe) /Ordering (Japan2) /Supplement 0 >>",
                "carries no CID-to-Unicode table",
            ),
        ];
        let phrases: Vec<&str> = cases.iter().map(|(_, _, phrase)| *phrase).collect();
        for (encoding, entries, expected) in cases {
            let (document, descendant) = font_dictionary(entries);
            let said = super::collection_gap(&document, &descendant, &descendant, encoding);
            for phrase in &phrases {
                assert_eq!(
                    said.contains(phrase),
                    *phrase == expected,
                    "{encoding:?} over {entries} should say only {expected:?}, and said: {said}"
                );
            }
            assert!(
                said.contains("it states no /ToUnicode to read the codes by instead"),
                "a dictionary with no /ToUnicode entry says so: {said}"
            );
        }
    }

    /// And the fifth fact: a `/ToUnicode` that is there and is not a stream.
    ///
    /// `issue11915.pdf` is the witness and it states `/ToUnicode /Identity-H` on all five of its
    /// Type 0 dictionaries. §9.10.1 requires that entry's "value shall be a stream object" and
    /// Table 119 types it a stream, so what the file states is not a `CMap` — but it is also not
    /// *nothing*, which is what the refusal said for a hundred sessions. The name is printed with
    /// its spelling because which name the producer wrote is the whole of what it meant.
    ///
    /// The second row is the control and it is the one that makes this a test of the condition
    /// rather than of the wording: the identical dictionary with the entry removed goes back to
    /// the original sentence, and neither row may say the other's.
    ///
    /// Calibrated (trap 13) by making [`to_unicode_gap`] answer its first arm unconditionally:
    /// the first row fails and the second still passes.
    #[test]
    fn a_to_unicode_that_is_not_a_stream_is_not_an_absent_one() {
        let identity = "/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>";
        let (document, with_name) = font_dictionary(&format!("{identity} /ToUnicode /Identity-H"));
        let said = super::collection_gap(&document, &with_name, &with_name, Some("Identity-H"));
        assert!(
            said.contains("its /ToUnicode is the name /Identity-H")
                && said.contains("shall be a stream object"),
            "the entry is reported as what it is: {said}"
        );
        assert!(
            !said.contains("states no /ToUnicode"),
            "and not as an entry the file never wrote: {said}"
        );

        let (document, without) = font_dictionary(identity);
        let said = super::collection_gap(&document, &without, &without, Some("Identity-H"));
        assert!(
            said.contains("it states no /ToUnicode to read the codes by instead"),
            "and the same dictionary without the entry says the original sentence: {said}"
        );
        assert!(
            !said.contains("shall be a stream object"),
            "and nothing about a type nobody wrote: {said}"
        );
    }
}
