//! Integration tests for ADR 1175's action-promotion decision.
//!
//! Parsed reachable actions, rather than byte substrings, witness preservation.

#![expect(
    clippy::expect_used,
    reason = "the fixture and conversion are required for this regression"
)]

use super::{
    Because, Conforming, Decision, Document, Flavour, Loss, Losses, Rewrite, Target, Verdict,
    behaviour_authorised, conversion, convert, decision, holds,
};
use pdf_model::action::{Action, Named, read};

fn open_action(document: &Document) -> Vec<Action> {
    let catalog = document.catalog().expect("converted catalog");
    read(document, &document.get_key(&catalog, "OpenAction"))
}

#[test]
fn removing_launch_promotes_each_survivor_in_table_196_note_1_order() {
    // Table 196 says /Next may be one action or an ordered array; NOTE 1 says each array member
    // is followed by its own descendants before the next member. The source tree is therefore:
    // Launch -> [NextPage -> PrevPage, FirstPage -> LastPage]. All four Named actions are among
    // the four actions permitted by Table 215 and PDF/A-4's action table. Removing Launch should
    // yield exactly that sequence. This distinguishes promotion from merely retaining an
    // unreachable /Next object somewhere in the serialized file.
    let source = Conforming {
        catalog: "/OpenAction 6 0 R".to_owned(),
        objects: vec![
            "<< /Type /Action /S /Launch /F (calc.exe) /Next [7 0 R 9 0 R] >>".to_owned(),
            "<< /Type /Action /S /Named /N /NextPage /Next 8 0 R >>".to_owned(),
            "<< /Type /Action /S /Named /N /PrevPage >>".to_owned(),
            "<< /Type /Action /S /Named /N /FirstPage /Next 10 0 R >>".to_owned(),
            "<< /Type /Action /S /Named /N /LastPage >>".to_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let target = Target::Four(Flavour::Plain);

    // Positive source control: the model reads the specified tree in the order the standard
    // describes, before any conversion takes place.
    let input = Document::open(source.clone()).expect("complete source fixture");
    assert!(
        !input.was_recovered(),
        "fixture has exact cross-reference offsets"
    );
    assert_eq!(
        after_launch(open_action(&input)),
        vec![
            Action::Named(Named::NextPage),
            Action::Named(Named::PrevPage),
            Action::Named(Named::FirstPage),
            Action::Named(Named::LastPage),
        ],
        "the source control establishes the forbidden parent and both ordered subtrees"
    );

    let (report, output) = convert(&source, target, behaviour_authorised());
    assert_eq!(
        decision(&report, "actions/no-launch-multimedia-or-form-actions"),
        Decision::Authorised {
            losses: Losses::one(Loss::InteractiveBehaviour),
            rewrite: Rewrite::ForbiddenActionRemoved,
        }
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let output = Document::open(output).expect("converted output is a complete PDF");
    assert!(
        !output.was_recovered(),
        "converter wrote exact cross-reference offsets"
    );
    assert_eq!(
        open_action(&output),
        vec![
            Action::Named(Named::NextPage),
            Action::Named(Named::PrevPage),
            Action::Named(Named::FirstPage),
            Action::Named(Named::LastPage),
        ],
        "the reachable action tree preserves parent-first, subtree-before-next-sibling order"
    );
    assert_eq!(
        conversion(&report).removed_actions.len(),
        1,
        "the only reported removed action is the Launch parent"
    );
}

#[test]
fn a_shared_survivor_with_two_required_tails_refuses_instead_of_splitting_the_action() {
    // The catalog OpenAction and an outline item's /A are separate action positions. Each
    // forbidden Launch promotes the same indirect NextPage action, but leaves a different
    // permitted sibling after it. Rewriting object 8 would require two /Next values. ADR 1175
    // says not to invent a cloned action dictionary, so the converter must refuse this conflict.
    let source = Conforming {
        catalog: "/OpenAction 6 0 R /Outlines 11 0 R".to_owned(),
        objects: vec![
            "<< /Type /Action /S /Launch /F (first.exe) /Next [8 0 R 9 0 R] >>".to_owned(),
            "<< /Type /Action /S /Launch /F (second.exe) /Next [8 0 R 10 0 R] >>".to_owned(),
            "<< /Type /Action /S /Named /N /NextPage >>".to_owned(),
            "<< /Type /Action /S /Named /N /PrevPage >>".to_owned(),
            "<< /Type /Action /S /Named /N /LastPage >>".to_owned(),
            "<< /Type /Outlines /First 12 0 R /Last 12 0 R /Count 1 >>".to_owned(),
            "<< /Title (item) /Parent 11 0 R /A 7 0 R >>".to_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let target = Target::Four(Flavour::Plain);
    let input = Document::open(source.clone()).expect("complete source fixture");
    assert!(
        !input.was_recovered(),
        "fixture has exact cross-reference offsets"
    );
    let catalog = input.catalog().expect("source catalog");
    let open = input.get_key(&catalog, "OpenAction");
    assert_eq!(
        after_launch(read(&input, &open)),
        vec![
            Action::Named(Named::NextPage),
            Action::Named(Named::PrevPage),
        ],
        "first position needs NextPage -> PrevPage"
    );
    let outlines = input.get_key(&catalog, "Outlines");
    let outline = input.get_key(outlines.as_dict().expect("outlines dictionary"), "First");
    let outline = input.resolve(&outline);
    let second = input.get_key(outline.as_dict().expect("outline item dictionary"), "A");
    assert_eq!(
        after_launch(read(&input, &second)),
        vec![
            Action::Named(Named::NextPage),
            Action::Named(Named::LastPage),
        ],
        "second position needs the same NextPage -> LastPage"
    );

    let (report, output) = convert(&source, target, behaviour_authorised());
    assert!(
        output.is_none(),
        "conflicting shared-object edits are not written"
    );
    assert!(
        matches!(
            decision(&report, "actions/no-launch-multimedia-or-form-actions"),
            Decision::Refused(Because::NotBuiltYet(reason))
                if reason.contains("different actions to follow")
        ),
        "the report should explain the different tails: {:?}",
        decision(&report, "actions/no-launch-multimedia-or-form-actions")
    );
}

fn after_launch(actions: Vec<Action>) -> Vec<Action> {
    let mut actions = actions.into_iter();
    assert!(
        matches!(actions.next(), Some(Action::Launch(_))),
        "source begins with a file-naming Launch action"
    );
    actions.collect()
}
