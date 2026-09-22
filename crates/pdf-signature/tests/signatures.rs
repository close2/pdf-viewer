//! ISO 32000-2 §12.8's signatures, over the corpus.
//!
//! Eight documents carry one, and the interesting number is not how many but **what their byte
//! ranges cover**. §12.8.1 says a byte range digest "should be the entire PDF file, including the
//! signature dictionary but excluding the signature value itself", so a range that stops short
//! names bytes nobody signed — and measuring that needs no cryptography, only the file's length.
//!
//! This is the whole of what a renderer can say about a signature on its own evidence, and the
//! test exists to keep it honest in both directions: it is *not* a validity verdict, and a
//! document whose range covers everything is not thereby verified.

use std::path::{Path, PathBuf};

use pdf_signature::revision::{Comparison, Judgement};
use pdf_signature::revocation::{Material, Revocation};
use pdf_signature::signature::{
    Authenticity, Coverage, Excluded, Integrity, Modification, Right, Signature, SignedEnd,
    UsageRights, permissions, security_store, signatures, signing_certificate_bindings,
};
use pdf_signature::trust::{Trust, TrustAnchors};
use pdf_signature::x509::Instant;
use pdf_syntax::Document;

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// What a document's `/UR3` grants of the two rights this program can exceed.
///
/// §12.8.2.3's `should` binds a processor that writes, and what it binds is "modif[ying] a PDF …
/// in excess of the rights that are granted". This program fills in a field and saves, so those
/// are the two to ask about — and the point of printing them is trap 11's: the condition is
/// derived from the clause and then *counted*, rather than assumed to have members.
fn granted(name: &str, rights: &UsageRights) -> String {
    format!(
        "{name}: FillIn {}, FullSave {}, /P {}, /V 2.2 {}",
        rights.grants(Right::FillInForm),
        rights.grants(Right::FullSave),
        rights.restrictive,
        rights.version_understood
    )
}

/// Whether filling in a field and saving would exceed what this document grants.
fn exceeded(rights: &UsageRights) -> bool {
    !rights.grants(Right::FillInForm) || !rights.grants(Right::FullSave)
}

/// §12.8.2.3's condition, counted over the corpus rather than assumed to have members.
///
/// > A PDF processor that modifies a PDF, with a UR signature in excess of the rights that are
/// > granted by that signature, should remove that signature prior to writing the newly modified
/// > PDF.
///
/// **It has no members, and that is the finding rather than a gap.** All four documents carrying
/// a `/UR3` grant `/Form /FillIn` and `/Document /FullSave`, which is exactly what this program
/// does to a document, and all four come out `/P false` — two say so, two leave it to Table
/// 258's default — which is the entry that says "any possible restriction may be ignored", so
/// the arrays are not even reached. So `ViewState::save` withdraws no
/// signature on any file this corpus holds, and `usage_rights_are_withdrawn_when_a_save_exceeds
/// _them` in `forms_data.rs` is what exercises the code, on a file written for it. Held at zero
/// here so that a document arriving which *does* trip it announces itself. ADR 0159.
#[test]
fn no_corpus_documents_usage_rights_are_exceeded_by_what_this_program_does() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut stated = Vec::new();
    let mut withdrawn = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let Some(rights) = permissions(&document).usage_rights else {
            continue;
        };
        stated.push(granted(&name, &rights));
        if exceeded(&rights) {
            withdrawn.push(name.to_string());
        }
    }

    println!("§12.8.2.3's usage rights, and what they grant this program:");
    for entry in &stated {
        println!("  {entry}");
    }
    assert_eq!(stated.len(), 4, "documents with a /UR3: {stated:?}");
    assert!(
        withdrawn.is_empty(),
        "no corpus document's usage rights are exceeded by filling in a field and saving: \
         {withdrawn:?}"
    );
}

/// The two places §12.8.2.4 has a writer state which fields a signature covers, counted.
///
/// Table 259's transform parameters "shall be copied from the corresponding fields in the
/// signature field lock dictionary", so a conforming document that states one states both — and
/// this counts each independently, because the interesting document is the one that states only
/// one of them. **This corpus has exactly that**: one document states the `FieldMDP` transform and
/// **none** states a `/Lock` on a signed signature field, so the copy the clause describes is not
/// what a real producer wrote.
///
/// **It is also the finding that this clause's own ledger row was reasoned from a `grep` that
/// lied.** `grep -rl FieldMDP doc/pdf.js/test/pdfs` prints nothing on this machine and `grep
/// -arl` prints `xfa_filled_imm1344e.pdf`: without `-a`, grep classes these files as binary and
/// the `-l` never names them. Every "nothing in the corpus states X" taken that way is a
/// measurement of the instrument. `doc/habits.md`'s *Measuring* section has it.
///
/// So this clause has a real witness and it is the corpus's **one certification signature**,
/// whose `/Reference` array carries two entries — a `DocMDP` and a `FieldMDP` — of which this
/// tree read only the first for its whole life. What the second names is the *signature field* of
/// the recipient rather than a text field, which is the workflow §12.8.2.4's second bullet
/// describes: "after a specific recipient has signed the document, any modifications to specific
/// form fields shall invalidate that recipient's signature".
#[test]
fn the_corpus_states_the_fields_one_signature_covers() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut locks = Vec::new();
    let mut transforms = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !pdf_signature::signature::field_locks(&document).is_empty() {
            locks.push(name.to_string());
        }
        let covered = pdf_signature::signature::field_mdp(&document);
        if !covered.is_empty() {
            transforms.push(format!("{name}: {covered:?}"));
        }
    }

    println!("§12.7.5.5 /Lock on a signed field: {}", locks.len());
    for entry in &transforms {
        println!("§12.8.2.4 FieldMDP: {entry}");
    }
    assert!(locks.is_empty(), "documents with a signed /Lock: {locks:?}");
    assert_eq!(
        transforms,
        vec![
            "xfa_filled_imm1344e.pdf: [FieldMdp { selection: Include([\"form1[0].SignatureField3\
             [0]\"]), data: Some(ObjectId { number: 1, generation: 0 }) }]"
                .to_owned(),
        ],
        "the corpus's FieldMDP transforms, with the object Table 256's /Data scopes each to"
    );
}

/// Every corpus signature, with what its range covers and what its document permits.
#[test]
fn every_signed_corpus_document_says_what_it_signed() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut signed = Vec::new();
    let mut certifications = 0usize;
    let mut with_permissions = Vec::new();
    let mut whole_file = 0usize;
    let mut unsigned_tails = Vec::new();
    let mut malformed = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let length = bytes.len() as u64;
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let found = signatures(&document);
        let perms = permissions(&document);
        if perms.doc_mdp.is_some() || perms.usage_rights.is_some() {
            with_permissions.push(format!("{name}: {perms:?}"));
        }
        if found.is_empty() {
            continue;
        }
        signed.push(format!(
            "{name}: {} signature(s), {:?}",
            found.len(),
            found
                .iter()
                .map(|signature| signature.sub_filter.clone().unwrap_or_default())
                .collect::<Vec<_>>()
        ));
        for signature in &found {
            certifications = certifications.saturating_add(usize::from(signature.certification));
            match signature.coverage(length) {
                Coverage::WholeFile => whole_file = whole_file.saturating_add(1),
                Coverage::Unsigned { tail } => {
                    unsigned_tails.push(format!("{name}: {tail} bytes after the signed range"));
                }
                Coverage::Malformed => malformed.push(format!(
                    "{name}: {:?} against {length} bytes",
                    signature.byte_range
                )),
            }
        }
    }

    println!("{} documents carry a signature:", signed.len());
    for entry in &signed {
        println!("  {entry}");
    }
    println!("  {certifications} of them are certification signatures (a DocMDP transform)");
    println!("  {whole_file} ranges cover the whole file");
    println!("  ranges with an unsigned tail: {unsigned_tails:?}");
    println!("  ranges that do not describe this file: {malformed:?}");
    println!("documents stating §12.8.6 permissions: {with_permissions:?}");
    assert_eq!(
        signed.len(),
        6,
        "documents with a signature in a signature field"
    );
    assert!(
        malformed.is_empty(),
        "every corpus signature's range describes its own file: {malformed:?}"
    );
    assert_eq!(whole_file, 4, "ranges running to the end of their file");

    // The two with an unsigned tail, and the second is §12.8.2.2 demonstrated by a real file.
    // `xfa_filled_imm1344e.pdf` is the corpus's one certification signature: its `/Perms`
    // `/DocMDP` states `/P 2`, which "permit[s] modifications that are appropriate for form
    // field or comment workflows", and 2.5 MB were appended after it was signed — a filled-in
    // form, saved by incremental update exactly as §12.8.1's NOTE 1 describes. Whether those
    // bytes contain *only* permitted changes is §12.8.2.2.2's question and needs the digest and
    // a comparison of two revisions; what this reader can say is that they are there.
    assert_eq!(
        unsigned_tails.len(),
        2,
        "signatures with bytes after them: {unsigned_tails:?}"
    );
    assert!(
        unsigned_tails
            .iter()
            .any(|entry| entry.starts_with("xfa_filled_imm1344e.pdf: 2542822")),
        "{unsigned_tails:?}"
    );
    assert_eq!(
        certifications, 1,
        "certification signatures, which §12.8.1 permits at most one of per document"
    );
    assert_eq!(
        with_permissions.len(),
        4,
        "documents stating §12.8.6 permissions: {with_permissions:?}"
    );
}

/// Every signature dictionary one document holds, from both places §12.8.1 puts one.
///
/// A signature reached from `/Perms` is the same object as a field's whenever a field points at
/// it, so the certification signature of a document that states both is returned once.
/// `Signature` compares by value, which is what makes that de-duplication exact.
fn every_signature(document: &Document) -> Vec<Signature> {
    let permissions = permissions(document);
    let mut found = signatures(document);
    for extra in [
        permissions.usage_rights_signature,
        permissions.doc_mdp_signature,
    ]
    .into_iter()
    .flatten()
    {
        if !found.contains(&extra) {
            found.push(extra);
        }
    }
    found
}

/// Every signature dictionary in the corpus, asked whether its document changed after signing.
///
/// **This is the round's measurement** (ADR 0215) and it covers more signatures than the test
/// above: §12.8.1 puts a usage rights signature's dictionary in the permissions dictionary "(not
/// from a signature field)", so `signatures` cannot reach one, and three corpus documents carry
/// nothing else.
///
/// **Four of the ten come out `Changed`, and that is the finding rather than a failure.** Each is
/// a file whose bytes no longer hash to what its own signature records, which §12.8.1 says
/// "indicates that modifications have been made since the document was signed":
///
/// - `issue6127.pdf` and both of `xfa_filled_imm1344e.pdf`'s signatures were re-saved rather than
///   incrementally updated. Their `/ByteRange` no longer even brackets their own `/Contents` — the
///   gap it names falls in the middle of the hexadecimal string in one and inside the XFA packet
///   in the other — so the bytes before the signature moved, which is a rewrite and not §7.5.6's
///   append. The gap's *size* still matches the signature value's to the byte in both, which is
///   what says these were once correct.
/// - `poppler-395-0-fuzzed.pdf` is a fuzzed file and is expected to fail everything.
///
/// The counts are held so that a change announces itself in either direction.
#[test]
fn every_corpus_signature_is_asked_whether_its_document_changed() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut lines = Vec::new();
    let mut documents = 0usize;
    let mut unchanged = 0usize;
    let mut changed = Vec::new();
    let mut under_the_key = 0usize;
    let mut unreadable = Vec::new();
    let mut algorithms = std::collections::BTreeMap::<&'static str, usize>::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let file = document.bytes().clone();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let found = every_signature(&document);
        if found.is_empty() {
            continue;
        }
        documents = documents.saturating_add(1);
        for signature in &found {
            let integrity = signature.integrity(&file);
            match integrity {
                Integrity::Unchanged { digest } => {
                    unchanged = unchanged.saturating_add(1);
                    *algorithms.entry(digest.name()).or_default() += 1;
                }
                Integrity::Changed { digest } => {
                    changed.push(format!("{name} ({})", digest.name()));
                }
                Integrity::UnderTheSignersKey => under_the_key = under_the_key.saturating_add(1),
                other => unreadable.push(format!("{name}: {other:?}")),
            }
            lines.push(format!(
                "  {name}: {} /{}, {:?}, {:?}",
                if signature.timestamp {
                    "timestamp"
                } else {
                    "signature"
                },
                signature.sub_filter.as_deref().unwrap_or("(no SubFilter)"),
                signature.coverage(file.len() as u64),
                integrity
            ));
        }
    }

    lines.sort();
    println!("{documents} corpus documents carry a signature dictionary:");
    for line in &lines {
        println!("{line}");
    }
    println!("  digest recomputed and unchanged: {unchanged} {algorithms:?}");
    println!("  digest recomputed and CHANGED:   {changed:?}");
    println!("  digest under the signer's key:   {under_the_key}");
    println!("  signature values not readable:   {unreadable:?}");

    assert_eq!(documents, 9, "documents carrying a signature dictionary");
    assert_eq!(lines.len(), 10, "signature dictionaries");
    assert_eq!(
        unchanged, 5,
        "signatures whose signed bytes still hash to what they record"
    );
    changed.sort();
    assert_eq!(
        changed,
        [
            "issue6127.pdf (SHA256)",
            "poppler-395-0-fuzzed.pdf (SHA1)",
            "xfa_filled_imm1344e.pdf (SHA1)",
            "xfa_filled_imm1344e.pdf (SHA256)",
        ],
        "signed bytes that no longer hash to what the signature records"
    );
    // `bug854315.pdf`, whose `SignerInfo` carries no signed attributes at all. RFC 5652 then
    // signs the content directly, so no `message-digest` records the document's digest in the
    // clear and question 1 cannot be answered without question 2 — which is the shape this
    // round's separation of the three exists to be able to say out loud.
    assert_eq!(
        under_the_key, 1,
        "signatures recording no digest in the clear"
    );
    assert!(
        unreadable.is_empty(),
        "signature values that could not be read: {unreadable:?}"
    );

    // Table 260's algorithms, counted: the corpus uses two of the ten — the base standard's six
    // and ISO/TS 32001's four — which is why the other eight are checked against published vectors
    // in `cms.rs` rather than by a document.
    assert_eq!(
        algorithms.get("SHA1").copied().unwrap_or_default(),
        2,
        "{algorithms:?}"
    );
    assert_eq!(
        algorithms.get("SHA256").copied().unwrap_or_default(),
        3,
        "{algorithms:?}"
    );
}

/// **What every corpus signature's `/ByteRange` leaves out, and where it stops.**
///
/// §12.8.1 names one thing a signed range may skip — "the signature value itself (the Contents
/// entry)" — and one place it may stop: "the end of the \"%%EOF\" comment, possibly followed by
/// an optional EOL marker". Neither is arithmetic over the pairs, which is why
/// `Signature::coverage` cannot see either and why this walk exists: it reads the region off the
/// file and decodes it as §7.3.4.3's hexadecimal string.
///
/// **The two questions have the same four answers, and that was not put in by hand.** The four
/// signatures this names — both of `xfa_filled_imm1344e.pdf`'s, `issue6127.pdf`'s and the fuzzed
/// file's — are exactly the four that `every_corpus_signature_is_asked_whether_its_document_changed`
/// finds `Changed`, and exactly the four whose range stops somewhere other than an `%%EOF`. That
/// test's prose already said of the first three that their "`/ByteRange` no longer even brackets
/// their own `/Contents`"; nothing checked it until now, and what the check adds is that the same
/// four fail *both* of §12.8.1's structural rules, which is a stronger statement about a re-saved
/// file than either alone. The other six leave out their value and nothing else and stop at a
/// marker — five of them writing §12.8.3.3.1's string whole into the hole, and `signed_verified.pdf`
/// signing its own delimiters, which is the third answer and not a failure of the first. Held by
/// name in both directions; a corpus that is not checked out says so rather than passing.
#[test]
fn every_corpus_signature_says_what_its_range_leaves_out_and_where_it_stops() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut lines = Vec::new();
    let mut value_only = 0usize;
    let mut digits_only = Vec::new();
    let mut not_the_value = Vec::new();
    let mut at_a_marker = 0usize;
    let mut elsewhere = Vec::new();
    let mut indirect = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let file = document.bytes().clone();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in &every_signature(&document) {
            let excluded = signature.excluded(&file);
            let end = signature.signed_end(&file);
            match excluded {
                Excluded::TheSignatureValue => value_only = value_only.saturating_add(1),
                // §12.8.3.3.1's "shall fit precisely": the region holds the value's octets and
                // nothing else, and a delimiter is inside the signed range rather than the hole.
                Excluded::TheDigitsOfTheSignatureValue => digits_only.push(name.to_string()),
                other => not_the_value.push(format!("{name} ({other:?})")),
            }
            if end == SignedEnd::AtAnEndOfFileMarker {
                at_a_marker = at_a_marker.saturating_add(1);
            } else {
                elsewhere.push(format!("{name} ({end:?})"));
            }
            // §12.8.1: "When a byte range digest is present, all values in the signature
            // dictionary shall be direct objects." The condition is the clause's and is applied
            // here rather than in the reader, which records the fact and not the departure.
            if !signature.byte_range.is_empty() && !signature.indirect_values.is_empty() {
                indirect.push(format!("{name} {:?}", signature.indirect_values));
            }
            lines.push(format!("  {name}: {excluded:?}, ends {end:?}"));
        }
    }

    lines.sort();
    for line in &lines {
        println!("{line}");
    }
    println!("  leave out their value, delimiters and all: {value_only}");
    println!("  leave out its digits, delimiters signed:  {digits_only:?}");
    println!("  leave out something else:               {not_the_value:?}");
    println!("  stop at an %%EOF marker:                {at_a_marker}");
    println!("  stop elsewhere:                         {elsewhere:?}");
    println!("  dictionaries with an indirect value:    {indirect:?}");

    assert_eq!(
        value_only, 5,
        "signatures whose range leaves out §12.8.3.3.1's string, delimiters and all"
    );
    digits_only.sort();
    assert_eq!(
        digits_only,
        ["signed_verified.pdf"],
        "signatures whose range leaves out the digits alone, signing their delimiters"
    );
    not_the_value.sort();
    assert_eq!(
        not_the_value,
        [
            "issue6127.pdf (NotTheSignatureValue { at: 1529, length: 12400 })",
            "poppler-395-0-fuzzed.pdf (NotTheSignatureValue { at: 9001, length: 4244 })",
            "xfa_filled_imm1344e.pdf (NotTheSignatureValue { at: 15651, length: 32578 })",
            "xfa_filled_imm1344e.pdf (NotTheSignatureValue { at: 568130, length: 10118 })",
        ],
        "ranges leaving out something other than their own signature value"
    );
    elsewhere.sort();
    assert_eq!(
        elsewhere,
        [
            "issue6127.pdf (Elsewhere)",
            "poppler-395-0-fuzzed.pdf (Elsewhere)",
            "xfa_filled_imm1344e.pdf (Elsewhere)",
            "xfa_filled_imm1344e.pdf (Elsewhere)",
        ],
        "ranges not stopping at an end-of-file marker"
    );
    assert!(
        indirect.is_empty(),
        "signature dictionaries breaking \u{a7}12.8.1's direct-objects rule: {indirect:?}"
    );
}

/// Table 257's answer for every corpus signature whose signed revision can be reconstructed.
///
/// Held rather than printed, because the ranking is the round's claim: six signatures have
/// nothing to rank, and `prefilled_f1040.pdf`'s fifteen changed objects are each named by what
/// they are. Its `/P FormFilling` line is the one that matters — nine objects Table 257's default
/// level does not permit, every one of them an annotation modified rather than a field filled in.
const TABLE_257S_ANSWERS: &[&str] = &[
    "160F-2019.pdf /P FormFilling: NoChangeToRank",
    "160F-2019.pdf /P FormFillingAndAnnotation: NoChangeToRank",
    "160F-2019.pdf /P None: NoChangeToRank",
    "bug854315.pdf /P FormFilling: NoChangeToRank",
    "bug854315.pdf /P FormFillingAndAnnotation: NoChangeToRank",
    "bug854315.pdf /P None: NoChangeToRank",
    "issue16553.pdf /P FormFilling: NoChangeToRank",
    "issue16553.pdf /P FormFillingAndAnnotation: NoChangeToRank",
    "issue16553.pdf /P None: NoChangeToRank",
    "issue17069.pdf /P FormFilling: NoChangeToRank",
    "issue17069.pdf /P FormFillingAndAnnotation: NoChangeToRank",
    "issue17069.pdf /P None: NoChangeToRank",
    "prefilled_f1040.pdf /P FormFilling: NotPermitted { level: FormFilling, objects: 9 }",
    "prefilled_f1040.pdf /P FormFillingAndAnnotation: WithinWhatIsPermitted { level: \
             FormFillingAndAnnotation, objects: 15, disregarded: 0 }",
    "prefilled_f1040.pdf /P None: NotPermitted { level: None, objects: 12 }",
    "prefilled_f1040.pdf object 1574: Annotation",
    "prefilled_f1040.pdf object 1575: Annotation",
    "prefilled_f1040.pdf object 1576: Annotation",
    "prefilled_f1040.pdf object 1592: Annotation",
    "prefilled_f1040.pdf object 1596: Annotation",
    "prefilled_f1040.pdf object 1600: Annotation",
    "prefilled_f1040.pdf object 1790: AnnotationAppearance",
    "prefilled_f1040.pdf object 1791: AnnotationAppearance",
    "prefilled_f1040.pdf object 1792: AnnotationAppearance",
    "prefilled_f1040.pdf object 1793: FieldAppearance",
    "prefilled_f1040.pdf object 2008: CrossReferenceStream",
    "prefilled_f1040.pdf object 2009: CrossReferenceStream",
    "prefilled_f1040.pdf object 200: FieldFilledIn",
    "prefilled_f1040.pdf object 2010: CrossReferenceStream",
    "prefilled_f1040.pdf object 206: FieldFilledIn",
    "signed_verified.pdf /P FormFilling: NoChangeToRank",
    "signed_verified.pdf /P FormFillingAndAnnotation: NoChangeToRank",
    "signed_verified.pdf /P None: NoChangeToRank",
];

/// Table 257's answer at each of its three levels, and what each changed object was taken to be.
///
/// The arithmetic check is the load-bearing one: every object the comparison found is in exactly
/// one of the ranking's four buckets, which is what makes "permitted" a statement about all of
/// them rather than about the ones this reader happened to look at.
fn rankings(name: &str, comparison: &Comparison, level: Modification) -> Vec<String> {
    let changes = comparison.changes();
    let ranking = comparison.rank(level);
    let ranked = ranking
        .disregarded
        .count()
        .saturating_add(ranking.permitted.count())
        .saturating_add(ranking.not_permitted.count())
        .saturating_add(ranking.unrankable.count());
    let found = changes
        .added
        .count()
        .saturating_add(changes.redefined.count())
        .saturating_add(changes.removed.count())
        .saturating_add(changes.unplaceable.count());
    assert!(ranked >= found, "{name}: the ranking lost an object");
    // A moved `/Root` is the one entry the ranking can hold that no object bucket does, and it is
    // one: it names the catalog the current trailer points at, which either is a changed object
    // already or is this single extra.
    assert!(
        ranked <= found.saturating_add(u64::from(changes.catalog_moved)),
        "{name}: the ranking holds an object the comparison did not find"
    );
    if ranking.judgement() == Judgement::NoChangeToRank {
        assert!(
            changes.is_empty(),
            "{name}: nothing to rank and something changed: {changes:?}"
        );
    }
    let mut lines: Vec<String> = [
        Modification::None,
        Modification::FormFilling,
        Modification::FormFillingAndAnnotation,
    ]
    .into_iter()
    .map(|level| {
        format!(
            "{name} /P {level:?}: {:?}",
            comparison.rank(level).judgement()
        )
    })
    .collect();
    for object in &ranking.detail {
        lines.push(format!(
            "{name} object {}: {:?}",
            object.number, object.kind
        ));
    }
    lines
}

/// §12.8.2.2.2's second step over the corpus: whose signed revision can be reconstructed, what
/// changed after it, and what Table 257 says about each change.
///
/// > Therefore, PDF processors may compare the signed and current versions of the document to see
/// > whether there have been modifications to any objects that are not permitted by the transform
/// > parameters.
///
/// **Four of the ten refuse, and the refusals are the point.** A comparison is only worth making
/// where the prefix the signature signed is a state the document was actually in — `Excluded` and
/// `SignedEnd` are the two checks, and the four signatures that fail either of them are the four
/// this tree already reports as leaving out something other than their own value. Nothing is
/// compared for those, by name.
///
/// The six that can be compared are the finding, and `prefilled_f1040.pdf` is the one with
/// anything to rank: **three incremental updates carrying fifteen changed objects, every one of
/// them classified and none refused.** Two widgets were filled in, six were filled in *and* had
/// their rectangles moved by a thousandth of a unit, four appearance streams came with them, and
/// three are the updates' own cross-reference streams. So Table 257 answers differently at each
/// level — forbidden at 1, forbidden at 2 because a moved rectangle is an annotation modified,
/// and permitted at 3 — which is the distinction the table's two workflows exist to draw. The
/// level asked here is the author's where there is one and Table 257's default of 2 where there
/// is not; this file carries a `/UR3` and no `/DocMDP`, so the question put to it is the
/// counterfactual one. ADR 1049.
#[test]
fn every_corpus_signature_is_asked_what_changed_after_it() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut compared = Vec::new();
    let mut refused = Vec::new();
    let mut ranked = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in &every_signature(&document) {
            match Comparison::of(signature, &document) {
                Ok(comparison) => {
                    let changes = comparison.changes();
                    compared.push(format!(
                        "{name}: ends at {}, {} update(s) after it, +{} ~{} -{} ?{}{}",
                        comparison.end(),
                        comparison.updates_after(),
                        changes.added.count(),
                        changes.redefined.count(),
                        changes.removed.count(),
                        changes.unplaceable.count(),
                        if changes.catalog_moved {
                            ", catalog moved"
                        } else {
                            ""
                        }
                    ));
                    // The level is the author's where there is one, and Table 257's default of 2
                    // where a signature carries a transform with no `/P`.
                    let level = permissions(&document)
                        .doc_mdp
                        .unwrap_or(Modification::FormFilling);
                    ranked.extend(rankings(&name, &comparison, level));
                }
                Err(refusal) => refused.push(format!("{name}: {refusal}")),
            }
        }
    }

    compared.sort();
    refused.sort();
    ranked.sort();
    for line in &compared {
        println!("  compared  {line}");
    }
    for line in &refused {
        println!("  refused   {line}");
    }
    for line in &ranked {
        println!("  ranked    {line}");
    }

    assert_eq!(
        refused,
        [
            "issue6127.pdf: the signed range leaves out NotTheSignatureValue { at: 1529, \
             length: 12400 } rather than the signature value alone",
            "poppler-395-0-fuzzed.pdf: the signed range leaves out NotTheSignatureValue { at: \
             9001, length: 4244 } rather than the signature value alone",
            "xfa_filled_imm1344e.pdf: the signed range leaves out NotTheSignatureValue { at: \
             15651, length: 32578 } rather than the signature value alone",
            "xfa_filled_imm1344e.pdf: the signed range leaves out NotTheSignatureValue { at: \
             568130, length: 10118 } rather than the signature value alone",
        ],
        "signatures whose signed bytes are not a revision, and are therefore not compared"
    );
    assert_eq!(
        ranked, TABLE_257S_ANSWERS,
        "Table 257's ranking of what each comparable signature's document did after it"
    );
    assert_eq!(
        compared,
        [
            "160F-2019.pdf: ends at 328400, 0 update(s) after it, +0 ~0 -0 ?0",
            "bug854315.pdf: ends at 107532, 0 update(s) after it, +0 ~0 -0 ?0",
            "issue16553.pdf: ends at 718356, 0 update(s) after it, +0 ~0 -0 ?0",
            "issue17069.pdf: ends at 706317, 0 update(s) after it, +0 ~0 -0 ?0",
            "prefilled_f1040.pdf: ends at 299340, 3 update(s) after it, +7 ~8 -0 ?0",
            "signed_verified.pdf: ends at 10252, 0 update(s) after it, +0 ~0 -0 ?0",
        ],
        "the signed revisions this corpus lets be reconstructed, and what changed after each"
    );
}

/// Every signature in the corpus answers both questions the same through a file on disk.
///
/// The digest over `/ByteRange` is fed a window at a time off the disk since ADR 0812, where it
/// used to be computed over slices of the whole file held in memory; this is the check that the
/// windows add up to the range on every real signature this tree holds — ten dictionaries in
/// nine documents, two digest algorithms, four constructions — with the answers compared exact
/// rather than counted. A corpus that is not checked out says so.
#[test]
fn every_corpus_signature_answers_the_same_on_disk() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut compared = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(in_memory) = Document::open(bytes) else {
            continue;
        };
        let found = every_signature(&in_memory);
        if found.is_empty() {
            continue;
        }
        let on_disk = Document::open(pdf_syntax::FileBytes::on_disk(path).expect("opens"))
            .expect("a document that opened in memory opens on disk");
        assert!(on_disk.bytes().is_on_disk());
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in &found {
            assert_eq!(
                signature.integrity(on_disk.bytes()),
                signature.integrity(in_memory.bytes()),
                "{name}: the two routes disagree about whether the document changed"
            );
            assert_eq!(
                signature.authenticity(on_disk.bytes()),
                signature.authenticity(in_memory.bytes()),
                "{name}: the two routes disagree about whether the signature verifies"
            );
            compared += 1;
        }
    }
    println!("{compared} corpus signatures answer the same on disk as in memory");
    assert_eq!(compared, 10, "signature dictionaries compared both ways");
}

/// Every signature dictionary in the corpus, asked whether it verifies under the signer's key.
///
/// **This is the three-hundred-and-ninety-second session's measurement** (ADR 0229), and it is
/// §12.8.1's second question. It is printed beside the first because neither answer means much
/// alone and the pairing is the whole point: `Signed` records, per signature, whether verifying it
/// binds the document directly or only through the digest question 1 compares.
///
/// **All ten verify, and four of them are the interesting ones.** `issue6127.pdf`,
/// `poppler-395-0-fuzzed.pdf` and both of `xfa_filled_imm1344e.pdf`'s signatures answer `Changed`
/// to question 1 and `Verified` to question 2 — an authentic signature over attributes that record
/// a digest the file's own bytes no longer produce. That is a stronger statement than either
/// answer alone: these are not broken signatures, they are real signatures whose documents were
/// re-saved out from under them.
///
/// And `bug854315.pdf` is the one question 2 answers *for* question 1. Its `SignerInfo` states no
/// signed attributes, so RFC 5652 signs the content itself — which for a detached signature is the
/// byte range — and `Integrity::UnderTheSignersKey` was this program saying so. The key it needed
/// was in the file all along.
#[test]
fn every_corpus_signature_is_asked_whether_it_verifies() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut lines = Vec::new();
    let mut verified = Vec::new();
    let mut failed = Vec::new();
    let mut other = Vec::new();
    let mut binds_the_document = 0usize;
    let mut widths = std::collections::BTreeMap::<usize, usize>::new();
    let mut discriminates = 0usize;
    // What answering question 2 costs, summed over the ten. It is not a gate — a wall clock on a
    // shared machine is not one — but it is the number a reader wants, because this is the first
    // thing in the tree that runs a modular exponentiation on the document's own thread.
    let mut elapsed = std::time::Duration::ZERO;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let file = document.bytes().clone();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in &every_signature(&document) {
            let started = std::time::Instant::now();
            let authenticity = signature.authenticity(&file);
            elapsed = elapsed.saturating_add(started.elapsed());
            match &authenticity {
                Authenticity::Verified { key_bits, over, .. } => {
                    verified.push(format!("{name} ({key_bits}-bit, over {over:?})"));
                    *widths.entry(*key_bits).or_default() += 1;
                    if over.binds_the_document() {
                        binds_the_document = binds_the_document.saturating_add(1);
                    }
                    // **The instrument has to be able to say no.** A measurement over ten
                    // documents that all pass is exactly the shape a verifier stuck at `true`
                    // would produce, so each one is asked again with one bit of its signature
                    // value turned over. ADR 0229.
                    if !flipped(signature).authenticity(&file).eq(&authenticity) {
                        discriminates = discriminates.saturating_add(1);
                    }
                }
                Authenticity::NotUnderThatKey { key_bits, .. } => {
                    failed.push(format!("{name} ({key_bits}-bit)"));
                }
                rest => other.push(format!("{name}: {rest:?}")),
            }
            lines.push(format!(
                "  {name}: {:?} / {:?}",
                signature.integrity(&file),
                authenticity
            ));
        }
    }

    lines.sort();
    println!("question 2 over the corpus's ten signature dictionaries:");
    for line in &lines {
        println!("{line}");
    }
    verified.sort();
    failed.sort();
    other.sort();
    println!(
        "  verifies under the signer's certificate: {}",
        verified.len()
    );
    for entry in &verified {
        println!("    {entry}");
    }
    println!("  does NOT verify:              {failed:?}");
    println!("  question 2 not answered:      {other:?}");
    println!("  key widths:                   {widths:?}");
    println!("  of those, verifying binds the document's own bytes: {binds_the_document}");
    println!("  and with one bit of the signature turned over, no longer verify: {discriminates}");
    println!(
        "  all ten answered in {:.3} ms of processor time between them",
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(lines.len(), 10, "signature dictionaries");
    assert_eq!(
        verified.len(),
        10,
        "signatures verifying under the key in a certificate the file itself carries"
    );
    assert!(
        failed.is_empty(),
        "signatures that do not verify: {failed:?}"
    );
    assert!(other.is_empty(), "question 2 unanswered: {other:?}");
    assert_eq!(
        discriminates, 10,
        "every one of them stops verifying when one bit of it is changed"
    );
    // `bug854315.pdf`, the one whose signature is over the document's bytes directly.
    assert_eq!(
        binds_the_document, 1,
        "signatures with no signed attributes"
    );
    // Table 260 caps RSA at 4096 bits and the corpus spans three of the sizes it names.
    assert_eq!(widths.get(&1024).copied().unwrap_or_default(), 3);
    assert_eq!(widths.get(&2048).copied().unwrap_or_default(), 6);
    assert_eq!(widths.get(&4096).copied().unwrap_or_default(), 1);
}

/// §12.8.3.4.5 (a)'s first sentence over the corpus, and the one document that exercises it.
///
/// > A signature handler shall compare the hash value of the signer's certificate, with the hash
/// > value given in the signing-certificate attribute or the signing-certificate-v2 attribute. If
/// > the hashes do not match, then the signature is considered invalid.
///
/// **One of the ten states such an attribute, and it is `issue16553.pdf`** — a real file, a real
/// certificate, and RFC 5035 section 5.4.1's `SigningCertificateV2` under its `DEFAULT` SHA-256.
/// So this rule has a witness in the world and not only in a fixture, which is what trap 8 asks a
/// round to establish before believing a clean column.
///
/// **The count is asserted from both ends on purpose.** "None of them Differs" is the answer a
/// comparison that never ran would also give, so the number that *did* run is asserted too; and
/// the file is named because the population is one, and one is the size at which a silent
/// regression looks exactly like a corpus that moved.
#[test]
fn every_corpus_signature_is_asked_whether_it_names_the_certificate_it_used() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut answers = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in &every_signature(&document) {
            let Ok(cms) = signature.signed_data() else {
                continue;
            };
            for binding in signing_certificate_bindings(&cms) {
                answers.push(format!("{name}: {binding:?}"));
            }
        }
    }
    answers.sort();
    println!("§12.8.3.4.5 (a) over the corpus's signature dictionaries:");
    for answer in &answers {
        println!("  {answer}");
    }
    let matched: Vec<_> = answers
        .iter()
        .filter(|answer| answer.contains("Matches"))
        .collect();
    assert_eq!(
        answers.len(),
        1,
        "signature dictionaries stating one of §12.8.3.4.3 (f)'s two attributes"
    );
    assert_eq!(
        matched.len(),
        1,
        "and the comparison it makes comes out equal"
    );
    assert!(
        answers[0].starts_with("issue16553.pdf: "),
        "the witness is named because the population is one: {answers:?}"
    );
}

/// §12.8.3.4.4's policy attribute over the corpus, asked of every signature rather than assumed.
///
/// The clause hands the attribute's rules to ETSI EN 319 122-1 clause 5.2.9, and
/// `pdf_signature::policy` reads it: which policy, what the signer said about its document's
/// digest, the qualifiers beside it, and clause 5.2.10's stored copy checked against that digest.
///
/// **The population is zero and that is the finding, not a gap.** No signature this corpus carries
/// states a policy identifier at all, so what exercises the reader is the fixture beside it
/// (`policy::tests`), and this is held at zero so that a document arriving which *does* state one
/// announces itself — the same shape `no_corpus_documents_usage_rights_are_exceeded_by_what_this
/// _program_does` keeps for §12.8.2.3. Asked of every signature rather than grepped for, because a
/// `grep` over these files is the measurement of the instrument this file already records once.
///
/// What is asserted from the other end is that the asking *happened*: an empty answer list is what
/// a reader that never ran would also produce, so the signatures reached are counted too.
#[test]
fn every_corpus_signature_is_asked_which_policy_it_was_made_under() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut asked = 0_usize;
    let mut answers = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in &every_signature(&document) {
            let Ok(cms) = signature.signed_data() else {
                continue;
            };
            asked = asked.saturating_add(1);
            match signature.signature_policy(&cms) {
                Ok(None) => {}
                Ok(Some(policy)) => answers.push(format!(
                    "{name}: policy {}, {:?}, {:?}",
                    policy.identifier,
                    policy.hash,
                    policy.binding()
                )),
                Err(error) => answers.push(format!("{name}: {error}")),
            }
        }
    }
    answers.sort();
    println!("§12.8.3.4.4's signature policy over the corpus's signature dictionaries:");
    for answer in &answers {
        println!("  {answer}");
    }
    assert_eq!(asked, 10, "signature dictionaries whose CMS object read");
    assert!(
        answers.is_empty(),
        "no corpus signature states a policy identifier: {answers:?}"
    );
}

/// The same signature with one bit of its `/Contents` turned over.
///
/// The last octet rather than the first: `/Contents` may carry §12.8.3.3.1's zero padding after
/// the CMS object, and changing a padding byte would leave the signature value itself intact —
/// which is a mutation the verifier is *right* to ignore, and would make this check pass for the
/// wrong reason. The byte chosen is inside the signature, found by walking to the `SignerInfo`'s
/// own value through the reader this crate ships.
fn flipped(signature: &Signature) -> Signature {
    let mut out = signature.clone();
    let at = signature
        .signed_data()
        .ok()
        .filter(|cms| !cms.signature.is_empty())
        .and_then(|cms| {
            signature
                .contents
                .windows(cms.signature.len())
                .position(|window| window == cms.signature)
        })
        .unwrap_or(0);
    if let Some(byte) = out.contents.get_mut(at) {
        *byte ^= 0x01;
    }
    out
}

/// **The population's one DER-encoded ECDSA signature, verified against the file that carries it.**
///
/// `signature_algorithm_census` over 67 460 documents finds exactly one signature whose
/// `SignerInfo` states RFC 5758 section 3.2's `ecdsa-with-SHA256` and whose value is the DER
/// `ECDSA-Sig-Value` ISO/TS 32002 section 5.1.3's NOTE 2 requires. Until the
/// six-hundred-and-eighty-ninth session it reached a reader as `AlgorithmNotVerifiable
/// 1.2.840.10045.4.3.2`; it now verifies under the P-256 key in a certificate the value itself
/// carries (ADR 0532).
///
/// **This is the demand-side half and `ecdsa.rs`'s fixtures are the other.** A real file is the
/// only thing that proves the whole path works on bytes nobody here chose — the certificate's
/// `namedCurve`, the point, the signed attributes, the DER integers as some other producer spelled
/// them. What it cannot do is prove a *positive* verification over bytes this tree chose, because
/// nobody here holds that signer's private key, which is why the module's fixtures exist too.
///
/// The document is in the machine-local `SafeDocs` crawl (`tools/safedocs`), so this **skips and
/// says so** where it is not there, exactly as the pdf.js tests above do for their submodule.
#[test]
fn the_crawls_one_ecdsa_signature_verifies_under_its_own_p256_certificate() {
    use pdf_signature::ecdsa::Curve;
    use pdf_signature::signature::Family;

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus-cache/safedocs/cc-main-2021-31/6100/6100006.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: {} is not in this machine's crawl", path.display());
        return;
    };
    let document = Document::open(bytes).expect("the crawl's ECDSA-signed document opens");
    let file = document.bytes().clone();
    let found = signatures(&document);
    let [signature] = found.as_slice() else {
        panic!("one signature dictionary, not {}", found.len());
    };
    let authenticity = signature.authenticity(&file);
    println!("6100006.pdf: {authenticity:?}");
    let Authenticity::Verified { family, .. } = &authenticity else {
        panic!("the crawl's ECDSA signature should verify, not {authenticity:?}");
    };
    assert_eq!(*family, Family::Ecdsa(Curve::P256));
    // **The instrument has to be able to say no.** One bit of the signature value turned over is
    // the same discrimination check the ten pdf.js signatures get, and for the same reason: a
    // verifier stuck at `true` passes every assertion above.
    assert!(
        !matches!(
            flipped(signature).authenticity(&file),
            Authenticity::Verified { .. }
        ),
        "one bit of the signature value moved and it still verified"
    );
}

/// §12.8.1's third question asked of every corpus signature, with and without an anchor.
///
/// **Two populations and one instrument.** With no anchors the answer must be
/// `Trust::NoAnchorSupplied` for every signature there is — the default this program has always
/// had, now as a typed value rather than a sentence — and with the signature's own self-signed
/// certificate offered as an anchor, RFC 5280 section 6.1 runs over a chain a real authority
/// issued rather than over the hierarchy `trust`'s unit tests build.
///
/// **The anchor here is not a trust decision and the test is not one either.** Taking the root a
/// file supplies and believing it is precisely what a trust store exists to prevent; what it
/// exercises is the *algorithm*, on chains nobody here could have made, which is the half a
/// fixture cannot reach. The verdicts are printed with the instant they were asked at, because
/// `Trust::NotCurrent` on a certificate that expired is a fact about the calendar.
#[test]
fn every_corpus_signature_is_asked_the_third_question_both_ways() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    // 2020-06-01T00:00:00Z. A fixed instant rather than the clock, so the test says the same
    // thing tomorrow: RFC 5280 section 6.1.1 makes the time an input and this is the input.
    let at = Instant::from_unix_seconds(1_590_969_600);
    let mut asked = 0usize;
    let mut verdicts = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in signatures(&document) {
            asked = asked.saturating_add(1);
            assert_eq!(
                signature.trust(&TrustAnchors::none(), &Material::none(), at),
                Trust::NoAnchorSupplied,
                "{name}: with nobody named there is no question to answer"
            );
            let Ok(cms) = signature.signed_data() else {
                continue;
            };
            // Every self-signed certificate the signature carries, offered as an anchor — which
            // is what "the root of the chain in the file" means and all a file can offer.
            let certificates: Vec<_> = cms
                .certificates
                .iter()
                .filter_map(|entry| pdf_signature::x509::read(*entry).ok())
                .filter(|certificate| certificate.subject == certificate.issuer)
                .collect();
            if certificates.is_empty() {
                verdicts.push(format!(
                    "{name}: the signature carries no self-signed certificate"
                ));
                continue;
            }
            let anchors = TrustAnchors::of(&certificates);
            // §12.8.4.3's own material, which is the only supply there is: no host here has a
            // network, and the store is what a verifier is meant to reach for instead.
            let store = security_store(&document);
            let material = store.material();
            verdicts.push(format!(
                "{name}: {:?}",
                signature.trust(&anchors, &material, at)
            ));
        }
    }
    for line in &verdicts {
        println!("{line}");
    }
    assert!(asked > 0, "the corpus carries signatures to ask about");
    // **The calibration, and the whole reason this test is a gate rather than a census.** At least
    // one real chain has to reach `Anchored`, because every refusal below is also what a broken
    // verification, a misread validity period or a name compared at the wrong offset would
    // produce — a suite in which nothing ever validates cannot tell those apart from a corpus of
    // expired certificates. `xfa_filled_imm1344e.pdf` is the witness at this instant: two
    // certificates, RSA, issued by an authority nobody here can sign for.
    assert!(
        verdicts
            .iter()
            .any(|line| line.contains("Anchored { length: 2")),
        "no corpus chain validated, which a working section 6.1 over these files does not do: \
         {verdicts:?}"
    );
    // And the other half of the calibration, which changed in the thousand-and-fifty-third
    // session and is the point of ADR 1067: an anchored path now carries what §12.8.4's material
    // said, and what it may never carry is a `Good` this program did not compute. Every corpus
    // document reaching `Anchored` carries no DSS at all — the census below is what says so — so
    // the answer here is `NotChecked`, and the assertion is written against the *rule* rather than
    // against that fact: no `Good` without material, ever.
    assert!(
        verdicts
            .iter()
            .all(|line| !line.contains("Anchored") || !line.contains("revocation: Good")),
        "a path was called unrevoked by material this corpus does not carry: {verdicts:?}"
    );
}

/// §12.8.4's document security store, counted over the corpus, and what it answers where it is
/// there.
///
/// **This is the denominator ADR 1067 rests on** and it is a fact about the world rather than
/// about the standard, which is why it is a command and not a sentence in a note (trap 8): the
/// revocation reader can only ever say as much as the documents supply, so how many supply
/// anything at all is the size of what it is for. Every store found is printed with its four
/// counts and with what it refused, so that a store this reader could only half read is visible
/// rather than absent.
#[test]
fn every_corpus_document_is_asked_whether_it_carries_a_security_store() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut opened = 0usize;
    let mut with_store = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let store = security_store(&document);
        if store.is_empty() {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let material = store.material();
        with_store.push(format!(
            "{name}: {} certs, {} CRLs, {} OCSPs, {} VRI; material read {} CRLs and {} responses, \
             refused {:?} and {:?}",
            store.certificates.len(),
            store.revocation_lists.len(),
            store.ocsp_responses.len(),
            store.validation_information.len(),
            material.lists.len(),
            material.responses.len(),
            store.refused,
            material.refused,
        ));
    }
    for line in &with_store {
        println!("{line}");
    }
    println!(
        "{} of {opened} corpus documents carry a document security store",
        with_store.len()
    );
    assert!(opened > 0, "the corpus opened");
}

/// The honest default, asserted rather than assumed: no material means no answer.
///
/// **Trap 13's calibration for the sentence above.** A sweep that finds no corpus document with a
/// store reads identically as "the reader is right" and as "the reader is broken", so this plants
/// the case the reader must never get wrong — nothing supplied — and requires the one answer that
/// is a statement about this program rather than about the certificate.
#[test]
fn a_signature_with_no_revocation_material_is_never_called_unrevoked() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let at = Instant::from_unix_seconds(1_590_969_600);
    let mut asked = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        for signature in signatures(&document) {
            let Ok(cms) = signature.signed_data() else {
                continue;
            };
            let certificates: Vec<_> = cms
                .certificates
                .iter()
                .filter_map(|entry| pdf_signature::x509::read(*entry).ok())
                .filter(|certificate| certificate.subject == certificate.issuer)
                .collect();
            if certificates.is_empty() {
                continue;
            }
            asked = asked.saturating_add(1);
            let anchors = TrustAnchors::of(&certificates);
            if let Trust::Anchored { revocation, .. } =
                signature.trust(&anchors, &Material::none(), at)
            {
                assert_eq!(
                    revocation,
                    Revocation::NotChecked,
                    "a path validated with no material supplied claimed something about revocation"
                );
            }
        }
    }
    assert!(asked > 0, "the corpus carries chains to ask about");
}

/// **§12.8.5's chain, over the crawl's real document timestamps.**
///
/// No document in `doc/pdf.js` carries a `/DocTimeStamp` and the crawl carries many, which is the
/// two denominators in one sentence: what a corpus finds is what documents contain, and these are
/// real archival files with two and three tokens stacked on them the way §12.8.5.3 describes.
///
/// **What it asserts is the rule, not a verdict** (trap 13's shape, and ADR 1039's): no anchor is
/// supplied, so not one link of any chain here may establish an instant — and the ordering,
/// the coverage and the refusals are checked on every one of them. A document whose later token
/// did not cover its earlier one would be named by path.
///
/// The files are in the machine-local crawl (`tools/safedocs`), so this **skips and says so**
/// where they are not there, exactly as the pdf.js tests above do for their submodule.
#[test]
fn the_crawls_stacked_document_timestamps_are_ordered_and_none_of_them_is_a_time() {
    use pdf_signature::timestamp::{ChainRefusal, Time, chain};

    // Three witnesses the census named, at one, two and three timestamps: the shapes §12.8.5.2 and
    // §12.8.5.3 describe, and the second and third are what makes "chain" more than a word.
    let witnesses = [
        "tika-issue-tracker/batch5/DSS/DSS-2244-0.pdf",
        "tika-issue-tracker/batch5/DSS/DSS-1794-1.pdf",
        "tika-issue-tracker/batch5/DSS/DSS-1696-1.pdf",
        "tika-issue-tracker/batch5/DSS/DSS-1330-1.pdf",
    ];
    let mut read = 0usize;
    let mut links = 0usize;
    for witness in witnesses {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus-cache")
            .join(witness);
        let Ok(bytes) = std::fs::read(&path) else {
            println!("skipped: {witness} is not in this machine's crawl");
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            println!("{witness}: does not open");
            continue;
        };
        read = read.saturating_add(1);
        let store = security_store(&document);
        let material = store.material();
        let at = Instant::from_unix_seconds(1_790_812_800);
        let chain = chain(&document, &TrustAnchors::none(), &material, at);
        println!(
            "{witness}: {} timestamp(s), {} other signature(s), refusals {:?}",
            chain.links.len(),
            chain.signatures,
            chain.refused
        );
        links = links.saturating_add(chain.links.len());
        let mut previous = 0_u64;
        for (index, link) in chain.links.iter().enumerate() {
            println!(
                "  [{index}] to {} ends_a_revision {} claim {:?} integrity {:?} covers {:?} \
                 material {}/{} time {:?}",
                link.covers_to,
                link.ends_a_revision,
                link.claim
                    .as_ref()
                    .map(|claim| claim.gen_time.unix_seconds()),
                link.integrity,
                link.covers,
                link.material_covered.len(),
                link.material_covered.len() + link.material_uncovered.len(),
                link.time
            );
            assert!(
                link.covers_to >= previous,
                "{witness}: the links are not ordered by extent"
            );
            previous = link.covers_to;
            // **The rule, and the whole of what this program may say.** No host in this tree names
            // a trust anchor, so no token's `genTime` is an instant this program states (ADR 1039,
            // ADR 1071 section 1).
            assert!(
                matches!(link.time, Time::Unknown(_)),
                "{witness}: a timestamp established a time with no anchor supplied: {:?}",
                link.time
            );
        }
        // §12.8.5.3's own requirement, held against files that were built to meet it: a later
        // token protects the earlier one. A file that fails it is named rather than tolerated.
        assert!(
            !chain.refused.iter().any(|refusal| matches!(
                refusal,
                ChainRefusal::DoesNotCoverTheEarlierTimestamp { .. }
            )),
            "{witness}: a later timestamp does not cover an earlier one: {:?}",
            chain.refused
        );
    }
    if read == 0 {
        println!("skipped: none of the crawl's timestamp witnesses is on this machine");
        return;
    }
    assert!(
        links >= read,
        "every witness the census named carries at least one document timestamp"
    );
}

/// **§12.8.3.3.1's signature timestamp attribute, over every corpus signature.**
///
/// The clause's other timestamp — "Timestamp information as an unsigned attribute ( PDF 1.6 )" —
/// and the one a reader can check with no certificate in hand, because RFC 3161 Appendix A says
/// what its imprint is over: "The value of messageImprint field within TimeStampToken shall be a
/// hash of the value of signature field within SignerInfo for the signedData being time-stamped."
/// That is a digest of bytes this program already holds.
///
/// **Two of the 974 carry one** — `examples/signature_algorithm_census` is the command that names
/// them and this walk re-derives the count rather than asserting it — and on each the imprint must
/// match. **The calibration is one bit of the signature turned over** (trap 13): a comparison
/// stuck at `true` would pass the first half of this test and say nothing, so the same attribute
/// over a moved signature value must stop covering it.
#[expect(
    clippy::doc_markdown,
    reason = "RFC 3161 Appendix A is quoted verbatim and its ASN.1 names are camel case; a \
              quotation with backticks added to please a lint is no longer a quotation"
)]
#[test]
fn every_corpus_signature_timestamp_attribute_is_checked_against_the_signature_it_sits_on() {
    use pdf_signature::timestamp::signature_timestamp;

    let Some(corpus) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut carried = 0usize;
    for path in corpus {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for signature in every_signature(&document) {
            let Ok(cms) = signature.signed_data() else {
                continue;
            };
            let Some(stamped) = signature_timestamp(&cms) else {
                continue;
            };
            carried = carried.saturating_add(1);
            let stamp = stamped.unwrap_or_else(|refusal| {
                panic!("{name}: a real signature timestamp did not read: {refusal}")
            });
            println!(
                "{name}: signature timestamp genTime {}, covers the signature {}",
                stamp.claim.stated, stamp.covers_the_signature
            );
            assert!(
                stamp.covers_the_signature,
                "{name}: RFC 3161 Appendix A's imprint is not the digest of this signature"
            );
            // The planted defect: the same token over a signature one bit different.
            let moved = flipped(&signature);
            let Ok(moved) = moved.signed_data() else {
                panic!("{name}: the flipped signature no longer reads");
            };
            let Some(Ok(after)) = signature_timestamp(&moved) else {
                panic!("{name}: the flipped signature lost its timestamp attribute");
            };
            assert!(
                !after.covers_the_signature,
                "{name}: the imprint still covers a signature value that moved"
            );
        }
    }
    println!("{carried} corpus signature(s) carry §12.8.3.3.1's timestamp attribute");
    assert!(
        carried > 0,
        "the census names two witnesses in doc/pdf.js; finding none means this walk did not run"
    );
}
