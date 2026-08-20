//! The tree an assistive technology is handed, built from what `viewer-core` answers with.
//!
//! Plain data on both sides, so these run on every platform — which is the point of the crate
//! being in two halves. What the *bus* does with the result needs a session bus, so it is not
//! here: `src/bridge.rs`'s own tests hold the half of it that can be asked without one — that a
//! build with no adapter names its shortfall rather than doing nothing — and nothing in this tree
//! drives a real bus. (This sentence named a `tests/atspi.rs` that has never existed, found by
//! `doc/todo/01`'s eighth sweep on the round it became a program.)

#![expect(
    clippy::panic,
    reason = "a test asserts; a failed assertion is the point"
)]

use accesskit::{Action, Node, NodeId, Role, TextDirection, Toggled};
use pdf_model::form::{ChoiceControl, Control, TextControl};
use pdf_model::structure::HeaderScope;
use viewer_accessibility::{DocumentView, PageView, tree};
use viewer_core::{AccessibilityNode, Character, TextLine};

/// One structure element, as `Query::AccessibilityTree` would answer with it.
fn element(parent: Option<usize>, role: &str, name: &str) -> AccessibilityNode {
    AccessibilityNode {
        parent,
        role: role.to_owned(),
        name: name.to_owned(),
        substituted: false,
        language: None,
        quads: Vec::new(),
        header_scope: None,
        bounds: None,
        control: None,
        annotation: None,
        headers: Vec::new(),
        lines: Vec::new(),
    }
}

/// One `Form`, with the §12.7.5 control of the widget its object reference names.
fn form(control: Option<Control>) -> AccessibilityNode {
    AccessibilityNode {
        control,
        ..element(None, "Form", "")
    }
}

/// One `TH`, with the axis Table 384 states or §14.8.5.7 assumes for it.
fn header(parent: Option<usize>, name: &str, scope: Option<HeaderScope>) -> AccessibilityNode {
    AccessibilityNode {
        header_scope: scope,
        ..element(parent, "TH", name)
    }
}

/// A page with nothing wrong with it and nothing on it.
fn view<'a>(nodes: &'a [AccessibilityNode], reports: &'a [String]) -> PageView<'a> {
    PageView {
        page: 0,
        label: None,
        bounds: [0.0, 0.0, 800.0, 1000.0],
        nodes,
        reports,
        readback: pdf_model::content::Shortfall::default(),
    }
}

/// The tree for a window showing one page, which is `SinglePage` and most of these tests.
fn built(page: PageView<'_>) -> accesskit::TreeUpdate {
    shown(&[page])
}

/// The tree for a window showing whichever pages are handed to it, which is Table 29's column.
fn shown(pages: &[PageView<'_>]) -> accesskit::TreeUpdate {
    tree::build(&DocumentView {
        window: "a window",
        document: "a document",
        pages: 3,
        viewport: (800.0, 1000.0),
        shown: pages,
    })
}

/// Finds a node by identifier in an update.
fn node(update: &accesskit::TreeUpdate, id: NodeId) -> &Node {
    update
        .nodes
        .iter()
        .find(|(at, _)| *at == id)
        .map_or_else(|| panic!("{id:?} is in the update"), |(_, node)| node)
}

/// Every node named as a child is in the update, and the root is the window.
///
/// `accesskit::TreeUpdate` states the invariant itself — "[i]t is an error for any node in this
/// list to not be either the root or a child of another node" — and the AT-SPI adapter treats
/// the root specially only where its role is `Window`.
#[test]
fn the_root_is_a_window_and_every_child_named_is_present() {
    let nodes = [
        element(None, "P", "the first paragraph"),
        element(Some(0), "Span", "a span inside it"),
    ];
    let update = built(view(&nodes, &[]));
    assert_eq!(update.tree.as_ref().map(|tree| tree.root), Some(NodeId(0)));
    assert_eq!(update.focus, NodeId(0));
    assert_eq!(node(&update, NodeId(0)).role(), Role::Window);
    assert_eq!(node(&update, NodeId(1)).role(), Role::PdfRoot);

    let listed: Vec<NodeId> = update.nodes.iter().map(|(id, _)| *id).collect();
    for (id, held) in &update.nodes {
        for child in held.children() {
            assert!(
                listed.contains(child),
                "{id:?} names {child:?}, which is not in the update"
            );
        }
    }
    // Every node but the root is somebody's child.
    for (id, _) in &update.nodes {
        if *id == NodeId(0) {
            continue;
        }
        assert!(
            update
                .nodes
                .iter()
                .any(|(_, held)| held.children().contains(id)),
            "{id:?} is in the update and nothing is its parent"
        );
    }
}

/// §14.8.4's types reach the tree as roles, and a heading keeps its level.
#[test]
fn a_pages_elements_arrive_as_roles() {
    let nodes = [
        element(None, "H2", "A heading"),
        element(None, "P", "A paragraph"),
        element(None, "Figure", "a photograph of a bridge"),
    ];
    let update = built(view(&nodes, &[]));
    let heading = node(&update, NodeId(16));
    assert_eq!(heading.role(), Role::Heading);
    assert_eq!(heading.level(), Some(2));
    assert_eq!(heading.label(), Some("A heading"));
    assert_eq!(node(&update, NodeId(17)).role(), Role::Paragraph);
    assert_eq!(node(&update, NodeId(18)).role(), Role::Image);
}

/// §14.9.3's `/Alt` replaces the element, so what is under it is not published as well.
///
/// > When applied to structure elements, the alternate description text (see 7.9.2.2, "Text
/// > string type") is a complete (or whole) word or phrase substitution for the current element.
///
/// A tree that carried both would have the picture described once by the author and once by
/// whatever text happened to be inside it.
#[test]
fn a_substitution_stops_the_walk() {
    let mut figure = element(None, "Figure", "a bar chart of quarterly revenue");
    figure.substituted = true;
    let nodes = [figure, element(Some(0), "Span", "Q1 Q2 Q3 Q4")];
    let update = built(view(&nodes, &[]));
    assert_eq!(
        node(&update, NodeId(16)).label(),
        Some("a bar chart of quarterly revenue")
    );
    assert!(node(&update, NodeId(16)).children().is_empty());
    assert!(
        update.nodes.iter().all(|(id, _)| *id != NodeId(17)),
        "the span under a substitution is not published"
    );
}

/// An untagged page says so, which is a statement about the document rather than silence.
#[test]
fn an_untagged_page_says_that_the_document_states_no_structure() {
    let update = built(view(&[], &[]));
    let note = node(&update, NodeId(16));
    assert_eq!(note.role(), Role::Label);
    // A `Role::Label`'s accessible name comes from its *value*: see `tree::say`.
    let label = note.value().unwrap_or_default();
    assert!(label.contains("no logical structure"), "{label}");
    assert!(label.contains("14.7"), "{label}");
}

/// What the page could not draw is in the tree, item by item.
///
/// The half of this round that is not a mapping: a person who cannot see the page is the one
/// person for whom "the title bar says two items were not drawn" is no answer at all.
#[test]
fn what_the_page_could_not_draw_is_in_the_tree() {
    let reports = [
        "an image using JPXDecode was not drawn".to_owned(),
        "a font could not be loaded, so 24 characters drew nothing".to_owned(),
    ];
    let nodes = [element(None, "P", "some text that did draw")];
    let update = built(view(&nodes, &reports));
    let status = node(&update, NodeId(3));
    assert_eq!(status.role(), Role::Status);
    assert_eq!(status.children().len(), 2);
    assert_eq!(
        node(&update, NodeId(1_000_001)).value(),
        Some("a font could not be loaded, so 24 characters drew nothing")
    );

    // And a page with nothing to report grows no such group.
    let quiet = built(view(&nodes, &[]));
    assert!(quiet.nodes.iter().all(|(id, _)| *id != NodeId(3)));
}

/// What the page could not be *read* as reaches the same group, and is not called a drawing fault.
///
/// **The one population for whom this is not a nicety.** A page whose codes ISO 32000-2 §9.10.2
/// cannot name draws perfectly and speaks short, so every other channel this program has says the
/// page is fine — `Interpretation::is_complete` is true and `Query::Reports` is empty. The
/// sentence has to say the words are missing and must not say the picture is wrong; ADR 0422.
#[test]
fn what_the_page_could_not_be_read_as_is_in_the_tree_and_is_not_a_refusal() {
    let nodes = [element(None, "P", "text that drew and cannot be read")];
    let short = PageView {
        readback: pdf_model::content::Shortfall {
            unnamed: pdf_model::content::UnnamedCodes {
                unlisted_name: 28,
                ..pdf_model::content::UnnamedCodes::default()
            },
            ..pdf_model::content::Shortfall::default()
        },
        ..view(&nodes, &[])
    };
    let update = built(short);
    let status = node(&update, NodeId(3));
    assert_eq!(status.role(), Role::Status);
    assert_eq!(status.children().len(), 1);
    let said = node(&update, NodeId(1_000_000))
        .value()
        .unwrap_or_default()
        .to_owned();
    assert!(said.contains("28"), "{said}");
    assert!(said.contains("cannot be read"), "{said}");
    assert!(
        !said.contains("not drawn"),
        "a code the standard leaves unnameable is not a mark this program failed to make: {said}"
    );

    // A glyph the font describes as empty is a space, and says nothing at all (ADR 0270).
    let spaces = PageView {
        readback: pdf_model::content::Shortfall {
            reaching_a_blank_glyph: 12,
            ..pdf_model::content::Shortfall::default()
        },
        ..view(&nodes, &[])
    };
    let quiet = built(spaces);
    assert!(quiet.nodes.iter().all(|(id, _)| *id != NodeId(3)));
}

/// An element's rectangle covers every quadrilateral it answered with.
#[test]
fn an_elements_bounds_cover_all_of_its_shapes() {
    let mut paragraph = element(None, "P", "two lines");
    paragraph.quads = vec![
        [10.0, 20.0, 110.0, 20.0, 110.0, 32.0, 10.0, 32.0],
        [10.0, 34.0, 90.0, 34.0, 90.0, 46.0, 10.0, 46.0],
    ];
    let nodes = [paragraph];
    let update = built(view(&nodes, &[]));
    let bounds = node(&update, NodeId(16))
        .bounds()
        .expect("it covers the lines");
    assert!((bounds.x0 - 10.0).abs() < 1e-6, "{bounds:?}");
    assert!((bounds.y0 - 20.0).abs() < 1e-6, "{bounds:?}");
    assert!((bounds.x1 - 110.0).abs() < 1e-6, "{bounds:?}");
    assert!((bounds.y1 - 46.0).abs() < 1e-6, "{bounds:?}");
}

/// An element that drew no text is placed by the rectangle its document states.
///
/// §14.8.5.4.3's `/BBox` is "the rectangle that completely encloses its visible content", and a
/// `Figure` is the element it exists for: nothing it draws is text, so `quads` is empty and
/// AccessKit would otherwise be handed a node with no place. Where both are present the measured
/// shapes win, which is [`tree`]'s own order — the picture on the screen is what this program
/// drew, and the attribute is a claim about a layout it has already carried out.
#[test]
fn a_figure_that_drew_no_text_is_placed_by_its_stated_bounds() {
    let mut figure = element(None, "Figure", "a chart of sales");
    figure.bounds = Some([12.0, 24.0, 212.0, 124.0]);
    let mut paragraph = element(None, "P", "a caption");
    paragraph.quads = vec![[10.0, 20.0, 110.0, 20.0, 110.0, 32.0, 10.0, 32.0]];
    paragraph.bounds = Some([0.0, 0.0, 400.0, 400.0]);
    let nodes = [figure, paragraph];
    let update = built(view(&nodes, &[]));

    let placed = node(&update, NodeId(16))
        .bounds()
        .expect("a figure with no shapes still has the rectangle its document states");
    assert!((placed.x0 - 12.0).abs() < 1e-6, "{placed:?}");
    assert!((placed.y0 - 24.0).abs() < 1e-6, "{placed:?}");
    assert!((placed.x1 - 212.0).abs() < 1e-6, "{placed:?}");
    assert!((placed.y1 - 124.0).abs() < 1e-6, "{placed:?}");

    let measured = node(&update, NodeId(17))
        .bounds()
        .expect("a paragraph is placed by what was drawn");
    assert!((measured.x1 - 110.0).abs() < 1e-6, "{measured:?}");
}

/// §12.4.2's page label names the page where the document states one.
#[test]
fn the_page_is_named_by_its_label_where_there_is_one() {
    let mut labelled = view(&[], &[]);
    labelled.label = Some("iii");
    labelled.page = 2;
    let update = built(labelled);
    let name = node(&update, NodeId(2)).label().unwrap_or_default();
    assert!(name.contains("iii"), "{name}");
    assert!(name.contains("3 of 3"), "{name}");
}

/// A static-text node carries its text where AT-SPI will look for it.
///
/// `accesskit_consumer`'s `label_comes_from_value` is `role == Role::Label` and nothing else, so
/// the one role this mapping uses for text that is only text is also the one whose `label` an
/// assistive technology never reads. Asserted here because it is not visible in the shape of the
/// tree — the first end-to-end run of this bridge put every `Label` on the bus with an empty
/// name (ADR 0214).
#[test]
fn a_static_text_node_puts_its_text_in_the_value() {
    let nodes = [element(None, "Span", "a run of text")];
    let update = built(view(&nodes, &[]));
    let span = node(&update, NodeId(16));
    assert_eq!(span.role(), Role::Label);
    assert_eq!(span.value(), Some("a run of text"));
    assert_eq!(span.label(), None);

    // And a role that is not `Label` keeps its text in the label, which is where that role's
    // name comes from.
    let paragraphs = [element(None, "P", "a paragraph")];
    let update = built(view(&paragraphs, &[]));
    assert_eq!(node(&update, NodeId(16)).label(), Some("a paragraph"));
}

/// A header cell's axis decides its role, and the axis the platform cannot say is said in words.
///
/// ISO 32000-2 §14.8.4.8.3 makes a `TH` a cell "describing one or more rows, columns or rows and
/// columns of the table", and Table 384's `/Scope` is which. AccessKit has a role for two of the
/// three; the third — and a cell whose axis could not be determined at all — are described rather
/// than guessed at, which is what `role.rs` argues.
#[test]
fn a_header_cells_axis_decides_its_role_and_the_rest_is_said_in_words() {
    let nodes = [
        header(None, "Region", Some(HeaderScope::Row)),
        header(None, "2024", Some(HeaderScope::Column)),
        header(None, "corner", Some(HeaderScope::Both)),
        header(None, "loose", None),
    ];
    let update = built(view(&nodes, &[]));

    let row = node(&update, NodeId(16));
    assert_eq!(row.role(), Role::RowHeader);
    assert_eq!(row.description(), None, "nothing was lost");
    assert_eq!(node(&update, NodeId(17)).role(), Role::ColumnHeader);
    assert_eq!(node(&update, NodeId(17)).description(), None);

    // `Both` has no role in AccessKit and none in AT-SPI either, so the loss is stated.
    let both = node(&update, NodeId(18));
    assert_eq!(both.role(), Role::ColumnHeader);
    assert!(
        both.description().is_some_and(|note| note.contains("both")),
        "{:?}",
        both.description()
    );

    // And an axis this reader could not determine says so rather than passing for a column's.
    let unknown = node(&update, NodeId(19));
    assert_eq!(unknown.role(), Role::ColumnHeader);
    assert!(
        unknown
            .description()
            .is_some_and(|note| note.contains("not known")),
        "{:?}",
        unknown.description()
    );
}

/// A cell's header cells reach a person, in the order §14.8.4.8.3 builds them in.
///
/// Table 384's `/Short` says what this is for — "for each table cell the applicable header cells
/// are read to the user in order to allow that user to understand the content of the table cell" —
/// and on this platform the description is the channel that gets there. See `tree::headers` for
/// why it is not the `labelled_by` relation.
#[test]
fn a_cells_header_cells_are_said_in_the_order_the_clause_builds_them() {
    let mut cell = element(None, "TD", "1.4 million");
    cell.headers = vec![1, 2];
    let nodes = [
        cell,
        header(None, "France", Some(HeaderScope::Row)),
        header(None, "Population", Some(HeaderScope::Column)),
    ];
    let update = built(view(&nodes, &[]));
    assert_eq!(
        node(&update, NodeId(16)).description(),
        Some("headers, most specific first: France, Population")
    );

    // A cell with no headers says nothing, and a header cell that lost something to the platform
    // keeps saying it beside its own headers rather than instead of them.
    assert_eq!(node(&update, NodeId(17)).description(), None);
    let mut corner = header(None, "corner", Some(HeaderScope::Both));
    corner.headers = vec![1];
    let both = [corner, header(None, "Region", Some(HeaderScope::Row))];
    let update = built(view(&both, &[]));
    let note = node(&update, NodeId(16)).description().unwrap_or_default();
    assert!(note.contains("both"), "{note}");
    assert!(
        note.contains("headers, most specific first: Region"),
        "{note}"
    );
}

/// A header cell whose words are in a `P` inside it is still said by name.
///
/// Found by reading the tree back off a real bus rather than in a test: `bug2014080.pdf` puts each
/// cell's text in a paragraph within the cell, so every `TH` in it has an empty
/// `AccessibilityNode::name` — which is right, because that field is the element's own text and
/// not its subtree's. A cell *named as a header* is the one place the subtree is what is wanted,
/// because nothing else will descend into it to say the words.
#[test]
fn a_header_cell_whose_text_is_in_a_child_is_still_named() {
    let mut cell = element(None, "TD", "23");
    cell.headers = vec![1];
    let nodes = [
        cell,
        header(None, "", Some(HeaderScope::Column)),
        element(Some(1), "P", "Sydney"),
    ];
    let update = built(view(&nodes, &[]));
    let note = node(&update, NodeId(16)).description().unwrap_or_default();
    assert!(
        note.contains("headers, most specific first: Sydney"),
        "{note}"
    );
}

/// §14.8.4.7.2's `Form` becomes the control its widget is, with the state a toggling one has.
///
/// Table 368 makes the type one that "[e]ncloses a PDF widget annotation and associated content,
/// if any" — Errata Collection 3's Issue #437 — and requires one per widget: "[i]n a tagged PDF,
/// Form shall be used for each PDF widget annotation that belongs to the real content of the
/// document". So it is a control, and a group was the wrong answer for all 272 of the corpus's.
#[test]
fn a_form_element_becomes_the_control_its_widget_annotation_is() {
    let combo = |combo: bool, editable: bool| {
        Control::Choice(ChoiceControl {
            combo,
            editable,
            ..ChoiceControl::default()
        })
    };
    let text = |multiline: bool, password: bool| {
        Control::Text(TextControl {
            multiline,
            password,
            ..TextControl::default()
        })
    };
    let role = |control: Option<Control>| {
        let nodes = [form(control)];
        node(&built(view(&nodes, &[])), NodeId(16)).role()
    };

    assert_eq!(role(Some(Control::PushButton)), Role::Button);
    assert_eq!(role(Some(Control::CheckBox { on: false })), Role::CheckBox);
    assert_eq!(
        role(Some(Control::RadioButton {
            on: false,
            no_toggle_to_off: false,
            in_unison: false,
        })),
        Role::RadioButton
    );
    assert_eq!(role(Some(text(false, false))), Role::TextInput);
    assert_eq!(role(Some(text(true, false))), Role::MultilineTextInput);
    // Table 231 bit 14 before bit 13: a control that echoed a password because the file also
    // asked for multiple lines is the one mistake here that cannot be taken back.
    assert_eq!(role(Some(text(true, true))), Role::PasswordInput);
    assert_eq!(role(Some(combo(false, false))), Role::ListBox);
    assert_eq!(role(Some(combo(true, false))), Role::ComboBox);
    assert_eq!(role(Some(combo(true, true))), Role::EditableComboBox);

    // §12.7.5.5's signature and a field stating no `/FT` keep the group and say why, which is
    // this crate's rule for every distinction the platform cannot carry.
    for control in [Control::Signature, Control::Unstated] {
        let nodes = [form(Some(control))];
        let built = built(view(&nodes, &[]));
        assert_eq!(node(&built, NodeId(16)).role(), Role::Group);
        assert!(
            node(&built, NodeId(16)).description().is_some(),
            "a loss the platform cannot carry is named rather than silent"
        );
    }
    // And a `Form` this program could not follow to a widget.
    let nodes = [form(None)];
    let built = built(view(&nodes, &[]));
    assert_eq!(node(&built, NodeId(16)).role(), Role::Group);
    assert!(node(&built, NodeId(16)).description().is_some());
}

/// A check box says whether it is ticked, which is half of what the control means.
///
/// §12.7.5.2.3's field "toggles between two states, on and off", and `pdf_model::form` has already
/// resolved which through Table 226's `/V` and this view's own edits — so a box a person has just
/// clicked reaches the bus as clicked.
#[test]
fn a_toggling_button_carries_its_state_and_nothing_else_does() {
    let toggled = |control: Control| {
        let nodes = [form(Some(control))];
        node(&built(view(&nodes, &[])), NodeId(16)).toggled()
    };
    assert_eq!(toggled(Control::CheckBox { on: true }), Some(Toggled::True));
    assert_eq!(
        toggled(Control::CheckBox { on: false }),
        Some(Toggled::False)
    );
    assert_eq!(
        toggled(Control::RadioButton {
            on: true,
            no_toggle_to_off: true,
            in_unison: false,
        }),
        Some(Toggled::True)
    );
    // §12.7.5.3's text field is neither on nor off, and saying `false` would be an answer to a
    // question the clause does not ask.
    assert_eq!(toggled(Control::Text(TextControl::default())), None);
    assert_eq!(toggled(Control::PushButton), None);
    assert_eq!(
        node(
            &built(view(&[element(None, "P", "a paragraph")], &[])),
            NodeId(16)
        )
        .toggled(),
        None
    );
}

/// One paragraph of laid-out text, with each character a `width`-wide box on one baseline.
fn typed(text: &str, origin: (f32, f32), width: f32) -> AccessibilityNode {
    let mut characters = Vec::new();
    for (index, letter) in text.chars().enumerate() {
        let count = u16::try_from(index).unwrap_or(u16::MAX);
        let at = origin.0 + f32::from(count) * width;
        characters.push(Character {
            bytes: letter.len_utf8(),
            bounds: [at, origin.1, at + width, origin.1 + 10.0],
        });
    }
    AccessibilityNode {
        lines: vec![TextLine {
            text: text.to_owned(),
            characters,
        }],
        ..element(None, "P", text)
    }
}

/// A paragraph's own text reaches the platform as a run a caret can move through.
///
/// The three arrays are what `org.a11y.atspi.Text` is built out of, and the invariant each of
/// them has to keep is asserted rather than assumed: the lengths sum to the value, the positions
/// are measured from the run's own edge, and there is one of each per character.
#[test]
fn a_paragraphs_own_text_reaches_the_platform_as_a_run() {
    let nodes = [typed("two words", (100.0, 50.0), 6.0)];
    let update = built(view(&nodes, &[]));
    let paragraph = node(&update, NodeId(16));
    assert_eq!(paragraph.role(), Role::Paragraph);
    assert_eq!(paragraph.children(), [NodeId(2_000_000)]);
    let run = node(&update, NodeId(2_000_000));
    assert_eq!(run.role(), Role::TextRun);
    assert_eq!(run.value(), Some("two words"));
    assert_eq!(run.text_direction(), Some(TextDirection::LeftToRight));
    assert_eq!(run.character_lengths().len(), 9);
    assert_eq!(
        usize::from(run.character_lengths().iter().copied().sum::<u8>()),
        "two words".len(),
        "AccessKit requires the lengths to sum to the value's bytes"
    );
    // The run begins at x = 100, so the first character stands at 0 and each is one width along.
    assert_eq!(
        run.character_positions(),
        Some([0.0, 6.0, 12.0, 18.0, 24.0, 30.0, 36.0, 42.0, 48.0].as_slice())
    );
    assert_eq!(run.character_widths().map(<[f32]>::len), Some(9));
    // "two" begins at character 0 and "words" at 4, the space belonging to the word before it.
    assert_eq!(run.word_starts(), [0, 4]);
}

/// The page is the node that carries the text, and that is a platform's requirement.
///
/// `accesskit_consumer::Node::supports_text_ranges` answers `true` only for a text input or for
/// `Label`, `Document` or `Terminal`, so no §14.8.4 role this crate maps to could carry AT-SPI's
/// `Text` — a `Paragraph` cannot. The page node is this program's own rather than a structure
/// element's, which is why it is the one that may take the role. See `tree`'s documentation.
#[test]
fn the_page_is_the_node_a_caret_moves_through() {
    let nodes = [typed("a line", (0.0, 0.0), 5.0)];
    let update = built(view(&nodes, &[]));
    assert_eq!(node(&update, NodeId(2)).role(), Role::Document);
}

/// A line longer than one run's arrays becomes several runs of one line, joined.
#[test]
fn a_long_line_becomes_runs_that_say_they_are_one_line() {
    let long = "x".repeat(300);
    let nodes = [typed(&long, (0.0, 0.0), 1.0)];
    let update = built(view(&nodes, &[]));
    let first = node(&update, NodeId(2_000_000));
    let second = node(&update, NodeId(2_000_001));
    assert_eq!(first.character_lengths().len(), 255);
    assert_eq!(second.character_lengths().len(), 45);
    assert_eq!(first.next_on_line(), Some(NodeId(2_000_001)));
    assert_eq!(second.previous_on_line(), Some(NodeId(2_000_000)));
}

/// §14.8.2.2's artifact is not somewhere a caret goes, for the reason it is not spoken.
#[test]
fn an_artifact_has_no_run_to_move_through() {
    let artifact = AccessibilityNode {
        role: "Artifact".to_owned(),
        ..typed("page 7", (0.0, 0.0), 5.0)
    };
    let update = built(view(&[artifact], &[]));
    assert!(
        !update
            .nodes
            .iter()
            .any(|(_, held)| held.role() == Role::TextRun),
        "an artifact's words are not the document's content (ISO 32000-2 §14.8.2.2)"
    );
}

/// Text drawn right to left is measured from the run's right edge, which is where a caret is.
#[test]
fn a_line_that_runs_the_other_way_says_so_and_measures_from_the_other_edge() {
    let mut backwards = typed("abc", (0.0, 0.0), 10.0);
    if let Some(line) = backwards.lines.first_mut() {
        line.characters.reverse();
        line.text = "cba".to_owned();
    }
    let update = built(view(&[backwards], &[]));
    let run = node(&update, NodeId(2_000_000));
    assert_eq!(run.text_direction(), Some(TextDirection::RightToLeft));
    // The character drawn last sits at x = 0 and the run's right edge is at 30, so the three
    // stand 0, 10 and 20 along the direction they are read in.
    assert_eq!(
        run.character_positions(),
        Some([0.0, 10.0, 20.0].as_slice())
    );
}

/// An element with a place says a client may scroll to it; one with none says nothing.
///
/// The condition is the answerable one rather than the plausible one: `Command::Scroll` takes a
/// rectangle, and an element that marked no text and states no Table 379 `/BBox` names none.
#[test]
fn an_element_that_has_a_place_invites_being_scrolled_to() {
    let placed = AccessibilityNode {
        bounds: Some([10.0, 20.0, 30.0, 40.0]),
        ..element(None, "Figure", "a chart")
    };
    let nowhere = element(None, "Div", "");
    let update = built(view(&[placed, nowhere], &[]));
    assert!(node(&update, NodeId(16)).supports_action(Action::ScrollIntoView));
    assert!(!node(&update, NodeId(17)).supports_action(Action::ScrollIntoView));
}

/// An element whose content is an annotation says a client may click it, and a paragraph does not.
///
/// §12.5.1: "[w]hen the user activates the annotation by clicking it, it exhibits its associated
/// object". Table 368 gives three structure types whose content is an annotation, and §14.7.5.3's
/// object reference is how each names one — which is what `AccessibilityNode::annotation` carries.
/// `accesskit_atspi_common` puts `org.a11y.atspi.Action` on a node only where `Action::Click` is
/// declared, so this is the one of the three declarations that decides whether the request can
/// arrive at all.
#[test]
fn an_element_that_is_an_annotation_invites_a_click() {
    let widget = AccessibilityNode {
        annotation: Some(pdf_syntax::ObjectId::new(12, 0)),
        bounds: Some([10.0, 20.0, 30.0, 40.0]),
        ..form(Some(Control::CheckBox { on: false }))
    };
    let update = built(view(&[widget, element(None, "P", "words")], &[]));
    assert!(node(&update, NodeId(16)).supports_action(Action::Click));
    assert!(
        !node(&update, NodeId(17)).supports_action(Action::Click),
        "clicking a paragraph does nothing, so the tree may not invite it"
    );
}

/// The page says a caret may be placed in it, and only where there is text to place one in.
///
/// The same node that carries `org.a11y.atspi.Text` — see this file's `tree` module comment — is
/// where `set_caret_offset` and `set_selection` raise the action, so it is where the declaration
/// belongs. A page whose elements drew no text has no run and declares nothing.
#[test]
fn the_page_invites_a_caret_only_where_there_is_text_to_put_one_in() {
    let with_text = built(view(&[typed("abc", (0.0, 0.0), 10.0)], &[]));
    assert!(node(&with_text, NodeId(2)).supports_action(Action::SetTextSelection));

    let without = built(view(&[element(None, "Figure", "a chart")], &[]));
    assert!(!node(&without, NodeId(2)).supports_action(Action::SetTextSelection));
}

/// A column publishes one page node per page on the screen, and every page keeps its own text.
///
/// **The defect this exists for is a silence.** `Query::AccessibilityTree` answered for the page
/// being shown while a window could hold four, so a screen reader walking a continuous document
/// was handed one page's elements and had no way of learning that three more were in front of the
/// person's eyes. What that looks like on a bus is a document that is one page long.
///
/// Two things are asserted rather than one: that each page is a node of its own with the number a
/// person navigates by, and that no identifier is shared — the second is what would silently put
/// page four's paragraph under page three.
#[test]
fn a_column_publishes_one_page_node_per_page_and_no_identifier_twice() {
    let first = [element(None, "P", "the first page's paragraph")];
    let second = [element(None, "P", "the second page's paragraph")];
    let update = shown(&[
        PageView {
            page: 2,
            bounds: [0.0, 0.0, 800.0, 480.0],
            ..view(&first, &[])
        },
        PageView {
            page: 3,
            bounds: [0.0, 488.0, 800.0, 968.0],
            ..view(&second, &[])
        },
    ]);

    let document = node(&update, NodeId(1));
    assert_eq!(document.role(), Role::PdfRoot);
    assert_eq!(
        document.children().len(),
        2,
        "one page node per page on the screen"
    );
    let named: Vec<String> = document
        .children()
        .iter()
        .map(|child| node(&update, *child).label().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(named, vec!["page 3 of 3", "page 4 of 3"], "{named:?}");

    let mut identifiers: Vec<u64> = update.nodes.iter().map(|(id, _)| id.0).collect();
    let held = identifiers.len();
    identifiers.sort_unstable();
    identifiers.dedup();
    assert_eq!(identifiers.len(), held, "no node is published twice");

    // And each page's paragraph is under that page, which is what a caret moving from the end of
    // one page to the start of the next depends on.
    for (child, words) in document
        .children()
        .iter()
        .zip(["the first page's paragraph", "the second page's paragraph"])
    {
        let page = node(&update, *child);
        let paragraph = page
            .children()
            .first()
            .map(|first| node(&update, *first))
            .expect("the page holds its own paragraph");
        assert_eq!(paragraph.label().unwrap_or_default(), words);
    }
}

/// An untagged page in a column still says it is untagged, beside a tagged one that says nothing.
///
/// Trap 5, in the place a column could break it quietly: §14.7 leaves a producer free to state no
/// structure, and this reader's answer for such a page is one sentence saying so rather than an
/// invented reading order. A screen holding one tagged page and one untagged page has to say both
/// things — and a merged tree, or a page whose empty entry was dropped on the way, would say only
/// the first.
#[test]
fn an_untagged_page_beside_a_tagged_one_still_says_it_is_untagged() {
    let tagged = [element(None, "P", "a paragraph the producer tagged")];
    let update = shown(&[
        PageView {
            page: 0,
            ..view(&tagged, &[])
        },
        PageView {
            page: 1,
            ..view(&[], &[])
        },
    ]);

    let document = node(&update, NodeId(1));
    let [first, second] = document.children() else {
        panic!("two pages are on the screen");
    };
    assert_eq!(
        node(&update, node(&update, *first).children()[0]).label(),
        Some("a paragraph the producer tagged"),
        "the tagged page keeps its own element"
    );
    let untagged = node(&update, node(&update, *second).children()[0]);
    let said = untagged.value().unwrap_or_default();
    assert!(
        said.contains("states no logical structure"),
        "the untagged page says so in this program's own words: {said:?}"
    );
}
