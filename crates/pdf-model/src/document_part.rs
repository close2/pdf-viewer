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
//! destinations do. That is `doc/HANDOVER.md`'s own habit arriving on schedule — "\[a]n
//! `inapplicable` row decays exactly as a `silent` one does", which §12.7.4.2's field names
//! demonstrated when §12.6.4.11's hide action made a field name decide whether an annotation is
//! drawn.
//!
//! So this module reads two things, and both are read because something outside the job ticket
//! asks for them. [`first_page`] is the first: the page a `DPart` begins at, which §12.6.4.5's
//! `GoToDp` action needs. [`hierarchy`] is the second, and §14.13.8 is what asks — "\[o]ne or more
//! files may be associated with any `DPart`", and a file nothing can enumerate is a file no panel
//! can list and no host can extract, which is the shape ADR 0295 found one clause family over.
//! Table 408's `/DPartRoot` is therefore walked, and `crate::attachment::attachments` is where
//! what it finds reaches a person.
//!
//! §14.12.4.2's `/DPM` is still the job ticket's and is still `inapplicable`; so is what a
//! `/RecordLevel` or a `/NodeNameList` *means*, which is why [`Hierarchy`] carries both and acts
//! on neither. Table 409's `/Metadata` is not read at all, and that is the standard's doing rather
//! than this module's: Errata Collection 3 Issue #290 withdraws the entry outright, replacing it
//! with the sentence that XMP metadata streams shall not be used in `DPart` dictionaries.

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// Most `DPart` dictionaries enumerated from one document.
///
/// §14.12.3 gives each page object to "one and only one `DPart` dictionary", so the leaves of a
/// conforming hierarchy cannot outnumber the document's pages and the nodes above them are a
/// handful of levels of branching over those. The standard states no number, which is why this is
/// a budget rather than a limit read off a clause (`CLAUDE.md` principle 3, trap 38): a file
/// describing more parts than this is describing something other than its own pages, and the walk
/// answers with what it has rather than following it.
///
/// It is the walk's only budget. Depth is not one: §14.12.2's tree rules bound no chain's length,
/// so a long chain is a deep hierarchy rather than a cycle, and a cycle is already finite through
/// the set of references walked (ADR 2984).
const MAX_PARTS: usize = 65_536;

/// One `DPart` dictionary of §14.12.2's hierarchy, as [`hierarchy`] enumerates it.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The dictionary itself, so a caller can read what Table 409 states on it.
    ///
    /// §14.13.8's `/AF` is the entry this program has a use for — "\[t]o associate files with a
    /// `DPart`, the appropriate `DPart` dictionary shall contain an AF entry whose value is an
    /// array of file specification dictionaries" — and `crate::attachment::associated` reads it.
    pub dictionary: Dictionary,
    /// Table 409's `/Start`: "the page object that defines the first page of the range of pages
    /// belonging to this `DPart` dictionary", as a reference, because a page is found in the page
    /// tree by identity.
    ///
    /// A node has none, the entry being exclusive with `/DParts`.
    pub start: Option<ObjectId>,
    /// Table 409's `/End`: the page object that defines the last page of the same range.
    ///
    /// Absent on a leaf whose range is one page, which Table 409 states outright — the entry is
    /// "\[r]equired if there is a Start key and the page range has more than one page, not present
    /// otherwise".
    pub end: Option<ObjectId>,
    /// How far below `/DPartRootNode` this node sits, counting the root node itself as zero.
    ///
    /// The level Table 408's `/NodeNameList` and `/RecordLevel` count in, both of which are
    /// stated as levels of the hierarchy rather than as counts of nodes.
    pub level: usize,
}

/// §14.12.2's document part hierarchy: Table 408's dictionary, and every node under it.
///
/// Answered by [`hierarchy`], and empty of parts only for a `/DPartRoot` whose `/DPartRootNode`
/// leads nowhere — Table 408 makes that entry `Required`, so a hierarchy with no root node is a
/// malformed file rather than an empty document.
#[derive(Debug, Clone, PartialEq)]
pub struct Hierarchy {
    /// ISO 32000-2 §14.12.4.1, Table 408's `/RecordLevel`, carried as the file states it.
    ///
    /// > This attribute may be used when a single PDF file encodes multiple documents. It
    /// > identifies the zero based level of the document part hierarchy where each DPart node of
    /// > that level corresponds to a component or hierarchy of components.
    ///
    /// What a component *is* belongs to the job ticket rather than to this program, which is why
    /// the entry is carried rather than acted on: §14.12.1 puts the whole family in "a production
    /// workflow, digital printing device, or other messaging channel", and the ledger's §14.12 row
    /// says what that makes `inapplicable` and what it does not.
    pub record_level: Option<i64>,
    /// Table 408's `/NodeNameList`, one name per level of the hierarchy, in the table's order.
    ///
    /// Table 408 states a constraint a reader can check rather than only a writer meet: "\[i]f
    /// present, the number of entries present in this array shall be equal to the number of `DPart`
    /// node levels in the document part hierarchy", which is [`Hierarchy::levels`].
    pub node_names: Vec<String>,
    /// Every `DPart` dictionary under `/DPartRootNode`, depth first, the root node first.
    ///
    /// The order is §14.12.3's, and it is the order that makes the list mean something: "\[t]he
    /// order of page objects as defined by the page tree shall be in the same order in which page
    /// objects are referenced from leaf node `DPart` dictionaries in a depth-first traversal of the
    /// document part hierarchy".
    pub parts: Vec<Part>,
}

impl Hierarchy {
    /// How many levels the hierarchy has — what Table 408 says `/NodeNameList`'s length shall
    /// equal.
    ///
    /// Zero for a hierarchy with no nodes at all, which is the one case the entry cannot describe.
    #[must_use]
    pub fn levels(&self) -> usize {
        self.parts
            .iter()
            .map(|part| part.level.saturating_add(1))
            .max()
            .unwrap_or_default()
    }
}

/// §14.12.2's hierarchy, read from Table 29's `/DPartRoot` down.
///
/// Two hops rather than one, and the standard says so twice:
///
/// > The root node of this hierarchy of dictionaries is identified by the DPartRoot dictionary
/// > referenced from the catalog dictionary.
///
/// with Table 408's `/DPartRootNode` — "\[s]hall be an indirect reference to the `DPart` dictionary
/// that is the root node of the document part tree structure" — as the second. §14.12.2's own
/// last sentence used to make it one hop, giving the catalog's key "an indirect object reference
/// to a `DPartRootNode` dictionary"; Errata Collection 3 Issue #609 replaces that phrase with the
/// `DPartRoot` dictionary and its cross-reference to Table 408, so the two sentences now agree and
/// this reads the shape they agree on. Table 29's own row gains "shall be an indirect reference"
/// from Issue #614, which costs a reader nothing: `Document::get_key` resolves either shape.
///
/// `None` for the overwhelming majority of documents, which state no `/DPartRoot` at all — no
/// document in any corpus on this disk states one, nor a `/DPart`:
/// `cargo run --release -p pdf-model --example witness_census -- DPartRoot DPart` reads 1479
/// files, opens 1452, and answers zero at each of its three layers. That makes this a reading of
/// the clause rather than of a file, and the tests below are hand-built for it (trap 4).
///
/// # What bounds the walk
///
/// §14.12.2: "A child `DPart` dictionary shall not be referenced by more than one parent `DPart`
/// dictionary", so a node reached twice is a malformed file and is not descended into twice. That
/// set is what makes a cycle finite, and `MAX_PARTS` is the budget on the whole.
#[must_use]
pub fn hierarchy(document: &Document) -> Option<Hierarchy> {
    let catalog = document.catalog().ok()?;
    let root = document.get_key(&catalog, "DPartRoot");
    let root = root.as_dict()?;

    let mut parts = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(id) = reference(root.get("DPartRootNode")) {
        seen.insert(id);
    }
    let node = document.get_key(root, "DPartRootNode");
    if let Some(dict) = node.as_dict() {
        walk(document, dict, &mut seen, |part, level| {
            parts.push(Part {
                dictionary: part.clone(),
                start: reference(part.get("Start")),
                end: reference(part.get("End")),
                level,
            });
            None::<()>
        });
    }

    let node_names = document
        .get_key(root, "NodeNameList")
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| document.resolve(item))
                // The name's own characters and not its `/`: Table 408 requires each to "conform
                // to the rules for an XML Name token", and a solidus is the syntax that
                // introduces a name object (§7.3.5) rather than part of one.
                .filter_map(|item| {
                    item.as_name()
                        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                })
                .collect()
        })
        .unwrap_or_default();

    Some(Hierarchy {
        record_level: document.get_key(root, "RecordLevel").as_integer(),
        node_names,
        parts,
    })
}

/// The `DPart` dictionaries from `root` down, depth first and the root first — §14.12.3's order
/// — handed to `visit` one at a time with their level, until it answers or [`MAX_PARTS`] nodes
/// have been walked.
///
/// The walk keeps its own stack rather than recursing, so a hierarchy as deep as its file makes
/// it costs memory in proportion to its nodes and no stack frames.
///
/// Errata Collection 3 Issue #612 rewrites Table 409's `/DParts` as an array that "shall be an
/// array of indirect references to immediate descendant `DPart` dictionaries", replacing the array
/// of arrays the printed table describes. A file written to the printed shape is still read:
/// an element that is an array contributes its elements, which costs nothing and loses no file,
/// because a `DPart` dictionary and an array of them are different types.
///
/// §14.12.2: "A child `DPart` dictionary shall not be referenced by more than one parent `DPart`
/// dictionary." A reference already walked is that file, and stopping there is both the clause's
/// answer and what makes a cycle finite; `seen` holds the references walked.
fn walk<T>(
    document: &Document,
    root: &Dictionary,
    seen: &mut BTreeSet<ObjectId>,
    mut visit: impl FnMut(&Dictionary, usize) -> Option<T>,
) -> Option<T> {
    let mut stack = vec![(root.clone(), 0_usize)];
    let mut walked = 0_usize;
    while let Some((part, level)) = stack.pop() {
        if walked >= MAX_PARTS {
            return None;
        }
        walked = walked.saturating_add(1);
        if let Some(found) = visit(&part, level) {
            return Some(found);
        }
        let children = document.get_key(&part, "DParts");
        let Some(rows) = children.as_array() else {
            continue;
        };
        let mut below = Vec::new();
        for row in rows {
            // The element itself, unresolved, where it is not an array: its reference is what
            // `seen` is keyed on.
            let candidates: Vec<Object> = match document.resolve(row) {
                Object::Array(items) => items.clone(),
                _ => vec![row.clone()],
            };
            for child in candidates {
                if let Object::Reference(id) = child
                    && !seen.insert(id)
                {
                    continue;
                }
                if let Some(dict) = document.resolve(&child).as_dict() {
                    below.push((dict.clone(), level.saturating_add(1)));
                }
            }
        }
        stack.extend(below.into_iter().rev());
    }
    None
}

/// The object an entry refers to, for the entries Table 409 requires to be indirect.
///
/// `/Start` and `/End` are each "an indirect reference to the page object", and the reference is
/// what a caller wants: a resolved dictionary could not be looked up in the page tree.
fn reference(entry: Option<&Object>) -> Option<ObjectId> {
    match entry {
        Some(Object::Reference(id)) => Some(*id),
        _ => None,
    }
}

/// The page object a `DPart` dictionary's range begins at, if it names one.
///
/// Table 409 makes `/Start` and `/DParts` exclusive — "\[s]hall not be present if a Start key is
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
/// `/DParts` is the flat array of references Errata Collection 3 Issue #612 makes it, and the
/// array of arrays the printed table describes is read too (`walk`).
#[must_use]
pub fn first_page(document: &Document, part: &Dictionary) -> Option<ObjectId> {
    // `/Start` "shall be an indirect reference to the page object", so the *reference* is what
    // this answers: a resolved dictionary could not be looked up in the page tree. The first
    // node in depth-first order that states one is the first leaf, since Table 409 gives a node
    // either `/Start` or `/DParts` and never both.
    walk(document, part, &mut BTreeSet::new(), |node, _| {
        reference(node.get("Start"))
    })
}

#[cfg(test)]
mod tests {
    use super::{first_page, hierarchy};
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

    /// §14.12.2's tree rules bound no chain's length, so a hierarchy two hundred levels deep is
    /// a valid file and is read whole: every node is listed and the root's first page is the
    /// deepest leaf's `/Start`. The children are Issue #612's flat arrays of references.
    #[test]
    fn a_deep_chain_is_read_whole() {
        const DEPTH: usize = 200;
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R /DPartRoot 4 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".to_owned(),
            "<< /Type /DPartRoot /DPartRootNode 5 0 R >>".to_owned(),
        ];
        for level in 0..DEPTH {
            objects.push(format!(
                "<< /Type /DPart /DParts [{} 0 R] >>",
                level.saturating_add(6)
            ));
        }
        objects.push("<< /Type /DPart /Start 3 0 R >>".to_owned());
        let objects: Vec<&str> = objects.iter().map(String::as_str).collect();
        let document = document(&objects);
        let parts = hierarchy(&document).expect("a hierarchy").parts;
        assert_eq!(parts.len(), DEPTH.saturating_add(1));
        assert_eq!(parts.last().map(|part| part.level), Some(DEPTH));
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
            })
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

    /// ISO 32000-2 §14.12.2 and Table 408: the catalog's `/DPartRoot`, then its `/DPartRootNode`.
    ///
    /// > The root node of this hierarchy of dictionaries is identified by the DPartRoot dictionary
    /// > referenced from the catalog dictionary.
    ///
    /// Two hops, and the fixture writes the intermediate dictionary rather than pointing the
    /// catalog straight at a `DPart`, because that is the shape Table 408 defines and the shape
    /// Errata Collection 3 Issue #609 settles §14.12.2's last sentence on. Every node of the
    /// hierarchy comes back, in §14.12.3's depth-first order, which is the half `first_page` never
    /// had: it descends to one leaf and stops.
    #[test]
    fn the_whole_hierarchy_is_enumerated_from_the_catalogs_root() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R /DPartRoot 5 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            // Table 408's DPartRoot dictionary, with both of its optional entries.
            "<< /Type /DPartRoot /DPartRootNode 6 0 R /RecordLevel 1 \
              /NodeNameList [/Volume /Statement] >>",
            "<< /Type /DPart /Parent 5 0 R /DParts [[7 0 R 8 0 R]] >>",
            "<< /Type /DPart /Parent 6 0 R /Start 3 0 R >>",
            "<< /Type /DPart /Parent 6 0 R /Start 4 0 R /End 4 0 R >>",
        ]);
        let hierarchy = hierarchy(&document).expect("the catalog names a /DPartRoot");

        assert_eq!(hierarchy.record_level, Some(1));
        assert_eq!(hierarchy.node_names, vec!["Volume", "Statement"]);
        assert_eq!(
            hierarchy
                .parts
                .iter()
                .map(|part| part.level)
                .collect::<Vec<_>>(),
            vec![0, 1, 1],
            "the root node and its two leaves, depth first"
        );
        assert_eq!(
            hierarchy
                .parts
                .iter()
                .map(|part| part.start.map(|id| id.number))
                .collect::<Vec<_>>(),
            vec![None, Some(3), Some(4)],
            "Table 409's /Start, absent on the node it is exclusive with /DParts on"
        );
        assert_eq!(
            hierarchy
                .parts
                .last()
                .and_then(|part| part.end)
                .map(|id| id.number),
            Some(4),
            "Table 409's /End, which the other leaf's one-page range does not state"
        );

        // Table 408: "If present, the number of entries present in this array shall be equal to
        // the number of DPart node levels in the document part hierarchy." The fixture is written
        // to meet it, so the check is of the reading rather than of the file.
        assert_eq!(hierarchy.levels(), hierarchy.node_names.len());
    }

    /// §14.12.2: a child referenced twice is walked once.
    ///
    /// > A child DPart dictionary shall not be referenced by more than one parent DPart dictionary.
    ///
    /// The clause makes the second reference a malformed file rather than a second part, and that
    /// is also what keeps the walk finite: this fixture's two nodes name each other, which no
    /// depth bound alone would stop from producing sixty-four parts.
    #[test]
    fn a_part_referenced_twice_is_one_part() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R /DPartRoot 4 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /DPartRoot /DPartRootNode 5 0 R >>",
            "<< /Type /DPart /DParts [[6 0 R]] >>",
            "<< /Type /DPart /DParts [[5 0 R 6 0 R]] >>",
        ]);
        let hierarchy = hierarchy(&document).expect("the catalog names a /DPartRoot");
        assert_eq!(
            hierarchy.parts.len(),
            2,
            "objects 5 and 6, each once, although each names the other"
        );
    }

    /// A `/DPartRoot` whose required `/DPartRootNode` is missing is a hierarchy with no parts.
    ///
    /// Table 408 makes the entry `Required`, so this is a malformed file; what the reading owes it
    /// is an empty answer rather than a panic or a guess at which dictionary was meant.
    #[test]
    fn a_root_with_no_node_answers_nothing() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R /DPartRoot 4 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /DPartRoot >>",
        ]);
        let hierarchy = hierarchy(&document).expect("the catalog names a /DPartRoot");
        assert!(hierarchy.parts.is_empty());
        assert_eq!(hierarchy.levels(), 0);
    }

    /// A document that states no `/DPartRoot` — which is every document in every corpus here.
    #[test]
    fn a_document_with_no_document_parts_has_no_hierarchy() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert!(hierarchy(&document).is_none());
    }
}
