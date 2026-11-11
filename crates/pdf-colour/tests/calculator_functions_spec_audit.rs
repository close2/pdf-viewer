//! Focused regressions for ISO 32000-2 §7.10.5 Type 4 calculator functions.
//!
//! These fixtures use complete one-page PDFs with strict xref tables and exact stream lengths.
//! They exercise `pdf-colour::Function` through its public parser and evaluator.

#![expect(
    clippy::expect_used,
    reason = "small generated PDF fixtures must fail loudly if malformed"
)]

use std::fmt::Write as _;

use pdf_colour::function::{Function, FunctionError};
use pdf_syntax::{Document, Object, ObjectId};

/// Build a complete one-page PDF with an indirect Type 4 function stream at object 5.
fn type4_document(program: &str, domain: &str, range: &str) -> Document {
    let content = b"q Q";
    let function = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [{domain}] /Range [{range}] /Length {} >>\n\
         stream\n{program}\nendstream\nendobj\n",
        program.len()
    );
    let bodies = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_owned(),
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_owned(),
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n"
            .to_owned(),
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content.len(),
            String::from_utf8_lossy(content)
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

    let document = Document::open(pdf.into_bytes()).expect("complete PDF opens");
    assert!(
        !document.was_recovered(),
        "fixture uses its exact xref table"
    );
    document
}

fn parse_function(document: &Document) -> Result<Function, FunctionError> {
    let object = Object::Reference(ObjectId {
        number: 5,
        generation: 0,
    });
    Function::parse(document, &object)
}

/// PostScript `sin` and `cos` use degree arguments, as delegated by §7.10.5.2.
#[test]
fn type4_sine_and_cosine_take_degrees() {
    let document = type4_document("{ pop 30 sin 60 cos }", "0 1", "-1 1 -1 1");
    let function = parse_function(&document).expect("two-output Type 4 function parses");
    let output = function.eval(&[0.5]);
    assert_eq!(output.len(), 2);
    assert!(
        (output[0] - 0.5).abs() < 1.0e-6,
        "sin(30°) = 0.5: {output:?}"
    );
    assert!(
        (output[1] - 0.5).abs() < 1.0e-6,
        "cos(60°) = 0.5: {output:?}"
    );
}

/// Numeric comparison operators follow their inclusive or strict relations at equality.
#[test]
fn type4_numeric_ordering_operators_distinguish_strict_and_inclusive_bounds() {
    let program = "{ pop 2 2 ge { 1 } { 0 } ifelse 2 3 ge { 1 } { 0 } ifelse \
                   3 2 gt { 1 } { 0 } ifelse 2 2 gt { 1 } { 0 } ifelse \
                   2 2 le { 1 } { 0 } ifelse 3 2 le { 1 } { 0 } ifelse \
                   2 3 lt { 1 } { 0 } ifelse 2 2 lt { 1 } { 0 } ifelse }";
    let document = type4_document(program, "0 1", "0 1 ".repeat(8).trim());
    let function = parse_function(&document).expect("eight-output Type 4 function parses");
    assert_eq!(
        function.eval(&[0.5]),
        vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]
    );
}

/// The parser policy refuses syntax outside §7.10.5.1's calculator subset.
#[test]
fn type4_rejects_composite_operands_and_names() {
    let control = type4_document("{ pop 1 true not pop 2.0 }", "0 1", "0 2 0 3");
    let valid = parse_function(&control).expect("scalar integer, boolean, and real are admitted");
    assert_eq!(valid.eval(&[0.5]), vec![1.0, 2.0]);

    for source in ["{ [1] }", "{ (text) }", "{ /variable }", "{ variable }"] {
        let document = type4_document(source, "0 1", "0 1");
        assert!(
            matches!(
                parse_function(&document),
                Err(FunctionError::Malformed { .. })
            ),
            "the calculator subset does not admit {source}"
        );
    }
}

/// §7.10.5.3 guarantees at least 100 operand-stack entries; this reaches the exact floor.
#[test]
fn type4_evaluator_can_use_one_hundred_stack_entries() {
    let domain = "0 10";
    let inputs = [7.0];

    // A lower positive control peaks at 99 entries and isolates fixture construction.
    let below = stack_probe(97);
    let document = type4_document(&below, domain, "0 50");
    let function = parse_function(&document).expect("99-entry control parses");
    assert_eq!(function.eval(&inputs), vec![42.0]);

    // The input-derived value, 98 zeroes, and the index operand reach 100 entries.
    // `index` copies that input-derived value to the top; repeated `exch pop` retains it.
    // The unknown input also keeps this from becoming an all-constant program at parse time.
    let at_floor = stack_probe(98);
    let document = type4_document(&at_floor, domain, "0 50");
    let function = parse_function(&document).expect("100-entry function parses");
    assert_eq!(
        function.eval(&inputs),
        vec![42.0],
        "the valid program with an abstract peak of 100 entries retains its input-derived value"
    );
}
/// Copies an input-derived value from below the zeroes without exceeding the stated peak.
fn stack_probe(zeros: usize) -> String {
    let mut source = String::from("{ 6 mul ");
    for _ in 0..zeros {
        source.push_str("0 ");
    }
    let _ = write!(source, "{zeros} index ");
    for _ in 0..=zeros {
        source.push_str("exch pop ");
    }
    source.push('}');
    source
}
