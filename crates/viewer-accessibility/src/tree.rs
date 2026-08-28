//! The structure of every page on the screen, as an [`accesskit::TreeUpdate`].
//!
//! # The shape
//!
//! ```text
//! Window          the window, named as its title bar is
//!  └ PdfRoot      the document
//!     ├ Document  a page: §12.4.2's label where it has one, its number where it has not
//!     │  └ …      §14.7's elements, parent-first, in §14.8.2.5's logical order
//!     │     └ …   each element's own text as [`Role::TextRun`], one per line — see below
//!     ├ Status    what this reader could not draw on that page — only when something was
//!     ├ Document  the next page Table 29's arrangement is showing, if it shows one
//!     └ …
//! ```
//!
//! # A column is several page nodes, not one longer page
//!
//! Table 29's five continuous arrangements put several pages in one window, and each of them
//! crosses as a page node of its own with a band of identifiers of its own ([`Band`]). Two
//! reasons, and neither is convenience. **§14.7's tree does not compose**: a page's elements are
//! reached through §14.7.5.4's structural parent tree keyed by that page's own `/StructParents`,
//! two pages that share an ancestor each answer with it, and §14.8.2.5 states an order *within* a
//! structure tree and none between two pages — so a merged list would say a section twice and
//! could not say which came first. **And a page is what a person navigates**: §12.4.2 makes a
//! page something a document *names*, so a client that has just read the end of one page is
//! entitled to be told that what follows is page 4 rather than more of page 3.
//!
//! The root has to be [`accesskit::Role::Window`]: `accesskit_atspi_common` treats the root
//! specially and only for that role, and a tree whose root is anything else appears on the bus
//! without the frame an assistive technology looks for.
//!
//! # A caret, and the one node that may carry it
//!
//! A structure element crossed as a node with a name, so an assistive technology could read a
//! paragraph and could not move through it. Moving by character, by word or by line, and saying
//! where the caret is, is `org.a11y.atspi.Text`, and AccessKit builds that interface out of
//! [`Role::TextRun`] children carrying each character's length, position and width — which this
//! program has, one per character code, since ADR 0118's text layer.
//!
//! **Which node gets the interface is AccessKit's decision and not this crate's, and it is the
//! whole reason the page is a [`Role::Document`].** `accesskit_consumer::Node::supports_text_ranges`
//! is
//!
//! ```text
//! (self.is_text_input() || matches!(self.role(), Role::Label | Role::Document | Role::Terminal))
//!     && self.text_runs().next().is_some()
//! ```
//!
//! so **no role any §14.8.4 type maps to can carry it**: not `Paragraph`, not `Heading`, not
//! `Cell`. Putting the interface on the paragraphs would mean mapping `P` to `Label`, which trades
//! the vocabulary §14.8.4 spends five tables defining for a platform interface — the opposite of
//! the trade ADR 0338 made when it refused to call a signature field a button. So the interface
//! goes on the **page**, which is this program's own node rather than a structure element's: it
//! was an unnamed group standing between the document and its elements, and AT-SPI's
//! `DocumentFrame` is what a text-bearing document container is there. Every element keeps the
//! role §14.8.4 gives it, and the caret crosses the page in §14.8.2.5's logical order because
//! that is the order the runs are in.
//!
//! `common_filter` answers `ExcludeNode` for a [`Role::TextRun`], so **not one of the runs appears
//! on the bus**: the tree an assistive technology walks is the same tree it walked before, with an
//! interface on it that was not there. What this does not do is give each *element* its own text
//! interface — a client asking a paragraph for `org.a11y.atspi.Text` still gets nothing, and that
//! is the platform's answer rather than this one's. `doc/todo/31` records it as owed upstream,
//! beside `Table`, `TableCell` and the relation set.
//!
//! # What a client may ask a node to do
//!
//! A tree that only *says* things invites nothing, and this one declared no [`Action`] at all
//! until the five-hundred-and-ninetieth session. Three are declared now, each on the condition
//! that makes it answerable rather than on the role that suggests it:
//!
//! | action | on | because |
//! |---|---|---|
//! | [`Action::ScrollIntoView`] | an element with a place | a scroll takes a rectangle, and an element with no place names none |
//! | [`Action::Click`] | an element whose content is an annotation | §12.5.1: "[w]hen the user activates the annotation by clicking it, it exhibits its associated object" |
//! | [`Action::SetTextSelection`] | the page, where it has runs | the interface a caret moves through is on this node and nowhere else, for the reason above |
//!
//! **Two of the three would arrive whether or not they were declared**, which is why declaring
//! them is about honesty rather than about capability: `accesskit_atspi_common` offers
//! `Component.ScrollTo` to any node with bounds and `Text.SetCaretOffset` to whatever supports
//! text ranges, both without consulting the action set. Only [`Action::Click`] is gated on the
//! declaration — it is what puts `org.a11y.atspi.Action` on the node at all. So the tree now
//! invites exactly what it answers, and [`crate::Bridge::requested`] turns each into a place.
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

use accesskit::{
    Action, Node, NodeId, Rect, Role, TextDirection, Toggled, Tree, TreeId, TreeUpdate,
};
use viewer_core::AccessibilityNode;

/// The window, and the root of the tree.
const ROOT: NodeId = NodeId(0);
/// The document inside it.
const DOCUMENT: NodeId = NodeId(1);
/// The page node of the first page on the screen.
const PAGE: u64 = 2;
/// The group holding what that page could not draw.
const STATUS: u64 = 3;
/// The first identifier a structure element may take, within one page's band.
///
/// Above the four fixed ones with room to spare, so that adding a fixed node later does not
/// renumber every element in a tree an assistive technology is already holding.
const ELEMENT_BASE: u64 = 16;
/// The first identifier a report may take, within one page's band.
///
/// Above `viewer_core`'s own bound on how many structure elements one page answers with (8192),
/// so the two ranges cannot meet however many elements a page has.
const REPORT_BASE: u64 = 1_000_000;
/// The first identifier a line of text may take, within one page's band.
///
/// A range of its own above the reports', for the reason [`ELEMENT_BASE`] has one: a page's lines
/// are counted in thousands where its elements are counted in tens, and a scheme that derived a
/// line's identifier from its element's would have to bound the lines per element to stay
/// injective.
const RUN_BASE: u64 = 2_000_000;
/// How far apart two pages' identifier bands are.
///
/// **Table 29's continuous arrangements are what made this necessary**: a column publishes one
/// page node per page on the screen and every identifier under it has to be distinct from every
/// other page's, so each page is given a band of its own and the four ranges above are offsets
/// inside it. The first page's band starts at zero, so a `SinglePage` tree carries exactly the
/// identifiers it carried before — a client holding one across a layout change sees the page it
/// was reading keep its nodes.
///
/// Wide enough that no page can reach the next band: a page's elements are bounded by
/// `viewer_core`'s 8192 and its runs by the characters it drew, and 2^32 is four billion of them
/// on a page whose text is measured in thousands. [`ceiling`] is where that is enforced rather
/// than asserted.
const BAND: u64 = 1 << 32;

/// One page's identifier band: the four ranges above, offset by which page on the screen it is.
#[derive(Debug, Clone, Copy)]
struct Band(u64);

impl Band {
    /// The band of the `slot`th page on the screen, counting from zero.
    fn of(slot: usize) -> Self {
        Self(u64::try_from(slot).unwrap_or(u64::MAX).saturating_mul(BAND))
    }

    /// The page node itself.
    fn page(self) -> NodeId {
        NodeId(self.0.saturating_add(PAGE))
    }

    /// The group holding what the page could not draw.
    fn status(self) -> NodeId {
        NodeId(self.0.saturating_add(STATUS))
    }

    /// The identifier one of the answer's elements takes.
    fn element(self, index: usize) -> NodeId {
        NodeId(
            self.0
                .saturating_add(ELEMENT_BASE)
                .saturating_add(ceiling(index)),
        )
    }

    /// The identifier one of the status group's items takes.
    fn report(self, index: usize) -> NodeId {
        NodeId(
            self.0
                .saturating_add(REPORT_BASE)
                .saturating_add(ceiling(index)),
        )
    }

    /// The identifier one line's run takes.
    fn run(self, allocated: u64) -> NodeId {
        NodeId(
            self.0
                .saturating_add(RUN_BASE)
                .saturating_add(allocated.min(BAND - RUN_BASE - 1)),
        )
    }
}

/// An offset held inside a band, so that a page can never reach the next page's identifiers.
///
/// A clamp rather than an `assert`: this is a tree handed to an assistive technology and a panic
/// there would take the window with it, while two nodes sharing an identifier would put a page's
/// text under another page in silence. The bound is unreachable by any document — it is four
/// billion elements or runs on one page — and it is here because "unreachable" is a claim about
/// documents that exist rather than about documents that can be written.
fn ceiling(index: usize) -> u64 {
    u64::try_from(index)
        .unwrap_or(u64::MAX)
        .min(BAND - RUN_BASE - 1)
}
/// How many characters one [`Role::TextRun`] may hold.
///
/// AccessKit's word starts are indices into the run's characters, held in a `u8`, so a run of more
/// than this could not state where its later words begin. See [`runs`].
const MAX_RUN: usize = u8::MAX as usize;

/// What the host knows about the window and the pages in it, which is everything this crate needs.
///
/// Borrowed throughout: the caller has all of it already, and a tree is built on a page change
/// rather than on a frame.
#[derive(Debug, Clone, Copy)]
pub struct DocumentView<'a> {
    /// What the window is called — §12.2's `/DisplayDocTitle` having already been obeyed.
    pub window: &'a str,
    /// What the document is called.
    pub document: &'a str,
    /// How many pages the document has.
    pub pages: usize,
    /// The viewport, in device pixels — which is the space every `quads` is in.
    pub viewport: (f32, f32),
    /// The pages Table 29's arrangement is showing, in page order.
    ///
    /// **One entry, and only one, under `SinglePage`** — and several under the five continuous
    /// arrangements, which is what this type exists for: a screen reader handed the current page's
    /// tree while the window shows four is being told the document is one page long, and it is the
    /// person who cannot see the other three who would never find out.
    pub shown: &'a [PageView<'a>],
}

/// What the host knows about one page on the screen.
#[derive(Debug, Clone, Copy)]
pub struct PageView<'a> {
    /// Which page this is, counting from zero.
    pub page: usize,
    /// §12.4.2's page label, where the document states one.
    pub label: Option<&'a str>,
    /// Where this page sits in the viewport, `[x0, y0, x1, y1]` in device pixels.
    ///
    /// What `viewer_core::Query::PageGeometry` answers, mapped to two corners. A page rather than
    /// the whole window, because under a column two page nodes with the window's extents would be
    /// two nodes claiming the same place and a client asking where it is reading would be told
    /// "everywhere".
    pub bounds: [f32; 4],
    /// §14.7's elements for this page, parent-first, as `viewer_core::Query::AccessibilityTree`
    /// answers for it.
    pub nodes: &'a [AccessibilityNode],
    /// What this page could not draw, as `viewer_core::Query::Reports` answers.
    pub reports: &'a [String],
    /// What this page could not be *read* as, as `viewer_core::Query::Readback` answers.
    ///
    /// **The half of the same omission that had no channel.** The reports above are what the
    /// interpreter could not draw; this is the text a person who cannot see the page is being
    /// spoken *short* of, on a page that drew perfectly — ISO 32000-2 §9.10.2's own "there is no
    /// way to determine what the character code represents", counted. It is not a report and must
    /// not become one (ADR 0422), and it belongs here for the reason the reports do: the person
    /// for whom the picture is no answer is the one entitled to know the words are missing.
    ///
    /// [`Default::default`] is a page whose every code was named and drawn, which is most of them.
    pub readback: pdf_model::content::Shortfall,
}

/// Builds the whole tree, which is what an assistive technology is handed when it attaches.
///
/// A full tree every time rather than a difference: `accesskit::ActivationHandler` requires one
/// on activation, a page turn replaces every node anyway, and the alternative — remembering which
/// nodes changed — would be a second model of the page kept beside the first one.
#[must_use]
pub fn build(view: &DocumentView) -> TreeUpdate {
    let mut nodes: Vec<(NodeId, Node)> = Vec::new();
    let mut children: Vec<NodeId> = Vec::new();

    // One page node per page the arrangement is showing, in page order, each with a band of
    // identifiers of its own. They are siblings under the document rather than one merged tree:
    // §14.7's structure is the document's, but the route to a page's elements is §14.7.5.4's
    // structural parent tree keyed by *that page's* `/StructParents`, and two pages' answers
    // share their ancestors — so a merge would state a section twice and §14.8.2.5 gives no
    // order between two pages to merge them in.
    for (slot, shown) in view.shown.iter().enumerate() {
        let band = Band::of(slot);
        let mut page = Node::new(Role::Document);
        page.set_label(page_name(shown, view.pages));
        page.set_bounds(Rect {
            x0: f64::from(shown.bounds[0]),
            y0: f64::from(shown.bounds[1]),
            x1: f64::from(shown.bounds[2]),
            y1: f64::from(shown.bounds[3]),
        });
        let before = nodes.len();
        let roots = elements(shown, band, &mut nodes);
        // §14.8.2.5's order as somewhere a caret may be *put*, which is the other half of the
        // interface ADR 0394 gave this node: `accesskit_atspi_common`'s `set_caret_offset` and
        // `set_selection` both raise [`Action::SetTextSelection`] on whatever carries the text,
        // and that is this node for the reason the module comment gives. Declared only where
        // there is something to put a caret in — an empty claim would be a client offered a caret
        // it cannot place. Asked of this page's own nodes, because a column's other pages have
        // theirs in the same list.
        if nodes
            .iter()
            .skip(before)
            .any(|(id, _)| id.0 >= band.0.saturating_add(RUN_BASE))
        {
            page.add_action(Action::SetTextSelection);
        }
        page.set_children(roots);
        nodes.push((band.page(), page));
        children.push(band.page());
        if !shown.reports.is_empty() || !shown.readback.is_whole() {
            children.push(band.status());
            let status = reports(shown, band, &mut nodes);
            nodes.push((band.status(), status));
        }
    }

    let mut document = Node::new(Role::PdfRoot);
    document.set_label(view.document);
    document.set_bounds(Rect {
        x0: 0.0,
        y0: 0.0,
        x1: f64::from(view.viewport.0),
        y1: f64::from(view.viewport.1),
    });
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
fn page_name(view: &PageView, of: usize) -> String {
    let number = view.page.saturating_add(1);
    match view.label {
        Some(label) if !label.is_empty() => {
            format!("page {label} ({number} of {of})")
        }
        _ => format!("page {number} of {of}"),
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
fn elements(view: &PageView, band: Band, out: &mut Vec<(NodeId, Node)>) -> Vec<NodeId> {
    let mut roots: Vec<NodeId> = Vec::new();
    // What each cell named as a header would be said as, built once for the whole page.
    let spoken = spoken_headers(view);
    // §14.8.5.5's continuation, turned round: the answer says which list each one continues, and
    // the relation this platform has runs the other way. See [`continued_by`].
    let continued_by = continued_by(view);
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
        let id = band.element(index);
        match node.parent.and_then(|parent| children.get_mut(parent)) {
            Some(list) => list.push(id),
            None => roots.push(id),
        }
    }

    // The nodes themselves, in the same order, once every child list is complete.
    let mut allocated: u64 = 0;
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
        // §14.8.5.7's `/Summary` — "[a] summary of the table's purpose and structure", written
        // "[f]or use in non-visual rendering such as speech or braille" — on the same channel
        // as the headers below and for the same reason: the description is the one channel of
        // this platform that reaches a person, and the summary is *about* the table, so putting
        // it in the name would speak it as if the table's content said it. The word in front
        // says which kind of sentence is coming, exactly as the headers' prefix does.
        if let Some(summary) = node.summary.as_deref() {
            description.push(format!("summary: {summary}"));
        }
        description.extend(continuation(node));
        description.extend(headers(node, &spoken));
        if !description.is_empty() {
            built.set_description(description.join("; "));
        }
        if let Some(language) = node.language.as_deref() {
            built.set_language(language);
        }
        if let Some(bounds) = place(node) {
            built.set_bounds(bounds);
            // **An element that has a place can be brought into view**, and that is the whole
            // condition: `Command::Scroll` takes a rectangle of the viewport and an element with no
            // place names none. AT-SPI reaches this through `Component.ScrollTo`, which the adapter
            // offers for any node with bounds whether or not the action is declared — so declaring
            // it is what stops the tree inviting a request it means to decline (trap 5), rather
            // than what makes the request possible.
            built.add_action(Action::ScrollIntoView);
        }
        // **And an element that *is* an annotation can be clicked.** §12.5.1: "[w]hen the user
        // activates the annotation by clicking it, it exhibits its associated object, such as by
        // opening a popup window displaying a text note." Table 368 gives three structure types
        // whose content is an annotation — `Link`, `Annot` and `Form` — and §14.7.5.3's object
        // reference is how each names one, which is exactly what
        // [`AccessibilityNode::annotation`] carries. A paragraph declares no click, because
        // clicking a paragraph does nothing and a declared action that does nothing is worse than
        // an absent one.
        if node.annotation.is_some() {
            built.add_action(Action::Click);
        }
        flows_to(&mut built, band, continued_by.get(index), &published);
        // §14.8.2.5's order within one element is the file's `/K` order, and the answer this crate
        // is handed does not record where an element's own content items sat among its child
        // elements — so the lines go first and the child elements after them. It costs nothing on
        // the elements that have both kinds in the corpus, because an element with text of its own
        // and structure elements below it is rare: a `P` has text and no element children, a
        // `Sect` has element children and no text. Where a document does write both, a caret
        // crosses this element's own words before its children's rather than between them.
        let mut below: Vec<NodeId> = runs(node, band, mapping.speaks, &mut allocated, out);
        if let Some(list) = children.get(index) {
            below.extend(list.iter().copied());
        }
        if !below.is_empty() {
            built.set_children(below);
        }
        out.push((band.element(index), built));
    }

    if view.nodes.is_empty() {
        // §14.7 leaves a producer free to say nothing about its own structure, and this is what
        // "nothing" sounds like. A statement about the *document*, not about this reader.
        let id = band.element(0);
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

/// One [`Role::TextRun`] per line of the element's own text, appended, and their identifiers.
///
/// # What this is for, and why the nodes are invisible
///
/// A structure element crosses as one node with one name, so an assistive technology reads the
/// paragraph or it does not. Moving through it — by character, by word, by line — and saying where
/// the caret is, is `org.a11y.atspi.Text`, and AccessKit builds that interface out of nodes of
/// this one role: `accesskit_consumer`'s `supports_text_ranges` asks for text runs below the node
/// and `character_index_at_point` reads their arrays. The runs themselves are `ExcludeNode` in
/// `common_filter`, which every platform adapter applies, so **none of them appears on the bus**:
/// what the change adds is an interface on the nodes that were already there, not nodes.
///
/// # An artifact has no run, for the reason it has no name
///
/// §14.8.2.2.1 divides a page into the author's content and everything "generated by the PDF
/// writer in the course of pagination, layout, or other mechanical processes or introduced by the
/// document author for decoration", and a caret that walked through a running head would be
/// walking through what tagging exists to keep out of the reading. The same `speaks` that
/// withholds the label withholds these.
///
/// # What the platform is told about direction, and where that comes from
///
/// AccessKit states a run's character positions "in the direction given by `text_direction`", so
/// the direction is not decoration — it is the axis the numbers are measured along. Nothing in the
/// document states it: §9.4.4's text matrix says where each glyph landed and a reader has to look.
/// So it is read off the glyphs, first to last, and a run of one character is left-to-right
/// because there is nothing to read. A `/Rotate 90` page comes out top-to-bottom, which is the
/// truth about the screen rather than about the text.
/// # A line may be more than one run, and the two are not the same thing
///
/// AccessKit's word starts are indices into a run's characters held in a `u8`, so a run of more
/// than [`MAX_RUN`] characters could not say where its later words begin. A line longer than that
/// is therefore published as several runs joined by `previous_on_line` and `next_on_line`, which
/// is the pair `accesskit_consumer::is_line_start` and `is_line_end` read: the line stays one line
/// to a caret moving by line, and the arrays stay addressable. It is not hypothetical — a
/// two-column page of ISO 32000-2 has lines of about ninety characters and a full-width table row
/// has more.
fn runs(
    node: &AccessibilityNode,
    band: Band,
    speaks: bool,
    allocated: &mut u64,
    out: &mut Vec<(NodeId, Node)>,
) -> Vec<NodeId> {
    if !speaks {
        return Vec::new();
    }
    let mut ids = Vec::new();
    for line in &node.lines {
        let direction = direction(line);
        let mut on_line: Vec<NodeId> = Vec::new();
        for characters in chunked(line) {
            let Some(bounds) = surrounding(&characters) else {
                continue;
            };
            let mut run = Node::new(Role::TextRun);
            let mut value = String::new();
            let mut lengths: Vec<u8> = Vec::with_capacity(characters.len());
            let mut positions: Vec<f32> = Vec::with_capacity(characters.len());
            let mut widths: Vec<f32> = Vec::with_capacity(characters.len());
            for (text, length, place) in &characters {
                let (position, width) = along(*place, bounds, direction);
                value.push_str(text);
                lengths.push(*length);
                positions.push(position);
                widths.push(width);
            }
            run.set_value(value);
            run.set_bounds(bounds);
            run.set_text_direction(direction);
            run.set_word_starts(word_starts(&characters));
            run.set_character_lengths(lengths);
            run.set_character_positions(positions);
            run.set_character_widths(widths);
            let id = band.run(*allocated);
            *allocated = allocated.saturating_add(1);
            out.push((id, run));
            on_line.push(id);
        }
        join(&on_line, out);
        ids.extend(on_line);
    }
    ids
}

/// A line's characters as text and place, in runs of at most [`MAX_RUN`].
///
/// A character whose readback does not fit a `u8` is left out with its text: AccessKit's character
/// lengths are bytes in a `u8` and their sum must equal the run's value, so carrying such a
/// character with a truncated length would break the invariant the whole interface is indexed by.
/// It takes a `/ToUnicode` mapping one code to more than 255 bytes, which no document does and
/// which a hostile one may.
fn chunked(line: &viewer_core::TextLine) -> Vec<Vec<(&str, u8, [f32; 4])>> {
    let mut chunks: Vec<Vec<(&str, u8, [f32; 4])>> = Vec::new();
    let mut at = 0usize;
    for character in &line.characters {
        let text = line.text.get(at..at.saturating_add(character.bytes));
        at = at.saturating_add(character.bytes);
        let Some((text, length)) = text
            .filter(|text| !text.is_empty())
            .and_then(|text| Some((text, u8::try_from(text.len()).ok()?)))
        else {
            continue;
        };
        match chunks.last_mut() {
            Some(chunk) if chunk.len() < MAX_RUN => chunk.push((text, length, character.bounds)),
            _ => chunks.push(vec![(text, length, character.bounds)]),
        }
    }
    chunks
}

/// Links the runs of one line, so that a caret moving by line crosses all of them.
fn join(on_line: &[NodeId], out: &mut [(NodeId, Node)]) {
    for pair in on_line.windows(2) {
        let (Some(first), Some(second)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        for (id, node) in out.iter_mut() {
            if id == first {
                node.set_next_on_line(*second);
            } else if id == second {
                node.set_previous_on_line(*first);
            }
        }
    }
}

/// The rectangle a run's characters cover, or `None` for a run with none.
fn surrounding(characters: &[(&str, u8, [f32; 4])]) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    for (_, _, place) in characters {
        let rect = Rect {
            x0: f64::from(place[0]),
            y0: f64::from(place[1]),
            x1: f64::from(place[2]),
            y1: f64::from(place[3]),
        };
        bounds = Some(match bounds {
            None => rect,
            Some(so_far) => Rect {
                x0: so_far.x0.min(rect.x0),
                y0: so_far.y0.min(rect.y0),
                x1: so_far.x1.max(rect.x1),
                y1: so_far.y1.max(rect.y1),
            },
        });
    }
    bounds
}

/// Which way a line runs, read off where its first and last glyphs are.
///
/// The larger of the two displacements decides the axis and its sign decides the way round. A line
/// of one character has no displacement and is called left to right, which is a default and is
/// marked as one: a single glyph is the same rectangle whichever direction is claimed for it, so
/// the choice can only be wrong about a run that has nothing to be wrong about.
fn direction(line: &viewer_core::TextLine) -> TextDirection {
    let (Some(first), Some(last)) = (line.characters.first(), line.characters.last()) else {
        return TextDirection::LeftToRight;
    };
    let across = (last.bounds[0] + last.bounds[2]) - (first.bounds[0] + first.bounds[2]);
    let down = (last.bounds[1] + last.bounds[3]) - (first.bounds[1] + first.bounds[3]);
    if across.abs() >= down.abs() {
        if across < 0.0 {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        }
    } else if down < 0.0 {
        TextDirection::BottomToTop
    } else {
        TextDirection::TopToBottom
    }
}

/// Where one character begins along the run, and how far it reaches — the two arrays' entries.
///
/// Measured from the run's own edge, which edge depending on the direction, because that is what
/// `accesskit_consumer::character_index_at_point` subtracts: `point.x - rect.x0` going one way and
/// `rect.x1 - point.x` going the other. A width is the extent along the same axis and is never
/// negative, so it is the same number whichever way the run reads.
fn along(character: [f32; 4], run: Rect, direction: TextDirection) -> (f32, f32) {
    let (x0, y0, x1, y1) = (character[0], character[1], character[2], character[3]);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the run's rectangle came from these same `f32` corners one function up; the \
                  widening to `f64` is AccessKit's rectangle type and the narrowing back is exact"
    )]
    match direction {
        TextDirection::LeftToRight => (x0 - run.x0 as f32, x1 - x0),
        TextDirection::RightToLeft => (run.x1 as f32 - x1, x1 - x0),
        TextDirection::TopToBottom => (y0 - run.y0 as f32, y1 - y0),
        TextDirection::BottomToTop => (run.y1 as f32 - y1, y1 - y0),
    }
}

/// Which characters begin a word, as indices into the line's own characters.
///
/// AccessKit asks for this and says why it will not compute it: "not all assistive technologies
/// require information about word boundaries … users will get unpredictable results if the word
/// boundaries exposed by the accessibility tree don't match the editor's behavior". There is no
/// editor here and no clause either — a PDF states glyphs and positions, and §9.4 has no word — so
/// this is a choice, and the plainest one: a word begins where a run of whitespace ends. The
/// leading whitespace of a line belongs to the word that follows it rather than being a word of
/// its own, which is the one place this differs from AccessKit's own description of the common
/// case; the difference is a caret step at the very start of an indented line.
fn word_starts(characters: &[(&str, u8, [f32; 4])]) -> Vec<u8> {
    let mut starts = Vec::new();
    let mut previous_was_space = true;
    for (index, (text, _, _)) in characters.iter().enumerate() {
        let space = text.chars().all(char::is_whitespace);
        // The index fits by construction — `chunked` stops a run at `MAX_RUN` characters — and a
        // run that somehow held more would rather lose a word boundary than an assertion.
        if previous_was_space
            && !space
            && let Ok(index) = u8::try_from(index)
        {
            starts.push(index);
        }
        previous_was_space = space;
    }
    starts
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

/// Publishes §14.8.5.5's continuation as a *relation*, on the element the reading leaves.
///
/// `FlowTo` runs from the earlier list to the one that carries it on, which is what a client
/// follows to read the whole list in order — so it is published on the predecessor, and
/// [`continued_by`] is what turns the answer's own direction round. A successor that was not
/// published is dropped rather than pointed at: `accesskit::TreeUpdate` makes it "an error for any
/// node in this list to not be either the root or a child of another node", and a relation to a
/// node that is in neither is a tree naming something it does not carry.
fn flows_to(built: &mut Node, band: Band, next: Option<&Vec<usize>>, published: &[bool]) {
    for at in next.into_iter().flatten() {
        if published.get(*at).copied().unwrap_or(false) {
            built.push_flow_to(band.element(*at));
        }
    }
}

/// What §14.8.5.5's continuation is *said* as, where the element states one.
///
/// On the description channel and for the reason [`headers`] and a table's `/Summary` are: it is a
/// statement *about* the list rather than something the list's content says, and nothing else on
/// the page carries it — the numbering starting again at one is exactly what the producer wrote the
/// entry to contradict. The `FlowTo` relation is the machine half of the same fact; this is the
/// half a person hears, and the two sentences differ because a client that cannot follow the
/// relation should still be told the list is not a fresh one.
fn continuation(node: &AccessibilityNode) -> Option<String> {
    node.continues_a_list.then(|| {
        match node.continued_from {
            Some(_) => "a continuation of an earlier list on this page",
            None => "a continuation of an earlier list",
        }
        .to_owned()
    })
}

/// Which elements continue which, indexed as the answer is: entry `n` holds the lists that carry
/// element `n` on.
///
/// §14.8.5.5's two attributes point from a list to the one it continues, and
/// [`AccessibilityNode::continued_from`] keeps that direction because it is the document's. The
/// relation this platform has runs the other way — `FlowTo` is "read on at", published on the
/// element the reading *leaves* — so the inversion happens once, here, rather than in a search per
/// element.
///
/// A list may be continued by more than one, which is a document stating two continuations of one
/// list; nothing in §14.8.5.5 forbids it, so all of them are published rather than the first.
fn continued_by(view: &PageView) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); view.nodes.len()];
    for (index, node) in view.nodes.iter().enumerate() {
        if let Some(from) = node.continued_from
            && let Some(slot) = out.get_mut(from)
        {
            slot.push(index);
        }
    }
    out
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
        // Table 384's `/Short` is written for exactly this repetition — "[i]t can become
        // cumbersome for a user to repeatedly have to listen to the full contents of a TH
        // structure element" — so where the author stated one it is what the header is said
        // as *here*, in front of each cell it describes, and only here: the header cell's own
        // node keeps its full content, because the abbreviation substitutes for the
        // repetition and not for the cell.
        if let Some(short) = header.short.as_deref() {
            if let Some(slot) = out.get_mut(at) {
                short.trim().clone_into(slot);
            }
            continue;
        }
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

/// The group holding what the page could not draw and what it could not be read as.
///
/// **Two kinds of item under one group, and they are worded apart.** A report is a refusal of
/// this program's; a code §9.10.2 could not name is the standard's own answer about a page that
/// drew correctly, and calling the second "not drawn as the document specifies" would tell a
/// person the picture is wrong when it is not. The sentence for the second says what a reader
/// loses instead: characters missing from what this page can be read as.
fn reports(view: &PageView, band: Band, out: &mut Vec<(NodeId, Node)>) -> Node {
    let mut children = Vec::new();
    let mut said = 0_usize;
    for note in view.reports {
        let mut item = Node::new(Role::Label);
        say(&mut item, note);
        children.push(push_report(out, band, &mut said, item));
    }
    if !view.readback.is_whole() {
        let mut item = Node::new(Role::Label);
        say(&mut item, &readback_note(view.readback));
        children.push(push_report(out, band, &mut said, item));
    }
    let mut group = Node::new(Role::Status);
    group.set_label(format!(
        "{said} thing(s) on this page were not drawn or could not be read as the document \
         specifies"
    ));
    group.set_children(children);
    group
}

/// Files one item of the status group under the next identifier in the reports' own range.
fn push_report(out: &mut Vec<(NodeId, Node)>, band: Band, said: &mut usize, item: Node) -> NodeId {
    let id = band.report(*said);
    *said = said.saturating_add(1);
    out.push((id, item));
    id
}

/// One sentence for what a page's codes cost its reading.
///
/// Two numbers at most, because they are two different losses and a person hearing one number
/// could not tell them apart: a code no method of §9.10.2 could name is text that is on the page
/// and not in what can be read off it, and a code that reached no glyph is a mark that was never
/// drawn. A blank glyph is neither and is deliberately not said — the font describing a glyph as
/// empty is how every one of them stores a space (ADR 0270).
fn readback_note(shortfall: pdf_model::content::Shortfall) -> String {
    let named = shortfall.unnamed.total();
    let drawn = shortfall.without_a_glyph;
    match (named, drawn) {
        (0, missing) => format!(
            "{missing} character(s) shown on this page have no glyph in the font that shows them, so they were not drawn"
        ),
        (unnamed, 0) => format!(
            "{unnamed} character(s) shown on this page cannot be read: nothing in the document says which characters their codes stand for"
        ),
        (unnamed, missing) => format!(
            "{unnamed} character(s) shown on this page cannot be read, and {missing} more have no glyph in the font that shows them"
        ),
    }
}

/// Where a magnifier is pointed at this element, and which of three statements decides it.
///
/// **Measured first, stated last**, which is one ordering applied twice rather than two rules.
///
/// 1. The **text quads**: the shapes this program drew, glyph by glyph. Where they exist they are
///    what the ring should sit on, because they are the marks themselves and nothing coarser.
/// 2. §14.8.3.3's **content rectangle**, which `viewer_core::AccessibilityNode::drawn` carries —
///    the union of what the element's marked-content sequences painted, for an element that marks
///    no text. A `Figure` holding an image, a `TD` holding a logo, a `Div` around a rule. Still
///    measured, and coarser only because a picture has no glyphs to be precise about.
/// 3. **Table 379's `/BBox`**, and then §12.5.2's annotation rectangle, which
///    `viewer_core::AccessibilityNode::bounds` carries. These are what the *document* says, and
///    they answer where this program drew nothing it could measure.
///
/// The order is the conservative one and ADR 0301 argued its first half: a document's rectangle is
/// a *claim* about a layout this program has already carried out, so it is used where there is
/// nothing to compare it against and not in place of what was drawn. Extending that to the second
/// statement is the same argument — a rectangle a producer wrote is a claim whether the marks
/// under it are glyphs or a picture, and `doc/PDF20_AN001-BPC.pdf` states
/// `[-32768 -32768 32767 32767]` for one figure, which is a producer writing "somewhere".
///
/// Three statements rather than one merged rectangle, because they are different *kinds* of fact
/// and a host that wants to know which it has can still ask. Whether a stated `/BBox` should win
/// over measured text — a `Figure` holding a caption and a picture has both, and the text quads
/// cover only the caption — is a question nothing has measured, and `doc/todo/31` carries it.
///
/// `None` where the element has none of the three: an untagged region, an element whose sequences
/// marked nothing, or one reached only through §14.7.5.3's object reference and stating no bounds.
fn place(node: &AccessibilityNode) -> Option<Rect> {
    bounding_box(&node.quads)
        .or_else(|| node.drawn.map(rectangle))
        .or_else(|| node.bounds.map(rectangle))
}

/// One of `viewer_core`'s rectangles as AccessKit's.
fn rectangle(rect: [f32; 4]) -> Rect {
    Rect {
        x0: f64::from(rect[0]),
        y0: f64::from(rect[1]),
        x1: f64::from(rect[2]),
        y1: f64::from(rect[3]),
    }
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
