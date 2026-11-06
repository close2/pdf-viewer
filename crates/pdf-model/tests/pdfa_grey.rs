//! `DeviceGray` in a PDF/A file whose PDF/A output intent is a CMYK press.
//!
//! ISO 19005-2 and ISO 19005-4, section 6.2.4.3 (paraphrased, as `doc/third-party-data.md`
//! asks of ISO 19005): where a PDF/A file sets no device-independent `DefaultGray` and its PDF/A
//! output intent carries a CMYK destination profile, a conforming reader converts a `DeviceGray`
//! colour to CMYK by ISO 32000's classic method and renders it with that profile as the source.
//! The classic method is ISO 32000-2 §10.4.2.3:
//!
//! > Nominally, a gray level is the complement of the black component of CMYK . Therefore, the
//! > CMYK colour value equivalent to a specific gray level shall be
//!
//! cyan, magenta and yellow `0.0` and black `1.0 − gray`. So every expectation here is a pair of
//! renderings of one file, and the clause is what says they agree: a grey `g` against the same
//! mark painted `0 0 0 (1 − g) k`, which the intent's profile already draws (ADR 2712). The rule
//! binds only where its three conditions hold; each test that removes one expects the grey this
//! reader draws without an intent, which is ISO 32000's own implementation-dependent answer.
//!
//! The intent is the assumed inks (`pdf_colour::colour::assumed_inks_profile`, ADR 2700), whose
//! black ink draws a visibly warm dark grey rather than `(0, 0, 0)`, so a grey routed through it
//! and one drawn as this reader's own grey are far apart.
//!
//! One construction here is ISO 32000's alone and binds in any file: a grey inside a
//! `/DeviceCMYK` luminosity group under a CMYK intent, which §11.5.3's device branch reads the
//! same way (ADR 2734), so its test sits beside the PDF/A one it mirrors.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, and \
              the fixtures are small enough that no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

/// The XMP identification a PDF/A-2 file states, as object 7.
const PDFA_METADATA: &str = "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\
<pdfaid:part>2</pdfaid:part><pdfaid:conformance>B</pdfaid:conformance>\
</rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

/// What one fixture states about itself: the three conditions of section 6.2.4.3, each one a
/// switch.
#[derive(Clone, Copy)]
struct File {
    /// The catalog's `/Metadata` states `pdfaid:part`.
    identifies: bool,
    /// The output intent's `/S`: `GTS_PDFA1`, the PDF/A one, or something else; `None` for no
    /// intent at all.
    intent: Option<&'static str>,
}

const PDFA: File = File {
    identifies: true,
    intent: Some("GTS_PDFA1"),
};

const NO_INTENT: File = File {
    identifies: false,
    intent: None,
};

/// Builds a one-page 20 × 20 PDF: `extra` objects from 8 on, `resources` and `content` the
/// page's.
fn pdf(file: File, extra: &str, resources: &str, content: &str) -> Vec<u8> {
    let mut hex = String::new();
    for byte in pdf_colour::colour::assumed_inks_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    let mut catalog = String::new();
    if file.intent.is_some() {
        catalog.push_str("/OutputIntents [5 0 R] ");
    }
    if file.identifies {
        catalog.push_str("/Metadata 7 0 R ");
    }
    let subtype = file.intent.unwrap_or("GTS_PDFA1");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog}>>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /OutputIntent /S /{subtype} /OutputConditionIdentifier (Custom) \
         /DestOutputProfile 6 0 R >>\nendobj\n\
         6 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream\n\
         endobj\n\
         7 0 obj\n<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{PDFA_METADATA}\n\
         endstream\nendobj\n\
         {extra}",
        content.len().saturating_add(1),
        hex.len().saturating_add(1),
        PDFA_METADATA.len().saturating_add(1),
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        if object.trim().is_empty() {
            continue;
        }
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

/// Renders a fixture at 72 dpi and returns the colour at its centre.
fn centre(bytes: Vec<u8>) -> (u8, u8, u8) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let at = ((10 * raster.width) + 10) as usize * 4;
    (raster.data[at], raster.data[at + 1], raster.data[at + 2])
}

/// The grey level the fixtures paint, and the black ink §10.4.2.3 makes of it.
const GREY: &str = "0.25";
const BLACK: &str = "0.75";

/// Fills the page in a grey, by each route a `DeviceGray` colour can take.
fn grey_routes() -> [(&'static str, String, String); 4] {
    [
        (
            "the g operator",
            String::new(),
            format!("{GREY} g 0 0 20 20 re f"),
        ),
        (
            "a stroke",
            String::new(),
            format!("{GREY} G 20 w 0 10 m 20 10 l S"),
        ),
        (
            "a /DeviceGray cs selection",
            String::new(),
            format!("/DeviceGray cs {GREY} sc 0 0 20 20 re f"),
        ),
        (
            "an inline image",
            String::new(),
            // One sample of 0x40, which §8.9.5.2's default decode makes 64 ÷ 255.
            "q 20 0 0 20 0 0 cm BI /W 1 /H 1 /CS /G /BPC 8 /F /AHx ID 40> EI Q".to_owned(),
        ),
    ]
}

/// The same mark in the black ink alone, `0 0 0 (1 − g) k` (for the inline image, the sample
/// value 64 ÷ 255 rather than a quarter).
fn black_of(route: &str) -> String {
    if route == "an inline image" {
        let black = 1.0 - 64.0 / 255.0;
        format!("0 0 0 {black} k 0 0 20 20 re f")
    } else {
        format!("0 0 0 {BLACK} k 0 0 20 20 re f")
    }
}

/// Two colours within one level of each other in every channel: the press converts a grey and
/// its ink by one table, but a stroke's or an image's coverage is not a fill's.
fn within_a_level(left: (u8, u8, u8), right: (u8, u8, u8)) -> bool {
    left.0.abs_diff(right.0) <= 1 && left.1.abs_diff(right.1) <= 1 && left.2.abs_diff(right.2) <= 1
}

/// In a PDF/A file under a CMYK intent, a grey is the intent's black ink alone, by every route.
#[test]
fn a_pdfa_files_grey_is_the_intents_black_ink_alone() {
    for (route, resources, content) in grey_routes() {
        let grey = centre(pdf(PDFA, "", &resources, &content));
        let ink = centre(pdf(PDFA, "", &resources, &black_of(route)));
        let device = centre(pdf(NO_INTENT, "", &resources, &content));
        assert!(
            within_a_level(grey, ink),
            "{route}: a grey of {GREY} draws as the press's 0 0 0 {BLACK}, got {grey:?} against \
             {ink:?}"
        );
        assert!(
            !within_a_level(grey, device),
            "{route}: and so not as this reader's own grey {device:?}"
        );
    }
}

/// A shading in `DeviceGray` reaches the rule as its ramp's colours do.
#[test]
fn a_pdfa_files_grey_shading_is_the_intents_black_ink_alone() {
    let shading = "8 0 obj\n<< /ShadingType 2 /ColorSpace /DeviceGray /Coords [0 0 20 0] \
                   /Function << /FunctionType 2 /Domain [0 1] /C0 [0.25] /C1 [0.25] /N 1 >> >>\n\
                   endobj\n";
    let ink = "8 0 obj\n<< /ShadingType 2 /ColorSpace /DeviceCMYK /Coords [0 0 20 0] \
               /Function << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0.75] /C1 [0 0 0 0.75] \
               /N 1 >> >>\nendobj\n";
    let resources = "/Shading << /Sh 8 0 R >>";
    let grey = centre(pdf(PDFA, shading, resources, "/Sh sh"));
    let black = centre(pdf(PDFA, ink, resources, "/Sh sh"));
    assert!(
        within_a_level(grey, black),
        "a grey shading draws as its black ink, got {grey:?} against {black:?}"
    );
}

/// A file that does not identify as PDF/A is read by ISO 32000 alone, and its grey is this
/// reader's, intent or no intent — so is a PDF/A file whose intent is not the PDF/A one.
#[test]
fn a_grey_is_unchanged_where_the_file_is_not_pdfa_or_the_intent_is_not_its_pdfa_intent() {
    let content = format!("{GREY} g 0 0 20 20 re f");
    let device = centre(pdf(NO_INTENT, "", "", &content));
    for (why, file) in [
        (
            "no pdfaid:part",
            File {
                identifies: false,
                intent: Some("GTS_PDFA1"),
            },
        ),
        (
            "a PDF/X intent",
            File {
                identifies: true,
                intent: Some("GTS_PDFX"),
            },
        ),
    ] {
        assert_eq!(
            centre(pdf(file, "", "", &content)),
            device,
            "{why}: the grey is this reader's own"
        );
    }
}

/// A `/DefaultGray` the resources state outranks the rule, which is the clause's own condition
/// and §8.6.5.6's: the grey is drawn through the default, as it is in a file with no intent.
#[test]
fn a_default_grey_outranks_the_rule() {
    let resources = "/ColorSpace << /DefaultGray [/CalGray << /WhitePoint [0.9505 1 1.089] \
                     /Gamma 2.2 >>] >>";
    let content = format!("{GREY} g 0 0 20 20 re f");
    assert_eq!(
        centre(pdf(PDFA, "", resources, &content)),
        centre(pdf(NO_INTENT, "", resources, &content)),
        "the default decides the grey, intent or not"
    );
}

/// A soft mask built of a grey masks by that grey: §11.5.3's luminosity of `0 0 0 (1 − g)` is
/// `g`, which is what the mask group's grey now is.
///
/// Both of the shapes ADR 2708 found: a grey painted into a `/DeviceCMYK` luminosity group,
/// and a `/DeviceGray` luminosity group holding an isolated `/DeviceCMYK` group of black.
#[test]
fn a_grey_inside_a_soft_mask_masks_by_its_grey() {
    let masked = |cs: &str, mask: &str, extra: &str| masked(PDFA, cs, mask, extra);
    let grey = centre(masked(
        "DeviceCMYK",
        &format!("{GREY} g 0 0 20 20 re f"),
        "",
    ));
    let ink = centre(masked(
        "DeviceCMYK",
        &format!("0 0 0 {BLACK} k 0 0 20 20 re f"),
        "",
    ));
    assert!(
        within_a_level(grey, ink),
        "a grey in a /DeviceCMYK mask group masks as its black ink, got {grey:?} against {ink:?}"
    );

    // A `/DeviceGray` mask group whose only content is an isolated `/DeviceCMYK` group of full
    // black ink: luminosity 0, so the page's black is masked away to the paper.
    let black_group = "10 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] \
                       /Group << /S /Transparency /CS /DeviceCMYK /I true >> /Length 28 >>\n\
                       stream\n0 0 0 1 k 0 0 20 20 re f\nendstream\nendobj\n";
    let paper = centre(masked("DeviceGray", "/B Do", black_group));
    assert!(
        within_a_level(paper, (255, 255, 255)),
        "a /DeviceGray mask of black ink masks everything, got {paper:?}"
    );
}

/// A page painted in the press's black through a `/Luminosity` mask whose group, of space `cs`,
/// paints `mask`; `extra` holds objects from 10 on, which the group names as `/B`.
fn masked(file: File, cs: &str, mask: &str, extra: &str) -> Vec<u8> {
    let objects = format!(
        "8 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 9 0 R >> >>\n\
         endobj\n\
         9 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] \
         /Group << /S /Transparency /CS /{cs} >> /Resources << /XObject << /B 10 0 R >> >> \
         /Length {} >>\nstream\n{mask}\nendstream\nendobj\n{extra}",
        mask.len().saturating_add(1)
    );
    pdf(
        file,
        &objects,
        "/ExtGState << /M 8 0 R >>",
        "/M gs 0 0 0 1 k 0 0 20 20 re f",
    )
}

/// Outside PDF/A as well, under any CMYK output intent, a grey painted into a `/Luminosity`
/// group stating `/CS /DeviceCMYK` masks by its grey.
///
/// ISO 32000-2 §11.5.3's device branch, which this reader takes for every mask group (A23):
///
/// > For device colour spaces, convert the colour to DeviceGray by implementation-defined
/// > means and use the resulting gray value as the luminosity, with no compensation for gamma
/// > or other colour calibration.
///
/// So the grey meets the group's `DeviceCMYK` as two device spaces meet, by §10.4.2.3 — black
/// `1 − g` alone — and is not separated colorimetrically by the intent's profile into four inks
/// that EXAMPLE 2 then weighs darker (ADR 2734). The expectation is the same mask painted in that
/// black ink, by a fill and by an image, which is how a layout program's feather draws its ramp.
/// The page's own greys outside a mask stay this reader's
/// (`a_grey_is_unchanged_where_the_file_is_not_pdfa_or_the_intent_is_not_its_pdfa_intent`).
#[test]
fn a_grey_inside_a_cmyk_mask_group_masks_by_its_grey_under_any_cmyk_intent() {
    let pdfx = File {
        identifies: false,
        intent: Some("GTS_PDFX"),
    };
    let image_black = 1.0 - 64.0 / 255.0;
    for (route, grey, ink) in [
        (
            "a fill",
            format!("{GREY} g 0 0 20 20 re f"),
            format!("0 0 0 {BLACK} k 0 0 20 20 re f"),
        ),
        (
            "an image",
            "q 20 0 0 20 0 0 cm BI /W 1 /H 1 /CS /G /BPC 8 /F /AHx ID 40> EI Q".to_owned(),
            format!("0 0 0 {image_black} k 0 0 20 20 re f"),
        ),
    ] {
        let grey = centre(masked(pdfx, "DeviceCMYK", &grey, ""));
        let ink = centre(masked(pdfx, "DeviceCMYK", &ink, ""));
        assert!(
            within_a_level(grey, ink),
            "{route}: a grey in a /DeviceCMYK mask group under a PDF/X intent masks as its black \
             ink, got {grey:?} against {ink:?}"
        );
    }
}
