//! A push button with no appearance dictionary, and an appearance dictionary stating no entry.
//!
//! The expected values come from four places, named on each test: ISO 19005-2 section 6.3.3 and
//! ISO 19005-4 section 6.3.3 (an appearance dictionary of every annotation but the exempt
//! subtypes, `/N` its only key, and a button widget's `/N` a subdictionary — `TechNote 0010`'s
//! A012 a push button's included), ISO 32000-2 §12.7.5.2.2 and §12.7.5.2.3 (a push button retains
//! no value; a check box's state names are its values), Table 229's bit 17 (`/Ff 65536`, the
//! push button), and §12.5.2's Table 166 with §12.5.5's Table 170 (`/AP` optional for a `Link`,
//! `/N` required inside an appearance dictionary).
use super::{
    Because, Conforming, Decision, Flavour, Level, Rewrite, Target, Verdict, conversion, decision,
    holds, to_part_four, to_part_two,
};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

/// The first annotation of the given subtype the written file holds.
fn annotation(written: &Document, subtype: &[u8]) -> Dictionary {
    (1..64)
        .filter_map(|number| written.get(ObjectId::new(number, 0)).as_dict().cloned())
        .find(|dict| {
            written
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|name| name.as_bytes() == subtype)
        })
        .expect("the annotation is written")
}

/// A page holding one push button field's widget that states no appearance dictionary.
fn a_push_button(part_two: bool) -> Vec<u8> {
    let base = if part_two {
        Conforming::part_two()
    } else {
        Conforming::default()
    };
    Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        catalog: "/AcroForm << /Fields [6 0 R] >>".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (Go) /Rect [10 10 90 40] \
             /F 4 /MK << /IF << >> >> >>"
                .to_owned(),
        ],
        ..base
    }
    .build()
}

#[test]
fn a_push_button_with_no_appearance_is_given_one_under_the_state_its_new_as_selects() {
    // §12.7.5.2.2: a push button "retains no permanent value", so the one state a subdictionary
    // holds names nothing the field says, and ISO 19005's section 6.3.3 asks a subdictionary of
    // every button widget. The stream is the construction `doc/questions/A21` allows, reported.
    for (part_two, requirement, target) in [
        (
            false,
            "annotations/appearance-dictionary-present-from-base-standard",
            Target::Four(Flavour::Plain),
        ),
        (
            true,
            "annotations/appearance-dictionary-present",
            Target::Two(Level::B),
        ),
    ] {
        let source = a_push_button(part_two);
        let (report, output) = if part_two {
            to_part_two(&source)
        } else {
            to_part_four(&source)
        };
        assert!(
            matches!(
                decision(&report, requirement),
                Decision::Stated {
                    rewrite: Rewrite::AppearanceDictionary,
                    ..
                }
            ),
            "{requirement}: {:?}",
            decision(&report, requirement)
        );
        assert_eq!(
            conversion(&report).appearances.len(),
            1,
            "and it is reported"
        );
        assert!(
            conversion(&report).settled.is_empty(),
            "the first output stands, so no second pass restated it: {:?}",
            conversion(&report).settled
        );
        let output = output.expect("nothing is lost, so nothing is authorised");
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let written = Document::open(output).expect("the output opens");
        let widget = annotation(&written, b"Widget");
        let state = written.get_key(&widget, "AS");
        let state = state.as_name().expect("an AS is written");
        let appearance = written.get_key(&widget, "AP");
        let appearance = appearance.as_dict().expect("an AP");
        assert_eq!(appearance.len(), 1, "N is its only key");
        let normal = written.get_key(appearance, "N");
        let normal = normal.as_dict().expect("N is a subdictionary");
        assert_eq!(normal.len(), 1, "holding the one stream alone");
        let selected = normal.get_by_name(state).expect("AS selects its one entry");
        assert!(
            matches!(written.resolve(selected), Object::Stream(_)),
            "and the entry is the constructed form"
        );
    }
}

#[test]
fn a_check_box_with_no_appearance_keeps_its_refusal_because_a_state_would_be_its_value() {
    // §12.7.5.2.3: "The V entry … holds a name object representing the check box's appearance
    // state", so a state this conversion named would be a value of the field, and the file
    // states no appearance for it.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        catalog: "/AcroForm << /Fields [6 0 R] >>".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Widget /FT /Btn /T (Box) /V /Yes /AS /Yes \
             /Rect [10 10 90 40] /F 4 >>"
                .to_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    let Decision::Refused(Because::NotBuiltYet(why)) = decision(
        &report,
        "annotations/appearance-dictionary-present-from-base-standard",
    ) else {
        panic!("the check box keeps its refusal");
    };
    assert!(why.contains("check box"), "{why}");
    assert!(output.is_none(), "and no file is written");
}

#[test]
fn an_appearance_dictionary_stating_no_entry_is_taken_away_from_a_link() {
    // Table 170 requires `/N` inside an appearance dictionary, so one stating no entry describes
    // no appearance — what a `Link` with no `/AP` describes, which Table 166 and both parts'
    // section 6.3.3 admit. Taking the key away restates the file (`doc/adr/2752`). `/N 0 0 R`
    // states no entry either: §7.3.10 reads a reference to an undefined object as null, and
    // §7.3.7 treats "[a] dictionary entry whose value is null" "the same as if the entry does
    // not exist".
    for (part_two, target, stated) in [
        (false, Target::Four(Flavour::Plain), "<< >>"),
        (true, Target::Two(Level::B), "<< >>"),
        (false, Target::Four(Flavour::Plain), "<< /N 0 0 R >>"),
        (true, Target::Two(Level::B), "<< /N 0 0 R >>"),
    ] {
        let base = if part_two {
            Conforming::part_two()
        } else {
            Conforming::default()
        };
        let source = Conforming {
            page: "/Annots [6 0 R]".to_owned(),
            objects: vec![format!(
                "<< /Type /Annot /Subtype /Link /Rect [10 10 90 40] /F 4 /Border [0 0 0] \
                     /AP {stated} >>"
            )],
            ..base
        }
        .build();
        let (report, output) = if part_two {
            to_part_two(&source)
        } else {
            to_part_four(&source)
        };
        assert!(
            matches!(
                decision(&report, "annotations/normal-appearance-shape"),
                Decision::Stated {
                    rewrite: Rewrite::NormalAppearanceFromState,
                    ..
                }
            ),
            "{:?}",
            decision(&report, "annotations/normal-appearance-shape")
        );
        let output = output.expect("nothing is lost, so nothing is authorised");
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let written = Document::open(output).expect("the output opens");
        let link = annotation(&written, b"Link");
        assert!(
            written.get_key(&link, "AP").is_null(),
            "the empty AP is gone"
        );
    }
}

#[test]
fn a_square_whose_appearance_dictionary_states_no_entry_is_given_the_one_its_clause_states() {
    // The same empty dictionary on a subtype section 6.3.3 does not exempt: it describes no
    // appearance, so the annotation is one stating none, and §12.5.6.8 says what a square's
    // marks are. The two rows act on one annotation and the output holds `/N` alone.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 /IC [0 0 1] /AP << >> >>"
                .to_owned(),
        ],
        contents: Some((String::new(), b"1 0 0 rg 0 0 10 10 re f".to_vec())),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert!(matches!(
        decision(&report, "annotations/normal-appearance-shape"),
        Decision::Stated {
            rewrite: Rewrite::NormalAppearanceFromState,
            ..
        }
    ));
    let output = output.expect("nothing is lost, so nothing is authorised");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let written = Document::open(output).expect("the output opens");
    let square = annotation(&written, b"Square");
    let appearance = written.get_key(&square, "AP");
    let normal = written.get_key(appearance.as_dict().expect("an AP is written"), "N");
    assert!(
        matches!(normal, Object::Stream(_)),
        "whose N is the constructed form"
    );
}

#[test]
fn an_appearance_dictionary_of_null_entries_is_read_as_stating_no_appearance() {
    // §7.3.7: "A dictionary entry whose value is null … shall be treated the same as if the entry
    // does not exist", and §7.3.10 reads `0 0 R` as null. So a stamp whose `/AP` holds only
    // `/N 0 0 R` states no appearance dictionary for section 6.3.3's first sentence, as an empty
    // one does, and the requirement names it.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Stamp /Rect [10 10 90 90] /F 4 /AP << /N 0 0 R >> >>"
                .to_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let held = holds(&source, Target::Four(Flavour::Plain));
    assert!(
        held.failures().any(|judgement| judgement.id
            == "annotations/appearance-dictionary-present-from-base-standard"),
        "{}",
        held.render()
    );
}
