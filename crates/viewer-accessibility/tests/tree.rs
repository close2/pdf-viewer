//! The tree an assistive technology is handed, built from what `viewer-core` answers with.
//!
//! Plain data on both sides, so these run on every platform — which is the point of the crate
//! being in two halves. What the *bus* does with the result is `tests/atspi.rs`'s question and
//! needs a session bus, so it is not here.

#![expect(
    clippy::panic,
    reason = "a test asserts; a failed assertion is the point"
)]

use accesskit::{Node, NodeId, Role};
use pdf_model::structure::HeaderScope;
use viewer_accessibility::{PageView, tree};
use viewer_core::AccessibilityNode;

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
        window: "a window",
        document: "a document",
        page: 0,
        label: None,
        pages: 3,
        viewport: (800.0, 1000.0),
        nodes,
        reports,
    }
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
    let update = tree::build(&view(&nodes, &[]));
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
    let update = tree::build(&view(&nodes, &[]));
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
    let update = tree::build(&view(&nodes, &[]));
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
    let update = tree::build(&view(&[], &[]));
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
    let update = tree::build(&view(&nodes, &reports));
    let status = node(&update, NodeId(3));
    assert_eq!(status.role(), Role::Status);
    assert_eq!(status.children().len(), 2);
    assert_eq!(
        node(&update, NodeId(1_000_001)).value(),
        Some("a font could not be loaded, so 24 characters drew nothing")
    );

    // And a page with nothing to report grows no such group.
    let quiet = tree::build(&view(&nodes, &[]));
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
    let update = tree::build(&view(&nodes, &[]));
    let bounds = node(&update, NodeId(16))
        .bounds()
        .expect("it covers the lines");
    assert!((bounds.x0 - 10.0).abs() < 1e-6, "{bounds:?}");
    assert!((bounds.y0 - 20.0).abs() < 1e-6, "{bounds:?}");
    assert!((bounds.x1 - 110.0).abs() < 1e-6, "{bounds:?}");
    assert!((bounds.y1 - 46.0).abs() < 1e-6, "{bounds:?}");
}

/// §12.4.2's page label names the page where the document states one.
#[test]
fn the_page_is_named_by_its_label_where_there_is_one() {
    let mut labelled = view(&[], &[]);
    labelled.label = Some("iii");
    labelled.page = 2;
    let update = tree::build(&labelled);
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
    let update = tree::build(&view(&nodes, &[]));
    let span = node(&update, NodeId(16));
    assert_eq!(span.role(), Role::Label);
    assert_eq!(span.value(), Some("a run of text"));
    assert_eq!(span.label(), None);

    // And a role that is not `Label` keeps its text in the label, which is where that role's
    // name comes from.
    let paragraphs = [element(None, "P", "a paragraph")];
    let update = tree::build(&view(&paragraphs, &[]));
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
    let update = tree::build(&view(&nodes, &[]));

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
