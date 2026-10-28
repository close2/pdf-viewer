//! The actions of an annotation a subtype row removes go with the annotation.
//!
//! ISO 19005-4 section 6.3.1 and ISO 19005-2 section 6.3.1 do not permit the Screen type, and the
//! conversion takes such an annotation off its page. The Rendition action its `/A` holds is one
//! sections 6.6.1 and 6.5.1 do not permit either, but nothing of it is left in the output for
//! a reader to perform: the action row is answered by the annotation's removal, under the
//! removal's loss, and the interactive-behaviour loss is owed only for an action a kept holder
//! performs (ADR 2568).

use super::*;

/// The forbidden-annotation loss alone, which the subtype row asks for.
fn removing_annotations() -> Authorisations {
    Authorisations {
        forbidden_annotation: true,
        ..Authorisations::default()
    }
}

/// A §12.6.4.14 rendition action naming the Screen annotation in object 6.
const RENDITION: &str = "<< /S /Rendition /OP 0 /AN 6 0 R >>";

/// A page whose Screen annotation (object 6) performs `screen_action`, beside a Link annotation
/// (object 7) performing `link_action` where one is given, and with object 8 free for a shared
/// action dictionary.
fn a_screen(screen_action: &str, link_action: Option<&str>, shared: Option<&str>) -> Vec<u8> {
    on(Conforming::default(), screen_action, link_action, shared)
}

/// [`a_screen`]'s page on another part's fixture.
fn on(
    base: Conforming,
    screen_action: &str,
    link_action: Option<&str>,
    shared: Option<&str>,
) -> Vec<u8> {
    let link = link_action.map_or_else(
        || "<< /Type /Annot /Subtype /Link /Rect [60 30 80 60] /F 4 /Border [0 0 0] >>".to_owned(),
        |action| {
            format!(
                "<< /Type /Annot /Subtype /Link /Rect [60 30 80 60] /F 4 /Border [0 0 0] \
                 /A {action} >>"
            )
        },
    );
    let mut objects = vec![
        format!(
            "<< /Type /Annot /Subtype /Screen /Rect [20 30 40 60] /F 4 /AP << /N 9 0 R >> \
             {screen_action} >>"
        ),
        link,
    ];
    objects.push(shared.map_or_else(|| "null".to_owned(), str::to_owned));
    Conforming {
        page: "/Annots [6 0 R 7 0 R]".to_owned(),
        objects,
        binary_objects: vec![stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 20 30] /Length 15",
            b"0 0 20 30 re f\n",
        )],
        ..base
    }
    .build()
}

/// Writes a file under `REMOVED_ACTION_ARTIFACTS`, where it is set, for an outside validator.
fn artifact(name: &str, bytes: &[u8]) {
    if let Some(directory) = std::env::var_os("REMOVED_ACTION_ARTIFACTS") {
        std::fs::write(std::path::Path::new(&directory).join(name), bytes)
            .expect("the artifact is written");
    }
}

/// The requirement the Rendition action fails at every part.
const MULTIMEDIA: &str = "actions/no-launch-multimedia-or-form-actions";

#[test]
fn a_removed_screen_s_rendition_action_costs_only_the_removal() {
    let source = a_screen(&format!("/A {RENDITION}"), None, None);
    let target = Target::Four(Flavour::Plain);
    assert!(
        holds(&source, target)
            .failures()
            .any(|failed| failed.id == MULTIMEDIA),
        "the source fails the action row"
    );
    let (report, output) = convert(&source, target, removing_annotations());
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| panic!("{}", conversion.render()));
    assert_eq!(
        decision(&report, MULTIMEDIA).rewrite(),
        Some(Rewrite::ForbiddenAnnotationRemoved),
        "the row is the removal's: {:?}",
        decision(&report, MULTIMEDIA)
    );
    assert!(
        conversion.removed_actions.is_empty(),
        "nothing is taken out of an action tree the output does not hold: {:?}",
        conversion.removed_actions
    );
    assert_eq!(conversion.removed_annotations.len(), 1);
    artifact("screen-source.pdf", &source);
    artifact("screen-4.pdf", &output);
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
}

#[test]
fn a_removed_screen_s_additional_actions_cost_only_the_removal() {
    // Its `/AA` is the walk's root at part 2, where section 6.5.2 does not name an annotation's,
    // and at part 4 the row section 6.6.3 adds fails too: `/O` is a page's trigger in §12.6.3's
    // Table 198 and no annotation's in Table 197.
    let cases = [
        (Conforming::part_two(), "PV", vec![MULTIMEDIA]),
        (
            Conforming::default(),
            "O",
            vec![
                MULTIMEDIA,
                "actions/additional-actions-outside-widgets-hold-only-annotation-triggers",
            ],
        ),
    ];
    for (base, trigger, rows) in cases {
        let target = base.target;
        let source = on(
            base,
            &format!("/AA << /{trigger} {RENDITION} >>"),
            None,
            None,
        );
        let (report, output) = convert(&source, target, removing_annotations());
        let conversion = conversion(&report);
        let output = output.unwrap_or_else(|| panic!("{target}: {}", conversion.render()));
        for row in rows {
            assert_eq!(
                decision(&report, row).rewrite(),
                Some(Rewrite::ForbiddenAnnotationRemoved),
                "{target}: {row}"
            );
        }
        assert!(conversion.removed_actions.is_empty(), "{target}");
        artifact(&format!("screen-aa-{trigger}.pdf"), &output);
        let held = holds(&output, target);
        assert_eq!(
            held.verdict(),
            Verdict::Conforms,
            "{target}: {}",
            held.render()
        );
    }
}

#[test]
fn the_same_action_on_a_kept_link_still_costs_the_interactive_loss() {
    // The plant: the Rendition action on the Link annotation, which part 4 admits and keeps.
    let source = a_screen(&format!("/A {RENDITION}"), Some(RENDITION), None);
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert(&source, target, removing_annotations());
    assert!(
        output.is_none(),
        "the interactive loss nobody authorised stops it"
    );
    assert!(
        matches!(
            decision(&report, MULTIMEDIA),
            Decision::Unauthorised {
                rewrite: Rewrite::ForbiddenActionRemoved,
                ..
            }
        ),
        "{:?}",
        decision(&report, MULTIMEDIA)
    );
    let authorised = Authorisations {
        interactive_behaviour: true,
        ..removing_annotations()
    };
    let (report, output) = convert(&source, target, authorised);
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| panic!("{}", conversion.render()));
    assert_eq!(
        conversion.removed_actions.len(),
        1,
        "the Link's action alone: {:?}",
        conversion.removed_actions
    );
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
}

#[test]
fn one_action_object_both_holders_perform_is_the_kept_holder_s() {
    // Object 8 is performed from the removed Screen and from the kept Link, and the validator
    // reports it once: the walk over the kept holders is what finds it is still performed.
    let source = a_screen("/A 8 0 R", Some("8 0 R"), Some(RENDITION));
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert(&source, target, removing_annotations());
    assert!(output.is_none());
    assert!(matches!(
        decision(&report, MULTIMEDIA),
        Decision::Unauthorised { .. }
    ));
}

/// A Rendition action with `links - 1` more behind it, each the `/Next` of the one before.
fn a_chain(links: usize) -> String {
    (1..links).fold(RENDITION.to_owned(), |tail, _| {
        format!("<< /S /Rendition /OP 0 /AN 6 0 R /Next {tail} >>")
    })
}

#[test]
fn a_chain_past_the_bound_is_charged_as_the_action_rows_charge_it() {
    // Thirty-two links is the walk's bound, `actions::MAX_DEPTH`: a chain that ends inside it is
    // the removal's, and one cut at it may reach past what the walk saw, so it is answered as an
    // action row answers it.
    let target = Target::Four(Flavour::Plain);
    let inside = a_screen(&format!("/A {}", a_chain(32)), None, None);
    let (report, output) = convert(&inside, target, removing_annotations());
    assert!(output.is_some(), "{}", conversion(&report).render());
    assert_eq!(
        decision(&report, MULTIMEDIA).rewrite(),
        Some(Rewrite::ForbiddenAnnotationRemoved)
    );
    let past = a_screen(&format!("/A {}", a_chain(33)), None, None);
    let (report, output) = convert(&past, target, removing_annotations());
    assert!(output.is_none());
    assert!(
        matches!(
            decision(&report, MULTIMEDIA),
            Decision::Unauthorised {
                rewrite: Rewrite::ForbiddenActionRemoved,
                ..
            }
        ),
        "{:?}",
        decision(&report, MULTIMEDIA)
    );
}

#[test]
fn a_removal_that_stops_the_conversion_meets_no_action_row() {
    // The Screen's removal is not authorised, so nothing is removed: the action row keeps its own
    // answer and the refusal is reported once, at the subtype row.
    let source = a_screen(&format!("/A {RENDITION}"), None, None);
    let authorised = Authorisations {
        interactive_behaviour: true,
        ..Authorisations::default()
    };
    let (report, output) = convert(&source, Target::Four(Flavour::Plain), authorised);
    assert!(output.is_none());
    assert!(matches!(
        decision(&report, "annotations/subtype-defined-in-iso-32000-2"),
        Decision::Unauthorised { .. }
    ));
    assert!(matches!(
        decision(&report, MULTIMEDIA),
        Decision::Authorised {
            rewrite: Rewrite::ForbiddenActionRemoved,
            ..
        }
    ));
}
