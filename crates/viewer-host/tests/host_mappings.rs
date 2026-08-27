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
    Clicked, ControlKind, ImportRefusal, PanelRow, RowAction, attachment_rows, collection_rows,
    control_kind, layer_rows, may_write_extracted, outline_rows, resolve_import,
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
    // §7.11.4.1: the `/EmbeddedFiles` tree maps §7.7.4's strings to file specifications, and its
    // NOTE says that before PDF 1.6 a document-level embedded file had to be identified by the
    // string the name dictionary filed it under — so the key need not be a file name. What a
    // person is shown is Table 43's `/UF`, and what `Command::Extract` carries is the key. Two
    // strings, and a host that used one for both would be wrong on some document. The NOTE is
    // prose here rather than a quotation for Issue #214's reason, which
    // `pdf_model::attachment::Attachment::name` carries.
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

/// The way out of a refusal is a word, and the sentence that offers it has to name that word.
///
/// **This is one half of a two-part check and the weaker half**, which is worth saying: it holds
/// the *sentence* to the constant, and each host binary holds its own *parser* to the same
/// constant. Neither alone would have caught what ADR 0604 found — three windows saying
/// `--ignore-restrictions` while two of them answered "not an option this program has" to exactly
/// that word — because the sentences agreed with each other perfectly and with nothing else.
#[test]
fn the_refusal_names_the_word_that_turns_the_restrictions_off() {
    let said = viewer_host::refused(&["the document forbids extracting text".to_owned()]);
    assert!(
        said.contains("the document forbids extracting text"),
        "{said}"
    );
    assert!(
        said.contains(viewer_host::IGNORE_RESTRICTIONS),
        "the way out has to be in the sentence: {said}"
    );
}

/// The middle of a widget, which is the point an assistive technology's click resolves to.
///
/// `viewer_accessibility::Act::Click` takes the *node's* centre and a `Form` element's place is its
/// annotation's `/Rect` (§14.7.5.3, ADR 0338), so this is the same arithmetic that reaches
/// [`viewer_host::clicked`] over a real AT-SPI bus — computed here rather than borrowed, so that
/// the test cannot be satisfied by a mirror of the code it checks.
fn middle(quad: [f32; 8]) -> (f32, f32) {
    let x = (quad[0] + quad[2] + quad[4] + quad[6]) / 4.0;
    let y = (quad[1] + quad[3] + quad[5] + quad[7]) / 4.0;
    (x, y)
}

/// Every widget of a page, in reading order down the page and then across it.
///
/// The order is the *screen's* rather than `/Annots`', because what the assertions below name is
/// what a person sees: `annotation-button-widget.pdf` labels each of its rows in its own `/TU`.
fn widgets_down_the_page(viewer: &Viewer) -> Vec<(f32, f32)> {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("a viewer with a document open answers Query::Fields");
    };
    let mut points: Vec<(f32, f32)> = fields
        .iter()
        .flat_map(|field| field.widgets.iter().map(|widget| middle(widget.quad)))
        .collect();
    points.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
    points
}

/// §12.7.5.2's rule over the nine button widgets of a document that labels its own answers.
///
/// **The document is the witness `doc/verify.md` names for §14.8.4.7.2's controls**, and it is the
/// one ADR 0623 measured the delegated click's silence on: nine `Form` elements, each beside a
/// paragraph reading "Check box, checked", "Radio button, unselected" and so on. What is asserted
/// here is what the *clause* makes of a click on each, in the order they appear down the page:
///
/// - three check boxes — `/V /Off` with an on state named `1`, `/V /1`, and one whose `/Ff` sets
///   Table 227 bit 1;
/// - two radio button fields whose `/Ff` is `49152` (Table 229 bits 15 and 16), one with `/V /1`
///   and one with `/V /Off`, two widgets apiece;
/// - one radio button field whose `/Ff` is `49153`, which is those two bits and Table 227 bit 1.
///
/// So **five of the nine widgets toggle and four are refused by name**, and every one of the four
/// is a sentence the standard writes: three are Table 227's read-only flag and the fourth is Table
/// 229 bit 15 on the one button of the set that is already on. A host that toggled nine would be
/// disobeying the document; a host that toggled none is what both native windows did until
/// ADR 0630.
#[test]
fn a_click_on_each_of_nine_button_widgets_is_what_the_clause_makes_of_it() {
    let Some(bytes) = corpus_bytes("annotation-button-widget.pdf") else {
        return;
    };
    let viewer = opened(bytes);
    let points = widgets_down_the_page(&viewer);
    assert_eq!(points.len(), 9, "the document states nine button widgets");
    let outcomes: Vec<Clicked> = points
        .iter()
        .map(|at| viewer_host::clicked(&viewer, *at))
        .collect();
    let named = |outcome: &Clicked| match outcome {
        Clicked::Toggles { value, .. } => format!("toggles to {value}"),
        Clicked::ReadOnly { .. } => "read-only".to_owned(),
        Clicked::Stays { .. } => "stays".to_owned(),
        Clicked::Unnamed { .. } => "unnamed".to_owned(),
        Clicked::Pointed { .. } => "pointed".to_owned(),
        Clicked::Aimed { .. } => "aimed".to_owned(),
        Clicked::Page => "page".to_owned(),
    };
    let said: Vec<String> = outcomes.iter().map(named).collect();
    assert_eq!(
        said,
        vec![
            // "Check box, unchecked": `/V /Off`, and `/AP /N` names the on state `1`.
            "toggles to 1",
            // "Check box, checked": §12.7.5.2.3's off state, which the clause names.
            "toggles to Off",
            // "Check box, read-only": `/Ff 1`, Table 227 bit 1.
            "read-only",
            // The `/V /Off` radio field's two widgets, neither on: either may be turned on.
            "toggles to 0",
            "toggles to 1",
            // The `/V /1` radio field. The first widget answers to `0` and is off; the second
            // answers to `1` and is *on*, and Table 229 bit 15 is set — "selecting the currently
            // selected button has no effect".
            "toggles to 0",
            "stays",
            // The `/Ff 49153` radio field: the same two bits with Table 227 bit 1 beside them.
            "read-only",
            "read-only",
        ],
        "§12.7.5.2 over the nine widgets, down the page"
    );
    // Every refusal names the field §14.9.3 says a user interface shall name, and cites a clause.
    for outcome in &outcomes {
        let Some(said) = outcome.note(true) else {
            continue;
        };
        assert!(
            said.contains("Table 227") || said.contains("Table 229") || said.contains("§12.7.5"),
            "a refusal cites what refused it: {said}"
        );
    }
}

/// The edit a click decides on is one the viewer carries out, so the next click sees the new state.
///
/// **This is the half a decision function cannot assert about itself.** ADR 0623 measured a host
/// whose clicks answered `true` and changed nothing; what makes this one different is that the
/// value goes into the field and comes back out of `Query::Fields`, which is the same answer the
/// control on the screen is written back from.
#[test]
fn the_value_a_click_decides_on_reaches_the_field_and_comes_back() {
    let Some(bytes) = corpus_bytes("annotation-button-widget.pdf") else {
        return;
    };
    let mut viewer = opened(bytes);
    let at = widgets_down_the_page(&viewer)[0];
    let Clicked::Toggles { name, value } = viewer_host::clicked(&viewer, at) else {
        panic!("the first widget is a check box that is off, with an on state named");
    };
    viewer
        .handle(Command::Edit(viewer_core::Edit::SetField {
            field: name.qualified,
            value: viewer_core::Entered::Text(value),
        }))
        .for_each(drop);
    // §12.7.5.2.3: "[t]he value of the V key shall also be the value of the AS key", so the widget
    // is now in the state the click named — and the *second* click on it is therefore the other
    // one, which is what a person expects of a check box and what nine `DoAction`s used to miss.
    assert_eq!(
        viewer_host::clicked(&viewer, at),
        Clicked::Toggles {
            name: pdf_model::view::FieldName {
                qualified: match viewer.query(Query::FieldAt(at)) {
                    Answer::Field { name, .. } => name.qualified,
                    _ => panic!("the point is on a field"),
                },
                alternative: Some("Check box, unchecked".to_owned()),
            },
            value: "Off".to_owned(),
        }
    );
}

/// A document with a `/Collection`, its two files, its schema and its one folder.
///
/// Written here because **not one of the 974 pdf.js documents states a `/Collection`** and the one
/// that does is under `doc/corpora/`, which is optional in the strong sense: a test that skipped
/// itself where that submodule is absent would leave §12.3.5's `shall` ungated on every machine
/// and on CI. Trap 8's converse — a corpus finds what documents contain, not what the
/// specification says.
///
/// `initial` is Table 153's `/D`, `folders` its `/Folders` and `folder_id` Table 159's `/ID` on
/// the one folder, all written in so that the four outcomes, an absent tree and a key naming a
/// folder nobody wrote can each be varied by the tests below.
fn a_collection(initial: &str, folders: &str, folder_id: u32) -> Vec<u8> {
    use std::fmt::Write as _;

    let objects: [String; 10] = [
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names \
         [(<3>report.pdf) 4 0 R (readme.txt) 6 0 R] >> >> /Collection 8 0 R >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_owned(),
        "<< /Type /Filespec /UF (report.pdf) /Desc (the third chapter) /EF << /UF 5 0 R >> >>"
            .to_owned(),
        "<< /Length 5 /Params << /Size 5 >> >>\nstream\nhello\nendstream".to_owned(),
        "<< /Type /Filespec /UF (readme.txt) /Desc (read me) /EF << /UF 7 0 R >> >>".to_owned(),
        "<< /Length 5 /Params << /Size 5 >> >>\nstream\nthere\nendstream".to_owned(),
        format!("<< /Type /Collection {initial} /Schema 9 0 R {folders} >>"),
        // Table 155: `/O` orders the columns, `/V` says which are shown at all, and `/N` is the
        // name a person reads. `HD` states the *lowest* `/O` and `/V false`, so a panel obeying
        // `/O` alone would put it first.
        "<< /FN << /Subtype /F /N (File) /O 1 /V true >> /ZZ << /Subtype /Desc /N (About) >> \
         /HD << /Subtype /Size /N (Hidden) /O 0 /V false >> >>"
            .to_owned(),
        format!("<< /Type /Folder /ID {folder_id} /Name (Chapters) /Desc (the parts of it) >>"),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
    }
    let at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The two answers a host holds when it fills the files panel.
fn collection_and_files(
    bytes: Vec<u8>,
) -> (
    pdf_model::collection::Collection,
    pdf_model::collection::Initial,
    Vec<pdf_model::attachment::Attachment>,
) {
    let viewer = opened(bytes);
    let Answer::Collection {
        collection,
        initial,
    } = viewer.query(Query::Collection)
    else {
        panic!("the fixture's catalog states a /Collection");
    };
    let Answer::Attachments(files) = viewer.query(Query::Attachments) else {
        panic!("the fixture embeds two files");
    };
    (collection, initial, files)
}

/// §12.3.5: a collection is the same files *arranged*, and a native host now shows the arrangement.
///
/// > If this dictionary is present in a PDF document, the interactive PDF processor shall present
/// > the document as a portable collection.
///
/// The `shall` is addressed to a viewer. `viewer_core::Query::Collection` has carried Table 153
/// whole since the three-hundred-and-fifty-second session and neither native host asked it, so
/// both drew a collection as `attachment_rows`' flat list — which this test asserts alongside the
/// new shape, because the difference between the two *is* the defect that was closed.
#[test]
fn a_collection_becomes_a_folder_tree_with_the_schemas_columns() {
    let (collection, initial, files) =
        collection_and_files(a_collection("/D (<3>report.pdf)", "/Folders 10 0 R", 3));

    // What both native hosts showed before: two files, side by side, and no folder anywhere.
    let flat = attachment_rows(&files);
    assert_eq!(flat.len(), 2, "the /EmbeddedFiles tree has two entries");
    assert!(
        flat.iter().all(|row| row.children.is_empty()),
        "a flat list has no arrangement in it: {flat:?}"
    );

    let rows = collection_rows(&collection, &initial, &files);

    // §12.3.5.2: a key that does not name a folder "shall be treated as associated with the root
    // folder", so `readme.txt` is a top-level row — above the folders, where the root's own files
    // belong — and the folder is the other one.
    assert_eq!(rows.len(), 2, "one rootless file and one folder: {rows:?}");
    assert_eq!(rows[0].label, "readme.txt");
    assert_eq!(
        rows[0].action,
        RowAction::Extract {
            name: "readme.txt".to_owned()
        }
    );
    assert_eq!(rows[1].label, "Chapters", "Table 159's /Name");
    assert_eq!(rows[1].detail.as_deref(), Some("the parts of it"), "/Desc");
    assert!(
        rows[1].expanded,
        "a folder tree arrives open or says nothing"
    );
    assert_eq!(
        rows[1].action,
        RowAction::Inert,
        "a folder has no bytes to take out, so its row acts through its children"
    );

    // `<3>report.pdf` is *report.pdf* in folder 3, and the row still carries the **tree's** key,
    // folder number and all, because that is what `Command::Extract` names a file by.
    assert_eq!(rows[1].children.len(), 1);
    let inside = &rows[1].children[0];
    assert_eq!(inside.label, "report.pdf", "Table 43's /UF, not the key");
    assert_eq!(
        inside.action,
        RowAction::Extract {
            name: "<3>report.pdf".to_owned()
        }
    );

    // Table 155's `/V` decides which columns are shown and `/O` in what order. `HD` states the
    // lowest `/O` and is hidden, so a panel reading `/O` alone would have put it first.
    assert_eq!(
        inside.detail.as_deref(),
        Some("File: report.pdf  ·  About: the third chapter"),
        "the visible fields, /O before the one that states none"
    );
    assert!(
        flattened(&rows)
            .iter()
            .all(|row| !row.detail.as_deref().unwrap_or("").contains("Hidden")),
        "a field the schema hides is a field no row draws"
    );

    // §12.3.5.1's `/D` names one of them, and exactly one row is set apart.
    assert!(inside.emphasis, "Table 153's /D names <3>report.pdf");
    assert_eq!(
        flattened(&rows).iter().filter(|row| row.emphasis).count(),
        1
    );
}

/// §12.3.5.1's remaining `/D` outcomes, as a panel over a page obeys them.
///
/// Table 153's `/D` "identif[ies] an entry in the `EmbeddedFiles` name tree, determining the
/// document that shall be initially presented in the user interface", and the clause states three
/// fallbacks as `shall`s. The clause states no *appearance*, so what is checkable is which row is
/// marked — and that the container case marks none, because the container is what is already on
/// the screen.
#[test]
fn the_document_a_collection_opens_on_is_the_row_set_apart() {
    let marked = |initial: &str, folders: &str| {
        let (collection, initial, files) = collection_and_files(a_collection(initial, folders, 3));
        let rows = collection_rows(&collection, &initial, &files);
        flattened(&rows)
            .iter()
            .filter(|row| row.emphasis)
            .map(|row| row.label.clone())
            .collect::<Vec<_>>()
    };

    // "If the D entry is missing or is not a valid byte string, the initial document shall be the
    // one that contains the collection dictionary" — which is on the screen, so no row is marked.
    assert!(marked("", "/Folders 10 0 R").is_empty());
    assert!(
        marked("/D /report", "/Folders 10 0 R").is_empty(),
        "a name is not a byte string"
    );

    // "the interactive PDF processor shall select the first item from the list of files to display
    // in its user interface" — the first in the order the rows are *shown*, which is the rootless
    // file, not the first entry of the name tree.
    assert_eq!(
        marked("/D (missing.pdf)", "/Folders 10 0 R"),
        ["readme.txt"]
    );

    // "If no folder structure is specified, interactive PDF processors should show all files in
    // the collection in a flat list" — so the order is the name tree's own, `<3>report.pdf` first,
    // and neither file is dropped for naming a folder the document never wrote.
    assert_eq!(marked("/D (missing.pdf)", ""), ["report.pdf"]);
    assert_eq!(marked("/D (readme.txt)", ""), ["readme.txt"]);
}

/// A collection with nothing in it says so rather than drawing an empty panel.
///
/// §12.3.5.1's fourth outcome is "an empty preview window", and a panel that drew nothing for it
/// would be indistinguishable from one this program failed to fill — which is the whole reason
/// `PanelRow::saying` exists.
#[test]
fn a_collection_holding_no_files_says_so() {
    let collection = pdf_model::collection::Collection::default();
    let rows = collection_rows(&collection, &pdf_model::collection::Initial::Empty, &[]);
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].note,
        "a sentence about the document, not a thing in it"
    );
    assert_eq!(rows[0].action, RowAction::Inert);
}

/// §12.3.5.2: every embedded file is on the screen, however oddly its key is written.
///
/// Two sentences of the clause say so, and this panel obeyed neither until the
/// seven-hundred-and-seventy-second session — a file whose key named a folder the document did not
/// state, and every file of a collection with no `/Folders` at all, were dropped from the list
/// rather than placed. A panel drawing fewer files than the document embeds is the shape trap 5
/// exists for: it looks exactly like a document that embeds fewer files.
#[test]
fn every_embedded_file_is_shown_whatever_its_key_names() {
    let listed = |folders: &str, folder_id: u32| {
        let (collection, initial, files) =
            collection_and_files(a_collection("", folders, folder_id));
        assert_eq!(files.len(), 2, "the fixture embeds two files either way");
        let rows = collection_rows(&collection, &initial, &files);
        let mut names: Vec<String> = flattened(&rows)
            .iter()
            .filter_map(|row| match &row.action {
                RowAction::Extract { name } => Some(name.clone()),
                _ => None,
            })
            .collect();
        names.sort();
        names
    };
    let both = ["<3>report.pdf".to_owned(), "readme.txt".to_owned()];

    // "If no folder structure is specified, interactive PDF processors should show all files in
    // the collection in a flat list."
    assert_eq!(listed("", 3), both);

    // The document states folder 3 and one key names it: the ordinary case.
    assert_eq!(listed("/Folders 10 0 R", 3), both);

    // The document states folder 9 and a key names folder 3, which is the producer contradicting
    // "[t]he value shall correspond to a folder ID". The file is still a member of the structure —
    // "[w]hen folders are used, all files in the EmbeddedFiles name tree … shall be treated as
    // members of the folder structure by an interactive PDF processor" — so it is drawn at the
    // root rather than dropped.
    assert_eq!(listed("/Folders 10 0 R", 9), both);
}
