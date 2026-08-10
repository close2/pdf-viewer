//! §7.10.4's stitching function: the k the clause does not bound, and the nesting it does not
//! forbid.
//!
//! Two defects, one clause, both found in the four-hundred-and-twenty-fifth session's `SafeDocs`
//! sample and both in `Function::parse`'s bounds rather than in its arithmetic.
//!
//! # The k that is not a component count
//!
//! Table 41 makes `/Functions` "( Required ) An array of k , 1-input functions that shall make
//! up the stitching function", `/Bounds` "[a]n array of 𝑘 - 1 numbers" and `/Encode` "[a]n
//! array of 2 × 𝑘 numbers". The clause bounds k nowhere, and the only value it singles out is
//! the small one: "The value of k may be 1". A gradient of 256 stops is k = 255, which is
//! ordinary output, and `2750009.pdf` in the sample — corpus `cc-main-2021-31`, archive
//! `2750`, SHA-256 `58b30fccc67721da1d51cec8c5572c15b577fd909b022c63ea04b66492791e6a` — is
//! four shadings of exactly that shape. All four were refused whole, because the constant
//! being applied to k
//! was `MAX_VALUES`, whose own documentation says it bounds a function's *dimensionality*.
//! §7.10.4 settles that from the other side as well: for a type 3 function "Domain shall be of
//! size 2 (that is, 𝑚  =  1 )", so the quantity `MAX_VALUES` bounds is fixed at one here.
//!
//! # The nesting that had no floor
//!
//! A subfunction may itself be a stitching function, so `/Functions` naming its own object is
//! a cycle the parser followed until the stack was gone. The witness below is 720 bytes and
//! aborted every program in this tree — `CLAUDE.md` requires a crasher to become a permanent
//! regression test, and this is it. It is *generated* rather than committed, so it costs the
//! promotion budget nothing.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::function::Function;
use pdf_syntax::{Document, Object, ObjectId};

/// A one-page PDF whose object 5 is a shading and whose objects 6.. are `functions`.
///
/// Each entry of `functions` is written as its own indirect object, so a fixture can point one
/// at another — or at itself, which is the crasher.
fn document_of(functions: &[String]) -> Document {
    let mut body = String::from(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] \
         /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 11 >>\nstream\n/Sh0 sh\n\nendstream\nendobj\n\
         5 0 obj\n<< /ShadingType 2 /ColorSpace /DeviceGray /Coords [0 0 100 0] \
         /Function 6 0 R >>\nendobj\n",
    );
    for (index, function) in functions.iter().enumerate() {
        let number = index.saturating_add(6);
        let _ = write!(body, "{number} 0 obj\n{function}\nendobj\n");
    }

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// Object 6, which every fixture here makes the root function.
fn root() -> Object {
    Object::Reference(ObjectId {
        number: 6,
        generation: 0,
    })
}

/// A type 2 function that is the identity on `[0 1]`.
fn identity() -> String {
    "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>".to_owned()
}

/// **The crasher.** A stitching function whose `/Functions` names its own object.
///
/// Nothing in ISO 32000-2 forbids it — a subfunction may be a stitching function, and §7.3.10
/// makes a reference a reader must follow — so the only answer is a bound, and the bound must
/// produce an error rather than a stack the operating system takes back.
#[test]
fn a_stitching_function_that_names_itself_is_refused_rather_than_followed() {
    let document = document_of(&[
        "<< /FunctionType 3 /Domain [0 1] /Functions [6 0 R] /Bounds [] /Encode [0 1] >>"
            .to_owned(),
    ]);
    let error = Function::parse(&document, &root()).expect_err("a cycle cannot be built");
    assert!(
        format!("{error}").contains("nested deeper than"),
        "the refusal should name the bound it hit, and said {error}"
    );
}

/// Two objects naming each other is the same cycle with a longer period.
#[test]
fn two_stitching_functions_that_name_each_other_are_refused() {
    let document = document_of(&[
        "<< /FunctionType 3 /Domain [0 1] /Functions [7 0 R] /Bounds [] /Encode [0 1] >>"
            .to_owned(),
        "<< /FunctionType 3 /Domain [0 1] /Functions [6 0 R] /Bounds [] /Encode [0 1] >>"
            .to_owned(),
    ]);
    let error = Function::parse(&document, &root()).expect_err("a cycle cannot be built");
    assert!(
        format!("{error}").contains("nested deeper than"),
        "the refusal should name the bound it hit, and said {error}"
    );
}

/// A chain the bound admits is still built, so the guard is a ceiling and not a refusal.
///
/// Eight nested stitching functions with an identity at the bottom: each level wraps the one
/// below it in a k = 1 stitching function, which §7.10.4 allows outright — "The value of k may
/// be 1, in which case the Bounds array shall be empty".
#[test]
fn a_chain_of_stitching_functions_within_the_bound_is_built() {
    let mut functions: Vec<String> = (0..8)
        .map(|level| {
            let below = level + 7;
            format!(
                "<< /FunctionType 3 /Domain [0 1] /Functions [{below} 0 R] /Bounds [] \
                 /Encode [0 1] >>"
            )
        })
        .collect();
    functions.push(identity());
    let document = document_of(&functions);
    let function = Function::parse(&document, &root()).expect("eight levels are within the bound");
    let value = function.eval(&[0.25]);
    assert!(
        value.first().is_some_and(|out| (out - 0.25).abs() < 1e-4),
        "eight identities are the identity, and gave {value:?}"
    );
}

/// The witness's own shape: k = 255, which the clause permits and this tree refused.
///
/// `/Encode` is 2 × k numbers and every pair is `[1 0]`, so each subdomain is *reversed* —
/// §7.10.4's own EXAMPLE 2, "𝑔 (𝑥) =  𝑓 (1  -  𝑥)", applied 255 times. That is what makes this
/// test see the second half of the defect: `/Encode` is read through the same array reader as
/// `/Domain`, so a bound sized for a component count silently replaced a 510-number `/Encode`
/// with the identity, and an identity `/Encode` gives 0 where the file asks for 1.
#[test]
fn a_stitching_function_of_255_subfunctions_is_built_and_its_encode_is_read() {
    const K: usize = 255;

    let mut functions = vec![String::new()];
    let mut names = String::new();
    for index in 0..K {
        let number = index.saturating_add(7);
        let _ = write!(names, "{number} 0 R ");
        functions.push(identity());
    }
    let mut bounds = String::new();
    for index in 1..K {
        #[expect(
            clippy::cast_precision_loss,
            reason = "255 subdomain edges, each exactly representable"
        )]
        let at = index as f32 / K as f32;
        let _ = write!(bounds, "{at} ");
    }
    let encode = "1 0 ".repeat(K);
    functions[0] = format!(
        "<< /FunctionType 3 /Domain [0 1] /Functions [{names}] /Bounds [{bounds}] \
         /Encode [{encode}] >>"
    );

    let document = document_of(&functions);
    let function = Function::parse(&document, &root()).expect("k = 255 is what a gradient is");
    let value = function.eval(&[0.0]);
    assert!(
        value.first().is_some_and(|out| (out - 1.0).abs() < 1e-4),
        "the first subdomain's /Encode is [1 0], so x = 0 maps to f(1) = 1, and gave {value:?}"
    );
}

/// Above the budget the refusal names the count, so a person can see what the file asked for.
#[test]
fn more_subfunctions_than_the_budget_is_refused_by_count() {
    let mut names = String::new();
    for index in 0..5000 {
        let number = index + 7;
        let _ = write!(names, "{number} 0 R ");
    }
    let document = document_of(&[format!(
        "<< /FunctionType 3 /Domain [0 1] /Functions [{names}] /Bounds [] /Encode [0 1] >>"
    )]);
    let error = Function::parse(&document, &root()).expect_err("5000 is above the budget");
    assert!(
        format!("{error}").contains("5000 stitched functions"),
        "the refusal should name the count, and said {error}"
    );
}

/// The arithmetic the budget's documentation states, checked rather than recalled.
///
/// `MAX_FUNCTIONS` is priced in the module as "a `Function` is 120 bytes on this target, so
/// the ceiling is 480 KiB" — a claim about a type, which is the kind that decays.
#[test]
fn the_budgets_price_is_what_the_type_costs() {
    assert_eq!(
        size_of::<Function>(),
        120,
        "the module prices its budget at this size; correct the comment with the number"
    );
}
