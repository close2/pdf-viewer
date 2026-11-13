//! An annotation whose `/DA` names a font the form's `/DR` does not define is given the appearance
//! this viewer draws for it, set in ADR 0112's stand-in, under the loss `stand-in-face`
//! (`doc/adr/2858`).
//!
//! The expected values come from ISO 32000-2 §12.7.4.3 — "The specified font value shall match a
//! resource name in the Font entry of the default resource dictionary" — which these documents
//! break, so the face is this viewer's stand-in (ADR 0112) rather than one the standard states; and
//! from ISO 19005-2 and ISO 19005-4 section 6.3.3, which ask an appearance dictionary of every
//! annotation the output holds. The stand-in is written only where it is the whole of what the
//! construction owes: §12.5.6.6 makes a free text annotation's text its whole appearance, so a value
//! the construction lays out only in part stays a partial rendering, refused.
use super::*;
use pdf_archive::Flavour;
use pdf_transform::archive::Loss;

const APPEARANCE_PART_TWO: &str = "annotations/appearance-dictionary-present";
const APPEARANCE_PART_FOUR: &str = "annotations/appearance-dictionary-present-from-base-standard";

/// A page holding one free text annotation stating no appearance, its `/DA` naming `/Rufscript`,
/// which no `/DR` defines — `poppler-395-0-fuzzed.pdf`'s shape — and showing `contents`, in a file
/// whose header `target` admits.
fn a_note_in_an_undefined_font(contents: &str, target: Target) -> Vec<u8> {
    let header = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![format!(
            "<< /Type /Annot /Subtype /FreeText /Rect [20 20 300 80] /F 4 \
             /DA (/Rufscript 12 Tf 0 0 0.5 rg) /Contents ({contents}) >>"
        )],
        ..header
    }
    .build()
}

/// The plan a shipped profile makes for `target`, with the authorisations its `discard` steps
/// stand for.
fn under(profile: &str, target: Target) -> ArchivePlan {
    let text = pdf_transform::profiles::named(profile)
        .expect("a shipped profile")
        .text;
    let config = pdf_transform::archive::Configuration::read(text, target).expect("it reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

#[test]
fn a_note_set_in_the_stand_in_is_written_as_drawn_under_the_loss_stand_in_face() {
    for (target, row) in [
        (Target::Two(Level::B), APPEARANCE_PART_TWO),
        (Target::Four(Flavour::Plain), APPEARANCE_PART_FOUR),
    ] {
        let source = a_note_in_an_undefined_font("Phil Smith", target);
        let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
        assert_eq!(
            decision(&report, row),
            Decision::Authorised {
                losses: Losses::one(Loss::StandInFace),
                rewrite: Rewrite::AppearanceDictionary,
            }
        );
        let output = output.unwrap_or_else(|| panic!("a file is written: {:?}", report.refused));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let conversion = conversion(&report);
        let [written] = conversion.appearances.as_slice() else {
            panic!("one appearance: {:?}", conversion.appearances);
        };
        assert_eq!(written.subtype, "FreeText");
        assert_eq!(written.stood_in.as_deref(), Some("Rufscript"));
        // The first pass wrote the appearance: what the second pass settles is the face the fonts
        // row embeds for the stand-in and the intent its colours ask for, never this row.
        assert!(
            !conversion.settled.contains(&row),
            "{:?}",
            conversion.settled
        );
        let json = conversion.to_json().render();
        assert!(
            json.contains("\"stood_in\"") && json.contains("\"Rufscript\""),
            "the machine-readable report names the font: {json}"
        );
    }
}

// A value longer than the layout lays out (`variable_text::MAX_CODES`, 16384 codes) is truncated as
// well as set in the stand-in, and the stand-in's report is the one named first. Laying it out
// again with the stand-in unreported finds the truncation, so the stream is a partial rendering
// and keeps the fence's refusal.
#[test]
fn a_stand_in_beside_another_shortfall_is_refused_as_partial() {
    let target = Target::Two(Level::B);
    let source = a_note_in_an_undefined_font(&"a".repeat(17_000), target);
    let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
    assert!(output.is_none(), "a partial rendering is not frozen in");
    let Decision::Refused(Because::TheFence(because)) = decision(&report, APPEARANCE_PART_TWO)
    else {
        panic!("{:?}", decision(&report, APPEARANCE_PART_TWO));
    };
    assert!(because.contains("construct only in part"), "{because}");
}

// `no-data-loss` grants nothing at the appearance row, and `as-shown` stops at the face the stand-in
// would need (`machine-face`, `layout-from-this-viewer`), so for each the stand-in is the question
// it asks.
#[test]
fn a_profile_that_grants_no_stand_in_is_asked_for_it() {
    let target = Target::Two(Level::B);
    let source = a_note_in_an_undefined_font("Phil Smith", target);
    for profile in ["no-data-loss", "as-shown"] {
        let (report, output) = convert_with_plan(&source, &under(profile, target));
        assert!(output.is_none(), "{profile}");
        assert_eq!(
            decision(&report, APPEARANCE_PART_TWO),
            Decision::Unauthorised {
                losses: Losses::one(Loss::StandInFace),
                rewrite: Rewrite::AppearanceDictionary,
            },
            "{profile}"
        );
    }
}
