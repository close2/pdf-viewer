//! Four subclauses of ISO 32000-2 §14.8, each run over a document written here.
//!
//! The rows for §14.8.2.5.2, §14.8.2.6, §14.8.2.6.2 and §14.8.6 were held by
//! `logical_order.rs`'s and `notes.rs`'s measurements over the corpus, which say how the reader
//! behaves on the documents that exist and say nothing on a machine without them. A row's
//! evidence is a value its clause derives (principle 5, ADR 1497), so each test below builds the
//! construct its clause names and asserts what the quoted sentence makes of it.
//!
//! # The one machine dependency
//!
//! A glyph outline comes from a substituted standard-14 face, as in `accessibility.rs`. A machine
//! with no fonts would draw no glyphs and read back nothing, so [`interpret`] panics naming that
//! rather than letting an assertion about an empty string pass.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure, \
              and these offsets are within a fixture written in this file"
)]

use std::fmt::Write as _;

use pdf_model::structure::{Child, Tree};
use pdf_syntax::{Document, ObjectId};

/// The page every fixture draws, object 3.
const PAGE: ObjectId = ObjectId::new(3, 0);

/// A one-page tagged document: the catalog, the page tree, page 3 with `content`, a Helvetica
/// `/F1` whose dictionary carries `font_extra`, and `objects` spliced in as written.
///
/// `page_extra` goes into the page dictionary and `objects` may use any number from 6 up, since
/// offsets are keyed by the number each object states rather than by its position.
fn document(content: &str, page_extra: &str, font_extra: &str, objects: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
         /StructTreeRoot 10 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R {page_extra} >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica {font_extra} >>\nendobj\n\
         {objects}",
        content.len() + 1,
    );
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = std::collections::BTreeMap::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        let number: usize = object
            .split_whitespace()
            .next()
            .and_then(|word| word.parse().ok())
            .expect("every object states its number");
        offsets.insert(number, cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let xref_at = out.len();
    let size = offsets.keys().copied().max().unwrap_or(0) + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some(offset) => {
                let _ = writeln!(out, "{offset:010} 00000 n ");
            }
            None => {
                let _ = writeln!(out, "0000000000 65535 f ");
            }
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Opens a fixture.
fn open(bytes: Vec<u8>) -> Document {
    Document::open(bytes).expect("the fixture is a valid file")
}

/// Interprets the fixture's one page, failing where no glyph could be drawn.
fn interpret(document: &Document) -> pdf_model::Interpretation {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has one page");
    let drawn = pdf_model::interpret(document, &page);
    assert!(
        drawn.glyphs > 0,
        "no glyphs were drawn, so this machine has no substitute for Helvetica and every \
         assertion below would pass vacuously"
    );
    drawn
}

/// A text string written as UTF-16BE with its byte order mark, §7.9.2.2.1.
fn utf16(text: &str) -> String {
    let mut out = String::from("<FEFF");
    for unit in text.encode_utf16() {
        let _ = write!(out, "{unit:04X}");
    }
    out.push('>');
    out
}

/// ISO 32000-2 §14.8.2.5.2, its operative sentence:
///
/// > The position of an annotation in the logical content order is determined from the
/// > document's logical structure.
///
/// The fixture makes both of the orders the structure could be confused with disagree with it.
/// The page's `/Annots` lists link 7 before link 8, and its content stream draws `/MCID 1` before
/// `/MCID 0`; the structure tree's depth-first traversal — §14.8.2.5.1's definition of logical
/// content order — visits `MCID 0`, link 8, `MCID 1`, link 7. So the expected order is the
/// tree's, item for item, and neither the array's nor the stream's.
///
/// The clause's second paragraph is the construct the tree uses: a `Link` element whose content
/// item is §14.7.5.3's object reference to the annotation. And the annotation is placed rather
/// than read: `Tree::logical_text` is the page's readback in that order, which holds the two
/// sequences' text and no annotation's, because "[a]nnotations associated with a page are not
/// interleaved within the page's content stream".
#[test]
fn an_annotation_is_placed_where_the_structure_puts_it_and_not_where_annots_lists_it() {
    let bytes = document(
        "/P << /MCID 1 >> BDC BT /F1 12 Tf 10 20 Td (second) Tj ET EMC \
         /P << /MCID 0 >> BDC BT /F1 12 Tf 10 60 Td (first) Tj ET EMC",
        "/Annots [7 0 R 8 0 R] /StructParents 0",
        "",
        "7 0 obj\n<< /Type /Annot /Subtype /Link /Rect [100 10 150 30] /StructParent 1 \
         /Contents (link seven) >>\nendobj\n\
         8 0 obj\n<< /Type /Annot /Subtype /Link /Rect [100 50 150 70] /StructParent 2 \
         /Contents (link eight) >>\nendobj\n\
         10 0 obj\n<< /Type /StructTreeRoot /K [11 0 R] \
         /ParentTree << /Nums [0 [12 0 R 14 0 R] 1 15 0 R 2 13 0 R] >> >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /Document /P 10 0 R \
         /K [12 0 R 13 0 R 14 0 R 15 0 R] >>\nendobj\n\
         12 0 obj\n<< /Type /StructElem /S /P /P 11 0 R /Pg 3 0 R /K 0 >>\nendobj\n\
         13 0 obj\n<< /Type /StructElem /S /Link /P 11 0 R \
         /K << /Type /OBJR /Obj 8 0 R /Pg 3 0 R >> >>\nendobj\n\
         14 0 obj\n<< /Type /StructElem /S /P /P 11 0 R /Pg 3 0 R /K 1 >>\nendobj\n\
         15 0 obj\n<< /Type /StructElem /S /Link /P 11 0 R \
         /K << /Type /OBJR /Obj 7 0 R /Pg 3 0 R >> >>\nendobj\n",
    );
    let document = open(bytes);
    let tree = Tree::of(&document).expect("the fixture is tagged");

    let order = tree.logical_order(&document, PAGE);
    assert!(!order.truncated);
    let items: Vec<String> = order
        .items
        .iter()
        .map(|item| match item {
            Child::MarkedContent { mcid, .. } => format!("MCID {mcid}"),
            Child::Object { object, .. } => format!("annotation {}", object.number),
            Child::Element(_) => "element".to_owned(),
        })
        .collect();
    assert_eq!(
        items,
        ["MCID 0", "annotation 8", "MCID 1", "annotation 7"],
        "the structure's depth-first order, not `/Annots`' order nor the stream's"
    );

    let drawn = interpret(&document);
    let stream_order = (drawn.text.find("second"), drawn.text.find("first"));
    assert!(
        matches!(stream_order, (Some(second), Some(first)) if second < first),
        "the fixture's content stream draws the second paragraph first: {:?}",
        drawn.text
    );
    let logical = tree
        .logical_text(&document, PAGE, &drawn)
        .expect("a two-element tree is not truncated");
    let logical_order = (logical.find("first"), logical.find("second"));
    assert!(
        matches!(logical_order, (Some(first), Some(second)) if first < second),
        "the logical order puts the first paragraph first: {logical:?}"
    );
    assert!(
        !logical.contains("link"),
        "an annotation is placed in the order, not interleaved into the page's text: {logical:?}"
    );
}

/// ISO 32000-2 §14.8.2.6.1, the requirement and its two remedies, read back.
///
/// > Every character code that belongs to a structure element in the structure tree shall map to
/// > Unicode, except where an associated Alt or ActualText entry applies to the content to which
/// > the character code belongs.
///
/// The clause's own example of a code that does not map by itself is the soft hyphen, and it
/// names the two ways a document supplies one: a `ToUnicode` entry mapping the code (§9.10.2's
/// first method), or "an `ActualText` entry in the associated structure element to provide
/// substitute characters". Each is built here and each must read back U+00AD where the glyphs
/// alone would not give it — the first from code 0x2D, which Helvetica's standard encoding names
/// `hyphen` and whose `ToUnicode` entry here says otherwise, the second from an element reached
/// through §14.7.5.4's parent tree whose `/ActualText` replaces the six letters drawn.
#[test]
fn a_soft_hyphen_reads_back_through_either_route_the_clause_names() {
    let cmap = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
                /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def \
                /CMapName /Soft-Hyphen def /CMapType 2 def \
                1 begincodespacerange <00> <FF> endcodespacerange \
                1 beginbfchar <2D> <00AD> endbfchar \
                endcmap CMapName currentdict /CMap defineresource pop end end";
    let structure = "10 0 obj\n<< /Type /StructTreeRoot /K [12 0 R] \
                     /ParentTree << /Nums [0 [12 0 R]] >> >>\nendobj\n\
                     12 0 obj\n<< /Type /StructElem /S /P /P 10 0 R /Pg 3 0 R /K 0 {} >>\nendobj\n";

    let mapped = open(document(
        "/P << /MCID 0 >> BDC BT /F1 12 Tf 10 50 Td (hy-phen) Tj ET EMC",
        "/StructParents 0",
        "/ToUnicode 6 0 R",
        &format!(
            "6 0 obj\n<< /Length {} >>\nstream\n{cmap}\nendstream\nendobj\n{}",
            cmap.len() + 1,
            structure.replace("{}", "")
        ),
    ));
    assert_eq!(
        interpret(&mapped).text.trim_end(),
        "hy\u{AD}phen",
        "the ToUnicode entry maps code 0x2D to U+00AD, which §9.10.2 ranks first"
    );

    let replaced = open(document(
        "/P << /MCID 0 >> BDC BT /F1 12 Tf 10 50 Td (hyphen) Tj ET EMC",
        "/StructParents 0",
        "",
        &structure.replace("{}", &format!("/ActualText {}", utf16("hy\u{AD}phen"))),
    ));
    assert_eq!(
        interpret(&replaced).text.trim_end(),
        "hy\u{AD}phen",
        "the element's ActualText supplies the character the drawn codes do not have"
    );
}

/// ISO 32000-2 §14.8.2.6.2, the requirement on the reader:
///
/// > The identification of what constitutes a word shall be unrelated to how the text happens to
/// > be grouped into show strings. The division into show strings shall have no semantic
/// > significance.
///
/// So `(Wo) Tj (rd) Tj`, the second string starting exactly where the first ended, is one word,
/// and `(Hello ) Tj (world) Tj` is two words separated by the one SPACE the document stated —
/// the clause's "a SPACE (U+0020) or other word-breaking character shall be present in a
/// character stream even if a word break happens to fall at the end of a show string". That
/// document states its white space, which is NOTE 1's case: the processor "can determine word
/// breaks without having to rely on heuristics", and so this reader reconstructs no separator
/// from position (`inferred_separators` is zero).
///
/// The third string is the clause's "as augmented by replacement text specified with
/// ActualText": a word read across a replacement is read with the replacement's characters.
#[test]
fn a_show_string_boundary_is_not_a_word_boundary() {
    let read = |content: &str| {
        let document = open(document(
            content,
            "",
            "",
            "10 0 obj\n<< /Type /StructTreeRoot >>\nendobj\n",
        ));
        let drawn = interpret(&document);
        (drawn.text.trim_end().to_owned(), drawn.inferred_separators)
    };

    assert_eq!(
        read("BT /F1 12 Tf 10 50 Td (Wo) Tj (rd) Tj ET"),
        ("Word".to_owned(), 0),
        "two show strings, one word"
    );
    assert_eq!(
        read("BT /F1 12 Tf 10 50 Td (Hello ) Tj (world) Tj ET"),
        ("Hello world".to_owned(), 0),
        "the stated SPACE at a string's end is the word break, and the only one"
    );
    assert_eq!(
        read(
            "BT /F1 12 Tf 10 50 Td (He) Tj /Span << /ActualText (llo) >> BDC (xyz) Tj EMC \
             ( world) Tj ET"
        )
        .0,
        "Hello world",
        "the character stream as augmented by ActualText"
    );
}

/// ISO 32000-2 §14.8.6.1, the two namespace names and the default:
///
/// > To facilitate conversion of documents created against versions of the PDF standard earlier
/// > than PDF 2.0, the default standard structure namespace shall be "`http://iso.org/pdf/ssn`".
/// > When a namespace is not explicitly specified for a given structure element or attribute, it
/// > shall be assumed to be within this default standard structure namespace.
///
/// Three elements: a `P` that states no `/NS`, which is therefore in the PDF 1.7 namespace and not
/// the PDF 2.0 one; a `P` whose `/NS` is a namespace dictionary naming `http://iso.org/pdf2/ssn`,
/// the clause's other standard name; and a `Widget` in a namespace §14.8.6 does not define and no
/// role map leaves, which §14.8.6.2's "[i]n a tagged PDF, all structure elements shall be in at
/// least one of the standard structure namespaces or in a namespace identified in 14.8.6.3" makes
/// the one element outside the standard.
#[test]
fn an_element_stating_no_namespace_is_in_the_default_standard_one() {
    let document = open(document(
        "BT /F1 12 Tf 10 50 Td (x) Tj ET",
        "",
        "",
        "10 0 obj\n<< /Type /StructTreeRoot /K [11 0 R 12 0 R 13 0 R] \
         /Namespaces [20 0 R 21 0 R] >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /P /P 10 0 R >>\nendobj\n\
         12 0 obj\n<< /Type /StructElem /S /P /P 10 0 R /NS 20 0 R >>\nendobj\n\
         13 0 obj\n<< /Type /StructElem /S /Widget /P 10 0 R /NS 21 0 R >>\nendobj\n\
         20 0 obj\n<< /Type /Namespace /NS (http://iso.org/pdf2/ssn) >>\nendobj\n\
         21 0 obj\n<< /Type /Namespace /NS (http://example.invalid/tagset) >>\nendobj\n",
    ));
    let tree = Tree::of(&document).expect("the fixture is tagged");
    let element = |number: u32| {
        document
            .get(ObjectId::new(number, 0))
            .as_dict()
            .expect("the fixture's element")
            .clone()
    };
    let (unstated, pdf_2, foreign) = (element(11), element(12), element(13));

    assert_eq!(
        tree.namespace(&document, &unstated).as_deref(),
        Some("http://iso.org/pdf/ssn")
    );
    assert!(!tree.in_pdf_2_0_namespace(&document, &unstated));
    assert_eq!(
        tree.namespace(&document, &pdf_2).as_deref(),
        Some("http://iso.org/pdf2/ssn")
    );
    assert!(tree.in_pdf_2_0_namespace(&document, &pdf_2));
    assert_eq!(
        tree.namespace(&document, &foreign).as_deref(),
        Some("http://example.invalid/tagset")
    );

    let outside = tree.namespaces_outside_the_standard(&document);
    assert_eq!(outside.len(), 1, "{outside:?}");
    assert_eq!(
        outside[0].name.as_deref(),
        Some("http://example.invalid/tagset")
    );
    assert_eq!(
        outside[0].elements, 1,
        "the Widget, and neither standard P: {outside:?}"
    );
}
