//! Code 39 as MIL-STD-1189A defines it and XFA 3.3 types it, read back by an independent decoder.
//!
//! Every expected value below is the source's: MIL-STD-1189A paragraph 4.1 and table E.1 for the symbol,
//! XFA 3.3's template reference and chapter 12 for the identifiers, the checksum keywords and the
//! widths. `rxing`'s Code 39 reader is evidence that a symbol is readable as its data, never the
//! definition of the symbology (ADR 2877).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a symbol that does not encode has failed the test, and a scan line of a \
              few hundred pixels cannot overflow its arithmetic"
)]

use rxing::DecodeHints;
use rxing::common::BitArray;
use rxing::oned::{Code39Reader, OneDReader};
use xfa_barcode::code39::{self, Geometry, TABLE};
use xfa_barcode::{Checksum, Element, Linear, Refusal, Symbology};

fn code3of9() -> Symbology {
    Symbology::from_type("code3Of9").expect("XFA 3.3 page 617 defines code3Of9")
}

/// MIL-STD-1189A paragraph 4.1: "Each character is composed of nine elements: five bars and four spaces.
/// Three of the nine elements are wide"; every row of table E.1 holds to it, and no
/// two characters share a pattern.
#[test]
fn every_character_is_five_bars_four_spaces_and_three_wide() {
    assert_eq!(
        TABLE.len(),
        44,
        "43 characters and the delimiter, paragraph 4.1"
    );
    let mut seen = std::collections::BTreeSet::new();
    for (character, bars, spaces) in TABLE {
        assert_eq!(bars.len(), 5, "{character:?}");
        assert_eq!(spaces.len(), 4, "{character:?}");
        let wide = bars
            .chars()
            .chain(spaces.chars())
            .filter(|digit| *digit == '1')
            .count();
        assert_eq!(wide, 3, "{character:?}");
        assert!(
            seen.insert((bars, spaces)),
            "{character:?} repeats a pattern"
        );
    }
}

/// MIL-STD-1189A table E.1, first row: "1" is bars 10001 and spaces 0100, so its nine elements left to right
/// are a wide bar, a narrow space, a narrow bar, a wide space, a narrow bar, a narrow space, a
/// narrow bar, a narrow space and a wide bar.
#[test]
fn the_digit_one_is_table_e1s_first_row() {
    let wide: Vec<bool> = code39::character('1')
        .expect("table E.1 holds 1")
        .iter()
        .map(|element| element.wide)
        .collect();
    assert_eq!(
        wide,
        [true, false, false, true, false, false, false, false, true]
    );
    let bars: Vec<bool> = code39::character('1')
        .expect("table E.1 holds 1")
        .iter()
        .map(|element| element.bar)
        .collect();
    assert_eq!(
        bars,
        [true, false, true, false, true, false, true, false, true]
    );
}

/// A message of n characters is the start delimiter, the n characters and the stop delimiter
/// (table E.1's note), nine elements each, a narrow space between each two (paragraph 5.2.2's minimum).
#[test]
fn a_message_is_framed_by_the_delimiter_and_gapped_by_a_narrow_space() {
    let symbol = xfa_barcode::encode(code3of9(), Checksum::None, "A1").expect("A and 1 are held");
    assert_eq!(symbol.elements.len(), 4 * 9 + 3);
    let start = code39::character('*').expect("the delimiter is in table E.1");
    assert_eq!(symbol.elements.get(..9), Some(&start[..]));
    assert_eq!(symbol.elements.get(30..), Some(&start[..]));
    assert_eq!(
        symbol.elements.get(9),
        Some(&Element {
            bar: false,
            wide: false
        })
    );
}

/// Paragraph 4.1's set has no lower-case letter, and the delimiter may not appear inside a message.
#[test]
fn a_character_outside_the_set_is_refused_by_name() {
    for character in ['a', '*', '#'] {
        assert_eq!(
            xfa_barcode::encode(code3of9(), Checksum::None, &format!("A{character}")),
            Err(Refusal::Unencodable {
                symbology: code3of9(),
                character
            })
        );
    }
}

/// XFA 3.3 page 613 lists the keywords; page 440 ticks `auto` and `none` for `code3of9` and
/// `code3of9extended` and no other, so `1mod10` is refused for Code 39.
#[test]
fn a_checksum_page_440_does_not_tick_is_refused() {
    assert_eq!(Checksum::from_keyword("none"), Some(Checksum::None));
    assert_eq!(Checksum::from_keyword("auto"), Some(Checksum::Auto));
    assert_eq!(Checksum::from_keyword("mod43"), None);
    assert_eq!(
        xfa_barcode::encode(code3of9(), Checksum::OneModTen, "A"),
        Err(Refusal::Checksum {
            symbology: code3of9(),
            checksum: Checksum::OneModTen
        })
    );
}

/// The modulo-43 check character (A857's source, `rxing`'s `ALPHABET_STRING`): each character's
/// value is its position among the 43, and the check is their sum modulo 43. "CODE39": C 12, O 24,
/// D 13, E 14, 3 3, 9 9 — 75, which is 32 modulo 43, the letter W.
#[test]
fn the_check_character_is_the_sum_modulo_43() {
    assert_eq!(code39::VALUES.len(), 43);
    assert_eq!(code39::check_character("CODE39"), Some('W'));
    assert_eq!(code39::check_character(""), Some('0'));
    // "%%": 42 + 42 = 84, 41 modulo 43, the character `+`.
    assert_eq!(code39::check_character("%%"), Some('+'));
    assert_eq!(code39::check_character("a"), None);
    let with = xfa_barcode::encode(code3of9(), Checksum::Auto, "CODE39").expect("held");
    let without = xfa_barcode::encode(code3of9(), Checksum::None, "CODE39W").expect("held");
    assert_eq!(with, without, "`auto` appends W before the stop delimiter");
}

/// Page 440 ticks no checksum for `logmars`, and page 613 says the attribute is then ignored: a
/// `logmars` symbol is the same whatever `checksum` says.
#[test]
fn logmars_ignores_the_checksum() {
    let logmars = Symbology::from_type("logmars").expect("a defined type");
    let none = xfa_barcode::encode(logmars, Checksum::None, "CODE39").expect("held");
    for checksum in [Checksum::Auto, Checksum::OneModTen] {
        assert_eq!(
            xfa_barcode::encode(logmars, checksum, "CODE39").as_ref(),
            Ok(&none)
        );
    }
}

/// The full-ASCII set: every one of the 128 characters is one base character or a shift and a
/// letter, the 43 base data characters but `$`, `/`, `+` and `%` stand for themselves, and no two
/// characters share an entry.
#[test]
fn the_extended_set_writes_every_ascii_character_once() {
    let mut seen = std::collections::BTreeSet::new();
    for (ascii, written) in code39::EXTENDED.iter().enumerate() {
        assert!(seen.insert(*written), "{ascii} repeats {written:?}");
        let character = char::from(u8::try_from(ascii).expect("128 entries"));
        if written.len() == 1 {
            assert_eq!(written.chars().next(), Some(character));
            assert!(code39::VALUES.contains(character) && !"$/+%".contains(character));
        } else {
            let mut pair = written.chars();
            assert!(
                matches!(pair.next(), Some('$' | '%' | '/' | '+')),
                "{ascii}"
            );
            assert!(matches!(pair.next(), Some('A'..='Z')), "{ascii}");
        }
    }
    let extended = Symbology::from_type("code3Of9extended").expect("defined");
    assert_eq!(
        xfa_barcode::encode(extended, Checksum::None, "é"),
        Err(Refusal::Unencodable {
            symbology: extended,
            character: 'é'
        })
    );
}

/// XFA 3.3 page 421: the `type` value is case-insensitive; pages 616–620 define 62 identifiers.
/// `logmars` is Code 39 by MIL-STD-1189B (page 1565); the symbologies not built are refused by
/// name.
#[test]
fn the_type_is_matched_without_case_and_unbuilt_types_are_refused_by_name() {
    assert_eq!(Symbology::from_type("CODE3OF9"), Some(code3of9()));
    assert_eq!(
        Symbology::from_type("pdf417 ").map(Symbology::name),
        Some("pdf417")
    );
    assert_eq!(Symbology::from_type("myInHouseCode"), None);
    for name in ["LOGMARS", "code3Of9extended", "dataMatrix", "pdf417"] {
        assert!(
            Symbology::from_type(name).is_some_and(Symbology::is_built),
            "{name}"
        );
    }
    for name in ["QRCode", "aztec", "code128"] {
        let symbology = Symbology::from_type(name).expect("a defined type");
        assert!(!symbology.is_built(), "{name}");
        assert_eq!(
            xfa_barcode::encode(symbology, Checksum::None, "A"),
            Err(Refusal::NotBuilt(symbology))
        );
    }
}

/// XFA 3.3 page 440, *Which 1D properties apply to which type*: `code3of9` takes a module of
/// 0.191 mm to 1.25 mm and a wide-to-narrow ratio of 2.2 to 3.0; `logmars` 0.125 mm and 2.0.
#[test]
fn the_widths_are_held_to_xfa_3_3_page_440() {
    let logmars = Symbology::from_type("logmars").expect("a defined type");
    let at = |narrow_mm, ratio| Geometry { narrow_mm, ratio };
    assert!(at(0.191, 2.2).check(code3of9()).is_ok());
    assert!(at(1.25, 3.0).check(code3of9()).is_ok());
    assert!(at(0.19, 2.5).check(code3of9()).is_err());
    assert!(at(0.25, 2.1).check(code3of9()).is_err());
    assert!(at(0.25, 2.1).check(logmars).is_ok());
    assert!(at(0.125, 2.0).check(logmars).is_ok());
    assert!(at(0.25, 3.1).check(logmars).is_err());
}

/// The symbol drawn as one scan line — `narrow` pixels to a narrow element, a quiet zone of ten
/// narrow elements each side (MIL-STD-1189A paragraph 5.2.3's minimum) — and read by `rxing`.
fn read_back(
    symbol: &Linear,
    narrow: usize,
    ratio_numerator: usize,
    mut reader: Code39Reader,
) -> Option<String> {
    let quiet = narrow * 10;
    let wide = narrow * ratio_numerator / 2;
    let width = quiet * 2
        + symbol
            .elements
            .iter()
            .map(|element| if element.wide { wide } else { narrow })
            .sum::<usize>();
    let mut row = BitArray::with_size(width);
    let mut x = quiet;
    for element in &symbol.elements {
        let run = if element.wide { wide } else { narrow };
        if element.bar {
            for pixel in x..x + run {
                row.set(pixel);
            }
        }
        x += run;
    }
    reader
        .decode_row(0, &row, &DecodeHints::default())
        .ok()
        .map(|result| result.getText().to_owned())
}

#[test]
fn every_symbol_reads_back_as_its_data() {
    let every: String = TABLE
        .iter()
        .map(|(character, _, _)| *character)
        .filter(|character| *character != '*')
        .collect();
    // An empty message is `**`, which this decoder reports as no symbol found; it is pinned by
    // `the_bars_are_placed_at_the_given_widths` from table E.1 alone.
    for data in ["0", "HELLO WORLD", "CODE-39 $/+%.", every.as_str()] {
        let symbol = xfa_barcode::encode(code3of9(), Checksum::None, data).expect("held");
        // Ratios 2.0, 2.5 and 3.0: the two ends of MIL-STD-1189A paragraph 5.2's range and a middle.
        for ratio_numerator in [4, 5, 6] {
            assert_eq!(
                read_back(&symbol, 4, ratio_numerator, Code39Reader::new()).as_deref(),
                Some(data),
                "{data:?} at a ratio of {ratio_numerator}/2"
            );
        }
    }
}

/// A symbol with its check character is read back by a reader that checks it and strips it, and
/// an extended symbol by a reader in full-ASCII mode, as every ASCII character.
#[test]
fn checked_and_extended_symbols_read_back_as_their_data() {
    let every_ascii: String = (0_u8..128).map(char::from).collect();
    let extended = Symbology::from_type("code3Of9extended").expect("defined");
    for (symbology, data) in [
        (code3of9(), "CODE39"),
        (code3of9(), "HELLO WORLD $/+%."),
        (extended, "Hello, World! {x|y} @`~"),
        (extended, every_ascii.as_str()),
    ] {
        for (checksum, checking) in [(Checksum::None, false), (Checksum::Auto, true)] {
            let symbol = xfa_barcode::encode(symbology, checksum, data).expect("held");
            let reader = Code39Reader::with_all_config(checking, symbology == extended);
            assert_eq!(
                read_back(&symbol, 4, 5, reader).as_deref(),
                Some(data),
                "{symbology} {data:?} with {}",
                checksum.keyword()
            );
        }
    }
}

/// The bars a drawing piece places are the symbol's printed elements at the widths it gives.
#[test]
fn the_bars_are_placed_at_the_given_widths() {
    let symbol = xfa_barcode::encode(code3of9(), Checksum::None, "").expect("held");
    // `**`: 18 elements and one gap, 6 of them wide.
    assert!((symbol.width(1.0, 3.0) - (19.0 + 6.0 * 2.0)).abs() < 1e-9);
    let bars = symbol.bars(1.0, 3.0);
    assert_eq!(bars.len(), 10);
    // table E.1's `*`: bars 00110, so the first bar is narrow and at the left edge.
    assert_eq!(bars.first().map(|bar| (bar.x, bar.width)), Some((0.0, 1.0)));
}
