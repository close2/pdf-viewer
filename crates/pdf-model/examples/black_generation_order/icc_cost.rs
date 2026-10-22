//! Whole-interpretation costs for ordinary ICC solids, generic ICC tints and mixed use.
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
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /A [/ICCBased 5 0 R] /B [/ICCBased 6 0 R] /PA [/Pattern [/ICCBased 5 0 R]] /PB [/Pattern [/ICCBased 6 0 R]] >> /Pattern << /Tile 7 0 R >> /ExtGState << /Off << /UseBlackPtComp /OFF >> /On << /UseBlackPtComp /ON >> >> >> >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()+1),
        encoded_stream("/N 1", &a), encoded_stream("/N 1", &b),
        format!("<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 {step} {step}] /XStep {step} /YStep {step} /Resources << >> /Length {} >>\nstream\n{cell}\nendstream", cell.len()+1),
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
    out
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
}
