//! ISO 32000-2 §8.6.6.3's out-of-range index, on a document that states the whole rule.
//!
//! `doc/corpora/pdf-differences/IndexedColor/IndexedCS_negative_and_high.pdf` is 1798 bytes of
//! hand-written PDF whose page is two rows of colour patches over one `Indexed` space of eight
//! entries. The clause is one sentence and it decides every patch in the upper row:
//!
//! > The index value should be an integer in the range 0 to *hival* . If the value is a real
//! > number, it shall be rounded to the nearest integer (0.5 values shall be rounded up); if it
//! > is outside the range 0 to *hival* , it shall be adjusted to the nearest value within that
//! > range.
//!
//! **The modal verbs are split across it and the split is the point.** Staying inside the range
//! is what a *producer* `should` do, so a document putting `-17` there is conforming; what a
//! *reader* does with one is two `shall`s. `-17 sc` selects entry 0, `6.5 sc` rounds up to 7
//! rather than to 6, and `17 sc` is adjusted to `hival`. Three of the eleven patches are
//! therefore statements about the rule and the other eight are the identity, which is what makes
//! one assertion over the row worth writing: a reader that clamped, rounded or did neither draws
//! a different row each time.
//!
//! # Why the expected colours are the file's own palette and not its reference row
//!
//! The document's lower row paints the same eleven colours with `rg` operators, and its README
//! says the two rows "should match exactly". **They do not, and the arithmetic says so before any
//! renderer does**: the palette's last entry is `F380FF` and the reference row writes
//! `0.95 0.5 1 rg` for it, where 0.95 × 255 is 242.25 and 0xF3 is 243. This tree draws 243 in the
//! upper row and 242 in the lower, on all three of those patches, which is the file's own
//! rounding rather than a defect — so comparing the rows would gate a decimal literal.
//!
//! The palette is the fourth element of the space's own array (§8.6.6.3's "a four-element
//! array"), read out of the file below, and the index each patch selects is the clause applied to
//! the number the content stream writes. Nothing here is derived from another renderer, and the
//! corpus's published picture is not consulted at all.

#![expect(
    clippy::indexing_slicing,
    reason = "test code: the page's geometry is fixed by the document, so a slice out of \
              range would mean the corpus file changed shape and should fail loudly"
)]

use std::path::Path;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// The eight entries of the document's `/Indexed` lookup string, `hival` 7.
///
/// Object 4 of the file, verbatim: `< 008000 FF0000 00FF00 0000FF 00FFFF FF00FF FFFF00 F380FF >`.
const PALETTE: [[u8; 3]; 8] = [
    [0x00, 0x80, 0x00],
    [0xff, 0x00, 0x00],
    [0x00, 0xff, 0x00],
    [0x00, 0x00, 0xff],
    [0x00, 0xff, 0xff],
    [0xff, 0x00, 0xff],
    [0xff, 0xff, 0x00],
    [0xf3, 0x80, 0xff],
];

/// The upper row's eleven patches: the number the content stream writes, and the entry
/// §8.6.6.3 makes it select.
///
/// The three that are not the identity are the whole subject: `-17` is below the range and is
/// "adjusted to the nearest value within that range", `6.5` is a real number and "0.5 values
/// shall be rounded up", and `17` is above `hival`.
const PATCHES: [(&str, usize); 11] = [
    ("-17", 0),
    ("0", 0),
    ("1", 1),
    ("2", 2),
    ("3", 3),
    ("4", 4),
    ("5", 5),
    ("6", 6),
    ("7", 7),
    ("6.5", 7),
    ("17", 7),
];

/// The page is 500 by 100; each patch is 20 by 20 with its lower-left corner at these x, and
/// the upper row's at y 50.
const LEFT: [u32; 11] = [5, 30, 55, 80, 105, 130, 155, 180, 205, 230, 255];
const PAGE_HEIGHT: u32 = 100;
const ROW_BOTTOM: u32 = 50;
const PATCH: u32 = 20;

#[test]
fn an_out_of_range_index_is_adjusted_to_the_nearest_entry() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/corpora/pdf-differences/IndexedColor/IndexedCS_negative_and_high.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        // A corpus that is not checked out is a skip, which is what `tests/corpus.rs` does:
        // none of the four under `doc/corpora/` is needed for a build.
        println!("skipped: {} is not checked out", path.display());
        return;
    };

    let document = Document::open(bytes).expect("the witness opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the witness has a page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert_eq!(
        interpretation.unsupported,
        Vec::new(),
        "nothing on a page of eleven filled rectangles should be refused"
    );

    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 16 << 20)
        .expect("a 500 by 100 page is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("nothing on the page is refused by the CPU backend");
    assert_eq!((raster.width, raster.height), (500, PAGE_HEIGHT));

    // The centre of a patch, which is solid: user space is y-up and the raster y-down.
    let centre = |left: u32| -> [u8; 3] {
        let x = left + PATCH / 2;
        let y = PAGE_HEIGHT - (ROW_BOTTOM + PATCH / 2);
        let at = ((y * raster.width + x) * 4) as usize;
        [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
    };

    for (index, ((written, entry), left)) in PATCHES.iter().zip(LEFT).enumerate() {
        assert_eq!(
            centre(left),
            PALETTE[*entry],
            "patch {} of the upper row is `{written} sc`, which §8.6.6.3 makes entry {entry}",
            index + 1
        );
    }
}
