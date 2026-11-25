//! Specification audit of ISO 32000-2 §14.6 to §14.9 (review 2229): marked content, logical
//! structure, tagged PDF and accessibility support, through `pdf-model`'s public readings.
//!
//! Each test is one behaviour finding of `doc/reviews/2229-specification-audit-14-6-14-9.md`,
//! with a control the same fixture passes on main beside the failing half. The fixtures are
//! exact-xref documents built here; their text is Helvetica, so nothing depends on a font file.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "exact-xref test fixtures and their assertions"
)]

use std::fmt::Write as _;

use pdf_model::action::Action;
use pdf_model::content::ArtifactSource;
use pdf_model::structure::Tree;
use pdf_syntax::{Document, Object, ObjectId};

/// A classic-xref PDF from object bodies numbered from 1. A body holding `@STREAM@` is a stream:
/// what precedes the marker is the dictionary's entries, what follows it the data.
fn pdf(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let body = match body.split_once("@STREAM@") {
            Some((dict, data)) => format!(
                "<< {dict} /Length {} >>\nstream\n{data}\nendstream",
                data.len().saturating_add(1)
            ),
            None => body.clone(),
        };
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The first page of `bytes`, interpreted, with the document it came from.
fn interpreted(bytes: Vec<u8>) -> (Document, pdf_model::Page, pdf_model::Interpretation) {
    let document = Document::open(bytes).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.glyphs > 0,
        "control: the page's text is shown"
    );
    (document, page, interpretation)
}

/// The page as `Interpretation::speech` vocalises it, its runs joined.
fn spoken(bytes: Vec<u8>) -> String {
    let (_, _, interpretation) = interpreted(bytes);
    interpretation
        .speech()
        .iter()
        .map(|run| run.text.as_str())
        .collect()
}

/// A tagged page that draws a form whose text is `Body` after its own heading `Head`, the form
/// being the one `P` element's content: through §14.7.5.3's object reference naming the form
/// (`as_object`), or through §14.7.5.2's first way, a marked-content sequence around the `Do`.
///
/// Objects: 1 catalog, 2 pages, 3 page, 4 content, 5 font, 6 root, 7 `P`, 8 form.
fn drawn_form(as_object: bool) -> Vec<u8> {
    let (content, form, kids, nums) = if as_object {
        (
            "BT /F1 12 Tf 10 80 Td (Head) Tj ET /Fm1 Do",
            "/StructParent 1",
            "<< /Type /OBJR /Obj 8 0 R /Pg 3 0 R >>",
            "0 [] 1 7 0 R",
        )
    } else {
        (
            "BT /F1 12 Tf 10 80 Td (Head) Tj ET /P <</MCID 0>> BDC /Fm1 Do EMC",
            "",
            "0",
            "0 [7 0 R]",
        )
    };
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R /MarkInfo << /Marked true >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 100] \
         /Resources << /Font << /F1 5 0 R >> /XObject << /Fm1 8 0 R >> >> /Contents 4 0 R \
         /StructParents 0 >>"
            .to_owned(),
        format!("@STREAM@{content}"),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!(
            "<< /Type /StructTreeRoot /K 7 0 R /ParentTree << /Nums [{nums}] >> \
             /ParentTreeNextKey 2 >>"
        ),
        format!("<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K {kids} >>"),
        format!(
            "/Type /XObject /Subtype /Form /BBox [0 0 300 100] {form} \
             /Resources << /Font << /F1 5 0 R >> >> @STREAM@BT /F1 12 Tf 10 50 Td (Body) Tj ET"
        ),
    ])
}

/// Whether the page calls `Body` an artifact by absence, and what its logical text holds.
fn form_reading(as_object: bool) -> (bool, Option<String>) {
    let (document, page, interpretation) = interpreted(drawn_form(as_object));
    let called_artifact = interpretation.artifacts.iter().any(|span| {
        span.found == ArtifactSource::Absence
            && interpretation
                .text
                .get(span.range.clone())
                .is_some_and(|text| text.contains("Body"))
    });
    let tree = Tree::of(&document).expect("the page is tagged");
    let logical = tree.logical_text(&document, page.id.expect("page object"), &interpretation);
    (called_artifact, logical)
}

/// 2229-B01. The control: the form drawn inside the element's marked-content sequence is real
/// content and is in the element's logical text. The failing half: the same form named by the
/// element's object reference, which §14.7.5.3 makes the element's content outright, is called an
/// artifact by absence and is missing from the logical text.
#[test]
fn a_form_named_by_an_object_reference_is_the_elements_real_content_spec_audit() {
    let (called_artifact, logical) = form_reading(false);
    assert!(
        !called_artifact,
        "control: text inside an /MCID sequence is not an artifact"
    );
    assert!(
        logical.as_deref().is_some_and(|text| text.contains("Body")),
        "control: the sequence's text is the element's logical text: {logical:?}"
    );
    let (called_artifact, logical) = form_reading(true);
    assert!(
        !called_artifact,
        "§14.7.5.3: the form an /OBJR names is a content item of the element, and §14.8.2.2.2's \
         artifact by absence is content \"not included in the structure tree\""
    );
    assert!(
        logical.as_deref().is_some_and(|text| text.contains("Body")),
        "§14.8.2.5.1: the logical content order is the structure's, and the form is the \
         element's content: {logical:?}"
    );
}

/// A `Span` stating Spanish around a structured sequence whose element states no `/Lang`, in a
/// document whose catalog states English; `parent_language` puts a `Sect` stating one above it.
fn span_around_structure(parent_language: Option<&str>) -> Vec<u8> {
    let (root_kid, parent, sect) = match parent_language {
        Some(language) => (
            "8 0 R",
            "8 0 R",
            format!("<< /Type /StructElem /S /Sect /P 6 0 R /Lang ({language}) /K [7 0 R] >>"),
        ),
        None => ("7 0 R", "6 0 R", "null".to_owned()),
    };
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Lang (en-US) /StructTreeRoot 6 0 R \
         /MarkInfo << /Marked true >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /StructParents 0 >>"
            .to_owned(),
        "@STREAM@/Span <</Lang (es-MX)>> BDC BT /F1 12 Tf 10 50 Td (Hasta la vista, ) Tj ET \
         /P <</MCID 0>> BDC BT /F1 12 Tf 10 30 Td (as Arnold would say.) Tj ET EMC EMC"
            .to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Type /StructTreeRoot /K {root_kid} /ParentTree << /Nums [0 [7 0 R]] >> >>"),
        format!("<< /Type /StructElem /S /P /P {parent} /Pg 3 0 R /K 0 >>"),
        sect,
    ])
}

/// The language the structured sentence is spoken in.
fn structured_language(bytes: Vec<u8>) -> Option<String> {
    let (_, _, interpretation) = interpreted(bytes);
    interpretation
        .speech()
        .into_iter()
        .find(|run| run.text.contains("Arnold"))
        .expect("the structured sentence is spoken")
        .language
}

/// 2229-B02. §14.9.2.3's EXAMPLE 3 with the element's language stated nowhere in the structure.
/// The control: where an ancestor element states one, the structured sentence takes it over the
/// enclosing `Span`'s. The failing half: where no element does, the sentence takes the `Span`'s
/// Spanish, which the clause withholds from "nested marked-content that is contained in the
/// structure hierarchy", rather than the document's English.
#[test]
fn a_span_language_stops_at_structured_content_inside_it_spec_audit() {
    assert_eq!(
        structured_language(span_around_structure(Some("en-GB"))).as_deref(),
        Some("en-GB"),
        "control: the element inherits its parent element's /Lang over the enclosing Span's"
    );
    assert_eq!(
        structured_language(span_around_structure(None)).as_deref(),
        Some("en-US"),
        "§14.9.2.3: a Span's language applies to its text \"except for nested marked-content that \
         is contained in the structure hierarchy (in which case the structure element's language \
         applies)\", and an element inheriting from no element has the catalog's default"
    );
}

/// A submission of one Required text field with no value, `/T (addr1)` and `alternative`'s
/// entries beside it.
fn required_field(alternative: &str) -> pdf_model::submission::Submission {
    let document = Document::open(pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] \
         /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>"
            .to_owned(),
        "@STREAM@".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [20 60 180 90] /P 3 0 R /F 4 \
             /FT /Tx /T (addr1) {alternative} /Ff 2 /DA (/Helv 12 Tf 0 g) >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        // Reserved example URL; the request is composed and never sent.
        "<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> \
         /Fields [(addr1)] /Flags 2 >>"
            .to_owned(),
    ]))
    .expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let actions = pdf_model::action::read(&document, &Object::Reference(ObjectId::new(7, 0)));
    let Some(Action::SubmitForm(action)) = actions.into_iter().next() else {
        panic!("object 7 is a submit-form action");
    };
    let view = pdf_model::view::ViewState::of(&document);
    pdf_model::submission::compose(&document, &view, &action, None).expect("the action composes")
}

/// 2229-B03. The control: a field stating no `/TU` is reported by its name. The failing half: a
/// field stating one is reported by the name §14.9.3 says a user interface shall not show.
#[test]
fn a_report_about_a_field_names_it_by_its_alternative_name_spec_audit() {
    let unnamed = required_field("");
    assert!(
        unnamed.owed.iter().any(|said| said.contains("addr1")),
        "control: Table 227 bit 2's report names the field: {:?}",
        unnamed.owed
    );
    let named = required_field("/TU (Street address)");
    assert!(
        named
            .owed
            .iter()
            .any(|said| said.contains("Street address")),
        "§14.9.3: /TU \"shall be used in place of the actual field name when an interactive PDF \
         processor identifies the field in a user-interface\": {:?}",
        named.owed
    );
}

/// A tagged page whose `Figure` states `/Alt (A star)`. `split` gives the figure two
/// marked-content sequences of its own instead of one; `nested` puts its one sequence under a
/// child `Span` element instead.
fn described_figure(split: bool, nested: bool) -> Vec<u8> {
    let content = if split {
        "/Figure <</MCID 0>> BDC BT /F1 12 Tf 10 50 Td (x) Tj ET EMC \
         /Figure <</MCID 1>> BDC BT /F1 12 Tf 40 50 Td (y) Tj ET EMC"
    } else {
        "/Figure <</MCID 0>> BDC BT /F1 12 Tf 10 50 Td (x) Tj ET EMC"
    };
    let (kids, nums, child) = match (split, nested) {
        (true, _) => ("[0 1]", "[7 0 R 7 0 R]", "null"),
        (false, true) => (
            "[8 0 R]",
            "[8 0 R]",
            "<< /Type /StructElem /S /Span /P 7 0 R /Pg 3 0 R /K 0 >>",
        ),
        (false, false) => ("[0]", "[7 0 R]", "null"),
    };
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R /MarkInfo << /Marked true >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /StructParents 0 >>"
            .to_owned(),
        format!("@STREAM@{content}"),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Type /StructTreeRoot /K 7 0 R /ParentTree << /Nums [0 {nums}] >> >>"),
        format!("<< /Type /StructElem /S /Figure /P 6 0 R /Pg 3 0 R /K {kids} /Alt (A star) >>"),
        child.to_owned(),
    ])
}

/// 2229-B05. The control: a figure whose one sequence it owns directly is spoken as its
/// description, once and without its letters. The failing halves: the same figure owning two
/// sequences is described twice, and owning its sequence through a child element is not described
/// at all — Table 355's `/Alt` describes "the structure element and its children".
#[test]
fn an_elements_alternate_description_is_spoken_once_over_all_it_encloses_spec_audit() {
    let single = spoken(described_figure(false, false));
    assert!(
        single.matches("A star").count() == 1 && !single.contains('x'),
        "control: the figure's description replaces its one sequence: {single:?}"
    );
    let split = spoken(described_figure(true, false));
    assert_eq!(
        split.matches("A star").count(),
        1,
        "§14.9.3: the description is a substitution for the element, not for each sequence it \
         owns: {split:?}"
    );
    let nested = spoken(described_figure(false, true));
    assert!(
        nested.contains("A star") && !nested.contains('x'),
        "Table 355's /Alt describes \"the structure element and its children\": {nested:?}"
    );
}
