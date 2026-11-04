//! ISO 32000-2 §14.7.5.4 at section 5.1 of both parts: every content item finds its structure
//! element through the parent tree.
//!
//! The structure tree is written downward — an element's `/K` names its content items — and a
//! reader holding a page needs the other direction, which is what the parent tree is for:
//!
//! > The tree shall contain an entry for each object that is a content item of at least one
//! > structure element and for each content stream containing at least one marked-content
//! > sequence that is a content item. The key for each entry shall be an integer given as the
//! > value of the StructParent or StructParents entry in the object (see "Table 359 -Additional
//! > dictionary entries for structure element access").
//!
//! and, of the values, that a content stream's is "an array of indirect references to the
//! sequences' parent structure elements", indexed by the marked-content identifier, while an
//! object's is "an indirect reference to the parent structure element". ISO 32000-1 section 14.7.4.4,
//! part 2's base document, states the same sentences.
//!
//! # Which sequences the clause binds
//!
//! A content item is what an element claims: §14.7.5.1.1 makes content items graphical objects
//! "associated with structure elements as described in the following subclauses", and §14.7.5.2
//! associates a sequence by an element's `/K` naming it. So the population is read from the
//! structure tree, not from the content streams: a sequence carrying an `/MCID` that no element
//! claims is no content item, and this row says nothing about it. The converse is the defect
//! the row exists for — an element claims a sequence, and the page it is on states no key, or a
//! key the parent tree does not hold, or an entry that leads to no element claiming it. That is
//! what an appended page whose parent tree leaf was withheld looks like (`doc/adr/2673`).
//!
//! # Two leniencies, both in the document's favour
//!
//! - A sequence two elements claim is accepted where the entry names either: the parent tree can
//!   name one, and which is the producer's is not this clause's question.
//! - A content item whose page and stream are both unstated is passed over, because no object is
//!   left to hold the key; Table 355's `/Pg` is a different requirement from this one.
//!
//! # Bounds, refused by name
//!
//! The structure tree and the parent tree are references a document controls. The element walk
//! visits each indirect element once, stops at [`MOST_STRUCTURE_DEPTH`] levels and at
//! [`MOST_STRUCTURE_VISITS`] elements and content items; the parent tree's walk stops at the same
//! depth and at [`MOST_PARENT_TREE_ENTRIES`] nodes and pairs. Past either bound, and wherever the
//! parent tree is not a number tree §7.9.7 describes, the row fails with a sentence naming why,
//! because a verdict that passed a tree it could not finish reading would be asserting what it had
//! not checked.

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::structure::{Child, Tree};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::examination::Examination;
use crate::finding::{Findings, Where};

/// How deep either walk descends: `pdf_model`'s own bound over the structure tree, and
/// `pdf_syntax`'s over a number tree.
const MOST_STRUCTURE_DEPTH: usize = 64;

/// How many elements and content items the structure walk visits: `pdf_model`'s `MAX_ELEMENTS`.
const MOST_STRUCTURE_VISITS: usize = 1 << 20;

/// How many nodes and key-value pairs the parent tree's walk reads.
const MOST_PARENT_TREE_ENTRIES: usize = 1 << 20;

/// The elements claiming one content item.
#[derive(Debug, Default)]
struct Claimants {
    /// The claiming elements written as objects of their own.
    indirect: BTreeSet<ObjectId>,
    /// Whether an element written inside its parent's `/K` claims it too; such an element has no
    /// reference a parent tree could hold, so any element the entry names is accepted for it.
    inline: bool,
}

impl Claimants {
    fn add(&mut self, element: Option<ObjectId>) {
        match element {
            Some(id) => {
                self.indirect.insert(id);
            }
            None => self.inline = true,
        }
    }

    /// Whether a parent tree value names one of these elements.
    fn named_by(&self, document: &Document, value: Option<&Object>) -> bool {
        match value {
            Some(Object::Reference(id)) => {
                self.indirect.contains(id) || (self.inline && document.get(*id).as_dict().is_some())
            }
            Some(Object::Dictionary(_)) => self.inline,
            _ => false,
        }
    }
}

/// Every content item the structure tree claims, by the object that has to hold its key.
#[derive(Debug, Default)]
struct Claims {
    /// §14.7.5.2's sequences: the page or stream they are in, then the `/MCID`.
    sequences: BTreeMap<ObjectId, BTreeMap<i64, Claimants>>,
    /// §14.7.5.3's objects, by the object.
    objects: BTreeMap<ObjectId, Claimants>,
}

impl Claims {
    /// Walks the structure tree depth first, each indirect element once.
    fn gather(document: &Document, tree: &Tree) -> Result<Self, String> {
        let mut claims = Self::default();
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        let mut visits = 0_usize;
        // The root's own `/K` names elements only (Table 354), so a content item there has no
        // parent element to be found and is not gathered.
        let mut stack: Vec<(Option<ObjectId>, Dictionary, usize)> = Vec::new();
        for (child, id) in tree.identified_children(document, None) {
            if let Child::Element(dict) = child
                && id.is_none_or(|id| seen.insert(id))
            {
                stack.push((id, dict, 0));
            }
        }
        while let Some((element, dict, depth)) = stack.pop() {
            if depth >= MOST_STRUCTURE_DEPTH {
                return Err(format!(
                    "the structure tree is deeper than the {MOST_STRUCTURE_DEPTH} levels this \
                     check reads, so whether each content item finds its element was not decided"
                ));
            }
            for (child, id) in tree.identified_children(document, Some(&dict)) {
                visits = visits.saturating_add(1);
                if visits > MOST_STRUCTURE_VISITS {
                    return Err(format!(
                        "the structure tree holds more than the {MOST_STRUCTURE_VISITS} elements \
                         and content items this check reads, so whether each content item finds \
                         its element was not decided"
                    ));
                }
                match child {
                    Child::Element(child) => {
                        if id.is_none_or(|id| seen.insert(id)) {
                            stack.push((id, child, depth.saturating_add(1)));
                        }
                    }
                    Child::MarkedContent {
                        mcid, page, stream, ..
                    } => {
                        if let Some(holder) = stream.or(page) {
                            claims
                                .sequences
                                .entry(holder)
                                .or_default()
                                .entry(mcid)
                                .or_default()
                                .add(element);
                        }
                    }
                    Child::Object { object, .. } => {
                        claims.objects.entry(object).or_default().add(element);
                    }
                }
            }
        }
        Ok(claims)
    }

    fn is_empty(&self) -> bool {
        self.sequences.is_empty() && self.objects.is_empty()
    }
}

/// Every key-value pair of a number tree, the values unresolved so that a reference's identity
/// survives, or the sentence naming why the tree cannot be read.
///
/// ISO 32000-2 §7.9.7. Where a key is stated twice the first is kept. `/Limits` are not consulted:
/// whether a reader descending by them reaches each key is §7.9.7's own question, and this one
/// asks only whether the tree contains the entry.
fn number_tree_entries(
    document: &Document,
    root: &Dictionary,
) -> Result<BTreeMap<i64, Object>, &'static str> {
    let mut entries = BTreeMap::new();
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let mut read = 0_usize;
    let mut stack: Vec<(Dictionary, usize)> = vec![(root.clone(), 0)];
    while let Some((node, depth)) = stack.pop() {
        if depth > MOST_STRUCTURE_DEPTH {
            return Err("it is deeper than the 64 levels this check reads");
        }
        match document.get_key(&node, "Nums") {
            Object::Null => {}
            Object::Array(pairs) => {
                if pairs.len() % 2 != 0 {
                    return Err("a node's Nums holds a key with no value");
                }
                for pair in pairs.chunks_exact(2) {
                    let [key, value] = pair else {
                        continue;
                    };
                    let Some(key) = document.resolve(key).as_integer() else {
                        return Err("a node's Nums holds a key that is not an integer");
                    };
                    read = read.saturating_add(1);
                    if read > MOST_PARENT_TREE_ENTRIES {
                        return Err("it holds more entries than the 1048576 this check reads");
                    }
                    entries.entry(key).or_insert_with(|| value.clone());
                }
            }
            _ => return Err("a node's Nums is not an array"),
        }
        match document.get_key(&node, "Kids") {
            Object::Null => {}
            Object::Array(kids) => {
                // Pushed in reverse so that the first kid is read first, and its first key kept.
                for kid in kids.iter().rev() {
                    if let Object::Reference(id) = kid
                        && !seen.insert(*id)
                    {
                        return Err("a node is reached twice, so the tree is not a tree");
                    }
                    let Some(kid) = document.resolve(kid).as_dict().cloned() else {
                        return Err("a node's Kids names something that is not a dictionary");
                    };
                    read = read.saturating_add(1);
                    if read > MOST_PARENT_TREE_ENTRIES {
                        return Err("it holds more nodes than the 1048576 this check reads");
                    }
                    stack.push((kid, depth.saturating_add(1)));
                }
            }
            _ => return Err("a node's Kids is not an array"),
        }
    }
    Ok(entries)
}

/// Section 5.1 of both parts, at ISO 32000-2 §14.7.5.4 and Table 359.
///
/// Each page or stream holding a sequence an element claims states `/StructParents`, the parent
/// tree holds that key, the value is an array, and its entry at each claimed `/MCID` names an
/// element claiming that sequence; each object an element claims states `/StructParent`, the tree
/// holds that key, and the value names an element claiming the object. One finding per holder.
pub(super) fn content_items_find_their_elements(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Some(tree) = Tree::of(document) else {
        return;
    };
    let claims = match Claims::gather(document, &tree) {
        Ok(claims) => claims,
        Err(why) => {
            findings.record(Where::file().named("StructTreeRoot"), why);
            return;
        }
    };
    if claims.is_empty() {
        return;
    }
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let root = document.get_key(&catalog, "StructTreeRoot");
    let Some(root) = root.as_dict() else {
        return;
    };
    let parent_tree = document.get_key(root, "ParentTree");
    let Some(parent_tree) = parent_tree.as_dict() else {
        findings.record(
            Where::file().named("ParentTree"),
            "the structure tree root states no parent tree, though its elements hold content \
             items",
        );
        return;
    };
    let entries = match number_tree_entries(document, parent_tree) {
        Ok(entries) => entries,
        Err(why) => {
            findings.record(
                Where::file().named("ParentTree"),
                format!("the parent tree cannot be read: {why}"),
            );
            return;
        }
    };
    sequences_find_their_elements(document, &entries, &claims.sequences, findings);
    objects_find_their_elements(document, &entries, &claims.objects, findings);
}

/// The sequences half of [`content_items_find_their_elements`]: one finding per holder.
fn sequences_find_their_elements(
    document: &Document,
    entries: &BTreeMap<i64, Object>,
    holders: &BTreeMap<ObjectId, BTreeMap<i64, Claimants>>,
    findings: &mut Findings,
) {
    for (holder, sequences) in holders {
        let value = document.get(*holder);
        let Some(dict) = value.as_dict() else {
            continue;
        };
        let place = || Where::object(*holder).named("StructParents");
        let Some(key) = document.get_key(dict, "StructParents").as_integer() else {
            findings.record(
                place(),
                "a page or stream holding marked-content sequences a structure element claims \
                 states no StructParents",
            );
            continue;
        };
        let Some(entry) = entries.get(&key) else {
            findings.record(
                place(),
                format!("the parent tree holds no entry under this StructParents key, {key}"),
            );
            continue;
        };
        let entry = document.resolve(entry);
        let Some(array) = entry.as_array() else {
            findings.record(
                place(),
                format!("the parent tree's entry under key {key} is not an array"),
            );
            continue;
        };
        let lost: Vec<i64> = sequences
            .iter()
            .filter(|(mcid, claimants)| {
                let value = usize::try_from(**mcid)
                    .ok()
                    .and_then(|index| array.get(index));
                !claimants.named_by(document, value)
            })
            .map(|(mcid, _)| *mcid)
            .collect();
        if let Some(first) = lost.first() {
            findings.record(
                place(),
                format!(
                    "{} of its claimed marked-content sequences, the first MCID {first}, find no \
                     element claiming them in the parent tree's entry under key {key}",
                    lost.len()
                ),
            );
        }
    }
}

/// The objects half of [`content_items_find_their_elements`]: one finding per object.
fn objects_find_their_elements(
    document: &Document,
    entries: &BTreeMap<i64, Object>,
    objects: &BTreeMap<ObjectId, Claimants>,
    findings: &mut Findings,
) {
    for (object, claimants) in objects {
        let value = document.get(*object);
        let Some(dict) = value.as_dict() else {
            continue;
        };
        let place = || Where::object(*object).named("StructParent");
        let Some(key) = document.get_key(dict, "StructParent").as_integer() else {
            findings.record(
                place(),
                "an object a structure element claims as a content item states no StructParent",
            );
            continue;
        };
        if !claimants.named_by(document, entries.get(&key)) {
            findings.record(
                place(),
                format!(
                    "the parent tree's entry under this StructParent key, {key}, names no element \
                     claiming the object"
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use pdf_syntax::Document;

    use super::content_items_find_their_elements;
    use crate::Examination;
    use crate::finding::Findings;
    use crate::target::{Level, Target};

    /// A file the real parser opens: `objects` numbered from 1, object 1 the catalog.
    fn document_of(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let number = index.saturating_add(1);
            let _ = writeln!(out, "{number} 0 obj\n{body}\nendobj");
        }
        let start = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer << /Size {size} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).unwrap_or_else(|_| Document::empty())
    }

    /// What the row says, as its findings' sentences.
    fn said(objects: &[&str]) -> Vec<String> {
        let document = document_of(objects);
        let mut findings = Findings::default();
        content_items_find_their_elements(
            &Examination::new(&document, Target::Two(Level::A)),
            &mut findings,
        );
        findings
            .kept()
            .iter()
            .map(|found| found.what.clone())
            .collect()
    }

    /// A tagged one-page document: object 3 the page stating `page`, 4 the root naming the parent
    /// tree as `tree`, 5 a `P` claiming the page's MCID 0 and an annotation 7, 6 a second `P`
    /// claiming MCID 1, 7 the annotation stating `annotation`, and `more` from object 8 on.
    fn tagged(page: &str, tree: &str, annotation: &str, more: &[&str]) -> Vec<String> {
        let page =
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Annots [7 0 R] {page} >>");
        let root = format!("<< /Type /StructTreeRoot /K [5 0 R 6 0 R] {tree} >>");
        let annotation =
            format!("<< /Type /Annot /Subtype /Link /Rect [0 0 1 1] /P 3 0 R {annotation} >>");
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &page,
            &root,
            "<< /Type /StructElem /S /P /P 4 0 R /Pg 3 0 R \
             /K [0 << /Type /OBJR /Obj 7 0 R >>] >>",
            "<< /Type /StructElem /S /P /P 4 0 R /Pg 3 0 R /K 1 >>",
            &annotation,
        ];
        objects.extend_from_slice(more);
        said(&objects)
    }

    /// §14.7.5.4's two forms, each found: the page's array by `/MCID`, the annotation's reference.
    #[test]
    fn every_content_item_finds_its_element() {
        assert_eq!(
            tagged(
                "/StructParents 0",
                "/ParentTree << /Nums [0 [5 0 R 6 0 R] 1 5 0 R] >>",
                "/StructParent 1",
                &[],
            ),
            Vec::<String>::new()
        );
        // The same entries one level down, through `/Kids`.
        assert_eq!(
            tagged(
                "/StructParents 0",
                "/ParentTree << /Kids [8 0 R] >>",
                "/StructParent 1",
                &["<< /Limits [0 1] /Nums [0 [5 0 R 6 0 R] 1 5 0 R] >>"],
            ),
            Vec::<String>::new()
        );
    }

    /// ADR 2673's withheld leaf: the page's key is in no node, so neither sequence is found.
    #[test]
    fn a_page_whose_key_the_tree_does_not_hold_fails() {
        let said = tagged(
            "/StructParents 4",
            "/ParentTree << /Kids [8 0 R] >>",
            "/StructParent 1",
            &["<< /Limits [0 1] /Nums [0 [5 0 R 6 0 R] 1 5 0 R] >>"],
        );
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].contains("no entry under this StructParents key, 4"),
            "{said:?}"
        );
    }

    /// Every other way the route breaks, one finding each.
    #[test]
    fn each_broken_route_is_named() {
        let tree = "/ParentTree << /Nums [0 [5 0 R 6 0 R] 1 5 0 R] >>";
        let one = |page: &str, tree: &str, annotation: &str| {
            let said = tagged(page, tree, annotation, &[]);
            assert_eq!(said.len(), 1, "{page} {tree} {annotation}: {said:?}");
            said[0].clone()
        };
        assert!(one("", tree, "/StructParent 1").contains("states no StructParents"));
        assert!(one("/StructParents 0", tree, "").contains("states no StructParent"));
        assert!(
            one(
                "/StructParents 0",
                "/ParentTree << /Nums [0 5 0 R 1 5 0 R] >>",
                "/StructParent 1"
            )
            .contains("is not an array")
        );
        // The array names the elements the wrong way round, and stops short of MCID 1.
        assert!(
            one(
                "/StructParents 0",
                "/ParentTree << /Nums [0 [6 0 R] 1 5 0 R] >>",
                "/StructParent 1"
            )
            .contains("2 of its claimed marked-content sequences, the first MCID 0")
        );
        assert!(one("/StructParents 0", tree, "/StructParent 0").contains("names no element"));
        assert!(one("/StructParents 0", "", "/StructParent 1").contains("states no parent tree"));
    }

    /// A parent tree that is not §7.9.7's number tree is refused by name rather than half read.
    #[test]
    fn a_parent_tree_that_cannot_be_read_is_refused_by_name() {
        for (tree, more, why) in [
            ("/ParentTree << /Nums 3 >>", &[][..], "Nums is not an array"),
            (
                "/ParentTree << /Nums [0] >>",
                &[][..],
                "a key with no value",
            ),
            (
                "/ParentTree << /Nums [/a 5 0 R] >>",
                &[][..],
                "not an integer",
            ),
            (
                "/ParentTree << /Kids [8 0 R 8 0 R] >>",
                &["<< /Nums [] >>"][..],
                "reached twice",
            ),
            (
                "/ParentTree << /Kids [8 0 R] >>",
                &["<< /Kids [8 0 R] >>"][..],
                "reached twice",
            ),
            ("/ParentTree << /Kids [3] >>", &[][..], "not a dictionary"),
        ] {
            let said = tagged("/StructParents 0", tree, "/StructParent 1", more);
            assert_eq!(said.len(), 1, "{tree}: {said:?}");
            assert!(
                said[0].contains("cannot be read") && said[0].contains(why),
                "{said:?}"
            );
        }
    }

    /// The depth bound both sides: sixty-four nested elements are read, sixty-five refused.
    #[test]
    fn a_structure_tree_past_the_depth_bound_is_refused_by_name() {
        let nested = |levels: usize| {
            let mut objects: Vec<String> = vec![
                "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>".to_owned(),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>".to_owned(),
                "<< /Type /StructTreeRoot /K 5 0 R /ParentTree << /Nums [0 [LAST]] >> >>"
                    .to_owned(),
            ];
            for level in 0..levels {
                let number = level + 5;
                let kids = if level + 1 == levels {
                    "0".to_owned()
                } else {
                    format!("{} 0 R", number + 1)
                };
                objects.push(format!("<< /S /Div /Pg 3 0 R /K {kids} >>"));
            }
            let last = format!("{} 0 R", levels + 4);
            objects[3] = objects[3].replace("LAST", &last);
            let borrowed: Vec<&str> = objects.iter().map(String::as_str).collect();
            said(&borrowed)
        };
        assert_eq!(nested(64), Vec::<String>::new());
        let refused = nested(65);
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(
            refused[0].contains("deeper than the 64 levels"),
            "{refused:?}"
        );
    }

    /// An `/MCID` no element claims is no content item, so its page owes the tree nothing.
    #[test]
    fn an_unclaimed_sequence_owes_nothing() {
        assert_eq!(
            said(&[
                "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 5 0 R >>",
                "<< /Type /StructTreeRoot /K [] >>",
                "<< /Length 30 >>\nstream\n/P << /MCID 0 >> BDC 0 0 m EMC\nendstream",
            ]),
            Vec::<String>::new()
        );
    }
}
