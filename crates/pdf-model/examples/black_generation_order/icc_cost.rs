//! Whole-interpretation costs for ordinary ICC solids, generic ICC tints and mixed use, and
//! for the `Separation`, `DeviceN`, calibrated and Lab solids ADR 2248 retains and the
//! `DeviceGray`, `DeviceCMYK` and `Indexed` ones ADR 2264 does, and, under an output intent,
//! the `DeviceCMYK` family §11.7.2 redefines at the paint (ADR 2276).
//! The generated tables match the independently tested intent fixture; this is no oracle.

use pdf_syntax::Document;
use std::fmt::Write as _;

fn mid_table(mid: [f32; 3]) -> Vec<u8> {
    let white = [0.964_2f32, 1.0, 0.824_9];
    let fixed = |value: f32| ((value * 32768.0) as u16).to_be_bytes();

    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&[1, 3, 3, 0]); // one in, three out, three grid points
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes()); // input table entries
    tag.extend_from_slice(&2u16.to_be_bytes()); // output table entries
    for value in [0u16, 0xFFFF] {
        tag.extend_from_slice(&value.to_be_bytes());
    }
    for point in [
        white,
        mid,
        [white[0] / 10.0, white[1] / 10.0, white[2] / 10.0],
    ] {
        for value in point {
            tag.extend_from_slice(&fixed(value));
        }
    }
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }
    tag
}

/// An ICC profile that answers a mid-tone in three different colours, one per rendering intent.
///
/// ISO 15076-1:2010 gives `A2B0`, `A2B1` and `A2B2` the device-to-connection transform of one
/// rendering intent each (its tag listing, clause 9.2), and §8.6.5.8 says the names PDF uses
/// "have been chosen to correspond to those defined by the International Color Consortium
/// (ICC)". So this profile is three answers to one question, and which one comes back says which
/// intent was in force:
///
/// - `A2B0`, perceptual: a red mid-tone.
/// - `A2B1`, colorimetric: a neutral mid-tone — the one Table 51 makes the initial value.
/// - `A2B2`, saturation: a blue mid-tone.
///
/// No real profile is built this way; that is the point of a fixture (trap 8). A profile whose
/// tables agreed to within a few levels, as real ones tend to, could pass these tests with the
/// selection unimplemented.
fn three_table_profile() -> Vec<u8> {
    let tables: [(&[u8; 4], [f32; 3]); 3] = [
        (b"A2B0", [0.45, 0.25, 0.05]),
        (b"A2B1", [0.25, 0.25, 0.25]),
        (b"A2B2", [0.12, 0.25, 0.75]),
    ];

    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[16..20].copy_from_slice(b"GRAY");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut out = header;
    out.extend_from_slice(&(tables.len() as u32).to_be_bytes());
    let mut offset = 128 + 4 + 12 * tables.len();
    let mut data = Vec::new();
    for (name, mid) in tables {
        let tag = mid_table(mid);
        out.extend_from_slice(name);
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(tag.len() as u32).to_be_bytes());
        offset += tag.len();
        data.extend_from_slice(&tag);
    }
    out.extend_from_slice(&data);
    out
}

fn encoded_stream(dictionary: &str, bytes: &[u8]) -> String {
    let mut hex = String::new();
    for byte in bytes {
        write!(hex, "{byte:02X}").expect("string write");
    }
    format!(
        "<< {dictionary} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream",
        hex.len() + 1
    )
}
fn fixture(content: &str, step: usize) -> Vec<u8> {
    let a = three_table_profile();
    let mut b = a.clone();
    b[132..136].copy_from_slice(b"A2B2");
    b[156..160].copy_from_slice(b"A2B0");
    let cell = format!("0 0 {step} {step} re f");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /IA [/Indexed [/ICCBased 5 0 R] 2 <4080C0>] /IB [/Indexed [/ICCBased 6 0 R] 2 <4080C0>] /IT [/Indexed [/ICCBased 5 0 R] 2 <C08040>] /End [/Indexed [/ICCBased 5 0 R] 0 <FF>] /A [/ICCBased 5 0 R] /B [/ICCBased 6 0 R] /PA [/Pattern [/ICCBased 5 0 R]] /PB [/Pattern [/ICCBased 6 0 R]] {TINT_SPACES}{DEVICE_SPACES}>> /Pattern << /Tile 7 0 R >> /ExtGState << /Off << /UseBlackPtComp /OFF >> /On << /UseBlackPtComp /ON >> >> >> >>"),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()+1),
        encoded_stream("/N 1", &a), encoded_stream("/N 1", &b),
        format!("<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 {step} {step}] /XStep {step} /YStep {step} /Resources << >> /Length {} >>\nstream\n{cell}\nendstream", cell.len()+1),
        encoded_stream("/FunctionType 0 /Domain [0 1] /Range [0 1 0 1 0 1] /Size [256] /BitsPerSample 8", &sampled_ramp()),
        "<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1] /Length 8 >>\nstream\n{ pop }\nendstream".to_owned(),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{object}\nendobj", index + 1).expect("object");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).expect("xref");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("offset");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("trailer");
    out.into_bytes()
}

struct Workload {
    name: &'static str,
    population: &'static str,
    content: String,
    step: usize,
}
fn workloads() -> Vec<Workload> {
    let mut out = Vec::new();
    // One chunk is one primary mark; alternating setters include their real parsing/selection cost.
    for (name, population, prefix, even, odd, step) in [
        (
            "icc-solid-repeat",
            "same-output",
            "/Perceptual ri /A cs .5 scn ",
            "",
            "",
            100,
        ),
        (
            "icc-solid-intent",
            "required-current-state",
            "/A cs .5 scn ",
            "/Perceptual ri ",
            "/Saturation ri ",
            100,
        ),
        (
            "icc-solid-bpc",
            "required-current-state",
            "/RelativeColorimetric ri /A cs .5 scn ",
            "/Off gs ",
            "/On gs ",
            100,
        ),
        (
            "icc-tint-repeat",
            "generic-tint-control",
            "/Perceptual ri /PA cs .5 /Tile scn ",
            "",
            "",
            100,
        ),
        (
            "icc-tint-many",
            "generic-tint-geometry",
            "/Perceptual ri /PA cs .5 /Tile scn ",
            "",
            "",
            10,
        ),
        (
            "icc-tint-values",
            "generic-tint-selection",
            "/Perceptual ri /PA cs ",
            ".25 /Tile scn ",
            ".75 /Tile scn ",
            100,
        ),
        (
            "icc-tint-intent",
            "generic-tint-context",
            "/PA cs .5 /Tile scn ",
            "/Perceptual ri ",
            "/Saturation ri ",
            100,
        ),
        (
            "icc-tint-bpc",
            "generic-tint-context",
            "/RelativeColorimetric ri /PA cs .5 /Tile scn ",
            "/Off gs ",
            "/On gs ",
            100,
        ),
        (
            "rgb-solid-repeat",
            "rgb-control",
            ".2 .4 .6 rg ",
            "",
            "",
            100,
        ),
        (
            "rgb-solid-values",
            "rgb-selection-control",
            "",
            ".2 .4 .6 rg ",
            ".6 .4 .2 rg ",
            100,
        ),
    ] {
        let mut content = format!("/Off gs {prefix}");
        for index in 0..1000 {
            content.push_str(if index % 2 == 0 { even } else { odd });
            content.push_str("0 0 50 100 re f ");
        }
        out.push(Workload {
            name,
            population,
            content,
            step,
        });
    }
    paired_workloads(&mut out);
    indexed_workloads(&mut out);
    tint_and_cie_workloads(&mut out);
    device_workloads(&mut out);
    out
}

/// ADR 2264's `Indexed` bases: `DeviceCMYK`, inline like the device families, and an identity
/// `Separation` into profile A, whose entry is carried with its whole space.
const DEVICE_SPACES: &str = "/IK [/Indexed /DeviceCMYK 1 <00000000 00FF0000>] /IS [/Indexed [/Separation /Spot [/ICCBased 5 0 R] << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>] 1 <4080>] ";

// `g`, `k` and `Indexed` selections, hits and restorations, every one painting on the device
// the colour its selection painted: the baseline converted these at selection and kept no
// source, so these price the source and the memo alone.
fn device_workloads(out: &mut Vec<Workload>) {
    for (name, population, prefix, even, odd) in [
        ("gray-repeat", "same-output-hit", ".6 g ", "", ""),
        (
            "gray-select",
            "same-output-selection",
            "",
            ".25 g ",
            ".75 g ",
        ),
        ("cmyk-repeat", "same-output-hit", "0 1 0 0 k ", "", ""),
        (
            "cmyk-select",
            "same-output-selection",
            "",
            "0 1 0 0 k ",
            "1 0 0 0 k ",
        ),
        (
            "indexed-cmyk-select",
            "same-output-selection",
            "/IK cs ",
            "0 scn ",
            "1 scn ",
        ),
        (
            "indexed-sep-select",
            "same-output-selection",
            "/Perceptual ri /IS cs ",
            "0 scn ",
            "1 scn ",
        ),
    ] {
        let mut content = format!("/Off gs {prefix}");
        for index in 0..1000 {
            content.push_str(if index % 2 == 0 { even } else { odd });
            content.push_str("0 0 50 100 re f ");
        }
        out.push(Workload {
            name,
            population,
            content,
            step: 100,
        });
    }
    for (name, outer, inner) in [
        ("gray-restore", ".6 g ", ".25 g "),
        ("cmyk-restore", "0 1 0 0 k ", "1 0 0 0 k "),
    ] {
        let mut content = format!("/Off gs {outer}");
        for _ in 0..500 {
            write!(content, "q {inner}0 0 50 100 re f Q 0 0 50 100 re f ").expect("restore pair");
        }
        out.push(Workload {
            name,
            population: "same-output-restoration",
            content,
            step: 100,
        });
    }
}

/// The spaces ADR 2248 retains whole: an identity tint into profile A, a `DeviceN` keeping its
/// first tint, a 256-entry sampled tint into `DeviceRGB` (whose table a selection clones), and
/// D50 `CalRGB` and Lab.
const TINT_SPACES: &str = "/S [/Separation /Spot [/ICCBased 5 0 R] << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>] /N [/DeviceN [/Spot /Other] [/ICCBased 5 0 R] 9 0 R] /SS [/Separation /Spot /DeviceRGB 8 0 R] /C [/CalRGB << /WhitePoint [0.9642 1 0.8249] /Matrix [1 0 0 0 1 0 0 0 1] >>] /L [/Lab << /WhitePoint [0.9642 1 0.8249] /Range [-100 100 -100 100] >>] ";

/// Object 8's samples: tint `t` to RGB `(1 - t, 1 - t / 2, 1)`, one byte per output.
fn sampled_ramp() -> Vec<u8> {
    (0..=255u8)
        .flat_map(|tint| [255 - tint, 255 - tint / 2, 255])
        .collect()
}

// Same-output hits, selections and restorations are separate from the intent and black-point
// changes the stale baseline did not perform. Whole interpretations, not conversion timings.
fn tint_and_cie_workloads(out: &mut Vec<Workload>) {
    for (name, population, prefix, even, odd) in [
        (
            "sep-repeat",
            "same-output-hit",
            "/Perceptual ri /S cs .5 scn ",
            "",
            "",
        ),
        (
            "sep-select",
            "same-output-selection",
            "/Perceptual ri /S cs ",
            ".25 scn ",
            ".75 scn ",
        ),
        (
            "sep-intent",
            "required-current-state",
            "/S cs .5 scn ",
            "/Perceptual ri ",
            "/Saturation ri ",
        ),
        (
            "sep-bpc",
            "required-current-state",
            "/RelativeColorimetric ri /S cs 1 scn ",
            "/Off gs ",
            "/On gs ",
        ),
        (
            "devicen-select",
            "same-output-selection",
            "/Perceptual ri /N cs ",
            ".25 .5 scn ",
            ".75 .5 scn ",
        ),
        (
            "sampled-select",
            "same-output-selection",
            "/SS cs ",
            ".25 scn ",
            ".75 scn ",
        ),
        (
            "sampled-cs",
            "same-output-initial-selection",
            "",
            "/SS cs ",
            "/SS cs ",
        ),
        (
            "cal-repeat",
            "same-output-hit",
            "/C cs .45 .25 .05 scn ",
            "",
            "",
        ),
        (
            "cal-select",
            "same-output-selection",
            "/C cs ",
            ".45 .25 .05 scn ",
            ".12 .25 .75 scn ",
        ),
        (
            "lab-select",
            "same-output-selection",
            "/L cs ",
            "50 60 -20 scn ",
            "50 -20 60 scn ",
        ),
    ] {
        let mut content = format!("/Off gs {prefix}");
        for index in 0..1000 {
            content.push_str(if index % 2 == 0 { even } else { odd });
            content.push_str("0 0 50 100 re f ");
        }
        out.push(Workload {
            name,
            population,
            content,
            step: 100,
        });
    }
    for (name, outer, inner) in [
        ("sep-restore", "/Perceptual ri /S cs .5 scn ", ".75 scn "),
        ("sampled-restore", "/SS cs .5 scn ", ".75 scn "),
        ("cal-restore", "/C cs .45 .25 .05 scn ", ".12 .25 .75 scn "),
    ] {
        let mut content = format!("/Off gs {outer}");
        for _ in 0..500 {
            write!(content, "q {inner}0 0 50 100 re f Q 0 0 50 100 re f ").expect("restore pair");
        }
        out.push(Workload {
            name,
            population: "same-output-restoration",
            content,
            step: 100,
        });
    }
}

fn paired_workloads(out: &mut Vec<Workload>) {
    // Two marks per pair: the second resolves a restored source without reselecting it.
    for (name, population, inner) in [
        ("icc-solid-values", "same-output-source-churn", ".75 scn "),
        (
            "icc-solid-profiles",
            "same-output-profile-churn",
            "/B cs .5 scn ",
        ),
        ("icc-mixed-repeat", "mixed-control", "/PA cs .5 /Tile scn "),
        (
            "icc-mixed-context",
            "mixed-context-control",
            "/Saturation ri /PA cs .5 /Tile scn ",
        ),
        (
            "icc-mixed-profile",
            "mixed-profile-control",
            "/PB cs .5 /Tile scn ",
        ),
    ] {
        let mut content = String::from("/Off gs /Perceptual ri /A cs .5 scn ");
        for _ in 0..500 {
            write!(content, "q {inner}0 0 50 100 re f Q 0 0 50 100 re f ").expect("pair");
        }
        out.push(Workload {
            name,
            population,
            content,
            step: 100,
        });
    }
}

/// Measures separate populations without treating generic ICC tint conversions as cache hits.
pub(super) fn run() {
    for work in workloads() {
        let document =
            Document::open(fixture(&work.content, work.step)).expect("valid ICC cost fixture");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let warm = pdf_model::interpret(&document, &page);
        assert!(warm.is_complete(), "{}: {:?}", work.name, warm.unsupported);
        assert!(warm.display_list.command_count() > 0, "empty {}", work.name);
        println!(
            "population {}: {}, tile-step {}, commands {}",
            work.name,
            work.population,
            work.step,
            warm.display_list.command_count()
        );
        println!("sample {}: {:?}", work.name, super::pixel(&warm, 25, 50));
        std::hint::black_box(warm);
        let began = std::time::Instant::now();
        for _ in 0..20 {
            std::hint::black_box(pdf_model::interpret(&document, &page));
        }
        println!(
            "{}: 20 interpretations x 1000 primary marks: {:.3} ms",
            work.name,
            began.elapsed().as_secs_f64() * 1000.0
        );
    }
    for work in redefinition_workloads() {
        let document = Document::open(intent_fixture(&work.content)).expect("valid intent fixture");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let warm = pdf_model::interpret(&document, &page);
        assert!(warm.is_complete(), "{}: {:?}", work.name, warm.unsupported);
        assert!(warm.display_list.command_count() > 0, "empty {}", work.name);
        println!(
            "population {}: {}, tile-step {}, commands {}",
            work.name,
            work.population,
            work.step,
            warm.display_list.command_count()
        );
        println!("sample {}: {:?}", work.name, super::pixel(&warm, 25, 50));
        std::hint::black_box(warm);
        let began = std::time::Instant::now();
        for _ in 0..20 {
            std::hint::black_box(pdf_model::interpret(&document, &page));
        }
        println!(
            "{}: 20 interpretations x 1000 primary marks: {:.3} ms",
            work.name,
            began.elapsed().as_secs_f64() * 1000.0
        );
    }
}

/// A v2 CMYK output profile whose one tag is a two-point `A2B1` `mft2` over a multiplicative
/// press: small, so that what a population measures is the paint rather than the profile's
/// decoding, which a full press characterisation makes most of an interpretation.
fn press_profile() -> Vec<u8> {
    let mut tag = Vec::from(*b"mft2");
    tag.extend_from_slice(&[0, 0, 0, 0, 4, 3, 2, 0]);
    for value in [1i32, 0, 0, 0, 1, 0, 0, 0, 1] {
        tag.extend_from_slice(&(value * 65536).to_be_bytes());
    }
    tag.extend_from_slice(&[0, 2, 0, 2]);
    for _ in 0..4 {
        tag.extend_from_slice(&[0, 0, 0xFF, 0xFF]);
    }
    for corner in 0..16usize {
        let ink = |axis: usize| {
            if (corner >> (3 - axis)) & 1 == 1 {
                1.0f32
            } else {
                0.0
            }
        };
        let absorb = [
            [0.60f32, 0.10, 0.10, 0.80],
            [0.10, 0.60, 0.20, 0.80],
            [0.10, 0.20, 0.70, 0.80],
        ];
        for (white, row) in [0.964_2f32, 1.0, 0.824_9].into_iter().zip(absorb) {
            let xyz = (0..4).fold(white, |xyz, axis| xyz * (1.0 - row[axis] * ink(axis)));
            let encoded = (xyz * 32768.0).clamp(0.0, 65535.0) as u16;
            tag.extend_from_slice(&encoded.to_be_bytes());
        }
    }
    for _ in 0..3 {
        tag.extend_from_slice(&[0, 0, 0xFF, 0xFF]);
    }
    let mut out = vec![0u8; 128];
    out[8] = 2;
    out[12..16].copy_from_slice(b"prtr");
    out[16..20].copy_from_slice(b"CMYK");
    out[20..24].copy_from_slice(b"XYZ ");
    out[36..40].copy_from_slice(b"acsp");
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(b"A2B1");
    out.extend_from_slice(&144u32.to_be_bytes());
    out.extend_from_slice(&u32::try_from(tag.len()).expect("a small tag").to_be_bytes());
    out.extend_from_slice(&tag);
    out
}

/// A page under an output intent whose profile is [`press_profile`], with
/// `/G` an isolated group in the same profile stated as `ICCBased`, `/SC` a `Separation` over
/// `DeviceCMYK` and `/PK` an uncoloured pattern over it. `content` is the page's, and a
/// `/G Do` in it paints `/G`'s own stream, which is `content` after its `%G` marker.
fn intent_fixture(content: &str) -> Vec<u8> {
    let profile = press_profile();
    let (page, group) = content.split_once("%G\n").unwrap_or((content, ""));
    let resources = "/ColorSpace << /SC [/Separation /Spot /DeviceCMYK << /FunctionType 2 \
        /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 0 0] /N 1 >>] /PK [/Pattern /DeviceCMYK] >> \
        /Pattern << /Tile 7 0 R >> /ExtGState << /Off << /UseBlackPtComp /OFF >> >>";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX \
         /OutputConditionIdentifier (GRACoL) /DestOutputProfile 5 0 R >>] >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << {resources} /XObject << /G 6 0 R >> >> >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{page}\nendstream",
            page.len() + 1
        ),
        encoded_stream("/N 4", &profile),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Group << /S /Transparency /I true /CS [/ICCBased 5 0 R] >> \
             /Resources << {resources} >> /Length {} >>\nstream\n{group}\nendstream",
            group.len() + 1
        ),
        "<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 100 100] \
         /XStep 100 /YStep 100 /Resources << >> /Length 15 >>\nstream\n0 0 100 100 re f\nendstream"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{object}\nendobj", index + 1).expect("object");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).expect("xref");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("offset");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("trailer");
    out.into_bytes()
}

/// ADR 2276's populations: `DeviceCMYK` read as the output intent's profile — selected,
/// restored, as a `Separation`'s alternate and an uncoloured pattern's underlying space — and
/// inherited into an `ICCBased` CMYK group, where a restored colour is read again at each
/// paint. Whole interpretations of 1 000 primary marks, not conversion timings.
fn redefinition_workloads() -> Vec<Workload> {
    let mark = "0 0 50 100 re f ";
    let alternate = |prefix: &str, even: &str, odd: &str| {
        let mut content = format!("/Off gs {prefix}");
        for index in 0..1000 {
            content.push_str(if index % 2 == 0 { even } else { odd });
            content.push_str(mark);
        }
        content
    };
    let restored = |outer: &str, inner: &str| {
        let mut content = format!("/Off gs {outer}");
        for _ in 0..500 {
            write!(content, "q {inner}{mark}Q {mark}").expect("restore pair");
        }
        content
    };
    let inherited = |outer: &str, inner: &str| {
        let mut content = format!("/Off gs {outer}/G Do\n%G\n");
        for _ in 0..500 {
            write!(content, "q {inner}{mark}Q {mark}").expect("inherited pair");
        }
        content
    };
    [
        (
            "cmyk-oi-repeat",
            "same-output-hit",
            alternate("0 1 0 0 k ", "", ""),
        ),
        (
            "cmyk-oi-select",
            "same-output-selection",
            alternate("", "0 1 0 0 k ", "1 0 0 0 k "),
        ),
        (
            "cmyk-oi-restore",
            "same-output-restoration",
            restored("0 1 0 0 k ", "1 0 0 0 k "),
        ),
        (
            "sep-cmyk-select",
            "same-output-selection",
            alternate("/SC cs ", ".25 scn ", ".75 scn "),
        ),
        (
            "sep-cmyk-restore",
            "same-output-restoration",
            restored("/SC cs .5 scn ", ".75 scn "),
        ),
        (
            "tint-cmyk-repeat",
            "generic-tint-control",
            alternate("/PK cs 0 1 0 0 /Tile scn ", "", ""),
        ),
        (
            "cmyk-oi-inherit",
            "required-current-state",
            inherited("0 1 0 0 k ", "1 0 0 0 k "),
        ),
        (
            "sep-oi-inherit",
            "required-current-state",
            inherited("/SC cs .5 scn ", ".75 scn "),
        ),
    ]
    .into_iter()
    .map(|(name, population, content)| Workload {
        name,
        population,
        content,
        step: 100,
    })
    .collect()
}

// Equal-output source/table/profile restoration is separate from context work that
// the stale baseline omits. These are whole interpretations, not conversion timings.
fn indexed_workloads(out: &mut Vec<Workload>) {
    for (name, population, prefix, even, odd) in [
        (
            "indexed-repeat",
            "same-output-hit",
            "/Perceptual ri /IA cs 1 scn ",
            "",
            "",
        ),
        (
            "indexed-select",
            "same-output-selection",
            "/Perceptual ri /IA cs ",
            "0 scn ",
            "2 scn ",
        ),
        (
            "indexed-intent",
            "required-current-state",
            "/IA cs 1 scn ",
            "/Perceptual ri ",
            "/Saturation ri ",
        ),
        (
            "indexed-bpc",
            "required-current-state",
            "/RelativeColorimetric ri /End cs 0 scn ",
            "/Off gs ",
            "/On gs ",
        ),
    ] {
        let mut content = format!("/Off gs {prefix}");
        for index in 0..1000 {
            content.push_str(if index % 2 == 0 { even } else { odd });
            content.push_str("0 0 50 100 re f ");
        }
        out.push(Workload {
            name,
            population,
            content,
            step: 100,
        });
    }
    for (name, inner) in [
        ("indexed-entry-restore", "2 scn "),
        ("indexed-profile-restore", "/IB cs 0 scn "),
        ("indexed-table-restore", "/IT cs 0 scn "),
    ] {
        let mut content = String::from("/Off gs /Perceptual ri /IA cs 0 scn ");
        for _ in 0..500 {
            write!(content, "q {inner}0 0 50 100 re f Q 0 0 50 100 re f ").expect("indexed pair");
        }
        out.push(Workload {
            name,
            population: "same-output-restoration",
            content,
            step: 100,
        });
    }
}
