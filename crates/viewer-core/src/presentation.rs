//! ISO 32000-2 §12.4.4.2's sub-page navigation: the current navigation node, and what moves it.
//!
//! The clause opens with a `shall` that is a state machine in one sentence:
//!
//! > An interactive PDF processor shall maintain a current navigation node.
//!
//! and states four things about it. Where the node comes from — "[w]hen a user navigates to a
//! page, if the page dictionary has a PresSteps entry, the node specified by that entry shall
//! become the current node. (Otherwise, there is no current node.)"; what a forward request does —
//! "[t]he sequence of actions specified by NA (if present) shall be executed" and then "[t]he node
//! specified by Next (if present) shall become the new current navigation node"; what a backward
//! request does, with `/PA` and `/Prev`; and what a page arrived at with a `/PresSteps` does,
//! which is the two of them run together.
//!
//! Every one of those is a `shall`, so this module is the clause and not a design. What is *not*
//! the clause is named where it is decided, and there are two such places: what happens when the
//! chain runs out, and when the mode is entered on a page nobody navigated to.
//!
//! # The nodes are respected only while a presentation is running
//!
//! NOTE 3: "[a]n interactive PDF processor needs to respect navigation nodes only when in
//! presentation mode". That is a permission and this program takes it, because the alternative is
//! not neutral: a reader turning the pages of a document that happens to state `/PresSteps` would
//! find the arrow key toggling somebody's bullets instead of turning the page. So every function
//! here is reached only under [`crate::PresentationMode::On`], which a host states and this crate
//! cannot deduce.
//!
//! # A position in the `/Next` chain, rather than an object
//!
//! [`pdf_model::navigation::steps`] flattens the chain into a list, on the clause's own ground:
//! the nodes "form a doubly linked list by means of their Next and Prev entries", so `/Prev` is
//! that list read backwards and a file whose two chains disagree has stated two orders with no
//! rule to choose between them. The current node is therefore a position in that list, and moving
//! to `/Next` is moving one along.

#![expect(
    clippy::doc_markdown,
    reason = "the quotations here are verbatim, and the standard writes its entry names bare — \
              backticks inside a blockquote would make the conformance gate's check fail"
)]

use pdf_model::action::Action;
use pdf_model::navigation::Node;

use crate::open::Open;

/// The navigation nodes of the page being shown, in `/Next` order.
///
/// Empty for a page stating no `/PresSteps`, which is every page of every corpus document.
///
/// **The page is fetched from the tree rather than read from [`Open::current`]**, for the reason
/// the `/Trans` lookup beside it is: `current` is filled during interpretation, and a page turn
/// asks this before the page it turned to has been interpreted. One page-tree walk per navigation
/// request in presentation mode, which is a key press rather than the per-item loop ADR 0124 was
/// about.
fn nodes(open: &Open) -> Vec<Node> {
    let pages = pdf_model::Pages::new(&open.document);
    pages
        .get(open.page_index)
        .map(|page| pdf_model::navigation::steps(&open.document, &page.dict))
        .unwrap_or_default()
}

/// Enters presentation mode for one document.
///
/// Two things, and both are the clause's. NOTE 2 is the first — "[i]nteractive PDF processors need
/// to save the state of optional content groups when a user enters presentation mode and restore
/// it when presentation mode ends" — and NOTE 1 says why it matters here rather than anywhere
/// else: a page's states *are* optional content groups, so walking them is what would otherwise
/// leave the document changed after the slide show.
///
/// The second is the current node, and it is **this program's reading rather than the clause's
/// words**: the clause names the primary node as what becomes current "[w]hen a user navigates to
/// a page", and entering the mode on a page already showing is not navigating to it. A processor
/// that then had no current node would not be maintaining one, which is the sentence the whole
/// subclause opens with — so the page showing supplies its primary node, exactly as an arrival
/// would, and nothing is executed, because nothing was requested.
pub(crate) fn enter(open: &mut Open) {
    open.saved_groups = open.view.optional_content_snapshot();
    open.node = (!nodes(open).is_empty()).then_some(0);
    open.node_shown_for = 0.0;
}

/// Leaves presentation mode, and says whether the page has to be drawn again.
///
/// NOTE 2's other half. What is restored is the state as the mode found it, so a document whose
/// bullets were turned on one at a time is the document the file describes again — which is what
/// the NOTE says it is for: "transient changes to bullets do not affect the printing of the
/// document".
pub(crate) fn leave(open: &mut Open) -> bool {
    open.node = None;
    open.node_shown_for = 0.0;
    open.view.restore_optional_content(open.saved_groups.take())
}

/// What one navigation request does, where there was a current node to make it against.
#[derive(Debug)]
pub(crate) struct Step {
    /// §12.4.4.2 (a) or (c): the sequence of actions to execute.
    ///
    /// Handed back rather than performed, because performing them is §12.6's business and this
    /// module's subject is which node is current.
    pub(crate) actions: Vec<Action>,
    /// Whether this request also turns the page, which is the erratum below.
    pub(crate) onward: bool,
}

/// One navigation request, forward or backward, where there is a current node.
///
/// §12.4.4.2 states a forward one as two steps, in this order:
///
/// > a) The sequence of actions specified by NA (if present) shall be executed.
///
/// > b) The node specified by Next (if present) shall become the new current navigation node.
///
/// and `/PA` with `/Prev` for a backward one.
///
/// `None` means there is no current node, which is the clause's own other state and is what tells
/// a caller to turn the page instead.
///
/// # Where the chain ends: a decision, until an erratum answered it
///
/// **This used to say the end of the chain leaves no current node, and called that a decision.**
/// As printed, the clause says the node named by `/Next` "(if present)" becomes current and says
/// nothing at all about a node whose `/Next` is absent, so the reading here was that running off
/// either end lands in the state the clause already names — "there is no current node" — and that
/// the *next* request after the last state turns the page.
///
/// ISO 32000-2 errata **issue #304**, state Review/Completed, answers it outright, and differently.
/// `cargo run -p spec-errata -- emit doc/*.pdf` reports two `Text` annotations on the page whose
/// heading it prints as §12.5.1; their icons sit in the right margin of §12.4.4.2's own items (b)
/// and (d), which are the two sentences quoted above. The one against (b) reads *If there is no
/// node specified by Next then navigate to the next page. If the current page is the last page,
/// then the current navigation node remains unchanged*, and the one against (d) says the same of
/// `/Prev`, the previous page and the first page.
///
/// So two things change and both were wrong here:
///
/// - **The request that runs the last node's actions also turns the page**, rather than being
///   swallowed and leaving a person to press the key twice for one step. [`Step::onward`] is that.
/// - **On the last page there is no page to turn to, and then the node stays current.** The
///   objection this module used to record — that leaving the last node current would re-execute
///   its `/NA` and no page could ever be turned — does not apply where the clause has put it,
///   because on the last page there is no page to turn to in the first place.
pub(crate) fn step(open: &mut Open, forward: bool) -> Option<Step> {
    let index = open.node?;
    let nodes = nodes(open);
    let node = nodes.get(index)?;
    let actions = if forward {
        node.forward.clone()
    } else {
        node.backward.clone()
    };
    open.node_shown_for = 0.0;
    let next = if forward {
        index.checked_add(1).filter(|next| *next < nodes.len())
    } else {
        index.checked_sub(1)
    };
    if let Some(next) = next {
        open.node = Some(next);
        return Some(Step {
            actions,
            onward: false,
        });
    }
    // Issue #304: "[i]f the current page is the last page, then the current navigation node
    // remains unchanged" — so `open.node` is left where it is, and nothing is turned.
    let another_page = if forward {
        index_of_a_further_page(open)
    } else {
        open.page_index > 0
    };
    Some(Step {
        actions,
        onward: another_page,
    })
}

/// Whether there is a page after the one being shown, which is what issue #304 conditions on.
fn index_of_a_further_page(open: &Open) -> bool {
    open.page_index.saturating_add(1) < open.page_count
}

/// A page arrived at, in §12.4.4.2's last paragraph.
///
/// > a) The navigation node represented by PresSteps shall become the current node.
///
/// and then, in the same list, a step that is one request against it: if the request was forward
/// "or if the navigation request was for random access (such as by clicking on a link), the
/// actions specified by NA shall be executed and the node specified by Next shall become the new
/// current node, as described previously".
///
/// So an arrival is the primary node becoming current and one request against it, which is why
/// this is [`step`] with the position set first rather than a second construction. `forward` is
/// false only for a request the person made *backwards*; random access is forward, on the clause's
/// own words.
///
/// Backward is worth naming for what it does: (a) still makes the *primary* node current, so the
/// `/PA` executed is the first node's and its `/Prev` — which the list has none of — leaves that
/// same node current. That is the clause read literally, and it is not this program choosing to
/// skip a page's states: paging backwards into a page lands on the state its own producer put
/// first.
///
/// **[`Step::onward`] is dropped here, deliberately.** Errata issue #304 makes a request with no
/// node in that direction navigate to the next or previous page, and an arrival performs one
/// request against the primary node "as described previously" — but this paragraph ends at its
/// own step (c), "[t]he interactive PDF processor shall make the new page the current page and
/// shall display it". A page whose only node has no `/Next` would otherwise be arrived at and
/// left again in the same breath, and no reader would ever see it.
pub(crate) fn arrived(open: &mut Open, forward: bool) -> Vec<Action> {
    open.node_shown_for = 0.0;
    if nodes(open).is_empty() {
        // "(Otherwise, there is no current node.)"
        open.node = None;
        return Vec::new();
    }
    open.node = Some(0);
    step(open, forward)
        .map(|step| step.actions)
        .unwrap_or_default()
}

/// Table 165's `/Dur`, after `seconds` more of the current node being shown (§12.4.4.2).
///
/// > The maximum number of seconds before the interactive PDF processor shall automatically
/// > advance forward to the next navigation node. If this entry is not specified, no automatic
/// > advance shall occur.
///
/// True means the node has been up for as long as it asked and the caller performs a forward
/// request, which is what "advance forward to the next navigation node" is. A *maximum*, so the
/// comparison is `>=` and every request resets the clock — the same shape §12.4.4.1's `/Dur` has
/// one clause up, and deliberately a second clock rather than a shared one: a page states how long
/// the *page* is shown and a node how long the *node* is, both as maxima, and a file whose page
/// duration is shorter than its steps take has said the page turns.
pub(crate) fn node_due(open: &mut Open, seconds: f32) -> bool {
    open.node_shown_for += seconds;
    let Some(index) = open.node else {
        return false;
    };
    nodes(open)
        .get(index)
        .and_then(|node| node.duration)
        .is_some_and(|duration| open.node_shown_for >= duration)
}
