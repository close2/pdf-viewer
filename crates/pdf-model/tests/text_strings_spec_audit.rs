//! ISO 32000-2 §§7.9.2.2 and 7.9.3: text strings and text streams use the same encoding rules.
//!
//! The stream witness is Flate encoded, but its *unencoded* bytes start with §7.9.2.2.1's UTF-8
//! marker. We check those bytes after the stream filter, then send the value through the field
//! appearance reader and require visible ink equal to the equivalent string object.

use std::{fmt::Write as _, io::Write as _};

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, Object, ObjectId};
use render_cpu::CpuRasterizer;

const TEXT: &str = "café";

/// A one-page text field whose `/V` can be either an indirect text stream or a string object.
fn document(value: &str, stream: Option<&[u8]>) -> Vec<u8> {
    let mut out = b"%PDF-2.0\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0_u64];
    let mut object = |number: u32, body: &[u8]| {
        offsets.push(out.len() as u64);
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };

    object(
        1,
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /DR << /Font << /Helv 6 0 R >> >> >> >>",
    );
    object(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    object(
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>",
    );
    object(4, b"<< /Length 0 >>\nstream\n\nendstream");
    let widget = format!(
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx /T (field) /V {value} /DA (/Helv 12 Tf 0 g) >>"
    );
    object(5, widget.as_bytes());
    object(
        6,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    if let Some(encoded) = stream {
        let mut body = format!(
            "<< /Length {} /Filter /FlateDecode >>\nstream\n",
            encoded.len()
        )
        .into_bytes();
        body.extend_from_slice(encoded);
        body.extend_from_slice(b"\nendstream");
        object(7, &body);
    }
    let xref = out.len();
    let size = offsets.len();
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

/// Draws the widget value and returns its supported marks.
#[expect(
    clippy::expect_used,
    reason = "a malformed fixture or unsupported raster is a test setup failure"
)]
fn draw(bytes: Vec<u8>) -> (Vec<String>, pdf_render::Raster) {
    let doc = Document::open(bytes).expect("the generated PDF parses");
    assert!(!doc.was_recovered(), "the generated PDF needs no recovery");
    let page = pdf_model::Pages::new(&doc).get(0).expect("page one exists");
    let result = pdf_model::interpret(&doc, &page);
    let reports = result
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect();
    let target =
        TargetSpec::for_page(&result.display_list, 1.0, 1 << 30).expect("page fits the target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&result.display_list, target)
        .expect("the field appearance is supported");
    (reports, raster)
}

/// §7.9.3 places §7.9.2.2's encoding rules on a stream's decoded, unencoded bytes.
#[test]
fn a_filtered_utf8_text_stream_uses_the_text_string_encoding_after_decoding() {
    let utf8_text_string = [b"\xef\xbb\xbf".as_slice(), TEXT.as_bytes()].concat();
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(&utf8_text_string)
        .expect("compresses fixture data");
    let compressed = encoder.finish().expect("finishes fixture data");

    let stream_doc =
        Document::open(document("7 0 R", Some(&compressed))).expect("the stream fixture parses");
    assert!(
        !stream_doc.was_recovered(),
        "the stream fixture needs no recovery"
    );
    let Object::Stream(stream) = stream_doc.get(ObjectId::new(7, 0)) else {
        panic!("object 7 is the fixture's text stream");
    };
    assert_eq!(
        stream_doc.decoded_stream_data(&stream).as_deref(),
        Some(utf8_text_string.as_slice()),
        "the filter yields the BOM and UTF-8 payload required by §7.9.2.2.1"
    );
    assert_eq!(
        pdf_syntax::text_string(
            &stream_doc
                .decoded_stream_data(&stream)
                .expect("filter succeeds")
        ),
        TEXT,
        "the unencoded stream bytes select UTF-8"
    );

    // The equivalent text-string value uses §7.9.2.2.1's same UTF-8 marker and payload.
    // This comparison must carry ink: two blank appearances are not evidence that `/V` was read.
    let literal = format!("<{}>", hex(&utf8_text_string));
    let (string_reports, string_raster) = draw(document(&literal, None));
    let (stream_reports, stream_raster) = draw(document("7 0 R", Some(&compressed)));
    let empty = format!("<{}>", hex(b"\xef\xbb\xbf"));
    let (_, empty_raster) = draw(document(&empty, None));
    assert!(string_reports.is_empty(), "{string_reports:?}");
    assert!(stream_reports.is_empty(), "{stream_reports:?}");
    assert!(
        string_raster
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[3] != 0),
        "the reference page must produce nontransparent marks"
    );
    assert_ne!(
        string_raster, empty_raster,
        "the text changes the output compared with an empty value"
    );
    assert_eq!(
        stream_raster, string_raster,
        "stream and string values draw alike"
    );
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("{byte:02X}"));
    }
    out
}

/// A small content-stream fixture for reaching the spoken language selected by `/Lang`.
#[expect(
    clippy::expect_used,
    reason = "a malformed fixture or missing page makes the test setup invalid"
)]
fn interpretation(content: &str, catalog: &str) -> pdf_model::Interpretation {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Lang ({catalog}) >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        content.len().saturating_add(1),
    );
    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = vec![0_usize];
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len();
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref}\n%%EOF\n"
    );

    let doc = Document::open(bytes.into_bytes()).expect("the accessibility fixture parses");
    assert!(
        !doc.was_recovered(),
        "the accessibility fixture needs no recovery"
    );
    let page = pdf_model::Pages::new(&doc).get(0).expect("page one exists");
    let interpreted = pdf_model::interpret(&doc, &page);
    assert!(
        interpreted.glyphs > 0,
        "the fixture's source text reaches text drawing"
    );
    interpreted
}

/// §7.9.2.2.2 and §14.9.2.1: a Unicode language escape overrides the prevailing `/Lang`.
#[test]
fn a_text_string_language_escape_overrides_the_catalog_language() {
    let ordinary_actual_text = "FEFF0068006F006C0061";
    let ordinary = interpretation(
        &format!(
            "BT /F1 12 Tf 10 50 Td /Span << /ActualText <{ordinary_actual_text}> >> BDC \
             (x) Tj EMC ET"
        ),
        "en-US",
    );
    let ordinary_spoken = ordinary.speech();
    assert_eq!(ordinary_spoken.len(), 1);
    assert_eq!(ordinary_spoken[0].text, "hola");
    assert_eq!(
        ordinary_spoken[0].language.as_deref(),
        Some("en-US"),
        "without an embedded language escape, the catalog Lang applies"
    );

    // FEFF, ESC, the BCP 47 tag `es`, ESC, then the replacement text `hola`, all UTF-16BE.
    let actual_text = "FEFF001B00650073001B0068006F006C0061";
    let interpreted = interpretation(
        &format!(
            "BT /F1 12 Tf 10 50 Td /Span << /ActualText <{actual_text}> >> BDC \
             (x) Tj EMC ET"
        ),
        "en-US",
    );
    let spoken = interpreted.speech();
    assert_eq!(spoken.len(), 1);
    assert_eq!(spoken[0].text, "hola");
    assert_eq!(
        spoken[0].language.as_deref(),
        Some("es"),
        "the text string's language escape overrides the catalog Lang"
    );
}

/// An embedded language also overrides `/Lang` attached to the marked-content property list.
#[test]
fn a_text_string_language_escape_overrides_property_list_language() {
    let ordinary = interpretation(
        "BT /F1 12 Tf 10 50 Td /Span << /Lang (en-US) /ActualText <FEFF0068006F006C0061> >> BDC (x) Tj EMC ET",
        "",
    );
    let ordinary_spoken = ordinary.speech();
    assert_eq!(ordinary_spoken.len(), 1);
    assert_eq!(ordinary_spoken[0].text, "hola");
    assert_eq!(ordinary_spoken[0].language.as_deref(), Some("en-US"));
    let actual_text = "FEFF001B00650073001B0068006F006C0061";
    let property_language = interpretation(
        &format!(
            "BT /F1 12 Tf 10 50 Td /Span << /Lang (en-US) /ActualText <{actual_text}> >> BDC \
             (x) Tj EMC ET"
        ),
        "",
    );
    let property_spoken = property_language.speech();
    assert_eq!(property_spoken.len(), 1);
    assert_eq!(property_spoken[0].text, "hola");
    assert_eq!(
        property_spoken[0].language.as_deref(),
        Some("es"),
        "the text string's language escape overrides the property-list Lang"
    );
}

/// §7.9.2.3 and Table 246 require FDF `/Status` to use `PDFDocEncoding` specifically.
#[test]
fn a_pdfdocencoded_fdf_status_is_not_mistaken_for_a_unicode_marker() {
    let mut fdf = String::from("%FDF-1.2\n");
    let object_offset = fdf.len();
    fdf.push_str("1 0 obj\n<< /FDF << /Status <FEFF> >> >>\nendobj\n");
    let xref_offset = fdf.len();
    fdf.push_str("xref\n0 2\n0000000000 65535 f \n");
    let _ = writeln!(fdf, "{object_offset:010} 00000 n ");
    let _ = write!(
        fdf,
        "trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
    );

    let doc = Document::open(fdf.into_bytes()).expect("the FDF fixture parses");
    assert!(!doc.was_recovered(), "the FDF fixture needs no recovery");
    let catalog = doc.catalog().expect("the FDF catalog exists");
    let fdf_dictionary = doc.get_key(&catalog, "FDF");
    let fdf_dictionary = fdf_dictionary
        .as_dict()
        .expect("the catalog's FDF entry is a dictionary");
    let status = doc.get_key(fdf_dictionary, "Status");
    assert_eq!(
        status.as_string(),
        Some(&b"\xfe\xff"[..]),
        "the parsed value preserves the intended PDFDocEncoded source bytes"
    );
    let data = pdf_model::forms_data::FormsData::read(&doc).expect("the FDF catalog is readable");
    assert_eq!(
        data.status.as_deref(),
        Some("þÿ"),
        "bytes FE FF are two PDFDocEncoding characters here, not a UTF-16BE marker"
    );
}
