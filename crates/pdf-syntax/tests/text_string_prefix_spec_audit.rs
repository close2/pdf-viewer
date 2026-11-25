//! Specification audit of ISO 32000-2 §7.9.2.2 (review 2234): a text string this crate writes is
//! read back as the text it was written from.
//!
//! The test is behaviour finding 2234-B03 of
//! `doc/reviews/2234-specification-audit-7-7-7-12.md`, with a control beside the failing half.

use pdf_syntax::text_string;
use pdf_syntax::text_string::encode_text_string;

/// §7.9.2.2.1: "For text strings encoded in UTF-16BE, the first two bytes shall be 254 followed
/// by 255", and "\[f]or text strings encoded in UTF-8, the first three bytes shall be 239 followed
/// by 187, followed by 191". NOTE 3 and NOTE 4 draw the consequence for a writer: the mechanism
/// precludes beginning a `PDFDocEncoding` string with `þÿ` or with `ï»¿` (Errata Collection 3's
/// Issue #161 corrects NOTE 4's first character to idieresis). So a text that begins with either
/// sequence has no `PDFDocEncoding` form, and a writer has to choose a Unicode form for it.
#[test]
fn a_text_beginning_with_a_prefix_spelling_is_written_in_a_unicode_form_spec_audit() {
    // Control: a Latin-1 text with the same characters elsewhere is PDFDocEncoding and reads back.
    for control in ["aþÿ", "xï»¿"] {
        assert_eq!(
            text_string(&encode_text_string(control)),
            control,
            "control: {control:?} round-trips"
        );
    }
    for text in ["þÿ names", "ï»¿ names"] {
        let written = encode_text_string(text);
        assert_eq!(
            text_string(&written),
            text,
            "{text:?} was written as {written:?}, whose first bytes are a Unicode form's prefix"
        );
    }
}
