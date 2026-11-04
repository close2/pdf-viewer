//! The structural parent tree rebuilt from the structure tree, where a source's loses a content
//! item (ISO 32000-2 §14.7.5.4, Table 359; `doc/adr/2674`).
//!
//! Each fixture plants one shape a pdf.js document has — a structure tree root with no
//! `/ParentTree`, one naming an object that does not exist, `Link` annotations an `OBJR` claims
//! stating no `/StructParent`, a parent tree with `/Kids` — and one shape no corpus document was
//! seen with: a producer entry naming an element that does not claim the item. Every output is read
//! back through `pdf_model::structure::ParentTree`, the lookup this reader draws with, page by page
//! and object by object, and held to its target again by the checker's row.

use super::*;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

const ROW: &str = "logical-structure/content-items-find-their-elements-through-the-parent-tree";

const TARGETS: [Target; 3] = [
    Target::Two(Level::A),
    Target::Two(Level::B),
    Target::Four(Flavour::Plain),
];

/// One page (object 3) whose content holds three sequences, `/MCID` 1 to 3 as `issue15340`'s, and
/// a structure tree written as `structure` from object 6 on; the catalog names object 6 as the
/// structure tree root, and the page states `page`.
fn tagged(target: Target, structure: &[&str], page: &str) -> Vec<u8> {
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        target,
        catalog: "/MarkInfo << /Marked true >> /StructTreeRoot 6 0 R /Lang (en)".to_owned(),
        objects: structure.iter().map(|&body| body.to_owned()).collect(),
        page: page.to_owned(),
        contents: Some((
            String::new(),
            b"/P << /MCID 1 >> BDC 0 0 1 rg 0 0 10 10 re f EMC \
              /P << /MCID 2 >> BDC 0 1 0 rg 10 0 10 10 re f EMC \
              /P << /MCID 3 >> BDC 1 0 0 rg 20 0 10 10 re f EMC"
                .to_vec(),
        )),
        ..base
    }
    .build()
}

/// Converts and answers the output, failing with the decisions where none was written.
fn converted(source: &[u8], target: Target) -> (Report, Vec<u8>) {
    let (report, output) = convert(source, target, Authorisations::default());
    let output = output.unwrap_or_else(|| {
        panic!(
            "{target}: no output; {:#?}",
            conversion(&report)
                .decided
                .iter()
                .filter(|decided| decided.requirement == ROW
                    || !matches!(decided.decision, Decision::Mechanical(_)))
                .collect::<Vec<_>>()
        )
    });
    (report, output)
}

/// The structure type of the element the page's sequence `mcid` finds through the parent tree.
fn found_from_the_page(document: &Document, mcid: i64) -> Option<Vec<u8>> {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(0).expect("the page");
    let element =
        pdf_model::structure::ParentTree::for_page(document, &page.dict).element(document, mcid)?;
    type_of(document, &element)
}

/// The element an object's `/StructParent` finds, read with the same lookup as a page's: the
/// clause makes an object's value "an indirect reference to the parent structure element", which
/// that lookup reads as an array of one.
fn found_from_the_object(document: &Document, object: ObjectId) -> Option<Dictionary> {
    let value = document.get(object);
    let dict = value.as_dict()?;
    let key = document.get_key(dict, "StructParent").as_integer()?;
    let mut as_holder = Dictionary::new();
    as_holder.insert(
        pdf_syntax::object::Name::new(&b"StructParents"[..]),
        Object::Integer(key),
    );
    pdf_model::structure::ParentTree::for_page(document, &as_holder).element(document, 0)
}

fn type_of(document: &Document, element: &Dictionary) -> Option<Vec<u8>> {
    document
        .get_key(element, "S")
        .as_name()
        .map(|name| name.as_bytes().to_vec())
}

/// The row's own answer on the output, which has to pass, and the rebuild's count in the report.
fn rebuilt(report: &Report, output: &[u8], target: Target, entries: usize) {
    let checked = holds(output, target);
    assert!(
        !checked.failures().any(|judgement| judgement.id == ROW),
        "{target}: the row passes the output"
    );
    assert_eq!(
        checked.verdict(),
        Verdict::Conforms,
        "{target}: {:?}",
        checked
            .failures()
            .map(|judgement| judgement.id)
            .collect::<Vec<_>>()
    );
    let decided = conversion(report)
        .decided
        .iter()
        .find(|decided| decided.requirement == ROW)
        .expect("the source failed the row");
    assert_eq!(
        decided.decision,
        Decision::Mechanical(Rewrite::ParentTreeRebuilt)
    );
    assert_eq!(decided.changed, entries, "{target}: the entries counted");
}

/// `issue15340`'s shape: three `P` elements claim the page's sequences 1 to 3, and the structure
/// tree root states no `/ParentTree` at all. The page takes key 0 and the tree is written into the
/// root, slot 0 `null` because no element claims a sequence 0.
#[test]
fn a_root_with_no_parent_tree_gains_one_read_from_its_elements() {
    let structure = [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R 9 0 R 10 0 R] >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 1 >>",
        "<< /Type /StructElem /S /H1 /P 7 0 R /Pg 3 0 R /K 2 >>",
        "<< /Type /StructElem /S /Span /P 7 0 R /Pg 3 0 R /K 3 >>",
    ];
    for target in TARGETS {
        let source = tagged(target, &structure, "");
        assert!(
            holds(&source, target)
                .failures()
                .any(|judgement| judgement.id == ROW),
            "{target}: the source fails the row"
        );
        let (report, output) = converted(&source, target);
        rebuilt(&report, &output, target, 1);
        let document = Document::open(output).expect("the output opens");
        assert!(
            the_packet_of(&document).contains(
                "the structural parent tree was rebuilt from the structure tree, 1 entry"
            ),
            "{target}: xmpMM:History names the rebuild: {}",
            the_packet_of(&document)
        );
        assert_eq!(found_from_the_page(&document, 0), None);
        assert_eq!(
            found_from_the_page(&document, 1).as_deref(),
            Some(&b"P"[..])
        );
        assert_eq!(
            found_from_the_page(&document, 2).as_deref(),
            Some(&b"H1"[..])
        );
        assert_eq!(
            found_from_the_page(&document, 3).as_deref(),
            Some(&b"Span"[..])
        );
        let catalog = document.catalog().expect("a catalog");
        let root = document.get_key(&catalog, "StructTreeRoot");
        let root = root.as_dict().expect("the root");
        assert_eq!(
            document.get_key(root, "ParentTreeNextKey").as_integer(),
            Some(1),
            "{target}: the next key moved past the one used"
        );
        assert!(
            matches!(root.get("K"), Some(Object::Reference(_))),
            "{target}: the producer's /K is left as written"
        );
    }
}

/// `issue20516`'s shape: the root's `/ParentTree` names an object the file does not hold, which
/// §7.3.10 makes null and so the same as no entry.
#[test]
fn a_parent_tree_naming_no_object_is_written_as_if_absent() {
    let structure = [
        "<< /Type /StructTreeRoot /K 7 0 R /ParentTree 40 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R] >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 1 >>",
    ];
    for target in TARGETS {
        let (report, output) = converted(&tagged(target, &structure, ""), target);
        rebuilt(&report, &output, target, 1);
        let document = Document::open(output).expect("the output opens");
        assert_eq!(
            found_from_the_page(&document, 1).as_deref(),
            Some(&b"P"[..])
        );
    }
}

/// `pdfjs_wikipedia`'s shape, under a parent tree with `/Kids` as `prefilled_f1040`'s: the page's
/// key is right and stays; two `Link` annotations an `OBJR` claims state no `/StructParent` and
/// take keys from `/ParentTreeNextKey`, in a new leaf; a third states a key whose entry names an
/// element that does not claim it, and is given a new key, the producer's entry left as it was.
#[test]
fn claimed_annotations_take_keys_in_a_new_leaf_and_a_right_key_is_kept() {
    // 6 the root, 7 the document element, 8 the page's `P`, 9-11 the `Link` elements, 12 the
    // parent tree's root, 13 its leaf, 14-16 the annotations.
    let structure = [
        "<< /Type /StructTreeRoot /K 7 0 R /ParentTree 12 0 R /ParentTreeNextKey 1 >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R 9 0 R 10 0 R 11 0 R] >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K [1 2 3] >>",
        "<< /Type /StructElem /S /Link /P 7 0 R /Pg 3 0 R \
            /K << /Type /OBJR /Obj 14 0 R /Pg 3 0 R >> >>",
        "<< /Type /StructElem /S /Link /P 7 0 R /Pg 3 0 R \
            /K [<< /Type /OBJR /Obj 15 0 R /Pg 3 0 R >>] >>",
        "<< /Type /StructElem /S /Link /P 7 0 R /Pg 3 0 R \
            /K << /Type /OBJR /Obj 16 0 R >> >>",
        "<< /Kids [13 0 R] >>",
        "<< /Limits [0 0] /Nums [0 [null 8 0 R 8 0 R 8 0 R]] >>",
        "<< /Type /Annot /Subtype /Link /F 4 /Border [0 0 0] /Rect [0 0 10 10] >>",
        "<< /Type /Annot /Subtype /Link /F 4 /Border [0 0 0] /Rect [10 0 20 10] >>",
        "<< /Type /Annot /Subtype /Link /F 4 /Border [0 0 0] /Rect [20 0 30 10] \
            /StructParent 0 >>",
    ];
    for target in TARGETS {
        let source = tagged(
            target,
            &structure,
            "/StructParents 0 /Tabs /S /Annots [14 0 R 15 0 R 16 0 R]",
        );
        let (report, output) = converted(&source, target);
        rebuilt(&report, &output, target, 3);
        let document = Document::open(output).expect("the output opens");
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(0).expect("the page");
        assert_eq!(
            document.get_key(&page.dict, "StructParents").as_integer(),
            Some(0),
            "{target}: the producer's right key is kept"
        );
        for mcid in 1..=3 {
            assert_eq!(
                found_from_the_page(&document, mcid).as_deref(),
                Some(&b"P"[..])
            );
        }
        let annotations: Vec<ObjectId> = document
            .get_key(&page.dict, "Annots")
            .as_array()
            .expect("the annotations")
            .iter()
            .filter_map(Object::as_reference)
            .collect();
        assert_eq!(annotations.len(), 3);
        let mut keys = Vec::new();
        for annotation in annotations {
            let element = found_from_the_object(&document, annotation)
                .unwrap_or_else(|| panic!("{target}: {annotation:?} finds no element"));
            assert_eq!(type_of(&document, &element).as_deref(), Some(&b"Link"[..]));
            let claims = document.get_key(&element, "K");
            let claimed = match &claims {
                Object::Array(items) => items.first().cloned(),
                other => Some(other.clone()),
            }
            .expect("a claim");
            let claimed = document.resolve(&claimed);
            let claimed = claimed.as_dict().expect("an OBJR");
            assert_eq!(
                claimed.get("Obj").and_then(Object::as_reference),
                Some(annotation),
                "{target}: the element found claims this very annotation"
            );
            let value = document.get(annotation);
            keys.push(
                document
                    .get_key(value.as_dict().expect("a dictionary"), "StructParent")
                    .as_integer(),
            );
        }
        assert_eq!(keys, vec![Some(1), Some(2), Some(3)], "{target}");
        let catalog = document.catalog().expect("a catalog");
        let root = document.get_key(&catalog, "StructTreeRoot");
        let root = root.as_dict().expect("the root");
        assert_eq!(
            document.get_key(root, "ParentTreeNextKey").as_integer(),
            Some(4)
        );
        let tree = document.get_key(root, "ParentTree");
        let kids = document
            .get_key(tree.as_dict().expect("the tree"), "Kids")
            .as_array()
            .map(<[Object]>::len);
        assert_eq!(
            kids,
            Some(2),
            "{target}: one new leaf beside the producer's"
        );
    }
}

/// An object claimed as a content item and holding claimed sequences at once is refused by name:
/// Table 359 says an object may be one or the other "but not both".
#[test]
fn an_object_both_item_and_container_is_refused_by_name() {
    let structure = [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R 9 0 R] >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 1 >>",
        "<< /Type /StructElem /S /Figure /P 7 0 R /Alt (a page) \
            /K << /Type /OBJR /Obj 3 0 R >> >>",
    ];
    let target = Target::Two(Level::B);
    let (report, output) = convert(
        &tagged(target, &structure, ""),
        target,
        Authorisations::default(),
    );
    assert!(output.is_none(), "nothing is written");
    let decided = conversion(&report)
        .decided
        .iter()
        .find(|decided| decided.requirement == ROW)
        .expect("the row failed");
    let because = format!("{:?}", decided.decision);
    assert!(because.contains("but not both"), "{because}");
}

/// A sequence only an element written inside its parent's `/K` claims is refused by name: the
/// entry §14.7.5.4 asks for is an indirect reference to the element, which it does not have.
#[test]
fn a_sequence_only_an_inline_element_claims_is_refused_by_name() {
    let structure = [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R \
            /K [<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 1 >>] >>",
    ];
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert(
        &tagged(target, &structure, ""),
        target,
        Authorisations::default(),
    );
    assert!(output.is_none(), "nothing is written");
    let decided = conversion(&report)
        .decided
        .iter()
        .find(|decided| decided.requirement == ROW)
        .expect("the row failed");
    let because = format!("{:?}", decided.decision);
    assert!(because.contains("written directly inside"), "{because}");
}

/// A page no element claims, stating the key the rebuild would otherwise hand out first: the new
/// key goes past it, so the unclaimed page still finds no element, rather than finding one that
/// never claimed it through an entry this conversion wrote.
#[test]
fn a_key_an_unclaimed_page_states_is_not_handed_out_again() {
    // 6 the root, 7 the document element, 8 the claiming `P`, 9 a second page no element claims,
    // stating `/StructParents 0` and drawing the same content.
    let structure = [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R] >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
            /Contents 4 0 R /StructParents 0 >>",
    ];
    for target in TARGETS {
        let base = match target {
            Target::Two(_) => Conforming::part_two(),
            Target::Four(_) => Conforming::default(),
        };
        let source = Conforming {
            target,
            catalog: "/MarkInfo << /Marked true >> /StructTreeRoot 6 0 R /Lang (en)".to_owned(),
            objects: structure.iter().map(|&body| body.to_owned()).collect(),
            more_pages: vec![9],
            contents: Some((
                String::new(),
                b"/P << /MCID 1 >> BDC 0 0 1 rg 0 0 10 10 re f EMC".to_vec(),
            )),
            ..base
        }
        .build();
        let (report, output) = converted(&source, target);
        rebuilt(&report, &output, target, 1);
        let document = Document::open(output).expect("the output opens");
        let pages = pdf_model::Pages::new(&document);
        let key = |index: usize| {
            let page = pages.get(index).expect("the page");
            document.get_key(&page.dict, "StructParents").as_integer()
        };
        assert_eq!(
            key(0),
            Some(1),
            "{target}: the claimed page's key is past 0"
        );
        assert_eq!(
            key(1),
            Some(0),
            "{target}: the unclaimed page's key is the producer's"
        );
        assert_eq!(
            found_from_the_page(&document, 1).as_deref(),
            Some(&b"P"[..])
        );
        let unclaimed = pages.get(1).expect("the second page");
        assert!(
            pdf_model::structure::ParentTree::for_page(&document, &unclaimed.dict)
                .element(&document, 1)
                .is_none(),
            "{target}: the unclaimed page finds no element"
        );
    }
}
