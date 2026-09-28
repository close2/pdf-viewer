//! Fuzzes the `CCITTFaxDecode` decoder, ISO 32000-2 §7.4.6 over ITU-T T.4 and T.6.
//!
//! The first six bytes choose Table 11's parameters and the rest is the coded data, so every
//! coding, every flag and the `/DamagedRowsBeforeError` concealment are reached from one input.
//! Three properties are under test, beyond never panicking:
//!
//! - **Every delivered line is a line.** Its changing elements rise strictly and stay inside
//!   `/Columns` — what the packer in `pdf-sandbox` relies on to write exactly one row's bytes.
//! - **The counts are honest.** No more lines than the bound, no more concealed than delivered,
//!   and an error's `delivered` is what was handed over — the rows `pdf_model::image` draws.
//! - **It terminates.** The damaged-row search seeks backwards to the row's own start, and a
//!   decoder that failed to consume a bit a line would loop until the bound; libFuzzer's timeout
//!   is what finds that, which is why the bound here is small.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_ccitt::{Coding, Error, Parameters, Rows, decode};

/// What the decoder handed over, checked as it arrives.
struct Checked {
    columns: u32,
    lines: u32,
}

impl Rows for Checked {
    fn row(&mut self, changes: &[u32]) {
        assert!(
            changes.windows(2).all(|pair| pair[0] < pair[1]),
            "changing elements out of order: {changes:?}"
        );
        assert!(
            changes.iter().all(|&change| change < self.columns),
            "a changing element at or past column {}: {changes:?}",
            self.columns
        );
        self.lines = self.lines.saturating_add(1);
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((head, coded)) = data.split_first_chunk::<6>() else {
        return;
    };
    let coding = match head[0] % 3 {
        0 => Coding::Group4,
        1 => Coding::Group3OneDimensional,
        _ => Coding::Group3Mixed,
    };
    let parameters = Parameters {
        coding,
        // Up to 4095 pels, so wide-line arithmetic is reached without every input costing a
        // megapixel.
        columns: u32::from(u16::from_be_bytes([head[1], head[2]]) & 0x0FFF),
        rows: u32::from(head[3]),
        end_of_line: head[4] & 1 != 0,
        encoded_byte_align: head[4] & 2 != 0,
        end_of_block: head[4] & 4 != 0,
        damaged_rows_before_error: u32::from(head[5]),
    };
    let mut checked = Checked {
        columns: parameters.columns,
        lines: 0,
    };
    match decode(coded, &parameters, &mut checked) {
        Ok(summary) => {
            assert_eq!(summary.rows, checked.lines);
            assert!(summary.rows <= parameters.rows);
            assert!(summary.concealed <= summary.rows);
            assert!(summary.concealed <= parameters.damaged_rows_before_error);
        }
        Err(Error::Damaged { row, delivered, .. }) => {
            assert_eq!(delivered.rows, checked.lines);
            assert_eq!(row, delivered.rows);
            assert!(delivered.rows < parameters.rows);
            assert!(delivered.concealed <= parameters.damaged_rows_before_error);
        }
        Err(Error::NoColumns) => assert_eq!(parameters.columns, 0),
    }
});
