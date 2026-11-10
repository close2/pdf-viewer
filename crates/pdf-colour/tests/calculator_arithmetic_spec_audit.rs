//! Public-path regression for Type 4 arithmetic semantics from PLRM3 §8.2.
//!
//! This uses a strict,
//! complete PDF with exact xref offsets and stream lengths.
#![expect(
    clippy::expect_used,
    reason = "small generated PDF fixtures must fail loudly if malformed"
)]

use std::fmt::Write as _;

use pdf_colour::function::Function;
use pdf_syntax::{Document, Object, ObjectId};

fn type4_document(program: &str, domain: &str, range: &str) -> Document {
    let function = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [{domain}] /Range [{range}] /Length {} >>\n\
         stream\n{program}\nendstream\nendobj\n",
        program.len()
    );
    let content = "q Q";
    let bodies = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_owned(),
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_owned(),
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n"
            .to_owned(),
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content.len(),
            content
        ),
        function,
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize];
    for body in &bodies {
        offsets.push(pdf.len());
        pdf.push_str(body);
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
    let document = Document::open(pdf.into_bytes()).expect("strict Type 4 fixture opens");
    assert!(!document.was_recovered(), "xref table is exact");
    document
}

fn evaluate(program: &str, domain: &str, inputs: &[f32], range: &str) -> Vec<f32> {
    let document = type4_document(program, domain, range);
    let object = Object::Reference(ObjectId {
        number: 5,
        generation: 0,
    });
    Function::parse(&document, &object)
        .expect("valid arithmetic Type 4 function parses")
        .eval(inputs)
}

/// PLRM3 §8.2 `add` keeps exact integer arithmetic while the result fits the integer domain.
///
/// The second addition crosses the f32 exact-integer boundary. An evaluator that converts the
/// first sum to f32 loses the unit and returns `16_777_216`, so this assertion discriminates that
/// fault from the typed integer path.
#[test]
fn type4_integer_addition_keeps_exact_values_past_f32_precision() {
    let values = evaluate("{ cvi 16777216 add 1 add }", "0 10", &[1.0], "0 20000000");
    assert_eq!(values, vec![16_777_218.0]);
}

/// PLRM3 §8.2 requires abs and neg of the smallest integer to return a real.
///
/// Wrapping the i32 minimum would return the negative value again, so the positive expected value
/// distinguishes promotion from wraparound through the public evaluator.
#[test]
fn type4_abs_and_neg_promote_the_smallest_integer_to_real() {
    assert_eq!(
        evaluate(
            "{ cvi abs }",
            "-2147483648 0",
            &[-2_147_483_648.0],
            "0 3000000000"
        ),
        vec![2_147_483_648.0]
    );
    assert_eq!(
        evaluate(
            "{ cvi neg }",
            "-2147483648 0",
            &[-2_147_483_648.0],
            "0 3000000000"
        ),
        vec![2_147_483_648.0]
    );
}
