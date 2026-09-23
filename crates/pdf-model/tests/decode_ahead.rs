//! A page whose images are decoded ahead of their `Do`s is the page decoded at them (ADR 1321).
//!
//! `pdf_model::interpret` decodes a page's images on rayon's pool beside the run when the pool
//! has a thread to spare, and each `Do` takes its raster when it gets there. The owner's answer
//! to `doc/questions/Q121` is that nothing may be presented before the photograph it belongs to
//! exists, and what that makes testable is a stronger statement: **the interpretation is the
//! same value whichever route each raster arrived by.** So each test interprets one fixture on a
//! one-thread pool, where nothing is decoded ahead, and on a four-thread pool, where the walk
//! runs, and asks for the two interpretations to be one — every command, every report.
//!
//! The fixture is built to meet every place a decode ahead could disagree with the `Do`: images
//! drawn from the page's content, from a form's and from §8.7.3's tiling pattern cell; an image
//! stating Table 87's `/Intent`; an image drawn again after `ri` has changed the intent it is
//! converted under, which the decode ahead was not started under; a `/CS0` named through §7.8.3
//! Table 34's `/ColorSpace` entry; §8.9.6.2's stencil, which is never decoded ahead; and an image
//! whose samples stop short of its grid, whose report has to arrive with the raster.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

/// A body of numbered objects with the cross-reference section its offsets need.
fn assembled(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A 128 × 128 eight-bit grey image as an `ASCIIHexDecode` stream object, its samples a ramp
/// that differs per `seed`, so that two images answered for each other would show.
///
/// 128 × 128 is `image::AHEAD_FLOOR` exactly, the smallest image the walk offers. `extra` is
/// added to the dictionary; `rows` short of 128 leaves the grid unfilled.
fn grey_image(number: u32, seed: u8, extra: &str, rows: usize) -> String {
    let mut hex = String::new();
    for row in 0..rows {
        for column in 0..128usize {
            let value = (row.wrapping_mul(3) ^ column).wrapping_add(usize::from(seed)) & 0xFF;
            let _ = write!(hex, "{value:02X}");
        }
    }
    hex.push('>');
    format!(
        "{number} 0 obj\n<< /Type /XObject /Subtype /Image /Width 128 /Height 128 \
         /BitsPerComponent 8 /ColorSpace /DeviceGray /Filter /ASCIIHexDecode {extra} \
         /Length {} >>\nstream\n{hex}\nendstream\nendobj\n",
        hex.len()
    )
}

/// A stream object holding `content`.
fn stream(number: u32, dict: &str, content: &str) -> String {
    format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len()
    )
}

/// The fixture: one page, and every route an image reaches a `Do` by.
fn fixture() -> Vec<u8> {
    let content = "q 20 0 0 20 0 0 cm /Im1 Do Q \
                   q 20 0 0 20 20 0 cm /Im2 Do Q \
                   q 20 0 0 20 40 0 cm /Im3 Do Q \
                   /Fm Do \
                   q /Pattern cs /P scn 0 40 60 20 re f Q \
                   q 1 0 0 rg 20 0 0 20 0 60 cm /St Do Q \
                   q 20 0 0 20 20 60 cm /Short Do Q \
                   /Perceptual ri q 20 0 0 20 40 60 cm /Im1 Do Q";
    let mut body = String::new();
    body.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.push_str(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 80 80] /Contents 4 0 R \
         /Resources << /ColorSpace << /CS0 /DeviceGray >> \
         /XObject << /Im1 5 0 R /Im2 6 0 R /Im3 7 0 R /Fm 8 0 R /St 9 0 R /Short 12 0 R >> \
         /Pattern << /P 10 0 R >> >> >>\nendobj\n",
    );
    body.push_str(&stream(4, "", content));
    body.push_str(&grey_image(5, 0, "", 128));
    body.push_str(&grey_image(6, 40, "", 128).replace("/DeviceGray", "/CS0"));
    body.push_str(&grey_image(7, 80, "/Intent /Perceptual", 128));
    body.push_str(&stream(
        8,
        "/Type /XObject /Subtype /Form /BBox [0 0 80 80] \
         /Resources << /XObject << /Im4 11 0 R >> >>",
        "q 20 0 0 20 60 0 cm /Im4 Do Q",
    ));
    body.push_str(&stream(
        9,
        "/Type /XObject /Subtype /Image /Width 16 /Height 2 /ImageMask true \
         /Filter /ASCIIHexDecode",
        "F0F00F0F>",
    ));
    body.push_str(&stream(
        10,
        "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] \
         /XStep 20 /YStep 20 /Resources << /XObject << /Im5 13 0 R >> >>",
        "20 0 0 20 0 0 cm /Im5 Do",
    ));
    body.push_str(&grey_image(11, 120, "", 128));
    body.push_str(&grey_image(12, 160, "", 64));
    body.push_str(&grey_image(13, 200, "", 128));
    assembled(&body)
}

/// Page one of `bytes`, interpreted on a pool of `threads`, as the text of the whole value.
fn interpreted(bytes: &[u8], threads: usize) -> String {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("a pool");
    pool.install(|| {
        let document = pdf_syntax::Document::open(bytes.to_vec()).expect("the fixture opens");
        let page = pdf_model::Pages::new(&document)
            .get(0)
            .expect("the page exists");
        format!("{:?}", pdf_model::interpret(&document, &page))
    })
}

#[test]
fn a_page_decoded_ahead_is_the_page_decoded_at_its_dos() {
    let bytes = fixture();
    let alone = interpreted(&bytes, 1);
    // Every route drew: five images of their own, the pattern's cell, the stencil, and the
    // short image — whose report is the one a decode ahead would lose if it lost any.
    assert!(
        alone.matches("Image {").count() >= 7,
        "the fixture should draw its images"
    );
    for _ in 0..8 {
        assert_eq!(
            interpreted(&bytes, 4),
            alone,
            "a page whose images were decoded ahead is not the page decoded at its Dos"
        );
    }
}
