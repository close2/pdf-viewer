//! ISO 32000-2 §7.9.2.2's text string type, and the Annex D.3 table it rests on.
//!
//! A *text string* is the type every human-readable string in a PDF carries: an annotation's
//! `/Contents`, a form field's `/V`, an outline item's `/Title`. §7.9.2.2.1 gives it three
//! encodings and one rule for telling them apart, and that rule is a prefix:
//!
//! > For text strings encoded in UTF-16BE, the first two bytes shall be 254 followed by 255.
//!
//! > For text strings encoded in UTF-8, the first three bytes shall be 239 followed by 187,
//! > followed by 191.
//!
//! Anything else is `PDFDocEncoding`, whose 256 code points are Table D.3 and are compiled in
//! below. The clause's own NOTE 3 and NOTE 4 say why a prefix is enough to decide: the byte
//! sequences that introduce the two Unicode forms spell `þÿ` and `¨»¿` in `PDFDocEncoding`,
//! "which is unlikely to be a meaningful beginning of a word or phrase".
//!
//! # Why this is in the syntax crate
//!
//! A text string is a *string object type* — §7.9.2, inside clause 7 — rather than anything to
//! do with fonts. Table D.3 is a code-to-Unicode table and needs no glyph names, which is what
//! separates it from Annex D.2's font encodings in `pdf-font`.

/// Decodes ISO 32000-2 §7.9.2.2's text string type.
///
/// The three encodings the clause names, chosen by the prefix it specifies, with
/// §7.9.2.2.2's language escape sequences removed from the two Unicode forms.
///
/// A byte with no character in Table D.3 — the clause's `U`, "Undefined code point in
/// `PDFDocEncoding`" — becomes U+FFFD REPLACEMENT CHARACTER, which is Unicode's own name for
/// a byte that represents no character. Nothing draws it, so a caller laying the string out
/// reports it rather than dropping it silently.
#[must_use]
pub fn text_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&UTF16BE_PREFIX) {
        return without_language_escapes(&utf16be(rest));
    }
    if let Some(rest) = bytes.strip_prefix(&UTF8_PREFIX) {
        return without_language_escapes(&String::from_utf8_lossy(rest));
    }
    bytes
        .iter()
        .map(|byte| PDF_DOC_ENCODING[usize::from(*byte)].unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Encodes a string as ISO 32000-2 §7.9.2.2's text string type.
///
/// The inverse of [`text_string`], and it chooses between two of the clause's three encodings by
/// what the string contains: `PDFDocEncoding` where every character has a Table D.3 code, and
/// UTF-16BE with §7.9.2.2.1's two-byte prefix otherwise.
///
/// **`PDFDocEncoding` first, because it is the encoding a reader assumes.** The clause makes the
/// two Unicode forms the ones with a prefix and everything else `PDFDocEncoding`, so a string
/// that fits it needs no prefix, no second byte per character, and no decision by whoever reads
/// it back. UTF-8 is the third form and is not produced here: it is PDF 2.0 only, and a file
/// this program writes has to be readable by every processor that could read what it wrote into.
///
/// A code that Table D.3 maps to a *different* character is not a fit: the table is not Latin-1,
/// and the round trip through [`text_string`] is what "fits" means.
#[must_use]
pub fn encode_text_string(text: &str) -> Vec<u8> {
    if let Some(bytes) = pdf_doc_encoded(text) {
        return bytes;
    }
    let mut out = Vec::from(UTF16BE_PREFIX);
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

/// The string in `PDFDocEncoding`, or `None` if any character has no code in Table D.3.
///
/// The table is small enough to search per character (256 entries), and a form field's value is
/// the length of a form field's value. A reverse map built once would be the right answer if this
/// were on a page's path; it is on a person's keystroke.
///
/// **Also §7.6.4.3.2 step (a)'s conversion**, which wants a *password* in `PDFDocEncoding` and is
/// the same operation on a string of the same order of length. `crypt.rs` derived a partial
/// version of this from the ranges where the encoding and Unicode agree for a hundred and
/// twenty-nine sessions, and refused every password outside them; this is the whole table, in
/// the crate that already held it.
pub(crate) fn pdf_doc_encoded(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len());
    for character in text.chars() {
        let code = PDF_DOC_ENCODING
            .iter()
            .position(|entry| *entry == Some(character))?;
        out.push(u8::try_from(code).ok()?);
    }
    Some(out)
}

/// §7.9.2.2.1's UTF-16BE byte order marker: "the first two bytes shall be 254 followed by 255".
const UTF16BE_PREFIX: [u8; 2] = [254, 255];

/// §7.9.2.2.1's UTF-8 byte order marker: "the first three bytes shall be 239 followed by 187,
/// followed by 191".
const UTF8_PREFIX: [u8; 3] = [239, 187, 191];

/// Decodes UTF-16BE, pairing surrogates.
///
/// §7.9.2.2.1 requires it of a reader in as many words — "PDF readers that process PDF files
/// containing Unicode text strings shall be prepared to handle supplementary characters; that
/// is, characters requiring more than two bytes to represent" — and its NOTE 5 says why the
/// sentence is there: UTF-16BE is not to be confused with UCS-2, and is not a fixed-width
/// encoding scheme.
///
/// An odd trailing byte and an unpaired surrogate both become U+FFFD rather than ending the
/// string: a truncated value is still worth showing as far as it goes.
fn utf16be(bytes: &[u8]) -> String {
    let units = bytes
        .chunks(2)
        .map(|pair| match pair {
            [high, low] => u16::from_be_bytes([*high, *low]),
            // A trailing odd byte is not a code unit. U+FFFD is not a surrogate, so
            // `decode_utf16` passes it through unchanged.
            _ => 0xFFFD,
        })
        .collect::<Vec<u16>>();
    char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Removes §7.9.2.2.2's language escape sequences.
///
/// The clause states the sequence element by element: U+001B, "a 2-byte BCP 47 language code",
/// an optional "2-byte ISO 3166 country code", then U+001B again. It marks "the language in
/// which subsequent text shall be written", so it carries no character to display and leaving
/// it in would draw `\u{1b}en\u{1b}` in front of the text.
///
/// A lone U+001B with no closing one is left alone: the clause defines an escape sequence as
/// the whole five-or-seven-character run, and discarding the tail of a string on one stray
/// byte would lose text the file does state.
fn without_language_escapes(text: &str) -> String {
    if !text.contains(ESCAPE) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(ESCAPE) {
        let (before, from_escape) = rest.split_at(start);
        out.push_str(before);
        let body = &from_escape[ESCAPE.len_utf8()..];
        // A language code of two letters, or one of two letters and a country code of two
        // more, then the closing escape — the only two shapes the clause states. Anything
        // else is not one of its sequences and is left where it stands.
        let mut after = None;
        let mut unread = body;
        for _ in 0..2 {
            let Some((letters, tail)) = unread.split_at_checked(2) else {
                break;
            };
            if !letters.chars().all(|letter| letter.is_ascii_alphabetic()) {
                break;
            }
            unread = tail;
            if let Some(beyond) = tail.strip_prefix(ESCAPE) {
                after = Some(beyond);
                break;
            }
        }

        if let Some(beyond) = after {
            rest = beyond;
        } else {
            out.push(ESCAPE);
            rest = body;
        }
    }
    out.push_str(rest);
    out
}

/// §7.9.2.2.2 element a): "The Unicode value ESCAPE (U+001B)".
const ESCAPE: char = '\u{1b}';

/// ISO 32000-2 Table D.3, the `PDFDocEncoding` character set.
///
/// One entry per byte, holding the Unicode code point the table's own column gives it. `None`
/// is the table's `U` note, "Undefined code point in `PDFDocEncoding`" — codes 0 to 8, 11, 12,
/// 14 to 23, 127, 159 and 173.
///
/// It is not ISO Latin 1 and the differences are load-bearing: 0x18 to 0x1F are accents where
/// Latin 1 has control codes, 0x80 to 0x9E are punctuation, and **0xA0 is EURO SIGN** where
/// Latin 1 has NO-BREAK SPACE. Reading a `/V` as Latin 1 turns a price into a space.
#[rustfmt::skip]
const PDF_DOC_ENCODING: [Option<char>; 256] = [
    None, None, None, None,
    None, None, None, None,
    None, Some('\u{0009}'), Some('\u{000A}'), None,
    None, Some('\u{000D}'), None, None,
    None, None, None, None,
    None, None, None, None,
    Some('\u{02D8}'), Some('\u{02C7}'), Some('\u{02C6}'), Some('\u{02D9}'),
    Some('\u{02DD}'), Some('\u{02DB}'), Some('\u{02DA}'), Some('\u{02DC}'),
    Some('\u{0020}'), Some('\u{0021}'), Some('\u{0022}'), Some('\u{0023}'),
    Some('\u{0024}'), Some('\u{0025}'), Some('\u{0026}'), Some('\u{0027}'),
    Some('\u{0028}'), Some('\u{0029}'), Some('\u{002A}'), Some('\u{002B}'),
    Some('\u{002C}'), Some('\u{002D}'), Some('\u{002E}'), Some('\u{002F}'),
    Some('\u{0030}'), Some('\u{0031}'), Some('\u{0032}'), Some('\u{0033}'),
    Some('\u{0034}'), Some('\u{0035}'), Some('\u{0036}'), Some('\u{0037}'),
    Some('\u{0038}'), Some('\u{0039}'), Some('\u{003A}'), Some('\u{003B}'),
    Some('\u{003C}'), Some('\u{003D}'), Some('\u{003E}'), Some('\u{003F}'),
    Some('\u{0040}'), Some('\u{0041}'), Some('\u{0042}'), Some('\u{0043}'),
    Some('\u{0044}'), Some('\u{0045}'), Some('\u{0046}'), Some('\u{0047}'),
    Some('\u{0048}'), Some('\u{0049}'), Some('\u{004A}'), Some('\u{004B}'),
    Some('\u{004C}'), Some('\u{004D}'), Some('\u{004E}'), Some('\u{004F}'),
    Some('\u{0050}'), Some('\u{0051}'), Some('\u{0052}'), Some('\u{0053}'),
    Some('\u{0054}'), Some('\u{0055}'), Some('\u{0056}'), Some('\u{0057}'),
    Some('\u{0058}'), Some('\u{0059}'), Some('\u{005A}'), Some('\u{005B}'),
    Some('\u{005C}'), Some('\u{005D}'), Some('\u{005E}'), Some('\u{005F}'),
    Some('\u{0060}'), Some('\u{0061}'), Some('\u{0062}'), Some('\u{0063}'),
    Some('\u{0064}'), Some('\u{0065}'), Some('\u{0066}'), Some('\u{0067}'),
    Some('\u{0068}'), Some('\u{0069}'), Some('\u{006A}'), Some('\u{006B}'),
    Some('\u{006C}'), Some('\u{006D}'), Some('\u{006E}'), Some('\u{006F}'),
    Some('\u{0070}'), Some('\u{0071}'), Some('\u{0072}'), Some('\u{0073}'),
    Some('\u{0074}'), Some('\u{0075}'), Some('\u{0076}'), Some('\u{0077}'),
    Some('\u{0078}'), Some('\u{0079}'), Some('\u{007A}'), Some('\u{007B}'),
    Some('\u{007C}'), Some('\u{007D}'), Some('\u{007E}'), None,
    Some('\u{2022}'), Some('\u{2020}'), Some('\u{2021}'), Some('\u{2026}'),
    Some('\u{2014}'), Some('\u{2013}'), Some('\u{0192}'), Some('\u{2044}'),
    Some('\u{2039}'), Some('\u{203A}'), Some('\u{2212}'), Some('\u{2030}'),
    Some('\u{201E}'), Some('\u{201C}'), Some('\u{201D}'), Some('\u{2018}'),
    Some('\u{2019}'), Some('\u{201A}'), Some('\u{2122}'), Some('\u{FB01}'),
    Some('\u{FB02}'), Some('\u{0141}'), Some('\u{0152}'), Some('\u{0160}'),
    Some('\u{0178}'), Some('\u{017D}'), Some('\u{0131}'), Some('\u{0142}'),
    Some('\u{0153}'), Some('\u{0161}'), Some('\u{017E}'), None,
    Some('\u{20AC}'), Some('\u{00A1}'), Some('\u{00A2}'), Some('\u{00A3}'),
    Some('\u{00A4}'), Some('\u{00A5}'), Some('\u{00A6}'), Some('\u{00A7}'),
    Some('\u{00A8}'), Some('\u{00A9}'), Some('\u{00AA}'), Some('\u{00AB}'),
    Some('\u{00AC}'), None, Some('\u{00AE}'), Some('\u{00AF}'),
    Some('\u{00B0}'), Some('\u{00B1}'), Some('\u{00B2}'), Some('\u{00B3}'),
    Some('\u{00B4}'), Some('\u{00B5}'), Some('\u{00B6}'), Some('\u{00B7}'),
    Some('\u{00B8}'), Some('\u{00B9}'), Some('\u{00BA}'), Some('\u{00BB}'),
    Some('\u{00BC}'), Some('\u{00BD}'), Some('\u{00BE}'), Some('\u{00BF}'),
    Some('\u{00C0}'), Some('\u{00C1}'), Some('\u{00C2}'), Some('\u{00C3}'),
    Some('\u{00C4}'), Some('\u{00C5}'), Some('\u{00C6}'), Some('\u{00C7}'),
    Some('\u{00C8}'), Some('\u{00C9}'), Some('\u{00CA}'), Some('\u{00CB}'),
    Some('\u{00CC}'), Some('\u{00CD}'), Some('\u{00CE}'), Some('\u{00CF}'),
    Some('\u{00D0}'), Some('\u{00D1}'), Some('\u{00D2}'), Some('\u{00D3}'),
    Some('\u{00D4}'), Some('\u{00D5}'), Some('\u{00D6}'), Some('\u{00D7}'),
    Some('\u{00D8}'), Some('\u{00D9}'), Some('\u{00DA}'), Some('\u{00DB}'),
    Some('\u{00DC}'), Some('\u{00DD}'), Some('\u{00DE}'), Some('\u{00DF}'),
    Some('\u{00E0}'), Some('\u{00E1}'), Some('\u{00E2}'), Some('\u{00E3}'),
    Some('\u{00E4}'), Some('\u{00E5}'), Some('\u{00E6}'), Some('\u{00E7}'),
    Some('\u{00E8}'), Some('\u{00E9}'), Some('\u{00EA}'), Some('\u{00EB}'),
    Some('\u{00EC}'), Some('\u{00ED}'), Some('\u{00EE}'), Some('\u{00EF}'),
    Some('\u{00F0}'), Some('\u{00F1}'), Some('\u{00F2}'), Some('\u{00F3}'),
    Some('\u{00F4}'), Some('\u{00F5}'), Some('\u{00F6}'), Some('\u{00F7}'),
    Some('\u{00F8}'), Some('\u{00F9}'), Some('\u{00FA}'), Some('\u{00FB}'),
    Some('\u{00FC}'), Some('\u{00FD}'), Some('\u{00FE}'), Some('\u{00FF}'),
];

#[cfg(test)]
mod tests {
    use super::{encode_text_string, text_string};

    /// §7.9.2.2.1's EXAMPLE 1: a string with no prefix is `PDFDocEncoding`, and the byte 0x8B
    /// is U+2030 PER MILLE SIGN.
    ///
    /// The clause and the table are independent statements of the same fact, which is what
    /// makes this worth asserting: the example writes the string as `text‰` and says "the
    /// character … after the 'text' is represented by the hex code 8B", while Table D.3's own
    /// row for 139 gives U+2030. Two sources, one answer.
    #[test]
    fn a_string_with_no_prefix_is_pdfdocencoded() {
        assert_eq!(text_string(b"text\x8b"), "text‰");
    }

    /// 0xA0 is EURO SIGN in `PDFDocEncoding` and NO-BREAK SPACE in ISO Latin 1.
    ///
    /// The one byte that catches the whole table being an off-the-shelf Latin 1 decode, which
    /// is what a reader reaches for when it has not read Table D.3.
    #[test]
    fn the_table_is_not_latin_1() {
        assert_eq!(text_string(b"\xa0"), "\u{20AC}");
        assert_eq!(text_string(b"\x18\x19"), "\u{02D8}\u{02C7}");
    }

    /// §7.9.2.2.1's EXAMPLE 2: `FE FF 04 42 04 35 04 41 04 42` is "тест".
    #[test]
    fn a_utf16be_marker_selects_utf16be() {
        let bytes = b"\xfe\xff\x04\x42\x04\x35\x04\x41\x04\x42";
        assert_eq!(text_string(bytes), "тест");
    }

    /// A supplementary character is two UTF-16 code units, and the clause requires a reader
    /// to be "prepared to handle" it. U+1F600 is `D83D DE00`.
    #[test]
    fn a_surrogate_pair_is_one_character() {
        let bytes = b"\xfe\xff\xd8\x3d\xde\x00";
        assert_eq!(text_string(bytes), "\u{1F600}");
    }

    /// §7.9.2.2.1's UTF-8 marker, `EF BB BF`, introduces the PDF 2.0 form.
    #[test]
    fn a_utf8_marker_selects_utf8() {
        let bytes = b"\xef\xbb\xbfg\xc3\xbcltig";
        assert_eq!(text_string(bytes), "gültig");
    }

    /// §7.9.2.2.2's escape sequence carries no character, in both its lengths.
    ///
    /// The clause's own two examples: `en` for a bare language code, `enUS` for one with a
    /// country code, each between two U+001B.
    #[test]
    fn a_language_escape_sequence_shows_nothing() {
        let bare = b"\xfe\xff\x00\x1b\x00e\x00n\x00\x1b\x00h\x00i";
        assert_eq!(text_string(bare), "hi");
        let country = b"\xfe\xff\x00\x1b\x00e\x00n\x00U\x00S\x00\x1b\x00h\x00i";
        assert_eq!(text_string(country), "hi");
    }

    /// An unterminated U+001B is not a sequence, so the text after it survives.
    #[test]
    fn a_lone_escape_does_not_swallow_the_rest_of_the_string() {
        let bytes = b"\xfe\xff\x00h\x00\x1b\x00i";
        assert_eq!(text_string(bytes), "h\u{1b}i");
    }

    /// A byte Table D.3 marks `U` has no character.
    #[test]
    fn an_undefined_code_point_is_the_replacement_character() {
        assert_eq!(text_string(b"\x7f"), "\u{FFFD}");
        assert_eq!(text_string(b"\xad"), "\u{FFFD}");
    }
    /// What is written comes back, in both encodings the writer produces.
    ///
    /// The strongest statement available about an encoder whose decoder is beside it, and the
    /// one that catches the table being used backwards: `PDFDocEncoding` is not Latin 1, so a
    /// reverse map built by assuming it is would fail here on exactly the bytes
    /// `the_table_is_not_latin_1` names.
    #[test]
    fn every_text_string_survives_the_round_trip() {
        for text in [
            "",
            "Simple",
            "text\u{2030}",
            "\u{20AC}\u{02D8}\u{02C7}",
            "тест",
            "\u{1F600}",
            "gültig",
            "a mix: тест \u{20AC}",
        ] {
            assert_eq!(text_string(&encode_text_string(text)), text, "{text:?}");
        }
    }

    /// A string Table D.3 covers is written *without* a prefix, and one it does not is written
    /// with §7.9.2.2.1's UTF-16BE marker.
    ///
    /// Not merely a round trip: the choice is the whole of the encoder's judgement, and a
    /// version that wrote UTF-16BE for everything would pass the test above.
    #[test]
    fn the_shorter_encoding_is_chosen_where_the_table_allows_it() {
        assert_eq!(encode_text_string("text\u{2030}"), b"text\x8b");
        assert_eq!(
            encode_text_string("тест"),
            b"\xfe\xff\x04\x42\x04\x35\x04\x41\x04\x42"
        );
    }
}
