//! Fuzzes ISO 32000-2 §12.7.4.3's variable text: the `/DA` parser and the layout it drives.
//!
//! This is the one place in the tree that *writes* a content stream, and everything it writes
//! comes from the document: the `/DA` string is content-stream syntax a file states, the `/V`
//! is a §7.9.2.2 text string a file states, and `/MaxLen` decides how many combs the layout
//! divides a box into. So the fuzzer's bytes go into all three at once, inside a document that
//! is otherwise well formed and carries a widget on its first page.
//!
//! Three properties, none of which is "the text is in the right place":
//!
//! - **No panic.** `/DA` operands are numbers a file chooses, and a size, a leading or a
//!   horizontal scaling of zero, a negative or a NaN reaches every arithmetic step of the
//!   layout — including the divisions that place a comb's cells and centre a line.
//! - **No unbounded loop.** Auto-sizing bisects, line breaking consumes one code per
//!   iteration and the value is bounded by `MAX_CODES`; a break condition that depended on a
//!   width the document controls is what this would notice.
//! - **No unbounded allocation.** One glyph per character reaches the display list, and the
//!   content stream this builds grows with the value.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::{Document, Limits};

/// Wraps the fuzzer's bytes as a widget's `/DA` and `/V` on an otherwise valid page.
///
/// The bytes are split in half so that one input varies both, which matters because the two
/// interact: the `/DA`'s size decides whether the value is auto-sized, and the value's length
/// decides how many lines the size has to fit.
fn document_around(bytes: &[u8]) -> Vec<u8> {
    let (appearance, value) = bytes.split_at(bytes.len() / 2);
    let mut out = Vec::with_capacity(bytes.len().saturating_add(768));
    out.extend_from_slice(b"%PDF-1.7\n");
    out.extend_from_slice(
        b"1 0 obj <</Type /Catalog /Pages 2 0 R /AcroForm \
          <</Fields [5 0 R] /DR <</Font <</Helv 6 0 R>>>>>>>> endobj\n",
    );
    out.extend_from_slice(b"2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj\n");
    out.extend_from_slice(
        b"3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
          /Contents 4 0 R /Annots [5 0 R]>> endobj\n",
    );
    out.extend_from_slice(b"4 0 obj <</Length 0>> stream\n\nendstream endobj\n");

    out.extend_from_slice(
        b"5 0 obj <</Type /Annot /Subtype /Widget /Rect [10 20 190 80] /F 4 /FT /Tx \
          /T (fuzzed) /Ff 16777216 /MaxLen 7 /Q 1 /DA <",
    );
    hex(&mut out, appearance);
    out.extend_from_slice(b"> /V <");
    hex(&mut out, value);
    out.extend_from_slice(b">>> endobj\n");

    out.extend_from_slice(
        b"6 0 obj <</Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding>> endobj\n",
    );
    // No cross-reference table: `Document::open` recovers by scanning, which every fuzz
    // target here relies on and which the corpus exercises daily.
    out.extend_from_slice(b"trailer <</Size 7 /Root 1 0 R>>\n%%EOF\n");
    out
}

/// Writes bytes as a §7.3.4.3 hexadecimal string's digits.
///
/// Hexadecimal rather than literal so that the fuzzer's bytes cannot close the string or the
/// dictionary around it: an input that terminated the object early would test the recovery
/// scanner rather than the clause this target is aimed at.
fn hex(out: &mut Vec<u8>, bytes: &[u8]) {
    for byte in bytes {
        out.push(b"0123456789ABCDEF"[usize::from(byte >> 4)]);
        out.push(b"0123456789ABCDEF"[usize::from(byte & 0x0f)]);
    }
}

fuzz_target!(|bytes: &[u8]| {
    // A value longer than this says nothing new about the clause and turns every input into
    // a memory test of the layout rather than a test of its rules.
    if bytes.len() > 4096 {
        return;
    }
    let Ok(document) = Document::open_with_limits(document_around(bytes), Limits::default()) else {
        return;
    };
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        return;
    };
    let _ = pdf_model::interpret(&document, &page);
});
