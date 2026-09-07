//! What a verdict says, and what it says about itself.
//!
//! Two claims, and the second is the one this crate exists to keep. **A document that breaks a
//! rule is told which rule and where** — a verdict naming neither would send its reader back
//! into the file to find out. And **a verdict always says how much of the target it covers**:
//! `doc/questions/Q20`'s discipline is that a requirement this crate does not check is named in
//! the report, so a clean answer is never mistakable for a complete one.
//!
//! Synthetic documents rather than real ones, deliberately. A corpus finds what documents
//! contain; these fixtures are built to contain exactly one fault each, which is the only way to
//! assert that a given requirement is what caught it.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::fmt::Write as _;

use pdf_archive::{Level, Outcome, Target, Verdict, check};
use pdf_syntax::Document;

/// Wraps a body of numbered objects in a header, a cross-reference table and a trailer.
///
/// `trailer` is the extra entries beyond `/Size` and `/Root`, so that a fixture can state the
/// `/Encrypt` and `/ID` the requirements are about.
fn assemble(body: &str, trailer: &str) -> Vec<u8> {
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
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A one-page document, with whatever extra objects and trailer entries a fixture needs.
fn document(extra: &str, trailer: &str) -> Document {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>\nendobj\n{extra}"
    );
    Document::open(assemble(&body, trailer)).expect("the fixture is a valid PDF")
}

/// The identifiers of every requirement a report says was not met.
fn failed(report: &pdf_archive::Report) -> Vec<&'static str> {
    report.failures().map(|judgement| judgement.id).collect()
}

#[test]
fn an_encrypted_document_fails_by_name_at_every_target() {
    let encrypted = document(
        "4 0 obj\n<< /Filter /Standard /V 5 /R 6 /Length 256 >>\nendobj\n",
        "/Encrypt 4 0 R /ID [<00> <11>]",
    );
    for target in Target::ALL {
        let report = check(&encrypted, target);
        assert_eq!(report.verdict(), Verdict::Fails, "{target}");
        assert!(
            failed(&report).contains(&"file-structure/no-encryption"),
            "{target}: the trailer's Encrypt key is what should have caught it, not something else"
        );
    }
}

/// Both parts want a file identifier, and they want it for different reasons.
///
/// **This test asserted the opposite until the file-structure rows were extended**, on the
/// reading that ISO 19005-4 §6.1.3 states no identifier requirement — which is true, and was the
/// wrong conclusion. Part 4's §5.1 makes the whole of ISO 32000-2 binding, and that standard's
/// Table 15 makes `/ID` "Required in PDF 2.0 and later". So the requirement reaches a PDF/A-4
/// file through the base standard, and what differs between the parts is the **citation** rather
/// than the rule. A report that cited §6.1.3 to a part 4 reader would send them to a clause that
/// does not contain it, which is the failure this test now guards.
#[test]
fn the_file_identifier_is_cited_to_each_part_where_that_part_states_it() {
    let no_id = document("", "");
    for (target, clause) in [
        (Target::Two(Level::B), "ISO 19005-2 §6.1.3"),
        (
            Target::Four(pdf_archive::Flavour::Plain),
            "ISO 19005-4 §5.1",
        ),
    ] {
        let report = check(&no_id, target);
        let judgement = report
            .failures()
            .find(|judgement| judgement.id == "file-structure/file-identifier")
            .expect("a trailer with no ID fails the rule at both parts");
        assert_eq!(judgement.citation, clause, "{target}");
    }
}

#[test]
fn a_stream_whose_data_is_outside_the_file_is_found_and_placed() {
    let external = document(
        "4 0 obj\n<< /Length 0 /F (/etc/passwd) >>\nstream\n\nendstream\nendobj\n",
        "/ID [<00> <11>]",
    );
    let report = check(&external, Target::Four(pdf_archive::Flavour::Plain));
    let judgement = report
        .failures()
        .find(|judgement| judgement.id == "file-structure/no-external-stream-data")
        .expect("the F key is what this fixture is for");
    let Outcome::Failed { places, total } = &judgement.outcome else {
        unreachable!("a failure has places")
    };
    assert_eq!(*total, 1);
    assert_eq!(places.len(), 1);
    assert_eq!(
        places[0].place.object.map(|id| id.number),
        Some(4),
        "the report names the object, because a reader has to be able to find it"
    );
}

/// The rule `doc/questions/Q20` asks for, asserted rather than described.
///
/// **Not asserted of a *conforming* document**, deliberately: a three-object fixture is not a
/// PDF/A file — both parts require an XMP packet and an identification schema it does not carry —
/// and an earlier version of this test asserted `Conforms` only because the table was small
/// enough not to notice. What the report owes its reader is the same either way, so the claim is
/// made of a report rather than of a verdict.
#[test]
fn every_report_says_how_much_of_the_target_it_covers() {
    let minimal = document("", "/ID [<00> <11>]");
    let encrypted = document(
        "4 0 obj\n<< /Filter /Standard /V 5 /R 6 >>\nendobj\n",
        "/Encrypt 4 0 R /ID [<00> <11>]",
    );
    for subject in [&minimal, &encrypted] {
        let report = check(subject, Target::Two(Level::B));
        assert!(
            report.unchecked().count() > 0,
            "this crate does not yet check every requirement, and a report that implied it did \
             would be the failure Q20 exists to prevent"
        );
        let rendered = report.render();
        assert!(
            rendered.contains("not checked"),
            "the rendered report names the section even when a reader might not look for it"
        );
        // The denominator is the requirements a *document* can be held to, so the ones that
        // bind a conforming processor come out of it: counting "your processor must ignore the
        // BG function" against a file would make every report understate its own coverage.
        let about_the_file = report
            .judgements
            .len()
            .saturating_sub(report.processor_obligations().count());
        assert!(
            rendered.contains(&format!(
                "{} of {about_the_file} requirements about this file checked",
                report.checked(),
            )),
            "and says how much of the target the verdict covers, next to the verdict"
        );
    }
}

/// Levels are an applicability column, which is `doc/questions/Q46`'s whole argument.
#[test]
fn a_stricter_level_binds_at_least_what_a_looser_one_binds() {
    let clean = document("", "/ID [<00> <11>]");
    let mut previous = 0;
    for level in [Level::B, Level::U, Level::A] {
        let count = check(&clean, Target::Two(level)).judgements.len();
        assert!(
            count >= previous,
            "{level:?} binds fewer requirements than the level below it, which inverts §5.2's \
             containment"
        );
        previous = count;
    }
}
