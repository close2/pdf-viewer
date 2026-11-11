//! Tests for ISO 32000-2 §§7.10.1, 7.10.3 and 7.10.4.
//!
//! Each fixture is a complete one-page PDF with
//! an xref table and valid stream extents; the function object is read through `Document`.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "PDF fixtures are fixed test data and must fail loudly if malformed"
)]

use std::fmt::Write as _;

use pdf_colour::function::Function;
use pdf_syntax::{Document, Object, ObjectId};

/// Open one complete page document whose indirect function dictionary is object 5.
fn document_with_function(function: &str) -> Document {
    let contents = b"q Q";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n{}\nendobj\n",
        contents.len(),
        String::from_utf8_lossy(contents),
        function
    );
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize];
    for object in body.split_inclusive("endobj\n") {
        offsets.push(pdf.len());
        pdf.push_str(object);
    }
    let xref = pdf.len();
    let _ = writeln!(pdf, "xref\n0 {}", offsets.len());
    pdf.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    let document = Document::open(pdf.into_bytes()).expect("the complete PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture must use its xref table"
    );
    document
}

fn function(document: &Document) -> Function {
    let object = Object::Reference(ObjectId {
        number: 5,
        generation: 0,
    });
    Function::parse(document, &object).expect("valid function parses")
}

/// Table 40's formula uses x^N, with x clipped to /Domain; /Range clips each output afterward.
#[test]
fn exponential_uses_its_exponent_clips_domain_and_clips_each_output_to_range() {
    let document = document_with_function(
        "<< /FunctionType 2 /Domain [0 2] /Range [0.2 0.8 1 2] \
         /C0 [0 1] /C1 [1 3] /N 2 >>",
    );
    let f = function(&document);

    assert_eq!(
        f.eval(&[0.5]),
        vec![0.25, 1.5],
        "N=2 differs from N=1 before clipping"
    );
    // At x=1, x^2=1, so the raw vector is [1, 3] and Range gives [0.8, 2].
    assert_eq!(f.eval(&[1.0]), vec![0.8, 2.0]);
    // x below Domain clips to 0 before evaluation; Range then raises output 0 to 0.2.
    assert_eq!(f.eval(&[-4.0]), vec![0.2, 1.0]);
    // x above Domain clips to 2: x^2=4, raw outputs [4, 9], then Range clips both.
    assert_eq!(f.eval(&[7.0]), vec![0.8, 2.0]);
}

/// Table 40 defaults C0 and C1 independently to [0] and [1].
#[test]
fn exponential_uses_the_table_40_coefficient_defaults() {
    let document = document_with_function("<< /FunctionType 2 /Domain [0 1] /N 1 >>");
    let f = function(&document);
    assert_eq!(f.eval(&[0.25]), vec![0.25]);
    assert_eq!(f.eval(&[1.0]), vec![1.0]);
}

/// Table 41 assigns exact bounds to the interval on their right, except the final endpoint.
/// It also maps each selected subdomain through its own Encode pair before evaluating its child.
#[test]
fn stitching_selects_right_hand_interval_at_a_bound_and_applies_each_encode_pair() {
    let document = document_with_objects(&[
        (
            5,
            "<< /FunctionType 3 /Domain [-1 1] /Functions [6 0 R 7 0 R] \
             /Bounds [0] /Encode [1 0 0 1] >>",
        ),
        (
            6,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
        (
            7,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
    ]);
    let f = function(&document);
    assert_eq!(f.eval(&[-0.5]), vec![0.5], "first interval reverses Encode");
    assert_eq!(
        f.eval(&[0.0]),
        vec![0.0],
        "bound belongs to interval on its right"
    );
    assert_eq!(
        f.eval(&[1.0]),
        vec![1.0],
        "last interval includes Domain's high end"
    );
    assert_eq!(
        f.eval(&[-5.0]),
        vec![1.0],
        "input below Domain clips before mapping"
    );
    assert_eq!(
        f.eval(&[5.0]),
        vec![1.0],
        "input above Domain clips before mapping"
    );
}

/// Table 41 permits k=1: one child covers the full domain and Bounds is empty.
#[test]
fn one_function_stitch_covers_the_whole_domain_with_no_bounds() {
    let document = document_with_objects(&[
        (
            5,
            "<< /FunctionType 3 /Domain [-2 2] /Functions [6 0 R] \
             /Bounds [] /Encode [0 1] >>",
        ),
        (
            6,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
    ]);
    let f = function(&document);
    assert_eq!(f.eval(&[-2.0]), vec![0.0]);
    assert_eq!(f.eval(&[0.0]), vec![0.5]);
    assert_eq!(f.eval(&[2.0]), vec![1.0]);
}

/// When the final bound equals Domain high, §7.10.4 specifies Encode[2(k-1)] at that
/// endpoint instead of interpolating across an empty final interval.
#[test]
fn stitching_final_bound_at_domain_high_uses_the_last_encode_low() {
    let document = document_with_objects(&[
        (
            5,
            "<< /FunctionType 3 /Domain [0 1] /Functions [6 0 R 7 0 R] \
             /Bounds [1] /Encode [0 1 0.25 0.75] >>",
        ),
        (
            6,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
        (
            7,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
    ]);
    let f = function(&document);
    assert_eq!(f.eval(&[0.5]), vec![0.5]);
    assert_eq!(f.eval(&[1.0]), vec![0.25]);
}

/// The Domain may have zero width only for k=1; the sole input value still maps to Encode[0].
#[test]
fn one_function_stitch_allows_a_degenerate_domain() {
    let document = document_with_objects(&[
        (
            5,
            "<< /FunctionType 3 /Domain [0 0] /Functions [6 0 R] \
             /Bounds [] /Encode [0.6 1] >>",
        ),
        (
            6,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
    ]);
    let f = function(&document);
    assert_eq!(f.eval(&[0.0]), vec![0.6]);
}

/// A boundary at Domain's lower end is closed on both sides for the first interval, then
/// excluded by the next interval; the degenerate Encode mapping uses Encode[0] there.
#[test]
fn stitching_domain_start_bound_uses_its_explicit_degenerate_encode_value() {
    let document = document_with_objects(&[
        (
            5,
            "<< /FunctionType 3 /Domain [0 1] /Functions [6 0 R 7 0 R] \
             /Bounds [0] /Encode [0.75 0.25 0 1] >>",
        ),
        (
            6,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
        (
            7,
            "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        ),
    ]);
    let f = function(&document);
    assert_eq!(
        f.eval(&[0.5]),
        vec![0.5],
        "positive control: interior selects the second interval"
    );
    assert_eq!(f.eval(&[0.0]), vec![0.75]);
    assert_eq!(
        f.eval(&[-1.0]),
        vec![0.75],
        "clipping to Domain low selects the same singleton interval"
    );
}

/// Assemble a complete valid one-page file with an arbitrary set of indirect function objects.
fn document_with_objects(functions: &[(u32, &str)]) -> Document {
    let mut body = String::from(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 3 >>\nstream\nq Q\nendstream\nendobj\n",
    );
    let highest = functions
        .iter()
        .map(|(number, _)| *number)
        .max()
        .unwrap_or(4);
    for (number, source) in functions {
        let _ = writeln!(body, "{number} 0 obj\n{source}\nendobj");
    }
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize; usize::try_from(highest).expect("u32 fits usize") + 1];
    for object in body.split_inclusive("endobj\n") {
        let number = object
            .split_once(" 0 obj\n")
            .and_then(|(number, _)| {
                number
                    .rsplit_once('\n')
                    .map_or(Some(number), |(_, n)| Some(n))
            })
            .and_then(|number| number.parse::<usize>().ok())
            .expect("object header is generated above");
        offsets[number] = pdf.len();
        pdf.push_str(object);
    }
    let xref = pdf.len();
    let _ = writeln!(pdf, "xref\n0 {}", offsets.len());
    pdf.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        if *offset == 0 {
            pdf.push_str("0000000000 00000 f \n");
        } else {
            let _ = writeln!(pdf, "{offset:010} 00000 n ");
        }
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    let document = Document::open(pdf.into_bytes()).expect("the complete PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture must use its xref table"
    );
    document
}

/// An omitted Range does not conceal domain clipping behind output clipping.
#[test]
fn exponential_without_range_clips_input_but_preserves_unbounded_output() {
    let document =
        document_with_function("<< /FunctionType 2 /Domain [0 2] /C0 [0 1] /C1 [1 3] /N 2 >>");
    let f = function(&document);
    assert_eq!(f.eval(&[0.5]), vec![0.25, 1.5]);
    assert_eq!(f.eval(&[-4.0]), vec![0.0, 1.0]);
    assert_eq!(f.eval(&[7.0]), vec![4.0, 9.0]);
}

/// Each optional coefficient has its own default even when its partner is explicit.
#[test]
fn exponential_coefficient_defaults_are_independent() {
    for (dictionary, expected) in [
        ("<< /FunctionType 2 /Domain [0 1] /C1 [4] /N 1 >>", 1.0),
        ("<< /FunctionType 2 /Domain [0 1] /C0 [3] /N 1 >>", 2.5),
    ] {
        let document = document_with_function(dictionary);
        assert_eq!(function(&document).eval(&[0.25]), vec![expected]);
    }
}

/// Fractional and negative exponents are evaluated over valid domains rather than
/// accidentally treated as the linear shortcut or rounded to integers.
#[test]
fn valid_fractional_and_negative_exponents_preserve_their_meaning() {
    let square_root = document_with_function("<< /FunctionType 2 /Domain [0 4] /N 0.5 >>");
    assert_eq!(function(&square_root).eval(&[0.25]), vec![0.5]);
    assert_eq!(function(&square_root).eval(&[4.0]), vec![2.0]);
    let reciprocal = document_with_function("<< /FunctionType 2 /Domain [0.25 4] /N -1 >>");
    assert_eq!(function(&reciprocal).eval(&[0.5]), vec![2.0]);
    assert_eq!(function(&reciprocal).eval(&[4.0]), vec![0.25]);
}
