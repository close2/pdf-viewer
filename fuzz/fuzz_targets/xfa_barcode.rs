//! Fuzzes every symbology `xfa_barcode` builds, with the field's data and the template's options
//! cut from the input.
//!
//! A barcode field's value is the document's, or a script's, and arrives at the encoder unchecked:
//! the encoders' arithmetic is argued bounded in their `expect` reasons (ADR 0096), and this
//! target is what holds the claim. **It never panics, and a symbol is the shape its symbology
//! says**: a Data Matrix symbol is square with one of the 24 sides, a PDF417 symbol is `17 c + 69`
//! modules wide (`17 c + 35` compact) and three module rows to a row, and a Code 39 symbol is nine
//! elements a character with a narrow space between.

#![no_main]

use libfuzzer_sys::fuzz_target;
use xfa_barcode::pdf417::{self, Options, Shape};
use xfa_barcode::{Checksum, Symbology, code39, data_matrix};

fuzz_target!(|input: &[u8]| {
    let Some((&selector, data)) = input.split_first() else {
        return;
    };
    if let Ok(symbol) = data_matrix::encode(data) {
        assert_eq!(symbol.width, symbol.height, "a square symbol");
        assert!(
            data_matrix::SIZES
                .iter()
                .any(|size| size.side == symbol.width)
        );
    }
    let shape = match selector % 4 {
        0 => Shape::Square,
        1 => Shape::Columns(usize::from(selector >> 3)),
        2 => Shape::Ratio(f64::from(selector) / 16.0),
        _ => Shape::Fixed {
            rows: usize::from(selector >> 1),
            columns: usize::from(selector >> 3),
        },
    };
    let options = Options {
        level: Some(selector % 10),
        shape,
        compact: selector & 4 != 0,
    };
    if let Ok((symbol, layout)) = pdf417::encode(data, options) {
        let overhead = if options.compact { 35 } else { 69 };
        assert_eq!(
            Some(symbol.width),
            layout
                .columns
                .checked_mul(17)
                .and_then(|width| width.checked_add(overhead))
        );
        assert_eq!(
            Some(symbol.height),
            layout.rows.checked_mul(pdf417::ROW_HEIGHT)
        );
    }
    let text = String::from_utf8_lossy(data);
    for name in ["code3Of9", "code3Of9extended", "logmars"] {
        let Some(symbology) = Symbology::from_type(name) else {
            continue;
        };
        for checksum in [Checksum::None, Checksum::Auto] {
            if let Ok(symbol) = code39::encode(symbology, checksum, &text) {
                assert_eq!(
                    symbol.elements.len() % 10,
                    9,
                    "nine a character, one between"
                );
            }
        }
    }
});
