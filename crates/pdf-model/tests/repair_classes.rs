//! `pdf_model::repair`'s classes reach the document's one channel where the reader makes them,
//! for a viewer as much as for a validator (ADR 3477, ADR 3541).
//!
//! Each fixture is one page whose single departure is the one named, beside a twin that states
//! the same page as ISO 32000-2 asks; the twin logs nothing, the damaged page logs its class at
//! the site the class belongs to.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not open should fail loudly, and every offset is a \
              few hundred bytes into a file built here"
)]

use std::fmt::Write as _;

use pdf_model::repair;
use pdf_model::{Pages, content};
use pdf_syntax::{Document, ObjectId, RepairClass, Site};

/// A whole file from object bodies, numbered from 1, with an exact cross-reference table.
fn file(objects: &[String]) -> Document {
    let mut bytes: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    let size = objects.len() + 1;
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(table.as_bytes());
    Document::open(bytes).expect("the fixture opens")
}

/// One page, its dictionary carrying `extra` and its content stream `operators`.
fn page(operators: &str, extra: &str) -> Document {
    file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources << >> {extra} >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{operators}\nendstream",
            operators.len()
        ),
    ])
}

const PAGE: ObjectId = ObjectId::new(3, 0);

/// The page read and drawn, and the classes its document logged, with their sites.
fn logged(document: &Document) -> Vec<(&'static str, Site)> {
    let pages = Pages::new(document);
    let page = pages.get(0).expect("the page is read");
    let _ = content::interpret(document, &page);
    document
        .repairs()
        .into_iter()
        .map(|repair| (repair.class.name, repair.at))
        .collect()
}

fn has(document: &Document, class: &RepairClass, at: Site) {
    let log = logged(document);
    assert!(
        log.contains(&(class.name, at)),
        "{} at {at:?} is in the log, which holds {log:?}",
        class.name
    );
}

/// A page stating every entry as Table 31 asks needs no repair.
#[test]
fn a_well_formed_page_logs_nothing() {
    assert_eq!(logged(&page("0 0 m 1 1 l S", "")), []);
}

/// Table 31: `/Rotate` "shall be a multiple of 90".
#[test]
fn a_rotation_of_forty_five_is_logged_at_the_page() {
    has(
        &page("0 0 m 1 1 l S", "/Rotate 45"),
        &repair::ROTATE_NOT_A_MULTIPLE_OF_NINETY,
        Site::Object(PAGE),
    );
}

/// Table 31: `/MediaBox` is a rectangle.
#[test]
fn a_media_box_that_is_a_name_is_logged_at_the_page() {
    let document = file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox /Letter /Contents 4 0 R /Resources << >> >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
    ]);
    has(
        &document,
        &repair::MEDIA_BOX_SUBSTITUTED,
        Site::Object(PAGE),
    );
}

/// §8.4.2: "Occurrences of the q and Q operators shall be balanced within a given content
/// stream". The interpreter ignores the `Q` and logs it at the page's content.
#[test]
fn a_restore_with_nothing_saved_is_logged_at_the_content() {
    has(
        &page("Q 0 0 m 1 1 l S", ""),
        &repair::RESTORE_WITH_NOTHING_SAVED,
        Site::Content(Some(PAGE)),
    );
}

/// §8.6.8: three `sc` operands under `DeviceGray`.
#[test]
fn colour_operands_that_do_not_fit_are_logged_at_the_content() {
    has(
        &page("/DeviceGray cs 1 0 0 sc 0 0 m 1 1 l S", ""),
        &repair::COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE,
        Site::Content(Some(PAGE)),
    );
}

/// The content reader a viewer draws through logs what lexing needed, so the viewer is not
/// silent about a string that never ends.
#[test]
fn the_page_reader_logs_its_lexical_repairs() {
    has(
        &page("0 0 m 1 1 l S (open", ""),
        &pdf_syntax::repair::STRING_UNTERMINATED,
        Site::Content(Some(PAGE)),
    );
}

/// §7.8.2: "operands shall not be left over when an operator finishes execution". The `5` in
/// front of `m`'s two is dropped, and logged at the page's content.
#[test]
fn operands_left_over_are_logged_at_the_content() {
    has(
        &page("5 0 0 m 1 1 l S", ""),
        &repair::OPERANDS_LEFT_OVER,
        Site::Content(Some(PAGE)),
    );
}

/// §7.8.2: `BX` and `EX` "shall occur in pairs" — an `EX` ending nothing, and a `BX` left open.
#[test]
fn unbalanced_compatibility_sections_are_logged_at_the_content() {
    for operators in ["EX 0 0 m 1 1 l S", "BX 0 0 m 1 1 l S"] {
        has(
            &page(operators, ""),
            &repair::COMPATIBILITY_SECTIONS_UNBALANCED,
            Site::Content(Some(PAGE)),
        );
    }
    assert_eq!(logged(&page("BX 0 0 m 1 1 l S EX", "")), []);
}

/// Table 352: `BMC` is "terminated by a balancing EMC operator" — an `EMC` balancing nothing,
/// and a sequence left open.
#[test]
fn unbalanced_marked_content_is_logged_at_the_content() {
    for operators in ["EMC 0 0 m 1 1 l S", "/Span BMC 0 0 m 1 1 l S"] {
        has(
            &page(operators, ""),
            &repair::MARKED_CONTENT_UNBALANCED,
            Site::Content(Some(PAGE)),
        );
    }
    assert_eq!(logged(&page("/Span BMC 0 0 m 1 1 l S EMC", "")), []);
}

/// §14.6.2: a named property list is "a named resource in the Properties subdictionary"; this
/// page's resources hold none.
#[test]
fn a_property_list_the_resources_do_not_hold_is_logged_at_the_content() {
    has(
        &page("/OC /Missing BDC 0 0 m 1 1 l S EMC", ""),
        &repair::PROPERTY_LIST_UNRESOLVED,
        Site::Content(Some(PAGE)),
    );
}

/// One page whose resources hold `/GS0` as `state` and `/Fm0` as a form stating `form`.
fn page_with_resources(operators: &str, state: &str, form: &str) -> Document {
    file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources << /ExtGState << /GS0 {state} >> /XObject << /Fm0 5 0 R >> >> >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{operators}\nendstream",
            operators.len()
        ),
        format!(
            "<< /Type /XObject /Subtype /Form {form} /Length 13 >>\nstream\n0 0 m 1 1 l S\nendstream"
        ),
    ])
}

/// Table 57: `/LW` is a number and `/SA` a boolean; `/TR` is a function, an array of four or
/// `/Identity`. Each of the wrong type is ignored, and logged at the content that set it.
#[test]
fn graphics_state_entries_of_the_wrong_type_are_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /LW /Thick >>", box_),
        &repair::GRAPHICS_STATE_ENTRY_OF_THE_WRONG_TYPE,
        Site::Content(Some(PAGE)),
    );
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /SA 1 >>", box_),
        &repair::GRAPHICS_STATE_ENTRY_OF_THE_WRONG_TYPE,
        Site::Content(Some(PAGE)),
    );
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /TR /Inverted >>", box_),
        &repair::TRANSFER_FUNCTION_UNREADABLE,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /LW 2 /SA true /TR /Identity >>",
            box_
        )),
        []
    );
}

/// Table 93: `/BBox` is required and four numbers; `/Matrix` is six. A form missing the first
/// and stating five numbers for the second is drawn, and both are logged at the content that
/// invoked it.
#[test]
fn a_form_with_no_bbox_and_a_short_matrix_is_logged_at_the_content() {
    let document = page_with_resources("/Fm0 Do", "<< >>", "/Matrix [1 0 0 1 0]");
    has(
        &document,
        &repair::FORM_BBOX_UNUSABLE,
        Site::Content(Some(PAGE)),
    );
    has(
        &document,
        &repair::FORM_MATRIX_MALFORMED,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/Fm0 Do",
            "<< >>",
            "/BBox [0 0 1 1] /Matrix [1 0 0 1 0 0]"
        )),
        []
    );
}

/// Table 93: a form's `/Resources` is a dictionary. An integer there is read as absent, the form's
/// names then looked up in the page's, and logged at the content that invoked it.
#[test]
fn a_form_with_resources_of_the_wrong_type_is_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/Fm0 Do", "<< >>", &format!("{box_} /Resources 7")),
        &repair::FORM_RESOURCES_NOT_A_DICTIONARY,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/Fm0 Do",
            "<< >>",
            &format!("{box_} /Resources << >>")
        )),
        []
    );
}

/// ISO 32000-2 §11.3.7.2: "All of the shape and opacity inputs shall have values in the range
/// 0.0 to 1.0 (inclusive)". A `/CA` of 2 is read as 1, and logged.
#[test]
fn an_alpha_constant_out_of_range_is_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /CA 2 >>", box_),
        &repair::ALPHA_CONSTANT_OUT_OF_RANGE,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /CA 0.5 /ca 1 >>",
            box_
        )),
        []
    );
}

/// ISO 32000-2 §11.6.3: a `/BM` name "shall be either a name object, designating one of the
/// standard blend modes listed in" Tables 134 and 135, or an array of such names. A name outside
/// both is read as `Normal` and logged; a known one, and an array whose names none are known
/// (the clause's own "or Normal if it recognizes none of them"), are not.
#[test]
fn a_blend_mode_no_table_lists_is_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /BM /Blurry >>", box_),
        &repair::BLEND_MODE_UNKNOWN,
        Site::Content(Some(PAGE)),
    );
    for state in [
        "<< /BM /Multiply >>",
        "<< /BM [/Blurry /Screen] >>",
        "<< /BM [/Blurry] >>",
    ] {
        assert_eq!(
            logged(&page_with_resources("/GS0 gs 0 0 m 1 1 l S", state, box_)),
            [],
            "{state}"
        );
    }
}

/// Table 57's `/HT` is a dictionary, a stream or the name `/Default`, and Table 128's
/// `/TransferFunction` a function or `/Identity`. Each of the wrong kind is logged.
#[test]
fn an_unreadable_halftone_and_its_transfer_function_are_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /HT 5 >>", box_),
        &repair::HALFTONE_UNREADABLE,
        Site::Content(Some(PAGE)),
    );
    has(
        &page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /HT << /HalftoneType 1 /TransferFunction /Inverted >> >>",
            box_,
        ),
        &repair::HALFTONE_TRANSFER_FUNCTION_UNREADABLE,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /HT << /HalftoneType 1 /TransferFunction /Identity >> >>",
            box_
        )),
        []
    );
}

/// A document whose catalog states `extra`.
fn catalog(extra: &str) -> Document {
    file(&[
        format!("<< /Type /Catalog /Pages 2 0 R {extra} >>"),
        "<< /Type /Pages /Kids [] /Count 0 >>".to_owned(),
    ])
}

fn logged_alone(document: &Document) -> Vec<(&'static str, Site)> {
    document
        .repairs()
        .into_iter()
        .map(|repair| (repair.class.name, repair.at))
        .collect()
}

/// Table 29's `/PageLayout` names one of six layouts and Table 147's `/HideToolbar` is a boolean:
/// a name outside the table and a value of another type are read as the defaults, and logged at
/// the catalog. `/PrintScaling`'s unrecognised value is the clause's own `AppDefault`.
#[test]
fn display_entries_and_viewer_preferences_of_the_wrong_kind_are_logged_at_the_catalog() {
    let document = catalog("/PageLayout /Sideways");
    let _ = pdf_model::viewer_preferences::Opening::read(&document);
    assert_eq!(
        logged_alone(&document),
        [(
            repair::CATALOG_DISPLAY_ENTRY_UNUSABLE.name,
            Site::Object(ObjectId::new(1, 0))
        )]
    );
    let document = catalog("/ViewerPreferences << /HideToolbar 1 >>");
    let _ = pdf_model::viewer_preferences::ViewerPreferences::read(&document);
    assert_eq!(
        logged_alone(&document),
        [(
            repair::VIEWER_PREFERENCE_UNUSABLE.name,
            Site::Object(ObjectId::new(1, 0))
        )]
    );
    let document = catalog(
        "/PageLayout /TwoPageLeft /ViewerPreferences << /HideToolbar true /PrintScaling /Fit >>",
    );
    let _ = pdf_model::viewer_preferences::Opening::read(&document);
    let _ = pdf_model::viewer_preferences::ViewerPreferences::read(&document);
    assert_eq!(logged_alone(&document), []);
}

/// Table 215's `/H` is a boolean: one of another type is read as its default, and logged.
#[test]
fn an_action_entry_of_the_wrong_type_is_logged() {
    let document = catalog("");
    let action = pdf_syntax::Object::Dictionary(
        pdf_syntax::Parser::new(b"<< /S /Hide /T (a) /H 1 >>")
            .parse_object()
            .expect("the action parses")
            .as_dict()
            .expect("a dictionary")
            .clone(),
    );
    let _ = pdf_model::action::read(&document, &action);
    assert_eq!(
        logged_alone(&document),
        [(repair::ACTION_ENTRY_UNUSABLE.name, Site::WholeFile)]
    );
}

/// One page drawing `a` and `b` in a Type 3 font whose dictionary states `widths` and
/// `differences` as Table 110 and Table 112 read them.
fn type3_page(widths: &str, differences: &str) -> Document {
    let glyph = "0 0 0 0 0 0 d1";
    file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_owned(),
        {
            let operators = "BT /F1 1 Tf (ab) Tj ET";
            format!(
                "<< /Length {} >>\nstream\n{operators}\nendstream",
                operators.len()
            )
        },
        format!(
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 0 0] \
             /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /a 6 0 R /b 6 0 R >> \
             /Encoding << /Type /Encoding /Differences {differences} >> {widths} >>"
        ),
        format!("<< /Length {} >>\nstream\n{glyph}\nendstream", glyph.len()),
    ])
}

/// Table 110 makes `/FirstChar`, `/LastChar` and `/Widths` required, the last one number per code
/// of the range; a Type 3 font stating them so logs nothing, and one whose array falls short of its
/// range, or that states none, logs `font/widths-malformed`, its missing code given 0.
#[test]
fn a_type3_font_whose_widths_fall_short_is_logged() {
    let stated = type3_page("/FirstChar 97 /LastChar 98 /Widths [500 500]", "[97 /a /b]");
    assert_eq!(logged(&stated), []);
    let short = type3_page("/FirstChar 97 /LastChar 98 /Widths [500]", "[97 /a /b]");
    has(&short, &pdf_font::repair::WIDTHS_MALFORMED, Site::WholeFile);
    let absent = type3_page("", "[97 /a /b]");
    has(
        &absent,
        &pdf_font::repair::WIDTHS_MALFORMED,
        Site::WholeFile,
    );
}

/// §9.6.5.1: each code in `/Differences` is "the first index in a sequence of character codes",
/// a byte for a simple font; a name before any code is read past and logged.
#[test]
fn a_type3_differences_array_with_a_name_before_a_code_is_logged() {
    has(
        &type3_page(
            "/FirstChar 97 /LastChar 98 /Widths [500 500]",
            "[/a 97 /a /b]",
        ),
        &pdf_font::repair::DIFFERENCES_MALFORMED,
        Site::WholeFile,
    );
}

/// A dictionary written in PDF syntax, for a reader handed one directly.
fn dictionary(text: &str) -> pdf_syntax::Dictionary {
    pdf_syntax::Parser::new(text.as_bytes())
        .parse_object()
        .expect("the dictionary parses")
        .as_dict()
        .expect("a dictionary")
        .clone()
}

/// Table 31 types a page's `/Annots` an array: one of another type lists no annotation, and is
/// logged once by the one function every reader of the entry goes through (ADR 3750).
#[test]
fn a_page_annots_that_is_no_array_is_logged() {
    has(
        &page("", "/Annots 5"),
        &repair::PAGE_ANNOTS_NOT_AN_ARRAY,
        Site::WholeFile,
    );
    assert_eq!(logged(&page("", "/Annots []")), []);
}

/// ISO 32000-2 §8.9.7: `/L` "shall be present on all inline images". A stated length that is no
/// non-negative integer is read past, even where §8.9.3's arithmetic then finds `EI` (ADR 3749).
#[test]
fn an_inline_image_length_that_is_no_length_is_logged_at_the_content() {
    has(
        &page("BI /W 1 /H 1 /BPC 8 /CS /G /L -1 ID x EI", ""),
        &repair::INLINE_IMAGE_DATA_EXTENT,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page("BI /W 1 /H 1 /BPC 8 /CS /G /L 1 ID x EI", "")),
        []
    );
}

/// Table 98 makes `/D` required: an `/OCProperties` without a configuration dictionary is read
/// with every group on, and logged (ADR 3749).
#[test]
fn an_optional_content_configuration_that_is_no_dictionary_is_logged() {
    let document = catalog("/OCProperties << /OCGs [] /D 5 >>");
    let _ = pdf_model::optional_content::OptionalContent::read(&document);
    assert_eq!(
        logged_alone(&document),
        [(
            repair::OPTIONAL_CONTENT_ENTRY_UNUSABLE.name,
            Site::WholeFile
        )]
    );
    let document = catalog("/OCProperties << /OCGs [] /D << >> >>");
    let _ = pdf_model::optional_content::OptionalContent::read(&document);
    assert_eq!(logged_alone(&document), []);
}

/// Table 272's `/Names` is "(Required; PDF 2.0)": a point data dictionary without it is read as
/// naming no column, and logged (ADR 3749).
#[test]
fn a_point_data_dictionary_without_its_names_is_logged() {
    let document = catalog("");
    let _ = pdf_model::measurement::point_data(
        &document,
        &dictionary("<< /PtData << /Type /PtData /Subtype /Cloud /XPTS [[1 2]] >> >>"),
    );
    assert_eq!(
        logged_alone(&document),
        [(repair::MEASUREMENT_ENTRY_UNUSABLE.name, Site::WholeFile)]
    );
    let document = catalog("");
    let _ = pdf_model::measurement::point_data(
        &document,
        &dictionary(
            "<< /PtData << /Type /PtData /Subtype /Cloud /Names [/LAT /LON] /XPTS [[1 2]] >> >>",
        ),
    );
    assert_eq!(logged_alone(&document), []);
}

/// A document holding one embedded file stream, object 3, whose dictionary carries `extra`.
fn embedded(extra: &str) -> Document {
    file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [] /Count 0 >>".to_owned(),
        format!("<< /Type /EmbeddedFile /Length 1 {extra} >>\nstream\nx\nendstream"),
    ])
}

/// Table 43 makes `/F` "Required if the DOS, Mac, and Unix entries are all absent", and Table 44
/// types `/Params` a dictionary: a specification naming no file is shown with the empty name,
/// a `/Params` of another type is read as absent, and each is logged (ADR 3751).
#[test]
fn a_file_specification_naming_no_file_is_logged() {
    let specification = |entries: &str| {
        pdf_syntax::Object::Dictionary(dictionary(&format!(
            "<< /Type /Filespec /EF << /F 3 0 R >> {entries} >>"
        )))
    };
    let document = embedded("");
    let read = pdf_model::attachment::associated_in_array(&document, &[specification("")]);
    assert_eq!(read.len(), 1, "the file is still read");
    assert_eq!(
        logged_alone(&document),
        [(
            repair::FILE_SPECIFICATION_ENTRY_UNUSABLE.name,
            Site::WholeFile
        )]
    );
    let document = embedded("/Params 5");
    let _ = pdf_model::attachment::associated_in_array(&document, &[specification("/UF (a)")]);
    assert_eq!(
        logged_alone(&document),
        [(
            repair::FILE_SPECIFICATION_ENTRY_UNUSABLE.name,
            Site::WholeFile
        )]
    );
    let document = embedded("/Params << /Size 1 >>");
    let _ = pdf_model::attachment::associated_in_array(&document, &[specification("/F (a)")]);
    assert_eq!(logged_alone(&document), []);
}

/// XML 1.0 defines five entities and character references to characters: a packet's reference
/// to neither is kept as written, and logged at its metadata stream (ADR 3752).
#[test]
fn an_xmp_reference_xml_does_not_define_is_logged_at_the_stream() {
    let metadata = |producer: &str| {
        let packet = format!(
            "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
             xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description \
             rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\"><pdf:Producer>{producer}\
             </pdf:Producer></rdf:Description></rdf:RDF></x:xmpmeta>"
        );
        file(&[
            "<< /Type /Catalog /Pages 2 0 R /Metadata 3 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [] /Count 0 >>".to_owned(),
            format!(
                "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{packet}\nendstream",
                packet.len()
            ),
        ])
    };
    let document = metadata("a&#xZZ;b");
    let read = pdf_model::xmp::Xmp::document(&document).expect("the catalog states a packet");
    assert!(read.is_ok(), "the packet is still read: {read:?}");
    assert_eq!(
        logged_alone(&document),
        [(
            repair::XMP_REFERENCE_UNDEFINED.name,
            Site::Object(ObjectId::new(3, 0))
        )]
    );
    let document = metadata("a&amp;b&#x41;");
    let _ = pdf_model::xmp::Xmp::document(&document);
    assert_eq!(logged_alone(&document), []);
}

/// Table 158's `/Position` "shall range from 0 to 100" and Table 159's `/ID` is required: a split
/// past the range is read at its end and a folder without an identifier as folder 0, each
/// logged (ADR 3757).
#[test]
fn a_collection_split_or_folder_entry_out_of_its_table_is_logged() {
    let collection = |entries: &str| {
        let document = catalog(&format!("/Collection << {entries} >>"));
        let read = pdf_model::collection::Collection::read(&document);
        (read, logged_alone(&document))
    };
    let unusable = [(repair::COLLECTION_ENTRY_UNUSABLE.name, Site::WholeFile)];
    let (read, log) = collection("/Split << /Direction /V /Position 150 >>");
    assert_eq!(
        read.and_then(|read| read.split)
            .and_then(|split| split.position),
        Some(100.0)
    );
    assert_eq!(log, unusable);
    let (_, log) = collection("/Folders << /Name (root) >>");
    assert_eq!(log, unusable);
    let (_, log) = collection("/Split << /Position 40 >> /Folders << /ID 0 /Name (root) >>");
    assert_eq!(log, []);
}

/// Table 396 makes `/BoxColorInfo` a dictionary and Table 397 a box style's `/W` a number: an
/// entry of another type is drawn in its default, said, and logged (ADR 3757).
#[test]
fn a_box_colour_info_entry_out_of_its_table_is_logged() {
    let read = |extra: &str| {
        let document = page("", extra);
        let page = Pages::new(&document).get(0).expect("the page is read");
        let _ = pdf_model::guidelines::of(&document, &page);
        logged_alone(&document)
    };
    let unusable = [(repair::BOX_COLOUR_INFO_UNUSABLE.name, Site::WholeFile)];
    assert_eq!(read("/BoxColorInfo 5"), unusable);
    assert_eq!(
        read("/CropBox [0 0 10 10] /BoxColorInfo << /CropBox << /W /Thick >> >>"),
        unusable
    );
    assert_eq!(
        read("/CropBox [0 0 10 10] /BoxColorInfo << /CropBox << /W 2 >> >>"),
        []
    );
}
