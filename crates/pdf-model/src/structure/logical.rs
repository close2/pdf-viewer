//! §14.8.2.5's logical order: the document read element by element.
//!
//! The walk of the tree in the order its `/K` arrays state, and the text each element's content
//! gives. `Tree::logical_range`, the range of one element's text, stays in the root beside the
//! entry points it calls these through (ADR 2046).

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, ObjectId};

use super::{Child, MAX_DEPTH, MAX_ELEMENTS, Reading, Tree, Walk};

impl Tree {
    /// §14.8.2.5.1's logical content order, for one page.
    ///
    /// > Logical content order -the ordering for semantic purposes -shall be defined by a
    /// > depth-first traversal of the document's logical structure hierarch y.
    ///
    /// against which the other order is the stream's: "\[p]age content order shall be defined by
    /// the sequencing of graphics objects within a page's content stream". The clause says the
    /// two "should coincide" and then spends a NOTE explaining when they cannot — overlapping
    /// objects, a headline spanning two pages, two articles beginning on one page — so a
    /// consumer that wants meaning has to walk the tree and a consumer that wants pixels has to
    /// walk the stream.
    ///
    /// The items are this page's only, in the tree's order, and **annotations are among them**:
    /// §14.8.2.5.2 says an annotation "is not interleaved within the page's content stream" and
    /// that "\[t]he position of an annotation in the logical content order is determined from the
    /// document's logical structure", which is exactly the object references this walk already
    /// returns.
    ///
    /// A content item whose `/Pg` is absent is included: Table 355 makes the entry required only
    /// where an element has content items of the integer form, and an element that states none
    /// anywhere in its ancestry has left the page unstated rather than said "not this page".
    #[must_use]
    pub fn logical_order(&self, document: &Document, page: ObjectId) -> Reading<Child> {
        let walk = self.walk(document);
        Reading {
            items: walk
                .items
                .into_iter()
                .filter_map(|(_, child)| match child {
                    // An element is not a content item; the walk returns both and only the
                    // leaves are ordered content.
                    Child::MarkedContent { page: at, .. } | Child::Object { page: at, .. } => {
                        at.is_none_or(|at| at == page).then_some(child)
                    }
                    Child::Element(_) => None,
                })
                .collect(),
            truncated: walk.truncated,
        }
    }

    /// A page's readback, rearranged into §14.8.2.5.1's logical content order.
    ///
    /// Every marked-content sequence the structure tree reaches, in the tree's order, with the
    /// text each one covered in [`crate::Interpretation::text`]. Sequences the tree does not
    /// reach are **left out**, which is the clause's own position twice over: only structure
    /// elements are part of the logical content order, and §14.8.2.5.1 NOTE 3 says of the one
    /// case a reader might expect otherwise that "\[a]rtifacts not contained within an Artifact
    /// structure element are not considered part of the logical content order".
    ///
    /// Annotations are in [`Self::logical_order`] and not here. This is the page's *content
    /// streams* in the other order, which is what `tests/logical_order.rs` compares with the
    /// order they were shown in; an appearance's readback has no stream order on the page to be
    /// compared with, and [`Self::logical_range`] is where it is placed (ADR 1970).
    ///
    /// Equal to `interpretation.text` on a page whose two orders coincide, which the clause
    /// says they *should* — and `tests/logical_order.rs` measures how often they do.
    ///
    /// `None` where [`Self::logical_order`] was truncated: a prefix of the tree gives a prefix
    /// of the page's text, and answering with it would be the one failure a caller cannot see.
    #[must_use]
    pub fn logical_text(
        &self,
        document: &Document,
        page: ObjectId,
        interpretation: &crate::Interpretation,
    ) -> Option<String> {
        let order = self.logical_order(document, page);
        if order.truncated {
            return None;
        }
        let mut out = String::new();
        for item in order.items {
            let Child::MarkedContent { mcid, stream, .. } = item else {
                continue;
            };
            // Both halves of §14.7.5.2's key: the identifier is unique only "within its content
            // stream", so a page whose `/Contents` and whose form `XObject` both number from zero
            // has two sequences called `/MCID 0` and one of them is not this element's.
            for span in crate::content::named_sequences(&interpretation.marked, mcid, stream) {
                if let Some(text) = interpretation.text.get(span.range.clone()) {
                    out.push_str(text);
                }
            }
        }
        Some(out)
    }

    /// One level of [`Self::walk`].
    ///
    /// The visited set holds [`ObjectId`]s and not dictionaries, which is what makes a large
    /// tree walkable at all: a `Vec<Dictionary>` searched linearly is quadratic in the number
    /// of elements *and* compares whole dictionaries at each step. Measured on ISO 32000-2
    /// (ADR 0257): **16.8 s** for 44 651 elements searched linearly, against **151 ms** for all
    /// 78 468 with the set. `logical_order` walks the whole tree once per page, so the linear
    /// search would cost 16.8 s of every §14.8.2.5 question asked of the document this project
    /// checks itself against.
    ///
    /// An element reached other than through a reference has no identity to remember and is
    /// always descended into. That loses nothing: a dictionary written inline in its parent's
    /// `/K` is contained by that parent, so it can be reached once and cannot close a cycle,
    /// and [`MAX_DEPTH`] bounds it regardless.
    pub(super) fn descend(
        &self,
        document: &Document,
        element: Option<&Dictionary>,
        depth: usize,
        out: &mut Walk,
        seen: &mut BTreeSet<ObjectId>,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        for (child, id) in self.identified_children(document, element) {
            if out.items.len() >= MAX_ELEMENTS {
                out.truncated = true;
                return;
            }
            let descend_into = match (&child, id) {
                (Child::Element(dict), None) => Some(dict.clone()),
                (Child::Element(dict), Some(id)) if seen.insert(id) => Some(dict.clone()),
                _ => None,
            };
            out.items.push((depth, child));
            if let Some(dict) = descend_into {
                self.descend(document, Some(&dict), depth.saturating_add(1), out, seen);
            }
        }
    }
}
