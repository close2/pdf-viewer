//! ISO 32000-2 §7.7.3.2's page tree nodes, as the three verbs that write a whole file read them:
//! a node no reader can walk refused by name before anything is written (ADR 2444), and a
//! `/Count` that is no integer restated from the leaves this reader walks (ADR 2456).
//!
//! # The reading
//!
//! Table 30 states two entries of every page tree node as required, and states their types:
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
//! with no `/Kids` array reaches no page.
//!
//! # Two entries, two answers
//!
//! **A node with no `/Kids` array is refused on every route**: the tree states no child, so the
//! pages this reader draws from it — none, or the ones a scan of the file finds — are not the
//! producer's tree, and nothing written from it would be.
//!
//! **A `/Count` that is no integer has a value the standard states**: the same cell defines it as
//! the number of descendant leaves, and its NOTE puts the restatement on the writer — "\[a\] PDF
//! writer shall ensure that the value of the Count key is consistent with the number of entries in
//! the Kids array and its descendants which definitively determines the number of descendant
//! pages". The conversion restates it, because ISO 19005's section 5.1 is failed there and says so
//! in a row of its own, `file-structure/page-tree-node-count-is-an-integer` — ADR 0947's second
//! rule, nothing changed that no failed requirement asked for, is kept by that row. The count
//! written is [`restated`]'s, the leaves `pdf_model::Pages` reaches beneath the node, so the
//! output states the pages this reader already drew. `optimize` and `redact` carry the producer's
//! nodes as they stand and refuse both shapes ([`refuse_a_page_tree_no_reader_can_walk`]): the
//! restatement is an object rewrite, and theirs is a closure copy with no object rewritten.

use std::collections::BTreeMap;

use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use super::decision::Because;
use crate::Refusal;

/// How deep a walk goes before it stops, `pdf_model`'s `MAX_TREE_DEPTH` and `crate::update`'s
/// bound for the same tree.
const MAX_DEPTH: usize = 64;

/// How many nodes a walk visits before it stops, `pdf_model`'s own bound over the same tree.
const MAX_NODES_VISITED: usize = 1 << 20;

/// Which of Table 30's two entries a refusal is asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    /// Both: a verb that carries every node as it stands.
    KidsAndCount,
    /// `/Kids` alone: the conversion, which restates a `/Count` its validator finds failing.
    KidsOnly,
}

/// Refuses a document whose page tree has a node §7.7.3.2's Table 30 makes unwalkable or
/// uncountable — `optimize`'s and `redact`'s check, which carry every node as it stands.
///
/// Every node the catalog's `/Pages` reaches through `/Kids` is read, bounded by depth and by a
/// node budget because `/Kids` holds references a file can make into a cycle. A dictionary is a
/// node where it states `/Kids` or declares `/Type /Pages`, `pdf_model`'s own reading; any other
/// kid is a leaf and is not this check's.
///
/// # Errors
///
/// [`Refusal::Reconstructed`] naming the node and the entry Table 30 makes required.
pub(crate) fn refuse_a_page_tree_no_reader_can_walk(
    document: &Document,
    root: ObjectId,
) -> Result<(), Refusal> {
    refuse(document, root, Held::KidsAndCount)
}

/// The same check held to `/Kids` alone — the conversion's, whose `/Count` is restated where
/// ISO 19005's section 5.1 row finds it failing.
///
/// # Errors
///
/// [`Refusal::Reconstructed`] naming the node whose `/Kids` is missing or not an array.
pub(super) fn refuse_a_page_tree_node_with_no_children(
    document: &Document,
    root: ObjectId,
) -> Result<(), Refusal> {
    refuse(document, root, Held::KidsOnly)
}

/// Both refusals' walk, from the catalog's `/Pages`.
fn refuse(document: &Document, root: ObjectId, held: Held) -> Result<(), Refusal> {
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
    walk(document, pages, 0, &mut visited, held)
}

/// One node and the nodes below it.
fn walk(
    document: &Document,
    node: ObjectId,
    depth: usize,
    visited: &mut usize,
    held: Held,
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
    if held == Held::KidsAndCount && document.get_key(dict, "Count").as_integer().is_none() {
        return Err(Refusal::Reconstructed(format!(
            "§7.7.3.2's Table 30 makes a page tree node's /Count \"( Required ) The number of leaf \
             nodes (page objects) that are descendants of this node within the page tree.\", and \
             object {}'s is {}; this reader counts the leaves under its /Kids instead, and this \
             verb carries the producer's page tree as it stands, so the output would state a \
             node no reader can count",
            node.number,
            if dict.get("Count").is_some() {
                "not an integer"
            } else {
                "missing"
            }
        )));
    }
    for kid in kids.iter().filter_map(Object::as_reference) {
        walk(document, kid, depth.saturating_add(1), visited, held)?;
    }
    Ok(())
}

/// Why a node written inside its parent's `/Kids` keeps its failing `/Count`.
const WRITTEN_INSIDE_ITS_PARENT: &str = "a page tree node whose Count is no integer is written \
     directly inside its parent's Kids array rather than as an object of its own, and this \
     conversion restates an entry at the object that holds it, which here is the parent - so the \
     restatement has no object to be written at (ADR 2456)";

/// Why a node reached by more than one route keeps its failing `/Count`.
const MORE_THAN_ONE_NUMBER: &str = "a page tree node whose Count is no integer is reached by \
     more than one route through the tree and has a different number of leaves beneath it on \
     each, which ISO 32000-2 section 7.7.3.2's \"A page tree shall not contain multiple indirect \
     references to the same page tree node.\" is about; no one integer is the number of its \
     descendants, and choosing one would decide which of the producer's routes the page count \
     follows (ADR 2456)";

/// Why a failing `/Count` the walk did not reach is not answered.
const NOT_REACHED: &str = "the requirement failed, and this conversion's walk of the page tree \
     restated no Count - the failing node is past the walk's depth or node bound - so a rewrite \
     of what it found would leave the failure in place (ADR 2456)";

/// The page tree nodes whose `/Count` is restated, and the count each states.
#[derive(Debug, Default)]
pub(super) struct RestatedCounts {
    /// The number of descendant leaves, by node, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, i64>,
}

/// Every node whose `/Count` is no integer, with the number of leaves this reader walks beneath
/// it, or why one cannot be restated.
///
/// The walk is `pdf_model::Pages`'s leaf count, read the same way entry by entry: a `/Kids` entry
/// that is a reference or a dictionary is a child, any other is stepped over; a child naming the
/// node or a node above it closes a cycle and is stepped over; a dictionary with no `/Kids` array
/// is a leaf unless it declares `/Type /Pages`; and both bounds are the reader's. So the root's
/// restated count is `Pages::len` wherever the tree, rather than a scan, yields the pages.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where a failing node is written inside its parent or lies past the
/// walk's bounds, and [`Because::TheFence`] where one is reached by two routes with different
/// counts beneath it.
pub(super) fn restated(document: &Document) -> Result<RestatedCounts, Because> {
    let root = document
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get("Pages").cloned());
    let mut tally = Tally {
        document,
        visited: 0,
        path: Vec::new(),
        out: RestatedCounts::default(),
        refused: None,
    };
    if let Some(root) = root {
        tally.leaves(&root, 0);
    }
    if let Some(because) = tally.refused {
        return Err(because);
    }
    if tally.out.at.is_empty() {
        return Err(Because::NotBuiltYet(NOT_REACHED));
    }
    Ok(tally.out)
}

/// One count of the tree.
struct Tally<'a> {
    /// The document whose tree is counted.
    document: &'a Document,
    /// Nodes visited, against [`MAX_NODES_VISITED`].
    visited: usize,
    /// The identities of the node being counted and the nodes above it, root first.
    path: Vec<ObjectId>,
    /// What it found to restate.
    out: RestatedCounts,
    /// The first node it could not answer for, which leaves the requirement refused.
    refused: Option<Because>,
}

impl Tally<'_> {
    /// The leaves beneath one `/Kids` entry, recording each failing node's number on the way out.
    fn leaves(&mut self, entry: &Object, depth: usize) -> usize {
        if depth > MAX_DEPTH || self.visited > MAX_NODES_VISITED {
            return 0;
        }
        let (id, value) = match entry {
            Object::Reference(id) => (Some(*id), self.document.get(*id)),
            Object::Dictionary(_) | Object::Stream(_) => (None, entry.clone()),
            _ => return 0,
        };
        let Some(dict) = value.as_dict() else {
            return 0;
        };
        self.visited = self.visited.saturating_add(1);
        let kids = self.document.get_key(dict, "Kids");
        let Some(kids) = kids.as_array() else {
            let declares_a_node = self
                .document
                .get_key(dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Pages");
            return usize::from(!declares_a_node);
        };
        self.path.extend(id);
        let mut count = 0_usize;
        for kid in kids {
            if let Object::Reference(kid) = kid
                && self.path.contains(kid)
            {
                continue;
            }
            count = count.saturating_add(self.leaves(kid, depth.saturating_add(1)));
        }
        if id.is_some() {
            self.path.pop();
        }
        if self.document.get_key(dict, "Count").as_integer().is_none() {
            self.record(id, count);
        }
        count
    }

    /// One failing node's count, or the reason it cannot be written.
    fn record(&mut self, id: Option<ObjectId>, count: usize) {
        let Some(id) = id else {
            self.refused
                .get_or_insert(Because::NotBuiltYet(WRITTEN_INSIDE_ITS_PARENT));
            return;
        };
        let Ok(count) = i64::try_from(count) else {
            self.refused
                .get_or_insert(Because::NotBuiltYet(NOT_REACHED));
            return;
        };
        if let Some(earlier) = self.out.at.insert(id, count)
            && earlier != count
        {
            self.refused
                .get_or_insert(Because::TheFence(MORE_THAN_ONE_NUMBER));
        }
    }
}

#[cfg(test)]
mod tests {
    //! The expected refusals come from §7.7.3.2's Table 30: `/Kids` is a required array and
    //! `/Count` a required integer at every node; a dictionary with neither `/Kids` nor
    //! `/Type /Pages` is a leaf.
    use pdf_syntax::{Document, Limits, ObjectId};

    use super::{
        refuse_a_page_tree_no_reader_can_walk, refuse_a_page_tree_node_with_no_children, restated,
    };
    use crate::Refusal;
    use crate::archive::decision::Because;

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

    /// The conversion's check is held to `/Kids` alone: its `/Count` is restated instead.
    #[test]
    fn the_conversions_check_refuses_a_missing_kids_and_passes_a_broken_count() {
        let streamed = file(&[
            CATALOG,
            "<< /Type /Pages /Kids [4 0 R] /Count 3 0 R >>",
            "<< /Length 1 >>\nstream\n2\nendstream",
            PAGE,
        ]);
        assert!(refuse_a_page_tree_node_with_no_children(&streamed, ObjectId::new(1, 0)).is_ok());
        let childless = file(&[CATALOG, "<< /Type /Pages /Count 1 >>"]);
        assert!(matches!(
            refuse_a_page_tree_node_with_no_children(&childless, ObjectId::new(1, 0)),
            Err(Refusal::Reconstructed(_))
        ));
    }

    /// The restated counts of `objects`, by object number.
    fn counts(objects: &[&str]) -> Result<Vec<(u32, i64)>, Because> {
        restated(&file(objects)).map(|found| {
            found
                .at
                .into_iter()
                .map(|(id, count)| (id.number, count))
                .collect()
        })
    }

    /// `poppler-91414-0-53.pdf`'s and `T02-02_008`'s shapes, one level down as well: the count a
    /// failing node is given is the leaves beneath it, and the root's is the page count the
    /// reader opens the source with.
    #[test]
    fn a_count_that_is_no_integer_is_restated_from_the_leaves_beneath_it() {
        let streamed = [
            CATALOG,
            "<< /Type /Pages /Kids [4 0 R 5 0 R] /Count 3 0 R >>",
            "<< /Length 1 >>\nstream\n7\nendstream",
            PAGE,
            PAGE,
        ];
        assert_eq!(counts(&streamed), Ok(vec![(2, 2)]));
        assert_eq!(pdf_model::Pages::new(&file(&streamed)).len(), 2);

        let nested = [
            CATALOG,
            "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 3 >>",
            "<< /Type /Pages /Parent 2 0 R /Kids [4 0 R 6 0 R] >>",
            PAGE,
            PAGE,
            "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 100 100] >>",
        ];
        assert_eq!(
            counts(&nested),
            Ok(vec![(3, 2)]),
            "a sound node is not restated"
        );
    }

    /// A kid naming a node above it is the cycle `pdf_model` steps over, and so is this count.
    #[test]
    fn a_cycle_is_stepped_over_as_the_reader_steps_over_it() {
        let looped = [CATALOG, "<< /Type /Pages /Kids [3 0 R 2 0 R] >>", PAGE];
        assert_eq!(counts(&looped), Ok(vec![(2, 1)]));
        assert_eq!(pdf_model::Pages::new(&file(&looped)).len(), 1);
    }

    /// Two nodes naming each other beneath one root: the failing node has two leaves beneath it on
    /// one route and one on the other, and no one integer is its count.
    #[test]
    fn a_node_with_a_different_count_on_each_route_keeps_its_refusal() {
        let tangled = [
            CATALOG,
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            "<< /Type /Pages /Parent 2 0 R /Kids [5 0 R 4 0 R] >>",
            "<< /Type /Pages /Parent 2 0 R /Kids [3 0 R 6 0 R] /Count 1 >>",
            PAGE,
            PAGE,
        ];
        assert!(matches!(counts(&tangled), Err(Because::TheFence(_))));
    }

    /// A failing node written inside its parent's array has no object of its own to restate.
    #[test]
    fn a_failing_node_written_inside_its_parent_keeps_its_refusal() {
        let direct = [
            CATALOG,
            "<< /Type /Pages /Kids [<< /Type /Pages /Kids [3 0 R] >>] /Count 1 >>",
            PAGE,
        ];
        assert!(matches!(counts(&direct), Err(Because::NotBuiltYet(_))));
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
