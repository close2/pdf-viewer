//! Direct-evaluation tests for ISO 32000-2 §7.10.2.
//!
//! Every function is an indirect object in a complete one-page PDF with a valid xref table.
//! These tests exercise the public function parser/evaluator, independent of a shading's
//! separate dimensionality and colour-space constraints.

#![expect(
    clippy::expect_used,
    reason = "fixed fixtures must parse before testing their numerical functions"
)]

use pdf_colour::function::Function;
use pdf_syntax::{Document, Object, ObjectId};

/// A complete one-page PDF with a Type 0 function stream in object 5.
fn sampled_document(entries: &str, data: &[u8]) -> Document {
    let contents = b"q Q\n";
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
          /Resources << >> /Contents 4 0 R >>"
            .to_vec(),
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            contents.len(),
            String::from_utf8_lossy(contents)
        )
        .into_bytes(),
    ];
    let mut function = format!(
        "<< /FunctionType 0 {entries} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    function.extend_from_slice(data);
    function.extend_from_slice(b"\nendstream");
    objects.push(function);

    let mut pdf = b"%PDF-1.7\n%\x80\x81\xFE\xFF\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let size = offsets.len().saturating_add(1);
    pdf.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    let document = Document::open(pdf).expect("the complete PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture must use its xref table"
    );
    document
}

fn parse_sampled(document: &Document) -> Function {
    let object = Object::Reference(ObjectId {
        number: 5,
        generation: 0,
    });
    Function::parse(document, &object).expect("valid sampled function parses")
}

/// Table 39's default Encode maps the Domain across [0, Size-1], and real encoded positions
/// interpolate between adjacent samples instead of rounding to an integer index.
#[test]
fn sampled_default_encode_interpolates_at_a_real_sample_position() {
    let document = sampled_document(
        "/Domain [0 1] /Range [0 255] /Decode [0 255] /Size [3] /BitsPerSample 8",
        &[0, 100, 200],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[0.0]), vec![0.0]);
    assert_eq!(function.eval(&[0.25]), vec![50.0]);
    assert_eq!(function.eval(&[0.5]), vec![100.0]);
    assert_eq!(function.eval(&[1.0]), vec![200.0]);
}

/// Domain clipping occurs before interpolation even when Encode maps the endpoints into
/// the interior of the sample table, where encoded-coordinate clipping cannot explain equality.
#[test]
fn sampled_input_is_clipped_to_domain_before_encode() {
    let document = sampled_document(
        "/Domain [0 1] /Range [0 255] /Decode [0 255] /Size [3] \
         /BitsPerSample 8 /Encode [0.5 1.5]",
        &[10, 110, 210],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[-2.0]), vec![60.0]);
    assert_eq!(function.eval(&[0.0]), vec![60.0]);
    assert_eq!(function.eval(&[1.0]), vec![160.0]);
    assert_eq!(function.eval(&[2.0]), vec![160.0]);
}

/// Encode values outside the sample table are clipped to [0, Size-1] after mapping.
#[test]
fn sampled_encoded_position_is_clipped_to_sample_table() {
    let document = sampled_document(
        "/Domain [0 1] /Range [0 255] /Decode [0 255] /Size [3] \
         /BitsPerSample 8 /Encode [-4 8]",
        &[10, 110, 210],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[0.0]), vec![10.0]);
    assert_eq!(function.eval(&[1.0]), vec![210.0]);
}

/// Table 39 defaults Decode to Range; decoded samples are in the function's output range.
#[test]
fn sampled_decode_defaults_to_the_declared_range() {
    let document = sampled_document(
        "/Domain [0 1] /Range [-2 2] /Size [3] /BitsPerSample 8 /Encode [0 2]",
        &[0, 128, 255],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[0.0]), vec![-2.0]);
    assert!((function.eval(&[0.5])[0] - (-2.0 + 4.0 * 128.0 / 255.0)).abs() < 1e-6);
    assert_eq!(function.eval(&[1.0]), vec![2.0]);
}

/// The clause's Example 3 uses Decode [-1, 16/14] so sample 7 of 4 bits represents the
/// midpoint exactly, even though the usual [-1,1] Decode would put zero between samples 7 and 8.
#[test]
fn example_3_decode_places_zero_on_the_seventh_four_bit_sample() {
    let document = sampled_document(
        "/Domain [0 1] /Range [-1 1] /Decode [-1 1.1429] /Size [2] /BitsPerSample 4",
        &[0x70],
    );
    let function = parse_sampled(&document);
    assert_eq!(
        function.eval(&[1.0]),
        vec![-1.0],
        "sample zero maps to Decode low"
    );
    assert!(function.eval(&[0.0])[0].abs() < 1e-4);
}

/// Decoding can extend beyond Range; §7.10.2 clips the decoded value afterward.
#[test]
fn sampled_decoded_outputs_are_clipped_to_range() {
    let document = sampled_document(
        "/Domain [0 1] /Range [10 20] /Decode [0 30] /Size [2] \
         /BitsPerSample 8 /Encode [0 1]",
        &[0, 255],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[0.0]), vec![10.0]);
    assert_eq!(function.eval(&[1.0]), vec![20.0]);
}

/// For two outputs, each grid point stores values in the component order of Range.
#[test]
fn sampled_outputs_follow_the_order_of_range_components() {
    let document = sampled_document(
        "/Domain [0 1] /Range [0 1 0 1] /Decode [0 1 0 1] \
         /Size [2] /BitsPerSample 8",
        &[0, 255, 255, 0],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[0.0]), vec![0.0, 1.0]);
    assert_eq!(function.eval(&[1.0]), vec![1.0, 0.0]);
}

/// Dimension zero varies fastest, as in Example 1 and the Type 0 data order rule.
#[test]
fn sampled_first_input_dimension_varies_fastest_in_the_array() {
    let document = sampled_document(
        "/Domain [0 1 0 1] /Range [0 255] /Decode [0 255] \
         /Size [2 2] /BitsPerSample 8",
        &[0, 10, 20, 50],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[1.0, 0.0]), vec![10.0]);
    assert_eq!(function.eval(&[0.0, 1.0]), vec![20.0]);
    assert_eq!(function.eval(&[1.0, 1.0]), vec![50.0]);
    assert_eq!(
        function.eval(&[0.25, 0.75]),
        vec![21.25],
        "bilinear interior has four unequal weights"
    );
}

/// Size=1 has one permitted sample, so every input in that dimension maps to it.
#[test]
fn sampled_single_entry_dimension_returns_its_only_sample_for_every_input() {
    let document = sampled_document(
        "/Domain [0 1] /Range [0 255] /Decode [0 255] /Size [1] /BitsPerSample 8",
        &[128],
    );
    let function = parse_sampled(&document);
    for x in [-10.0, 0.0, 0.37, 1.0, 10.0] {
        assert_eq!(function.eval(&[x]), vec![128.0]);
    }
}

/// The 12-bit fields are adjacent across byte boundaries and high-order-bit first:
/// 0x123, 0xABC, 0x456 pack to 12 3A BC 45 60 (four unused low bits at the end).
#[test]
fn sampled_twelve_bit_values_are_high_order_first_and_have_no_inter_sample_padding() {
    let document = sampled_document(
        "/Domain [0 1] /Range [0 4095] /Decode [0 4095] /Size [3] /BitsPerSample 12",
        &[0x12, 0x3A, 0xBC, 0x45, 0x60],
    );
    let function = parse_sampled(&document);
    assert_eq!(function.eval(&[0.0]), vec![f32::from(0x123_u16)]);
    assert_eq!(function.eval(&[0.5]), vec![f32::from(0xABC_u16)]);
    assert_eq!(function.eval(&[1.0]), vec![f32::from(0x456_u16)]);
}
