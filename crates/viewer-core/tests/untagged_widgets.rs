//! An untagged page's widget annotations, answered as nodes an assistive technology can act on.
//!
//! ADR 1369. A page whose document states no structure tree says so for its text (ADR 0214), and
//! its fields are published all the same: §12.5.1 makes an annotation something "the user
//! activates … by clicking it", and a field is that whether or not the producer tagged the page.
//! What is *not* invented is a reading order — the order is §12.5.1's tab order, which Table 31's
//! `/Tabs` states for the page's annotations — and the name is Table 226's `/TU` where the field
//! states one.
//!
//! One hand-built document, three times: untagged; with a `/StructTreeRoot` whose one `Form` element
//! names the text field and nothing else changed (trap 8); and with a tree naming both fields. A
//! tagged page's widget that no element names is published after the structure's own nodes, and one
//! an element names is published only as that element (ADR 1381).

#![expect(
    clippy::panic,
    reason = "a test asserts; a failed assertion is the point"
)]

use viewer_core::{Answer, Command, DocumentId, PageStructure, Query, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Assembles a document from object bodies given as bytes, with object 1 the catalog.
fn assembled(objects: &[&str]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(body.as_bytes());
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

/// A form of three fields and a link, with `catalog_extra` added to the catalog and `structure`
/// appended as objects 8 onwards: the structure tree root and its elements, where there are any.
///
/// The page lists the push button first and the text field second, and asks for Table 31's row
/// order: the text field is the higher of the two, so row order puts it first and array order
/// would not. The third widget is Hidden (§12.5.3 bit 2), and the link is not a widget.
fn form(catalog_extra: &str, structure: &[&str]) -> Vec<u8> {
    let catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R 6 0 R] >> \
         {catalog_extra} >>"
    );
    let mut objects: Vec<&str> = vec![
        &catalog,
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Tabs /R \
         /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (go) /TU (Place the order) \
         /Rect [10 10 60 30] /P 3 0 R >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /Rect [10 150 190 170] /P 3 0 R \
         /StructParent 0 >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (secret) /F 2 /Rect [10 80 190 100] \
         /P 3 0 R >>",
        "<< /Type /Annot /Subtype /Link /Rect [100 10 150 30] /A << /S /URI /URI (a:b) >> >>",
    ];
    objects.extend_from_slice(structure);
    assembled(&objects)
}

/// Opens a document and asks for the one page's accessibility answer.
fn answered(bytes: Vec<u8>) -> PageStructure {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Accessibility(mut pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("an open document answers the question");
    };
    assert_eq!(pages.len(), 1, "one page on the screen");
    pages.remove(0)
}

/// The untagged page's fields, in the page's tab order, named by `/TU` where one is stated.
#[test]
fn an_untagged_pages_fields_are_answered_in_its_tab_order() {
    let page = answered(form("", &[]));
    assert!(
        page.nodes.is_empty(),
        "no structure is invented for the page (ADR 0214)"
    );
    let said: Vec<(&str, &str)> = page
        .widgets
        .iter()
        .map(|widget| (widget.role.as_str(), widget.name.as_str()))
        .collect();
    assert_eq!(
        said,
        [("Form", "name"), ("Form", "Place the order")],
        "row order puts the higher field first; /TU names the button, and the hidden field and \
         the link are not offered"
    );
    let [text, button] = page.widgets.as_slice() else {
        panic!("two widgets");
    };
    assert_eq!(
        button.control,
        Some(pdf_model::form::Control::PushButton),
        "Table 229 bit 17 makes it a push button"
    );
    assert_eq!(
        button.annotation,
        Some(pdf_syntax::ObjectId::new(4, 0)),
        "the annotation a click acts on"
    );
    assert!(
        matches!(text.control, Some(pdf_model::form::Control::Text(_))),
        "{:?}",
        text.control
    );
    let place = button.bounds.expect("a widget on the screen has a place");
    assert!(
        place[1] > text.bounds.expect("and so does the other")[1],
        "the button is lower on the screen than the text field: {place:?}"
    );
}

/// A structure tree whose one `Form` element names the text field through §14.7.5.3's `/OBJR`, and
/// whose parent tree answers that widget's `/StructParent` (§14.7.5.4).
const ONE_FORM: &str = "<< /Type /StructTreeRoot /K 9 0 R /ParentTree << /Nums [0 9 0 R] >> >>";

/// The `Form` element [`ONE_FORM`] holds: the text field, and nothing else of the page.
const TEXT_FIELD_ELEMENT: &str = "<< /Type /StructElem /S /Form /P 8 0 R /Pg 3 0 R \
     /K << /Type /OBJR /Obj 5 0 R >> >>";

/// A tagged page whose structure names one widget and leaves the other out: the one it names is
/// its `Form` element and nothing else, and the one it leaves out is still published, by `/TU`, as
/// a widget after the structure's nodes (ADR 1381). Table 368's `Form` "shall be used for each PDF
/// widget annotation that belongs to the real content of the document" binds the producer; the
/// button is still §12.5.1's to click.
#[test]
fn a_tagged_pages_widget_no_element_names_is_still_published() {
    let page = answered(form(
        "/StructTreeRoot 8 0 R /MarkInfo << /Marked true >>",
        &[ONE_FORM, TEXT_FIELD_ELEMENT],
    ));
    let named: Vec<Option<pdf_syntax::ObjectId>> =
        page.nodes.iter().map(|node| node.annotation).collect();
    assert_eq!(
        named,
        [Some(pdf_syntax::ObjectId::new(5, 0))],
        "the structure's one element is the text field's Form: {:?}",
        page.nodes
    );
    let said: Vec<(&str, &str, Option<pdf_syntax::ObjectId>)> = page
        .widgets
        .iter()
        .map(|widget| {
            (
                widget.role.as_str(),
                widget.name.as_str(),
                widget.annotation,
            )
        })
        .collect();
    assert_eq!(
        said,
        [(
            "Form",
            "Place the order",
            Some(pdf_syntax::ObjectId::new(4, 0))
        )],
        "the button no element names is published once, by /TU; the text field is not published \
         twice, and the hidden field and the link are not offered"
    );
}

/// The same page with an element for the button too publishes no widget list: every interactable
/// widget is an element's, and a list beside them would announce each field twice.
#[test]
fn a_tagged_page_whose_structure_names_every_widget_answers_no_list() {
    let page = answered(form(
        "/StructTreeRoot 8 0 R",
        &[
            "<< /Type /StructTreeRoot /K [9 0 R 10 0 R] /ParentTree << /Nums [0 9 0 R] >> >>",
            TEXT_FIELD_ELEMENT,
            "<< /Type /StructElem /S /Form /P 8 0 R /Pg 3 0 R /K << /Type /OBJR /Obj 4 0 R >> >>",
        ],
    ));
    assert_eq!(page.nodes.len(), 2, "{:?}", page.nodes);
    assert!(page.widgets.is_empty(), "{:?}", page.widgets);
}

/// A `Form` element whose only content is its widget is named, not left empty (ADR 1394).
///
/// Table 355's `/T` is "a text string representing it in human-readable form", stated by the
/// producer on the element itself, so it names the element first; where the element states none,
/// §14.9.3's "[a]n alternative name may be specified for an interactive form field" — `/TU`, which
/// "shall be used in place of the actual field name when an interactive PDF processor identifies
/// the field in a user-interface" — and then §12.7.4.2's fully qualified name are the field's.
#[test]
fn a_form_element_with_no_text_is_named_by_its_title_then_its_field() {
    // No `/T` on either element: the button by its `/TU`, the text field by its qualified name.
    let page = answered(form(
        "/StructTreeRoot 8 0 R /MarkInfo << /Marked true >>",
        &[
            "<< /Type /StructTreeRoot /K [9 0 R 10 0 R] /ParentTree << /Nums [0 9 0 R] >> >>",
            TEXT_FIELD_ELEMENT,
            "<< /Type /StructElem /S /Form /P 8 0 R /Pg 3 0 R /K << /Type /OBJR /Obj 4 0 R >> >>",
        ],
    ));
    let named: Vec<(&str, bool)> = page
        .nodes
        .iter()
        .map(|node| (node.name.as_str(), node.substituted))
        .collect();
    assert_eq!(
        named,
        [("name", false), ("Place the order", false)],
        "/TU where the field states one, the qualified name where it does not; a name is not a \
         substitution, so nothing below the element is withheld"
    );

    // The element's own `/T` is the producer naming the element, and it comes before the field's.
    let page = answered(form(
        "/StructTreeRoot 8 0 R /MarkInfo << /Marked true >>",
        &[
            "<< /Type /StructTreeRoot /K [9 0 R 10 0 R] /ParentTree << /Nums [0 9 0 R] >> >>",
            "<< /Type /StructElem /S /Form /P 8 0 R /Pg 3 0 R /T (Your full name) \
             /K << /Type /OBJR /Obj 5 0 R >> >>",
            "<< /Type /StructElem /S /Form /P 8 0 R /Pg 3 0 R /T (Order button) \
             /Alt (Send the order now) /K << /Type /OBJR /Obj 4 0 R >> >>",
        ],
    ));
    let named: Vec<(&str, bool)> = page
        .nodes
        .iter()
        .map(|node| (node.name.as_str(), node.substituted))
        .collect();
    assert_eq!(
        named,
        [("Your full name", false), ("Send the order now", true)],
        "the element's /T before the field's names; §14.9.3's /Alt, a substitution, before /T"
    );
}

/// A `Sect` whose content is its children's is named by its title, and an element with text of its
/// own keeps that text (ADR 1405).
///
/// Table 355 defines `/T` for every structure element — "[t]he title of the structure element, a
/// text string representing it in human-readable form" — and its example is a section's: "such as
/// Chapter 1". The element's own text still comes first, so a paragraph that states a title is
/// spoken as what it says.
#[test]
fn a_section_with_no_text_of_its_own_is_named_by_its_title() {
    let content = "/P << /MCID 0 >> BDC BT /F1 12 Tf 20 150 Td (The first paragraph) Tj ET EMC \
                   /P << /MCID 1 >> BDC BT /F1 12 Tf 20 100 Td (A titled paragraph) Tj ET EMC";
    let stream = format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    );
    let page = answered(assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R /MarkInfo << /Marked true >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> /StructParents 0 >>",
        &stream,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        "<< /Type /StructTreeRoot /K 7 0 R /ParentTree << /Nums [0 [8 0 R 9 0 R]] >> >>",
        "<< /Type /StructElem /S /Sect /P 6 0 R /T (Chapter 1) /K [8 0 R 9 0 R] >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 0 >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /T (Paragraph two) /K 1 >>",
    ]));
    let named: Vec<(&str, &str, bool)> = page
        .nodes
        .iter()
        // Trimmed, because the second paragraph's text begins on a new line and the extraction
        // says so; the line break is not what this test is about.
        .map(|node| (node.role.as_str(), node.name.trim(), node.titled))
        .collect();
    assert_eq!(
        named,
        [
            ("Sect", "Chapter 1", true),
            ("P", "The first paragraph", false),
            ("P", "A titled paragraph", false),
        ],
        "the section by its title; each paragraph by its own text, titled or not"
    );
    assert!(
        page.nodes.iter().all(|node| !node.substituted),
        "a title substitutes for nothing, so both paragraphs are still published"
    );
}

/// A two-page document whose catalog states `catalog_extra` and whose structure tree holds one
/// paragraph: page one's text, reached through its `/StructParents` (§14.7.5.4). Page two draws
/// text of its own and nothing in the tree names it.
fn two_pages(catalog_extra: &str) -> Vec<u8> {
    let first = "/P << /MCID 0 >> BDC BT /F1 12 Tf 20 100 Td (Tagged words) Tj ET EMC";
    let second = "BT /F1 12 Tf 20 100 Td (Words nobody tagged) Tj ET";
    let catalog = format!("<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>");
    let first_stream = format!("<< /Length {} >>\nstream\n{first}\nendstream", first.len());
    let second_stream = format!(
        "<< /Length {} >>\nstream\n{second}\nendstream",
        second.len()
    );
    assembled(&[
        &catalog,
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 5 0 R \
         /Resources << /Font << /F1 7 0 R >> >> /StructParents 0 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 6 0 R \
         /Resources << /Font << /F1 7 0 R >> >> >>",
        &first_stream,
        &second_stream,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        "<< /Type /StructTreeRoot /K 9 0 R /ParentTree << /Nums [0 [9 0 R]] >> >>",
        "<< /Type /StructElem /S /P /P 8 0 R /Pg 3 0 R /K 0 >>",
    ])
}

/// Each page of [`two_pages`], as the viewer answers it after turning to that page.
fn both_pages(bytes: Vec<u8>) -> [PageStructure; 2] {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let mut asked = |index: usize| {
        viewer
            .handle(Command::GoTo(viewer_core::PageTarget::Index(index)))
            .for_each(drop);
        let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
            panic!("an open document answers the question");
        };
        pages
            .into_iter()
            .find(|page| page.page == index)
            .unwrap_or_else(|| panic!("page {index} is on the screen"))
    };
    let first = asked(0);
    [first, asked(1)]
}

/// A tagged document's page that its structure reaches nothing on is not answered as untagged
/// (ADR 1393).
///
/// §14.7.2 locates the structure tree root through the catalog's `/StructTreeRoot`, so whether a
/// document states a structure is one fact for all its pages;
/// and §14.8.1's "[a] tagged PDF document shall contain a mark information dictionary" with
/// `/Marked true` is the claim that makes an unreached page the producer's omission.
#[test]
fn a_tagged_documents_unreached_page_is_answered_as_that() {
    let [first, second] = both_pages(two_pages(
        "/StructTreeRoot 8 0 R /MarkInfo << /Marked true >>",
    ));
    assert_eq!(first.tagging, viewer_core::Tagging::Reached);
    let said: Vec<(&str, &str)> = first
        .nodes
        .iter()
        .map(|node| (node.role.as_str(), node.name.as_str()))
        .collect();
    assert_eq!(said, [("P", "Tagged words")]);
    assert!(second.nodes.is_empty(), "{:?}", second.nodes);
    assert_eq!(
        second.tagging,
        viewer_core::Tagging::Unreached { marked: true }
    );

    // The same structure without `/MarkInfo`: a structure, and no claim to be a tagged PDF.
    let [_, second] = both_pages(two_pages("/StructTreeRoot 8 0 R"));
    assert_eq!(
        second.tagging,
        viewer_core::Tagging::Unreached { marked: false }
    );

    // And with no structure tree at all, both pages are the untagged answer ADR 0214 states.
    let [first, second] = both_pages(two_pages("/MarkInfo << /Marked true >>"));
    assert_eq!(first.tagging, viewer_core::Tagging::Untagged);
    assert_eq!(second.tagging, viewer_core::Tagging::Untagged);
    assert!(first.nodes.is_empty() && second.nodes.is_empty());
}
