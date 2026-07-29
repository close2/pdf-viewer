//! What `CCITTFaxDecode` delivers, checked without asking another renderer.
//!
//! # What can be checked here, and what cannot
//!
//! ISO 32000-2 §7.4.6 defers the *coding* entirely to ITU-T T.4 and T.6 — it says in as many
//! words that "the encoding algorithm is not described in detail in this document" — so
//! nothing in the PDF standard states what a given bit stream decodes to. The clause's own
//! content is Table 11's eight parameters, and that is exactly what this file checks:
//! statements the standard makes, on data the corpus supplies, with no reference render
//! involved.
//!
//! Two invariants do the work, and both come from the standard's own sentences rather than
//! from anyone's output:
//!
//! - **`/BlackIs1` inverts, exactly.** Table 11 defines it as "whether bits with a value of 1
//!   shall be interpreted as black pixels and 0 bits as white pixels, **the reverse of the
//!   normal PDF syntactic convention** for image data". Reverse is a strong word: decoding one
//!   stream under both settings must produce two images that are bitwise complements, sample
//!   for sample. A decoder that applied the flag anywhere except at the last step — to the
//!   reference line, say, or only to run colours — would produce two plausible images that
//!   are not complements.
//! - **A scan of a printed page is mostly white.** Weaker, and it catches the one error the
//!   complement invariant cannot: a *global* inversion, which is self-consistent and turns a
//!   page of text into a photographic negative. Two percent ink is what a text page has; fifty
//!   is what its negative has.
//!
//! What is deliberately not here is an expected picture. There is no specification-supplied
//! CCITT bit stream to decode, unlike §7.4.7's worked JBIG2 example, and inventing one by
//! encoding with some other library would make that library the oracle. Whether the pixels
//! are *right* is `oracle.rs`'s question, over the 12 corpus documents that carry one.

#![expect(
    clippy::print_stdout,
    reason = "test code: the survey output is the point of the run"
)]

use std::path::{Path, PathBuf};

use pdf_sandbox::{CcittParameters, Decoded, Request};
use pdf_syntax::{Document, Object};

/// A corpus document whose page one is a single CCITT-encoded scan of printed text.
///
/// Named rather than searched for, because the invariants below depend on what it *is* — a
/// scanned sheet, mostly white — and a search would silently pick a different file if the
/// corpus grew one.
const SCANNED_PAGE: &str = "issue5747.pdf";

/// The first CCITT-encoded image in a document, with Table 11's parameters resolved.
///
/// `None` means one thing only: the corpus submodule is not checked out. **A document that is
/// present but has no CCITT image is a panic**, not a skip, and the distinction is not
/// pedantry — the first draft of this file named `bug1001080.pdf`, whose fax data is inside a
/// Type 3 font's glyph descriptions rather than in an image `XObject`, and both tests below
/// "passed" by quietly doing nothing.
fn scanned_image(name: &str) -> Option<(Vec<u8>, CcittParameters)> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).unwrap_or_else(|e| panic!("{name} does not open: {e}"));

    // Page one's own image `XObject`s, which is what the page draws — a walk of every object
    // in the file would also find images no page uses.
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .unwrap_or_else(|| panic!("{name} has no page one"));
    let Object::Dictionary(xobjects) = document.get_key(&page.resources, "XObject") else {
        panic!("{name} page one names no XObject resources");
    };
    for (_, entry) in xobjects.iter() {
        let Object::Stream(stream) = document.resolve(entry) else {
            continue;
        };
        let Some(image) = document.image_stream(&stream) else {
            continue;
        };
        if image.codec.as_deref() != Some(b"CCITTFaxDecode") {
            continue;
        }
        let parms = image.parms.as_ref();
        let integer = |key: &str, default: i64| {
            parms
                .map(|parms| document.get_key(parms, key))
                .and_then(|value| value.as_integer())
                .unwrap_or(default)
        };
        let height = document
            .get_key(&stream.dict, "Height")
            .as_integer()
            .unwrap_or(0);
        let parameters = CcittParameters {
            k: i32::try_from(integer("K", 0)).unwrap_or(0),
            columns: u32::try_from(integer("Columns", 1728)).unwrap_or(1728),
            rows: match u32::try_from(integer("Rows", 0)).unwrap_or(0) {
                0 => u32::try_from(height).unwrap_or(0),
                rows => rows,
            },
            end_of_line: false,
            encoded_byte_align: false,
            end_of_block: true,
            black_is_1: false,
        };
        return Some((image.data.to_vec(), parameters));
    }
    panic!("{name} page one has no CCITTFaxDecode image; this test needs one")
}

/// Decodes one request, or explains why the test cannot run.
fn decode(data: &[u8], parameters: CcittParameters) -> Vec<u8> {
    match pdf_sandbox::decode(&Request::Ccitt { data, parameters }) {
        Ok(Decoded::Bilevel(bilevel)) => bilevel.rows,
        Ok(other) => panic!("CCITTFaxDecode produced {other:?} rather than bilevel samples"),
        Err(error) => panic!(
            "the sandboxed decoder refused a corpus image: {error}\n\
             `cargo build --release -p pdf-sandbox --bins` builds the worker; see trap 10."
        ),
    }
}

/// ISO 32000-2 §7.4.6 Table 11: `/BlackIs1` is "the reverse of the normal PDF syntactic
/// convention", so the two settings must give bitwise complements.
///
/// This is the whole of what the clause says about the entry, turned into an assertion. It
/// needs no reference renderer and no expected image: the statement is about two decodes of
/// the *same* data, so the corpus supplies the input and the standard supplies the
/// expectation.
#[test]
fn black_is_1_gives_the_exact_complement() {
    let Some((data, parameters)) = scanned_image(SCANNED_PAGE) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let normal = decode(
        &data,
        CcittParameters {
            black_is_1: false,
            ..parameters
        },
    );
    let reversed = decode(
        &data,
        CcittParameters {
            black_is_1: true,
            ..parameters
        },
    );

    assert_eq!(
        normal.len(),
        reversed.len(),
        "the same stream must produce the same number of samples either way"
    );
    let disagreeing = normal
        .iter()
        .zip(&reversed)
        .filter(|(left, right)| **left != !**right)
        .count();
    assert_eq!(
        disagreeing,
        0,
        "{disagreeing} of {} packed bytes are not the complement of their counterpart",
        normal.len()
    );
}

/// A scan of a printed page is mostly white, which pins the sense against a global inversion.
///
/// The complement invariant above cannot see this: a decoder that inverted *everything*
/// satisfies it perfectly. What settles it is that `DeviceGray` gives white the sample value
/// 1 (§8.6.4.2), a scanned sheet of text is a few percent ink, and its negative is not.
///
/// The bound is deliberately loose — anything under a fifth — because the assertion being
/// made is "this is a page, not its negative", and tightening it would make the test about
/// this particular document's ink coverage instead.
#[test]
fn a_scanned_page_comes_back_mostly_white() {
    let Some((data, parameters)) = scanned_image(SCANNED_PAGE) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let rows = decode(&data, parameters);
    let black: u32 = rows.iter().map(|byte| byte.count_zeros()).sum();
    let total = u32::try_from(rows.len())
        .unwrap_or(u32::MAX)
        .saturating_mul(8);
    let ink = f64::from(black) / f64::from(total.max(1));
    println!("{SCANNED_PAGE}: {:.2}% of samples are black", ink * 100.0);

    assert!(
        ink < 0.2,
        "a scanned page of text is a few percent ink; {:.1}% black means the sense is \
         inverted, not that the page is dark",
        ink * 100.0
    );
    assert!(
        ink > 0.0,
        "an all-white result means nothing decoded at all"
    );
}
