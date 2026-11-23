//! A named resource no content stream names, exempt under ISO 19005 section 6.2.2, is removed
//! where the exemption withdrew a finding at what it reaches (ADR 3410).
//!
//! Both parts' section 6.2.2 close with the sentence that such a resource is not used for rendering
//! and is exempt, so this tree's validator withdraws the finding; a validator that does
//! not apply the exemption fails the file, and the converter removes the entry, which draws
//! nothing, so that neither reading fails the output.

use super::*;

/// A page whose `/XObject` resource names an image the validator fails, interpolated, and whose
/// content stream either names it or does not.
fn an_interpolated_image(drawn: bool) -> Vec<u8> {
    let content: &[u8] = if drawn {
        b"q 10 0 0 10 0 0 cm /Im0 Do Q"
    } else {
        b"0 0 m 10 10 l S"
    };
    Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
                 /BitsPerComponent 8 /Interpolate true /Length 1",
                b"\x00",
            ))
            .into_owned(),
        ],
        resources: "/XObject << /Im0 6 0 R >>".to_owned(),
        contents: Some((String::new(), content.to_vec())),
        ..Conforming::default()
    }
    .build()
}

#[test]
fn an_unnamed_resource_whose_finding_the_exemption_withdrew_is_not_written() {
    let source = an_interpolated_image(false);
    let row = "graphics/image-interpolation-is-off";
    let held = holds(&source, Target::Four(Flavour::Plain));
    let judgement = held
        .judgements
        .iter()
        .find(|judgement| judgement.id == row)
        .expect("the row binds part 4");
    assert!(
        matches!(judgement.outcome, Outcome::Met) && judgement.withdrawn > 0,
        "the exemption withdraws the image's finding: {:?}, {}",
        judgement.outcome,
        judgement.withdrawn
    );
    let (report, output) = to_part_four(&source);
    let output = output.expect("the document converts");
    assert_eq!(
        conversion(&report).removed_unreferenced_resources.len(),
        1,
        "{:?}",
        conversion(&report).removed_unreferenced_resources
    );
    let text = String::from_utf8_lossy(&output);
    assert!(
        !text.contains("/Im0") && !text.contains("/Interpolate"),
        "neither the entry nor the image it alone reached is written"
    );
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
}

#[test]
fn a_resource_the_page_draws_is_kept_whatever_it_fails() {
    // The same image drawn: the exemption withdraws nothing, and the removal never runs, so the
    // row asks its own remedy and the resource stays.
    let (report, _) = to_part_four(&an_interpolated_image(true));
    assert_eq!(
        conversion(&report).removed_unreferenced_resources,
        Vec::<String>::new()
    );
    assert!(matches!(
        decision(&report, "graphics/image-interpolation-is-off"),
        Decision::Unauthorised { .. }
    ));
}

#[test]
fn a_resource_a_form_stating_no_resources_names_is_kept() {
    // pdf.js `issue6541`'s shape: the page draws a form that states no `/Resources` of its own,
    // and the form names the image from the page's dictionary. The page's own content never
    // names the image, but a removal has to be safe for every stream that could look the name up,
    // so the entry stays (ADR 3410).
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length 7",
                b"/Im0 Do",
            ))
            .into_owned(),
            String::from_utf8_lossy(&stream(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
                 /BitsPerComponent 8 /Interpolate true /Length 1",
                b"\x00",
            ))
            .into_owned(),
        ],
        resources: "/XObject << /Fm0 6 0 R /Im0 7 0 R >>".to_owned(),
        contents: Some((String::new(), b"/Fm0 Do".to_vec())),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        conversion(&report).removed_unreferenced_resources,
        Vec::<String>::new()
    );
    if let Some(output) = output {
        assert!(String::from_utf8_lossy(&output).contains("/Im0"));
    }
}
