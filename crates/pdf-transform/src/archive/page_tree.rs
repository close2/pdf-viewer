//! A page tree this converter would carry into a file no reader can walk, refused by name before
//! anything is written (ADR 2444).
//!
//! # The reading
//!
//! ISO 32000-2 §7.7.3.2's Table 30 states two entries of every page tree node as required, and
//! states their types:
//!
//! > ( Required ) An array of indirect references to the immediate children of this node.
//!
//! for `/Kids`, and for `/Count`:
//!
//! > ( Required ) The number of leaf nodes (page objects) that are descendants of this node within
//! > the page tree.
//!
//! This reader draws a document whose nodes break either: `pdf_model::Pages` counts the leaves
//! under `/Kids` where a `/Count` is missing or no number, and a node that declares `/Type /Pages`
//! with no `/Kids` array reaches no page. A conversion carries the producer's page tree as it
//! stands — ADR 0947's second rule, nothing changed that no failed requirement asked for, and no
//! requirement of ISO 19005 this tree checks is failed by a node's types, because the one sentence
//! that would be, each part's section 5.1, is `conformance/adheres-to-the-base-standard` and
//! `Unchecked`. So the output kept the broken node, and the file it wrote is one veraPDF reports it
//! cannot parse.
//!
//! # Why a refusal rather than a restated count
//!
//! The same row's NOTE puts the restatement on the writer — "\[a\] PDF writer shall ensure that the
//! value of the Count key is consistent with the number of entries in the Kids array" — so a
//! `/Count` that is no number is a remedy the standard states the result of. Writing it here would
//! be a change no failed requirement asked for, which is the rule above; the route that keeps the
//! rule is a requirement of its own under section 5.1 beside
//! `file-structure/hexadecimal-string-holds-only-digits`, failed by a node whose `/Count` is no
//! number and answered by the writer's restatement. Until that row exists the document is refused
//! here, by its clause, rather than written into a file that does not open. A node with no `/Kids`
//! array is refused on any route: the tree states no child, so the pages this reader draws from it
//! — none, or the ones a scan of the file finds — are not the producer's tree.

use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use crate::Refusal;

/// How deep the walk goes before it stops, `crate::update`'s bound for the same tree.
const MAX_DEPTH: usize = 64;

/// How many nodes the walk visits before it stops, `pdf_model`'s own bound over the same tree.
const MAX_NODES_VISITED: usize = 1 << 20;

/// Refuses a document whose page tree has a node §7.7.3.2's Table 30 makes unwalkable.
///
/// Every node the catalog's `/Pages` reaches through `/Kids` is read, bounded by depth and by a
/// node budget because `/Kids` holds references a file can make into a cycle. A dictionary is a
/// node where it states `/Kids` or declares `/Type /Pages`, `pdf_model`'s own reading; any other
/// kid is a leaf and is not this check's.
///
/// # Errors
///
/// [`Refusal::Reconstructed`] naming the node and the entry Table 30 makes required.
pub(super) fn refuse_a_page_tree_no_reader_can_walk(
    document: &Document,
    root: ObjectId,
) -> Result<(), Refusal> {
    let Some(pages) = document
        .get(root)
        .as_dict()
        .and_then(|catalog| catalog.get("Pages").and_then(Object::as_reference))
    else {
        // `optimize::refuse_a_document_only_recovery_reads` has already refused a catalog whose
        // `/Pages` is no node, and a direct one carries no object for this walk to name.
        return Ok(());
    };
    let mut visited = 0_usize;
    walk(document, pages, 0, &mut visited)
}

/// One node and the nodes below it.
fn walk(
    document: &Document,
    node: ObjectId,
    depth: usize,
    visited: &mut usize,
) -> Result<(), Refusal> {
    if depth > MAX_DEPTH || *visited > MAX_NODES_VISITED {
        return Ok(());
    }
    let value = document.get(node);
    let Some(dict) = value.as_dict() else {
        return Ok(());
    };
    *visited = visited.saturating_add(1);
    let declares_a_node = document
        .get_key(dict, "Type")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Pages");
    let kids = dict.get("Kids").map(|kids| document.resolve(kids));
    if kids.is_none() && !declares_a_node {
        return Ok(());
    }
    let Some(kids) = kids.as_ref().and_then(Object::as_array) else {
        return Err(Refusal::Reconstructed(format!(
            "§7.7.3.2's Table 30 makes a page tree node's /Kids \"( Required ) An array of \
             indirect references to the immediate children of this node.\", and object {}'s is \
             {}; this reader reaches no page of the producer's through it, and rewriting it would \
             state a page tree no reader can walk",
            node.number,
            if kids.is_some() {
                "not an array"
            } else {
                "missing"
            }
        )));
    };
    if document.get_key(dict, "Count").as_integer().is_none() {
        return Err(Refusal::Reconstructed(format!(
            "§7.7.3.2's Table 30 makes a page tree node's /Count \"( Required ) The number of leaf \
             nodes (page objects) that are descendants of this node within the page tree.\", and \
             object {}'s is {}; this reader counts the leaves under its /Kids instead, and this \
             converter carries the producer's page tree as it stands, so the output would state \
             a node no reader can count",
            node.number,
            if dict.get("Count").is_some() {
                "not an integer"
            } else {
                "missing"
            }
        )));
    }
    for kid in kids.iter().filter_map(Object::as_reference) {
        walk(document, kid, depth.saturating_add(1), visited)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The expected refusals come from §7.7.3.2's Table 30: `/Kids` is a required array and
    //! `/Count` a required integer at every node; a dictionary with neither `/Kids` nor
    //! `/Type /Pages` is a leaf.
    use pdf_syntax::{Document, Limits, ObjectId};

    use super::refuse_a_page_tree_no_reader_can_walk;
    use crate::Refusal;

    /// A file of `objects`, numbered from 1 with object 1 the catalog, under a cross-reference
    /// table whose offsets are the ones written.
    fn file(objects: &[&str]) -> Document {
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(
                format!("{} 0 obj\n{object}\nendobj\n", index.saturating_add(1)).as_bytes(),
            );
        }
        let xref = bytes.len();
        let size = objects.len().saturating_add(1);
        bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n")
                .as_bytes(),
        );
        Document::open_with_limits(bytes, Limits::DEFAULT).expect("the fixture opens")
    }

    /// The sentence a refusal of the fixture states, or `None` where it is walked whole.
    fn refused(objects: &[&str]) -> Option<String> {
        match refuse_a_page_tree_no_reader_can_walk(&file(objects), ObjectId::new(1, 0)) {
            Ok(()) => None,
            Err(Refusal::Reconstructed(sentence)) => Some(sentence),
            Err(other) => panic!("refused under another variant: {other}"),
        }
    }

    const CATALOG: &str = "<< /Type /Catalog /Pages 2 0 R >>";
    const PAGE: &str = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>";

    #[test]
    fn a_tree_of_two_levels_with_both_entries_at_each_node_is_walked() {
        let nested = "<< /Type /Pages /Parent 2 0 R /Kids [4 0 R] /Count 1 >>";
        assert_eq!(
            refused(&[
                CATALOG,
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                nested,
                PAGE
            ]),
            None
        );
        // An indirect integer is an integer: Table 30 states the type, not the form.
        assert_eq!(
            refused(&[
                CATALOG,
                "<< /Type /Pages /Kids [4 0 R] /Count 3 0 R >>",
                "1",
                PAGE
            ]),
            None
        );
    }

    /// `poppler-91414-0-53.pdf`'s shape: the root's `/Count` names a stream.
    #[test]
    fn a_count_that_is_a_stream_is_refused_by_its_clause() {
        let sentence = refused(&[
            CATALOG,
            "<< /Type /Pages /Kids [4 0 R] /Count 3 0 R >>",
            "<< /Length 1 >>\nstream\n2\nendstream",
            PAGE,
        ])
        .expect("refused");
        assert!(sentence.contains("/Count") && sentence.contains("object 2's is not an integer"));
    }

    /// `poppler-937-0-fuzzed.pdf`'s shape: a declared node whose `/Kids` the parser could not read.
    #[test]
    fn a_declared_node_with_no_kids_array_is_refused_by_its_clause() {
        let missing = refused(&[CATALOG, "<< /Type /Pages /Count 1 >>"]).expect("refused");
        assert!(missing.contains("/Kids") && missing.contains("object 2's is missing"));
        let reference =
            refused(&[CATALOG, "<< /Type /Pages /Kids 3 0 R /Count 1 >>", PAGE]).expect("refused");
        assert!(reference.contains("object 2's is not an array"));
    }

    #[test]
    fn a_node_below_the_root_is_held_to_the_same_two_entries() {
        let sentence = refused(&[
            CATALOG,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Pages /Parent 2 0 R /Kids [4 0 R] >>",
            PAGE,
        ])
        .expect("refused");
        assert!(sentence.contains("object 3's is missing"));
    }

    /// A kid that states neither entry is a leaf, `pdf_model`'s reading, and not this check's.
    #[test]
    fn a_leaf_is_not_held_to_a_nodes_entries() {
        let untyped = "<< /Parent 2 0 R /MediaBox [0 0 100 100] >>";
        assert_eq!(
            refused(&[
                CATALOG,
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                untyped
            ]),
            None
        );
    }
}
