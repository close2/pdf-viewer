//! One page's structure, as an [`accesskit::TreeUpdate`].
//!
//! # The shape
//!
//! ```text
//! Window        the window, named as its title bar is
//!  └ PdfRoot    the document
//!     ├ Group   the page: §12.4.2's label where it has one, its number where it has not
//!     │  └ …    §14.7's elements, parent-first, in §14.8.2.5's logical order
//!     └ Status  what this reader could not draw on it — present only when something was
//! ```
//!
//! The root has to be [`accesskit::Role::Window`]: `accesskit_atspi_common` treats the root
//! specially and only for that role, and a tree whose root is anything else appears on the bus
//! without the frame an assistive technology looks for.
//!
//! # What a page that is not tagged says
//!
//! 885 of this project's 974 corpus documents state no structure tree at all, and §14.7 leaves a
//! producer free to say nothing. What crosses then is one node saying so, in this program's own
//! words — not silence, and **not** an invented structure over the text layer: reading order is
//! precisely what §14.7 exists to state, and a reader that guessed one would be presenting a
//! guess in the place a person is entitled to expect the author's answer.
//!
//! # What this program refused
//!
//! A page with an unreported gap is one thing; a page whose text is not drawn at all is another,
//! and a reader that says nothing about it to the one person who cannot see the page is lying by
//! omission. `viewer_core::Query::Reports` already answers with what the current page could not
//! draw, in the words `viewer_core::report` chose for a person — the same sentences the title bar
//! counts and the terminal prints. They go into the tree as a [`accesskit::Role::Status`] group,
//! which is AT-SPI's `StatusBar`: advisory, findable, and not an alert that interrupts.

use accesskit::{Node, NodeId, Rect, Role, Toggled, Tree, TreeId, TreeUpdate};
use viewer_core::AccessibilityNode;

/// The window, and the root of the tree.
const ROOT: NodeId = NodeId(0);
/// The document inside it.
const DOCUMENT: NodeId = NodeId(1);
/// The page being shown.
const PAGE: NodeId = NodeId(2);
/// The group holding what the page could not draw.
const STATUS: NodeId = NodeId(3);
/// The first identifier a structure element may take.
///
/// Above the four fixed ones with room to spare, so that adding a fixed node later does not
/// renumber every element in a tree an assistive technology is already holding.
const ELEMENT_BASE: u64 = 16;
/// The first identifier a report may take.
///
/// Above `viewer_core`'s own bound on how many structure elements one page answers with (8192),
/// so the two ranges cannot meet however many elements a page has.
const REPORT_BASE: u64 = 1_000_000;

/// What the host knows about the page, which is everything this crate needs to build a tree.
///
/// Borrowed throughout: the caller has all of it already, and a tree is built on a page change
/// rather than on a frame.
#[derive(Debug, Clone, Copy)]
pub struct PageView<'a> {
    /// What the window is called — §12.2's `/DisplayDocTitle` having already been obeyed.
    pub window: &'a str,
    /// What the document is called.
    pub document: &'a str,
    /// The page being shown, counting from zero.
    pub page: usize,
    /// §12.4.2's page label, where the document states one.
    pub label: Option<&'a str>,
    /// How many pages the document has.
    pub pages: usize,
    /// The viewport, in device pixels — which is the space `quads` are in.
    pub viewport: (f32, f32),
    /// §14.7's elements for this page, parent-first, as `viewer_core::Query::AccessibilityTree`
    /// answers.
    pub nodes: &'a [AccessibilityNode],
    /// What this page could not draw, as `viewer_core::Query::Reports` answers.
    pub reports: &'a [String],
}

/// Builds the whole tree, which is what an assistive technology is handed when it attaches.
///
/// A full tree every time rather than a difference: `accesskit::ActivationHandler` requires one
/// on activation, a page turn replaces every node anyway, and the alternative — remembering which
/// nodes changed — would be a second model of the page kept beside the first one.
#[must_use]
pub fn build(view: &PageView) -> TreeUpdate {
    let mut nodes: Vec<(NodeId, Node)> = Vec::new();

    let mut page = Node::new(Role::Group);
    page.set_label(page_name(view));
    page.set_bounds(Rect {
        x0: 0.0,
        y0: 0.0,
        x1: f64::from(view.viewport.0),
        y1: f64::from(view.viewport.1),
    });
    let roots = elements(view, &mut nodes);
    page.set_children(roots);
    nodes.push((PAGE, page));

    let mut children = vec![PAGE];
    if !view.reports.is_empty() {
        children.push(STATUS);
        let status = reports(view, &mut nodes);
        nodes.push((STATUS, status));
    }

    let mut document = Node::new(Role::PdfRoot);
    document.set_label(view.document);
    document.set_children(children);
    nodes.push((DOCUMENT, document));

    let mut root = Node::new(Role::Window);
    root.set_label(view.window);
    root.set_children(vec![DOCUMENT]);
    nodes.push((ROOT, root));

    TreeUpdate {
        nodes,
        tree: Some(Tree::new(ROOT)),
        tree_id: TreeId::ROOT,
        // Nothing inside the tree takes the keyboard: this host aims the keyboard at §12.5's
        // annotations rather than at structure elements, and `TreeUpdate` requires the root
        // where no node within it is focused.
        focus: ROOT,
    }
}

/// What the page node is called.
///
/// §12.4.2's label where the document states one, because "[e]ach page in a PDF document shall be
/// identified by an integer page index … [i]t may also be identified by a page label" and a
/// document that numbers its front matter in roman numerals has said its third page is called
/// `iii`. The index is given beside it either way, because a person navigating needs to know how
/// far through the document they are and a label does not say.
fn page_name(view: &PageView) -> String {
    let number = view.page.saturating_add(1);
    match view.label {
        Some(label) if !label.is_empty() => {
            format!("page {label} ({number} of {})", view.pages)
        }
        _ => format!("page {number} of {}", view.pages),
    }
}

/// Appends §14.7's elements and answers with the ones the page node holds directly.
///
/// Two things happen here that the flat list does not state. **A substitution ends the walk**:
/// §14.9.3 makes `/Alt` "a complete (or whole) word or phrase substitution for the current
/// element", so an element stating one is published with that text and its descendants are not
/// published at all — speaking both would speak the thing twice, once in the author's words and
/// once in the file's. And **an element with nothing on this page never reaches here**, because
/// `viewer_core` prunes it.
fn elements(view: &PageView, out: &mut Vec<(NodeId, Node)>) -> Vec<NodeId> {
    let mut roots: Vec<NodeId> = Vec::new();
    // What each cell named as a header would be said as, built once for the whole page.
    let spoken = spoken_headers(view);
    // One entry per element of the answer: the children collected for it so far, and whether it
    // is published at all. Parent-first ordering means a node's parent is always already here.
    let mut children: Vec<Vec<NodeId>> = vec![Vec::new(); view.nodes.len()];
    let mut published: Vec<bool> = vec![false; view.nodes.len()];

    for (index, node) in view.nodes.iter().enumerate() {
        let publish = match node.parent {
            None => true,
            // Published only under a published parent that has not already spoken for it.
            Some(parent) => {
                published.get(parent).copied().unwrap_or(false)
                    && !view
                        .nodes
                        .get(parent)
                        .is_some_and(|parent| parent.substituted)
            }
        };
        if !publish {
            continue;
        }
        if let Some(slot) = published.get_mut(index) {
            *slot = true;
        }
        let id = element_id(index);
        match node.parent.and_then(|parent| children.get_mut(parent)) {
            Some(list) => list.push(id),
            None => roots.push(id),
        }
    }

    // The nodes themselves, in the same order, once every child list is complete.
    for (index, node) in view.nodes.iter().enumerate() {
        if !published.get(index).copied().unwrap_or(false) {
            continue;
        }
        let mapping = crate::role::map(
            &node.role,
            !node.name.trim().is_empty(),
            node.header_scope,
            node.control.as_ref(),
        );
        let mut built = Node::new(mapping.role);
        if mapping.speaks && !node.name.is_empty() {
            say(&mut built, &node.name);
        }
        if let Some(level) = mapping.level {
            built.set_level(usize::try_from(level).unwrap_or(usize::MAX));
        }
        // §12.7.5.2's two toggling buttons, whose state is half of what the control means. AT-SPI
        // reads this as the `checked` state, which is what a screen reader announces after the
        // control's name.
        if let Some(on) = mapping.toggled {
            built.set_toggled(if on { Toggled::True } else { Toggled::False });
        }
        let mut description: Vec<String> = Vec::new();
        if let Some(name) = mapping.unmapped {
            description.push(format!(
                "structure type {name}, which ISO 32000-2 §14.8.4 does not define and this \
                 document's role map does not map"
            ));
        } else if let Some(note) = mapping.note {
            // What the platform's vocabulary could not carry, in the description rather than in
            // the name: it is about the *reading* of the cell and not what the cell says.
            description.push(note.to_owned());
        }
        description.extend(headers(node, &spoken));
        if !description.is_empty() {
            built.set_description(description.join("; "));
        }
        if let Some(language) = node.language.as_deref() {
            built.set_language(language);
        }
        if let Some(bounds) = place(node) {
            built.set_bounds(bounds);
        }
        if let Some(list) = children.get(index) {
            built.set_children(list.clone());
        }
        out.push((element_id(index), built));
    }

    if view.nodes.is_empty() {
        // §14.7 leaves a producer free to say nothing about its own structure, and this is what
        // "nothing" sounds like. A statement about the *document*, not about this reader.
        let id = NodeId(ELEMENT_BASE);
        let mut untagged = Node::new(Role::Label);
        say(
            &mut untagged,
            "this document states no logical structure (ISO 32000-2 §14.7), so this reader can \
             offer no reading order for the page's text",
        );
        out.push((id, untagged));
        roots.push(id);
    }
    roots
}

/// What a table cell's header cells are, in words a person is meant to hear.
///
/// §14.8.4.8.3's whole purpose is what a reader does with the answer, and Table 384's `/Short`
/// says so outright: "[w]hen accessed by means of a screen reader, for each table cell the
/// applicable header cells are read to the user in order to allow that user to understand the
/// content of the table cell." So the headers have to reach a person, and on this platform the
/// **description** is the only channel that does — which is a choice about a platform rather than
/// a reading of the standard, and the two alternatives are why:
///
/// - **`accesskit::Node::set_labelled_by` is the relation this is**, and it reaches nobody:
///   `accesskit_atspi_common`'s `relation_set` builds exactly one relation, `ControllerFor`, out of
///   `Node::controls`, so a `LabelledBy` set here would stop at the crate. Worse than inert,
///   because `accesskit_consumer::Node::label` *falls back* to the labelled-by nodes' text where a
///   node has no label of its own — so an empty table cell would be announced as its own headers.
/// - **AT-SPI's `Table` and `TableCell` interfaces** are where a client would ordinarily ask, and
///   that adapter implements neither. `doc/todo/31` records it as owed to the platform.
///
/// The order is the standard's, and it is stated rather than assumed by the listener: the row's
/// headers first, then the column's, each from most specific to most general.
///
/// `None` where the cell has no headers, and where none of them has anything to say — a header
/// cell whose content drew no text has no name, and naming it with an empty string would put a
/// stray comma in the middle of a sentence somebody is listening to.
fn headers(node: &AccessibilityNode, spoken: &[String]) -> Option<String> {
    let named: Vec<&str> = node
        .headers
        .iter()
        .filter_map(|at| spoken.get(*at))
        .map(String::as_str)
        .filter(|name| !name.is_empty())
        .collect();
    (!named.is_empty()).then(|| format!("headers, most specific first: {}", named.join(", ")))
}

/// What each element named as a header cell would be *said* as, indexed as the answer is.
///
/// **A header cell's text is usually not its own**, and that is what reading this back off a bus
/// found: `bug2014080.pdf` puts each cell's words in a `P` inside the cell, so every `TH` in it has
/// an empty [`AccessibilityNode::name`] — which is correct, because that field is deliberately the
/// element's own text and not its subtree's, and a container repeating its children's would be
/// read twice. A header *named by another cell* is the one place where the subtree is what is
/// wanted: nothing is descending into it, so nothing would say the words at all.
///
/// The subtree is a contiguous run, because the answer is `viewer_core`'s depth-first walk with
/// elements removed rather than reordered — so this stops at the first node that is not below the
/// one it started at. An element stating §14.9.3's `/Alt` ends the descent for the reason
/// [`elements`] gives: the substitution is what to say instead of what is under it.
///
/// Only the elements some cell names are computed, and each of them once: a header row is named by
/// every cell in its column, and building its text per cell would be the same string over again.
fn spoken_headers(view: &PageView) -> Vec<String> {
    let mut out = vec![String::new(); view.nodes.len()];
    let mut wanted: Vec<usize> = view
        .nodes
        .iter()
        .flat_map(|node| node.headers.iter().copied())
        .collect();
    wanted.sort_unstable();
    wanted.dedup();
    for at in wanted {
        let Some(header) = view.nodes.get(at) else {
            continue;
        };
        let mut text = header.name.trim().to_owned();
        let mut inside = vec![at];
        let mut silenced: Vec<usize> = if header.substituted {
            vec![at]
        } else {
            Vec::new()
        };
        for (index, node) in view.nodes.iter().enumerate().skip(at.saturating_add(1)) {
            let Some(parent) = node.parent.filter(|parent| inside.contains(parent)) else {
                break;
            };
            inside.push(index);
            if silenced.contains(&parent) {
                silenced.push(index);
                continue;
            }
            let words = node.name.trim();
            if !words.is_empty() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(words);
            }
            if node.substituted {
                silenced.push(index);
            }
        }
        if let Some(slot) = out.get_mut(at) {
            *slot = text;
        }
    }
    out
}

/// Puts a node's text where the platform will read it from.
///
/// **[`Role::Label`] is the one role whose accessible name comes from its `value` rather than its
/// `label`.** `accesskit_consumer::Node::label_comes_from_value` is `self.role() == Role::Label`
/// and nothing else, and the AT-SPI adapter's `name` follows it — so a static-text node with its
/// text set as a label reaches an assistive technology with an empty name and nothing says so.
///
/// Found by reading the tree back off the bus rather than by reading the code: the first
/// end-to-end run of ADR 0214 showed every `Label` node named `''` beside paragraphs that were
/// named correctly. It is exactly the defect a test that stops at `TreeUpdate` cannot see.
fn say(node: &mut Node, text: &str) {
    if node.role() == Role::Label {
        node.set_value(text);
    } else {
        node.set_label(text);
    }
}

/// The identifier one of the answer's elements takes.
fn element_id(index: usize) -> NodeId {
    NodeId(ELEMENT_BASE.saturating_add(u64::try_from(index).unwrap_or(u64::MAX)))
}

/// The group holding what the page could not draw, and one node per item.
fn reports(view: &PageView, out: &mut Vec<(NodeId, Node)>) -> Node {
    let mut children = Vec::new();
    for (index, note) in view.reports.iter().enumerate() {
        let id = NodeId(REPORT_BASE.saturating_add(u64::try_from(index).unwrap_or(u64::MAX)));
        let mut item = Node::new(Role::Label);
        say(&mut item, note);
        out.push((id, item));
        children.push(id);
    }
    let mut group = Node::new(Role::Status);
    group.set_label(format!(
        "{} thing(s) on this page were not drawn as the document specifies",
        view.reports.len()
    ));
    group.set_children(children);
    group
}

/// Where a magnifier is pointed at this element, and which of two statements decides it.
///
/// The shapes this program drew come first: they are what is on the screen, measured from the
/// marks rather than declared, and they are what the ring should sit on wherever they exist.
/// **Table 379's `/BBox` is what answers where they do not** — an element that marks no text, a
/// `Figure` or a cell holding an image, whose extent no text layer can recover. §14.8.5.4.3
/// states it as "the rectangle that completely encloses its visible content", which is exactly
/// the question being asked here, and `viewer_core` has already mapped it into these pixels.
///
/// Two statements rather than one merged rectangle, and the order is the conservative one: the
/// document's number is a *claim* about a layout this program has already carried out, so it is
/// used where there is nothing to compare it against and not in place of what was drawn. Whether
/// a stated `/BBox` should win over measured text — a `Figure` holding a caption and a picture
/// has both, and the text quads cover only the caption — is a question nothing has measured, and
/// `doc/todo/31` carries it.
///
/// `None` where the element has neither: an untagged region, or an element reached only through
/// §14.7.5.3's object reference and stating no bounds.
fn place(node: &AccessibilityNode) -> Option<Rect> {
    bounding_box(&node.quads).or_else(|| {
        node.bounds.map(|rect| Rect {
            x0: f64::from(rect[0]),
            y0: f64::from(rect[1]),
            x1: f64::from(rect[2]),
            y1: f64::from(rect[3]),
        })
    })
}

/// The smallest axis-aligned rectangle covering an element's quadrilaterals.
///
/// AccessKit takes a rectangle and `viewer_core` answers with quadrilaterals, because a page's
/// own space may be rotated or sheared and a selection has to be drawn in it. A screen reader
/// wants somewhere to point a magnifier, so the bounding box is the right loss to take — and
/// `None` where the element covers nothing, which is a figure, a table cell holding an image, or
/// an element reached only through §14.7.5.3's object reference.
fn bounding_box(quads: &[[f32; 8]]) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    for quad in quads {
        for corner in quad.chunks_exact(2) {
            let (Some(&x), Some(&y)) = (corner.first(), corner.get(1)) else {
                continue;
            };
            let (x, y) = (f64::from(x), f64::from(y));
            bounds = Some(match bounds {
                None => Rect {
                    x0: x,
                    y0: y,
                    x1: x,
                    y1: y,
                },
                Some(rect) => Rect {
                    x0: rect.x0.min(x),
                    y0: rect.y0.min(y),
                    x1: rect.x1.max(x),
                    y1: rect.y1.max(y),
                },
            });
        }
    }
    bounds
}
