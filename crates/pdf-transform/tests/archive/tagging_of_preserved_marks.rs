//! A tagged document's annotation marks, kept on an appended page, stand outside its structure
//! tree as an artifact behind the loss `tagging-of-preserved-marks`, at PDF/A-2b and every PDF/A-4
//! flavour; Level A and Level U keep the refusal (`doc/questions/A1011`, `doc/adr/3509`).
//!
//! ISO 32000-2 §14.8.2.2.2: "Any content that is not included in the structure tree is an artifact
//! even when not enclosed in a markedcontent sequence using the tag Artifact", and its EXAMPLE 1,
//! `/Artifact BMC … EMC`, is the form "used to identify a generic artifact". So the expected page
//! content is the marks' invocation enclosed in that sequence, and the expected tree is the
//! producer's, untouched: nothing was added to it.

use super::*;
use pdf_syntax::object::Object;
use pdf_transform::profiles::{Contribution, named};

const ROW: &str = "annotations/subtype-defined-in-iso-32000-2";
const ROW_TWO: &str = "annotations/subtype-defined-in-iso-32000-1";

/// The fixture of the refusal's own test, at part 2's header where the target is part 2.
fn a_forbidden_annotation_in_a_tagged_document_for(target: Target) -> Vec<u8> {
    let marks = b"0 0 40 40 re f\n";
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        catalog: "/MarkInfo << /Marked true >> /StructTreeRoot 8 0 R".to_owned(),
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Sound /Rect [20 30 60 70] /F 4 /AP << /N 7 0 R >> >>"
                .to_owned(),
        ],
        binary_objects: vec![
            stream(
                &format!(
                    "/Type /XObject /Subtype /Form /BBox [0 0 40 40] /Length {}",
                    marks.len()
                ),
                marks,
            ),
            b"<< /Type /StructTreeRoot /K [9 0 R] >>".to_vec(),
            b"<< /Type /StructElem /S /Document /P 8 0 R >>".to_vec(),
        ],
        ..base
    }
    .build()
}

/// The row a target's report names for a `Sound` annotation.
const fn row_for(target: Target) -> &'static str {
    match target {
        Target::Two(_) => ROW_TWO,
        Target::Four(_) => ROW,
    }
}

/// A plan preserving the annotation by appended page, with the loss taken or not.
fn a_plan(target: Target, taken: bool) -> ArchivePlan {
    let row = row_for(target);
    let mut plan = plan_from(
        &format!("[site.\"{row}\"]\nremedy = \"preserve\"\nplacement = \"append\"\n"),
        target,
    );
    if taken {
        plan.authorised.authorise(Loss::TaggingOfPreservedMarks);
    }
    plan
}

#[test]
fn at_2b_and_part_4_the_marks_stand_outside_the_tree_as_an_artifact() {
    // PDF/A-4f is offered the loss as the other flavours are, and is not converted here only
    // because ISO 19005-4 Annex A.2 requires it to hold an embedded file this fixture has none of.
    for target in [
        Target::Two(Level::B),
        Target::Four(Flavour::Plain),
        Target::Four(Flavour::E),
    ] {
        let source = a_forbidden_annotation_in_a_tagged_document_for(target);
        let (report, output) = convert_with_plan(&source, &a_plan(target, true));
        let output = output.unwrap_or_else(|| {
            panic!(
                "{target:?}: the document converts:\n{}",
                conversion(&report).render()
            )
        });

        // The producer's page is unchanged and the marks are on the page after it, enclosed as
        // §14.8.2.2.2's generic artifact around the invocation `marks_stream` always wrote.
        let contents = page_contents(&output);
        assert_eq!(contents.len(), 2, "{target:?}: one page appended");
        let appended = String::from_utf8_lossy(&contents[1]).into_owned();
        assert!(
            appended.starts_with("/Artifact BMC\nq\n") && appended.ends_with("Do\nQ\nEMC\n"),
            "{target:?}: the marks enclosed as a generic artifact:\n{appended}"
        );

        // Nothing was added to the producer's tree: the root still names its one element, and the
        // appended page states no /StructParents because none of its content is in the tree.
        let document =
            Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("the output opens");
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(1).expect("the appended page");
        assert!(
            page.dict.get("StructParents").is_none(),
            "{target:?}: the appended page is not in the tree"
        );
        let root = document
            .catalog()
            .ok()
            .and_then(|catalog| catalog.get("StructTreeRoot").cloned())
            .expect("the tree is kept");
        let root = document.resolve(&root);
        let kids = root
            .as_dict()
            .and_then(|root| root.get("K"))
            .and_then(Object::as_array)
            .map(<[Object]>::len);
        assert_eq!(kids, Some(1), "{target:?}: the producer's one element only");

        // The loss is named on the row, per page, in the report a person reads and in the file's
        // own history.
        let preserved = &conversion(&report).preserved;
        let row = preserved.first().expect("the marks are preserved");
        assert_eq!(row.site, row_for(target));
        assert_eq!(row.pages, vec![1]);
        assert!(row.outside_the_tree, "{target:?}: the row carries the loss");
        assert!(
            row.declined
                .is_some_and(|declined| declined.contains("never among the producer's own")),
            "{target:?}: and says why the producer's page was not the one: {:?}",
            row.declined
        );
        let rendered = conversion(&report).render();
        assert!(
            rendered.contains("losing tagging-of-preserved-marks"),
            "{target:?}: the report names the loss:\n{rendered}"
        );
        let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
        assert!(
            packet.contains("losing tagging-of-preserved-marks"),
            "{target:?}: and so does the file's own history:\n{packet}"
        );
    }
}

#[test]
fn at_level_a_and_level_u_the_refusal_stands_whatever_was_authorised() {
    // A1011 keeps the refusal at Level A, whose subject is the logical structure, and names 2b and
    // part 4 only, so Level U keeps it too until the owner says otherwise (`doc/questions/Q1023`).
    for target in [Target::Two(Level::A), Target::Two(Level::U)] {
        let source = a_forbidden_annotation_in_a_tagged_document_for(target);
        let (report, output) = convert_with_plan(&source, &a_plan(target, true));
        assert!(output.is_none(), "{target:?}: nothing is written");
        let Decision::Refused(because) = decision(&report, ROW_TWO) else {
            panic!("{target:?}: the preservation is refused");
        };
        assert!(
            because.sentence().contains("structure type"),
            "{target:?}: by the sentence that says what it waits on: {}",
            because.sentence()
        );
    }
}

#[test]
fn without_the_loss_the_refusal_names_it() {
    let target = Target::Two(Level::B);
    let source = a_forbidden_annotation_in_a_tagged_document_for(target);
    let (report, output) = convert_with_plan(&source, &a_plan(target, false));
    assert!(output.is_none(), "nothing is written");
    let Decision::Refused(because) = decision(&report, ROW_TWO) else {
        panic!("the preservation is refused");
    };
    assert!(
        because
            .sentence()
            .contains("--authorise tagging-of-preserved-marks"),
        "the sentence names the loss that lifts it: {}",
        because.sentence()
    );
}

#[test]
fn as_shown_and_state_every_loss_take_the_loss_and_as_if_printed_has_no_marks_to_take_it_for() {
    // `as-if-printed` discards a forbidden annotation (paper never carried an annotation the
    // standard forbids), so it never appends marks and states no answer for the loss.
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let grants = |profile: &str| {
            Contribution::read(named(profile).expect("a shipped profile").text, target)
                .expect("a shipped profile reads")
                .authorised
                .grants(Loss::TaggingOfPreservedMarks)
        };
        assert!(grants("as-shown"), "{target:?}");
        assert!(grants("state-every-loss"), "{target:?}");
        assert!(!grants("as-if-printed"), "{target:?}");
    }
}
