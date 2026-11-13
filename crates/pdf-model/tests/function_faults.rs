//! ISO 32000-2 §7.10.5.3's run-time errors, said on every route a type 4 function reaches a page by.
//!
//! > It shall be an error for the number of remaining operands to differ from the number of
//! > output variables specified by Range or for any of them to be objects other than numbers.
//!
//! > … it shall be an error to overflow the stack.
//!
//! Each fixture's function is a program that meets one of the three only for some inputs — so
//! the parser's abstract walk cannot refuse it — and each route is asked twice: under an input
//! that errs, where the page must carry `Unsupported::FunctionEvaluation`, and under one that does
//! not (or under the same program written to conform), where it must carry nothing of the kind
//! (trap 11). ADRs 2833, 2848.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly and say what it found instead, \
              and the only arithmetic is the lengths of fixtures a few hundred bytes long"
)]

use std::fmt::Write as _;

use pdf_model::Unsupported;
use pdf_model::function::EvaluationFault;
use pdf_render::{Command, Paint, Transform};
use pdf_syntax::Document;

/// Three outputs for an input up to 0.5 and two above it: §7.10.5.3's count error, above 0.5 only.
const COUNT_ABOVE_HALF: &str = "{ dup 0.5 gt { pop 1 1 } { dup dup } ifelse }";

/// One output, and a boolean in its place up to 0.5: the clause's "objects other than numbers".
const BOOLEAN_BELOW_HALF: &str = "{ 0.5 gt { 1 } { true } ifelse }";

/// The same one-output shape, conforming on every input.
const ONE_OUTPUT: &str = "{ 0.5 gt { 1 } { 0 } ifelse }";

/// Three outputs for an input of 0.5 and above, two below it: the count error, below 0.5 only.
///
/// The tint transform's fixture errs low rather than high because §8.6.6.4 sets a `Separation`
/// space's initial tint to 1.0, which `cs` converts before any `scn` names another.
const COUNT_BELOW_HALF: &str = "{ dup 0.5 lt { pop 1 1 } { dup dup } ifelse }";

/// Pushes past this evaluator's stack only above 0.5, and conforms below it. §7.10.5.3 requires
/// "room for at least 100 entries"; this one has room for a thousand, so the arm pushes 1 001, of which
/// the evaluator keeps the thousand that fit, and pops all of those but one.
fn overflow_above_half() -> String {
    format!(
        "{{ 0.5 gt {{ {} {} }} {{ 1 }} ifelse }}",
        "1 ".repeat(1001),
        "pop ".repeat(999)
    )
}

/// A type 4 function object `number` with `domain`, `range` and `program`.
fn type4(number: u32, domain: &str, range: &str, program: &str) -> String {
    format!(
        "{number} 0 obj\n<< /FunctionType 4 /Domain {domain} /Range {range} /Length {} >>\n\
         stream\n{program}\nendstream\nendobj\n",
        program.len() + 1
    )
}

/// A one-page document: `page` added to the page dictionary, `resources`, `content`, and the
/// further objects `extra`, numbered from 5.
fn fixture(page: &str, resources: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page} \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
        content.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        if object.trim().is_empty() {
            continue;
        }
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn interpreted(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

/// The §7.10.5.3 faults a page's reports name.
fn faults(interpretation: &pdf_model::Interpretation) -> Vec<EvaluationFault> {
    interpretation
        .unsupported
        .iter()
        .filter_map(|report| match report {
            Unsupported::FunctionEvaluation { fault } => Some(*fault),
            _ => None,
        })
        .collect()
}

/// §8.6.6.4's tint transform, evaluated by `scn`: the program errs for a tint below 0.5.
#[test]
fn a_tint_transform_that_errs_for_the_tint_painted_is_said() {
    let page = |tint: &str| {
        fixture(
            "",
            "/ColorSpace << /Spot [/Separation /Spot /DeviceRGB 5 0 R] >>",
            &format!("/Spot cs {tint} scn 0 0 50 50 re f"),
            &type4(5, "[0 1]", "[0 1 0 1 0 1]", COUNT_BELOW_HALF),
        )
    };
    assert_eq!(
        faults(&interpreted(page("0.2"))),
        [EvaluationFault::OutputCount]
    );
    assert_eq!(faults(&interpreted(page("0.9"))), []);
}

/// The same kind of tint transform as an image's colour space (§8.9.5's samples through
/// §8.6.6.4). A one-component image of eight bits or fewer is converted as a table of every level
/// its depth holds (`image::unpack::palette`), so the function is evaluated across its domain
/// whatever samples the image carries, and the control is the program written to conform.
#[test]
fn an_images_tint_transform_that_errs_for_some_levels_is_said() {
    let page = |program: &str| {
        let samples = "1020";
        fixture(
            "",
            "/XObject << /Im 6 0 R >>",
            "q 50 0 0 50 0 0 cm /Im Do Q",
            &format!(
                "{}6 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 1 \
                 /ColorSpace [/Separation /Spot /DeviceRGB 5 0 R] /BitsPerComponent 8 \
                 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{samples}>\nendstream\nendobj\n",
                type4(5, "[0 1]", "[0 1 0 1 0 1]", program),
                samples.len() + 2
            ),
        )
    };
    assert_eq!(
        faults(&interpreted(page(COUNT_ABOVE_HALF))),
        [EvaluationFault::OutputCount]
    );
    assert_eq!(faults(&interpreted(page("{ dup dup }"))), []);
}

/// §10.5's transfer function, set by `gs`: it is sampled over its whole domain where a mark is
/// painted under it, so a program that errs anywhere is said, and the conforming one is not.
#[test]
fn a_transfer_function_that_errs_for_some_levels_is_said() {
    let page = |program: &str| {
        fixture(
            "",
            "/ExtGState << /G << /TR 5 0 R >> >>",
            "/G gs 0.3 g 0 0 50 50 re f",
            &type4(5, "[0 1]", "[0 1]", program),
        )
    };
    assert_eq!(
        faults(&interpreted(page(BOOLEAN_BELOW_HALF))),
        [EvaluationFault::NonNumberOutput]
    );
    assert_eq!(faults(&interpreted(page(ONE_OUTPUT))), []);
}

/// §10.4.2.4's black generation, met where §11.7.5.3 separates a `DeviceRGB` paint into the
/// page's `DeviceCMYK` group: the function is given the colour's black level.
#[test]
fn a_black_generation_that_errs_for_the_level_given_is_said() {
    let page = |program: &str| {
        fixture(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "/ExtGState << /G << /BG 5 0 R >> >>",
            "/G gs 0.1 0.1 0.1 rg 0 0 50 50 re f",
            &type4(5, "[0 1]", "[0 1]", program),
        )
    };
    assert_eq!(
        faults(&interpreted(page(&overflow_above_half()))),
        [EvaluationFault::StackOverflow]
    );
    assert_eq!(faults(&interpreted(page(ONE_OUTPUT))), []);
}

/// §8.7.4.5.5's free-form mesh whose vertices carry §8.7.4.5.5's parametric `t` and whose colour is
/// the `/Function` of it: the mesh's colours are taken from the function across `t`'s range.
#[test]
fn a_mesh_shadings_function_that_errs_across_its_parameter_is_said() {
    let page = |program: &str| {
        // Three vertices, flag 0, at (0,0), (100,0) and (0,100), with t = 0, 0.5 and 1.
        let data = "00000000 00FF0080 0000FFFF";
        fixture(
            "",
            "/Shading << /Sh 6 0 R >>",
            "/Sh sh",
            &format!(
                "{}6 0 obj\n<< /ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 8 \
                 /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 100 0 100 0 1] /Function 5 0 R \
                 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{data}>\nendstream\nendobj\n",
                type4(5, "[0 1]", "[0 1 0 1 0 1]", program),
                data.len() + 2
            ),
        )
    };
    assert_eq!(
        faults(&interpreted(page(COUNT_ABOVE_HALF))),
        [EvaluationFault::OutputCount]
    );
    assert_eq!(faults(&interpreted(page("{ dup dup }"))), []);
}

/// §8.7.4.5.2's type 1 shading is evaluated where a backend asks for its grid, after the
/// interpretation returned; asking the interpretation again after that is what says the fault.
/// The program errs for x above 0.5, which the corner the interpreter samples for opacity is not.
#[test]
fn a_type_1_shading_that_errs_where_it_is_drawn_is_said_after_drawing() {
    let interpretation = interpreted(fixture(
        "",
        "/Shading << /Sh 6 0 R >>",
        "/Sh sh",
        &format!(
            "{}6 0 obj\n<< /ShadingType 1 /ColorSpace /DeviceRGB /Domain [0 1 0 1] \
             /Matrix [100 0 0 100 0 0] /Function 5 0 R >>\nendobj\n",
            type4(
                5,
                "[0 1 0 1]",
                "[0 1 0 1 0 1]",
                "{ pop dup 0.5 gt { pop 1 1 } { dup dup } ifelse }"
            ),
        ),
    ));
    assert_eq!(
        faults(&interpretation),
        [],
        "nothing has drawn the grid yet"
    );
    assert_eq!(interpretation.function_faults_after_drawing(), []);
    let shading = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Fill {
                paint: Paint::Shading(shading),
                ..
            } => Some(shading),
            _ => None,
        })
        .expect("`sh` paints the shading");
    shading
        .sampled_at(Transform::IDENTITY, (100, 100))
        .expect("a type 1 shading is drawn from a grid");
    assert_eq!(
        interpretation.function_faults_after_drawing(),
        [Unsupported::FunctionEvaluation {
            fault: EvaluationFault::OutputCount
        }]
    );
}
