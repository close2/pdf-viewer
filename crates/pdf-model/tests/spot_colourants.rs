//! ISO 32000-2 §10.8.3's step a): which spot colourants a page names, read before its first mark.
//!
//! `pdf_model::colourants::spot_colourants` is the enumeration; these hold it to §8.6.6.4's and
//! §8.6.6.5's sentences about which names are colourants of their own, and to the objects a
//! page's marks can reach a colour space through (ADR 1281). The examples §8.6.6.4 and §8.6.6.5
//! print are fixtures here too, with only their elided tint transforms written out.
#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]
#![expect(
    clippy::doc_markdown,
    reason = "the tests quote §8.6.6.5 and §11.7.3 verbatim, and a quotation may not gain backticks"
)]

use std::fmt::Write as _;

use pdf_model::colourants::{SpotColourants, spot_colourants};
use pdf_syntax::Document;

/// A tint transform any `Separation` or `DeviceN` here can name: §7.10.3's exponential function
/// into four components. What it computes does not matter to the enumeration.
const TINT: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 1 0] /N 1 >>";

/// A one-page file: `page` is extra page-dictionary entries, `resources` the resource dictionary's
/// body, and `extra` further objects numbered from 5.
fn file(page: &str, resources: &str, content: &str, extra: &[&str]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << {resources} >> \
             /Contents 4 0 R {page} >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len().saturating_add(1)
        ),
    ];
    objects.extend(extra.iter().map(|object| (*object).to_owned()));
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The colourants page one of `bytes` names, and the names as text.
fn enumerated(bytes: Vec<u8>) -> (SpotColourants, Vec<String>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let spots = spot_colourants(&document, &page);
    let names = spots
        .names()
        .iter()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        .collect();
    (spots, names)
}

/// The names page one of a file with these colour-space resources names.
fn named_by(colour_spaces: &str) -> Vec<String> {
    enumerated(file(
        "",
        &format!("/ColorSpace << {colour_spaces} >>"),
        "",
        &[],
    ))
    .1
}

/// §8.6.6.5's `DeviceN` "may contain an arbitrary number of colour components", and five spot
/// colourants are two planes of three channels: `ceil(5 / 3)`, beside the two process planes.
#[test]
fn a_devicen_of_five_colourants_takes_two_planes() {
    let (spots, names) = enumerated(file(
        "",
        &format!(
            "/ColorSpace << /Five [/DeviceN [/Orange /Green /Violet /Gold /Silver] /DeviceCMYK \
             {TINT}] >>"
        ),
        "",
        &[],
    ));
    assert_eq!(names, ["Orange", "Green", "Violet", "Gold", "Silver"]);
    assert_eq!(spots.spot_planes(), 2, "ceil(5 / 3)");
    assert_eq!(
        spots.plane_count(),
        4,
        "two process planes and two spot ones"
    );
}

/// §8.6.6.4: "The special colourant name All shall refer collectively to all colourants
/// available on an output device, including those for the standard process colourants." It is
/// every plane rather than one, so it takes none; and `None` "shall not produce any visible
/// output", so it needs none.
#[test]
fn a_separation_named_all_or_none_is_no_colourant_of_its_own() {
    let names = named_by(&format!(
        "/Registration [/Separation /All /DeviceCMYK {TINT}] \
         /Nothing [/Separation /None /DeviceCMYK {TINT}]"
    ));
    assert!(names.is_empty(), "neither is a spot colourant: {names:?}");
    let (spots, _) = enumerated(file("", "", "", &[]));
    assert_eq!(spots.plane_count(), 2, "a page of process colours alone");
}

/// §8.6.6.5 forbids `All` in a `DeviceN` — "The special name All , used by Separation colour
/// spaces, shall not be used." — and passes `None` components to the tint transform rather than to
/// a colourant, so neither names one.
#[test]
fn all_and_none_in_a_devicen_name_no_colourant() {
    let names = named_by(&format!(
        "/Mixed [/DeviceN [/All /Orange /None /None] /DeviceCMYK {TINT}]"
    ));
    assert_eq!(names, ["Orange"]);
}

/// §8.6.6.4's four reserved names are the process planes' own, "reserved to name the process
/// colourants of a CMYK device", and so is §10.8.2's worked example, drawn with a `Cyan` and a
/// `Yellow` `Separation` space: a page naming only those is separated into the process planes.
#[test]
fn the_four_process_names_fold_into_the_process_planes() {
    let (spots, names) = enumerated(file(
        "",
        &format!(
            "/ColorSpace << /C [/Separation /Cyan /DeviceCMYK {TINT}] \
             /Y [/Separation /Yellow /DeviceCMYK {TINT}] \
             /MK [/DeviceN [/Magenta /Black /Orange] /DeviceCMYK {TINT}] >>"
        ),
        "",
        &[],
    ));
    assert_eq!(
        names,
        ["Orange"],
        "only the spot colourant is one of its own"
    );
    assert_eq!(spots.spot_planes(), 1);
}

/// §8.6.6.4 EXAMPLE 2, with its tint transformation function object 12 as printed.
#[test]
fn the_logo_green_example_names_logo_green() {
    let names = enumerated(file(
        "",
        "/ColorSpace << /CS0 5 0 R >>",
        "",
        &[
            "[/Separation /LogoGreen /DeviceCMYK 6 0 R]",
            "<< /FunctionType 4 /Domain [0.0 1.0] /Range [0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0] \
             /Length 62 >>\nstream\n{dup 0.84 mul exch 0.00 exch dup 0.44 mul exch 0.21 mul}\n\
             endstream",
        ],
    ))
    .1;
    assert_eq!(names, ["LogoGreen"]);
}

/// §8.6.6.5 EXAMPLE 2: a `DeviceN` of `Orange`, `Green` and `None` whose `/Colorants` also defines
/// `PANTONE 131`. That dictionary "may also include additional colourants not used by this colour
/// space", so it is the `names` array that says which colourants the page uses.
#[test]
fn a_colorants_entry_the_names_array_does_not_use_is_not_a_colourant_of_the_page() {
    let names = enumerated(file(
        "",
        "/ColorSpace << /CS0 5 0 R >>",
        "",
        &[
            &format!("[/DeviceN [/Orange /Green /None] /DeviceCMYK {TINT} << /Colorants 6 0 R >>]"),
            &format!(
                "<< /Orange [/Separation /Orange /DeviceCMYK {TINT}] \
                 /Green [/Separation /Green /DeviceCMYK {TINT}] \
                 /PANTONE#20131 [/Separation /PANTONE#20131 /DeviceCMYK {TINT}] >>"
            ),
        ],
    ))
    .1;
    assert_eq!(names, ["Orange", "Green"]);
}

/// §8.6.6.5 EXAMPLES 3 and 4: an `NChannel` space's process components are Table 71's, and "Any
/// component not specified in the process dictionary shall be considered to be a spot colourant."
/// The RGB example's process names are not the reserved four, and they fold all the same.
#[test]
fn an_nchannel_spaces_process_components_are_not_spot_colourants() {
    let names = named_by(&format!(
        "/Cmyk [/DeviceN [/Magenta /Spot1 /Yellow /Spot2] /DeviceCMYK {TINT} \
           << /Subtype /NChannel /Process << /ColorSpace /DeviceCMYK \
              /Components [/Cyan /Magenta /Yellow /Black] >> \
              /Colorants << /Spot1 [/Separation /Spot1 /DeviceCMYK {TINT}] \
                            /Spot2 [/Separation /Spot2 /DeviceCMYK {TINT}] >> >>] \
         /Rgb [/DeviceN [/ProcessRed /ProcessGreen /ProcessBlue /Red] /DeviceCMYK {TINT} \
           << /Subtype /NChannel /Process << /ColorSpace /DeviceRGB \
              /Components [/ProcessRed /ProcessGreen /ProcessBlue] >> \
              /Colorants << /Red [/Separation /Red /DeviceCMYK {TINT}] >> >>]"
    ));
    assert_eq!(names, ["Spot1", "Spot2", "Red"]);
}

/// A process dictionary means nothing without the `NChannel` subtype — "A value of DeviceN for the
/// Subtype entry, or no value, shall mean that only the previous features shall be supported" —
/// so a plain `DeviceN`'s arbitrary names stay spot colourants whatever it states.
#[test]
fn a_process_dictionary_without_the_nchannel_subtype_folds_nothing() {
    let names = named_by(&format!(
        "/Plain [/DeviceN [/ProcessRed /Red] /DeviceCMYK {TINT} \
           << /Process << /ColorSpace /DeviceRGB /Components [/ProcessRed /G /B] >> >>]"
    ));
    assert_eq!(names, ["ProcessRed", "Red"]);
}

/// §8.6.6.5's `Indexed` examples: a duotone of the reserved `Cyan` and `Black` names nothing, one
/// of `Black` and `Gold` names `Gold`, and the quadtone image's base space names its three PANTONE
/// inks — reached through an image `XObject`'s `/ColorSpace`, not through the colour-space
/// resources.
#[test]
fn an_indexed_base_is_read_through_an_image() {
    let (spots, names) = enumerated(file(
        "",
        "/ColorSpace << /Duo [/Indexed [/DeviceN [/Cyan /Black] /DeviceCMYK 7 0 R] 0 <0000>] \
                        /Gold [/Indexed [/DeviceN [/Black /Gold] /DeviceCMYK 7 0 R] 0 <0000>] >> \
         /XObject << /Im1 5 0 R >>",
        "",
        &[
            "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace [/Indexed 6 0 R 0 \
             <00000000000000>] /BitsPerComponent 8 /Length 1 >>\nstream\n\0\nendstream",
            "[/DeviceN [/Black /PANTONE#20216#20CVC /PANTONE#20409#20CVC /PANTONE#202985#20CVC \
             /None /None /None] /DeviceCMYK 7 0 R]",
            TINT,
        ],
    ));
    assert_eq!(
        names,
        [
            "Gold",
            "PANTONE 216 CVC",
            "PANTONE 409 CVC",
            "PANTONE 2985 CVC"
        ],
        "the resource dictionary's keys first, then the image's space"
    );
    assert_eq!(spots.spot_planes(), 2);
}

/// Every road a colour space reaches a mark by: a form's resources, a tiling pattern's, a shading
/// pattern's shading, a shading resource, a `Pattern` space's underlying space, a Type 3 font's
/// resources and an annotation's normal appearance (§12.5.5) — and a colourant named on two of
/// them is one colourant.
#[test]
fn every_road_to_a_colour_space_is_walked_and_a_name_counts_once() {
    let separation = |name: &str| format!("[/Separation /{name} /DeviceCMYK {TINT}]");
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] \
         /Resources << /ColorSpace << /F {} /Again {} >> >> /Length 1 >>\nstream\n\nendstream",
        separation("InForm"),
        separation("Underlying")
    );
    let tiling = format!(
        "<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 1 1] /XStep 1 \
         /YStep 1 /Resources << /ColorSpace << /T {} >> >> /Length 1 >>\nstream\n\nendstream",
        separation("InTiling")
    );
    let shading_pattern = format!(
        "<< /Type /Pattern /PatternType 2 /Shading << /ShadingType 2 /ColorSpace {} \
         /Coords [0 0 1 0] /Function {TINT} >> >>",
        separation("InShadingPattern")
    );
    let shading = format!(
        "<< /ShadingType 2 /ColorSpace {} /Coords [0 0 1 0] /Function {TINT} >>",
        separation("InShading")
    );
    let type3 = format!(
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] /FontMatrix [1 0 0 1 0 0] \
         /CharProcs << >> /Encoding << /Differences [] >> /FirstChar 0 /LastChar 0 /Widths [0] \
         /Resources << /ColorSpace << /G {} >> >> >>",
        separation("InGlyph")
    );
    let appearance = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] \
         /Resources << /ColorSpace << /A {} >> >> /Length 1 >>\nstream\n\nendstream",
        separation("InAppearance")
    );
    let names = enumerated(file(
        "/Annots [<< /Type /Annot /Subtype /Square /Rect [0 0 1 1] /AP << /N 10 0 R >> >>]",
        &format!(
            "/ColorSpace << /P [/Pattern {}] >> /XObject << /Fm 5 0 R >> \
             /Pattern << /Tile 6 0 R /Smooth 7 0 R >> /Shading << /Sh 8 0 R >> \
             /Font << /T3 9 0 R >>",
            separation("Underlying")
        ),
        "",
        &[
            &form,
            &tiling,
            &shading_pattern,
            &shading,
            &type3,
            &appearance,
        ],
    ))
    .1;
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        [
            "InAppearance",
            "InForm",
            "InGlyph",
            "InShading",
            "InShadingPattern",
            "InTiling",
            "Underlying"
        ],
        "each road once, and `Underlying` once for two spaces naming it: {names:?}"
    );
}

/// §11.7.3: "In particular, spot colours shall not be available in a transparency group XObject
/// that is used to define a soft mask; the alternate colour space shall always be substituted in
/// that case." A mask group's colourants need no plane, so `/ExtGState` is not walked.
#[test]
fn a_soft_mask_groups_colourants_take_no_plane() {
    let names = enumerated(file(
        "",
        "/ExtGState << /Masked << /SMask << /Type /Mask /S /Luminosity /G 5 0 R >> >> >>",
        "",
        &[&format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Group << /S /Transparency >> \
             /Resources << /ColorSpace << /M [/Separation /InMask /DeviceCMYK {TINT}] >> >> \
             /Length 1 >>\nstream\n\nendstream"
        )],
    ))
    .1;
    assert!(names.is_empty(), "{names:?}");
}

/// A form whose resources name the form itself, and a pattern space whose underlying space is
/// the resource naming it: each object is entered once per role, so the walk ends.
#[test]
fn a_cycle_through_the_resources_ends() {
    let names = enumerated(file(
        "",
        "/XObject << /Fm 5 0 R >> /ColorSpace << /Loop 6 0 R >>",
        "",
        &[
            &format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Resources \
                 << /XObject << /Self 5 0 R >> /ColorSpace << /S [/Separation /Once /DeviceCMYK \
                 {TINT}] >> >> /Length 1 >>\nstream\n\nendstream"
            ),
            "[/Pattern 6 0 R]",
        ],
    ))
    .1;
    assert_eq!(names, ["Once"]);
}
