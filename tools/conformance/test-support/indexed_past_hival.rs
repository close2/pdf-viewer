//! ISO 32000-2 §8.6.6.3's out-of-range index, as one document per route an `Indexed` value
//! reaches; the backends' tests share them so that each draws the same pages.
//!
//! > The index value should be an integer in the range 0 to hival . If the value is a real
//! > number, it shall be rounded to the nearest integer (0.5 values shall be rounded up); if it
//! > is outside the range 0 to hival , it shall be adjusted to the nearest value within that
//! > range.
//!
//! The sentence names no route, so it binds every place an index is read: an image `XObject`'s
//! samples at each depth, under a `/Decode` that maps below zero, with a `/Matte`, an inline
//! image, the colour operators, the base of an uncoloured tiling pattern, a mesh shading's vertex
//! colours, an axial shading's function output, and an image painted inside a luminosity soft
//! mask.
//!
//! Every space has `hival` 1 and two entries, so the two ends of the range are two different
//! colours and an index adjusted to the wrong end, or not adjusted at all, draws a third. Each
//! page is four patches of 20 by 20 side by side, and the expected colour of each is the entry the
//! clause selects, read from the table's own bytes; nothing is taken from another renderer.
//!
//! Included by `#[path]`; it names `pdf_render`, `pdf_syntax` and `pdf_model`.

#![allow(
    dead_code,
    reason = "each including test uses the part it needs, and a #[path] module is compiled per crate"
)]
#![allow(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test code over fixed 80 by 20 pages: a failed expectation or an index out of range \
              means a fixture changed shape and should fail loudly; `allow` rather than `expect` \
              because the including crates' lint sets differ"
)]

use std::fmt::Write as _;

/// Entry 0 of every RGB table here: `FF0000`.
pub(crate) const RED: [u8; 3] = [255, 0, 0];
/// Entry 1, `hival`, of every RGB table here: `0000FF`.
pub(crate) const BLUE: [u8; 3] = [0, 0, 255];
/// The page's own white, where nothing is painted.
pub(crate) const WHITE: [u8; 3] = [255, 255, 255];
/// The two-entry RGB space, `hival` 1: red, then blue.
const SPACE: &str = "[/Indexed /DeviceRGB 1 <FF00000000FF>]";
/// The page every document here draws on: four patches of 20 by 20 side by side.
pub(crate) const WIDTH: u32 = 80;
/// The page's height, one row of patches.
pub(crate) const HEIGHT: u32 = 20;

/// One route's page and the colour §8.6.6.3 gives each of its four patches, with the reason.
pub(crate) struct Fixture {
    /// The route, for a failure's message.
    pub(crate) route: String,
    /// The document, one page with an exact cross-reference table.
    pub(crate) pdf: Vec<u8>,
    /// Each patch's colour, left to right, and why the clause gives it.
    pub(crate) patches: [([u8; 3], &'static str); 4],
}

impl Fixture {
    /// Asserts that `raster`, the page drawn at scale 1, shows [`Self::patches`].
    pub(crate) fn assert_drawn(&self, raster: &pdf_render::Raster, backend: &str) {
        assert_eq!(
            (raster.width, raster.height),
            (WIDTH, HEIGHT),
            "{} on {backend}",
            self.route
        );
        for (index, (colour, why)) in (0_u32..).zip(self.patches) {
            let (x, y) = (index * 20 + 10, HEIGHT / 2);
            let at = usize::try_from((y * WIDTH + x) * 4).expect("a fixed 80 by 20 raster");
            let drawn = raster
                .data
                .get(at..at + 3)
                .expect("the centre of a patch is inside the raster");
            assert_eq!(
                drawn,
                colour.as_slice(),
                "{} on {backend}, patch {index}: {why}",
                self.route
            );
        }
    }
}

/// Every route's fixture, in the order the module's documentation names them.
pub(crate) fn every() -> Vec<Fixture> {
    let mut every: Vec<Fixture> = [8, 4, 2].into_iter().map(image_at_depth).collect();
    every.extend([
        sixteen_bit_image(),
        decoded_below_zero(),
        matted_image(),
        inline_image(),
        colour_operators(),
        uncoloured_pattern(),
        mesh_shading(),
        axial_shading(),
        luminosity_soft_mask(),
    ]);
    every
}

/// What the four samples `0, 1, 2, top` select: the first two are in range, and the last two
/// are past `hival` and are "adjusted to the nearest value within that range", which is 1.
const PAST_HIVAL: [([u8; 3], &str); 4] = [
    (RED, "sample 0 is entry 0"),
    (BLUE, "sample 1 is entry 1, hival"),
    (BLUE, "sample 2 is past hival and is adjusted to it"),
    (
        BLUE,
        "the depth's largest sample is past hival and is adjusted to it",
    ),
];

/// An image `XObject` of `bits` per sample holding `0, 1, 2` and the depth's largest value.
pub(crate) fn image_at_depth(bits: u8) -> Fixture {
    let data = match bits {
        8 => vec![0, 1, 2, 255],
        4 => vec![0x01, 0x2F],
        _ => vec![0b0001_1011],
    };
    Fixture {
        route: format!("a {bits}-bit image"),
        pdf: image_page(
            &format!(
                "<< /Type /XObject /Subtype /Image /Width 4 /Height 1 \
                 /ColorSpace {SPACE} /BitsPerComponent {bits} >>"
            ),
            &data,
        ),
        patches: PAST_HIVAL,
    }
}

/// Table 87 admits 16 bits a component (PDF 1.5); the samples are `0, 1, 2, 65535`.
pub(crate) fn sixteen_bit_image() -> Fixture {
    Fixture {
        route: "a 16-bit image".to_owned(),
        pdf: image_page(
            &format!(
                "<< /Type /XObject /Subtype /Image /Width 4 /Height 1 \
                 /ColorSpace {SPACE} /BitsPerComponent 16 >>"
            ),
            &[0, 0, 0, 1, 0, 2, 0xFF, 0xFF],
        ),
        patches: PAST_HIVAL,
    }
}

/// §8.9.5.2's `/Decode` maps sample `s` to `Dmin + s × (Dmax − Dmin) / 255`, so `[-5 250]`
/// takes the samples `0, 5, 6, 255` to `-5, 0, 1, 250`: below the range, its two ends, and past
/// it.
pub(crate) fn decoded_below_zero() -> Fixture {
    Fixture {
        route: "an image under /Decode [-5 250]".to_owned(),
        pdf: image_page(
            &format!(
                "<< /Type /XObject /Subtype /Image /Width 4 /Height 1 \
                 /ColorSpace {SPACE} /BitsPerComponent 8 /Decode [-5 250] >>"
            ),
            &[0, 5, 6, 255],
        ),
        patches: [
            (RED, "-5 is below the range and is adjusted to 0"),
            (RED, "0 is entry 0"),
            (BLUE, "1 is entry 1, hival"),
            (BLUE, "250 is past hival and is adjusted to it"),
        ],
    }
}

/// Table 144's `/Matte` restores "the colour values in the colour table (not the index values
/// themselves)" (§11.6.5.2), so the entry is selected first; an opaque mask leaves it as it is.
pub(crate) fn matted_image() -> Fixture {
    Fixture {
        route: "a matted image".to_owned(),
        pdf: document(
            "<< /XObject << /Im 5 0 R >> >>",
            format!("q {WIDTH} 0 0 {HEIGHT} 0 0 cm /Im Do Q").as_bytes(),
            &[
                (
                    format!(
                        "<< /Type /XObject /Subtype /Image /Width 4 /Height 1 \
                         /ColorSpace {SPACE} /BitsPerComponent 8 /SMask 6 0 R >>"
                    ),
                    Some(vec![0, 1, 2, 255]),
                ),
                (
                    "<< /Type /XObject /Subtype /Image /Width 4 /Height 1 \
                     /ColorSpace /DeviceGray /BitsPerComponent 8 /Matte [0 0 0] >>"
                        .to_owned(),
                    Some(vec![255; 4]),
                ),
            ],
        ),
        patches: PAST_HIVAL,
    }
}

/// §8.9.7's inline image: the abbreviation `/I` is `Indexed` inside its colour space array
/// (Table 91), and the data is the same four samples as hexadecimal.
pub(crate) fn inline_image() -> Fixture {
    let content = format!(
        "q {WIDTH} 0 0 {HEIGHT} 0 0 cm BI /W 4 /H 1 /BPC 8 \
         /CS [/I /RGB 1 <FF00000000FF>] /F /AHx ID 000102FF> EI Q"
    );
    Fixture {
        route: "an inline image".to_owned(),
        pdf: document("<<>>", content.as_bytes(), &[]),
        patches: PAST_HIVAL,
    }
}

/// §8.6.8's `scn` and `SCN` in an `Indexed` space take the index as their one operand, and the
/// clause's sentence is the one that decides a number outside it — the example above it is
/// `123 sc` itself.
pub(crate) fn colour_operators() -> Fixture {
    let content = "/Idx cs -3 scn 0 0 20 20 re f 7 scn 20 0 20 20 re f \
                   /Idx CS 20 w 1.5 SCN 50 0 m 50 20 l S -0.4 SCN 70 0 m 70 20 l S";
    Fixture {
        route: "the colour operators".to_owned(),
        pdf: document(
            &format!("<< /ColorSpace << /Idx {SPACE} >> >>"),
            content.as_bytes(),
            &[],
        ),
        patches: [
            (RED, "`-3 scn` is below the range and is adjusted to 0"),
            (BLUE, "`7 scn` is past hival and is adjusted to it"),
            (BLUE, "`1.5 SCN` rounds up to 2 and is adjusted to hival"),
            (RED, "`-0.4 SCN` rounds to 0"),
        ],
    }
}

/// §8.7.3.3's uncoloured tiling pattern is painted in the underlying space of a `[/Pattern
/// base]` space, so the `scn` operands before the pattern's name are an index into the `Indexed`
/// base.
pub(crate) fn uncoloured_pattern() -> Fixture {
    Fixture {
        route: "an uncoloured tiling pattern".to_owned(),
        pdf: document(
            &format!("<< /ColorSpace << /P [/Pattern {SPACE}] >> /Pattern << /Tile 5 0 R >> >>"),
            b"/P cs 9 /Tile scn 0 0 40 20 re f -9 /Tile scn 40 0 40 20 re f",
            &[(
                "<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 \
                 /BBox [0 0 4 4] /XStep 4 /YStep 4 /Resources << >> >>"
                    .to_owned(),
                Some(b"0 0 4 4 re f".to_vec()),
            )],
        ),
        patches: [
            (BLUE, "`9` is past hival and is adjusted to it"),
            (BLUE, "`9` is past hival and is adjusted to it"),
            (RED, "`-9` is below the range and is adjusted to 0"),
            (RED, "`-9` is below the range and is adjusted to 0"),
        ],
    }
}

/// §8.7.4.5.5's free-form mesh without a `/Function` carries one colour component per vertex,
/// here an index decoded through `/Decode`'s last pair `[0 255]`: two triangles of index 200
/// cover the left half of the page and two of index 0 the right.
pub(crate) fn mesh_shading() -> Fixture {
    let mut data: Vec<u8> = Vec::new();
    for (x_offset, index) in [(0_u8, 200_u8), (128, 0)] {
        // Flag, x, y and index per vertex; x and y in 0..=255 decode to 0..=80 and 0..=20.
        for (x, y) in [
            (0_u8, 0_u8),
            (127, 0),
            (0, 255),
            (127, 0),
            (127, 255),
            (0, 255),
        ] {
            data.extend_from_slice(&[0, x.saturating_add(x_offset), y, index]);
        }
    }
    Fixture {
        route: "a free-form mesh".to_owned(),
        pdf: document(
            "<< /Shading << /Mesh 5 0 R >> >>",
            b"/Mesh sh",
            &[(
                format!(
                    "<< /ShadingType 4 /ColorSpace {SPACE} /BitsPerCoordinate 8 \
                     /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 80 0 20 0 255] >>"
                ),
                Some(data),
            )],
        ),
        patches: [
            (BLUE, "index 200 is past hival and is adjusted to it"),
            (BLUE, "index 200 is past hival and is adjusted to it"),
            (RED, "index 0 is entry 0"),
            (RED, "index 0 is entry 0"),
        ],
    }
}

/// §8.7.4.5.3's axial shading takes its colour from its function, whose one output is here an
/// index: a Type 2 function with `C0 [5]` and `C1 [5]` outputs 5 everywhere, past `hival`.
pub(crate) fn axial_shading() -> Fixture {
    const WHY: &str = "the function's 5 is past hival and is adjusted to it";
    Fixture {
        route: "an axial shading".to_owned(),
        pdf: document(
            "<< /Shading << /Axial 5 0 R >> >>",
            b"/Axial sh",
            &[(
                format!(
                    "<< /ShadingType 2 /ColorSpace {SPACE} /Coords [0 0 80 0] \
                     /Function << /FunctionType 2 /Domain [0 1] /C0 [5] /C1 [5] /N 1 >> >>"
                ),
                None,
            )],
        ),
        patches: [(BLUE, WHY); 4],
    }
}

/// §11.6.5.2's luminosity soft mask is the luminosity of its group's drawing. The group draws a
/// grey `Indexed` image whose table is black then white (`<00FF>`): samples past `hival` are
/// white, so the mask is opaque there and the red fill under it shows; were they black, the mask
/// would hide the fill and the page's white would show.
pub(crate) fn luminosity_soft_mask() -> Fixture {
    Fixture {
        route: "a luminosity soft mask".to_owned(),
        pdf: document(
            "<< /ExtGState << /Masked << /SMask << /Type /Mask /S /Luminosity /G 5 0 R >> >> >> >>",
            format!("/Masked gs 1 0 0 rg 0 0 {WIDTH} {HEIGHT} re f").as_bytes(),
            &[
                (
                    format!(
                        "<< /Type /XObject /Subtype /Form /BBox [0 0 {WIDTH} {HEIGHT}] \
                         /Group << /S /Transparency /CS /DeviceGray >> \
                         /Resources << /XObject << /Im 6 0 R >> >> >>"
                    ),
                    Some(format!("q {WIDTH} 0 0 {HEIGHT} 0 0 cm /Im Do Q").into_bytes()),
                ),
                (
                    "<< /Type /XObject /Subtype /Image /Width 4 /Height 1 \
                     /ColorSpace [/Indexed /DeviceGray 1 <00FF>] /BitsPerComponent 8 >>"
                        .to_owned(),
                    Some(vec![1, 0, 2, 255]),
                ),
            ],
        ),
        patches: [
            (RED, "sample 1 is white, so the mask shows the fill"),
            (WHITE, "sample 0 is black, so the mask hides the fill"),
            (RED, "sample 2 is past hival, adjusted to white"),
            (RED, "sample 255 is past hival, adjusted to white"),
        ],
    }
}

/// An image `XObject` of four samples, drawn across the page's four patches.
fn image_page(dictionary: &str, data: &[u8]) -> Vec<u8> {
    document(
        "<< /XObject << /Im 5 0 R >> >>",
        format!("q {WIDTH} 0 0 {HEIGHT} 0 0 cm /Im Do Q").as_bytes(),
        &[(dictionary.to_owned(), Some(data.to_vec()))],
    )
}

/// A one-page document whose page has `resources` and `content`, followed by `extra` objects
/// numbered from 5, each a dictionary and its stream data (`None` for a plain dictionary).
fn document(resources: &str, content: &[u8], extra: &[(String, Option<Vec<u8>>)]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {WIDTH} {HEIGHT}] \
             /Resources {resources} /Contents 4 0 R >>"
        )
        .into_bytes(),
        stream("<<>>", content),
    ];
    for (dictionary, data) in extra {
        objects.push(match data {
            Some(data) => stream(dictionary, data),
            None => dictionary.clone().into_bytes(),
        });
    }
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, object) in (1_usize..).zip(&objects) {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = pdf.len();
    let size = offsets.len() + 1;
    let mut tail = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        writeln!(tail, "{offset:010} 00000 n ").expect("a String takes every write");
    }
    write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    )
    .expect("a String takes every write");
    pdf.extend_from_slice(tail.as_bytes());
    pdf
}

/// A stream object: `dictionary`, given the `/Length` of `data`, and `data`.
fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let open = dictionary
        .strip_suffix(">>")
        .expect("every dictionary here ends in `>>`");
    let mut bytes = format!("{open} /Length {} >>\nstream\n", data.len()).into_bytes();
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(b"\nendstream");
    bytes
}

/// The page's display list, after asserting the cross-reference table is exact and nothing on
/// the page was refused.
pub(crate) fn interpreted(fixture: &Fixture) -> pdf_render::DisplayList {
    let document = pdf_syntax::Document::open(fixture.pdf.clone()).expect("the document opens");
    assert!(
        !document.was_recovered(),
        "{}: the xref is exact",
        fixture.route
    );
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the document has a page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{}: {:?}",
        fixture.route,
        interpretation.unsupported
    );
    interpretation.display_list
}
