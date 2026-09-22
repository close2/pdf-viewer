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
use viewer_host::panel;
use viewer_host::policy::{Links, Opening, links, may_open_uri, open_uri, resolve_uri, uri_note};
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
            bytes: bytes.into(),
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
            file_select: false,
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

/// ISO 32000-2 §12.6.4.8, Table 211's `/Base`, on the document that states none:
///
/// > If no base URI is specified, such partial URIs shall be interpreted relative to the location
/// > of the document itself.
///
/// `pdf_model::action` applies the `/Base` a document states and leaves a partial reference
/// partial when it states none, because the location of the document is a fact about this machine
/// and `viewer_core` rule 2 keeps paths out of the core. A host has the path, so the `shall` is
/// carried out here — and so is the refusal beside it, which was four windows' own `println!`
/// until this test's session (ADR 1079).
#[test]
fn a_partial_uri_resolves_against_the_documents_own_location() {
    let document = Path::new("/documents/report.pdf");
    assert_eq!(
        resolve_uri(Some(document), "https://example.invalid/a?b#c"),
        "https://example.invalid/a?b#c",
        "an absolute reference needs no base and is not touched by one"
    );
    assert_eq!(
        resolve_uri(Some(document), "foo.bar.com"),
        "file:///documents/foo.bar.com",
        "a partial reference against the location of the document itself; `pr19449.pdf` is the \
         corpus document that states this one and no /Base"
    );
    // The encoding is the load-bearing half, not decoration: a `#` left as itself would make the
    // *base* carry a fragment, so RFC 3986 section 5.3's merge would take the directory from
    // `/awkward/a` rather than from `/awkward/a#b.pdf` and resolve to the wrong neighbour.
    assert_eq!(
        resolve_uri(Some(Path::new("/awk ward/a#b.pdf")), "next.pdf"),
        "file:///awk%20ward/next.pdf",
    );
    assert_eq!(
        resolve_uri(Some(Path::new("report.pdf")), "next.pdf"),
        "next.pdf",
        "a relative path is no base at all, so the reference stays what the document said"
    );

    // And the verb, which is the one question left once the string is decided.
    let resolved = resolve_uri(Some(document), "https://example.invalid/a");
    let Opening::Ask(question) = may_open_uri(&resolved, Links::default()) else {
        panic!("the default level puts the URI to a person rather than deciding for them");
    };
    assert!(
        question.reasons.contains(&resolved),
        "the URI is the only thing a person can judge this by, so the question carries it whole: \
         {}",
        question.reasons
    );
    let Opening::Refuse(unresolvable) = may_open_uri("next.pdf", Links::Open) else {
        panic!("a partial reference names no resource at any level");
    };
    assert!(
        unresolvable.contains("relative reference"),
        "the refusal names that failure rather than the level: {unresolvable}"
    );
    assert_eq!(
        uri_note(&resolved, Some(&unresolvable)),
        format!(
            "link: declined — {unresolvable}. The document asked for https://example.invalid/a"
        ),
        "a link this reader will not follow still says where it went"
    );
    assert_eq!(
        uri_note(&resolved, None),
        "link: https://example.invalid/a",
        "and a host that opened it says the same URI without the refusal"
    );
}

/// ISO 32000-2 §12.6.4.8's own verb, under `CLAUDE.md` principle 3's four levels:
///
/// > A URI action causes a URI to be resolved.
///
/// The clause introduces the string as one that "identifies (resolves to) a resource on the
/// Internet", so resolving it is reaching that resource — a program this machine would have to
/// start on a string the *document* chose. `CLAUDE.md` says what shape that decision takes: the
/// policy is asked once in a place a host can supply, at one of four levels, and "[a] refusal that
/// cannot become an 'ask' is the thing to avoid" (ADR 1155).
#[test]
fn four_levels_decide_whether_a_link_reaches_this_machine() {
    let uri = "https://example.invalid/a";
    assert_eq!(may_open_uri(uri, Links::Open), Opening::Proceed);
    assert!(
        matches!(may_open_uri(uri, Links::Ask), Opening::Ask(_)),
        "the ask level is a question rather than a verdict"
    );
    let Opening::Warn(warned) = may_open_uri(uri, Links::Warn) else {
        panic!("the warn level opens it and says so");
    };
    assert!(
        warned.contains(viewer_host::URI_HANDLER),
        "the warning names what the URI was handed to; `uri_note` names the URI itself, which is \
         why `link` composes the two: {warned}"
    );
    let Opening::Refuse(refused) = may_open_uri(uri, Links::Refuse) else {
        panic!("the refuse level hands nothing over");
    };
    assert!(
        refused.contains(viewer_host::LINKS),
        "a refusal this reader chose names the word that unchooses it: {refused}"
    );

    // Every level round-trips through the word a person types — the command line and the prompt
    // both spell one, and a word that spelled a different level would send a reader to the wrong
    // end of the scale.
    for level in Links::ALL {
        assert_eq!(links(level.as_str()), Ok(level));
    }
    let complaint = links("off").expect_err("`off` is RESTRICTIONS's word and means the other end");
    for level in Links::ALL {
        assert!(
            complaint.contains(level.as_str()),
            "the complaint names every level this option takes: {complaint}"
        );
    }
}

/// ISO 32000-2 §12.6.4.8's string "identifies (resolves to) a resource on the Internet", so which
/// schemes this machine starts a handler for is a decision of its own.
///
/// **It is taken before the level rather than inside it**, which is what this test holds: a
/// document free to name any scheme would be choosing which of this machine's handlers runs, and a
/// reader who asked for links to open asked for links rather than for that. `file` is the sharp
/// one, because `resolve_uri` produces a `file` URL for every partial reference beside the
/// document — opening one is §12.7.6.4's hazard one clause over, where `read_import` answers it
/// with a directory a person supplied (ADR 1155).
#[test]
fn a_scheme_this_machine_will_not_start_a_handler_for_is_refused_at_every_level() {
    for uri in [
        "file:///documents/next.pdf",
        "ms-msdt:/id",
        "javascript:alert(1)",
    ] {
        for level in Links::ALL {
            let Opening::Refuse(refused) = may_open_uri(uri, level) else {
                panic!("{uri} is not a scheme this reader hands over, {level:?} or not");
            };
            assert!(
                !refused.contains("relative reference"),
                "{uri} states a scheme, so the refusal is about the scheme: {refused}"
            );
        }
        assert!(
            open_uri(uri).is_err(),
            "{uri} is refused by the act as well as by the decision, because that is the last \
             place the guarantee can be made"
        );
    }
    // And the three that are handed over are handed over whatever case they are written in: RFC
    // 3986 section 3.1 makes a scheme case-insensitive, so a `HTTPS:` link is the same link.
    for scheme in viewer_host::LINK_SCHEMES {
        assert_eq!(
            may_open_uri(
                &format!("{}:example.invalid", scheme.to_uppercase()),
                Links::Open
            ),
            Opening::Proceed,
            "{scheme} is one of the three, in any case"
        );
    }
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
/// `quorra report.pdf#ef=x` write a file to disk with nobody having pressed anything.
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

/// `--restrictions` reads a level per operation, composes, and refuses a word it does not know.
///
/// **One parser for three windows**, which is this module's standing argument: the third copy of a
/// decision is where two hosts stop agreeing, and `--ignore-restrictions` is the incident behind
/// it (ADR 0604). The words a person types are the operations' own — `RestrictionPolicy::word` —
/// and the levels are `pdf_model::restriction::Level::as_str`'s, so what is spelled here is what
/// `pdf-transform`, the KIO face and `pdf-fuse` already spell (ADR 1144).
#[test]
fn a_restriction_level_can_be_set_for_one_operation_at_a_time() {
    use pdf_model::restriction::Operation;
    use viewer_core::{RestrictionLevel, RestrictionPolicy};

    let policy = viewer_host::restrictions("copy:ask,annotate:on", RestrictionPolicy::default())
        .expect("two operations and two levels this program has");
    assert_eq!(policy.level(Operation::Extract), RestrictionLevel::Ask);
    assert_eq!(policy.level(Operation::Annotate), RestrictionLevel::On);
    assert_eq!(
        policy.level(Operation::FillInForm),
        RestrictionLevel::Off,
        "an operation the list does not name keeps the level it had"
    );

    // Composing, because two of these on one command line are one policy and not the last one.
    let policy = viewer_host::restrictions("fill:warn", policy).expect("one more");
    assert_eq!(policy.level(Operation::Extract), RestrictionLevel::Ask);
    assert_eq!(policy.level(Operation::FillInForm), RestrictionLevel::Warn);

    // A bare level is all six, which is `pdf-transform`'s own spelling.
    let policy = viewer_host::restrictions("on", policy).expect("a level with no operation");
    for operation in RestrictionPolicy::OPERATIONS {
        assert_eq!(
            policy.level(operation),
            RestrictionLevel::On,
            "{operation:?}"
        );
    }

    // And a word this program does not have is a sentence naming the alternatives, never a guess.
    let complaint = viewer_host::restrictions("copy:maybe", RestrictionPolicy::default())
        .expect_err("`maybe` is not a level");
    assert!(complaint.contains("off, on, ask or warn"), "{complaint}");
    let complaint = viewer_host::restrictions("scribble:on", RestrictionPolicy::default())
        .expect_err("`scribble` is not an operation");
    assert!(complaint.contains("annotate"), "{complaint}");
    assert!(
        viewer_host::restrictions("", RestrictionPolicy::default()).is_err(),
        "a person who typed --restrictions= meant something"
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
        // Table 44's `/Subtype` on the embedded file stream, which is the entry Table 153's
        // `/View T` icon is chosen from (ADR 1215). The second file states none, on purpose.
        "<< /Length 5 /Subtype /application#2Fpdf /Params << /Size 5 >> >>\nstream\nhello\nendstream"
            .to_owned(),
        "<< /Type /Filespec /UF (readme.txt) /Desc (read me) /EF << /UF 7 0 R >> >>".to_owned(),
        "<< /Length 5 /Params << /Size 5 >> >>\nstream\nthere\nendstream".to_owned(),
        format!("<< /Type /Collection {initial} /Schema 9 0 R {folders} >>"),
        // Table 155: `/O` orders the columns, `/V` says which are shown at all, and `/N` is the
        // name a person reads. `HD` states the *lowest* `/O` and `/V false`, so a panel obeying
        // `/O` alone would put it first.
        "<< /FN << /Subtype /F /N (File) /O 1 /V true >> /ZZ << /Subtype /Desc /N (About) >> \
         /SZ << /Subtype /Size /N (Size) /O 2 /V true >> \
         /HD << /Subtype /CompressedSize /N (Hidden) /O 0 /V false >> >>"
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
    Vec<String>,
    Vec<pdf_model::attachment::Attachment>,
) {
    let viewer = opened(bytes);
    let Answer::Collection {
        collection,
        initial,
        order,
    } = viewer.query(Query::Collection)
    else {
        panic!("the fixture's catalog states a /Collection");
    };
    let Answer::Attachments(files) = viewer.query(Query::Attachments) else {
        panic!("the fixture embeds two files");
    };
    (collection, initial, order, files)
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
    let (collection, initial, order, files) =
        collection_and_files(a_collection("/D (<3>report.pdf)", "/Folders 10 0 R", 3));

    // What both native hosts showed before: two files, side by side, and no folder anywhere.
    let flat = attachment_rows(&files);
    assert_eq!(flat.len(), 2, "the /EmbeddedFiles tree has two entries");
    assert!(
        flat.iter().all(|row| row.children.is_empty()),
        "a flat list has no arrangement in it: {flat:?}"
    );

    let rows = collection_rows(&collection, &initial, &order, &files);

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
        Some("File: report.pdf  ·  Size: 5  ·  About: the third chapter"),
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
        let (collection, initial, order, files) =
            collection_and_files(a_collection(initial, folders, 3));
        let rows = collection_rows(&collection, &initial, &order, &files);
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
    let rows = collection_rows(
        &collection,
        &pdf_model::collection::Initial::Empty,
        &[],
        &[],
    );
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].note,
        "a sentence about the document, not a thing in it"
    );
    assert_eq!(rows[0].action, RowAction::Inert);
}

/// §12.3.6's selection rule is asked by the panel, and only an undrawable answer is said out loud.
///
/// The clause is written for a processor that cannot draw everything — "an interactive PDF
/// processor should present the first one it is capable of displaying in the order present in the
/// array" — so conformance is asking that question with this program's own capability, which is
/// `panel::DRAWN_LAYOUTS`. Three cases, because a sentence that fires on every navigator says
/// nothing about this one (trap 11):
///
/// - a navigator naming a layout these rows *are* adds no sentence, whatever it names first;
/// - one naming only layouts this panel cannot draw names them;
/// - and Table 153's `/View T` is the same gap without a navigator.
#[test]
fn a_navigator_is_selected_from_what_this_panel_draws_and_the_rest_is_said_out_loud() {
    use pdf_model::collection::{Layout, Navigator, View};

    let panel = |view: View, layouts: Option<Vec<Layout>>| {
        let collection = pdf_model::collection::Collection {
            view,
            navigator: layouts.map(|layouts| Navigator { layouts }),
            ..pdf_model::collection::Collection::default()
        };
        collection_rows(
            &collection,
            &pdf_model::collection::Initial::Container,
            &[],
            &[],
        )
    };
    let sentence = |rows: &[PanelRow]| {
        rows.iter()
            .find(|row| row.note && row.label.starts_with("This collection asks"))
            .map(|row| row.label.clone())
    };

    // The file prefers a strip of thumbnails and falls back to the tree, which is what a
    // conforming `/Layout` array looks like: something exotic, then one of D, T or H.
    let drawable = panel(
        View::Navigator,
        Some(vec![
            Layout::FilmStrip,
            Layout::Tree,
            Layout::View(View::Details),
        ]),
    );
    assert_eq!(
        sentence(&drawable),
        None,
        "`Tree` is drawn, so the selection rule is satisfied silently: {drawable:?}"
    );

    let undrawable = panel(
        View::Navigator,
        Some(vec![Layout::FilmStrip, Layout::Linear]),
    );
    assert_eq!(
        sentence(&undrawable),
        Some(
            "This collection asks to be presented as FilmStrip or Linear, which this panel does \
             not draw; its files are shown as a tree."
                .to_owned()
        ),
        "the report names what it matched: {undrawable:?}"
    );

    // Table 153's `/View T` is drawn since ADR 1215, so it is selected rather than reported —
    // which is the whole difference between a capability and a sentence about not having one.
    let tiled = panel(View::Tile, None);
    assert_eq!(
        sentence(&tiled),
        None,
        "`T` is one of the presentations these rows are: {tiled:?}"
    );
    assert_eq!(
        panel::presentation(&pdf_model::collection::Collection {
            view: View::Tile,
            ..pdf_model::collection::Collection::default()
        }),
        panel::Mode::Tile
    );

    let plain = panel(View::Details, None);
    assert_eq!(
        sentence(&plain),
        None,
        "the details view is what these rows are: {plain:?}"
    );
}

/// §12.7.5.3's file-select control turns a person's text into a *file*, and no other field does.
///
/// Table 231 bit 21:
///
/// > If the FileSelect flag ( PDF 1.4 ) is set, the field shall function as a file-select control.
/// > In this case, the field's text represents the pathname of a file whose contents shall be
/// > submitted as the field's value
///
/// So the same keystrokes mean two different verbs depending on one flag, and this is the one
/// place the three windows decide which (ADR 1216).
#[test]
fn table_231_bit_21_makes_a_persons_text_a_file_rather_than_a_value() {
    let entry = |file_select| ControlKind::Entry {
        multiline: false,
        password: false,
        max_len: None,
        file_select,
    };
    let written = std::env::temp_dir().join("quorra-1189-file-select.txt");
    std::fs::write(&written, b"one,two\n").expect("the test writes its own file");
    let path = written.to_string_lossy().into_owned();

    // The flag clear: the text is §7.9.2.2's text string and nothing is opened.
    assert_eq!(
        viewer_host::form::edit_of(
            Some(&entry(false)),
            "plain",
            viewer_core::Entered::Text(path.clone())
        ),
        Ok(viewer_core::Edit::SetField {
            field: "plain".to_owned(),
            value: viewer_core::Entered::Text(path.clone()),
        })
    );

    // The flag set: the pathname stays the field's text and the contents come with it.
    let Ok(viewer_core::Edit::ChooseFile {
        field,
        pathname,
        bytes,
        mime,
    }) = viewer_host::form::edit_of(
        Some(&entry(true)),
        "upload",
        viewer_core::Entered::Text(path.clone()),
    )
    else {
        panic!("a file-select control takes a file");
    };
    assert_eq!(field, "upload");
    assert_eq!(pathname, path);
    assert_eq!(bytes.bytes(), b"one,two\n");
    // Table 44's `/Subtype` is what an embedded file states; a path on a filesystem states
    // nothing, and HTML 4.01 section 17.13.4.2's fallback is `pdf_model::submission`'s to apply.
    assert_eq!(mime, None);

    // §12.7.6.3's own words for a value that is gone are "its V entry shall be removed", and
    // there is no path to read behind nothing.
    assert_eq!(
        viewer_host::form::edit_of(Some(&entry(true)), "upload", viewer_core::Entered::Cleared),
        Ok(viewer_core::Edit::SetField {
            field: "upload".to_owned(),
            value: viewer_core::Entered::Cleared,
        })
    );

    // A path that names nothing is a file-select control with nothing behind it, said rather than
    // taken as ordinary text — which would be the wrong value under the right name (trap 5).
    let refused = viewer_host::form::edit_of(
        Some(&entry(true)),
        "upload",
        viewer_core::Entered::Text(format!("{path}.no-such-file")),
    );
    assert!(
        refused.is_err_and(|said| said.contains("cannot read")),
        "the refusal names the path"
    );
    let _ = std::fs::remove_file(&written);
}

/// Table 153's two drawable views are two presentations of one collection, and they differ.
///
/// The entry states each as its own `shall`, and the two sentences say what separates them:
/// `D` is "presented in details mode, with all information in the Schema dictionary presented in a
/// multi- column format", `T` is "presented in tile mode, with each file in the collection denoted
/// by a small icon and a subset of information from the Schema dictionary". So what is checkable
/// is *all* against *a subset*, and the icon — and it is checked on one document read twice, so
/// that a difference is the view's and not the file's.
#[test]
fn table_153s_details_and_tile_views_are_two_presentations_of_one_collection() {
    fn file_rows(rows: &[PanelRow]) -> Vec<&PanelRow> {
        flattened(rows)
            .into_iter()
            .filter(|row| matches!(row.action, RowAction::Extract { .. }))
            .collect()
    }
    let rows_for = |view: &str| {
        let (collection, initial, order, files) = collection_and_files(a_collection(
            &format!("/D (<3>report.pdf) /View /{view}"),
            "/Folders 10 0 R",
            3,
        ));
        collection_rows(&collection, &initial, &order, &files)
    };

    let details = rows_for("D");
    let detailed = file_rows(&details);
    assert_eq!(detailed.len(), 2, "both files, either way: {details:?}");
    // The schema states three fields and hides one, so "all information in the Schema dictionary"
    // is the two `/V true` ones — a hidden field is a field the *document* took out of the
    // interface, which is Table 155's `/O`/`/V` reading this panel already made.
    assert_eq!(
        detailed[0]
            .cells
            .iter()
            .map(|cell| cell.heading.as_str())
            .collect::<Vec<_>>(),
        ["File", "Size", "About"],
        "all of the visible schema, in /O order"
    );
    assert!(
        detailed.iter().all(|row| row.icon.is_none()),
        "the details view states no icon: {detailed:?}"
    );

    let tile = rows_for("T");
    let tiled = file_rows(&tile);
    assert_eq!(tiled.len(), 2, "no file falls out of the other view");
    assert!(
        tiled[0].cells.len() <= panel::TILE_FIELDS
            && tiled[0].cells.len() < detailed[0].cells.len(),
        "a subset, and fewer than the details view's: {tiled:?}"
    );
    // "each file in the collection denoted by a small icon", and a folder is a node of
    // §12.3.5.2's tree rather than a file in it — both get one, and which picture is the
    // toolkit's.
    assert!(
        tiled.iter().all(|row| row.icon.is_some()),
        "every file is denoted by an icon: {tiled:?}"
    );
    assert_eq!(
        flattened(&tile)
            .iter()
            .find(|row| row.label == "Chapters")
            .and_then(|row| row.icon),
        Some(panel::Icon::Folder)
    );
    // §12.3.5.1's `/D` is the same row in either view: the presentation changes what a row shows
    // and never which document the file named.
    assert_eq!(
        flattened(&details)
            .iter()
            .filter(|row| row.emphasis)
            .map(|row| row.label.clone())
            .collect::<Vec<_>>(),
        flattened(&tile)
            .iter()
            .filter(|row| row.emphasis)
            .map(|row| row.label.clone())
            .collect::<Vec<_>>()
    );
}

/// Table 44's `/Subtype` decides which kind of icon a tile carries, and a file stating none is one.
///
/// Table 153 asks for "a small icon" and states nothing about its artwork, so what is checkable is
/// the *kind*: ADR 1215's choice is that the kind comes from the media type, which is the only
/// thing about the file the standard puts in this program's hands. Read off the document rather
/// than off a hand-built value, so that the entry a producer writes is the entry this reads.
#[test]
fn a_tiles_icon_names_the_kind_table_44s_subtype_states() {
    let (_, _, _, files) = collection_and_files(a_collection(
        "/D (<3>report.pdf) /View /T",
        "/Folders 10 0 R",
        3,
    ));
    let kinds: Vec<(String, panel::Icon)> = files
        .iter()
        .map(|file| (file.name.clone(), panel::icon_of(file)))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("<3>report.pdf".to_owned(), panel::Icon::Document),
            // The second file's stream states no Table 44 `/Subtype` at all, and a file whose
            // kind the document did not state says only that it is a file — which is the honest
            // picture for one nothing is known about.
            ("readme.txt".to_owned(), panel::Icon::File),
        ]
    );
}

/// §12.3.5.2's restricted names reach a person, and a conforming collection says nothing.
///
/// The clause bounds a collection's names and then offers a choice — "[a]n interactive PDF
/// processor may choose to support invalid names or not. If not, an appropriate error message
/// shall be provided." This program supports them (ADR 1050), so the row still says `a:b`; the
/// sentence under it is what stops that being silent.
///
/// Both directions, because a sentence that fires on every collection is not a sentence about
/// this one (trap 11): the same panel with one conforming name adds no row at all.
#[test]
fn a_name_the_clause_restricts_is_said_out_loud_and_a_valid_one_is_not() {
    let panel = |name: &str| {
        let mut collection = pdf_model::collection::Collection::default();
        collection.folders = Some(pdf_model::collection::Folder {
            id: 3,
            name: name.to_owned(),
            description: None,
            item: pdf_model::collection::Item::default(),
            has_thumbnail: false,
            children: Vec::new(),
        });
        collection.invalid_names = pdf_model::collection::file_name_defects(name)
            .into_iter()
            .map(|restriction| pdf_model::collection::NameDefect {
                name: name.to_owned(),
                owner: pdf_model::collection::Named::Folder { id: 3 },
                restriction,
            })
            .collect();
        collection_rows(
            &collection,
            &pdf_model::collection::Initial::Container,
            &[],
            &[],
        )
    };

    let restricted = panel("a:b.");
    assert_eq!(
        restricted.last().map(|row| (row.note, row.label.as_str())),
        Some((
            true,
            "One name here is not a valid file name; it is shown as the document wrote it."
        )),
        "two restrictions broken by one name is still one name: {restricted:?}"
    );
    assert!(
        restricted.iter().any(|row| row.label == "a:b."),
        "supporting the name means drawing it: {restricted:?}"
    );

    let plain = panel("Chapters");
    assert!(
        plain.iter().all(|row| row.label
            != restricted
                .last()
                .map(|row| row.label.clone())
                .unwrap_or_default()),
        "a conforming name adds no sentence: {plain:?}"
    );
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
        let (collection, initial, order, files) =
            collection_and_files(a_collection("", folders, folder_id));
        assert_eq!(files.len(), 2, "the fixture embeds two files either way");
        let rows = collection_rows(&collection, &initial, &order, &files);
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

/// **§12.8.1's third question is answered by a host, and by nobody who did not ask.**
///
/// The two halves of `viewer_host::trust_anchors`, and the first is the one ADR 1039 decided:
/// with no directory named, the policy is empty and every signature in every document answers
/// `Trust::NoAnchorSupplied` — which is what this program has said since the
/// three-hundred-and-seventy-seventh session and what nobody typing nothing may change.
///
/// The second is the reading itself, over a directory laid out here: one DER certificate, one PEM
/// file holding two, one file that is neither, and one subdirectory. What must come back is three
/// anchors, one named refusal, and an order that does not depend on how the filesystem listed them.
#[test]
fn a_host_supplies_the_anchors_or_nobody_does() {
    let (none, refused) = viewer_host::trust_anchors(None, false);
    assert_eq!(
        viewer_host::trust_anchors(None, true).0.acceptance,
        pdf_signature::verdict::Acceptance::UnknownRevocationAccepted,
        "a word a person typed is not dropped because the other one was not typed"
    );
    assert!(none.anchors.is_empty(), "nobody named an anchor");
    assert_eq!(none.anchors.source(), "");
    assert!(refused.is_empty());
    assert_eq!(
        none.acceptance,
        pdf_signature::verdict::Acceptance::RevocationMustBeGood,
        "a host that has not decided gets the conservative answer (ADR 1067)"
    );

    let directory =
        std::env::temp_dir().join(format!("quorra-anchors-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("a-subdirectory")).expect("a scratch directory");
    // X.690 clause 8.1.2's constructed `SEQUENCE`, which is what makes a file DER here. Its
    // contents need not be a certificate: whether it is one is `pdf_signature::x509`'s to say, and
    // this host's job ends at handing the octets over.
    std::fs::write(directory.join("1-root.der"), [0x30, 0x03, 0x02, 0x01, 0x00])
        .expect("a DER file");
    std::fs::write(
        directory.join("2-pair.pem"),
        "a comment PEM permits before the boundary\n\
         -----BEGIN CERTIFICATE-----\nMAMCAQA=\n-----END CERTIFICATE-----\n\
         -----BEGIN CERTIFICATE-----\nMAMCAQE=\n-----END CERTIFICATE-----\n",
    )
    .expect("a PEM file");
    std::fs::write(directory.join("3-notes.txt"), "not a certificate at all\n")
        .expect("a plain file");

    let (policy, refused) = viewer_host::trust_anchors(Some(&directory), true);
    assert_eq!(policy.anchors.len(), 3, "one DER and two PEM blocks");
    let names: Vec<&str> = policy
        .anchors
        .certificates()
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        names,
        ["1-root.der", "2-pair.pem", "2-pair.pem (certificate 2)"],
        "sorted by name, so two runs over one directory supply one store"
    );
    assert_eq!(
        policy.anchors.certificates()[1].1,
        [0x30, 0x03, 0x02, 0x01, 0x00],
        "the PEM block's base64 is what came out of it"
    );
    assert!(
        policy.anchors.source().contains("--trust-anchors"),
        "a verdict names where its anchors came from: {}",
        policy.anchors.source()
    );
    assert!(
        policy.anchors.at().unix_seconds() > 1_700_000_000,
        "a clock"
    );
    assert_eq!(
        policy.acceptance,
        pdf_signature::verdict::Acceptance::UnknownRevocationAccepted
    );
    let [one] = refused.as_slice() else {
        panic!("the one file that is neither is named, not dropped: {refused:?}");
    };
    assert!(one.to_string().contains("3-notes.txt"), "trap 5: {one}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// Table 153's `/Sort`, as the order the panel's rows stand in.
///
/// §12.3.5.1's Table 153, on the entry:
///
/// > A collection sort dictionary, which specifies the order in which items in the collection
/// > shall be sorted in the user interface
///
/// A `shall` about the rows, and the one entry of Table 153 that no host could obey for itself:
/// the values it orders by are §7.11.6's collection item on each file specification's `/CI`, which
/// `Answer::Attachments` does not carry. So `pdf_model::collection::sorted_keys` resolves the
/// order, `viewer_core::Answer::Collection` carries it and this mapping applies it — one answer
/// for the two native windows, and `viewer_host::panel::in_sort_order` the same answer for the
/// third (ADR 1168).
///
/// The fixture's two files are `<3>report.pdf` and `readme.txt`, in that order in the
/// `/EmbeddedFiles` tree. Sorting by a `/Subtype /F` file name puts *readme.txt* second when the
/// order runs up and first when it runs down, which is what makes the assertion about `/Sort`
/// rather than about the tree.
#[test]
fn a_collection_lists_its_files_in_the_order_table_153_states() {
    let extracted = |rows: &[PanelRow]| -> Vec<String> {
        flattened(rows)
            .iter()
            .filter_map(|row| match &row.action {
                RowAction::Extract { name } => Some(name.clone()),
                _ => None,
            })
            .collect()
    };
    // No `/Folders`, so every file is a top-level row and the order is the whole of what the rows
    // say — a folder tree would answer this question with its own nesting as well.
    let sorted = |ascending: &str| {
        let bytes = a_collection(&format!("/Sort << /S /FN{ascending} >>"), "", 3);
        let (collection, initial, order, files) = collection_and_files(bytes);
        extracted(&collection_rows(&collection, &initial, &order, &files))
    };

    assert_eq!(
        sorted(""),
        ["readme.txt", "<3>report.pdf"],
        "§12.3.5.1: text fields are \"ordered lexically from smaller to larger\" when ascending"
    );
    assert_eq!(
        sorted(" /A false"),
        ["<3>report.pdf", "readme.txt"],
        "Table 156's /A false is that reversed"
    );

    // And a collection stating no `/Sort` leaves the `/EmbeddedFiles` tree's own order, which is
    // the order the document also stated — nothing is invented where the clause says nothing.
    let (collection, initial, order, files) = collection_and_files(a_collection("", "", 3));
    assert!(order.is_empty(), "no /Sort states no order");
    assert_eq!(
        extracted(&collection_rows(&collection, &initial, &order, &files)),
        ["<3>report.pdf", "readme.txt"]
    );
}

/// ISO 32000-2 §12.6.4.3's file, at each of the four levels a reader may set.
///
/// Table 203 makes `/F` "[t]he file in which the destination shall be located", and which files a
/// document may name is a property of the processor rather than of the format — the same sentence
/// §12.7.6.4's import-data is answered with. So the **path rule comes before the level**, which is
/// the half this test exists for: a name outside the document's own directory is refused at every
/// level including the permissive one, because a reader who said their documents may
/// cross-reference each other did not say that any file on this disk may be opened on a
/// document's say-so (ADR 1155's position, ADR 1227).
#[test]
fn a_remote_go_to_is_confined_to_the_documents_directory_at_every_level() {
    use viewer_host::{Remote, RemoteDocuments, remote, remote_documents};

    let directory = Path::new("/documents");
    for level in RemoteDocuments::ALL {
        for hostile in ["../secrets.pdf", "/etc/passwd", "sub/next.pdf", ""] {
            assert!(
                matches!(remote(Some(directory), hostile, level), Remote::Refuse(_)),
                "{hostile} is not a plain file name beside the document, {level:?} or not"
            );
        }
        assert!(
            matches!(remote(None, "next.pdf", level), Remote::Refuse(_)),
            "a document with no directory has no neighbourhood to resolve against"
        );
    }

    // And the neighbour is admitted at three of the four, in three different shapes: the level
    // decides the act that is left rather than the path.
    let beside = PathBuf::from("/documents/next.pdf");
    assert_eq!(
        remote(Some(directory), "next.pdf", RemoteDocuments::Open),
        Remote::Supply {
            path: beside.clone(),
            note: None
        }
    );
    let Remote::Supply { path, note } = remote(Some(directory), "next.pdf", RemoteDocuments::Warn)
    else {
        panic!("warn opens the file and says so afterwards");
    };
    assert_eq!(path, beside);
    assert!(
        note.is_some_and(|note| note.contains(RemoteDocuments::Warn.as_str())),
        "the sentence names the level that produced it"
    );
    let Remote::Ask { path, question } = remote(Some(directory), "next.pdf", RemoteDocuments::Ask)
    else {
        panic!("ask is the default and puts the question");
    };
    assert_eq!(path, beside);
    assert!(
        question.reasons.contains("next.pdf") && question.reasons.contains("/documents/next.pdf"),
        "a person judging this is owed both the name the document wrote and the file it resolved \
         to: {}",
        question.reasons
    );
    let Remote::Refuse(refused) = remote(Some(directory), "next.pdf", RemoteDocuments::Refuse)
    else {
        panic!("refuse opens nothing");
    };
    assert!(
        refused.contains("next.pdf"),
        "and a refusal still says what the document asked for (trap 5): {refused}"
    );

    // Every level round-trips through the word a person types, and the complaint names all four —
    // `Links`'s rule, because a reader meets both options on one command line.
    for level in RemoteDocuments::ALL {
        assert_eq!(remote_documents(level.as_str()), Ok(level));
    }
    let complaint =
        remote_documents("on").expect_err("`on` is RESTRICTIONS's word and means the other end");
    for level in RemoteDocuments::ALL {
        assert!(
            complaint.contains(level.as_str()),
            "the complaint names every level this option takes: {complaint}"
        );
    }
}

/// ISO 32000-2 §10.8.3's simulation is a preference with two words, and §10.8.1 says whose it is.
///
/// > Whether separations are produced is up to the processing software.
///
/// Two words rather than `CLAUDE.md`'s four levels, and that is the decision this test pins: the
/// four levels are for what a *document* asserts over its reader, and nothing in any file asks for
/// this — so there is nobody to ask and nothing to warn about (ADR 1189 section 2's division,
/// ADR 1228).
#[test]
fn the_separation_simulation_is_a_preference_with_two_words() {
    use viewer_host::{SEPARATIONS, separations, separations_note};

    assert_eq!(separations("on"), Ok(true));
    assert_eq!(separations("off"), Ok(false));
    for word in ["ask", "warn", "refuse", "", "yes"] {
        let complaint = separations(word).expect_err("{word} is not one of the two");
        assert!(
            complaint.contains("on, off") && complaint.contains(SEPARATIONS),
            "the complaint names the option and both its words: {complaint}"
        );
    }
    // The sentence says which clause the reader is now looking at, because every colour on the
    // page has just changed and nothing else on the screen says why.
    assert!(
        separations_note(true).contains("§10.8.3"),
        "on names the simulation's own clause"
    );
    assert!(
        separations_note(false).contains("§10.8.2"),
        "off names the clause that describes what a screen does instead"
    );
}
