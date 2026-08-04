//! Fuzzes the two repairs this tree makes to a malformed TrueType glyph table.
//!
//! A font program is a stream in the document, so its bytes are as untrusted as any others,
//! and `pdf_font::repaired_font_program` walks a table directory, a `loca` and a `glyf`
//! taken from them — then *rewrites* an sfnt from what it found. ISO 32000-2 §9.6.3 is the
//! clause; the repairs are ADRs 0170 and 0174.
//!
//! Three properties are under test.
//!
//! **The repair terminates and never panics**, over any byte sequence, including directories
//! that name tables past the end of the data and `loca` tables whose entries are noise.
//!
//! **A repaired program is still an sfnt**, with the same table tags as the input: the repair
//! rewrites two tables and copies the rest, so losing one would be a font the caller cannot
//! load for a reason the repair invented.
//!
//! **The repair is idempotent.** Running it on its own output must change nothing — the
//! rebuilt `loca` ascends by construction, so the second pass has to leave on its first
//! check. A repair that kept finding work to do would be one whose own output it considers
//! malformed, which is the shape of a rewrite that loses information every time.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// The table tags an sfnt directory names, in the order it names them.
fn tags(data: &[u8]) -> Option<Vec<[u8; 4]>> {
    let count = usize::from(u16::from_be_bytes([*data.get(4)?, *data.get(5)?]));
    let mut out = Vec::with_capacity(count.min(64));
    for index in 0..count {
        let at = 12usize.checked_add(index.checked_mul(16)?)?;
        out.push(data.get(at..at.checked_add(4)?)?.try_into().ok()?);
    }
    Some(out)
}

fuzz_target!(|data: &[u8]| {
    let repaired = pdf_font::repaired_font_program(data);

    // Where nothing was repaired the bytes are the input, which is the common path and the
    // only one where the borrow is kept.
    if repaired.as_ref() == data {
        return;
    }

    let before = tags(data);
    let after = tags(&repaired);
    assert_eq!(
        before, after,
        "a repair rewrote two tables and must keep the directory's tags"
    );

    let again = pdf_font::repaired_font_program(&repaired);
    assert_eq!(
        again.as_ref(),
        repaired.as_ref(),
        "the repair must be idempotent: its own output is a well-formed table"
    );
});
