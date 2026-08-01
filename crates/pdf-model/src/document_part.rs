//! ISO 32000-2 §14.12's document part hierarchy, read only as far as §12.6.4.5 needs it.
//!
//! # Why a clause marked `inapplicable` has code
//!
//! §14.12 is a *production workflow* structure — a second hierarchy over the same pages,
//! carrying metadata for a job ticket — and nothing in it changes a mark. The conformance
//! ledger records it `inapplicable` for that reason, and the reason is sound as far as *marking*
//! goes.
//!
//! It stopped being the whole story the moment an action pointed at it. §12.6.4.5:
//!
//! > A GoToDp action changes the view to the Start page of a specified DPart
//!
//! which makes a `DPart` dictionary decide **which page is shown**, exactly as §12.3.2's
//! destinations do. That is `doc/HANDOVER.md`'s own habit arriving on schedule — "[a]n
//! `inapplicable` row decays exactly as a `silent` one does", which §12.7.4.2's field names
//! demonstrated when §12.6.4.11's hide action made a field name decide whether an annotation is
//! drawn.
//!
//! So this module reads one thing: the page a `DPart` begins at. The tree's metadata, its
//! `/NodeNameList`, its `/RecordLevel` and §14.12.4.2's `/DPM` are still the job ticket's and
//! are still `inapplicable`.

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// How far down the hierarchy [`first_page`] will descend.
///
/// §14.12.2 makes the structure a tree and a tree of any real document is a handful of levels
/// deep; a chain longer than this is a cycle a malformed file wrote, and the bound is what stops
/// following it. The `/Parent` links the clause requires are not followed at all, so a cycle
/// through them cannot be reached from here.
const MAX_DEPTH: usize = 64;

/// The page object a `DPart` dictionary's range begins at, if it names one.
///
/// Table 409 makes `/Start` and `/DParts` exclusive — "[s]hall not be present if a Start key is
/// present" and the converse — so a `DPart` is either a leaf naming a page range or a node naming
/// children, never both. §12.6.4.5 asks for "the Start page", which a *node* does not have, and
/// §14.12.3 is what says where to look for it:
///
/// > The order of page objects as defined by the page tree shall be in the same order in which
/// > page objects are referenced from leaf node DPart dictionaries in a depth-first traversal of
/// > the document part hierarchy.
///
/// So the first page of a node is the `/Start` of its first leaf in depth-first order, and
/// descending to it is a reading of that sentence rather than a guess. A `GoToDp` naming a node
/// would otherwise have nowhere to go, which the clause plainly does not intend.
///
/// `/DParts` is "[a]n array of arrays", so the first child is the first element of the first
/// element — a shape a first implementation flattens by accident and which changes nothing here,
/// since either reading reaches the same first leaf. It is written out because the *count* of
/// children would differ.
#[must_use]
pub fn first_page(document: &Document, part: &Dictionary) -> Option<ObjectId> {
    first_page_at(document, part, 0)
}

/// [`first_page`], carrying the depth that bounds it.
fn first_page_at(document: &Document, part: &Dictionary, depth: usize) -> Option<ObjectId> {
    if depth > MAX_DEPTH {
        return None;
    }
    // `/Start` "shall be an indirect reference to the page object", so the *reference* is what
    // this answers: a resolved dictionary could not be looked up in the page tree.
    if let Some(Object::Reference(id)) = part.get("Start") {
        return Some(*id);
    }

    let children = document.get_key(part, "DParts");
    let children = children.as_array()?;
    for row in children {
        let row = document.resolve(row);
        // A row that is not an array is a file writing Table 409's entry in the shape a reader
        // expects for one *level* down; taking it as a child directly costs nothing and loses
        // no valid file, because a `DPart` dictionary and an array of them are different types.
        let candidates: Vec<Object> = match &row {
            Object::Array(items) => items.clone(),
            other => vec![other.clone()],
        };
        for child in candidates {
            let resolved = document.resolve(&child);
            if let Some(dict) = resolved.as_dict()
                && let Some(page) = first_page_at(document, dict, depth.saturating_add(1))
            {
                return Some(page);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::first_page;
    use pdf_syntax::{Document, ObjectId};

    /// Assembles a document whose objects are given verbatim, numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", index.saturating_add(1));
        }
        let at = out.len();
        let size = offsets.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture is a valid PDF")
    }

    /// ISO 32000-2 §14.12.4.1, Table 409: a leaf's `/Start` is the page it begins at.
    ///
    /// > If present, the Start key shall be an indirect reference to the page object that
    /// > defines the first page of the range of pages belonging to this DPart dictionary.
    ///
    /// The *reference* and not what it resolves to, which is the whole reason this returns an
    /// `ObjectId`: a page is found in the page tree by identity.
    #[test]
    fn a_leaf_begins_at_the_page_its_start_names() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /DPart /Start 3 0 R >>",
        ]);
        let part = document.get(ObjectId {
            number: 4,
            generation: 0,
        });
        let part = part.as_dict().expect("the DPart dictionary");
        assert_eq!(
            first_page(&document, part),
            Some(ObjectId {
                number: 3,
                generation: 0
            })
        );
    }

    /// ISO 32000-2 §14.12.3: a node's first page is its first leaf's, depth first.
    ///
    /// > The order of page objects as defined by the page tree shall be in the same order in
    /// > which page objects are referenced from leaf node DPart dictionaries in a depth-first
    /// > traversal of the document part hierarchy.
    ///
    /// Table 409 makes `/Start` and `/DParts` exclusive, so a node has no `/Start` of its own
    /// and a `GoToDp` naming one would have nowhere to go under a reader that only looked there.
    /// `/DParts` is an array *of arrays*, which this fixture writes properly so that the nesting
    /// is what is under test.
    #[test]
    fn a_node_begins_at_the_first_leaf_of_its_first_child() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            // The root node, whose first child is itself a node.
            "<< /Type /DPart /DParts [[6 0 R 8 0 R]] >>",
            "<< /Type /DPart /Parent 5 0 R /DParts [[7 0 R]] >>",
            "<< /Type /DPart /Parent 6 0 R /Start 3 0 R >>",
            "<< /Type /DPart /Parent 5 0 R /Start 4 0 R >>",
        ]);
        let root = document.get(ObjectId {
            number: 5,
            generation: 0,
        });
        let root = root.as_dict().expect("the root node");
        assert_eq!(
            first_page(&document, root),
            Some(ObjectId {
                number: 3,
                generation: 0
            }),
            "depth first: the first child's first child, not the first child with a /Start"
        );
    }

    /// A hierarchy that points at itself answers nothing rather than looping.
    #[test]
    fn a_cycle_is_bounded_rather_than_followed() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /DPart /DParts [[5 0 R]] >>",
            "<< /Type /DPart /DParts [[4 0 R]] >>",
        ]);
        let root = document.get(ObjectId {
            number: 4,
            generation: 0,
        });
        let root = root.as_dict().expect("the root node");
        assert_eq!(first_page(&document, root), None);
    }
}
