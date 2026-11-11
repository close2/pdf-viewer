//! ADR 2500's mechanical resource repair preserves the reader's interpreted marks.
//! The malformed pattern case is a project preservation-policy regression, not an ISO
//! requirement for malformed-input rendering. §8.7.3.1 Table 74 requires pattern Resources.
use super::{
    Authorisations, Conforming, Document, Flavour, Level, Target, convert, dark_pixels, page_of,
    stream,
};

fn tiling_pattern(resources: &str) -> Vec<u8> {
    stream(
        &format!(
            "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
             /BBox [0 0 10 10] /XStep 10 /YStep 10 {resources}"
        ),
        b"/Sq Do",
    )
}

fn pattern_source(target: Target, pattern_resources: &str) -> Vec<u8> {
    // The separate page mark ensures device-gray color handling is already planned before
    // repair. Otherwise unrelated output-intent validation can refuse the newly exposed cell
    // and hide a change to the resource lookup.
    Conforming {
        binary_objects: vec![
            tiling_pattern(pattern_resources),
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << /ColorSpace << /G [/CalGray << /WhitePoint [0.9505 1 1.089] >>] >> >>",
                b"/G cs 0 scn 2 2 6 6 re f",
            ),
        ],
        ..page_of(
            target,
            "/Pattern << /P0 6 0 R >> /XObject << /Sq 7 0 R >>",
            "0 g 0 0 1 1 re f /Pattern cs /P0 scn 0 0 200 200 re f",
        )
    }
    .build()
}

fn check_pattern_resource_repair(target: Target) {
    // Positive control: a valid PDF pattern explicitly names the same /Sq form.
    // The source is not claimed to conform to the archival profile before conversion.
    let valid = pattern_source(target, "/Resources << /XObject << /Sq 7 0 R >> >>");
    let valid_pixels = dark_pixels(&valid);
    assert!(
        valid_pixels.iter().any(|dark| *dark),
        "the valid cell paints black"
    );
    let (_, valid_output) = convert(&valid, target, Authorisations::default());
    let valid_output = valid_output.expect("the explicitly resourced pattern converts");
    assert!(
        dark_pixels(&valid_output) == valid_pixels,
        "the explicitly resourced control keeps its mark mask"
    );

    // Malformed input: Table 74 requires /Resources, and the current reader uses an empty
    // dictionary here. /Sq exists only in the page resources, so this pattern cell paints none.
    let malformed = pattern_source(target, "");
    let before = Document::open(malformed.clone()).expect("malformed fixture is a readable PDF");
    assert!(
        !before.was_recovered(),
        "only the pattern resource entry is malformed"
    );
    let page = pdf_model::Pages::new(&before).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&before, &page);
    assert!(
        interpretation
            .unsupported
            .contains(&pdf_model::Unsupported::MissingResource {
                category: "XObject",
                detail: "/Sq is not in /XObject".to_owned(),
            }),
        "source pattern lookup must remain against its empty own dictionary"
    );
    let malformed_pixels = dark_pixels(&malformed);
    assert_eq!(
        malformed_pixels.iter().filter(|dark| **dark).count(),
        4,
        "only the one-point page control paints at two pixels per point; unresolved /Sq paints no cell marks"
    );

    // ADR 2500 claims mechanical resource assignment preserves name resolution and marks.
    // The input's missing required Resources is malformed; this expectation tests that
    // project preservation policy, not an ISO rule governing malformed-input recovery.
    let (_, output) = convert(&malformed, target, Authorisations::default());
    if let Some(output) = output {
        let repaired_pixels = dark_pixels(&output);
        assert!(
            repaired_pixels == malformed_pixels,
            "a mechanical repair emits {} dark pixels instead of 4 for {target:?}; it must preserve the reader's marks or refuse",
            repaired_pixels.iter().filter(|dark| **dark).count(),
        );
    }
}

#[test]
fn part2_pattern_resource_repair_preserves_the_readers_marks_or_refuses() {
    check_pattern_resource_repair(Target::Two(Level::B));
}

#[test]
fn part4_pattern_resource_repair_preserves_the_readers_marks_or_refuses() {
    check_pattern_resource_repair(Target::Four(Flavour::Plain));
}
