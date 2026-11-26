//! The classes of repair this crate's readers make, on `pdf_syntax`'s one channel (ADR 3477,
//! ADR 3565).
//!
//! # Why this crate declares its own, and records them itself
//!
//! `pdf_syntax::repair` holds the channel — [`pdf_syntax::RepairClass`], [`pdf_syntax::Site`] and
//! the log on the [`pdf_syntax::Document`]. A font is read at a level that layer cannot name: a
//! `loca` table whose offsets do not ascend, a `/ToUnicode` map that stops in the middle of a
//! section, a `/W` array of the wrong shape. Their classes are declared here and listed in
//! [`CLASSES`], so a validator maps every class by its stable name.
//!
//! Every reader in this crate that can repair is handed the document it reads from — that is how
//! [`crate::LoadedFont::load`] resolves a reference — so the reader records the repair where it
//! makes it, through `noted`, rather than handing it to every caller to record: the viewer's
//! font cache, the validator and the converter all load fonts, and a class only some of them
//! recorded would be a repair the log hears of by accident. The byte-level parsers below the
//! loader (`sfnt`, `cmap`, `tounicode`) are pure functions of a program's or a map's bytes and
//! say what they repaired in their return value, as `pdf_syntax`'s lexer does; the loader that
//! knows which stream the bytes came from records it.
//!
//! Where a repair is recorded: at [`pdf_syntax::Site::Object`] of the stream or entry that holds
//! the departure when the file writes it as an indirect object, else at the font's descriptor —
//! which Table 109 and Table 117 require to be an indirect reference — and only where neither is
//! one at [`pdf_syntax::Site::WholeFile`] (`site`).
//!
//! What is **not** a class is said beside the code, with a `// not a repair:` comment naming the
//! clause that makes the reading the specification's own.

use pdf_syntax::{Dictionary, Document, RepairClass, Site};

macro_rules! classes {
    ($($(#[$doc:meta])* $item:ident = $name:literal, $clause:literal, $instead:literal;)*) => {
        $(
            $(#[$doc])*
            pub static $item: RepairClass = RepairClass {
                name: $name,
                clause: $clause,
                read_instead: $instead,
            };
        )*
        /// Every class this crate records, in declaration order (ADR 3477).
        pub static CLASSES: &[&RepairClass] = &[$(&$item),*];
    };
}

classes! {
    // ---- the embedded program, §9.9 ---------------------------------------------------------

    /// A `loca` table whose directory record is shorter than `maxp`'s glyph count needs.
    SFNT_LOCA_EXTENT = "font/sfnt-loca-extent", "ISO 32000-2 §9.9",
        "an embedded TrueType program whose loca table is shorter than its glyph count needs was \
         read with the table's length raised to that extent";
    /// A `head.indexToLocFormat` that is neither 0 nor 1, or that `loca`'s own length contradicts.
    SFNT_LOCA_FORMAT = "font/sfnt-loca-format", "ISO 32000-2 §9.9",
        "an embedded TrueType program whose head table states an indexToLocFormat its loca table \
         contradicts was read in the format the loca and glyf tables agree on, or as written";
    /// A `loca` whose offsets do not ascend.
    SFNT_LOCA_ORDER = "font/sfnt-loca-order", "ISO 32000-2 §9.9",
        "an embedded TrueType program whose loca offsets do not ascend was read with glyf and loca \
         rebuilt in ascending order";
    /// An `sfnt` whose table directory names a table that runs past the program's end.
    SFNT_TABLE_PAST_THE_PROGRAM = "font/sfnt-table-past-the-program", "ISO 32000-2 §9.9",
        "an embedded TrueType or OpenType program whose table directory names a table past the \
         program's last byte was read as far as its bytes go, each glyph past the cut drawn as \
         nothing";
    /// A `/FontFile2` or `/FontFile3` holding a TrueType collection rather than one font.
    COLLECTION_CUT_TO_ONE_FACE = "font/collection-cut-to-one-face", "ISO 32000-2 §9.9",
        "an embedded program that is a TrueType collection was read as the one face its \
         descriptor's /FontName names, or as its first face where no face is so named";
    /// A `/FontFile3` whose `/Subtype` names a format other than the one its bytes are.
    FONT_FILE_SUBTYPE_DISAGREES = "font/font-file-subtype-disagrees", "ISO 32000-2 §9.9",
        "a /FontFile3 stream whose /Subtype disagrees with its program's own leading bytes was \
         read as the format the bytes are";
    /// A CID-keyed CFF program whose Font DICTs cannot be read as the program states them.
    CFF_FONT_DICTS_REWRITTEN = "font/cff-font-dicts-rewritten", "ISO 32000-2 §9.9",
        "a CID-keyed CFF program whose FDArray or Font DICTs could not be read was read with \
         rewritten Font DICTs, each unreadable one given an empty Private DICT";
    /// An embedded program that does not read at all, drawn with a substitute face.
    PROGRAM_UNREADABLE = "font/program-unreadable", "ISO 32000-2 §9.9",
        "an embedded font program that could not be read was set aside and the font drawn with a \
         substitute face";

    // ---- the maps and the metrics the font dictionary states ---------------------------------

    /// A `/ToUnicode` entry that is present and is not a stream.
    TO_UNICODE_NOT_A_STREAM = "font/to-unicode-not-a-stream", "ISO 32000-2 §9.10.3",
        "a /ToUnicode entry that is not a stream was read as no map";
    /// A `/ToUnicode` `CMap` whose sections stop part way or whose entries do not read.
    TO_UNICODE_DAMAGED = "font/to-unicode-damaged", "ISO 32000-2 §9.10.3",
        "a /ToUnicode CMap with a section left open or an entry that does not read was read as \
         the mappings that did";
    /// An embedded `CMap` whose sections stop part way or whose entries do not read.
    CMAP_DAMAGED = "font/cmap-damaged", "ISO 32000-2 §9.7.5.3",
        "an embedded CMap with a section left open or an entry that does not read was read as \
         the mappings that did";
    /// A `/Widths` array whose elements are not all numbers.
    WIDTHS_MALFORMED = "font/widths-malformed", "ISO 32000-2 §9.6.2.1",
        "a /Widths array with an element that is not a number was read with that code given the \
         font's missing width";
    /// A `/W` or `/W2` array that is not of the shape §9.7.4.3 states.
    CID_WIDTHS_MALFORMED = "font/cid-widths-malformed", "ISO 32000-2 §9.7.4.3",
        "a /W or /W2 array not of the shape the clause states was read as far as its first \
         malformed entry, every CID after it given the default metrics";
    /// A `/Differences` array with an element that is neither a code nor a glyph name in place.
    DIFFERENCES_MALFORMED = "font/differences-malformed", "ISO 32000-2 §9.6.5.1",
        "a /Differences array with an element that is neither an integer nor a name, or a name \
         before any code, was read with that element skipped";
    /// An `/Encoding` or `/BaseEncoding` name outside the three the clause permits, other than
    /// the standard encoding's, which is [`ENCODING_NAME_STANDARD`].
    ENCODING_NAME_UNKNOWN = "font/encoding-name-unknown", "ISO 32000-2 §9.6.5.1",
        "an /Encoding or /BaseEncoding naming none of the predefined encodings was read as no \
         base encoding, the program's own standing in its place";
    /// An `/Encoding` or `/BaseEncoding` naming the standard encoding, which Table D.1 says no
    /// processor has a predefined encoding under (ADR 3753). A class of its own because what it
    /// breaches depends on the font: a nonsymbolic `TrueType` font's name is the one ISO 19005
    /// asks it to restate as a predefined one.
    ENCODING_NAME_STANDARD = "font/encoding-name-standard", "ISO 32000-2 §D.1",
        "an /Encoding or /BaseEncoding naming StandardEncoding was read as no base encoding, \
         the program's own standing in its place";
    /// A font descriptor's `/FontWeight` or `/FontStretch` stated as no value Table 120 admits.
    DESCRIPTOR_STYLE_UNUSABLE = "font/descriptor-style-unusable", "ISO 32000-2 §9.8.1",
        "a /FontWeight outside 1 to 1000 or a /FontStretch Table 120 does not name was read as \
         absent where a substitute face was chosen";
    /// An embedded `CMap`'s `/WMode` of another type, or neither 0 nor 1.
    WMODE_UNUSABLE = "font/wmode-unusable", "ISO 32000-2 §9.7.5.3",
        "a CMap stream's /WMode that is not the integer 0 or 1 was read as the nearer of the two, \
         or as 0";

    // ---- bounds of this reader ---------------------------------------------------------------

    /// A `/ToUnicode` chain or map past a bound this reader keeps.
    TO_UNICODE_PAST_A_BOUND = "font/to-unicode-past-a-bound", "ISO 32000-2 §9.10.3",
        "a /ToUnicode CMap past the nesting depth or the entry count this reader keeps was read \
         as far as the bound";
}

/// Records a repair of `class` at `at` on the document read from (ADR 3565).
///
/// Cold: every caller is on a path a well-formed font never takes, so the common path costs the
/// test that led here and nothing else.
#[cold]
pub(crate) fn noted(document: &Document, class: &'static RepairClass, at: Site) {
    document.record_repair(class, at);
}

/// Where a repair of the entry `key` of `dict` is placed: the entry itself where the file writes
/// it as an indirect object, else the font's descriptor, else the whole file.
///
/// Table 109 and Table 117 require `/FontDescriptor` to be an indirect reference, so a conforming
/// font always has an object to name; a font that writes neither is placed at the file, which is
/// still a finding a validator places and reads.
pub(crate) fn site(dict: &Dictionary, key: &str) -> Site {
    dict.get(key)
        .and_then(pdf_syntax::Object::as_reference)
        .or_else(|| {
            dict.get("FontDescriptor")
                .and_then(pdf_syntax::Object::as_reference)
        })
        .map_or(Site::WholeFile, Site::Object)
}

#[cfg(test)]
mod tests {
    use super::CLASSES;

    /// Every name is this crate's, unique, and every clause is ISO 32000-2's.
    #[test]
    fn every_class_is_named_once_under_this_crate() {
        let mut names = std::collections::BTreeSet::new();
        for class in CLASSES {
            assert!(class.name.starts_with("font/"), "{}", class.name);
            assert!(
                class.clause.starts_with("ISO 32000-2 \u{a7}"),
                "{}",
                class.name
            );
            assert!(names.insert(class.name), "{} twice", class.name);
        }
    }
}
