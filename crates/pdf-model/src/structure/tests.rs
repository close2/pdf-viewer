//! The tests of the structure tree's reading: one clause's sentence each, with the fixture
//! that states it.

use super::{
    Artifact, ArtifactKind, AttributeObject, BlockProgression, BlockSpacing, Boundary, Category,
    CellFacts, CellPlacement, Checked, Child, FieldRole, HeaderScope, ListContinuation, ListEntry,
    MAX_TABLE_COLUMNS, MappedRole, Nonstandard, Owner, ParentTree, Priority, StandardType,
    TableGrid, TableStack, Tree, WritingMode, actual_text, allocation_rectangle,
    alternate_description, annotation_languages, annotation_rectangles, language,
    list_predecessors, outside_sequences, table_cell_rectangles, well_formed_language_tag,
};
use pdf_syntax::{Dictionary, Document, Object};
use std::collections::BTreeSet;

/// §14.7.5.3's NOTE 1, at the one place it decides anything: the bytes of an annotation's
/// readback that a marked-content sequence inside its appearance covers are that sequence's,
/// so the object's own pieces are what is left — in order, and none where the sequences
/// cover it all (ADR 1970).
#[test]
fn an_object_gives_up_the_bytes_its_sequences_cover() {
    let span = |range: std::ops::Range<usize>| crate::content::MarkedSpan {
        mcid: 0,
        stream: crate::content::ContentStream::Page,
        range,
        drawn: None,
        enclosed_a_refusal: false,
    };
    assert_eq!(outside_sequences(10..30, &[]), vec![10..30]);
    assert_eq!(
        outside_sequences(
            10..30,
            &[span(25..40), span(0..12), span(15..18), span(16..20)]
        ),
        vec![12..15, 20..25],
        "unordered and overlapping sequences, and two that reach past either end"
    );
    assert_eq!(
        outside_sequences(10..30, &[span(5..35)]),
        Vec::<std::ops::Range<usize>>::new()
    );
    assert_eq!(outside_sequences(10..30, &[span(30..40)]), vec![10..30]);
}

/// RFC 5646 section 2.1's grammar, one production at a time, in both directions.
///
/// The accepted side walks every branch — a bare language, extended-language subtags, a
/// script, both region forms, stacked variants, an extension, a private tail, a private
/// tag standing alone, a grandfathered tag, and §14.9.2.2's case-insensitivity. The
/// refused side is what real producers write where a tag belongs (prose, a comma, an
/// underscore) and each way a subtag can break its role's bounds.
#[test]
fn a_language_tag_is_judged_by_bcp_47s_grammar() {
    for accepted in [
        "en",
        "deu",
        "en-US",
        "EN-us",
        "es-419",
        "zh-Hant-TW",
        "zh-yue-HK",
        "de-CH-1901",
        "sl-rozaj-biske",
        "en-a-myext-b-another",
        "az-Arab-x-AZE-derbend",
        "x-private",
        "i-klingon",
        "zh-min-nan",
        "abcdefgh",
    ] {
        assert!(well_formed_language_tag(accepted), "{accepted:?} is a tag");
    }
    for refused in [
        "",
        "a",
        "en-",
        "-en",
        "en--US",
        "en_US",
        "German, not a tag",
        "en US",
        "ninecharss",
        "en-a",
        "x",
        "419",
        "en-US-x",
    ] {
        assert!(!well_formed_language_tag(refused), "{refused:?} is not");
    }
}

/// Builds a document from object bodies numbered from 1.
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

/// A marked-content identifier finds its element through the number tree.
///
/// Two lookups, both of which the clause states: the page's `/StructParents` is the key
/// into `/ParentTree`, and the `/MCID` is "a zero-based index into the array" that comes
/// back. The fixture puts two elements in the array so that indexing, rather than taking
/// the first, is what passes.
#[test]
fn a_marked_content_identifier_finds_its_element() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 7 >>",
        "<< /Type /StructTreeRoot /ParentTree 5 0 R >>",
        "<< /Nums [7 [6 0 R 8 0 R]] >>",
        "<< /Type /StructElem /S /P >>",
        "<< /Unused true >>",
        "<< /Type /StructElem /S /Span /ActualText (fi) >>",
    ]);
    let pages = crate::page::Pages::new(&doc);
    let page = pages.get(0).expect("page one");
    let parents = ParentTree::for_page(&doc, &page.dict);

    assert!(!parents.is_empty());
    let first = parents.element(&doc, 0).expect("the element for /MCID 0");
    assert_eq!(
        doc.get_key(&first, "S")
            .as_name()
            .map(|s| s.as_bytes().to_vec()),
        Some(b"P".to_vec())
    );
    assert_eq!(
        parents.element(&doc, 1).and_then(|e| actual_text(&doc, &e)),
        Some("fi".to_owned()),
        "the second entry, indexed rather than taken first"
    );
    assert!(parents.element(&doc, 2).is_none());
}

/// §14.7.5.4's three routes from a page to the elements its content items belong to.
///
/// One fixture per kind of content item the clause names, because each is keyed differently
/// and a reader that found two of the three would look right on nearly every file: the page's
/// own `/StructParents` array for its marked-content sequences, the annotation's
/// `/StructParent` for §14.7.5.3's object reference, and the form `XObject`'s — which is the
/// clause's own EXAMPLE 1. The three elements are siblings under one `Sect`, so `ancestry`
/// has something to add that `elements_on_page` cannot know.
#[test]
fn a_page_finds_its_elements_through_the_structural_parent_tree() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 \
         /Annots [10 0 R] /Resources << /XObject << /Fm 11 0 R >> >> >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /ParentTree 9 0 R >>",
        "<< /Type /StructElem /S /Sect /P 4 0 R /Pg 3 0 R /K [6 0 R 7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K [0] >>",
        "<< /Type /StructElem /S /Form /P 5 0 R \
         /K [<< /Type /OBJR /Obj 10 0 R /Pg 3 0 R >>] >>",
        "<< /Type /StructElem /S /Figure /P 5 0 R \
         /K [<< /Type /OBJR /Obj 11 0 R /Pg 3 0 R >>] >>",
        "<< /Nums [0 [6 0 R] 1 7 0 R 2 8 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [0 0 1 1] /StructParent 1 >>",
        "<< /Type /XObject /Subtype /Form /Length 0 /StructParent 2 >>\nstream\n\nendstream",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let page = crate::page::Pages::new(&doc).get(0).expect("page one");

    let reference = |number| pdf_syntax::ObjectId {
        number,
        generation: 0,
    };
    let found = tree
        .elements_on_page(&doc, &page.dict)
        .expect("the page states `/StructParents`");
    assert_eq!(
        found,
        [reference(6), reference(7), reference(8)]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        "the sequence's element, the annotation's and the form XObject's"
    );

    // Table 355's `/P`, which is what a walk coming down from the root needs: the `Sect` is
    // not named by the parent tree and every one of the three is under it.
    assert_eq!(
        tree.ancestry(&doc, &found),
        [reference(5), reference(6), reference(7), reference(8)]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );

    // A page that states no `/StructParents` answers `None` — "the file has not said" —
    // rather than an empty set, which would be a page asserting that it has no structure.
    // The difference is what a caller decides whether to prune on.
    let plain = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] >>",
        "<< /Type /StructElem /S /P /P 4 0 R >>",
    ]);
    let bare = Tree::of(&plain).expect("a structure tree root");
    let page = crate::page::Pages::new(&plain).get(0).expect("page one");
    assert_eq!(bare.elements_on_page(&plain, &page.dict), None);
}

/// §14.7.5.4's array written as an object of its own, which §7.3.10 permits of any value.
///
/// > Any object in a PDF file may be labelled as an indirect object.
///
/// The clause's own EXAMPLE 2 spells one `/Nums` both ways — `6 [1 0 R]` beside `0 101 0 R` —
/// and this is the second spelling: the page's key resolves to a reference, and the array is
/// behind it. Matching `Object::Array` on what the tree *states* reads such a page as having
/// said nothing, which is what 75 of the 109 corpus documents with a structure tree were read
/// as until ADR 1151.
#[test]
fn the_parent_trees_array_may_be_an_object_of_its_own() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /ParentTree 7 0 R >>",
        "<< /Type /StructElem /S /Sect /P 4 0 R /Pg 3 0 R /K [6 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K [0] >>",
        "<< /Nums [0 8 0 R] >>",
        "[6 0 R]",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let page = crate::page::Pages::new(&doc).get(0).expect("page one");

    let reference = |number| pdf_syntax::ObjectId {
        number,
        generation: 0,
    };
    assert_eq!(
        tree.elements_on_page(&doc, &page.dict),
        Some([reference(6)].into_iter().collect::<BTreeSet<_>>()),
        "the array is the entry's value however the file spelled it"
    );
}

/// §14.8.4.7.2 Table 368's `shall`, when the file has broken §14.7.5.4 Table 359's.
///
/// The widget here states no `/StructParent`, which Table 359 requires "for all objects that
/// are structural content items", so the parent tree cannot name its element and the array
/// for the page does not hold it. §14.7.5.3's direction survives — "the object shall be
/// identified in the structure element's K entry by an object reference dictionary" — and
/// that is what [`Tree::unkeyed_widget_owners`] asks, two levels below anything the parent
/// tree named. Table 368 is why it is asked for a widget and not for every annotation, and
/// that clause's own words are quoted there.
///
/// The `Link` beside it is the control: its annotation is silent in exactly the same way, and
/// Table 368 asks only that a `Link` element *mean* an association rather than that one exist,
/// so nothing looks for it and the element that names it stays out of the answer.
#[test]
fn a_widget_without_table_359s_key_is_found_through_its_elements_object_reference() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 \
         /Annots [11 0 R 13 0 R] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /ParentTree 10 0 R >>",
        "<< /Type /StructElem /S /Sect /P 4 0 R /Pg 3 0 R /K [6 0 R 7 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K [0] >>",
        "<< /Type /StructElem /S /Div /P 5 0 R /K [8 0 R 9 0 R] >>",
        "<< /Type /StructElem /S /Form /P 7 0 R \
         /K [<< /Type /OBJR /Obj 11 0 R /Pg 3 0 R >>] >>",
        "<< /Type /StructElem /S /Link /P 7 0 R \
         /K [<< /Type /OBJR /Obj 13 0 R /Pg 3 0 R >>] >>",
        "<< /Nums [0 12 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [0 0 1 1] >>",
        "[6 0 R]",
        "<< /Type /Annot /Subtype /Link /Rect [1 1 2 2] >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let page = crate::page::Pages::new(&doc).get(0).expect("page one");

    let reference = |number| pdf_syntax::ObjectId {
        number,
        generation: 0,
    };
    let found = tree
        .elements_on_page(&doc, &page.dict)
        .expect("the page states `/StructParents`");
    assert_eq!(
        found,
        [reference(6), reference(8)]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        "the sequence's element, and the `Form` the parent tree could not name"
    );
    // And the ancestry reaches it, which is what a walk coming down from the root needs: the
    // `Div` between them is named by nobody and is on the only path to the `Form`.
    assert!(tree.ancestry(&doc, &found).contains(&reference(7)));

    let owners = tree.unkeyed_widget_owners(&doc, &page.dict);
    assert_eq!(owners.items, vec![reference(8)]);
    assert!(
        !owners.truncated,
        "a tree of five elements is inside the bound"
    );
}

/// And nothing is searched for where every widget states the key Table 359 requires.
///
/// The cost of running §14.7.5.3's association backwards is a search of the tree, which is
/// what ADR 0325 took off a page turn, so the page that pays it has to be one whose file has
/// already contradicted itself. This is the same fixture with the key restored.
#[test]
fn a_widget_that_states_the_key_is_looked_up_and_not_searched_for() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 \
         /Annots [8 0 R] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /ParentTree 7 0 R >>",
        "<< /Type /StructElem /S /Sect /P 4 0 R /Pg 3 0 R /K [6 0 R] >>",
        "<< /Type /StructElem /S /Form /P 5 0 R \
         /K [<< /Type /OBJR /Obj 8 0 R /Pg 3 0 R >>] >>",
        "<< /Nums [0 9 0 R 1 6 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [0 0 1 1] /StructParent 1 >>",
        "[]",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let page = crate::page::Pages::new(&doc).get(0).expect("page one");

    let reference = |number| pdf_syntax::ObjectId {
        number,
        generation: 0,
    };
    assert!(
        tree.unkeyed_widget_owners(&doc, &page.dict)
            .items
            .is_empty(),
        "every widget on the page states its key, so there is nothing to search for"
    );
    assert_eq!(
        tree.elements_on_page(&doc, &page.dict),
        Some([reference(6)].into_iter().collect::<BTreeSet<_>>()),
        "and the element arrives by the lookup Table 359 exists for"
    );
}

/// §14.7.2's tree, walked from the root, with §14.7.3's role map applied.
///
/// The fixture is the shape §14.7.5.1.1 describes: an element whose children are another
/// element and the three forms a content item takes — a bare integer, a marked-content
/// reference and an object reference. Its own type is a name the document invented, which
/// the role map takes to a standard one *transitively*, because a role map may name a
/// type that is itself mapped.
#[test]
fn the_structure_tree_reads_its_children_and_maps_their_roles() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K 5 0 R \
         /RoleMap << /Heading2 /MyHeading /MyHeading /H2 >> >>",
        "<< /Type /StructElem /S /Heading2 /P 4 0 R /Pg 3 0 R /K [0 6 0 R 7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /Span /P 5 0 R >>",
        // Table 357's `/Stm`, the entry that says this sequence is *not* in the page's own
        // content stream — "[t]his entry should be present only if the marked-content
        // sequence resides in a content stream other than the content stream for the page".
        "<< /Type /MCR /Pg 3 0 R /Stm 10 0 R /MCID 4 >>",
        "<< /Type /OBJR /Obj 9 0 R >>",
        "<< /Type /Annot /Subtype /Link /Rect [0 0 1 1] >>",
        "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /StructParents 1 >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let page = pdf_syntax::ObjectId {
        number: 3,
        generation: 0,
    };

    let top = tree.children(&doc, None);
    assert_eq!(top.len(), 1, "one child of the root");
    let Some(Child::Element(heading)) = top.first() else {
        panic!("the root's child is an element: {top:?}");
    };
    assert_eq!(
        tree.role(&doc, heading).as_deref(),
        Some("H2"),
        "/Heading2 maps to /MyHeading maps to /H2"
    );

    let kids = tree.children(&doc, Some(heading));
    assert_eq!(
        kids.len(),
        4,
        "an element and three content items: {kids:?}"
    );
    assert_eq!(
        kids.first(),
        Some(&Child::MarkedContent {
            mcid: 0,
            page: Some(page),
            stream: None,
            owner: None
        }),
        "an integer takes its page from the element's /Pg, and names the page's own stream"
    );
    assert!(matches!(kids.get(1), Some(Child::Element(_))));
    assert_eq!(
        kids.get(2),
        Some(&Child::MarkedContent {
            mcid: 4,
            page: Some(page),
            stream: Some(pdf_syntax::ObjectId {
                number: 10,
                generation: 0
            }),
            owner: None
        }),
        "Table 357's /Stm names the stream the sequence is in, and /MCID 4 there is not \
         /MCID 4 in the page's own content stream"
    );
    assert_eq!(
        kids.get(3),
        Some(&Child::Object {
            object: pdf_syntax::ObjectId {
                number: 9,
                generation: 0
            },
            page: Some(page)
        }),
        "an object reference inherits the element's page where it states none"
    );

    // The walk is the same tree in the order `/K` states it, one level deeper for the
    // nested element's own children.
    let walked = tree.walk(&doc).items;
    assert_eq!(
        walked.len(),
        5,
        "the heading and its four children: {walked:?}"
    );
    assert!(matches!(walked.first(), Some((0, Child::Element(_)))));
    assert!(matches!(
        walked.get(1),
        Some((1, Child::MarkedContent { mcid: 0, .. }))
    ));
    assert!(
        matches!(walked.get(2), Some((1, Child::Element(_)))),
        "the nested element is a child at depth 1 and has none of its own"
    );
}

/// §14.9.2's language and §14.9.3's Unicode text are read and answered where a phoneme would
/// be spoken; the pronunciation hint itself is not processed, on the permission §14.9.6 states
/// outright:
///
/// > A PDF processor is not required to process pronunciation hints.
///
/// Table 355 (§14.7.2) defines `/PhoneticAlphabet` and `/Phoneme`; §14.9.6 is where the
/// decision not to process them lives, on the NOTE that a text-to-speech function does well
/// enough from Unicode text and a language. No corpus document states `/Phoneme` — it is a
/// PDF 2.0 entry and the pdf.js corpus predates PDF 2.0 — so this fixture is the calibration
/// for that silence (trap 13): the element answers its `/Lang` and its `/Alt`, and the
/// phoneme string is none of those answers. A reader that began consulting the hint would
/// return it here, and the last assertion would fail.
#[test]
fn a_phoneme_is_read_past_and_the_language_and_unicode_text_answered_in_its_place() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K 5 0 R >>",
        "<< /Type /StructElem /S /P /Lang (fr-FR) /Alt (tomato) \
         /PhoneticAlphabet /ipa /Phoneme (tuh-MAY-toh) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let top = tree.children(&doc, None);
    let Some(Child::Element(paragraph)) = top.first() else {
        panic!("the root's child is an element: {top:?}");
    };

    assert_eq!(tree.role(&doc, paragraph).as_deref(), Some("P"));
    assert_eq!(
        language(&doc, paragraph).as_deref(),
        Some("fr-FR"),
        "§14.9.2's language is read"
    );
    assert_eq!(
        alternate_description(&doc, paragraph).as_deref(),
        Some("tomato"),
        "§14.9.3's Unicode text is answered where a phoneme would be spoken"
    );
    let phoneme = "tuh-MAY-toh";
    assert!(
        [
            language(&doc, paragraph),
            alternate_description(&doc, paragraph),
            actual_text(&doc, paragraph),
        ]
        .into_iter()
        .flatten()
        .all(|answer| answer != phoneme),
        "the pronunciation hint is read past, never answered (§14.9.6's permission)"
    );
}

/// §14.8.4.7.3's link element, in the shape the clause's own EXAMPLE 1 prints.
///
/// Errata Collection 3 Issue #133 gave this number to a clause of its own, and moved the
/// heading the ledger carried under it — Ruby and warichu elements — one number along, to
/// `14.8.4.7.4`. ISO 32000-2 §14.8.4.7.3's two `shall`s meet in one fixture:
///
/// > When a Link structure element describes a span of text to be associated with a link
/// > annotation and that span wraps from the end of one line to the beginning of another,
/// > the Link structure element shall include a single object reference that associates the
/// > span with the associated link annotation. Further, the link annotation shall use the
/// > QuadPoint entry to denote the active areas on the page.
///
/// So the element holds one marked-content item and *one* object reference, and the
/// annotation it names carries sixteen numbers — EXAMPLE 1's two quadrilaterals, one per
/// line. Both are activated and the gap between them is not, which is the reader's whole
/// share of the clause: the structure says which annotation, and §12.5.6.5's `/QuadPoints`
/// says where.
#[test]
fn a_link_element_names_one_annotation_whose_quad_points_are_the_active_areas() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /StructParents 0 \
         /Annots [7 0 R] >>",
        "<< /Type /StructTreeRoot /K 5 0 R >>",
        "<< /Type /StructElem /S /Link /P 4 0 R /Pg 3 0 R /K [0 6 0 R] >>",
        "<< /Type /OBJR /Obj 7 0 R >>",
        "<< /Type /Annot /Subtype /Link /Rect [10 10 190 90] \
         /QuadPoints [100 70 180 70 180 90 100 90  20 20 60 20 60 40 20 40] >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");

    let top = tree.children(&doc, None);
    let Some(Child::Element(link)) = top.first() else {
        panic!("the root's child is the link element: {top:?}");
    };
    assert_eq!(
        tree.role(&doc, link).as_deref(),
        Some("Link"),
        "the element's own type is the standard one"
    );
    let annotation = pdf_syntax::ObjectId {
        number: 7,
        generation: 0,
    };
    let kids = tree.children(&doc, Some(link));
    assert_eq!(
        kids.iter()
            .filter(|child| matches!(child, Child::Object { object, .. } if *object == annotation))
            .count(),
        1,
        "a single object reference for the wrapped span: {kids:?}"
    );

    let pages = crate::page::Pages::new(&doc);
    let page = pages.get(0).expect("page one");
    let found = crate::link::links(&doc, &page);
    assert_eq!(found.len(), 1, "one annotation for the two lines");
    assert_eq!(
        found.first().map(|link| link.region.len()),
        Some(2),
        "sixteen numbers are two quadrilaterals"
    );
    assert!(
        crate::link::at(&found, 140.0, 80.0).is_some(),
        "the end of the first line"
    );
    assert!(
        crate::link::at(&found, 40.0, 30.0).is_some(),
        "the beginning of the second"
    );
    assert!(
        crate::link::at(&found, 40.0, 80.0).is_none(),
        "and the part of the first line the span does not reach is not active, \
         which is what makes the quadrilaterals the active areas rather than the rectangle"
    );
}

/// A `/K` cycle terminates, and an untagged document has no tree at all.
#[test]
fn a_structure_tree_that_points_at_itself_terminates() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R >>",
        "<< /Type /StructElem /S /Sect /K [6 0 R] >>",
        "<< /Type /StructElem /S /Sect /K [5 0 R] >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert_eq!(
        tree.walk(&doc).items.len(),
        3,
        "each element is entered once"
    );

    let untagged = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
    ]);
    assert!(Tree::of(&untagged).is_none());
}

/// §14.7.6's attributes, through both routes, with both precedence rules.
///
/// The fixture attaches three attribute objects to one element: a class from the
/// `/ClassMap` and two in the element's own `/A`, all three naming `/BackgroundColor`. The
/// clause decides the winner twice over — "[i]f a given attribute is specified more than
/// once, the later (in array order) entry shall take precedence" within `/A`, and "[i]f
/// both the A and C entries are present and a given attribute is specified by both, the
/// one specified by the A entry shall take precedence" between the two. So the answer is
/// the *last* object of `/A`, and an attribute only the class states is still found.
#[test]
fn an_attribute_is_found_through_a_class_and_overridden_by_the_elements_own() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R /ClassMap << /Warm 8 0 R >> >>",
        "<< /Type /StructElem /S /P /C [/Warm 3] /A [6 0 R 7 0 R 4] >>",
        "<< /O /Layout /BackgroundColor [0 0 1] >>",
        "<< /O /Layout /BackgroundColor [0 1 0] /TextAlign /Center >>",
        "<< /O /Layout /BackgroundColor [1 0 0] /Placement /Block >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element under the root");
    };

    let attached = tree.attributes(&doc, &element);
    assert_eq!(
        attached.len(),
        3,
        "one class and two of its own: {attached:?}"
    );
    assert_eq!(
        attached.iter().map(|a| a.revision).collect::<Vec<_>>(),
        vec![3, 0, 4],
        "§14.7.6.3: the integer after an entry is its revision, and an entry without \
         one is 0"
    );

    let colour = tree.attribute(&doc, &element, "BackgroundColor");
    assert_eq!(
        colour
            .as_ref()
            .and_then(Object::as_array)
            .map(<[Object]>::to_vec),
        Some(vec![
            Object::Integer(0),
            Object::Integer(1),
            Object::Integer(0)
        ]),
        "the later of the two /A objects wins, and /A beats /C"
    );
    assert!(
        tree.attribute(&doc, &element, "Placement").is_some(),
        "an attribute only the class states is still attached"
    );
    assert!(
        tree.attribute(&doc, &element, "O").is_none(),
        "/O identifies the owner and is not an attribute"
    );
}

/// §14.7.6.2's precedence *within* the class route, which the published clause never stated.
///
/// The 2020 text ranks `/A`'s objects among themselves and `/A` against `/C`, and says
/// nothing about two class objects disagreeing. Errata Collection 3's Issue #289 closes the
/// gap with two sentences inserted after the attach sentence: *Attribute objects included
/// through a class and through an array of classes within the C entry may have the value of
/// O and NS repeated. If a given attribute is specified more than once across the attribute
/// objects, the later (in array order) shall take precedence.* [`Tree::attributes`] already
/// answers both — it walks `/C` in array order and each class's objects in theirs, and
/// [`Tree::attribute`] takes the last match — but until this test the only evidence was a
/// fixture with one class object, which no ordering of the class route can fail.
///
/// The fixture attaches three objects through `/C` alone: one class naming a single object,
/// then a class naming an array of two, all three repeating `/O /Layout` as the erratum's
/// first sentence permits and all three stating `/BackgroundColor`. Calibrated per trap 13:
/// a plant that walks the `/C` classes in reverse order passes the single-class test above
/// and fails every assertion here except `/Placement`'s.
#[test]
fn an_attribute_two_class_objects_state_goes_to_the_later_one() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R /ClassMap << /Cold 6 0 R /Warm [7 0 R 8 0 R] >> >>",
        "<< /Type /StructElem /S /P /C [/Cold /Warm] >>",
        "<< /O /Layout /BackgroundColor [1 0 0] >>",
        "<< /O /Layout /BackgroundColor [0 0 1] /Placement /Block >>",
        "<< /O /Layout /BackgroundColor [0 1 0] >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element under the root");
    };

    let attached = tree.attributes(&doc, &element);
    assert_eq!(
        attached.len(),
        3,
        "one object from /Cold and two from /Warm: {attached:?}"
    );

    let colour = tree.attribute(&doc, &element, "BackgroundColor");
    assert_eq!(
        colour
            .as_ref()
            .and_then(Object::as_array)
            .map(<[Object]>::to_vec),
        Some(vec![
            Object::Integer(0),
            Object::Integer(1),
            Object::Integer(0)
        ]),
        "the later class wins over the earlier, and within one class's array the later \
         object wins"
    );
    assert!(
        tree.attribute(&doc, &element, "Placement").is_some(),
        "an attribute only one class object states is still attached"
    );
}

/// §14.7.6.4's user properties, read from the clause's own example.
///
/// The EXAMPLE at the end of §14.7.6.4 attaches four properties to a `Figure` — a part
/// name, a part number, a supplier marked `/H true`, and a price of -37.99 formatted as
/// `($37.99)`. Three of them are here, which are the three that state something a reader
/// has to decide about: a plain value, a hidden one, and one whose formatted form differs
/// from its value. That last is why the raw `/V` is kept beside `/F`.
#[test]
fn user_properties_are_read_with_their_formatting_and_their_hidden_flag() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R >>",
        "<< /Type /StructElem /S /Figure /A 6 0 R >>",
        "<< /O /UserProperties /P [ << /N (Part Name) /V (Framostat) >> \
         << /N (Supplier) /V (Just Framostats) /H true >> \
         << /N (Price) /V -37.99 /F ($37.99) >> ] >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element under the root");
    };
    let attached = tree.attributes(&doc, &element);
    let [object] = attached.as_slice() else {
        panic!("one attribute object: {attached:?}");
    };
    assert_eq!(object.owner, "UserProperties");

    let properties = object.user_properties(&doc);
    assert_eq!(properties.len(), 3);
    assert_eq!(
        properties.first().map(|p| p.name.as_str()),
        Some("Part Name")
    );
    assert_eq!(
        properties.get(1).map(|p| p.hidden),
        Some(true),
        "/H true hides the property from a panel"
    );
    assert_eq!(
        properties.get(2).and_then(|p| p.formatted.clone()),
        Some("$37.99".to_owned())
    );
    assert!(
        tree.attributes(&doc, &element)
            .iter()
            .all(|object| object.get(&doc, "P").is_some()),
        "the /P array is reachable as the attribute it is"
    );
}

/// §14.8.6.2: an element in an explicit namespace is mapped by *that* namespace.
///
/// > if the structure element does not explicitly identify its namespace using an NS
/// > entry, it should use the RoleMap entry in the Structure Tree Root dictionary … If the
/// > structure element is in an explicit namespace, then that namespace shall be identified
/// > in the structure tree root dictionary's Namespaces array entry and the RoleMapNS entry
/// > within that namespace dictionary shall provide the role mapping, if any.
///
/// The fixture states both maps and they disagree on purpose: the root's `/RoleMap` would
/// take `/Recipe` to `/Div`, and the element's own namespace takes it to `/Sect`. A reader
/// that consulted the root would pass a test where only one map existed.
#[test]
fn a_namespaces_own_role_map_is_the_one_that_applies() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R 7 0 R] /Namespaces [6 0 R] \
         /RoleMap << /Recipe /Div >> >>",
        "<< /Type /StructElem /S /Recipe /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://example.invalid/cookbook) \
         /RoleMapNS << /Recipe /Sect >> >>",
        "<< /Type /StructElem /S /Recipe >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let children = tree.children(&doc, None);
    let [Child::Element(in_namespace), Child::Element(plain)] = children.as_slice() else {
        panic!("two elements: {children:?}");
    };

    assert_eq!(tree.role(&doc, in_namespace).as_deref(), Some("Sect"));
    assert_eq!(
        tree.namespace(&doc, in_namespace).as_deref(),
        Some("http://example.invalid/cookbook")
    );
    assert_eq!(
        tree.role(&doc, plain).as_deref(),
        Some("Div"),
        "an element with no /NS is mapped by the root's /RoleMap"
    );
    assert_eq!(
        tree.namespace(&doc, plain).as_deref(),
        Some(super::DEFAULT_STANDARD_NAMESPACE),
        "§14.8.6.1: an element that names no namespace is in the default standard one"
    );
}

/// A `/RoleMapNS` may map *into another namespace*, and the walk follows it.
///
/// Table 356's second form: "an array where the first value shall be a structure element
/// type name in a target namespace with the second value being an indirect reference to
/// the target namespace dictionary". So one document's `/Ingredient` becomes another
/// namespace's `/ListItem`, which that namespace maps to the standard `/LI`.
#[test]
fn a_role_map_may_lead_into_another_namespace() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R /Namespaces [6 0 R 7 0 R] >>",
        "<< /Type /StructElem /S /Ingredient /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://example.invalid/cookbook) \
         /RoleMapNS << /Ingredient [/ListItem 7 0 R] >> >>",
        "<< /Type /Namespace /NS (http://example.invalid/lists) \
         /RoleMapNS << /ListItem /LI >> >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element");
    };
    assert_eq!(tree.role(&doc, &element).as_deref(), Some("LI"));
    assert_eq!(
        tree.standard_role(&doc, &element),
        Some(StandardType::ListItem),
        "the last step of the map leaves it in the default standard namespace"
    );
}

/// §14.8.6.2's other half: a name that ends in a namespace which is not a standard structure
/// one is that namespace's word, whatever §14.8.4 happens to call the same string.
#[test]
fn a_foreign_namespaces_homonym_is_not_a_standard_type() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R /Namespaces [6 0 R] >>",
        "<< /Type /StructElem /S /Table /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://example.invalid/furniture) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element");
    };
    assert_eq!(
        tree.role(&doc, &element).as_deref(),
        Some("Table"),
        "the name is what the document wrote"
    );
    assert_eq!(
        tree.standard_role(&doc, &element),
        None,
        "and it is not §14.8.4's table, because it is not in a standard structure namespace"
    );
}

/// §14.7.3 NOTE 2's two stops: a circular role map is followed until it returns to a type it
/// has met, and answers with the first standard type along the circle — so the NOTE's own
/// bidirectional map reads as `P` from either end — or, where the circle holds none, with the
/// last new name, said to be circular rather than undefined (ADR 2028).
///
/// > A PDF processor using the role map needs to follow the chain of associations until it
/// > either finds a structure type it recognises or returns to one it has already encountered.
#[test]
fn a_circular_role_map_stops_at_the_first_type_it_returns_to() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R] \
         /RoleMap << /MyPara /P /P /MyPara /Left /Right /Right /Left \
         /Chap /Sect /Sect /Div >> >>",
        "<< /Type /StructElem /S /MyPara >>",
        "<< /Type /StructElem /S /P >>",
        "<< /Type /StructElem /S /Left >>",
        "<< /Type /StructElem /S /Right >>",
        "<< /Type /StructElem /S /Chap >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let elements: Vec<Dictionary> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(element) => Some(element),
            _ => None,
        })
        .collect();
    let [my_para, p, left, right, chap] = elements.as_slice() else {
        panic!("five elements: {elements:?}");
    };
    for element in [my_para, p] {
        assert_eq!(
            tree.mapped_role(&doc, element),
            MappedRole {
                name: "P".to_owned(),
                standard: Ok(StandardType::Paragraph),
            },
            "the NOTE's bidirectional map reads as the standard type from either end"
        );
    }
    // No standard type on the circle: the element's own name mapped once, and the reason.
    assert_eq!(
        tree.mapped_role(&doc, left),
        MappedRole {
            name: "Right".to_owned(),
            standard: Err(Nonstandard::Circular),
        }
    );
    assert_eq!(tree.role(&doc, right).as_deref(), Some("Left"));
    // An acyclic chain is the normative sentence's: followed past a standard name the map
    // maps onward, to the name it does not map.
    assert_eq!(tree.standard_role(&doc, chap), Some(StandardType::Division));
}

/// [`Tree::mapped_role`]'s other two reasons: a foreign namespace's homonym, and a name the
/// default namespace does not define — the distinction a host is now told (ADR 2028).
#[test]
fn a_type_that_is_not_standard_says_why() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R 7 0 R 8 0 R] /Namespaces [6 0 R] >>",
        "<< /Type /StructElem /S /Table /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://example.invalid/furniture) >>",
        "<< /Type /StructElem /S /Advertising >>",
        "<< /Type /StructElem /K 0 >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let reasons: Vec<Result<StandardType, Nonstandard>> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(element) => Some(tree.mapped_role(&doc, &element).standard),
            _ => None,
        })
        .collect();
    assert_eq!(
        reasons,
        [
            Err(Nonstandard::Foreign),
            Err(Nonstandard::Undefined),
            Err(Nonstandard::Unstated),
        ]
    );
}

/// §14.8.6.1's default, applied to what a name means: an element stating no namespace is in
/// PDF 1.7's, so its `Note` is that namespace's type — and its `Em`, which the namespace does
/// not define, is read as the one standard namespace's that does (ADR 2014).
///
/// > When a namespace is not explicitly specified for a given structure element or attribute,
/// > it shall be assumed to be within this default standard structure namespace.
///
/// The fixture states each name three ways — no `/NS`, the PDF 1.7 namespace named, and the
/// PDF 2.0 one named — so the answer is shown to follow the namespace and not the name; and a
/// root `/RoleMap` takes a document's own `/Footnote` to `/Note`, which leaves it in the
/// default namespace as §14.8.6.2 says a single-name mapping does.
#[test]
fn a_name_is_read_in_the_vocabulary_of_the_namespace_it_ends_in() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R 10 0 R 11 0 R 12 0 R] \
         /Namespaces [13 0 R 14 0 R] /RoleMap << /Footnote /Note >> >>",
        "<< /Type /StructElem /S /Note >>",
        "<< /Type /StructElem /S /Note /NS 13 0 R >>",
        "<< /Type /StructElem /S /Note /NS 14 0 R >>",
        "<< /Type /StructElem /S /Em >>",
        "<< /Type /StructElem /S /Em /NS 13 0 R >>",
        "<< /Type /StructElem /S /Em /NS 14 0 R >>",
        "<< /Type /StructElem /S /Footnote >>",
        "<< /Type /StructElem /S /H7 >>",
        "<< /Type /Namespace /NS (http://iso.org/pdf/ssn) >>",
        "<< /Type /Namespace /NS (http://iso.org/pdf2/ssn) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let roles: Vec<Option<StandardType>> = tree
        .children(&doc, None)
        .iter()
        .map(|child| match child {
            Child::Element(element) => tree.standard_role(&doc, element),
            other => panic!("an element: {other:?}"),
        })
        .collect();
    assert_eq!(
        roles,
        [
            Some(StandardType::Note),
            Some(StandardType::Note),
            None,
            Some(StandardType::Emphasis),
            None,
            Some(StandardType::Emphasis),
            Some(StandardType::Note),
            Some(StandardType::Heading(7)),
        ],
        "an explicit namespace is held to, no namespace reads either, and the mapped Footnote \
         is the default's Note"
    );
    assert_eq!(
        StandardType::read("Note"),
        Some(StandardType::Note),
        "and a name read with no namespace beside it is either vocabulary's"
    );
    assert_eq!(StandardType::read("Em"), Some(StandardType::Emphasis));
}

/// Annex M's two lists, whole: eleven types only the PDF 1.7 namespace defines and nine only
/// the PDF 2.0 one does, counting `Hn` past six as one. The rest of either vocabulary is in
/// both, which is the annex's sentence between its two lists.
#[test]
fn annex_m_says_which_namespace_defines_which_type() {
    use super::StandardNamespace::{Pdf1_7, Pdf2_0};

    for name in [
        "Art",
        "BlockQuote",
        "TOC",
        "TOCI",
        "Index",
        "Private",
        "Quote",
        "Note",
        "Reference",
        "BibEntry",
        "Code",
    ] {
        assert!(
            StandardType::read_in(name, Pdf1_7).is_some(),
            "{name} is PDF 1.7's"
        );
        assert_eq!(
            StandardType::read_in(name, Pdf2_0),
            None,
            "{name} is not PDF 2.0's"
        );
    }
    for name in [
        "DocumentFragment",
        "Aside",
        "H7",
        "H17",
        "Title",
        "FENote",
        "Sub",
        "Em",
        "Strong",
        "Artifact",
    ] {
        assert!(
            StandardType::read_in(name, Pdf2_0).is_some(),
            "{name} is PDF 2.0's"
        );
        assert_eq!(
            StandardType::read_in(name, Pdf1_7),
            None,
            "{name} is not PDF 1.7's"
        );
    }
    for name in ["H6", "H", "P", "Sect", "Table", "Link", "Figure", "Formula"] {
        assert!(
            StandardType::read_in(name, Pdf1_7).is_some()
                && StandardType::read_in(name, Pdf2_0).is_some(),
            "{name} is in both"
        );
    }
}

/// The PDF 1.7 types end with the boundary ISO 32000-1's category gives them: the six of its
/// Table 333 are grouping elements and end a block, the five of its Table 338 are inline and
/// end nothing. Before §14.8.6.1's default was read, every one of them ended with nothing and
/// took its parent's category, which is the reading for a name no namespace defines (ADR 2014).
#[test]
fn a_pdf_1_7_type_ends_with_its_own_categorys_boundary() {
    for (name, ends_with) in [
        ("TOCI", Boundary::Block),
        ("TOC", Boundary::Block),
        ("Art", Boundary::Block),
        ("BlockQuote", Boundary::Block),
        ("Note", Boundary::Flow),
        ("Reference", Boundary::Flow),
        ("Code", Boundary::Flow),
    ] {
        let kind = StandardType::read(name).unwrap_or_else(|| panic!("{name} is standard"));
        assert_eq!(
            Boundary::ending(Some(&kind), Some(Category::Grouping)).0,
            ends_with,
            "{name}"
        );
    }
    assert_eq!(
        StandardType::read("Note").map(|kind| kind.category()),
        Some(Category::Inline)
    );
}

/// A structure tree that is its own descendant is counted once and terminates.
///
/// The walk this report added is a second descent through §14.7.2's tree, so it owes the same
/// two bounds the first one has and for the same reason (principle 3: a cycle in untrusted
/// input is a resource question rather than a memory-safety one). The element below is its own
/// child through `/K`, and it is in a foreign namespace, so a walk that did not remember what
/// it had seen would count for ever rather than counting one.
#[test]
fn an_element_that_is_its_own_child_is_counted_once() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [6 0 R] >>",
        "<< /Type /StructElem /S /Widget /NS 6 0 R /K [5 0 R] >>",
        "<< /Type /Namespace /NS (http://example.invalid/tagset) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert_eq!(
        tree.namespaces_outside_the_standard(&doc),
        vec![super::ForeignNamespace {
            name: Some("http://example.invalid/tagset".to_owned()),
            elements: 1,
        }]
    );
}

/// A `math` element with no `Formula` above it, which is §14.8.6.3's first `shall` broken.
///
/// The `shall` is Errata Collection 3's rather than the 2020 text's — Issues #72 and #719
/// replace the subclause's `MathML` sentence with one requiring the `math` type to be used to
/// enclose the formula under the `Formula` structure element type — so it is stated here in
/// prose and quoted in [`Tree::mathml_outside_a_formula`] beside the sentence it replaced.
///
/// The planted violation, and trap 13's positive half: every test below it says *nothing*, so
/// without this one the report could be a constant zero and all of them would pass.
#[test]
fn a_mathml_formula_outside_a_formula_element_is_counted() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [6 0 R] >>",
        "<< /Type /StructElem /S /math /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert_eq!(tree.mathml_outside_a_formula(&doc), 1);
}

/// The same file with a `Formula` around it says nothing, which is the clause satisfied.
///
/// It is also the floor under the cheap gate for this report: the root declares the `MathML`
/// namespace here too, so the walk runs and the answer is nought because of what it found
/// rather than because it never looked.
#[test]
fn a_mathml_formula_under_a_formula_element_is_not_counted() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [7 0 R] >>",
        "<< /Type /StructElem /S /Formula /K [6 0 R] >>",
        "<< /Type /StructElem /S /math /NS 7 0 R >>",
        "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert!(
        tree.declares_the_mathml_namespace(&doc),
        "the gate opens, so the nought below is what the walk found"
    );
    assert_eq!(tree.mathml_outside_a_formula(&doc), 0);
}

/// And a `Formula` further up satisfies it, because the sentence does not say *immediately*.
///
/// The narrow reading is the one that reports, so an element with a `Formula` anywhere above
/// it is outside this condition — it satisfies the clause on either reading of *under*, and
/// firing here would be firing on a word the standard did not write (trap 11).
#[test]
fn a_mathml_formula_below_a_formulas_descendant_is_not_counted() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [8 0 R] >>",
        "<< /Type /StructElem /S /Formula /K [6 0 R] >>",
        "<< /Type /StructElem /S /Div /K [7 0 R] >>",
        "<< /Type /StructElem /S /math /NS 8 0 R >>",
        "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert_eq!(tree.mathml_outside_a_formula(&doc), 0);
}

/// A `math` in some other vocabulary is not the type this subclause names.
///
/// §14.8.6.2 is the clause that makes a name mean what its namespace makes it mean, and this
/// is that rule applied to §14.8.6.3's own type: "the math structure element type, **as
/// defined in `MathML`**". The root declares `MathML` as well, so the gate is open and the walk
/// looked at this element and declined it.
#[test]
fn a_math_element_in_another_namespace_is_not_mathml() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [6 0 R 7 0 R] >>",
        "<< /Type /StructElem /S /math /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://example.invalid/algebra) >>",
        "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert_eq!(tree.mathml_outside_a_formula(&doc), 0);
}

/// And a `Formula` in some other vocabulary is not §14.8.4's, so it encloses nothing.
///
/// The mirror of the test above, on the other half of the sentence, and the reason the
/// enclosure test goes through `standard_role` rather than through the name: a homonym
/// standing where the clause names a standard type would silently acquit the file.
#[test]
fn a_formula_in_another_namespace_does_not_enclose_the_mathml() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /Formula /NS 7 0 R /K [6 0 R] >>",
        "<< /Type /StructElem /S /math /NS 8 0 R >>",
        "<< /Type /Namespace /NS (http://example.invalid/paper) >>",
        "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert_eq!(tree.mathml_outside_a_formula(&doc), 1);
}

/// `Math` is not `math`, because §7.3.5 makes a name's identity its bytes.
///
/// NOTE 2 of the subclause says the type "is all lowercase to match the `MathML` 3.0
/// specification", so a document writing it otherwise has written a different type — one no
/// vocabulary in reach defines — and this report says nothing about it rather than guessing.
#[test]
fn a_capitalised_math_is_a_different_type() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [6 0 R] >>",
        "<< /Type /StructElem /S /Math /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert_eq!(tree.mathml_outside_a_formula(&doc), 0);
}

/// The floor under [`Tree::namespaces_outside_the_standard`]'s cheap gate, and it is a real
/// document rather than a fixture.
///
/// `doc/traps/instruments-and-reports.md` trap 11's ninth instance: an exemption that costs
/// nothing to widen has no floor under it, and this report's exemption is the one that stops
/// the elements being read at all. `bug1937438_af_from_latex.pdf` declares four namespaces of
/// its own — two LaTeX tagsets and a `data:` URI — beside §14.8.6.1's two, so it is a
/// document on which the gate **opens** and the whole tree is walked; and the answer is empty,
/// because each of those namespaces carries the `/RoleMapNS` §14.8.6.2's third bullet asks
/// for. Without a case like this the gate could be inverted and every test above would still
/// pass.
#[test]
fn a_document_whose_own_namespaces_all_role_map_is_read_and_says_nothing() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/bug1937438_af_from_latex.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let doc = Document::open(bytes).expect("a valid file");
    let tree = Tree::of(&doc).expect("a structure tree root");
    assert!(
        tree.declares_a_namespace_outside_the_standard(&doc),
        "the root declares the LaTeX tagsets, so the elements are read"
    );
    assert_eq!(
        tree.namespaces_outside_the_standard(&doc),
        Vec::new(),
        "and every element ends in a standard structure namespace after its role map"
    );
}

/// The same element in the PDF 2.0 standard structure namespace, which is what makes the
/// test above a statement about the namespace rather than about the `/NS` entry.
#[test]
fn the_same_name_in_a_standard_namespace_is_the_standard_type() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R /Namespaces [6 0 R] >>",
        "<< /Type /StructElem /S /Table /NS 6 0 R >>",
        "<< /Type /Namespace /NS (http://iso.org/pdf2/ssn) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element");
    };
    assert_eq!(
        tree.standard_role(&doc, &element),
        Some(StandardType::Table)
    );
}

/// A page with no `/StructParents` has no structure, and that is not a failure.
#[test]
fn a_page_outside_the_structure_tree_reads_as_empty() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
    ]);
    let pages = crate::page::Pages::new(&doc);
    let page = pages.get(0).expect("page one");
    assert!(ParentTree::for_page(&doc, &page.dict).is_empty());
}

/// §14.8.4's vocabulary, including the family the table cannot enumerate.
///
/// `Hn` is written in Table 372 as `H n`, "with n being a sequence of digits representing an
/// unsigned integer greater than or equal to 1", so it is a *family* of names and not a name
/// — which is why it is read before the table and why `H17` is as standard as `H1`. `H0` is
/// not: the clause's own bound is 1.
#[test]
fn a_heading_carries_its_level_and_the_table_carries_the_rest() {
    use super::{Category, StandardType};

    assert_eq!(StandardType::read("H1"), Some(StandardType::Heading(1)));
    assert_eq!(StandardType::read("H17"), Some(StandardType::Heading(17)));
    assert_eq!(
        StandardType::read("H"),
        Some(StandardType::UnnumberedHeading)
    );
    assert_eq!(
        StandardType::read("H0"),
        None,
        "the clause's own bound is 1"
    );
    assert_eq!(StandardType::read("Hx"), None);
    assert_eq!(StandardType::read("Paragraph"), None, "not a standard name");

    assert_eq!(
        StandardType::read("TD").map(|kind| kind.category()),
        Some(Category::Internal),
        "a table cell is stated as internal to a table rather than as a level"
    );
    assert!(
        StandardType::read("Em").is_some_and(|kind| kind.since_pdf_2_0()),
        "emphasis is one of the eight types PDF 2.0 added"
    );
    assert!(
        StandardType::read("P").is_some_and(|kind| !kind.since_pdf_2_0()),
        "and a paragraph is not"
    );
}

/// ISO 32000-2 §14.8.4.7.4's Table 369, whole: two assemblies, seven types, all inline.
///
/// One test rather than seven because the table's own claim is a single one — every ruby
/// and warichu type is `Inline` — in one ledger row, since Errata Collection 3 gives the number
/// of a second one to §14.8.4.7.3's link element (ADR 0273).
#[test]
fn every_ruby_and_warichu_type_is_inline() {
    use super::{Category, StandardType};

    for name in ["Ruby", "RB", "RT", "RP", "Warichu", "WT", "WP"] {
        let kind = StandardType::read(name).unwrap_or_else(|| panic!("{name} is standard"));
        assert_eq!(
            kind.category().of(Some(Category::Block)),
            Category::Inline,
            "{name} is inline wherever it is used"
        );
        assert!(
            !kind.since_pdf_2_0(),
            "{name} predates PDF 2.0, which added eight types and none of these"
        );
    }
}

/// §14.8.4.1's rule for a type that is block *or* inline, applied where it is used.
///
/// > If the structure element is used inside a block level element, it is an inline level
/// > structure element … In all other cases it is a block level structure element.
#[test]
fn a_contextual_type_takes_its_category_from_its_parent() {
    use super::{Category, StandardType};

    let figure = StandardType::read("Figure").expect("a standard type");
    assert_eq!(figure.category(), Category::Contextual);
    assert_eq!(
        figure.category().of(Some(Category::Block)),
        Category::Inline,
        "inside a paragraph, a figure is inline"
    );
    assert_eq!(
        figure.category().of(Some(Category::Grouping)),
        Category::Block,
        "and anywhere else it is block level"
    );
    assert_eq!(figure.category().of(None), Category::Block);
    assert_eq!(
        StandardType::read("P")
            .expect("a paragraph")
            .category()
            .of(Some(Category::Block)),
        Category::Block,
        "a type with one category keeps it wherever it is used"
    );
}

/// The role map's answer is read as the standard type it names.
///
/// The point of the pair: a document's own `/Chapter` is not a standard type, and after
/// §14.7.3's mapping it *is* one. A consumer asking "is this a heading" gets an answer for
/// the mapped name and nothing for a name nobody mapped, which §14.8.4.1 makes a defect in
/// the document rather than a gap in the reader.
#[test]
fn a_mapped_role_reads_as_its_standard_type() {
    use super::StandardType;

    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R] /RoleMap << /Chapter /H1 >> >>",
        "<< /Type /StructElem /S /Chapter >>",
        "<< /Type /StructElem /S /Sidebar >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let children = tree.children(&doc, None);
    let [Child::Element(mapped), Child::Element(unmapped)] = children.as_slice() else {
        panic!("two elements: {children:?}");
    };
    assert_eq!(
        tree.standard_role(&doc, mapped),
        Some(StandardType::Heading(1))
    );
    assert_eq!(
        tree.standard_role(&doc, unmapped),
        None,
        "a name no role map takes to a standard type is not one"
    );
    assert_eq!(
        tree.role(&doc, unmapped).as_deref(),
        Some("Sidebar"),
        "and the name the document wrote is still available"
    );
}

/// §14.8.1's mark information dictionary, and the difference between having a tree and
/// claiming to be tagged.
///
/// "A tagged PDF document shall contain a mark information dictionary … with a value of true
/// for the Marked entry", so a document with a structure tree and no `/MarkInfo` has a tree
/// and has not made the claim — which is a real distinction and not a technicality: §14.8's
/// rules are what `Marked` asserts conformance to.
#[test]
fn a_document_says_it_is_tagged_in_its_mark_information_dictionary() {
    use super::MarkInfo;

    let tagged = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R \
         /MarkInfo << /Marked true /UserProperties true >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot >>",
    ]);
    let info = MarkInfo::read(&tagged);
    assert!(info.marked);
    assert!(info.user_properties);
    assert!(!info.suspects, "Table 353's default");

    let untagged = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot >>",
    ]);
    assert!(
        !MarkInfo::read(&untagged).marked,
        "a structure tree is not the claim; the entry is"
    );
    assert!(Tree::of(&untagged).is_some(), "and the tree is still there");
}

/// §14.8.5.3's priority: a format-specific owner is skipped and an inheritable value walks
/// up `/P`.
///
/// The fixture states `/TextAlign` three times over two elements: an `HTML-4.01` object on
/// the child, a `Layout` object on the *parent*, and a `Layout` object on the child. The
/// clause's first priority applies "if processing based on the format indicated by the owner
/// value" — this program processes to no format, so the `HTML-4.01` value is not the answer
/// even though it outranks `Layout` for a processor translating to HTML. The parent's value
/// is the fourth priority and only reachable through `inherited_attribute`.
#[test]
fn an_attributes_owner_and_its_ancestry_decide_which_value_applies() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R >>",
        "<< /Type /StructElem /S /Sect /A 8 0 R /K [6 0 R 7 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R /A [9 0 R 10 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R >>",
        "<< /O /Layout /TextAlign /Justify >>",
        "<< /O /HTML-4.01 /TextAlign /Center >>",
        "<< /O /Layout /TextAlign /Start >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(section)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element under the root");
    };
    let kids = tree.children(&doc, Some(&section));
    let [Child::Element(stated), Child::Element(bare)] = kids.as_slice() else {
        panic!("two paragraphs: {kids:?}");
    };

    assert_eq!(
        tree.attributes(&doc, stated)
            .iter()
            .map(|object| object.kind.clone())
            .collect::<Vec<_>>(),
        vec![Owner::Format("HTML-4.01".to_owned()), Owner::Layout]
    );
    assert_eq!(
        tree.attribute(&doc, stated, "TextAlign")
            .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec())),
        Some(b"Start".to_vec()),
        "the Layout value, because nothing here processes to HTML"
    );
    assert_eq!(
        tree.attribute(&doc, bare, "TextAlign"),
        None,
        "the parent's value is not this element's own"
    );
    assert_eq!(
        tree.inherited_attribute(&doc, bare, "TextAlign")
            .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec())),
        Some(b"Justify".to_vec()),
        "§14.8.5.3's fourth priority, up /P"
    );
    assert_eq!(
        tree.inherited_attribute(&doc, stated, "TextAlign")
            .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec())),
        Some(b"Start".to_vec()),
        "an element that states its own is not overridden by its parent's"
    );
}

/// §14.8.5.3's bands, where the clause's order and the array's disagree.
///
/// The fixture states `/TextAlign` twice on one element, in one `/A` array, and puts the
/// object the clause ranks **higher** *first* — so a reader that took the last value written
/// and a reader that took the highest band give different answers, which is what makes this
/// a calibration rather than a restatement (`doc/traps/instruments-and-reports.md` trap 13).
///
/// Three elements, one per case §14.7.4.2's closing paragraph produces for an `NSO` owner:
///
/// - **the PDF 2.0 standard structure namespace.** §14.8.6.1 defines it as the schema for
///   §14.8.4 and §14.8.5 themselves, so it is not one Table 376 owner value and falls where
///   §14.8.5.3 puts an owner that is none of the five — band 1, above the `Layout` object
///   written after it. `Center` wins (ADR 1077).
/// - **the namespace name `Layout`.** This is the one equivalence the clause states:
///   "[f]or common namespace names which correspond to the values of owner entries defined
///   in Table 376 … they shall be considered equivalent". So the object is a `Layout` object,
///   band 2, and the `Layout` object written *after* it in the array wins on §14.7.6's rule.
///   `Start` — the same shape as the case above with the opposite answer, and the only
///   difference between them is the clause's equivalence.
/// - **`MathML`'s namespace.** No Table 376 value corresponds to it and it is not a standard
///   structure namespace, so it is in `HTML-4.01`'s position: an owner this program does not
///   process, refused for every attribute and named by [`Tree::unranked_owners`].
#[test]
fn a_namespace_owner_ranks_where_the_clause_puts_the_owner_it_names() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R >>",
        "<< /Type /StructElem /S /Sect /K [6 0 R 7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R /A [9 0 R 12 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R /A [10 0 R 12 0 R] >>",
        "<< /Type /StructElem /S /P /P 5 0 R /A [11 0 R 12 0 R] >>",
        "<< /O /NSO /NS 13 0 R /TextAlign /Center >>",
        "<< /O /NSO /NS 14 0 R /TextAlign /Center >>",
        "<< /O /NSO /NS 15 0 R /TextAlign /Center >>",
        "<< /O /Layout /TextAlign /Start >>",
        "<< /Type /Namespace /NS (http://iso.org/pdf2/ssn) >>",
        "<< /Type /Namespace /NS (Layout) >>",
        "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(section)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element under the root");
    };
    let kids = tree.children(&doc, Some(&section));
    let [
        Child::Element(standard),
        Child::Element(equivalent),
        Child::Element(foreign),
    ] = kids.as_slice()
    else {
        panic!("three paragraphs: {kids:?}");
    };
    let align = |element: &Dictionary| {
        tree.attribute(&doc, element, "TextAlign")
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec()))
    };

    assert_eq!(
        tree.attributes(&doc, standard)[0].priority(),
        Some(Priority::Owner),
        "a standard structure namespace is not one of the five owners §14.8.5.3 excludes"
    );
    assert_eq!(
        align(standard),
        Some(b"Center".to_vec()),
        "band 1 outranks the Layout object written after it"
    );

    assert_eq!(
        tree.attributes(&doc, equivalent)[0].effective_owner(),
        Some(Owner::Layout),
        "§14.7.4.2's equivalence, which is the one correspondence the clause states"
    );
    assert_eq!(
        align(equivalent),
        Some(b"Start".to_vec()),
        "two Layout objects in one band, and §14.7.6 gives the later one"
    );

    assert_eq!(
        tree.attributes(&doc, foreign)[0].priority(),
        None,
        "MathML is a vocabulary this program does not process"
    );
    assert_eq!(
        align(foreign),
        Some(b"Start".to_vec()),
        "the refused object states the attribute and is consulted for nothing"
    );
    assert_eq!(
        tree.unranked_owners(&doc, foreign),
        vec!["http://www.w3.org/1998/Math/MathML".to_owned()],
        "and the refusal names the owner rather than dropping it in silence"
    );
    assert!(
        tree.unranked_owners(&doc, standard).is_empty(),
        "which the two ranked elements do not do"
    );
}

/// §14.8.5.3 band 3: the class map loses to the element's own `/A`, and keeps what `/A` omits.
///
/// The class states `/Placement` and `/TextAlign`; the `/A` object states `/Placement` alone.
/// The clause's third priority is reached only for an attribute the second one did not state,
/// so `/Placement` is the `/A` object's and `/TextAlign` is the class's — one element, two
/// bands, and the band is per *attribute* rather than per element.
///
/// Calibrated per trap 13: a plant that gives a class-map object band 1 instead of band 3
/// fails this and the two-route test above, which is what says the route decides the band.
#[test]
fn a_class_map_answers_only_what_the_elements_own_attributes_did_not() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K 5 0 R /ClassMap << /Boxed 7 0 R >> >>",
        "<< /Type /StructElem /S /P /C /Boxed /A 6 0 R >>",
        "<< /O /Layout /Placement /Inline >>",
        "<< /O /Layout /Placement /Block /TextAlign /Justify >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree root");
    let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
        panic!("one element under the root");
    };
    let bands = tree
        .attributes(&doc, &element)
        .iter()
        .map(AttributeObject::priority)
        .collect::<Vec<_>>();
    assert_eq!(
        bands,
        vec![Some(Priority::Class), Some(Priority::Standard)],
        "the class map first, the element's own /A after it"
    );
    assert_eq!(
        tree.attribute(&doc, &element, "Placement")
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec())),
        Some(b"Inline".to_vec()),
        "priority 2 beats priority 3"
    );
    assert_eq!(
        tree.attribute(&doc, &element, "TextAlign")
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec())),
        Some(b"Justify".to_vec()),
        "and priority 3 is what answers an attribute priority 2 did not state"
    );
}

/// §14.8.5.6's `PrintField`: what a form field was, after the field itself was flattened.
///
/// Three elements, one per way Table 383 can be written: a check box that is ticked, a
/// radio button using the **deprecated lower-case** `/checked`, and a text-value field with
/// only a `/Desc`. The last one also pins the table's default — an element that states no
/// state is `off`, which is what an unticked box on a printed form looks like.
#[test]
fn a_flattened_form_field_says_what_it_was() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /Form /Pg 3 0 R              /A << /O /PrintField /Role /cb /Checked /on /Desc (Agree to the terms) >> >>",
        "<< /Type /StructElem /S /Form /Pg 3 0 R              /A << /O /PrintField /Role /rb /checked /neutral >> >>",
        "<< /Type /StructElem /S /Form /Pg 3 0 R              /A << /O /PrintField /Role /tv /Desc (Surname) >> >>",
        "<< /Type /StructElem /S /P /Pg 3 0 R >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();

    let box_ticked = tree
        .print_field(&doc, &elements[0])
        .expect("a check box's attributes");
    assert_eq!(box_ticked.role, Some(FieldRole::CheckBox));
    assert_eq!(box_ticked.checked, Checked::On);
    assert_eq!(
        box_ticked.description.as_deref(),
        Some("Agree to the terms")
    );

    let button = tree
        .print_field(&doc, &elements[1])
        .expect("a radio button's attributes");
    assert_eq!(button.role, Some(FieldRole::RadioButton));
    assert_eq!(
        button.checked,
        Checked::Neutral,
        "the lower-case /checked is deprecated in PDF 2.0 and still read"
    );

    let text = tree
        .print_field(&doc, &elements[2])
        .expect("a text-value field's attributes");
    assert_eq!(text.role, Some(FieldRole::TextValue));
    assert_eq!(
        text.checked,
        Checked::Off,
        "Table 383's default for a field that states no state"
    );

    assert!(
        tree.print_field(&doc, &elements[3]).is_none(),
        "an element with no PrintField attributes has none"
    );
}

/// §14.8.5.5's two entries, and the list each continuing list turns out to continue.
///
/// Seven lists and a paragraph, one per shape Table 382 can be written in and one per way the
/// resolution can go:
///
/// - a plain list that states an `/ID`, which is what the others name and precede;
/// - a list naming its predecessor with `/ContinuedFrom`;
/// - a list stating `/ContinuedList false` **and** a `/ContinuedFrom`, where the entry whose
///   job is to say `no` wins;
/// - a list stating `/ContinuedList true` and nothing else, which takes the clause's own
///   fallback — "the preceding list at the same level in the structure hierarchy" — and gets
///   the list above, which is one that continues nothing itself;
/// - a list **nested inside** that one, stating nothing: Errata Collection 3's Issue #346
///   makes both entries *not inheritable*, so it continues nothing even though its parent
///   does, and its `/P` is stated so that inheriting would have somewhere to go;
/// - a second list taking the fallback, placed so that the nested list is the nearest list in
///   the walk and the answer must be the one at its own **level** instead;
/// - and a bare `/ContinuedFrom` with no flag — the choice [`Tree::list_continuation`]
///   documents — naming an identifier no list states, so it continues something this answer
///   cannot point at.
#[test]
fn a_list_says_which_earlier_list_it_carries_on() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 8 0 R 9 0 R 7 0 R 10 0 R 12 0 R] >>",
        "<< /Type /StructElem /S /L /Pg 3 0 R /ID (first) >>",
        "<< /Type /StructElem /S /P /Pg 3 0 R >>",
        "<< /Type /StructElem /S /L /Pg 3 0 R /K [11 0 R] \
          /A << /O /List /ContinuedList true >> >>",
        "<< /Type /StructElem /S /L /Pg 3 0 R \
          /A << /O /List /ContinuedList true /ContinuedFrom (first) >> >>",
        "<< /Type /StructElem /S /L /Pg 3 0 R \
          /A << /O /List /ContinuedList false /ContinuedFrom (first) >> >>",
        "<< /Type /StructElem /S /L /Pg 3 0 R /A << /O /List /ContinuedList true >> >>",
        "<< /Type /StructElem /S /L /Pg 3 0 R /P 7 0 R >>",
        "<< /Type /StructElem /S /L /Pg 3 0 R /A << /O /List /ContinuedFrom (nowhere) >> >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    // Only the `L` elements go in, which is what `list_predecessors` documents its population
    // as: the paragraph at position 1 is not a list, and offering it would make it the
    // "preceding list" the fallback below picks.
    let mut lists: Vec<ListEntry> = Vec::new();
    let mut stated: Vec<Option<ListContinuation>> = Vec::new();
    for (position, (depth, child)) in tree.walk(&doc).items.into_iter().enumerate() {
        let Child::Element(dict) = child else {
            continue;
        };
        if tree.standard_role(&doc, &dict) != Some(StandardType::List) {
            stated.push(None);
            continue;
        }
        let continuation = tree.list_continuation(&doc, &dict);
        stated.push(continuation.clone());
        lists.push(ListEntry {
            position,
            depth,
            id: document_id(&doc, &dict),
            continuation,
        });
    }
    assert_eq!(
        stated,
        vec![
            None,
            None,
            Some(ListContinuation::From(b"first".to_vec())),
            None,
            Some(ListContinuation::Preceding),
            None,
            Some(ListContinuation::Preceding),
            Some(ListContinuation::From(b"nowhere".to_vec())),
        ],
        "what each element states, in walk order"
    );

    let predecessors = list_predecessors(&lists);
    assert_eq!(
        predecessors.get(&2).copied(),
        Some(0),
        "/ContinuedFrom names the list that states that identifier"
    );
    assert_eq!(
        predecessors.get(&4).copied(),
        Some(3),
        "the fallback takes the preceding list, which is a list that continues nothing itself"
    );
    assert_eq!(
        predecessors.get(&6).copied(),
        Some(4),
        "the preceding list at depth 0, not the nested one at position 5"
    );
    assert_eq!(
        predecessors.get(&7).copied(),
        None,
        "an identifier no list states resolves to nothing"
    );
    assert!(
        !predecessors.contains_key(&3) && !predecessors.contains_key(&5),
        "a list that continues nothing is given no predecessor: {predecessors:?}"
    );
}

/// Table 355's `/ID`, as [`ListEntry`] wants it.
fn document_id(document: &Document, element: &Dictionary) -> Option<Vec<u8>> {
    document
        .get_key(element, "ID")
        .as_string()
        .map(<[u8]>::to_vec)
        .filter(|id| !id.is_empty())
}

/// Table 384's `/Scope` and the two spans, read off the cells that state them.
///
/// The `/Scope` a document states wins outright; a name outside the table's three is not one
/// of them and is read as nothing rather than as the nearest.
#[test]
fn a_header_cell_states_its_axis_and_a_cell_states_its_spans() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /TH /Pg 3 0 R /A << /O /Table /Scope /Row >> >>",
        "<< /Type /StructElem /S /TH /Pg 3 0 R /A << /O /Table /Scope /Sideways >> >>",
        "<< /Type /StructElem /S /TD /Pg 3 0 R /A << /O /Table /RowSpan 2 /ColSpan 3 >> >>",
        "<< /Type /StructElem /S /TD /Pg 3 0 R /A << /O /Table /ColSpan 0 >> >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();

    assert_eq!(
        tree.header_scope(&doc, &elements[0]),
        Some(HeaderScope::Row)
    );
    assert_eq!(
        tree.header_scope(&doc, &elements[1]),
        None,
        "a name Table 384 does not define is not one of its three"
    );
    assert_eq!(tree.cell_span(&doc, &elements[2]), (2, 3));
    assert_eq!(
        tree.cell_span(&doc, &elements[3]),
        (1, 1),
        "Table 384's default is 1, and a span of zero is not a number of columns"
    );
}

/// Table 384's `/Summary` and `/Short`, read off the elements that state them.
///
/// The second table's summary arrives through §14.7.6.2's class map, because the entry is an
/// attribute like any other and both of §14.7.6's routes carry it. The third states a name
/// where the entry is a text string, which is a value §14.8.5.7 does not define, and the
/// fourth an empty string, which has nothing to speak; both are read as nothing. And the
/// cell *inside* the summarised table answers nothing, because the entry is not
/// inheritable — a reader that inherited it would describe every cell with its table's
/// sentence.
#[test]
fn a_tables_summary_and_a_headers_short_form_are_read() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R] \
           /ClassMap << /Named << /O /Table /Summary (from the class map) >> >> >>",
        "<< /Type /StructElem /S /Table /Pg 3 0 R /K [10 0 R] \
           /A << /O /Table /Summary (sales by region and quarter) >> >>",
        "<< /Type /StructElem /S /Table /Pg 3 0 R /C /Named >>",
        "<< /Type /StructElem /S /Table /Pg 3 0 R /A << /O /Table /Summary /NotText >> >>",
        "<< /Type /StructElem /S /TH /Pg 3 0 R /A << /O /Table /Short () >> >>",
        "<< /Type /StructElem /S /TH /Pg 3 0 R /A << /O /Table /Short (Reg.) >> >>",
        "<< /Type /StructElem /S /TD /P 5 0 R /Pg 3 0 R >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();

    assert_eq!(
        tree.table_summary(&doc, &elements[0]).as_deref(),
        Some("sales by region and quarter")
    );
    assert_eq!(
        tree.table_summary(&doc, &elements[1]).as_deref(),
        Some("from the class map"),
        "§14.7.6.2's class map is the second route an attribute takes"
    );
    assert_eq!(
        tree.table_summary(&doc, &elements[2]),
        None,
        "the entry is a text string, and a name is a value §14.8.5.7 does not define"
    );
    assert_eq!(
        tree.header_short(&doc, &elements[3]),
        None,
        "an empty string has nothing to speak"
    );
    assert_eq!(
        tree.header_short(&doc, &elements[4]).as_deref(),
        Some("Reg.")
    );

    let inside: Vec<_> = tree
        .children(&doc, Some(&elements[0]))
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();
    assert_eq!(
        tree.table_summary(&doc, &inside[0]),
        None,
        "not inheritable: the cell has not been summarised by its table's sentence"
    );
}

/// Table 379's `/BBox`, read off the elements that state one.
///
/// The second figure states its corners the other way round, which §7.9.5 permits — "a pair
/// of diagonally opposite corners" — and comes back as the same rectangle. The third states
/// three numbers, which is not a rectangle and is answered with nothing rather than with a
/// rectangle three quarters invented.
#[test]
fn a_block_level_element_states_the_rectangle_enclosing_its_visible_content() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /Figure /Pg 3 0 R /A << /O /Layout /BBox [10 20 60 80] >> >>",
        "<< /Type /StructElem /S /Figure /Pg 3 0 R /A << /O /Layout /BBox [60 80 10 20] >> >>",
        "<< /Type /StructElem /S /Figure /Pg 3 0 R /A << /O /Layout /BBox [10 20 60] >> >>",
        "<< /Type /StructElem /S /Figure /Pg 3 0 R >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();

    assert_eq!(
        tree.bounds(&doc, &elements[0]),
        Some([10.0, 20.0, 60.0, 80.0])
    );
    assert_eq!(
        tree.bounds(&doc, &elements[1]),
        Some([10.0, 20.0, 60.0, 80.0]),
        "§7.9.5 states a rectangle as a pair of diagonally opposite corners, in no order"
    );
    assert_eq!(
        tree.bounds(&doc, &elements[2]),
        None,
        "three numbers are not the four Table 379 states"
    );
    assert_eq!(
        tree.bounds(&doc, &elements[3]),
        None,
        "the attribute is optional, and an element stating none has said nothing"
    );
}

/// Table 385's `/BBox` under the `Artifact` owner is the same answer as Table 379's.
///
/// ISO 32000-2 §14.8.5.3 ranks an attribute by the owner that states it, and both of the
/// PDF-native owners here state a rectangle called `/BBox`: §14.8.5.4.3's is a structure
/// element's visible content, §14.8.5.8's is an artifact's visible extent, and neither
/// clause gives the other's owner a different meaning for the name. So an `Artifact`
/// element that states its rectangle in an `Artifact`-owned object is placed exactly as a
/// `Figure` stating one in a `Layout`-owned object is.
///
/// The third element is the discrimination: `HTML-4.01` is one of Table 376's
/// format-specific owners, whose value applies only "if processing based on the format
/// indicated by the owner value", which this program is not — so a rectangle stated there
/// alone is not an answer, and a test that only asked the first two would pass with the
/// owner ignored altogether.
#[test]
fn an_artifact_owned_bounding_box_is_the_same_rectangle_as_a_layout_owned_one() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /StructElem /S /Artifact /Pg 3 0 R \
         /A << /O /Artifact /BBox [12 24 62 84] >> >>",
        "<< /Type /StructElem /S /Figure /Pg 3 0 R \
         /A << /O /Layout /BBox [12 24 62 84] >> >>",
        "<< /Type /StructElem /S /Artifact /Pg 3 0 R \
         /A << /O /HTML-4.01 /BBox [12 24 62 84] >> >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();

    assert_eq!(
        tree.bounds(&doc, &elements[0]),
        Some([12.0, 24.0, 62.0, 84.0]),
        "Table 385 states the entry as much as Table 379 does"
    );
    assert_eq!(
        tree.bounds(&doc, &elements[1]),
        tree.bounds(&doc, &elements[0]),
        "and the two owners give the name one meaning"
    );
    assert_eq!(
        tree.bounds(&doc, &elements[2]),
        None,
        "a format-specific owner's value applies only to a processor translating to it"
    );
}

/// Table 385's `/Type` and `/Subtype` on an `Artifact` element, into the same answer the
/// marked-content form already gives.
///
/// Five elements, one per way §14.8.5.8 can be answered:
///
/// - a running head: `/Type /Pagination` with `/Subtype /Header`, which is the shape
///   §14.8.2.2.2's NOTE about a text-to-speech engine is about;
/// - PDF 2.0's `/Type /Inline`, which is **Table 385's fourth name and not Table 363's** —
///   the discrimination the two tables' difference exists for;
/// - `/Type /Background`, which is Table 363's fourth name and **not** Table 385's, so the
///   element states a name this table does not define and no kind is read;
/// - the two entries under a `Layout` owner, which §14.8.5.8's own first sentence excludes
///   by name, along with `List`, `PrintField` and `Table`;
/// - an `Artifact` element with no attribute object at all, which is still an artifact, the
///   way `/Artifact BMC` is "a generic artifact".
#[test]
fn an_artifact_element_says_which_kind_of_artifact_it_is() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R] >>",
        "<< /Type /StructElem /S /Artifact /Pg 3 0 R \
         /A << /O /Artifact /Type /Pagination /Subtype /Header /BBox [10 80 90 95] >> >>",
        "<< /Type /StructElem /S /Artifact /Pg 3 0 R \
         /A << /O /Artifact /Type /Inline /Subtype /LineNum >> >>",
        "<< /Type /StructElem /S /Artifact /Pg 3 0 R \
         /A << /O /Artifact /Type /Background >> >>",
        "<< /Type /StructElem /S /Artifact /Pg 3 0 R \
         /A << /O /Layout /Type /Pagination /Subtype /Footer >> >>",
        "<< /Type /StructElem /S /Artifact /Pg 3 0 R >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();

    assert_eq!(
        tree.artifact(&doc, &elements[0]),
        Artifact {
            kind: Some(ArtifactKind::Pagination),
            subtype: Some("Header".to_owned()),
            bbox: Some([10.0, 80.0, 90.0, 95.0]),
            attached: [false; 4],
        },
        "the element form of a running head, read as the property list form is"
    );
    assert_eq!(
        tree.artifact(&doc, &elements[1]).kind,
        Some(ArtifactKind::Inline),
        "Table 385's fourth type name is Inline"
    );
    assert_eq!(
        tree.artifact(&doc, &elements[1]).subtype.as_deref(),
        Some("LineNum"),
        "and its /Subtype cell admits the subtypes of an Inline artifact"
    );
    assert_eq!(
        tree.artifact(&doc, &elements[2]).kind,
        None,
        "Background is Table 363's fourth name and is not one of Table 385's"
    );
    assert_eq!(
        tree.artifact(&doc, &elements[3]),
        Artifact::default(),
        "§14.8.5.8 excludes the Layout owner from its own table by name"
    );
    assert_eq!(
        tree.artifact(&doc, &elements[4]),
        Artifact::default(),
        "an Artifact element stating nothing is a generic artifact, not an absent one"
    );
}

/// §14.8.5.4.5's allocation rectangle, and the edges §14.8.3.3 says it adjusts.
///
/// Four elements over the two clauses' one join: a paragraph in the default `LrTb` mode,
/// whose before edge is the top and whose after edge is the bottom; the same numbers under
/// `TbRl`, where block progression runs right to left and the *same two attributes* move the
/// start and end sides of the page instead; a child of the `TbRl` element stating nothing,
/// which inherits the mode ("shall apply to the given structure element and all of its
/// descendants") and not the spacing (Table 379: "not inheritable"); and an element with a
/// rectangle and no spacing at all, which is §14.8.5.4.5's ILSE sentence arrived at rather
/// than branched on — "the allocation rectangle is the same as the content rectangle".
#[test]
fn the_allocation_rectangle_moves_the_edges_the_writing_mode_calls_before_and_after() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /StructParents 0 >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /P /Pg 3 0 R \
         /A << /O /Layout /BBox [20 30 70 60] /SpaceBefore 6 /SpaceAfter 4 >> >>",
        "<< /Type /StructElem /S /P /Pg 3 0 R /K [7 0 R] \
         /A << /O /Layout /BBox [20 30 70 60] /WritingMode /TbRl \
         /SpaceBefore 6 /SpaceAfter 4 >> >>",
        "<< /Type /StructElem /S /Span /P 6 0 R /Pg 3 0 R \
         /A << /O /Layout /BBox [20 30 70 60] >> >>",
        "<< /Type /StructElem /S /P /Pg 3 0 R /A << /O /Layout /BBox [20 30 70 60] >> >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();
    let below = tree.children(&doc, Some(&elements[1]));
    let [Child::Element(nested)] = below.as_slice() else {
        panic!("one child under the TbRl paragraph: {below:?}");
    };

    assert_eq!(
        tree.writing_mode(&doc, &elements[0]),
        WritingMode::LrTb,
        "Table 378's default, which an element stating none takes"
    );
    assert_eq!(
        tree.block_spacing(&doc, &elements[0]),
        BlockSpacing {
            before: 6.0,
            after: 4.0
        }
    );
    assert_eq!(
        tree.allocation(&doc, &elements[0]),
        Some([20.0, 26.0, 70.0, 66.0]),
        "LrTb stacks top to bottom, so before is the top edge and after the bottom"
    );
    assert_eq!(
        tree.allocation(&doc, &elements[1]),
        Some([16.0, 30.0, 76.0, 60.0]),
        "TbRl stacks right to left, so the same two numbers move the right and left edges"
    );
    assert_eq!(
        tree.writing_mode(&doc, nested),
        WritingMode::TbRl,
        "Table 378 makes the mode inheritable, and the cell says it reaches every descendant"
    );
    assert_eq!(
        tree.block_spacing(&doc, nested),
        BlockSpacing::default(),
        "Table 379 makes the spacing not inheritable, so the parent's is not this element's"
    );
    assert_eq!(
        tree.allocation(&doc, nested),
        tree.bounds(&doc, nested),
        "no spacing, so the allocation rectangle is the content rectangle"
    );
    assert_eq!(
        tree.allocation(&doc, &elements[2]),
        Some([20.0, 30.0, 70.0, 60.0]),
        "§14.8.5.4.5's ILSE sentence, reached by Table 379's default of 0"
    );
}

/// A negative `/SpaceBefore` may cross the edges, and the answer is still a rectangle.
///
/// Table 379 states the type as `number` and forbids no sign; `/Padding` next door documents
/// its negative case outright, so a producer writing one here is writing something the tables
/// admit. Every consumer of this rectangle unions and intersects it, and one whose lower-left
/// is above its upper-right would make both mean the opposite of what they say.
#[test]
#[expect(
    clippy::float_cmp,
    reason = "the arithmetic is one subtraction and a sort of values written in the test, so \
              every result is exactly representable and an epsilon here would accept the \
              defect this test exists to find"
)]
fn a_negative_spacing_still_leaves_a_rectangle_the_right_way_up() {
    let crossed = allocation_rectangle(
        [20.0, 30.0, 70.0, 60.0],
        BlockSpacing {
            before: -40.0,
            after: 0.0,
        },
        WritingMode::LrTb,
    );
    assert_eq!(
        crossed,
        [20.0, 20.0, 70.0, 30.0],
        "the before edge crossed the after one, and the pair was sorted rather than trusted"
    );
}

/// §14.8.5.4.5's row and column adjustment, and the control that says it did something.
///
/// A two-by-two grid whose four cells drew four differently sized marks. The clause makes each
/// cell's height its row's and its width its column's, so the four answers are the four
/// intersections of two row bands with two column bands — and the same four rectangles placed
/// one per row and one per column come back untouched, which is what says the grid rather than
/// the arithmetic is doing the work (trap 13).
#[test]
fn a_cell_takes_its_rows_height_and_its_columns_width() {
    let grid = [
        (at(0, 0), Some([0.0, 90.0, 40.0, 100.0])),
        (at(0, 1), Some([50.0, 70.0, 90.0, 100.0])),
        (at(1, 0), Some([0.0, 40.0, 30.0, 60.0])),
        (at(1, 1), Some([50.0, 50.0, 90.0, 60.0])),
    ];

    assert_eq!(
        table_cell_rectangles(&grid, BlockProgression::TopToBottom),
        vec![
            Some([0.0, 70.0, 40.0, 100.0]),
            Some([50.0, 70.0, 90.0, 100.0]),
            Some([0.0, 40.0, 40.0, 60.0]),
            Some([50.0, 40.0, 90.0, 60.0]),
        ],
        "each cell is its row's band by its column's"
    );

    let apart: Vec<_> = grid
        .iter()
        .enumerate()
        .map(|(index, (_, rect))| (at(index, index), *rect))
        .collect();
    assert_eq!(
        table_cell_rectangles(&apart, BlockProgression::TopToBottom),
        apart.iter().map(|(_, rect)| *rect).collect::<Vec<_>>(),
        "a cell alone in its row and its column has nothing to be equalised with"
    );
}

/// A cell that drew nothing is placed by the neighbours that did.
///
/// §14.8.5.4.5 makes the cell's height its row's and its width its column's, and both of those
/// are stated by other cells — so the empty cell of a grid has a rectangle although nothing in
/// it marked the page. Every number in the answer came off a sibling.
#[test]
fn a_cell_that_drew_nothing_is_placed_by_its_row_and_its_column() {
    let grid = [
        (at(0, 0), Some([0.0, 90.0, 40.0, 100.0])),
        (at(0, 1), Some([50.0, 90.0, 90.0, 100.0])),
        (at(1, 0), Some([0.0, 40.0, 40.0, 60.0])),
        (at(1, 1), None),
    ];
    assert_eq!(
        table_cell_rectangles(&grid, BlockProgression::TopToBottom)
            .get(3)
            .copied()
            .flatten(),
        Some([50.0, 40.0, 90.0, 60.0]),
        "the second row's band by the second column's, neither of them this cell's own"
    );
}

/// A cell that spans rows joins no row's band, and takes the bands of every row it covers.
///
/// Table 384's `/RowSpan` makes a cell as tall as the rows it occupies, so letting it into
/// either row's band would make every cell beside it that tall: the header here is 60 units
/// high over two rows, and the cell next to it stays the 10 units it drew.
#[test]
fn a_spanning_cell_joins_no_band_and_takes_the_ones_it_covers() {
    let grid = [
        (
            CellPlacement {
                row: 0,
                column: 0,
                row_span: 2,
                column_span: 1,
            },
            Some([0.0, 40.0, 40.0, 100.0]),
        ),
        (at(0, 1), Some([50.0, 90.0, 90.0, 100.0])),
        (at(1, 1), Some([50.0, 40.0, 90.0, 60.0])),
    ];
    let answer = table_cell_rectangles(&grid, BlockProgression::TopToBottom);

    assert_eq!(
        answer.first().copied().flatten(),
        Some([0.0, 40.0, 40.0, 100.0]),
        "the spanning cell covers both rows' bands and is unchanged by them"
    );
    assert_eq!(
        answer.get(1).copied().flatten(),
        Some([50.0, 90.0, 90.0, 100.0]),
        "its neighbour keeps the first row's band, which the spanning cell is not in"
    );
}

/// §14.8.3.3 decides which pair of edges a row equalises, and `TbRl` is the other pair.
///
/// Table 379 measures a height "in the block-progression direction", so a table whose rows run
/// down the page and stack across it has its rows equalised in x. The same four rectangles
/// under the default mode answer differently, which is what says the mode is read.
#[test]
fn a_vertical_writing_mode_equalises_the_other_pair_of_edges() {
    let grid = [
        (at(0, 0), Some([80.0, 60.0, 100.0, 100.0])),
        (at(0, 1), Some([80.0, 0.0, 100.0, 40.0])),
        (at(1, 0), Some([20.0, 60.0, 60.0, 100.0])),
        (at(1, 1), Some([40.0, 0.0, 60.0, 40.0])),
    ];
    let across = table_cell_rectangles(&grid, BlockProgression::RightToLeft);

    assert_eq!(
        across.get(3).copied().flatten(),
        Some([20.0, 0.0, 60.0, 40.0]),
        "the second row's x band by the second column's y band"
    );
    assert_ne!(
        across,
        table_cell_rectangles(&grid, BlockProgression::TopToBottom),
        "the mode decides which axis a row is equalised along"
    );
}

/// One cell of a table, spanning one row and one column, which is Table 384's default.
fn at(row: usize, column: usize) -> CellPlacement {
    CellPlacement {
        row,
        column,
        row_span: 1,
        column_span: 1,
    }
}

/// §12.5.2's rectangle for the annotations §14.7.5.3's object reference can name.
///
/// Four objects on the page's `/Annots`: one whose corners are the usual way round, one whose
/// are diagonally opposite the other way — §7.9.5 permits it — one stating no `/Rect` at all,
/// and one that is not a dictionary. And a fifth object that *is* an annotation dictionary
/// with a rectangle and is **not** on the page's array, which answers nothing: an element
/// placed from it would be placed from a page it is not on.
#[test]
fn a_pages_annotations_state_where_they_are() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 20 60 80] >>",
        "<< /Type /Annot /Subtype /Text /Rect [60 80 10 20] >>",
        "<< /Type /Annot /Subtype /Text >>",
        "42",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] >>",
    ]);
    let page = doc.get(pdf_syntax::ObjectId::new(3, 0));
    let page = page.as_dict().expect("a page");
    let places = annotation_rectangles(&doc, page);

    assert_eq!(
        places.get(&pdf_syntax::ObjectId::new(4, 0)),
        Some(&[10.0, 20.0, 60.0, 80.0])
    );
    assert_eq!(
        places.get(&pdf_syntax::ObjectId::new(5, 0)),
        Some(&[10.0, 20.0, 60.0, 80.0]),
        "§7.9.5 states a rectangle as a pair of diagonally opposite corners, in no order"
    );
    assert_eq!(
        places.get(&pdf_syntax::ObjectId::new(6, 0)),
        None,
        "Table 166 makes /Rect required, and a file that omits it has said nothing"
    );
    assert_eq!(places.get(&pdf_syntax::ObjectId::new(7, 0)), None);
    assert_eq!(
        places.get(&pdf_syntax::ObjectId::new(8, 0)),
        None,
        "an annotation this page does not list is not this page's"
    );
    assert_eq!(places.len(), 2);
}

/// Table 166's `/Lang` on an annotation, and the three answers §14.9.2.2 gives.
///
/// A stated tag is carried; an empty string is that clause's "the language is unknown", which
/// is the same `None` an absent entry gives; and a tag that is not well-formed BCP 47 is
/// carried as the file wrote it, because Errata Collection 3's Issue #105 states the
/// invalid-means-unknown recovery for Table 29's catalog entry alone. The fifth object is the
/// control [`annotation_rectangles`] has for the same reason: an annotation the page does not
/// list is not this page's, whatever it says about its language.
#[test]
fn a_pages_annotations_state_what_language_they_are_in() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] /Lang (es-MX) >>",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] /Lang () >>",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] >>",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] /Lang (not a tag) >>",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] /Lang (fr) >>",
    ]);
    let page = doc.get(pdf_syntax::ObjectId::new(3, 0));
    let page = page.as_dict().expect("a page");
    let languages = annotation_languages(&doc, page);

    assert_eq!(
        languages
            .get(&pdf_syntax::ObjectId::new(4, 0))
            .map(String::as_str),
        Some("es-MX")
    );
    assert_eq!(
        languages.get(&pdf_syntax::ObjectId::new(5, 0)),
        None,
        "§14.9.2.2 makes the empty string \"the language is unknown\""
    );
    assert_eq!(languages.get(&pdf_syntax::ObjectId::new(6, 0)), None);
    assert_eq!(
        languages
            .get(&pdf_syntax::ObjectId::new(7, 0))
            .map(String::as_str),
        Some("not a tag"),
        "no clause states a recovery for an annotation's malformed tag"
    );
    assert_eq!(
        languages.get(&pdf_syntax::ObjectId::new(8, 0)),
        None,
        "an annotation this page does not list is not this page's"
    );
    assert_eq!(languages.len(), 2);
}

/// §14.8.5.7's four assumptions, and the grid they are asked about.
///
/// The second row is the one worth having: its first *child* is not in its first column,
/// because the cell above it spans two rows. A reader counting children would call that cell
/// the first column's and assume [`HeaderScope::Row`] for it, where the clause's last bullet
/// assumes [`HeaderScope::Both`] — a cell in neither the first row nor the first column.
#[test]
fn a_spanning_cell_moves_the_next_rows_first_child_off_the_first_column() {
    let mut grid = TableGrid::default();
    assert_eq!(
        grid.place(1, 1),
        None,
        "a cell outside a row has no position, and none is invented for it"
    );

    // Row 0: a corner cell two rows tall, then two ordinary cells.
    grid.begin_row();
    assert_eq!(
        grid.place(2, 1).map(|cell| (cell.row, cell.column)),
        Some((0, 0))
    );
    assert_eq!(
        grid.place(1, 1).map(|cell| (cell.row, cell.column)),
        Some((0, 1))
    );
    assert_eq!(
        grid.place(1, 2).map(|cell| (cell.row, cell.column)),
        Some((0, 2))
    );

    // Row 1: column 0 is still occupied by the corner cell, so the first child is column 1.
    grid.begin_row();
    let first = grid.place(1, 1).expect("a cell in a row");
    assert_eq!((first.row, first.column), (1, 1));
    assert_eq!(
        HeaderScope::assumed(first.row, first.column),
        HeaderScope::Both,
        "neither the first row nor the first column"
    );

    // Row 2: the spill has expired and the first child is the first column again.
    grid.begin_row();
    let again = grid.place(1, 1).expect("a cell in a row");
    assert_eq!((again.row, again.column), (2, 0));
    assert_eq!(
        HeaderScope::assumed(again.row, again.column),
        HeaderScope::Row
    );

    assert_eq!(HeaderScope::assumed(0, 0), HeaderScope::Both);
    assert_eq!(HeaderScope::assumed(0, 4), HeaderScope::Column);
    assert_eq!(HeaderScope::read(b"Both"), Some(HeaderScope::Both));
    assert_eq!(HeaderScope::read(b"row"), None, "the names are Table 384's");
}

/// A `/ColSpan` a document controls cannot make this reader allocate without bound.
///
/// And the bound cannot change an answer: every cell past the first column is assumed the
/// same way, so a cell clamped to [`MAX_TABLE_COLUMNS`] is still not in the first column.
#[test]
fn a_hostile_span_is_bounded_and_the_bound_changes_no_assumption() {
    let mut grid = TableGrid::default();
    grid.begin_row();
    let huge = grid.place(1, usize::MAX).expect("a cell in a row");
    assert_eq!(huge.column, 0);
    let next = grid.place(1, 1).expect("a second cell");
    assert_eq!(next.column, MAX_TABLE_COLUMNS);
    assert_eq!(
        HeaderScope::assumed(next.row, next.column),
        HeaderScope::Column,
        "the first row, whatever the column"
    );
}

/// A table inside a cell has its own grid, and the outer table's rows go on afterwards.
///
/// §14.8.4.8.3 bars nothing from a `TD`, and a nested table's first row is its own first row:
/// its corner cell is assumed [`HeaderScope::Both`] however deep in another table it sits.
#[test]
fn a_table_inside_a_cell_does_not_disturb_the_table_around_it() {
    let one = || CellFacts {
        row_span: 1,
        column_span: 1,
        ..CellFacts::default()
    };
    let mut stack = TableStack::new();
    // Table(0) > TR(1) > TD(2), which is the outer table's first cell.
    assert_eq!(stack.enter(0, Some(&StandardType::Table), 0, one), None);
    assert_eq!(stack.enter(1, Some(&StandardType::TableRow), 1, one), None);
    let outer = stack
        .enter(2, Some(&StandardType::TableData), 2, one)
        .expect("a cell in a row");
    assert_eq!((outer.row, outer.column), (0, 0));

    // A table inside that cell: Table(3) > TR(4) > TH(5).
    assert_eq!(stack.enter(3, Some(&StandardType::Table), 3, one), None);
    assert_eq!(stack.enter(4, Some(&StandardType::TableRow), 4, one), None);
    let inner = stack
        .enter(5, Some(&StandardType::TableHeader), 5, one)
        .expect("a cell in the inner table's row");
    assert_eq!((inner.row, inner.column), (0, 0));
    assert_eq!(
        HeaderScope::assumed(inner.row, inner.column),
        HeaderScope::Both
    );

    // Back out to the outer table's second row, which is row 1 and not row 0.
    assert_eq!(stack.enter(1, Some(&StandardType::TableRow), 6, one), None);
    let again = stack
        .enter(2, Some(&StandardType::TableHeader), 7, one)
        .expect("a cell in the outer table's second row");
    assert_eq!((again.row, again.column), (1, 0));
    assert_eq!(
        HeaderScope::assumed(again.row, again.column),
        HeaderScope::Row,
        "the first column of a row that is not the first"
    );

    // And a cell with no table around it is placed nowhere.
    let mut loose = TableStack::new();
    assert_eq!(
        loose.enter(0, Some(&StandardType::TableHeader), 0, one),
        None
    );
}

/// §14.7.2's `/IDTree`: an element found by the identifier it states.
#[test]
fn an_element_is_found_by_its_identifier() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R] /IDTree 6 0 R >>",
        "<< /Type /StructElem /S /Sect /ID (Chap1) >>",
        "<< /Names [(Chap1) 5 0 R] >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let found = tree.element_by_id(&doc, b"Chap1").expect("the element");
    assert_eq!(tree.role(&doc, &found).as_deref(), Some("Sect"));
    assert!(tree.element_by_id(&doc, b"Chap2").is_none());
}

/// Drives one table through a [`TableStack`], a row at a time, and answers its headers.
///
/// `rows` is the table as the walk meets it: one entry per cell, `(is a TH, the cell's own
/// facts)`, grouped into rows. The token each cell gets is its position in the whole table
/// counting from zero, which is what the assertions name it by.
fn headers_of(rows: &[Vec<(bool, CellFacts)>]) -> Vec<(usize, Vec<usize>)> {
    let mut stack = TableStack::new();
    let mut token = 0usize;
    stack.enter(0, Some(&StandardType::Table), token, CellFacts::default);
    for row in rows {
        token = token.saturating_add(1);
        stack.enter(1, Some(&StandardType::TableRow), token, CellFacts::default);
        for (header, facts) in row {
            token = token.saturating_add(1);
            let kind = if *header {
                StandardType::TableHeader
            } else {
                StandardType::TableData
            };
            stack.enter(2, Some(&kind), token, || facts.clone());
        }
    }
    assert!(!stack.truncated(), "no fixture here is near the bound");
    stack.headers()
}

/// A cell of one row and one column, with nothing else stated.
fn cell() -> CellFacts {
    CellFacts {
        row_span: 1,
        column_span: 1,
        ..CellFacts::default()
    }
}

/// §14.8.4.8.3's search: a cell's row headers, then its column headers.
///
/// The table is a header row above a header column, which is the ordinary shape, plus one
/// header spanning two rows — because the row search of the *second* row has to meet it, and a
/// grid that recorded only the row a cell begins in would find nothing there.
///
/// ```text
///     c0        c1      c2
/// r0  TH corner TH one  TH two
/// r1  TH side   TD      TD
/// r2  (spans)   TD      TD
/// ```
#[test]
fn a_cells_headers_are_its_rows_then_its_columns() {
    let spanning = CellFacts {
        row_span: 2,
        ..cell()
    };
    let found = headers_of(&[
        vec![(true, cell()), (true, cell()), (true, cell())],
        vec![(true, spanning), (false, cell()), (false, cell())],
        vec![(false, cell()), (false, cell())],
    ]);
    // Tokens: 0 the table, 1 the first TR, 2 3 4 its cells, 5 the second TR, 6 7 8 its cells,
    // 9 the third TR, 10 and 11 its two — which begin at column 1, the spanning header
    // having taken column 0 of that row.
    let of = |token: usize| {
        found
            .iter()
            .find(|(cell, _)| *cell == token)
            .map(|(_, headers)| headers.clone())
            .unwrap_or_default()
    };
    assert_eq!(of(2), Vec::<usize>::new(), "the corner cell has no headers");
    assert_eq!(
        of(3),
        vec![2],
        "a header in the first row is described by the corner, which is scoped to both"
    );
    assert_eq!(
        of(7),
        vec![6, 3],
        "Table 384's order: the row's header first, then the column's"
    );
    assert_eq!(of(8), vec![6, 4], "the same, one column over");
    assert_eq!(
        of(10),
        vec![6, 3],
        "the row below the spanning header still meets it, at column zero"
    );
}

/// The search stops at the first data cell it meets *after* a header cell.
///
/// One row, `TH TD TH TD`, both headers scoped to the row so that both would be collected if
/// the search ran to the table's edge. The clause stops it at the data cell between them, so
/// the answer is the nearer header alone.
#[test]
fn the_search_stops_at_a_data_cell_after_a_header_cell() {
    let scoped = CellFacts {
        scope: Some(HeaderScope::Row),
        ..cell()
    };
    let found = headers_of(&[vec![
        (true, scoped.clone()),
        (false, cell()),
        (true, scoped),
        (false, cell()),
    ]]);
    // Tokens: 0 the table, 1 the row, 2 3 4 5 the four cells.
    assert_eq!(
        found,
        vec![(3, vec![2]), (4, vec![2]), (5, vec![4])],
        "the last cell's far header is behind a data cell and its near one is not"
    );
}

/// Table 384's `/Headers` replaces the search, and carries the headers of what it names.
///
/// §14.8.5.7:
///
/// > the headers associated with any cell shall be those in its Headers array plus those in
/// > the Headers array of any TH cells in that array, and so on recursively
///
/// The data cell names `B`, `B` names `A`, and the answer is both — in that order, because the
/// entry's own rule is "from most specific to most general". The search would have answered
/// `A` alone, which is what makes this a test of the choice between the two routes.
#[test]
fn a_stated_headers_array_replaces_the_search_and_carries_its_own_headers() {
    let named = |id: &[u8], headers: Option<Vec<Vec<u8>>>| CellFacts {
        id: Some(id.to_vec()),
        headers,
        ..cell()
    };
    let found = headers_of(&[
        vec![
            (true, named(b"A", None)),
            (true, named(b"B", Some(vec![b"A".to_vec()]))),
        ],
        vec![(false, {
            CellFacts {
                headers: Some(vec![b"B".to_vec()]),
                ..cell()
            }
        })],
    ]);
    // Tokens: 0 the table, 1 the first TR, 2 and 3 its cells, 4 the second TR, 5 its cell.
    assert_eq!(
        found.iter().find(|(cell, _)| *cell == 5),
        Some(&(5, vec![3, 2])),
        "the array, then what the array's own cell names"
    );
    // And a cell naming an identifier nobody states is answered with nothing rather than
    // falling back to the search, because the attribute *is* specified.
    let missing = headers_of(&[
        vec![(true, named(b"A", None))],
        vec![(false, {
            CellFacts {
                headers: Some(vec![b"Z".to_vec()]),
                ..cell()
            }
        })],
    ]);
    assert_eq!(missing, Vec::new());
}

/// Table 384's `/Headers`, read off the cells that state one.
#[test]
fn a_cell_states_the_identifiers_of_the_headers_that_describe_it() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /StructElem /S /TD /ID (cell) /A << /O /Table /Headers [(a) (b)] >> >>",
        "<< /Type /StructElem /S /TD /A << /O /Table /Headers [] >> >>",
        "<< /Type /StructElem /S /TD >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let elements: Vec<_> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(dict),
            _ => None,
        })
        .collect();

    let facts = tree.cell_facts(&doc, &elements[0]);
    assert_eq!(facts.id.as_deref(), Some(&b"cell"[..]));
    assert_eq!(
        facts.headers,
        Some(vec![b"a".to_vec(), b"b".to_vec()]),
        "the document's own order, which Table 384 makes part of the answer"
    );
    assert_eq!(
        tree.cell_headers(&doc, &elements[1]),
        Some(Vec::new()),
        "an empty array is the attribute specified, which is not the attribute absent"
    );
    assert_eq!(tree.cell_headers(&doc, &elements[2]), None);
}

/// Table 380's four entries a platform's text interface has a word for, as one element states
/// them: each name in its cell's list, a colour of three components in 0.0 to 1.0, and a name or
/// a colour outside what the cell allows read as the default and as unstated respectively (ADR
/// 2384).
#[test]
fn an_element_states_its_decoration_its_colour_its_shift_and_its_position() {
    use super::{InlineStatements, TextDecoration, TextPosition};
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /StructElem /S /Span /Pg 3 0 R /A << /O /Layout \
         /TextDecorationType /LineThrough /TextDecorationColor [1 0 0.5] \
         /BaselineShift -2.5 /TextPosition /Sup >> >>",
        "<< /Type /StructElem /S /Span /Pg 3 0 R /A << /O /Layout \
         /TextDecorationType /Wavy /TextDecorationColor [2 0 0] /TextPosition /Raised >> >>",
        "<< /Type /StructElem /S /Span /Pg 3 0 R >>",
    ]);
    let tree = Tree::of(&doc).expect("a structure tree");
    let stated: Vec<InlineStatements> = tree
        .children(&doc, None)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(dict) => Some(tree.inline_statements(&doc, &dict)),
            _ => None,
        })
        .collect();
    assert_eq!(
        stated,
        [
            InlineStatements {
                decoration: Some(TextDecoration::LineThrough),
                decoration_colour: Some([1.0, 0.0, 0.5]),
                baseline_shift: Some(-2.5),
                position: Some(TextPosition::Sup),
            },
            InlineStatements {
                decoration: Some(TextDecoration::None),
                decoration_colour: None,
                baseline_shift: None,
                position: Some(TextPosition::Normal),
            },
            InlineStatements::default(),
        ]
    );
}
