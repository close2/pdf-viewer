//! Where ISO 32000-2 §10.5's transfer function applies, and where §11.7.5.2 says it does not.
//!
//! # Why this file exists
//!
//! The function itself has been implemented since the three-hundred-and-fifty-eighth session and
//! the corpus cannot say anything about it: `examples/transfer_function_census` finds one document
//! in the whole of `doc/pdf.js` that states a `/TR` which is not `/Identity` or `/Default`, and
//! that one draws a single image at full alpha under the Normal blend mode with no mask anywhere.
//! So every rule below is defended by a fixture or by nothing — trap 8 — and the two departures
//! these tests pin were both *silent* until the six-hundred-and-thirty-seventh session.
//!
//! The two are different clauses and are tested apart:
//!
//! - **§11.7.5.2** makes the parameter a property of a *region*. The transfer function at a point
//!   is the topmost object's "but only if the object is fully opaque", and the page's default
//!   otherwise — so a page that states one and paints anything translucent over it is drawn with a
//!   function the clause does not put there. This tree applies each object's own function to its
//!   own colour before compositing, which agrees with the clause exactly on the fully opaque case.
//! - **§10.5** applies the function to every component value on its way to the device, and a
//!   shading's colours reach the backend as a ramp, a mesh or a program rather than as a colour
//!   the interpreter can map.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and these pages are 100 units \
              square where no arithmetic can overflow"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// An inverting §7.10.3 exponential function, which no reader can mistake for the identity.
///
/// `/C0 [1] /C1 [0] /N 1` maps 0 to 1 and 1 to 0. It is one component, which
/// `ext_gstate::Transfer::read` gives to all three of an RGB device's channels — Table 57's
/// "a single function" case rather than its array of four.
const INVERT: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";

/// A one-page fixture: `resources` goes in the page's resource dictionary, `extra` after object 4.
fn fixture(resources: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
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

/// Every [`pdf_model::Unsupported::TransferFunction`] this page raises, in the order they sort.
fn transfer_reports(bytes: Vec<u8>) -> Vec<String> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    interpretation
        .unsupported
        .iter()
        .filter_map(|item| match item {
            pdf_model::Unsupported::TransferFunction { detail } => Some(detail.clone()),
            _ => None,
        })
        .collect()
}

/// A page stating a transfer function and painting one translucent mark under it.
///
/// ISO 32000-2 §11.7.5.2:
///
/// > For portions of the page whose topmost object is not fully opaque or that are never painted
/// > at all, the default halftone and transfer function for the page shall be used
///
/// One stated function and one `ca` below 1.0 is the whole of the condition — no second function
/// competing with a first, which is what the ledger's row claimed for two hundred sessions. The
/// second half of the test is the mutation: the same page painted opaque agrees with the clause
/// exactly, because the six conditions "ensure that only the object itself shall contribute to the
/// colour at the given point".
#[test]
fn a_translucent_mark_under_a_transfer_function_is_reported() {
    let resources = format!("/ExtGState << /Solid << /TR {INVERT} >> /Half << /ca 0.5 >> >>");
    let translucent = transfer_reports(fixture(
        &resources,
        "/Solid gs 1 0 0 rg 0 0 50 50 re f /Half gs 0 0 1 rg 10 10 50 50 re f",
        "",
    ));
    assert_eq!(
        translucent.len(),
        1,
        "one report for the page, not one per mark: {translucent:?}"
    );
    let detail = translucent.first().expect("the report just counted");
    assert!(
        detail.contains("§11.7.5.2") && detail.contains("non-stroking alpha constant is below 1.0"),
        "the report names the clause and the condition that matched: {detail}"
    );

    let opaque = transfer_reports(fixture(
        &resources,
        "/Solid gs 1 0 0 rg 0 0 50 50 re f 0 0 1 rg 10 10 50 50 re f",
        "",
    ));
    assert!(
        opaque.is_empty(),
        "a fully opaque page is what the clause and this tree agree on: {opaque:?}"
    );
}

/// The clause's fifth condition, which a mark inside a group cannot see for itself.
///
/// ISO 32000-2 §11.7.5.2:
///
/// > The foregoing four conditions were also true at the time the Do operator was invoked for the
/// > group containing the object, as well as for any direct ancestor groups.
///
/// §11.6.6 resets the blend mode, both alpha constants and the soft mask before a transparency
/// group's content runs, so the mark below is fully opaque by its own graphics state and is not
/// fully opaque by the clause. A flag reading the mark alone would report nothing here, which is
/// exactly the nested case §11.7.5.2 spends four of its six conditions on.
#[test]
fn a_group_invoked_translucently_carries_its_opacity_to_the_marks_inside() {
    let resources = "/ExtGState << /Half << /ca 0.5 >> >> /XObject << /Fm 5 0 R >>";
    let inner = "/Inner gs 1 0 0 rg 0 0 50 50 re f";
    let form = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceRGB >> \
         /Resources << /ExtGState << /Inner << /TR {INVERT} >> >> >> /Length {} >>\n\
         stream\n{inner}\nendstream\nendobj\n",
        inner.len() + 1
    );

    let translucent = transfer_reports(fixture(resources, "/Half gs /Fm Do", &form));
    assert_eq!(
        translucent.len(),
        1,
        "the ancestry is what makes this one reportable: {translucent:?}"
    );
    let detail = translucent.first().expect("the report just counted");
    assert!(
        detail.contains("§11.7.5.2") && detail.contains("enclosing group's Do"),
        "the report names the condition the ancestry failed: {detail}"
    );

    let opaque = transfer_reports(fixture(resources, "/Fm Do", &form));
    assert!(
        opaque.is_empty(),
        "the same group invoked opaquely satisfies every one of the six: {opaque:?}"
    );
}

/// §10.5 applies to every component value, and a shading's colours never pass through it.
///
/// > In the sequence of steps for processing colours, the PDF processor shall apply the transfer
/// > function after performing any needed conversions between colour spaces.
///
/// A shading reaches the display list as a ramp, a mesh or a sampled program, and the transfer
/// would have to reach each colour inside it. Painting it unmapped is less ink, or more, than the
/// producer asked for and nothing about the page says which — so it is named. The mutation is the
/// same `sh` with no function in force, which is a page this tree draws exactly.
#[test]
fn a_shading_painted_under_a_transfer_function_says_the_colours_missed_it() {
    let shading = "/Shading << /Sh << /ShadingType 2 /ColorSpace /DeviceRGB \
                   /Coords [0 0 100 100] /Function << /FunctionType 2 /Domain [0 1] \
                   /C0 [0 0 0] /C1 [1 1 1] /N 1 >> /Extend [true true] >> >>";
    let stated = transfer_reports(fixture(
        &format!("/ExtGState << /Solid << /TR {INVERT} >> >> {shading}"),
        "/Solid gs /Sh sh",
        "",
    ));
    assert_eq!(
        stated.len(),
        1,
        "one report, for the clause the shading missed: {stated:?}"
    );
    let detail = stated.first().expect("the report just counted");
    assert!(
        detail.contains("§10.5") && detail.contains("shading"),
        "the report names §10.5 rather than §11.7.5.2, which this page does not depart from: \
         {detail}"
    );

    let unstated = transfer_reports(fixture(
        &format!("/ExtGState << /Solid << /TR /Identity >> >> {shading}"),
        "/Solid gs /Sh sh",
        "",
    ));
    assert!(
        unstated.is_empty(),
        "/Identity is Table 57's way of stating no function at all: {unstated:?}"
    );
}
