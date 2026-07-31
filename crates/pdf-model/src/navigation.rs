//! ISO 32000-2 §12.4.4.2's sub-page navigation: states of one page, in order.
//!
//! The clause's opening sentence is the whole idea — sub-page navigation is the ability to
//! navigate not only between pages but between different states of the same page.
//!
//! The clause's own NOTE 1 says what a state is made of, and it is a thing this program already
//! has: "[a] single page in a PDF presentation could have a series of bullet points that could
//! be individually turned on and off. In such an example, the bullets would be represented by
//! optional content, and each state of the page would be represented as a navigation node."
//!
//! So a navigation node is a pair of §12.6 actions — one to go forward, one to go back — and
//! walking a page's states is `crate::view::ViewState` performing them. §8.11's groups are
//! read in full, §12.6.4.13's action sets them, and this is the list that says in what order.
//! That is why the ledger's row for this subclause has called it "the one presentation row
//! whose missing piece is a control rather than a renderer" since the fifty-fourth session.
//!
//! # What is deliberately not here
//!
//! **Nothing turns the presentation on.** NOTE 3 says "[a]n interactive PDF processor needs to
//! respect navigation nodes only when in presentation mode", and this program has no
//! presentation mode — no full-screen state, and none of §12.4.4.1's `/Trans` transitions or
//! `/Dur` timings, which are animations. What is built is the list and the actions; a caller
//! that has a presentation mode drives it.
//!
//! NOTE 2 states the obligation that comes with driving it, and it is the reason
//! `ViewState` is a value a caller owns rather than something this crate hides: "[i]nteractive
//! PDF processors need to save the state of optional content groups when a user enters
//! presentation mode and restore it when presentation mode ends." A `ViewState` is `Clone`, and
//! that is the whole of what saving and restoring it takes.

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::action::Action;

/// Longest chain of `/Next` links followed.
///
/// The nodes "form a doubly linked list by means of their Next and Prev entries", and both are
/// references a document controls, so the list may be a ring. Each node is visited once, which
/// is what makes the walk terminate; this bounds the *length* as well, because a file may state
/// a very long list as cheaply as a short one.
const MAX_NODES: usize = 4096;

/// One of §12.4.4.2's navigation nodes. Table 165.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Table 165's `/NA`: what to perform "when a user navigates forward".
    ///
    /// A list rather than one action because the entry is "an action (which may be the first in
    /// a sequence of actions)" — §12.6.2's `/Next` chain, already flattened into the order it is
    /// performed in.
    pub forward: Vec<Action>,
    /// Table 165's `/PA`: what to perform "when a user navigates backward".
    pub backward: Vec<Action>,
}

/// A page's navigation nodes, from its `/PresSteps`, in `/Next` order.
///
/// §12.4.4.2:
///
/// > The primary node on a page shall be determined by the optional PresSteps entry in a page
/// > dictionary
///
/// Empty for a page with no `/PresSteps`, which is every page of every corpus document: neither
/// `PresSteps` nor `NavNode` occurs in the raw bytes of any of the 974, which is a lower bound
/// and enough — this is the specification track rather than the demand one, and says so.
///
/// Only `/Next` is walked. `/Prev` is the same list read the other way, and Table 165 makes the
/// two a doubly linked list rather than two lists: reading both would either produce the same
/// nodes twice or disagree, and a file whose `/Prev` chain is not the reverse of its `/Next`
/// chain has stated two orders with no rule to choose between them.
#[must_use]
pub fn steps(document: &Document, page: &Dictionary) -> Vec<Node> {
    let mut out = Vec::new();
    let mut seen: Vec<ObjectId> = Vec::new();
    let mut entry = page.get("PresSteps").cloned().unwrap_or(Object::Null);
    while out.len() < MAX_NODES {
        if let Object::Reference(id) = entry {
            if seen.contains(&id) {
                break;
            }
            seen.push(id);
        }
        let resolved = document.resolve(&entry);
        let Some(node) = resolved.as_dict() else {
            break;
        };
        out.push(Node {
            forward: crate::action::read(document, node.get("NA").unwrap_or(&Object::Null)),
            backward: crate::action::read(document, node.get("PA").unwrap_or(&Object::Null)),
        });
        entry = node.get("Next").cloned().unwrap_or(Object::Null);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::steps;
    use crate::action::Action;
    use crate::view::ViewState;
    use pdf_syntax::Document;

    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// The clause's own NOTE 1, built as a document: bullets that turn on one at a time.
    ///
    /// Two navigation nodes, each turning a layer on when a user goes forward and off when they
    /// go back, which is what makes this the row the ledger called "a control rather than a
    /// renderer": every part of it below the list already existed.
    #[test]
    fn a_page_of_bullets_walks_its_states() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [8 0 R 9 0 R] \
             /D << /BaseState /OFF >> >> >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /PresSteps 4 0 R >>",
            "<< /Type /NavNode /NA 6 0 R /PA 7 0 R /Next 5 0 R >>",
            "<< /Type /NavNode /NA 10 0 R /Prev 4 0 R >>",
            "<< /S /SetOCGState /State [/ON 8 0 R] >>",
            "<< /S /SetOCGState /State [/OFF 8 0 R] >>",
            "<< /Type /OCG /Name (bullet one) >>",
            "<< /Type /OCG /Name (bullet two) >>",
            "<< /S /SetOCGState /State [/ON 9 0 R] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        let nodes = steps(&doc, &page.dict);
        assert_eq!(nodes.len(), 2, "the /Next chain, in order");
        assert!(matches!(
            nodes.first().map(|node| node.forward.as_slice()),
            Some([Action::SetOcgState(_)])
        ));

        let id = |number| pdf_syntax::ObjectId {
            number,
            generation: 0,
        };
        let mut state = ViewState::of(&doc);
        let on = |state: &ViewState, group| state.optional_content().and_then(|oc| oc.state(group));
        assert_eq!(on(&state, id(8)), Some(false), "/BaseState OFF");

        for node in &nodes {
            state.perform_all(&doc, &node.forward);
        }
        assert_eq!(on(&state, id(8)), Some(true));
        assert_eq!(on(&state, id(9)), Some(true));

        if let Some(first) = nodes.first() {
            state.perform_all(&doc, &first.backward);
        }
        assert_eq!(on(&state, id(8)), Some(false), "and back again");
    }

    /// A ring of `/Next` links terminates.
    ///
    /// Table 165 makes the nodes a doubly linked list and nothing forbids a file from closing
    /// it into a ring; each node is visited once, so the walk ends where it began.
    #[test]
    fn a_ring_of_navigation_nodes_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /PresSteps 4 0 R >>",
            "<< /Type /NavNode /Next 5 0 R >>",
            "<< /Type /NavNode /Next 4 0 R >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        assert_eq!(steps(&doc, &page.dict).len(), 2);
    }

    /// A page with no `/PresSteps` has no states to walk, which is every corpus page.
    #[test]
    fn a_page_without_pres_steps_has_no_states() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        assert!(steps(&doc, &page.dict).is_empty());
    }
}
