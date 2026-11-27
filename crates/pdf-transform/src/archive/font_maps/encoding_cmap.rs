//! A damaged encoding `CMap` of a composite font, written again as this reader read it (ADR 3804).
//!
//! ISO 32000-2 §9.7.5.3 gives an embedded `CMap`'s syntax, and `pdf_font::cmap` reads an entry
//! outside it — a code that is no string of one to four bytes, a range whose high code is below its
//! low, a selector that is no integer, a section the stream ends inside — as no mapping, and logs
//! the map `font/cmap-damaged` (ADR 3565). What it read is every other entry, exactly.
//!
//! An encoding `CMap` is not a table beside the drawing, as a `/ToUnicode` is: it is the drawing's
//! first step. Its codespace ranges decide how §9.7.6.2 cuts each shown string into codes, and its
//! `cidchar`, `cidrange`, `notdefchar` and `notdefrange` entries decide the CID each code selects
//! (§9.7.4.2, §9.7.6.3). So a proof by lookup alone, code by code, would prove the selection and
//! not the cutting (ADR 3782).
//!
//! **The proof here is the reading itself.** The map is written again from the reading
//! (`pdf_font::cmap::CMap::sections_as_read`): its codespace ranges in the order extraction consults
//! them, and for each code length its single codes and its ranges in the order the lookup consults
//! them. The program written is read back by the same reader, and must read whole — not damaged,
//! not cut by a bound, its `/WMode` usable — and must hold exactly the codespace ranges, mappings,
//! notdef mappings and writing mode the producer's did (`CMap::reads_alike`). Those are everything
//! §9.7.6.2's extraction and §9.7.6.3's selection read, so every string any content stream shows is
//! cut into the same codes and selects the same glyphs: a draw proof over every string at once,
//! shown or not.
//!
//! A map built on another through `/UseCMap` is written as one map holding both, the reader's own
//! statement of the pair (Table 118), and the entry is not carried. Refused by name: a map that
//! names another with `usecmap` in its own text, whose base the dictionary does not identify, and
//! one whose own `/WMode` this reader read as the nearer of 0 and 1.

use pdf_font::cmap::CMap;
use pdf_syntax::object::{Dictionary, Object, Stream};

use super::to_unicode::stream_object;

/// Why a damaged encoding `CMap` is not written again.
pub(super) const NOT_WRITTEN: &str = "a composite font's encoding CMap holds entries this reader \
     read past, and this conversion writes such a map again from what it read only where the map \
     names no other with usecmap in its own text, its own WMode is 0 or 1, no bound of this reader \
     cut it, and the map written, read back, cuts every string into the codes and selects the CIDs \
     the producer's did; this map is not that";

/// `read` — the reader's reading of the encoding `CMap` stream `stream` — written again as a map
/// of what it read, proved to read alike.
///
/// # Errors
///
/// [`NOT_WRITTEN`] where the map is outside what the module comment lists, or the map written does
/// not read alike.
pub(super) fn written(stream: &Stream, read: &CMap) -> Result<Object, &'static str> {
    if read.references_another() || read.wmode_unusable() {
        return Err(NOT_WRITTEN);
    }
    let program = program(&stream.dict, read).ok_or(NOT_WRITTEN)?;
    let again = CMap::parse(&program, None);
    let whole = !again.damaged() && again.truncated().is_none() && !again.wmode_unusable();
    if !whole || !again.has_codespace() || !again.has_mappings() || !read.reads_alike(&again) {
        return Err(NOT_WRITTEN);
    }
    let mut dict = stream.dict.clone();
    // The entries that describe the old bytes' encoding, and the base the new map now holds.
    for entry in ["Filter", "DecodeParms", "Length", "DL", "UseCMap"] {
        dict.remove(entry);
    }
    Ok(stream_object(dict, program))
}

/// The `CMap` program: §9.7.5.4's EXAMPLE's frame — the stream dictionary's `/CIDSystemInfo` and
/// `/CMapName`, which Table 118 requires there, and the writing mode read — around the reading's
/// sections; `None` where a bound cut the reading.
fn program(dict: &Dictionary, read: &CMap) -> Option<Vec<u8>> {
    let sections = read.sections_as_read()?;
    let mut out = b"%!PS-Adobe-3.0 Resource-CMap\n\
          /CIDInit /ProcSet findresource begin\n\
          12 dict begin\n\
          begincmap\n"
        .to_vec();
    for key in ["CIDSystemInfo", "CMapName"] {
        if let Some(value) = dict.get(key).filter(|value| is_direct(value)) {
            out.push(b'/');
            out.extend_from_slice(key.as_bytes());
            out.push(b' ');
            pdf_syntax::write::object(value, &mut out);
            out.extend_from_slice(b" def\n");
        }
    }
    out.extend_from_slice(format!("/CMapType 1 def\n/WMode {} def\n", read.wmode()).as_bytes());
    out.extend_from_slice(sections.as_bytes());
    out.extend_from_slice(
        b"endcmap\n\
          CMapName currentdict /CMap defineresource pop\n\
          end\nend\n",
    );
    Some(out)
}

/// Whether `value` holds no reference, so that it means the same written inside a `CMap` program,
/// where §9.7.5.3's syntax has no indirect objects.
fn is_direct(value: &Object) -> bool {
    match value {
        Object::Reference(_) | Object::Stream(_) => false,
        Object::Array(items) => items.iter().all(is_direct),
        Object::Dictionary(dict) => dict.iter().all(|(_, value)| is_direct(value)),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use pdf_font::cmap::CMap;
    use pdf_syntax::object::{Dictionary, Name, Object};

    use super::program;

    const HEADER: &str = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
                          2 begincodespacerange\n<00> <80>\n<8140> <9ffc>\nendcodespacerange\n";

    fn dict() -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert(
            Name::new(&b"CMapName"[..]),
            Object::Name(Name::new(&b"Damaged-H"[..])),
        );
        dict
    }

    /// A `cidrange` whose high code is below its low, a `cidchar` whose selector is a name, and a
    /// section the stream ends inside: each entry outside §9.7.5.3's syntax is read as no mapping,
    /// and the map written again holds exactly what was read, one- and two-byte codes apart.
    #[test]
    fn a_damaged_encoding_cmap_is_written_as_read_and_reads_alike() {
        let source = format!(
            "{HEADER}2 begincidrange\n<20> <7e> 1\n<8150> <8140> 9\nendcidrange\n\
             2 begincidchar\n<8140> 633\n<8141> /a\nendcidchar\n\
             1 beginnotdefrange\n<00> <1f> 1\nendnotdefrange\n\
             1 begincidchar\n<8142> 700\n"
        );
        let read = CMap::parse(source.as_bytes(), None);
        assert!(read.damaged());
        let written = program(&dict(), &read).expect("no bound cut it");
        let again = CMap::parse(&written, None);
        assert!(!again.damaged(), "{}", String::from_utf8_lossy(&written));
        assert!(read.reads_alike(&again));
        let text = String::from_utf8_lossy(&written);
        assert!(text.contains("<20> <7e> 1"), "{text}");
        assert!(text.contains("<8140> 633"), "{text}");
        assert!(!text.contains("<8141>"), "{text}");
        assert!(text.contains("<8142> 700"), "{text}");
        assert!(!text.contains("<8150>"), "{text}");
        assert!(text.contains("/CMapName /Damaged-H def"), "{text}");
    }

    /// `reads_alike` tells two maps apart by their cutting alone: with the same mappings, one
    /// whose codespace admits the two-byte code `81 40` cuts those bytes as one code, and one whose
    /// codespace does not cuts them as two (§9.7.6.2), so the two do not read alike.
    #[test]
    fn maps_that_cut_strings_differently_do_not_read_alike() {
        let mappings = "1 begincidrange\n<00> <ff> 1\nendcidrange\n";
        let one = CMap::parse(
            format!(
                "2 begincodespacerange\n<00> <80>\n<8140> <9ffc>\nendcodespacerange\n{mappings}"
            )
            .as_bytes(),
            None,
        );
        let other = CMap::parse(
            format!("1 begincodespacerange\n<00> <ff>\nendcodespacerange\n{mappings}").as_bytes(),
            None,
        );
        assert!(!one.reads_alike(&other));
        assert!(one.reads_alike(&one.clone()));
    }
}
