//! ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3's JPX baseline rule, one fault per
//! fixture.
//!
//! Both subclauses restrict JPEG 2000 data to the JPX baseline set of features, as the base
//! standard and the subclause restrict or extend it, and both NOTE 1s send the definition to
//! ISO/IEC 15444-2 M.9.2 — held here as its identical joint text ITU-T T.801 (ADR 1383). Each
//! fixture below is a JPX file that breaks exactly one of M.9.2's subclauses, or none, so the
//! finding it draws is the subclause's and nothing else's.
//!
//! The files are the shape `opj_compress -o three.jp2 -F 4,1,3,8,u -n 1 -r 1` writes — signature,
//! a `jp2 ` File Type box, a JP2 Header box of an image header and one enumerated `colr` box, the
//! codestream — rebuilt here around that command's codestream with its `COM` marker segment
//! removed, which is what lets each test patch the colour specification box `opj_compress` cannot
//! be asked to write: it states sRGB for three components and greyscale for one, and nothing
//! else.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and the fixtures \
              are a few hundred bytes where no offset can overflow"
)]

use std::fmt::Write as _;

use pdf_archive::{Level, Outcome, Target, check};
use pdf_syntax::Document;

/// The requirement every test here is about.
const BASELINE: &str = "graphics/jpeg2000-uses-the-baseline-feature-set";

/// A 4×1 three-component codestream, `crates/pdf-model/tests/jpx_enumerated_spaces.rs`'s.
const THREE: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x07, 0x01, 0x01, 0x07, 0x01, 0x01,
    0x07, 0x01, 0x01, 0xff, 0x52, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x04, 0x04, 0x00,
    0x01, 0xff, 0x5c, 0x00, 0x04, 0x40, 0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x24, 0x00, 0x01, 0xff, 0x93, 0xc7, 0xd4, 0x0a, 0x0f, 0xf8, 0xe5, 0xba, 0x7f, 0xc7, 0xd4, 0x06,
    0x0d, 0xef, 0xbf, 0xdf, 0x80, 0x28, 0x0f, 0xf2, 0xb6, 0xc8, 0xf3, 0xff, 0xd9,
];

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

/// A `colr` payload: `METH`, `PREC`, `APPROX`, then `EnumCS` where the method is 1.
fn colour(method: u8, approximation: u8, space: u32) -> Vec<u8> {
    let mut out = vec![method, 0, approximation];
    if method == 1 {
        out.extend_from_slice(&space.to_be_bytes());
    } else {
        // A profile's or a vendor code's bytes; this rule reads only which method states them.
        out.extend_from_slice(&[0; 16]);
    }
    out
}

/// The JP2 Header box's payload: `opj_compress`'s image header, then the `colr` boxes given.
fn header(colours: &[Vec<u8>]) -> Vec<u8> {
    let mut out = boxed(*b"ihdr", &[0, 0, 0, 1, 0, 0, 0, 4, 0, 3, 7, 7, 0, 0]);
    for payload in colours {
        out.extend(boxed(*b"colr", payload));
    }
    out
}

/// A JP2 file with the given colour specification boxes, and `extra` boxes after it.
fn jp2(colours: &[Vec<u8>], extra: &[u8]) -> Vec<u8> {
    let mut out = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    out.extend(boxed(*b"ftyp", b"jp2 \0\0\0\0jp2 "));
    out.extend(boxed(*b"jp2h", &header(colours)));
    out.extend(boxed(*b"jp2c", THREE));
    out.extend_from_slice(extra);
    out
}

/// A one-page document whose object 4 is an image `XObject` over `data`.
fn document(data: &[u8]) -> Document {
    document_drawing(data, false)
}

/// The same, and where `drawn` the page's content stream draws the image.
fn document_drawing(data: &[u8], drawn: bool) -> Document {
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    if drawn {
        body.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 4 1] /Resources << /XObject \
              << /Im0 4 0 R >> >> /Contents 5 0 R >>\nendobj\n",
        );
    } else {
        body.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 4 1] >>\nendobj\n",
        );
    }
    body.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XObject /Subtype /Image /Width 4 /Height 1 /Filter /JPXDecode \
             /Length {} >>\nstream\n",
            data.len()
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    if drawn {
        body.extend_from_slice(
            b"5 0 obj\n<< /Length 22 >>\nstream\n4 0 0 1 0 0 cm /Im0 Do\nendstream\nendobj\n",
        );
    }

    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
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
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00> <11>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    Document::open(out).expect("the fixture is a valid PDF")
}

/// What one requirement found in `data` at both parts, which must agree: its findings' sentences.
fn found(id: &str, data: &[u8]) -> Vec<String> {
    let document = document(data);
    let mut answers = Vec::new();
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let report = check(&document, target);
        let sentences: Vec<String> = report
            .failures()
            .filter(|judgement| judgement.id == id)
            .flat_map(|judgement| match &judgement.outcome {
                Outcome::Failed { places, .. } => {
                    places.iter().map(|place| place.what.clone()).collect()
                }
                _ => Vec::new(),
            })
            .collect();
        answers.push(sentences);
    }
    assert_eq!(answers[0], answers[1], "both parts state the same rule");
    answers.swap_remove(0)
}

/// Every colour specification on M.9.2.4's list, and ISO 32000's CMYK, is baseline.
#[test]
fn every_listed_colour_specification_is_baseline() {
    for payload in [16, 17, 18, 20, 21, 24, 14, 12]
        .map(|space| colour(1, 0, space))
        .into_iter()
        .chain([colour(2, 0, 0), colour(3, 1, 0)])
    {
        assert_eq!(
            found(BASELINE, &jp2(std::slice::from_ref(&payload), &[])),
            Vec::<String>::new(),
            "{payload:?}"
        );
    }
}

/// A non-baseline enumerated space is named, and so is the list it is not on.
#[test]
fn a_space_off_the_list_is_named() {
    let findings = found(BASELINE, &jp2(&[colour(1, 0, 3)], &[]));
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].contains("enumerated colour space 3, which is not in the JPX baseline set")
            && findings[0].contains("M.9.2.4"),
        "{findings:?}"
    );
}

/// One listed specification beside an unlisted one is enough, since M.9.2.4 asks for at least one.
#[test]
fn one_listed_specification_beside_others_is_enough() {
    let data = jp2(&[colour(1, 0, 3), colour(1, 0, 16)], &[]);
    assert_eq!(found(BASELINE, &data), Vec::<String>::new());
}

/// The Vendor Colour method alone is not on the list.
#[test]
fn a_vendor_colour_alone_is_not_baseline() {
    let findings = found(BASELINE, &jp2(&[colour(4, 0, 0)], &[]));
    assert!(
        findings.len() == 1 && findings[0].contains("the Vendor Colour method"),
        "{findings:?}"
    );
}

/// CIE Jab is baseline by M.9.2.4 and forbidden by a sentence of its own, reported once, there.
#[test]
fn ciejab_is_reported_by_its_own_row_alone() {
    let data = jp2(&[colour(1, 0, 19)], &[]);
    assert_eq!(found(BASELINE, &data), Vec::<String>::new());
    assert_eq!(
        found("graphics/jpeg2000-no-ciejab-colour-space", &data).len(),
        1
    );
}

/// M.9.2.4's second requirement: one specification at `APPROX` 3 or less.
#[test]
fn a_poor_approximation_alone_is_not_baseline() {
    let findings = found(BASELINE, &jp2(&[colour(1, 4, 16)], &[]));
    assert!(
        findings.len() == 1 && findings[0].contains("APPROX of 3"),
        "{findings:?}"
    );
}

/// A bare codestream is not a JPX file at all.
#[test]
fn a_bare_codestream_is_not_a_baseline_file() {
    let findings = found(BASELINE, THREE);
    assert!(
        findings.len() == 1 && findings[0].contains("bare codestream"),
        "{findings:?}"
    );
}

/// Data that is not JPEG 2000 is not used as ISO 32000 specifies it.
#[test]
fn data_that_is_not_jpeg_2000_is_reported() {
    let findings = found(BASELINE, b"not a codestream");
    assert!(
        findings.len() == 1 && findings[0].contains("cannot be read as a JPX file"),
        "{findings:?}"
    );
}

/// M.9.2.3: a T.801 Table A.2 extension required to decode, other than the multiple component
/// transformation, is outside the baseline — here the variable DC offset.
#[test]
fn a_required_extension_is_not_baseline() {
    let mut data = jp2(&[colour(1, 0, 16)], &[]);
    let rsiz = data
        .windows(4)
        .position(|window| window == [0xff, 0x4f, 0xff, 0x51])
        .expect("the codestream is in the file")
        + 6;
    data[rsiz..rsiz + 2].copy_from_slice(&0x8001u16.to_be_bytes());
    let findings = found(BASELINE, &data);
    assert!(
        findings.len() == 1 && findings[0].contains("variable DC offset"),
        "{findings:?}"
    );
}

/// M.9.2.7: the JP2 Header box precedes the codestream.
#[test]
fn a_header_after_the_codestream_is_not_baseline() {
    let mut data = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    data.extend(boxed(*b"ftyp", b"jp2 \0\0\0\0jp2 "));
    data.extend(boxed(*b"jp2c", THREE));
    data.extend(boxed(*b"jp2h", &header(&[colour(1, 0, 16)])));
    let findings = found(BASELINE, &data);
    assert!(
        findings.len() == 1 && findings[0].contains("M.9.2.7"),
        "{findings:?}"
    );
}

/// M.9.2.5: a fragment in another file.
#[test]
fn a_fragment_in_another_file_is_not_baseline() {
    let mut list = 1u16.to_be_bytes().to_vec();
    list.extend_from_slice(&0u64.to_be_bytes());
    list.extend_from_slice(&10u32.to_be_bytes());
    list.extend_from_slice(&1u16.to_be_bytes());
    let table = boxed(*b"ftbl", &boxed(*b"flst", &list));
    let findings = found(BASELINE, &jp2(&[colour(1, 0, 16)], &table));
    assert!(
        findings.len() == 1 && findings[0].contains("another file"),
        "{findings:?}"
    );
}

/// M.9.2.2: the first compositing layer is one codestream, the file's first.
#[test]
fn a_first_layer_of_two_codestreams_is_not_baseline() {
    let mut registration = vec![0, 1, 0, 1];
    for stream in [0u16, 1] {
        registration.extend_from_slice(&stream.to_be_bytes());
        registration.extend_from_slice(&[1, 1, 0, 0]);
    }
    let layer = boxed(*b"jplh", &boxed(*b"creg", &registration));
    let findings = found(BASELINE, &jp2(&[colour(1, 0, 16)], &layer));
    assert!(
        findings.len() == 1 && findings[0].contains("M.9.2.2"),
        "{findings:?}"
    );
}

/// The first layer's own Colour Group box is its colour, T.801 M.11.7: a baseline one there
/// answers M.9.2.4 whatever the JP2 Header box states. Where the JP2 Header box states a colour
/// specification too, the layer's header holds a box of a type the JP2 Header box holds, which is
/// M.9.2.7's second sentence and the only finding (ADR 1399).
#[test]
fn the_first_layer_s_colour_group_is_what_is_judged() {
    let group = boxed(*b"cgrp", &boxed(*b"colr", &colour(1, 0, 16)));
    let mut data = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    data.extend(boxed(*b"ftyp", b"jpx \0\0\0\0jpx jp2 "));
    data.extend(boxed(*b"jp2h", &header(&[colour(1, 0, 3)])));
    data.extend(boxed(*b"jplh", &group));
    data.extend(boxed(*b"jp2c", THREE));
    let findings = found(BASELINE, &data);
    assert!(
        findings.len() == 1 && findings[0].contains("M.9.2.7") && findings[0].contains("colr"),
        "{findings:?}"
    );

    let mut alone = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    alone.extend(boxed(*b"ftyp", b"jpx \0\0\0\0jpx jp2 "));
    alone.extend(boxed(*b"jp2h", &header(&[])));
    alone.extend(boxed(*b"jplh", &group));
    alone.extend(boxed(*b"jp2c", THREE));
    assert_eq!(found(BASELINE, &alone), Vec::<String>::new());
}

/// M.9.2.7's second sentence: the first Codestream Header box restates the image header the JP2
/// Header box already holds. A box of a type the JP2 Header box does not hold is not a repetition.
#[test]
fn a_first_codestream_header_repeating_the_jp2_header_is_not_baseline() {
    let mut data = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    data.extend(boxed(*b"ftyp", b"jpx \0\0\0\0jpx jp2 "));
    data.extend(boxed(*b"jp2h", &header(&[colour(1, 0, 16)])));
    data.extend(boxed(
        *b"jpch",
        &boxed(*b"ihdr", &[0, 0, 0, 1, 0, 0, 0, 4, 0, 3, 7, 7, 0, 0]),
    ));
    data.extend(boxed(*b"jp2c", THREE));
    let findings = found(BASELINE, &data);
    assert!(
        findings.len() == 1
            && findings[0].contains("Codestream Header")
            && findings[0].contains("ihdr"),
        "{findings:?}"
    );

    let mut resolution = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    resolution.extend(boxed(*b"ftyp", b"jpx \0\0\0\0jpx jp2 "));
    resolution.extend(boxed(*b"jp2h", &header(&[colour(1, 0, 16)])));
    resolution.extend(boxed(*b"jplh", &boxed(*b"res ", &[])));
    resolution.extend(boxed(*b"jp2c", THREE));
    assert_eq!(found(BASELINE, &resolution), Vec::<String>::new());
}

/// A first Compositing Layer Header box whose Cross-Reference box names one fragment, laid out
/// with `before` boxes ahead of the codestream and `after` boxes behind it, the fragment being
/// the payload of the box `target` among them.
///
/// The Cross-Reference box's length does not depend on the offset it states, so the file is
/// built once to find where the target lies and again with that offset written in.
fn cross_referenced(before: &[Vec<u8>], after: &[Vec<u8>], target: [u8; 4]) -> Vec<u8> {
    let build = |offset: u64| {
        let mut list = 1u16.to_be_bytes().to_vec();
        list.extend_from_slice(&offset.to_be_bytes());
        list.extend_from_slice(&4u32.to_be_bytes());
        list.extend_from_slice(&0u16.to_be_bytes());
        let mut reference = b"lbl ".to_vec();
        reference.extend(boxed(*b"flst", &list));
        let mut data = boxed(*b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
        data.extend(boxed(*b"ftyp", b"jpx \0\0\0\0jpx jp2 "));
        data.extend(boxed(*b"jp2h", &header(&[colour(1, 0, 16)])));
        data.extend(boxed(*b"jplh", &boxed(*b"cref", &reference)));
        for each in before {
            data.extend_from_slice(each);
        }
        data.extend(boxed(*b"jp2c", THREE));
        for each in after {
            data.extend_from_slice(each);
        }
        data
    };
    let placed = build(0);
    let at = placed
        .windows(4)
        .position(|window| window == target)
        .expect("the target box is in the file")
        + 4;
    build(u64::try_from(at).expect("the fixtures are small"))
}

/// M.9.2.6's last requirement: a fragment the first layer cross-references lies before the data
/// of its codestream.
#[test]
fn a_cross_referenced_fragment_after_the_codestream_is_not_baseline() {
    let label = boxed(*b"xml ", b"text");
    let late = cross_referenced(&[], std::slice::from_ref(&label), *b"xml ");
    let findings = found(BASELINE, &late);
    assert!(
        findings.len() == 1 && findings[0].contains("M.9.2.6"),
        "{findings:?}"
    );

    let early = cross_referenced(std::slice::from_ref(&label), &[], *b"xml ");
    assert_eq!(found(BASELINE, &early), Vec::<String>::new());
}

/// The device colour requirements ISO 19005-2 section 6.2.4.3 reports for a drawn image over
/// `data`, which states no `ColorSpace`, in a file with no output intent.
fn device_findings(data: &[u8]) -> Vec<&'static str> {
    let report = check(&document_drawing(data, true), Target::Two(Level::B));
    [
        "graphics/device-gray-needs-a-default-or-an-output-intent",
        "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
        "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
    ]
    .into_iter()
    .filter(|id| report.failures().any(|judgement| judgement.id == *id))
    .collect()
}

/// ISO 19005-2 section 6.2.8.3's second route to the device colour requirements: data stating no
/// colour specification is drawn in the device space of its channel count, and enumerated CMYK
/// is a device space by T.801 Table M.25; sRGB and an off-list code beside a listed one are not
/// (ADR 1399).
#[test]
fn the_device_space_the_data_defines_is_judged() {
    assert_eq!(
        device_findings(THREE),
        vec!["graphics/device-rgb-needs-a-default-or-an-rgb-output-intent"]
    );
    assert_eq!(
        device_findings(&jp2(&[colour(1, 0, 12)], &[])),
        vec!["graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent"]
    );
    assert_eq!(
        device_findings(&jp2(&[colour(1, 0, 3)], &[])),
        vec!["graphics/device-rgb-needs-a-default-or-an-rgb-output-intent"]
    );
    assert_eq!(
        device_findings(&jp2(&[colour(1, 0, 16)], &[])),
        Vec::<&str>::new()
    );
    assert_eq!(
        device_findings(&jp2(&[colour(1, 1, 16), colour(1, 0, 12)], &[])),
        Vec::<&str>::new()
    );
}
