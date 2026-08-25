//! The whole of ISO 32000-2 Annex D.3, against the `PDFDocEncoding` this crate compiles in.
//!
//! `text_string.rs` holds Table D.3 as 256 hand-transcribed entries, and until this file existed
//! the only thing asserting any of them was a round trip through the encoder beside it plus seven
//! spot-checked codes. **A round trip cannot see a wrong transcription**: `encode_text_string`
//! searches the same array `text_string` indexes, so a table with two codes swapped round-trips
//! perfectly and draws a document's text wrong. That left 232 of the annex's mappings ungated
//! under a row the ledger calls `implemented`.
//!
//! The other side of the comparison is `doc/md/`, the Markdown conversion the citation gate
//! already reads — **not tracked in the clear**, because ISO's text is free to obtain and not
//! free to redistribute, so a developer unpacks `doc/specifications.zip` first (ADR 0187,
//! `NOTICE` section 3). This test fails rather than skipping when it is absent, which is the
//! cost that decision names, and which `tests/real_documents.rs` already pays for `doc/*.pdf`.
//!
//! Three errata of Errata Collection 3 land on this table — Issue #285, Issue #461 and Issue
//! #562 — and every one of them corrects a column a reader does not read: the printed glyph, the
//! alias of a code the annex marks undefined, and one `Unicode` cell of another such code.
//! `doc/errata-read.md` has them. What they leave standing is exactly what this file asserts.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::path::Path;

use pdf_syntax::text_string::{encode_text_string, text_string};

/// The annex's `U` note, "Undefined code point in `PDFDocEncoding`".
///
/// [`text_string`] answers U+FFFD REPLACEMENT CHARACTER for one, which is Unicode's own name for
/// a byte that represents no character.
const UNDEFINED: char = '\u{FFFD}';

/// Table D.3, read out of the standard: 256 rows, one per code.
///
/// The conversion mangles this table's columns — which of the seven a cell lands in varies from
/// one page block to the next, and the header of a block does not always have as many cells as
/// its rows do — so nothing here counts columns. Two things are read positionally-independently:
/// the row's single `0xNN`, which is its code, and its single `U+NNNN`, which is the character.
///
/// **Telling the annex's `U` note from a character that happens to be `U` is the one place that
/// needs care**, and code 0x55 is why: its row prints `U` twice, once as LATIN CAPITAL LETTER U
/// in the `Character` column and once — in a *different* block — not at all. So the cell showing
/// exactly the character the `Unicode` column names is dropped before the row is searched for a
/// bare `U`, which identifies the `Character` column by what it is for rather than by where the
/// conversion put it.
fn table_d3() -> Vec<Option<char>> {
    let markdown =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/md/ISO_32000-2_sponsored_EC3.md");
    let text = std::fs::read_to_string(&markdown).expect(
        "doc/md/ is unpacked from doc/specifications.zip; see ADR 0187 and NOTICE section 3",
    );

    let mut rows: Vec<Option<Option<char>>> = vec![None; 256];
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("Table D.3 -PDFDocEncoding") {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## D.4") {
            break;
        }
        if !inside || !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        let Some(code) = sole(&cells, "0x", 2, 16) else {
            continue; // a header, a rule, or a row the conversion split
        };
        let unicode = sole(&cells, "U+", 4, 16).and_then(char::from_u32);
        let noted_undefined = cells
            .iter()
            .filter(|cell| unicode.is_none_or(|character| **cell != character.to_string()))
            .any(|cell| *cell == "U" || *cell == "Undefined");
        let code = usize::try_from(code).expect("a two-digit hexadecimal number fits");
        rows[code] = Some(if noted_undefined { None } else { unicode });
    }

    (0..256)
        .map(|code| {
            rows[code].unwrap_or_else(|| panic!("Table D.3 states no row for code {code:#04x}"))
        })
        .collect()
}

/// The one value in `cells` written as `prefix` followed by exactly `digits` digits, if there is
/// exactly one — a second occurrence means the row is not the shape this reads, so it is skipped
/// rather than guessed at.
fn sole(cells: &[&str], prefix: &str, digits: usize, radix: u32) -> Option<u32> {
    let mut found = None;
    for cell in cells {
        for after in cell.split(prefix).skip(1) {
            let taken: String = after
                .chars()
                .take_while(|character| character.is_digit(radix))
                .collect();
            if taken.chars().count() != digits {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = u32::from_str_radix(&taken, radix).ok();
        }
    }
    found
}

/// Every one of the 256 codes decodes to the character Table D.3 gives it.
///
/// The annex's own arithmetic is asserted first, because a parse that quietly found nothing
/// would make every comparison below vacuous: 24 of the codes carry the `U` note — 0 to 8, 11,
/// 12, 14 to 23, 127, 159 and 173 — and the other 232 name a character.
#[test]
fn every_code_decodes_to_the_character_annex_d3_gives_it() {
    let table = table_d3();
    let undefined = table.iter().filter(|entry| entry.is_none()).count();
    assert_eq!(undefined, 24, "Table D.3 marks 24 of its 256 codes `U`");

    let mut wrong = Vec::new();
    for (code, expected) in table.iter().enumerate() {
        let code = u8::try_from(code).expect("the loop runs over 0..256");
        let decoded = text_string(&[code]);
        let want = expected.unwrap_or(UNDEFINED).to_string();
        if decoded != want {
            wrong.push(format!("{code:#04x}: annex {want:?}, table {decoded:?}"));
        }
    }
    assert!(wrong.is_empty(), "Table D.3 disagrees at {wrong:?}");
}

/// Every character Table D.3 names encodes back to its own code.
///
/// The reverse direction is a separate statement rather than a round trip: `pdf_doc_encoded`
/// searches the array for the *first* entry holding a character, so two codes transcribed with
/// one character would decode differently and encode alike, and only this direction sees it.
#[test]
fn every_character_annex_d3_names_encodes_back_to_its_code() {
    let mut wrong = Vec::new();
    for (code, expected) in table_d3().iter().enumerate() {
        let code = u8::try_from(code).expect("the loop runs over 0..256");
        let Some(character) = *expected else {
            continue; // an undefined code point names no character to encode
        };
        let encoded = encode_text_string(&character.to_string());
        if encoded != vec![code] {
            wrong.push(format!(
                "{character:?}: annex {code:#04x}, table {encoded:02x?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "Table D.3 disagrees at {wrong:?}");
}
