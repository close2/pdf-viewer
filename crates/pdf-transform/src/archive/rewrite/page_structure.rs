//! The structure of a page: its resources and boundaries, content taken out of it, and the pages
//! appended to carry what the target will not otherwise admit.
use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::preserve::Composed;
use super::{Rewrite, Rewriter, count};

impl Rewriter<'_> {
    /// §7.7.3.4's inherited resources, copied down onto the page that already resolves through
    /// them.
    ///
    /// **Never over an entry that is there.** The population is a page stating none, and a page
    /// the `/DefaultCMYK` rewrite has already given one is a page this rewrite has nothing left
    /// to do for — the value the two write is the same value, read from the same tree.
    pub(super) fn attach_page_resources(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::PageResources) || out.get("Resources").is_some() {
            return false;
        }
        let Some(resources) = self
            .page_resources
            .and_then(|pages| pages.at.get(&id))
            .cloned()
        else {
            return false;
        };
        out.insert(Name::new(&b"Resources"[..]), resources);
        count(applied, Rewrite::PageResources);
        true
    }

    /// §7.7.3.2's Table 30 `/Count`, written as the leaves beneath the node where the source's is
    /// missing or no integer; `super::super::page_tree::restated` counted them (`doc/adr/2456`).
    pub(super) fn restate_page_count(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(stated) = self
            .page_counts
            .filter(|_| self.wants(Rewrite::PageCountRestated))
            .and_then(|counts| counts.get(&id))
        else {
            return false;
        };
        out.insert(Name::new(&b"Count"[..]), Object::Integer(*stated));
        count(applied, Rewrite::PageCountRestated);
        true
    }

    /// The out-of-range optional page boundary entries one page loses.
    ///
    /// ISO 19005-2 section 6.1.13's limit, answered by ISO 32000-2 §7.7.3.3's Table 31 and
    /// §14.11.2.1's defaults: an entry Table 31 marks optional and whose default is another box
    /// in the same file is removable, and the preparation has already decided — per entry, on
    /// §14.11.2.1's intersection sentence — whether the region a reader computes changes with it
    /// (`doc/adr/1210`). Nothing is decided here.
    pub(super) fn remove_page_boundaries(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(keys) = self
            .boundaries
            .filter(|_| self.wants(Rewrite::PageBoundaryRemoved))
            .and_then(|removals| removals.at.get(&id))
        else {
            return false;
        };
        let mut changed = false;
        for key in keys {
            if out.remove(key).is_some() {
                count(applied, Rewrite::PageBoundaryRemoved);
                changed = true;
            }
        }
        changed
    }

    /// The dictionaries an appended page changes: the page tree's root, the label tree, the
    /// structure tree.
    ///
    /// `doc/adr/0954` is why the second is here at all — a page count that changes leaves
    /// §12.4.2's labels describing a document that no longer exists, and neither the label nor the
    /// page is a conformance requirement, so both are owed for the reader rather than for the
    /// validator. The third is a requirement: §14.8.2.2.1 makes content the structure tree does not
    /// reach an artifact, and the preserved content is not one (`doc/adr/1163`).
    ///
    /// **None of the three is counted.** `Rewrite::PreservedAsPage`'s count is the pages appended,
    /// which is what the report's "N place(s)" means here; the entries those pages owe the document
    /// they land in are part of appending them rather than places of their own.
    pub(super) fn preserve(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::PreservedAsPage) {
            return false;
        }
        let Some(composed) = self.preserved else {
            return false;
        };
        let mut changed = false;
        if Some(id) == self.sites.root_of_the_pages && !composed.pages.is_empty() {
            changed |= self.append_the_pages(out, composed, applied);
        }
        if let Some((Some(at), labels)) = composed.labels.as_ref()
            && *at == id
        {
            *out = labels.clone();
            changed = true;
        }
        if let Some(structure) = composed.structure.as_ref()
            && !composed.pages.is_empty()
        {
            changed |= structure.apply(self.document, id, out);
        }
        changed
    }

    /// One page's `/Annots` without the annotations named, counted against the rewrite that asked.
    ///
    /// **An array left empty is removed rather than written empty.** §7.7.3.3's Table 31 makes
    /// `/Annots` optional, so a page whose every annotation goes ends up saying what it now means
    /// — this page has no annotations — rather than stating an empty array.
    pub(super) fn take_out_of_the_page(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
        at: &BTreeSet<ObjectId>,
        rewrite: Rewrite,
    ) -> bool {
        let Some(listed) = self
            .document
            .get_key(out, "Annots")
            .as_array()
            .map(<[Object]>::to_vec)
        else {
            return false;
        };
        let kept: Vec<Object> = listed
            .iter()
            .filter(|entry| entry.as_reference().is_none_or(|id| !at.contains(&id)))
            .cloned()
            .collect();
        let removed = listed.len().saturating_sub(kept.len());
        if removed == 0 {
            return false;
        }
        for _ in 0..removed {
            count(applied, rewrite);
        }
        if kept.is_empty() {
            out.remove("Annots");
        } else {
            out.insert(Name::new(&b"Annots"[..]), Object::Array(kept));
        }
        true
    }

    /// Adds the composed pages to the page tree's root node.
    ///
    /// §7.7.3.2's Table 30 is the whole of what this has to get right, and it makes the two edits
    /// one each. `/Kids`:
    ///
    /// > An array of indirect references to the immediate children of this node. The children
    /// > shall only be page objects or other page tree nodes.
    ///
    /// A composed page is one of the two things a kid may be, so it is added to the root node
    /// without any other node in the tree knowing. And `/Count`:
    ///
    /// > The number of leaf nodes (page objects) that are descendants of this node within the page
    /// > tree.
    ///
    /// The root is an ancestor of every page, so it is the only node whose count changes. The
    /// producer's own number is added to rather than recomputed: a file whose count was wrong
    /// before this conversion is wrong in the same way afterwards, and correcting it is a change
    /// no failed requirement asked for.
    fn append_the_pages(
        &self,
        node: &mut Dictionary,
        composed: &Composed,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(mut kids) = self
            .document
            .get_key(node, "Kids")
            .as_array()
            .map(<[Object]>::to_vec)
        else {
            return false;
        };
        for page in &composed.pages {
            kids.push(Object::Reference(*page));
            count(applied, Rewrite::PreservedAsPage);
        }
        let was = self
            .document
            .get_key(node, "Count")
            .as_integer()
            .unwrap_or(0);
        let gained = i64::try_from(composed.pages.len()).unwrap_or(0);
        node.insert(Name::new(&b"Kids"[..]), Object::Array(kids));
        node.insert(
            Name::new(&b"Count"[..]),
            Object::Integer(was.saturating_add(gained)),
        );
        true
    }
}
