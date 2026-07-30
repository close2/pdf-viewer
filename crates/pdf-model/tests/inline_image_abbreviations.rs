//! ISO 32000-2 §8.9.7's abbreviations, against a document that states its own answer.
//!
//! `issue14256.pdf` is a `SafeDocs` conformance file — "Inline Image Test for abbreviated and
//! full key names" — whose page carries the *same picture eight times*, each written a
//! different way: full names only, abbreviations only, and six pairings of a correct
//! abbreviation with a contradicting full name for `/Width`, `/Filter`, `/ColorSpace`,
//! `/Decode`, `/Interpolate` and `/DecodeParms`.
//!
//! That makes it the shape `tests/jbig2.rs` is built on and the strongest kind of evidence
//! this project has: **the corpus states an invariant about itself**. Eight images that must
//! agree need no reference renderer, so principle 5 is not even in tension — the expectation
//! comes from the document, not from anyone's output.
//!
//! What it does not check is which of the two spellings won. It cannot: the whole point of
//! the file is that both readings are decodable for three of the six, and it is the *bytes*
//! of the other three that settle it — `#4`'s data is plainly ASCII hex against a full
//! `/Filter` naming ASCII85, and `#8`'s Flate stream was PNG-predicted against a full
//! `/DecodeParms` of nulls. The argument lives on `inline_image::expand_key`; what lives here
//! is that the eight pictures agree, which is false under any other rule.

#![expect(
    clippy::indexing_slicing,
    reason = "test code: the page's geometry is fixed by the document, so a slice out of \
              range would mean the corpus file changed shape and should fail loudly"
)]

use std::path::Path;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// The page is 900 by 900 and each image is 200 by 100 at these user-space origins.
const PLACES: [(u32, u32); 8] = [
    (10, 400),
    (220, 400),
    (430, 400),
    (640, 400),
    (10, 100),
    (220, 100),
    (430, 100),
    (640, 100),
];
const PAGE: u32 = 900;
const IMAGE: (u32, u32) = (200, 100);

#[test]
fn eight_spellings_of_one_inline_image_draw_one_picture() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/issue14256.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        // A missing submodule is a skip; a present one that lacks this file is not, and the
        // corpus gate would have said so first.
        return;
    };

    let document = Document::open(bytes).expect("the document opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert_eq!(
        interpretation.unsupported,
        Vec::new(),
        "every one of the eight images should decode"
    );

    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 16 << 20)
        .expect("a 900-point page is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("nothing on the page is refused by the CPU backend");
    assert_eq!(raster.width, PAGE);

    let block = |(x, y): (u32, u32)| -> Vec<u8> {
        // User space is y-up and the raster is y-down, and the image's origin is its lower
        // left corner.
        let top = PAGE - y - IMAGE.1;
        let mut out = Vec::with_capacity((IMAGE.0 * IMAGE.1 * 4) as usize);
        for row in top..top + IMAGE.1 {
            let at = ((row * raster.width + x) * 4) as usize;
            out.extend_from_slice(&raster.data[at..at + (IMAGE.0 * 4) as usize]);
        }
        out
    };

    let first = block(PLACES[0]);
    assert!(
        first
            .chunks_exact(4)
            .any(|pixel| pixel != [255, 255, 255, 255]),
        "the first image should have drawn something"
    );
    for (index, place) in PLACES.iter().enumerate().skip(1) {
        assert!(
            block(*place) == first,
            "image #{} differs from #1, so one of §8.9.7's spellings was read differently",
            index + 1
        );
    }
}
