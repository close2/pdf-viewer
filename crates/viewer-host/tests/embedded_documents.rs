//! ISO 32000-2 §O.2.1's `ef` under the four levels a reader sets over it.
//!
//! Table Annex O.3's row states the requirement and the caution in one breath: the processor opens
//! the file the fragment names, and "a PDF processor may choose to prompt the user or even prevent
//! opening of the file". The
//! levels are that choice given to the reader, read in `viewer_host::may_open_extracted` and set
//! from the restriction menu's third group (ADR 1331). What these hold is the default, each
//! level's answer, the file a person asked for being outside the level, and the menu's rows.

use viewer_core::Extraction;
use viewer_host::{
    Act, ActLevel, EMBEDDED_DOCUMENTS, EmbeddedDocuments, OPENING_EMBEDDED, Restrictions, Row,
    Unpacking, act_chosen, embedded_declined, embedded_documents, may_open_extracted,
};

const NAME: &str = "destination-doc.pdf";

/// `ask` until a person picks another — `doc/questions/A67`'s and `A98`'s argument, one act over.
#[test]
fn a_reader_is_asked_until_they_choose_otherwise() {
    assert_eq!(EmbeddedDocuments::default(), EmbeddedDocuments::Ask);
    let restrictions = Restrictions::new(viewer_core::RestrictionPolicy::default());
    assert_eq!(restrictions.embedded_documents(), EmbeddedDocuments::Ask);
    let Unpacking::Ask(question) = may_open_extracted(
        Extraction::Fragment,
        NAME,
        restrictions.embedded_documents(),
    ) else {
        panic!("the default level asks");
    };
    assert!(question.reasons.contains(NAME), "the name, whole");
    assert!(question.reasons.contains("§O.2.1"));
    assert!(question.choice.contains(OPENING_EMBEDDED));
}

/// Each level answers in its own way, and none of them silently as another.
#[test]
fn each_level_answers_the_fragment_in_its_own_way() {
    let Unpacking::Refuse(why) =
        may_open_extracted(Extraction::Fragment, NAME, EmbeddedDocuments::Refuse)
    else {
        panic!("refuse opens nothing");
    };
    assert!(why.contains("declined") && why.contains(NAME), "{why}");
    assert!(
        matches!(
            may_open_extracted(Extraction::Fragment, NAME, EmbeddedDocuments::Ask),
            Unpacking::Ask(_)
        ),
        "ask puts a question"
    );
    let Unpacking::Warn(said) =
        may_open_extracted(Extraction::Fragment, NAME, EmbeddedDocuments::Warn)
    else {
        panic!("warn opens and says so");
    };
    assert!(
        said.contains("without asking") && said.contains(NAME),
        "{said}"
    );
    assert_eq!(
        may_open_extracted(Extraction::Fragment, NAME, EmbeddedDocuments::Open),
        Unpacking::Open
    );
    let declined = embedded_declined(NAME);
    assert!(declined.contains("declined") && declined.contains(NAME));
}

/// The caution is about a sentence that is frequently not the reader's; a file a person asked
/// for from the files panel is opened at every level.
#[test]
fn a_file_a_person_asked_for_is_not_under_the_level() {
    for level in EmbeddedDocuments::ALL {
        assert_eq!(
            may_open_extracted(Extraction::Asked, NAME, level),
            Unpacking::Open,
            "{}",
            level.as_str()
        );
    }
}

/// The third group gains the act: four levels, `ask` ticked, a pick moves the tick and says so,
/// and the submission's levels are not moved by it.
#[test]
fn the_menu_holds_the_level_and_a_pick_moves_the_tick() {
    let mut restrictions = Restrictions::new(viewer_core::RestrictionPolicy::default());
    let rows = restrictions.rows();
    let heading = rows
        .iter()
        .position(|row| matches!(row, Row::Act { label, .. } if *label == OPENING_EMBEDDED))
        .unwrap_or_else(|| panic!("the act has a heading of its own"));
    assert!(
        rows[..heading]
            .iter()
            .any(|row| matches!(row, Row::Machine { .. })),
        "under the machine's group"
    );
    let ticked = |restrictions: &Restrictions| -> Vec<(&'static str, bool)> {
        restrictions
            .rows()
            .into_iter()
            .filter_map(|row| match row {
                Row::ActLevel(entry) if entry.level.act() == Act::OpeningEmbedded => {
                    Some((entry.label, entry.chosen))
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        ticked(&restrictions),
        [
            ("refuse", false),
            ("ask", true),
            ("warn", false),
            ("open", false)
        ]
    );
    for level in EmbeddedDocuments::ALL {
        restrictions.set(ActLevel::EmbeddedDocuments(level));
        assert_eq!(restrictions.embedded_documents(), level);
        assert_eq!(
            ticked(&restrictions)
                .iter()
                .filter(|(_, chosen)| *chosen)
                .map(|(label, _)| *label)
                .collect::<Vec<_>>(),
            [level.as_str()]
        );
    }
    assert_eq!(
        restrictions.submissions(),
        viewer_host::Submissions::Ask,
        "one act's pick leaves the other's level where it was"
    );
    assert_eq!(
        act_chosen(ActLevel::EmbeddedDocuments(EmbeddedDocuments::Warn)),
        "opening an embedded document is now warn in this window"
    );
    assert_eq!(
        restrictions.headings().len(),
        3,
        "still three groups: the act joins the third"
    );
}

/// The word a window is started at reads each level and names all four when it reads none.
#[test]
fn the_command_line_word_reads_each_level() {
    for level in EmbeddedDocuments::ALL {
        assert_eq!(embedded_documents(level.as_str()), Ok(level));
    }
    let Err(complaint) = embedded_documents("off") else {
        panic!("off is a restriction's word, not this act's");
    };
    assert!(complaint.starts_with(EMBEDDED_DOCUMENTS), "{complaint}");
    assert!(complaint.contains("refuse, ask, warn, open"), "{complaint}");
    let restrictions = Restrictions::new(viewer_core::RestrictionPolicy::default())
        .with(ActLevel::EmbeddedDocuments(EmbeddedDocuments::Refuse));
    assert_eq!(restrictions.embedded_documents(), EmbeddedDocuments::Refuse);
}
