//! ISO 32000-2 §9.7.5.2's predefined `CMap`s, compiled into the binary.
//!
//! Table 116 names some seventy `CMap`s a document may reference without carrying them, and
//! §9.7.5.2 states the obligation as a `shall`:
//!
//! > A PDF processor shall support Adobe-CNS1-7, Adobe-GB1-5, Adobe-Japan1-7 and Adobe-KR-9
//! > character collections.
//!
//! The clause is equally plain about where the data lives:
//!
//! > The CMap programs that define the predefined CMaps are available through a variety of
//! > online sources.
//!
//! So this is the same shape as [`crate::standard`]: a requirement whose answer is *data*, met
//! by carrying the data rather than by guessing. Until the hundred-and-fifty-sixth session a
//! font naming one of these was refused and reported — thirteen corpus documents — which was
//! honest and was not the clause.
//!
//! # What is here
//!
//! All 239 files Adobe publishes for the six character collections, as
//! `data/cmaps/`; `build.rs` deflates each on its own and this module inflates one when a
//! document asks for it by name. **Nothing is decompressed at startup**, which is `CLAUDE.md`'s
//! rule for compiled-in data: a document that names no predefined `CMap` pays for a `static`
//! array of 239 tuples and not one byte of inflate.
//!
//! # Why the files as they are rather than a compacted form
//!
//! [`crate::cmap::CMap::parse`] already reads this syntax, because §9.7.5.4's embedded `CMap`
//! streams are written in it — so shipping Adobe's own bytes needs no converter, no second
//! format to keep in step, and no opportunity to mistranslate a mapping. What it costs is
//! parse time on first use, measured in the ADR beside the alternative.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::cmap::CMap;

include!(concat!(env!("OUT_DIR"), "/cmaps.rs"));

/// The packed `CMap` data, one deflate stream per entry of [`PREDEFINED`].
static PACKED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cmaps.bin"));

/// Bounds the `usecmap` chain a predefined `CMap` may build on.
///
/// Adobe's own files nest one deep — a vertical `CMap` on its horizontal twin, a half-width
/// variant on its proportional one — and this is the same bound `read_cmap` puts on the
/// embedded form.
const MAX_DEPTH: u32 = 4;

/// Returns the predefined `CMap` of that name, or `None` if this binary does not carry one.
///
/// The result is memoised: a `CMap` of 20 000 ranges costs milliseconds to parse and a document
/// setting the same font on twenty pages would otherwise pay for each. The cache is keyed by
/// name, which is the whole of a predefined `CMap`'s identity — unlike a resource name, which
/// §7.8.3 scopes to the dictionary that defines it (ADR 0115).
#[must_use]
pub fn cmap(name: &str) -> Option<CMap> {
    resolve(name, 0)
}

/// Whether this binary carries a predefined `CMap` of that name.
///
/// Cheaper than [`cmap`] for a caller that only has to decide whether to refuse, and it
/// inflates nothing.
#[must_use]
pub fn exists(name: &str) -> bool {
    entry(name).is_some()
}

/// §9.10.2 step c)'s `registry-ordering-UCS2` `CMap`, which maps a CID to a Unicode value.
///
/// > c. Construct a second CMap name by concatenating the registry and ordering obtained in
/// > step (b) in the format registry -ordering -UCS2 (for example, Adobe -Japan1 -UCS2).
///
/// Step d) then says to obtain that `CMap`, and names an online source for it — which is the
/// same sentence §9.7.5.2 uses about the predefined `CMap`s themselves, and the reason both
/// sets are vendored here rather than derived.
///
/// Parsed by [`crate::tounicode::ToUnicode`] rather than by [`CMap`], and that is the files'
/// own doing: they state `/CMapType 2` and write their mappings with `beginbfchar` and
/// `beginbfrange`, which §9.10.3 makes the `/ToUnicode` form. So the reader this tree already
/// had for a producer's `/ToUnicode` reads Adobe's CID tables unchanged — the *keys* are CIDs
/// rather than character codes, which is the caller's business and not the parser's.
///
/// `None` for a collection this binary does not carry, which includes every registry that is
/// not Adobe's.
#[must_use]
pub fn cid_to_unicode(registry: &str, ordering: &str) -> Option<crate::tounicode::ToUnicode> {
    unicode_cmap(&format!("{registry}-{ordering}-UCS2"))
}

/// One of Adobe's `CMap`s read as a `/ToUnicode` map, by its published name.
///
/// The same files [`cid_to_unicode`] reads, reached the other way round: by the name a
/// document states rather than by the collection a descendant declares. §9.10.3 permits a
/// `/ToUnicode` `CMap` to be "based on another `ToUnicode` `CMap`", and the ones a producer can
/// name without carrying them are these.
///
/// A name whose file states no `bfchar` or `bfrange` — every CID `CMap` in the set — parses to
/// an empty map and is answered as `None`, so naming one buys nothing rather than being read as
/// a mapping it is not.
#[must_use]
pub fn unicode_cmap(name: &str) -> Option<crate::tounicode::ToUnicode> {
    resolve_unicode(name, 0)
}

/// [`unicode_cmap`], bounding the `usecmap` chain a file may build.
fn resolve_unicode(name: &str, depth: u32) -> Option<crate::tounicode::ToUnicode> {
    if depth > MAX_DEPTH {
        return None;
    }
    if let Some(held) = ucs2_cache()
        .lock()
        .ok()
        .and_then(|held| held.get(name).cloned())
    {
        return held;
    }
    let built = inflate(name)
        .map(|source| {
            // Adobe's own half-width and vertical variants state only their differences and
            // name the file they differ from, exactly as §9.10.3 lets a producer do.
            let used =
                used_by(&source).and_then(|used| resolve_unicode(&used, depth.saturating_add(1)));
            crate::tounicode::ToUnicode::parse_on(&source, used)
        })
        .filter(|table| !table.is_empty());
    if let Ok(mut held) = ucs2_cache().lock() {
        let _ = held.insert(name.to_owned(), built.clone());
    }
    built
}

/// Whether a CID is the *vertical form* of a character in its own character collection.
///
/// # What the question is, and why the collection can answer it
///
/// §9.7.5.1's NOTE says that "in some cases, different shapes are used when writing horizontally
/// and vertically", and that "the horizontal and vertical variants of a `CMap` specify different
/// CIDs for a given character code". Table 116 then publishes both variants for each collection
/// — `UniJIS-UCS2-H` and, in the row below it, "Vertical version of UniJIS-UCS2-H" — so the pair
/// *is* the collection's statement of which characters have a distinct vertical form and which
/// CID each one is.
///
/// This asks that pair the question directly: the vertical `CMap` sends `character` to `cid` and
/// the horizontal one sends it somewhere else. Both halves matter. Without the first, a producer
/// that chose the *horizontal* CID in a vertical `CMap` — which is legal, and is a statement —
/// would be answered with a rotated glyph it did not ask for. Without the second, every kanji on
/// the page would qualify, because a character with no vertical form is sent to one CID by both.
///
/// # Where it is used, and where it is not
///
/// Only [`crate::LoadedFont`]'s substituted composite route: a font whose program the document
/// embedded reaches the producer's own glyph through the CID and needs nothing here. See
/// [`crate::vertical`] for the other half — asking the substitute face which of its glyphs is
/// that form.
///
/// `false` for a registry that is not Adobe's, for a collection Table 116 publishes no vertical
/// `CMap` for, and for a character outside the basic multilingual plane. The last of those is
/// the encoding these two files are keyed by — §9.7.5.2 says that "`CMap` names containing UCS2 use
/// UCS-2 encoding" — and UCS-2 has no code for a character above U+FFFF.
#[must_use]
pub fn is_vertical_form(registry: &str, ordering: &str, character: char, cid: u32) -> bool {
    let Some(names) = unicode_pair(registry, ordering) else {
        return false;
    };
    let Ok(code) = u16::try_from(u32::from(character)) else {
        return false;
    };
    let Some(pair) = unicode_pair_maps(names) else {
        return false;
    };
    let (horizontal, vertical) = (&pair.0, &pair.1);
    let bytes = code.to_be_bytes();
    let upright = horizontal.cid(horizontal.next_code(&bytes));
    let downward = vertical.cid(vertical.next_code(&bytes));
    downward == Some(cid) && upright != Some(cid)
}

/// Whether Table 116 publishes a vertical `CMap` for this character collection at all.
///
/// [`is_vertical_form`] answers `false` for a collection without one, so this changes no answer;
/// what it saves is reading a substitute's `GSUB` for a font whose CIDs no table can rank. An
/// `Identity` ordering is the population that matters — §9.7.3 makes its CIDs "the glyph order of
/// a program nobody supplied", so no published pair could say which of them are vertical forms.
#[must_use]
pub fn has_vertical_forms(registry: &str, ordering: &str) -> bool {
    unicode_pair(registry, ordering).is_some()
}

/// Table 116's Unicode `CMap` pair for a character collection: horizontal, then vertical.
///
/// One row per collection the table publishes both variants for, and the names are the table's
/// own. **Adobe-KR has no row and that is the table's doing rather than an omission here**: the
/// collection §9.7.5.2 requires alongside the other three is published with `UniAKR-UTF16-H` and
/// no vertical counterpart, so there is no pair to compare and no CID it could name.
/// Adobe-Japan2 has none either, being deprecated in this edition.
fn unicode_pair(registry: &str, ordering: &str) -> Option<(&'static str, &'static str)> {
    if registry != "Adobe" {
        return None;
    }
    Some(match ordering {
        "Japan1" => ("UniJIS-UCS2-H", "UniJIS-UCS2-V"),
        "GB1" => ("UniGB-UCS2-H", "UniGB-UCS2-V"),
        "CNS1" => ("UniCNS-UCS2-H", "UniCNS-UCS2-V"),
        "Korea1" => ("UniKS-UCS2-H", "UniKS-UCS2-V"),
        _ => return None,
    })
}

/// One collection's pair of Unicode `CMap`s, inflated and parsed once.
type CMapPair = Arc<(CMap, CMap)>;

/// The pairs [`is_vertical_form`] has been asked for.
///
/// A memo of its own rather than [`cmap`]'s, because [`cmap`] hands out a **clone** — which is
/// right for a caller that keeps the map on a font and wrong for one asking a question per glyph
/// drawn. `UniJIS-UCS2-H` is twenty thousand ranges.
static UNICODE_PAIRS: OnceLock<RwLock<HashMap<&'static str, Option<CMapPair>>>> = OnceLock::new();

/// Builds or recalls one collection's pair, keyed by the vertical name.
fn unicode_pair_maps(names: (&'static str, &'static str)) -> Option<CMapPair> {
    let memo = UNICODE_PAIRS.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(held) = memo.read()
        && let Some(found) = held.get(names.1)
    {
        return found.clone();
    }
    let built = cmap(names.0)
        .zip(cmap(names.1))
        .map(|(horizontal, vertical)| Arc::new((horizontal, vertical)));
    if let Ok(mut held) = memo.write() {
        let _ = held.insert(names.1, built.clone());
    }
    built
}

/// The names this binary carries, in order, for a test that wants to walk them all.
pub fn names() -> impl Iterator<Item = &'static str> {
    PREDEFINED.iter().map(|(name, ..)| *name)
}

/// Builds one `CMap`, resolving its `usecmap` reference by name.
fn resolve(name: &str, depth: u32) -> Option<CMap> {
    if depth > MAX_DEPTH {
        return None;
    }
    if let Some(held) = cache().lock().ok().and_then(|held| held.get(name).cloned()) {
        return held;
    }

    let built = inflate(name).map(|source| {
        // Adobe's files name what they build on in a `usecmap` line — `/90ms-RKSJ-H usecmap`
        // — which is the same operator §9.7.5.4 a) describes for an embedded stream. Here
        // there is no `/UseCMap` entry to cross-check it against, because there is no stream
        // dictionary; the name in the file is the only statement of the relationship and is
        // therefore the one followed.
        let used = used_by(&source).and_then(|used| resolve(&used, depth.saturating_add(1)));
        CMap::parse(&source, used.as_ref())
    });
    if let Ok(mut held) = cache().lock() {
        let _ = held.insert(name.to_owned(), built.clone());
    }
    built
}

/// The `CMap` a file's `usecmap` operator names, if it has one.
///
/// Read here rather than by the parser because the parser is handed bytes and has no way to
/// fetch what they reference; [`crate::cmap::CMap::references_another`] is its half of the
/// same fact, and is what refuses an embedded `CMap` whose reference cannot be found.
///
/// Shared with the embedded form: §9.7.5.4 a) requires an in-file `usecmap` reference to be
/// named by the stream dictionary's `/UseCMap` as well, so on a conforming file the two say the
/// same thing and reading the file's own statement can only agree with the dictionary's.
pub(crate) fn used_by(source: &[u8]) -> Option<String> {
    let at = source
        .windows(b"usecmap".len())
        .position(|window| window == b"usecmap")?;
    let before = source.get(..at)?;
    let slash = before.iter().rposition(|byte| *byte == b'/')?;
    let named = before.get(slash.saturating_add(1)..)?;
    let named: Vec<u8> = named
        .iter()
        .copied()
        .take_while(|byte| !byte.is_ascii_whitespace())
        .collect();
    (!named.is_empty()).then(|| String::from_utf8_lossy(&named).into_owned())
}

/// The index entry for a name.
fn entry(name: &str) -> Option<&'static (&'static str, usize, usize, usize)> {
    let at = PREDEFINED
        .binary_search_by(|(held, ..)| (*held).cmp(name))
        .ok()?;
    PREDEFINED.get(at)
}

/// Inflates one entry's bytes.
fn inflate(name: &str) -> Option<Vec<u8>> {
    use std::io::Read as _;

    let (_, offset, packed, plain) = *entry(name)?;
    let source = PACKED.get(offset..offset.checked_add(packed)?)?;
    let mut into = Vec::with_capacity(plain);
    let read = flate2::read::DeflateDecoder::new(source)
        .take(plain as u64)
        .read_to_end(&mut into)
        .ok()?;
    (read == plain).then_some(into)
}

/// The memo, built on first use.
fn cache() -> &'static Mutex<HashMap<String, Option<CMap>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<CMap>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The same memo for §9.10.2's CID tables, which are the largest files here — 19 000 lines
/// for Adobe-Japan1 — and are asked for once per font rather than once per document.
///
/// Keyed by the published file name rather than by a registry-and-ordering pair, because
/// [`unicode_cmap`] reaches the same files by name and the two must share one memo.
type Ucs2Cache = Mutex<HashMap<String, Option<crate::tounicode::ToUnicode>>>;
fn ucs2_cache() -> &'static Ucs2Cache {
    static CACHE: OnceLock<Ucs2Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::{cid_to_unicode, cmap, exists, names, used_by};

    /// §9.10.2 step (e), on the collection its own example names: CID 1 of Adobe-Japan1 is the
    /// space, which the file states as `<0001> <003c> <0020>` — a range beginning at U+0020.
    #[test]
    fn a_cid_reaches_a_character_through_the_collections_own_table() {
        let table = cid_to_unicode("Adobe", "Japan1").expect("Adobe-Japan1-UCS2 is carried");
        assert_eq!(table.char_for(1), Some(' '));
        // `<0000> <fffd>`: CID 0 is .notdef, which the table gives as the replacement
        // character rather than as nothing.
        assert_eq!(table.char_for(0), Some('\u{fffd}'));
        // A registry nobody publishes a table for is a `None` rather than a guess.
        assert!(cid_to_unicode("Fontworks", "Japan1").is_none());
    }

    /// The whole of §9.10.2's third method, composed: a Shift-JIS code becomes a CID and the
    /// CID becomes the character. 0x8140 is the ideographic space.
    #[test]
    fn a_code_reaches_a_character_through_a_cid() {
        let map = cmap("90ms-RKSJ-H").expect("carried");
        let table = cid_to_unicode("Adobe", "Japan1").expect("carried");
        let cid = map.cid(map.next_code(b"\x81\x40")).expect("a CID");
        assert_eq!(table.char_for(cid), Some('\u{3000}'));
    }

    /// The four collections §9.7.5.2 states as a `shall` are represented, named one by one so
    /// that a set that lost one would say which.
    #[test]
    fn the_collections_the_clause_requires_are_carried() {
        for name in [
            "UniCNS-UTF16-H",
            "UniGB-UTF16-H",
            "UniJIS-UTF16-H",
            "UniKS-UTF16-H",
            "Adobe-CNS1-UCS2",
            "Adobe-GB1-UCS2",
            "Adobe-Japan1-UCS2",
            "Adobe-KR-UCS2",
        ] {
            assert!(exists(name), "{name} is not carried");
        }
    }

    /// Table 116's own example: `90ms-RKSJ-H` is Microsoft Code Page 932, so ASCII passes
    /// through as single-byte codes and a two-byte Shift-JIS code is a CJK CID.
    ///
    /// The expected values are the file's own `cidrange` lines, which is the only source there
    /// can be for them — the clause states no mapping, it states which *collection* the mapping
    /// is into.
    #[test]
    fn a_shift_jis_cmap_decodes_both_its_code_lengths() {
        let map = cmap("90ms-RKSJ-H").expect("90ms-RKSJ-H is carried");
        assert!(map.has_codespace() && map.has_mappings());
        // `<20> <7e> 231`: the code for a space is CID 231, taken as one byte.
        assert_eq!(map.cid(map.next_code(b"\x20")), Some(231));
        // `<8140> <817e> 633`: the two-byte range beginning at 0x8140.
        assert_eq!(map.cid(map.next_code(b"\x81\x40")), Some(633));
        assert_eq!(map.wmode(), 0);
    }

    /// A vertical `CMap` is its horizontal twin plus a handful of substitutions, expressed with
    /// `usecmap`. Following that reference is the whole reason this module resolves by name.
    #[test]
    fn a_vertical_cmap_inherits_from_its_horizontal_twin() {
        let horizontal = cmap("90ms-RKSJ-H").expect("carried");
        let vertical = cmap("90ms-RKSJ-V").expect("carried");
        assert_eq!(vertical.wmode(), 1);
        // Inherited: the vertical file states no mapping for ASCII at all.
        let space = b"\x20";
        assert_eq!(
            vertical.cid(vertical.next_code(space)),
            horizontal.cid(horizontal.next_code(space))
        );
        // Its own: 0x8141 is the ideographic comma, which the vertical form rotates.
        let comma = b"\x81\x41";
        assert_ne!(
            vertical.cid(vertical.next_code(comma)),
            horizontal.cid(horizontal.next_code(comma))
        );
    }

    /// Every carried file parses into something usable, which is what says the set is not
    /// half data and half prose.
    #[test]
    fn every_carried_cmap_states_a_codespace_and_a_mapping() {
        for name in names() {
            let map = cmap(name).unwrap_or_else(|| panic!("{name} inflates and parses"));
            assert!(
                map.has_codespace(),
                "{name} states no codespace range, so no code could be extracted from a string"
            );
            assert!(
                map.has_mappings(),
                "{name} states no mapping, so every code would be .notdef"
            );
        }
    }

    /// §9.7.5.1's NOTE, read off the two files Table 116 publishes for one collection.
    ///
    /// The collection is Adobe-Japan1 and the character is U+300C LEFT CORNER BRACKET, whose
    /// two CIDs are what `VerticalText.pdf` is about: `UniJIS-UCS2-H` sends the character to
    /// 686 and `UniJIS-UCS2-V` to 7911, so 7911 is its vertical form and 686 is not. **Neither
    /// number is written anywhere but here and in Adobe's own files**, which is what makes this
    /// a reading of the data rather than a table somebody typed.
    ///
    /// The negative rows are the half that keeps the rule from firing on everything: 縦 has one
    /// CID in both files, so it has no vertical form at all, and a CID belonging to another
    /// character is not this character's form however vertical it is elsewhere.
    #[test]
    fn a_collection_says_which_of_two_cids_is_the_vertical_form() {
        use super::is_vertical_form;

        // U+300C: 7911 in the vertical file, 686 in the horizontal one.
        assert!(is_vertical_form("Adobe", "Japan1", '\u{300c}', 7911));
        assert!(!is_vertical_form("Adobe", "Japan1", '\u{300c}', 686));
        // U+3002 IDEOGRAPHIC FULL STOP, the other shape `VerticalText.pdf` shows.
        assert!(is_vertical_form("Adobe", "Japan1", '\u{3002}', 7888));
        assert!(!is_vertical_form("Adobe", "Japan1", '\u{3002}', 635));
        // 縦 is the same CID in both files, so neither of its readings is a vertical form.
        assert!(!is_vertical_form("Adobe", "Japan1", '\u{7e26}', 2382));
        // 7911 is U+300C's form and not U+3002's.
        assert!(!is_vertical_form("Adobe", "Japan1", '\u{3002}', 7911));
        // A collection Table 116 publishes no vertical CMap for, and a registry that is not
        // Adobe's, are both `false` rather than a guess.
        assert!(!is_vertical_form("Adobe", "Identity", '\u{300c}', 7911));
        assert!(!is_vertical_form("Fontworks", "Japan1", '\u{300c}', 7911));
    }

    /// Every collection this reader names a vertical `CMap` for has both halves of the pair.
    ///
    /// A row naming a file this binary does not carry would make the whole rule silently
    /// inapplicable for that collection, which is the shape of failure nothing else here could
    /// see: the answer would be `false` for every CID and the page would draw exactly as it did
    /// before.
    #[test]
    fn each_collections_vertical_pair_is_carried() {
        for ordering in ["Japan1", "GB1", "CNS1", "Korea1"] {
            let (horizontal, vertical) =
                super::unicode_pair("Adobe", ordering).expect("a row for the collection");
            assert!(exists(horizontal), "{horizontal} is not carried");
            assert!(exists(vertical), "{vertical} is not carried");
            let pair = super::unicode_pair_maps((horizontal, vertical))
                .expect("both halves of the pair parse");
            assert_eq!(pair.0.wmode(), 0, "{horizontal} is the horizontal half");
            assert_eq!(pair.1.wmode(), 1, "{vertical} is the vertical half");
        }
    }

    /// The `usecmap` line is read out of the file's bytes, so the reader has to find the name
    /// before the operator and stop at the whitespace after it.
    #[test]
    fn a_usecmap_line_names_the_map_it_builds_on() {
        assert_eq!(
            used_by(b"/90ms-RKSJ-H usecmap\n/CMapName /90ms-RKSJ-V def"),
            Some("90ms-RKSJ-H".to_owned())
        );
        assert_eq!(used_by(b"/CMapName /H def\nbegincidrange"), None);
    }
}
