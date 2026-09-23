//! §11.6.5.2's pre-blending undone on a `JPXDecode` parent decoded at a reduced resolution level.
//!
//! ISO 32000-2 §7.4.9 NOTE 3 lets a processor "select and decode only the data making up a
//! lower-resolution version", and this tree's confined decoder does so for a codestream over its
//! sample budget. Table 143 pairs a `/Matte`'d mask with the parent's *stated* grid — its
//! `/Width` "shall be the same as the Width value of the parent image" — so on the reduced raster
//! the mask is carried onto the reduced grid over the same footprints and the inversion runs
//! there (ADR 1324).
//!
//! `tests/jpx/preblended_4800.j2k` is a 4800 × 4800 three-component codestream, lossless, six
//! decomposition levels — large enough that `pdf-sandbox`'s budget of 2^26 samples reduces it:
//!
//! ```sh
//! python3 -c "import numpy as np; a=np.empty((3,4800,4800),np.uint8); a[0]=255; \
//!   a[1][:, :2400]=0; a[1][:, 2400:]=191; a[2]=a[1]; a.tofile('parent.raw')"
//! opj_compress -i parent.raw -o preblended_4800.j2k -F 4800,4800,3,8,u -n 6 -r 1
//! ```
//!
//! Its left half is red and its right half is red pre-blended with a white matte at a mask sample
//! of 64: `c′ = m + α × (c − m)` with `m = 255`, `c = 0` and `α = 64 ⁄ 255` is exactly 191 in the
//! green and blue components.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: an explanatory panic is the intended failure mode, and every index and \
              sum is on a 40 × 40 raster and a 4800 × 4800 mask this file sized itself"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;

/// The fixture's side, in samples, on both the parent and its mask.
const SIDE: usize = 4800;

/// A one-page 40 × 40 file drawing the codestream over its whole page, with a `/SMask` whose
/// left half is opaque and right half is 64, and `/Matte [1 1 1]` where `matte` says so.
fn page(matte: bool) -> Vec<u8> {
    let parent = include_bytes!("jpx/preblended_4800.j2k");
    let mut mask = Vec::with_capacity(SIDE * SIDE);
    for _ in 0..SIDE {
        mask.extend(std::iter::repeat_n(255_u8, SIDE / 2));
        mask.extend(std::iter::repeat_n(64_u8, SIDE / 2));
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&mask).expect("writing to memory");
    let mask = encoder.finish().expect("compressing in memory");
    let stream = |number: usize, dict: &str, data: &[u8]| {
        let mut out = format!(
            "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
            data.len()
        )
        .into_bytes();
        out.extend_from_slice(data);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out
    };
    let matte = if matte { " /Matte [1 1 1]" } else { "" };
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream(4, "", b"40 0 0 40 0 0 cm /Im Do"),
        stream(
            5,
            &format!(
                "/Type /XObject /Subtype /Image /Width {SIDE} /Height {SIDE} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /JPXDecode /SMask 6 0 R"
            ),
            parent,
        ),
        stream(
            6,
            &format!(
                "/Type /XObject /Subtype /Image /Width {SIDE} /Height {SIDE} \
                 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode{matte}"
            ),
            &mask,
        ),
    ];
    let mut file = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(file.len());
        file.extend_from_slice(object);
    }
    let xref = file.len();
    let entries = objects.len() + 1;
    file.extend_from_slice(format!("xref\n0 {entries}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    file.extend_from_slice(
        format!("trailer\n<< /Size {entries} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n")
            .as_bytes(),
    );
    file
}

/// The page over white paper at one pixel per unit, and what it reported.
fn draw(bytes: Vec<u8>) -> (Vec<String>, pdf_render::Raster) {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("the sandboxed image decoder is not available: {error}");
    }
    let document = pdf_syntax::Document::open(bytes).expect("the fixture is a PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let reports = interpretation
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect();
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, u64::MAX).expect("the target fits");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the oracle draws the page");
    (reports, raster)
}

/// The RGB at raster pixel `(x, y)`.
fn at(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 3] {
    let index = ((y * raster.width + x) * 4) as usize;
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
    ]
}

/// §11.6.5.2 on a reduced grid: the right half's pre-blending is undone, so what reaches white
/// paper is red at a quarter's opacity — `255 − 64` in green and blue, 191 — rather than the
/// pre-blended pink at that opacity again, which is 239.
///
/// The control is the same page with no `/Matte`, which says the samples are not pre-blended and
/// so draws the pink at a quarter: 239. Both halves have to hold, or the assertion would not
/// discriminate between undoing the blend and a page that happened to draw 191 some other way.
#[test]
fn a_matte_is_undone_on_a_parent_decoded_at_a_reduced_level() {
    let (reports, raster) = draw(page(true));
    assert!(
        reports.is_empty(),
        "the pre-blending is undone, so nothing is owed: {reports:?}"
    );
    let left = at(&raster, 10, 20);
    let right = at(&raster, 30, 20);
    assert!(
        left[0] >= 254 && left[1] <= 1 && left[2] <= 1,
        "the opaque half is red: {left:?}"
    );
    assert!(
        right[0] >= 254 && right[1].abs_diff(191) <= 2 && right[2].abs_diff(191) <= 2,
        "the quarter-opaque half is red at a quarter over white, 191, not the pre-blended pink \
         drawn at a quarter again: {right:?}"
    );

    let (_, control) = draw(page(false));
    let unblended = at(&control, 30, 20);
    assert!(
        unblended[1].abs_diff(239) <= 2,
        "with no /Matte the samples are drawn as they stand, so the page can tell: {unblended:?}"
    );
}
