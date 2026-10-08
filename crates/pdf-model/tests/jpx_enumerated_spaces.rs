//! Every enumerated colour space of the JPX baseline, drawn from a JPEG 2000 file that states it.
//!
//! ISO 32000-2 §7.4.9 puts one sentence about colour spaces to a reader rather than to a file:
//!
//! > PDF processors shall support the JPX baseline set of enumerated colour spaces; they shall
//! > also be responsible for dealing with the interaction between the colour spaces and the bit
//! > depth of samples.
//!
//! "JPX baseline" is defined by ISO/IEC 15444-2, held here as its identical joint text ITU-T T.801
//! (`doc/md/T.801.md`, ADR 1383). Its M.9.2.4 lists the enumerated values — sRGB, sRGB-grey,
//! ROMM-RGB, sYCC, e-sRGB and e-sYCC, CIELab and CIEJab, with or without enumerated parameters —
//! and §7.4.9 adds CMYK (12) as a space "which is part of JPX but not JPX baseline, shall be
//! supported in a PDF file". Each test below states one of them in a `colr` box around the same
//! lossless codestream and draws it with no `/ColorSpace`, so the file's own statement is what
//! decides.
//!
//! **What an expected value may rest on.** The definitions behind three of the codes are in texts
//! this project does not hold — IEC 61966-2-1 Amd. 1 (sYCC), PIMA 7666 (ROMM-RGB), PIMA 7667
//! (e-sRGB, e-sYCC) — and CIE Publication 131 (CIEJab) is the fourth. So where a test asserts a
//! colour it is one the held texts fix: a luma-chroma colour with no chroma is neutral and an RGB
//! colour of three equal components is neutral, whatever the coefficients; CIELab's samples are
//! T.801's Equation M-18 with M.11.7.4.1's defaults, and the same L\*a\*b\* painted through
//! §8.6.5.4's `Lab` is the expected pixel; an ICC profile's colour is the same profile's painted
//! through §8.6.5.5's `ICCBased`. Where the tree does not draw a code as its definition states,
//! the test asserts §7.4.9's own fallback instead — "the colour space used shall be DeviceGray ,
//! DeviceRGB , or DeviceCMYK , depending on the whether the number of ordinary channels in the
//! JPEG 2000 data is 1, 3, or 4" — and not a guess at the definition.
//!
//! The codestreams are generated rather than written, as `jpx_channels.rs`'s is, with the `COM`
//! marker segment removed so no encoder version is baked in:
//!
//! ```sh
//! python3 -c "import numpy as np
//! np.array([(150,128,128),(150,150,150),(255,128,96),(0,128,96)],np.uint8).T.tofile('three.raw')
//! np.full(4, 100, np.uint8).tofile('one.raw')
//! np.array([200,210,100,110,50,60,25,35],np.uint8).tofile('four.raw')"
//! opj_compress -i three.raw -o three.j2k -F 4,1,3,8,u -n 1 -r 1
//! opj_compress -i one.raw -o one.j2k -F 4,1,1,8,u -n 1 -r 1
//! opj_compress -i four.raw -o four.j2k -F 2,1,4,8,u -n 1 -r 1
//! ```

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, and the \
              fixtures are four-pixel pages where no index can overflow"
)]
#![expect(
    clippy::doc_markdown,
    reason = "these comments quote the standard, and a quotation with backticks added to please a \
              lint is no longer a quotation"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the four-pixel pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A 4×1 three-component codestream, reversible 5/3 and so lossless: its pixels are, in order,
/// (150, 128, 128), (150, 150, 150), (255, 128, 96) and (0, 128, 96).
const THREE: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x07, 0x01, 0x01, 0x07, 0x01, 0x01,
    0x07, 0x01, 0x01, 0xff, 0x52, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x04, 0x04, 0x00,
    0x01, 0xff, 0x5c, 0x00, 0x04, 0x40, 0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x24, 0x00, 0x01, 0xff, 0x93, 0xc7, 0xd4, 0x0a, 0x0f, 0xf8, 0xe5, 0xba, 0x7f, 0xc7, 0xd4, 0x06,
    0x0d, 0xef, 0xbf, 0xdf, 0x80, 0x28, 0x0f, 0xf2, 0xb6, 0xc8, 0xf3, 0xff, 0xd9,
];

/// A 4×1 one-component codestream, every sample 100.
const ONE: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0xff, 0x93, 0xc3,
    0xe7, 0x06, 0x09, 0x0f, 0xf1, 0xff, 0xd9,
];

/// A 2×1 four-component codestream: (200, 100, 50, 25) and (210, 110, 60, 35).
const FOUR: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x07, 0x01, 0x01, 0x07, 0x01, 0x01,
    0x07, 0x01, 0x01, 0x07, 0x01, 0x01, 0xff, 0x52, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00,
    0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40, 0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x25, 0x00, 0x01, 0xff, 0x93, 0xc3, 0xe7, 0x04, 0x07, 0x03, 0xc7, 0xd4, 0x06,
    0x08, 0x7f, 0x7f, 0xcf, 0xb4, 0x0c, 0x01, 0xc7, 0x9f, 0xcf, 0xb4, 0x0c, 0x08, 0x29, 0x27, 0xff,
    0xd9,
];

/// T.801 Table M.29's CIE Illuminant D65.
const ILLUMINANT_D65: [u8; 4] = [0x00, 0x44, 0x36, 0x35];

/// T.801 M.11.7.4.1's own example of an illuminant stated by colour temperature alone, 7500 K.
const COLOUR_TEMPERATURE_7500: [u8; 4] = [0x43, 0x54, 0x1D, 0x4C];

/// The white point of a Planckian radiator at 7500 K, as `pdf_colour::planckian` sums it, to four
/// places — what a `Lab` space states to draw a 7500 K CIELab box's colours as that box means them.
const RADIATOR_AT_7500: &str = "0.9680 1 1.2550";

/// T.801 Table M.29's CIE Illuminant D50, the `IL` field's default.
const ILLUMINANT_D50: [u8; 4] = [0x00, 0x44, 0x35, 0x30];

/// M.11.7.4.1's default ranges and eight-bit offsets, stated explicitly: `RL`, `OL`, `RA`, `OA`,
/// `RB`, `OB`.
fn lab_parameters() -> Vec<u8> {
    [100u32, 0, 170, 128, 200, 96]
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

/// A box: `LBox`, `TBox`, payload. ISO/IEC 15444-1 I.4.
fn boxed(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(payload.len() + 8)
        .expect("the fixtures are small")
        .to_be_bytes()
        .to_vec();
    out.extend_from_slice(&kind);
    out.extend_from_slice(payload);
    out
}

/// A `colr` payload for the Enumerated method, T.801 Table M.24: `METH` 1, then `PREC`,
/// `APPROX`, `EnumCS` and the `EP` bytes.
fn enumerated(precedence: u8, space: u32, parameters: &[u8]) -> Vec<u8> {
    let mut out = vec![1, precedence, 1];
    out.extend_from_slice(&space.to_be_bytes());
    out.extend_from_slice(parameters);
    out
}

/// A `colr` payload for the Any ICC method, T.801 M.11.7.3.2.
fn any_icc(precedence: u8, profile: &[u8]) -> Vec<u8> {
    let mut out = vec![3, precedence, 1];
    out.extend_from_slice(profile);
    out
}

/// A JPX file around `codestream`, with one `colr` box per entry of `colours`: signature, file
/// type naming the JPX baseline (M.9.2), a JP2 Header box, the codestream.
fn jpx(codestream: &[u8], (width, components): (u32, u16), colours: &[Vec<u8>]) -> Vec<u8> {
    let mut image_header = 1u32.to_be_bytes().to_vec();
    image_header.extend_from_slice(&width.to_be_bytes());
    image_header.extend_from_slice(&components.to_be_bytes());
    image_header.extend_from_slice(&[7, 7, 0, 0]);
    let mut header = boxed(*b"ihdr", &image_header);
    for colour in colours {
        header.extend(boxed(*b"colr", colour));
    }
    let mut out = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    out.extend(boxed(*b"ftyp", b"jpx \0\0\0\0jpx jpxbjp2 "));
    out.extend(boxed(*b"jp2h", &header));
    out.extend(boxed(*b"jp2c", codestream));
    out
}

/// A one-page PDF, `width` units by one, whose content is `content` and whose objects from 5 on
/// are `objects` — the image `/Im0` is object 5 where a page draws one.
fn document(width: u32, resources: &str, content: &str, objects: &[Vec<u8>]) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} 1] \
             /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}",
            content.len()
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"\nendstream\nendobj\n");
    for (index, object) in objects.iter().enumerate() {
        body.extend_from_slice(format!("{} 0 obj\n", index + 5).as_bytes());
        body.extend_from_slice(object);
        body.extend_from_slice(b"\nendobj\n");
    }

    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    let mut rest = body.as_slice();
    while let Some(at) = rest
        .windows(7)
        .position(|window| window == b"endobj\n")
        .map(|at| at + 7)
    {
        offsets.push(out.len());
        out.extend_from_slice(&rest[..at]);
        rest = &rest[at..];
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

/// A stream object's bytes: its dictionary's `entries`, then `data`.
fn stream(entries: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {entries} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A page drawing `image` over its whole `width`×1 area, with `entries` beside `/JPXDecode`.
fn image_page(width: u32, image: &[u8], entries: &str) -> Vec<u8> {
    document(
        width,
        "/XObject << /Im0 5 0 R >>",
        &format!("q {width} 0 0 1 0 0 cm /Im0 Do Q"),
        &[stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height 1 /Filter /JPXDecode \
                 {entries}"
            ),
            image,
        )],
    )
}

/// Renders a page at one pixel per unit: each pixel's RGB, and what could not be drawn.
fn render(bytes: Vec<u8>) -> (Vec<[u8; 3]>, Vec<pdf_model::Unsupported>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let unsupported = interpretation.unsupported.clone();
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    let pixels = (0..raster.width as usize)
        .map(|x| {
            let at = x * 4;
            [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
        })
        .collect();
    (pixels, unsupported)
}

/// Draws a JPX file with no `/ColorSpace`, and fails on anything it could not draw.
fn drawn(width: u32, image: &[u8]) -> Vec<[u8; 3]> {
    let (pixels, unsupported) = render(image_page(width, image, ""));
    assert!(
        unsupported.is_empty(),
        "the image is drawn rather than refused: {unsupported:?}"
    );
    pixels
}

/// Pixels painted by operators on a `width`×1 page, one unit each.
fn painted(width: u32, resources: &str, fills: &[String], objects: &[Vec<u8>]) -> Vec<[u8; 3]> {
    let mut content = String::new();
    for (x, fill) in fills.iter().enumerate() {
        let _ = writeln!(content, "{fill} {x} 0 1 1 re f");
    }
    render(document(width, resources, &content, objects)).0
}

/// Asserts two rows of pixels agree to within one level on every channel.
fn assert_near(got: &[[u8; 3]], want: &[[u8; 3]], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}");
    for (a, b) in got.iter().zip(want) {
        assert!(
            a.iter().zip(b).all(|(x, y)| x.abs_diff(*y) <= 1),
            "{what}: drew {got:?} where {want:?} was expected"
        );
    }
}

/// Whether a pixel is neutral, its three channels within one level of each other.
fn neutral(pixel: [u8; 3]) -> bool {
    pixel[0].abs_diff(pixel[1]) <= 1 && pixel[1].abs_diff(pixel[2]) <= 1
}

/// Fails the test where the sandboxed decoder cannot run, as `jpx_channels.rs` does.
fn sandbox() {
    let confinement = pdf_sandbox::Sandbox::shared().confinement();
    assert!(
        confinement.is_ok(),
        "the sandboxed image decoder is not available: {confinement:?}"
    );
}

/// A v2 `mntr` RGB profile whose components are linear sRGB, as `spot_press.rs` builds it.
fn linear_srgb_profile() -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[12..16].copy_from_slice(b"mntr");
    header[16..20].copy_from_slice(b"RGB ");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");
    let xyz_tag = |column: [f32; 3]| {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"XYZ ");
        tag.extend_from_slice(&[0; 4]);
        for value in column {
            #[expect(clippy::cast_possible_truncation, reason = "a colourant below 1.0")]
            tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
        }
        tag
    };
    let mut curve = Vec::new();
    curve.extend_from_slice(b"curv");
    curve.extend_from_slice(&[0; 4]);
    curve.extend_from_slice(&1u32.to_be_bytes());
    curve.extend_from_slice(&0x0100u16.to_be_bytes());
    curve.extend_from_slice(&[0; 2]);
    let tags: [(&[u8; 4], Vec<u8>); 6] = [
        (b"rXYZ", xyz_tag([0.4361, 0.2225, 0.0139])),
        (b"gXYZ", xyz_tag([0.3851, 0.7169, 0.0971])),
        (b"bXYZ", xyz_tag([0.1431, 0.0606, 0.7141])),
        (b"rTRC", curve.clone()),
        (b"gTRC", curve.clone()),
        (b"bTRC", curve),
    ];
    let mut out = header;
    out.extend_from_slice(&6u32.to_be_bytes());
    let mut offset = 128 + 4 + 6 * 12;
    for (name, tag) in &tags {
        out.extend_from_slice(*name);
        out.extend_from_slice(&u32::try_from(offset).expect("small").to_be_bytes());
        out.extend_from_slice(&u32::try_from(tag.len()).expect("small").to_be_bytes());
        offset += tag.len();
    }
    for (_, tag) in &tags {
        out.extend_from_slice(tag);
    }
    out
}

/// The four pixels of [`THREE`] as colour operands, each sample over 255.
const THREE_AS_OPERANDS: [&str; 4] = [
    "0.5882353 0.5019608 0.5019608",
    "0.5882353 0.5882353 0.5882353",
    "1 0.5019608 0.3764706",
    "0 0.5019608 0.3764706",
];

/// Each of [`THREE_AS_OPERANDS`] with `operator` after it.
fn three_filled(prefix: &str, operator: &str) -> Vec<String> {
    THREE_AS_OPERANDS
        .iter()
        .map(|operands| format!("{prefix}{operands} {operator}"))
        .collect()
}

/// sRGB (16), part 1 Table I-10: the samples are the colour.
#[test]
fn srgb_is_drawn_as_its_samples() {
    sandbox();
    let pixels = drawn(4, &jpx(THREE, (4, 3), &[enumerated(0, 16, &[])]));
    assert_eq!(
        pixels,
        vec![
            [150, 128, 128],
            [150, 150, 150],
            [255, 128, 96],
            [0, 128, 96]
        ]
    );
}

/// sRGB-grey (17), part 1 Table I-10's greyscale: one component, drawn as the grey it states.
#[test]
fn srgb_grey_is_drawn_as_its_samples() {
    sandbox();
    let pixels = drawn(4, &jpx(ONE, (4, 1), &[enumerated(0, 17, &[])]));
    assert_eq!(pixels, vec![[100, 100, 100]; 4]);
}

/// CMYK (12), which §7.4.9 adds to the baseline for PDF: the four samples are the subtractive
/// colour the `k` operator paints with the same four values.
#[test]
fn cmyk_is_the_colour_its_four_samples_state() {
    sandbox();
    let pixels = drawn(2, &jpx(FOUR, (2, 4), &[enumerated(0, 12, &[])]));
    let expected = painted(
        2,
        "",
        &[
            "0.7843137 0.3921569 0.1960784 0.0980392 k".to_owned(),
            "0.8235294 0.4313726 0.2352941 0.1372549 k".to_owned(),
        ],
        &[],
    );
    assert_near(&pixels, &expected, "CMYK against the k operator");
}

/// sYCC (18): a colour of no chroma — both chroma samples at the offset — is neutral, and its
/// grey is its luma. Read as RGB, the same samples are not neutral at all.
#[test]
fn sycc_with_no_chroma_is_neutral() {
    sandbox();
    let pixels = drawn(4, &jpx(THREE, (4, 3), &[enumerated(0, 18, &[])]));
    assert_near(&pixels[..1], &[[150, 150, 150]], "sYCC of no chroma");
}

/// ROMM-RGB (21): three equal components are neutral in any RGB space.
#[test]
fn romm_rgb_with_equal_components_is_neutral() {
    sandbox();
    let pixels = drawn(4, &jpx(THREE, (4, 3), &[enumerated(0, 21, &[])]));
    assert!(neutral(pixels[1]), "drew {pixels:?}");
    assert!(
        !neutral(pixels[0]),
        "a chromatic sample stays chromatic: {pixels:?}"
    );
}

/// CIELab (14), with M.11.7.4.1's defaults, with the same values stated in `EP`, and with D50
/// stated as the illuminant: Equation M-18 gives each pixel an L\*a\*b\*, and §8.6.5.4's `Lab`
/// paints that colour.
///
/// With the defaults an eight-bit sample `N` is L\* = 100·N/255, a\* = 170·(N − 128)/255 and
/// b\* = 200·(N − 96)/255, so the fixture's pixels are (58.824, 0, 25.098), (58.824, 14.667,
/// 42.353), (100, 0, 0) and (0, 0, 0): a yellowish grey, a warmer one, white and black.
#[test]
fn cielab_is_drawn_through_lab() {
    sandbox();
    let expected = painted(
        4,
        "/ColorSpace << /L [/Lab << /WhitePoint [0.9642 1 0.8249] /Range [-128 127 -128 127] >>] \
         >>",
        &[
            "/L cs 58.823529 0 25.098039 sc".to_owned(),
            "/L cs 58.823529 14.666667 42.352941 sc".to_owned(),
            "/L cs 100 0 0 sc".to_owned(),
            "/L cs 0 0 0 sc".to_owned(),
        ],
        &[],
    );
    let stated = lab_parameters();
    let mut with_illuminant = stated.clone();
    with_illuminant.extend_from_slice(&ILLUMINANT_D50);
    for parameters in [&[][..], &stated, &with_illuminant] {
        let pixels = drawn(4, &jpx(THREE, (4, 3), &[enumerated(0, 14, parameters)]));
        assert_near(&pixels, &expected, "CIELab against the Lab space");
    }
}

/// CIELab (14) under CIE Illuminant D65 is drawn as the `Lab` space §8.6.5.4's own EXAMPLE
/// writes for that white point, `[0.9505 1.00 1.0890]` (ADR 1713) — and that is a different
/// picture from the same samples read under D50, by more than a level on every chromatic pixel,
/// which is why the illuminant may not be dropped.
#[test]
fn cielab_under_d65_is_drawn_through_lab_with_that_white_point() {
    sandbox();
    let fills = [
        "/L cs 58.823529 0 25.098039 sc".to_owned(),
        "/L cs 58.823529 14.666667 42.352941 sc".to_owned(),
        "/L cs 100 0 0 sc".to_owned(),
        "/L cs 0 0 0 sc".to_owned(),
    ];
    let under = |white: &str| {
        painted(
            4,
            &format!(
                "/ColorSpace << /L [/Lab << /WhitePoint [{white}] /Range [-128 127 -128 127] >>] \
                 >>"
            ),
            &fills,
            &[],
        )
    };
    let d65 = under("0.9505 1.00 1.0890");
    let d50 = under("0.9642 1 0.8249");
    let mut parameters = lab_parameters();
    parameters.extend_from_slice(&ILLUMINANT_D65);
    let pixels = drawn(4, &jpx(THREE, (4, 3), &[enumerated(0, 14, &parameters)]));
    assert_near(&pixels, &d65, "CIELab under D65 against the D65 Lab space");
    for (index, (one, other)) in pixels.iter().zip(&d50).enumerate().take(2) {
        assert!(
            one.iter().zip(other).any(|(x, y)| x.abs_diff(*y) > 1),
            "pixel {index} under D65 is not its D50 reading: {pixels:?} against {d50:?}"
        );
    }
}

/// CIELab under an illuminant stated as a colour temperature alone, M.11.7.4.1's own 7500 K, is
/// drawn under the white point of the Planckian radiator at that temperature, which is what CIE S
/// 017's vocabulary calls a colour temperature (ADR 1713).
#[test]
fn cielab_under_a_colour_temperature_is_drawn_under_the_radiators_white_point() {
    sandbox();
    let expected = painted(
        4,
        &format!(
            "/ColorSpace << /L [/Lab << /WhitePoint [{RADIATOR_AT_7500}] /Range [-128 127 -128 \
             127] >>] >>"
        ),
        &[
            "/L cs 58.823529 0 25.098039 sc".to_owned(),
            "/L cs 58.823529 14.666667 42.352941 sc".to_owned(),
            "/L cs 100 0 0 sc".to_owned(),
            "/L cs 0 0 0 sc".to_owned(),
        ],
        &[],
    );
    let mut parameters = lab_parameters();
    parameters.extend_from_slice(&COLOUR_TEMPERATURE_7500);
    let pixels = drawn(4, &jpx(THREE, (4, 3), &[enumerated(0, 14, &parameters)]));
    assert_near(
        &pixels,
        &expected,
        "CIELab at 7500 K against its radiator's Lab space",
    );
}

/// The codes this tree does not draw as their definitions state take §7.4.9's fallback: three
/// ordinary channels are `DeviceRGB`, and the image is drawn rather than refused.
///
/// CIEJab (19), e-sRGB (20) and e-sYCC (24) are baseline and defined in texts not held (ADR
/// 1383); YCbCr(2) (3) is not baseline at all; a CIELab whose `IL` is a code neither T.801 nor T.4
/// defines names no white point (ADR 1713).
#[test]
fn a_space_not_drawn_falls_back_to_the_device_space_of_its_channel_count() {
    sandbox();
    let mut undefined = lab_parameters();
    undefined.extend_from_slice(&[1, 2, 3, 4]);
    let expected = painted(4, "", &three_filled("", "rg"), &[]);
    for colour in [
        enumerated(0, 19, &[]),
        enumerated(0, 20, &[]),
        enumerated(0, 24, &[]),
        enumerated(0, 3, &[]),
        enumerated(0, 14, &undefined),
    ] {
        let pixels = drawn(4, &jpx(THREE, (4, 3), &[colour]));
        assert_near(&pixels, &expected, "the DeviceRGB fallback");
    }
}

/// The Any ICC method (3) is drawn through its profile, as §8.6.5.5's `ICCBased` draws it.
#[test]
fn the_any_icc_method_is_drawn_through_its_profile() {
    sandbox();
    let profile = linear_srgb_profile();
    let pixels = drawn(4, &jpx(THREE, (4, 3), &[any_icc(0, &profile)]));
    let expected = painted(
        4,
        "/ColorSpace << /P [/ICCBased 5 0 R] >>",
        &three_filled("/P cs ", "sc"),
        &[stream("/N 3", &profile)],
    );
    assert_near(&pixels, &expected, "the Any ICC method against ICCBased");
    assert!(
        !neutral(pixels[0]) && pixels[1] != [150, 150, 150],
        "the profile, not the samples, decided: {pixels:?}"
    );
}

/// "[T]he next lower colour space, in terms of precedence and approximation value, shall be
/// used": a first specification this tree does not draw gives way to the second.
#[test]
fn a_specification_not_drawn_gives_way_to_the_next() {
    sandbox();
    let profile = linear_srgb_profile();
    let pixels = drawn(
        4,
        &jpx(
            THREE,
            (4, 3),
            &[enumerated(0, 24, &[]), any_icc(0, &profile)],
        ),
    );
    let through_the_profile = drawn(4, &jpx(THREE, (4, 3), &[any_icc(0, &profile)]));
    assert_eq!(pixels, through_the_profile);
}

/// Precedence ranks two specifications both drawn: sYCC at precedence 1 over sRGB at 0, so the
/// first pixel, which is neutral in sYCC and pink in sRGB, is neutral.
#[test]
fn the_higher_precedence_is_used() {
    sandbox();
    let pixels = drawn(
        4,
        &jpx(
            THREE,
            (4, 3),
            &[enumerated(0, 16, &[]), enumerated(1, 18, &[])],
        ),
    );
    assert!(neutral(pixels[0]), "drew {pixels:?}");
}

/// A stated `/ColorSpace` sets every specification in the data aside, the codec's conversions
/// included: sYCC samples under `/DeviceRGB` are the RGB their numbers are, and a space the
/// codec would refuse is no reason to refuse an image whose dictionary overrides it.
#[test]
fn a_stated_colour_space_sets_the_data_s_specifications_aside() {
    sandbox();
    for space in [18, 19] {
        let (pixels, unsupported) = render(image_page(
            4,
            &jpx(THREE, (4, 3), &[enumerated(0, space, &[])]),
            "/ColorSpace /DeviceRGB",
        ));
        assert!(unsupported.is_empty(), "{unsupported:?}");
        assert_eq!(
            pixels,
            vec![
                [150, 128, 128],
                [150, 150, 150],
                [255, 128, 96],
                [0, 128, 96]
            ]
        );
    }
}
