//! What a native host has to derive from `viewer-core`'s answers, checked without a display.
//!
//! **The whole of what a workspace test suite can see of a native host is this.** Building a
//! `GtkApplicationWindow` or a `QMainWindow` needs a display, and a test that skipped itself when
//! there was none would be worse than no test — so the decisions live in this toolkit-free crate
//! and the widget construction lives in the hosts, which only wire them up. These are the
//! decisions, and both hosts are built on exactly these. The widgets are checked by running each
//! program under `Xvfb` and reading its pixels back, which is ADR 0126's recipe and is recorded in
//! ADRs 0244 and 0246.

#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::{Path, PathBuf};

use pdf_model::form::{Choice, ChoiceControl, Control, TextControl};
use viewer_core::{Answer, Command, DocumentId, Extraction, Query, Viewer};
use viewer_host::{
    ControlKind, ImportRefusal, PanelRow, RowAction, attachment_rows, control_kind, layer_rows,
    may_write_extracted, outline_rows, resolve_import,
};

/// The identity these tests give the one document they open.
const DOCUMENT: DocumentId = DocumentId(1);

/// A document committed in `doc/`, which every checkout has once the archive is unpacked.
fn specification_bytes() -> Vec<u8> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    std::fs::read(&path).unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()))
}

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// A viewer with the document open and its events drained.
fn opened(bytes: Vec<u8>) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

/// Every row of a tree, depth first, which is what a platform tree shows when it is open.
fn flattened(rows: &[PanelRow]) -> Vec<&PanelRow> {
    let mut flat = Vec::new();
    let mut stack: Vec<&PanelRow> = rows.iter().rev().collect();
    while let Some(row) = stack.pop() {
        flat.push(row);
        stack.extend(row.children.iter().rev());
    }
    flat
}

#[test]
fn the_outline_becomes_a_tree_whose_every_row_names_an_object_to_activate() {
    // ISO 32000-2 §12.3.3: "[c]licking the text of any visible item activates the item, causing
    // the interactive PDF processor to jump to a destination or trigger an action associated with
    // the item." Which of the two it is belongs to the document, so a row carries the *object*
    // and never a page number — the property this checks is that every row has one.
    let viewer = opened(specification_bytes());
    let Answer::Outline(outline) = viewer.query(Query::Outline) else {
        panic!("the note has a §12.3.3 outline");
    };
    let rows = outline_rows(&outline);
    let flat = flattened(&rows);
    assert!(
        flat.len() >= 5,
        "the note's outline has rows to show, not {}",
        flat.len()
    );
    for row in &flat {
        assert!(
            !row.label.is_empty(),
            "Table 151's /Title is what a row says"
        );
        assert!(
            matches!(row.action, RowAction::Activate(_)),
            "every outline row activates an object: {row:?}"
        );
    }
    // The tree is a tree: the flattening found more rows than there are at the top level, which
    // is what `TreeListModel`'s child models are built from.
    assert!(
        flat.len() > rows.len(),
        "the outline nests, {} rows under {} top-level ones",
        flat.len(),
        rows.len()
    );
}

#[test]
fn a_layer_row_carries_the_switch_and_a_heading_carries_none() {
    // §8.11.4.3 defines two shapes and says what each means: a nested array *with* a leading text
    // string is a heading over related groups, and one *without* is nesting of content. A panel
    // that drew both the same way would tell a person that a heading is a layer.
    let Some(bytes) = corpus_bytes("visibility_expressions.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let viewer = opened(bytes);
    let Answer::Layers(layers) = viewer.query(Query::Layers) else {
        panic!("this document states an /Order");
    };
    let rows = layer_rows(&layers);
    let flat = flattened(&rows);
    let switches = flat
        .iter()
        .filter(|row| matches!(row.action, RowAction::Toggle { .. }))
        .count();
    assert_eq!(switches, 3, "its three groups each get a switch");
    for row in &flat {
        match &row.action {
            RowAction::Toggle { .. } => {
                assert!(!row.label.is_empty(), "Table 96's /Name names the switch");
            }
            RowAction::Inert => {}
            other => panic!("a layer tree holds switches and headings, not {other:?}"),
        }
    }
}

#[test]
fn an_attachment_row_carries_the_key_the_extraction_names_and_shows_the_file_name() {
    // §7.11.4.1: the `/EmbeddedFiles` tree maps §7.7.4's name strings to file specifications, and
    // its NOTE says that before PDF 1.6 "it was necessary to identify document-level embedded
    // files by the name string provided in the name dictionary" — so the key need not be a file
    // name. What a person is shown is Table 43's `/UF`, and what `Command::Extract` carries is the
    // key. Two strings, and a host that used one for both would be wrong on some document.
    //
    // The first half was a quotation until the four-hundred-and-twenty-ninth session, of a
    // sentence Errata Collection 3 struck out with the two bullets around it (Issue #481). The
    // NOTE quoted here is what survives, and it is the half this test rests on.
    let Some(bytes) = corpus_bytes("attachment.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = opened(bytes);
    let Answer::Attachments(files) = viewer.query(Query::Attachments) else {
        panic!("the fixture embeds a file");
    };
    let rows = attachment_rows(&files);
    let [row] = rows.as_slice() else {
        panic!("one embedded file, not {}", rows.len());
    };
    let RowAction::Extract { name } = &row.action else {
        panic!("an attachment row extracts: {row:?}");
    };
    // The row's action is what the viewer answers to, which is the whole claim: the mapping
    // produced something `viewer-core` accepts, rather than something that merely looks right.
    let events: Vec<_> = viewer
        .handle(Command::Extract { name: name.clone() })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, viewer_core::Event::Extracted { .. })),
        "the key the row carries extracts the file: {events:?}"
    );
}

#[test]
fn a_real_forms_fields_each_decide_a_control() {
    // ADR 0235's answer, from a host's side: `Query::Fields` carries enough for a native control
    // to be *chosen* without reaching into `viewer-ui` or re-deriving anything. This asserts the
    // property rather than a census — every field on the page decides one control, and the ones
    // §12.7.5 gives no control to are the two the clause says have none.
    let Some(bytes) = corpus_bytes("160F-2019.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let viewer = opened(bytes);
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("this document has an /AcroForm");
    };
    assert!(
        fields.len() > 10,
        "the form has fields on its first page, not {}",
        fields.len()
    );
    let mut entries = 0_usize;
    for field in &fields {
        match control_kind(&field.control) {
            ControlKind::Entry { .. } => entries = entries.saturating_add(1),
            ControlKind::Check { .. }
            | ControlKind::Radio { .. }
            | ControlKind::Push
            | ControlKind::Combo { .. }
            | ControlKind::List { .. }
            | ControlKind::Signature
            | ControlKind::Unstated => {}
        }
        // §12.7.5.2.3 makes a check box's value a name the file invented, and no host could guess
        // it — which is the entry ADR 0235 found the audit had missed.
        if matches!(field.control, Control::CheckBox { .. }) {
            assert!(
                field.widgets.iter().any(|widget| widget.on_state.is_some()),
                "a check box names the state that turns it on: {:?}",
                field.name
            );
        }
    }
    assert!(entries > 0, "a form of text fields produces text entries");
}

#[test]
fn a_password_field_asks_for_the_platforms_secure_control() {
    // Table 231 bit 14: "intended for entering a secure password that should not be echoed
    // visibly to the screen". The flag decides the *control* — a `GtkPasswordEntry`, or a
    // `QLineEdit` in `QLineEdit::Password` echo mode — and
    // it is also the one control whose value this host does not read back, because
    // `Answer::Field` answers a password field with bullets rather than with its characters.
    let control = Control::Text(TextControl {
        password: true,
        max_len: Some(8),
        ..TextControl::default()
    });
    assert_eq!(
        control_kind(&control),
        ControlKind::Entry {
            multiline: false,
            password: true,
            max_len: Some(8),
        }
    );
}

#[test]
fn table_233_bit_18_decides_a_drop_down_against_a_list() {
    // "If set, the field is a combo box; if clear, the field is a list box." Two controls, and
    // §12.7.5.4 makes `/V` "the second of the two array elements" for both — so the options a
    // host shows are the labels and the export values stay out of the interface.
    let options = vec![
        Choice {
            export: Some("1".to_owned()),
            label: "One".to_owned(),
        },
        Choice {
            export: Some("2".to_owned()),
            label: "Two".to_owned(),
        },
    ];
    let combo = Control::Choice(ChoiceControl {
        combo: true,
        options: options.clone(),
        selected: vec![1],
        ..ChoiceControl::default()
    });
    assert_eq!(
        control_kind(&combo),
        ControlKind::Combo {
            options: vec!["One".to_owned(), "Two".to_owned()],
            selected: Some(1),
            editable: false,
        }
    );
    let list = Control::Choice(ChoiceControl {
        combo: false,
        options,
        selected: vec![0],
        multi_select: true,
        ..ChoiceControl::default()
    });
    assert_eq!(
        control_kind(&list),
        ControlKind::List {
            options: vec!["One".to_owned(), "Two".to_owned()],
            selected: vec![0],
            multi: true,
            top: 0,
        }
    );
}

/// Table 233 bit 19 decides whether a keyboard may put characters into a control at all.
///
/// ISO 32000-2 §12.7.5.4: if the bit is set the combo box "shall include an editable text box as
/// well as a drop-down list", and if it is clear it shall include only a drop-down list.
///
/// **Every variant is asked, and the arm that answers is exhaustive over the enumeration**, so a
/// ninth control kind fails to compile in `ControlKind::takes_typed_characters` rather than
/// silently taking the default of whichever arm it happened to fall into. That is `Key::ALL`'s and
/// `Tab::ALL`'s mechanism (ADRs 0526, 0564) applied to the third thing a host builds.
///
/// It is a *host* question because both halves of the flag are about what the reader is shown:
/// the two native hosts obey it by choosing a widget, and the tier-2 host — which draws the page's
/// own appearance and so has no widget to be constrained by — obeys it by asking this. Before ADR
/// 0596 it asked nothing and a person could type a value into a drop-down that states only three.
#[test]
fn table_233_bit_19_decides_whether_a_control_takes_typed_characters() {
    let options = vec![Choice {
        export: None,
        label: "One".to_owned(),
    }];
    let combo = |editable: bool| {
        control_kind(&Control::Choice(ChoiceControl {
            combo: true,
            options: options.clone(),
            editable,
            ..ChoiceControl::default()
        }))
    };
    assert!(
        combo(true).takes_typed_characters(),
        "bit 19 set: \"an editable text box as well as a drop-down list\""
    );
    assert!(
        !combo(false).takes_typed_characters(),
        "bit 19 clear: \"only a drop-down list\""
    );
    // §12.7.5.4's list box, whose value "identifies the item or items currently selected".
    assert!(
        !control_kind(&Control::Choice(ChoiceControl {
            combo: false,
            options,
            ..ChoiceControl::default()
        }))
        .takes_typed_characters()
    );
    // §12.7.5.3, which is the one control that takes characters without a flag saying so.
    assert!(
        control_kind(&Control::Text(TextControl::default())).takes_typed_characters(),
        "§12.7.5.3's text field is what §12.7.4.3 lays a typed value out for"
    );
    // §12.7.5.2's three buttons select an appearance state, and §12.7.5.5's signature holds a
    // dictionary. None of the four is text a person types.
    for control in [
        Control::PushButton,
        Control::CheckBox { on: false },
        Control::RadioButton {
            on: false,
            no_toggle_to_off: false,
            in_unison: false,
        },
        Control::Signature,
        Control::Unstated,
    ] {
        assert!(
            !control_kind(&control).takes_typed_characters(),
            "{control:?} has no text to type into"
        );
    }
}

/// Table 234's `/TI` reaches the control a host builds, and it is not the selection.
///
/// "(Optional; PDF 1.5) For scrollable list boxes, the top index (index in the Opt array) of the
/// first option visible in the list." `pdf-model` has read the entry since the
/// three-hundred-and-ninety-eighth session and the page's own appearance obeys it (ADR 0407); the
/// mapping a host builds its list from dropped it, so the control started at row 0 over a picture
/// that started somewhere else.
#[test]
fn table_234s_top_index_says_where_a_hosts_list_starts() {
    let list = Control::Choice(ChoiceControl {
        combo: false,
        options: vec![
            Choice {
                export: None,
                label: "One".to_owned(),
            },
            Choice {
                export: None,
                label: "Two".to_owned(),
            },
            Choice {
                export: None,
                label: "Three".to_owned(),
            },
        ],
        // The clause makes these two different questions: the second option is selected and the
        // third is the first one visible, which a control that read one of them for the other
        // would show as the same row.
        selected: vec![1],
        top: 2,
        ..ChoiceControl::default()
    });
    let ControlKind::List { top, selected, .. } = control_kind(&list) else {
        unreachable!("Table 233 bit 18 is clear, so this is a list box");
    };
    assert_eq!(top, 2, "Table 234's /TI");
    assert_eq!(selected, vec![1], "and the value, which is not it");
}

#[test]
fn the_import_policy_admits_a_neighbour_and_refuses_everything_else() {
    // §12.7.6.4 makes performing an import-data action a `shall` and says nothing about which
    // files a document may name, because that is a property of the processor. This is the
    // narrowest policy that still performs the action, and the refusals are the point: a name is
    // checked as a *path* rather than as a string, so a separator this platform recognises and
    // this program does not cannot slip through.
    let directory = Path::new("/documents");
    assert_eq!(
        resolve_import(Some(directory), "data.fdf"),
        Ok(PathBuf::from("/documents/data.fdf"))
    );
    for hostile in ["../data.fdf", "/etc/passwd", "sub/data.fdf", "..", ""] {
        assert!(
            matches!(
                resolve_import(Some(directory), hostile),
                Err(ImportRefusal::NotAPlainName { .. })
            ),
            "{hostile} is not a plain file name beside the document"
        );
    }
    assert_eq!(
        resolve_import(None, "data.fdf"),
        Err(ImportRefusal::NoDirectory),
        "a document with no directory has no neighbourhood to resolve against"
    );
}

/// ISO 32000-2 §O.2.1, Table Annex O.3's `ef`:
///
/// > Security should be strongly considered when opening an embedded file. When opening a file
/// > that is not from a trusted source, a PDF processor may choose to prompt the user or even
/// > prevent opening of the file.
///
/// The clause offers two answers and this project takes the second, because none of the three
/// hosts has a dialogue to prompt with — so the rule that has to hold is that the *provenance*
/// decides and nothing else does. A click still writes the file; a URI's fragment does not, and
/// says so. Without this the four-hundred-and-seventy-fifth session's `ef` would have made
/// `pdf-viewer report.pdf#ef=x` write a file to disk with nobody having pressed anything.
#[test]
fn a_uris_embedded_file_is_not_written_and_a_persons_is() {
    assert_eq!(may_write_extracted(Extraction::Asked), Ok(()));
    let refused = may_write_extracted(Extraction::Fragment)
        .expect_err("a URI's fragment is not a person asking");
    assert!(refused.contains("was not written to disk"), "{refused}");
    assert!(
        refused.contains("§O.2.1"),
        "and it cites the clause: {refused}"
    );
}
