//! The warnings a carry owes the person who asked for it.

use super::Carry;
use crate::Warning;

impl Carry {
    /// What the carry wrote and what it lost, by reason, so that a tagged document's reader is
    /// told rather than left to find out.
    pub(super) fn report(&self, warnings: &mut Vec<Warning>) {
        let mut said = |detail: String| {
            warnings.push(Warning {
                source: self.speaks_for,
                page: None,
                detail,
            });
        };
        said(format!(
            "§14.7: the structure tree is carried — {} element(s) written, {} dropped for \
             reaching no page the output holds",
            self.placed.len(),
            self.dropped
        ));
        if let Some(wrapper) = &self.wrapper {
            said(format!(
                "Annex L: Table L.2 gives the structure tree root one Document child, so the \
                 {} PDF 2.0 Document elements the sources hold at their roots are written inside \
                 one Document of that namespace",
                wrapper.children.len()
            ));
        }
        if self.dropped_items > 0 {
            said(format!(
                "§14.7.5: {} content item(s) name a page the output does not hold and were \
                 dropped from their element's /K",
                self.dropped_items
            ));
        }
        if self.orphaned_items > 0 {
            // §14.7.2 makes the structure hierarchy what `/StructTreeRoot`'s `/K` reaches and
            // Table 355 makes `/P` required, so an element a source's parent tree names but its
            // own hierarchy does not reach is in the file and not in the tree. This carry keeps
            // the hierarchy, so the array position goes to null — which is a marked-content
            // sequence an assistive reader will now find nothing for, and two corpus documents
            // do it (ADR 0839).
            said(format!(
                "§14.7.5.4: {} parent-tree entr{} name a structure element the source's own \
                 hierarchy does not reach (§14.7.2, Table 355's required /P), so the output \
                 states §7.3.9's null there and that marked content has no structure",
                self.orphaned_items,
                if self.orphaned_items == 1 { "y" } else { "ies" }
            ));
        }
        if self.unplaceable > 0 {
            said(format!(
                "§14.7.5.2: {} marked-content item(s) could not be placed on any page — Table \
                 355 makes /Pg required where /K is an integer, and the element states none — \
                 and were dropped",
                self.unplaceable
            ));
        }
        if self.duplicated_pages > 0 {
            said(format!(
                "§14.7.5.4: {} page(s) are in the output twice and the second placement states \
                 no /StructParents — Table 355 gives a structure element one /Pg, so its content \
                 items can name only one of the two page objects",
                self.duplicated_pages
            ));
        }
    }
}
