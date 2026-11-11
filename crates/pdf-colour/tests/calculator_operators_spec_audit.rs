//! Public-path tests for the remaining Type 4 Table 42 operators against PLRM3 §8.2.
//!
//! Complete strict-xref one-page PDFs are built for each evaluation. Inputs vary at runtime so
//! compile-time folding cannot replace the operator path under test.
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
        .expect("valid Type 4 function parses")
        .eval(inputs)
}

/// Convert a Boolean result inside the program: Type 4 outputs themselves must be numbers.
fn evaluate_boolean(program: &str, domain: &str, inputs: &[f32]) -> Vec<f32> {
    let body = program.strip_suffix('}').expect("fixture ends in a brace");
    evaluate(
        &format!("{body} {{ 1 }} {{ 0 }} ifelse }}"),
        domain,
        inputs,
        "-1 2",
    )
}

/// Dynamic numeric input distinguishes inclusive and strict comparisons at their boundary.
#[test]
fn type4_relations_keep_strict_and_inclusive_numeric_boundaries() {
    for (operator, input, expected) in [
        ("ge", 0.0, 1.0),
        ("ge", -0.5, 0.0),
        ("gt", 0.0, 0.0),
        ("gt", 0.5, 1.0),
        ("le", 0.0, 1.0),
        ("le", 0.5, 0.0),
        ("lt", 0.0, 0.0),
        ("lt", -0.5, 1.0),
    ] {
        assert_eq!(
            evaluate_boolean(&format!("{{ 0 {operator} }}"), "-1 1", &[input]),
            vec![expected],
            "{input} {operator} 0"
        );
    }
}

/// Boolean/numeric equality is false, whereas matching values of compatible types compare equal.
#[test]
fn type4_eq_and_ne_preserve_boolean_and_numeric_type_distinctions() {
    for (program, input, expected) in [
        ("{ 0 gt 1 eq }", 0.5, 0.0),
        ("{ 0 gt 1 ne }", 0.5, 1.0),
        ("{ 0 gt true eq }", 0.5, 1.0),
        ("{ 0 gt true ne }", 0.5, 0.0),
        ("{ 0 gt false eq }", -0.5, 1.0),
        ("{ 0 gt false ne }", -0.5, 0.0),
        ("{ 1 eq }", 1.0, 1.0),
        ("{ 1 ne }", 0.5, 1.0),
    ] {
        assert_eq!(
            evaluate_boolean(program, "-1 1", &[input]),
            vec![expected],
            "{program} at {input}"
        );
    }
}

/// Integer operators have different answers from the corresponding Boolean operations.
#[test]
fn type4_bitwise_operators_keep_integer_values() {
    assert_eq!(
        evaluate("{ cvi 3 and }", "0 10", &[6.0], "-10 10"),
        vec![2.0]
    );
    assert_eq!(
        evaluate("{ cvi 3 or }", "0 10", &[6.0], "-10 10"),
        vec![7.0]
    );
    assert_eq!(
        evaluate("{ cvi 3 xor }", "0 10", &[6.0], "-10 10"),
        vec![5.0]
    );
    assert_eq!(
        evaluate("{ cvi not }", "0 100", &[63.0], "-100 100"),
        vec![-64.0]
    );
}

/// Each Boolean operator is exercised at all four input pairs, including XOR's true/true case.
#[test]
fn type4_boolean_operators_follow_their_truth_tables() {
    for (operator, expected) in [
        ("and", [0.0, 0.0, 0.0, 1.0]),
        ("or", [0.0, 1.0, 1.0, 1.0]),
        ("xor", [0.0, 1.0, 1.0, 0.0]),
    ] {
        let program = format!("{{ 0 gt exch 0 gt {operator} }}");
        for (inputs, expected) in [[-1.0, -1.0], [-1.0, 1.0], [1.0, -1.0], [1.0, 1.0]]
            .iter()
            .zip(expected)
        {
            assert_eq!(
                evaluate_boolean(&program, "-1 1 -1 1", inputs),
                vec![expected],
                "{operator} at {inputs:?}"
            );
        }
    }
    assert_eq!(evaluate_boolean("{ 0 gt not }", "-1 1", &[0.5]), vec![0.0]);
    assert_eq!(evaluate_boolean("{ 0 gt not }", "-1 1", &[-0.5]), vec![1.0]);
}

/// Both value and count are runtime inputs; positive values shift in either direction.
#[test]
fn type4_bitshift_uses_dynamic_integer_value_and_count() {
    for (inputs, expected) in [([3.0, 2.0], 12.0), ([15.0, -1.0], 7.0), ([5.0, 0.0], 5.0)] {
        assert_eq!(
            evaluate(
                "{ cvi exch cvi exch bitshift }",
                "0 20 -4 4",
                &inputs,
                "-1 100"
            ),
            vec![expected]
        );
    }
}

/// PLRM3 specifies zero-filled bits, so shifting a negative value right clears its sign bit.
/// This tests that property without prescribing a particular supported integer width.
#[test]
fn type4_right_bitshift_inserts_zero_bits_for_negative_values() {
    let program = "{ cvi -1 bitshift 0 ge }";
    assert_eq!(
        evaluate_boolean(program, "-100 100", &[8.0]),
        vec![1.0],
        "positive control"
    );
    assert_eq!(
        evaluate_boolean(program, "-100 100", &[-8.0]),
        vec![1.0],
        "zero-fill must clear the sign bit"
    );
}

/// Dynamic conditions choose both paths; literal booleans are consumed before numeric output.
#[test]
fn type4_conditionals_select_only_the_matching_expression() {
    let if_program = "{ dup 0 gt { pop 8 } if }";
    assert_eq!(evaluate(if_program, "-1 1", &[0.5], "-1 10"), vec![8.0]);
    assert_eq!(evaluate(if_program, "-1 1", &[-0.5], "-1 10"), vec![-0.5]);
    let ifelse_program = "{ dup 0 gt { pop 8 } { pop 9 } ifelse }";
    assert_eq!(evaluate(ifelse_program, "-1 1", &[0.5], "0 10"), vec![8.0]);
    assert_eq!(evaluate(ifelse_program, "-1 1", &[-0.5], "0 10"), vec![9.0]);
    let literals = "{ 0 gt { true } { false } ifelse }";
    assert_eq!(evaluate_boolean(literals, "-1 1", &[0.5]), vec![1.0]);
    assert_eq!(evaluate_boolean(literals, "-1 1", &[-0.5]), vec![0.0]);
}

/// Distinct runtime inputs make duplication, indexing and the preserved stack prefix observable.
#[test]
fn type4_stack_operators_preserve_their_defined_order() {
    assert_eq!(
        evaluate("{ dup }", "-10 10", &[3.0], "-10 10 -10 10"),
        vec![3.0, 3.0]
    );
    assert_eq!(
        evaluate("{ exch }", "-10 10 -10 10", &[3.0, 7.0], "-10 10 -10 10"),
        vec![7.0, 3.0]
    );
    assert_eq!(
        evaluate("{ pop }", "-10 10 -10 10", &[3.0, 7.0], "-10 10"),
        vec![3.0]
    );
    assert_eq!(
        evaluate(
            "{ 2 copy }",
            "-10 10 -10 10",
            &[3.0, 7.0],
            "-10 10 -10 10 -10 10 -10 10"
        ),
        vec![3.0, 7.0, 3.0, 7.0]
    );
    assert_eq!(
        evaluate("{ 0 copy }", "-10 10 -10 10", &[3.0, 7.0], "-10 10 -10 10"),
        vec![3.0, 7.0]
    );
    assert_eq!(
        evaluate(
            "{ 1 index }",
            "-10 10 -10 10",
            &[3.0, 7.0],
            "-10 10 -10 10 -10 10"
        ),
        vec![3.0, 7.0, 3.0]
    );
    for (count, expected) in [
        (1, [99.0, 7.0, 8.0, 3.0]),
        (-1, [99.0, 3.0, 7.0, 8.0]),
        (4, [99.0, 7.0, 8.0, 3.0]),
        (-4, [99.0, 3.0, 7.0, 8.0]),
        (0, [99.0, 8.0, 3.0, 7.0]),
    ] {
        assert_eq!(
            evaluate(
                &format!("{{ 3 {count} roll }}"),
                "-100 100 -100 100 -100 100 -100 100",
                &[99.0, 8.0, 3.0, 7.0],
                "-100 100 -100 100 -100 100 -100 100"
            ),
            expected.to_vec(),
            "roll count {count}"
        );
    }
}
