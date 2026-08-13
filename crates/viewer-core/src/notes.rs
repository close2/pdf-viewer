//! What a document says about itself, said out loud once when it opens.
//!
//! Five clauses, and none of them is about a page. §12.11's requirements, §12.8's signatures,
//! §7.11.4's embedded files, §7.5's recovered cross-reference table and Annex I's version are all
//! claims about the *file*, and a person deciding whether to trust what they are looking at needs them before any
//! page is drawn. That is why they are a [`crate::Event::Reported`] with no page rather than
//! part of the page's own report.
//!
//! Nothing here is an error and nothing here stops a document opening.

use pdf_model::signature::PadesDeparture;
use pdf_syntax::Document;

/// Everything worth saying about a document the moment it opens.
pub(crate) fn about(document: &Document) -> Vec<String> {
    let mut notes = Vec::new();

    if document.was_recovered() {
        // Worth saying: the file's own cross-reference table was unusable and the document was
        // reconstructed by scanning. It may still be missing content.
        notes.push(
            "this file's cross-reference table was broken and was rebuilt by scanning".to_owned(),
        );
    }

    // Annex I, and it is the annex's own instruction rather than an inference from it:
    //
    // > If a PDF processor opens a PDF file with a version number newer than the version that it
    // > supports or it identifies document requirements (12.11, "Document requirements") that it
    // > is not prepared to process, it should warn the user that it is unlikely to be able to
    // > read the document successfully and that the user may not be able to change or save the
    // > document.
    //
    // The second half of that sentence is the loop below; this is the first, and it was owed for
    // three hundred and sixty sessions because Annex I had no ledger row until ADR 0206. **No
    // corpus document reaches it**: the newest of the 974 states 2.0, which is what this program
    // is written against, so this note exists for the file that has not been written yet.
    if let Some(version) = document.version()
        && version > pdf_syntax::Version::SUPPORTED
    {
        notes.push(format!(
            "this document states PDF {version}, which is newer than the {} this program \
             implements — it is unlikely to be read successfully",
            pdf_syntax::Version::SUPPORTED
        ));
    }

    // §12.11's document requirements. The clause makes this a statement about the *document*
    // rather than about any page — "there is no formal connection between the requirement type
    // and the operation of the associated feature(s)" — so it belongs here rather than in a
    // page's report, and it is the one thing a page report cannot do: tell a person before they
    // trust what they are looking at. §12.11.6 asks a processor that cannot meet the
    // requirements to stop; this one draws the document and names what it could not promise,
    // because refusing to open a file somebody asked for is a worse failure. No corpus document
    // states any of this.
    for (requirement, reason) in pdf_model::requirements::unmet(document) {
        notes.push(format!(
            "this document requires {} (penalty {}) — {reason}",
            requirement.kind.as_str(),
            requirement.penalty
        ));
    }

    // §7.11.4's embedded files, listed and not extracted: the bytes are inside the document, and
    // writing one out is a person's decision taken somewhere that can ask. Saying they exist is
    // the half a viewer with no attachment panel can still do honestly.
    for attachment in &pdf_model::attachment::attachments(document) {
        let size = attachment
            .size
            .map_or_else(String::new, |size| format!(", {size} bytes"));
        notes.push(format!(
            "this document carries an embedded file: {}{size}{}",
            attachment
                .file_name
                .as_deref()
                .unwrap_or(attachment.name.as_str()),
            attachment
                .media_type
                .as_deref()
                .map_or_else(String::new, |media| format!(" ({media})"))
        ));
    }

    signatures(document, &mut notes);
    notes
}

/// Why a document does not permit an operation, in sentences a host can show.
///
/// The other half of [`about`], and the same vocabulary: what a document asserts is read by
/// `pdf_model::restriction`, which answers with clauses and levels, and the words are here
/// because words about a document belong where the rest of this program's words about one are.
/// Empty means the document asserts nothing against it.
///
/// **Worded even where the reader is ignoring them.** [`crate::RestrictionLevel::Off`] means the
/// operation happens, not that the file said nothing — and *ask* and *warn*, when they arrive,
/// need exactly these sentences at exactly this moment.
pub(crate) fn refusal(
    document: &Document,
    operation: pdf_model::restriction::Operation,
    field: Option<&str>,
    annotation: Option<pdf_syntax::ObjectId>,
) -> Vec<String> {
    use pdf_model::restriction::Restriction;
    use pdf_model::signature::Modification;

    pdf_model::restriction::asserted(document, operation, field, annotation)
        .into_iter()
        .map(|restriction| match restriction {
            // §12.8.2.2.1's parenthesis is a `shall` addressed to a processor that modifies:
            // "(These changes to the document shall also be prevented if the signature
            // dictionary is referred from the DocMDP entry in the permissions dictionary.)"
            Restriction::Certified { level } => match level {
                Modification::None => format!(
                    "this document's author certified it as final (§12.8.2.2's /P 1), so it \
                     permits no change at all — {} was not done",
                    operation.as_str()
                ),
                Modification::FormFilling => format!(
                    "this document's author permitted only form filling and signing (§12.8.2.2's \
                     /P 2), which does not include {} — it was not done",
                    operation.as_str()
                ),
                // Level 3 and an undefined level both permit, so `asserted` never names them
                // here; saying which one arrived is better than a sentence that claims a rule.
                other => format!(
                    "this document's /DocMDP states {other:?}, and {} was not done",
                    operation.as_str()
                ),
            },
            // §7.6.4.2's Table 22, and §7.6.4.1's sentence about it: "PDF readers shall respect
            // the intent of the document creator by restricting user access to an encrypted PDF
            // file according to the permissions contained in the file."
            Restriction::AccessDenied { bit } => format!(
                "this document's encryption does not grant {} (§7.6.4.2's Table 22, bit {bit}) — \
                 it was not done",
                operation.as_str()
            ),
            // §12.7.5.5: "The signature field lock dictionary … contains the names of form
            // fields whose values shall no longer be changed after this signature has been
            // signed." The sentence names the *signature* as what locks the field, which is
            // what a person is owed here: the refusal is somebody's signature rather than the
            // document's encryption or its author's certification.
            Restriction::FieldLocked => format!(
                "a signature in this document locks this field against further change \
                 (§12.7.5.5's /Lock) — {} was not done",
                operation.as_str()
            ),
            // §12.5.3's Table 167 bit 10: "If set, do not allow the contents of the annotation to
            // be modified by the user." Bit 8's `Locked` is named in the same breath because it
            // is the flag a person would expect to be the reason and is not — its own row says it
            // "does not restrict changes to the annotation's contents".
            Restriction::AnnotationLocked => format!(
                "this annotation is marked LockedContents (§12.5.3's Table 167, bit 10), so its \
                 text may not be changed — {} was not done",
                operation.as_str()
            ),
        })
        .collect()
}

/// §12.8's signatures: what a program with no trust store can honestly say about one.
///
/// **A signature asks three questions and this program answers two of them** (ADR 0215, ADR 0229),
/// so the hardest part of this function is keeping them apart in the words it uses. It says who
/// signed, why, whether the range they signed runs to the end of the file (§12.8.1), whether the
/// bytes that range names still hash to the digest the signature records, and — since the
/// three-hundred-and-ninety-second session — **whether the signature verifies under the public key
/// in the certificate the file itself carries** (§12.8.3.3.1).
///
/// What it still does not say is whether that certificate belongs to anyone worth believing, or
/// whether it had been revoked: that needs a certificate store and a network, which is §7.6.5's
/// refusal one clause over (ADR 0031). **None of these sentences contains the word *valid*, and
/// that is the point of them** — a certificate that arrived in the same file as the signature it
/// verifies proves the two are consistent with each other and nothing about who made either.
fn signatures(document: &Document, notes: &mut Vec<String>) {
    let signatures = every_signature(document);
    if signatures.is_empty() {
        return;
    }
    let length = document.bytes().len() as u64;
    for signature in &signatures {
        about_one(signature, document, length, notes);
    }
    permissions(document, notes);
    // The three questions, named, in the order §12.8.1 states them. This paragraph is what stops
    // the sentences above it being read as "the signature is good". Two of them are answered and
    // the third is the one that decides whether a signature means anything about a *person*: a
    // key that verifies is a key, and this program has nothing to say about whose.
    notes.push(
        "of the three questions a signature asks, this program answers two: whether the document \
         changed since it was signed (§12.8.1's digest, recomputed above) and whether the \
         signature verifies under the public key in the certificate the file itself carries \
         (§12.8.3.3.1). It does not answer the third — it has no certificate store and makes no \
         network request, so it does not know whether that certificate belongs to anyone you have \
         reason to believe, nor whether it had been revoked. A signature that verifies here was \
         made by whoever holds the key in a certificate that arrived with the document, which is \
         not the same as a valid signature. Nothing here says valid"
            .to_owned(),
    );
}

/// Every signature dictionary the document holds, from both places §12.8.1 puts one.
///
/// §12.8.1 puts a usage rights signature's dictionary in the permissions dictionary "(not from a
/// signature field)", so the field walk cannot reach one and three corpus documents carry nothing
/// else. A certification signature is normally in both and is said once.
fn every_signature(document: &Document) -> Vec<pdf_model::signature::Signature> {
    let permissions = pdf_model::signature::permissions(document);
    let mut signatures = pdf_model::signature::signatures(document);
    for extra in [
        permissions.usage_rights_signature,
        permissions.doc_mdp_signature,
    ]
    .into_iter()
    .flatten()
    {
        if !signatures.contains(&extra) {
            signatures.push(extra);
        }
    }
    signatures
}

/// What one signature says, what it covers, and whether the bytes under it moved.
fn about_one(
    signature: &pdf_model::signature::Signature,
    document: &Document,
    length: u64,
    notes: &mut Vec<String>,
) {
    {
        let who = signature.name.as_deref().unwrap_or("an unnamed signer");
        let why = signature
            .reason
            .as_deref()
            .map_or_else(String::new, |reason| format!(", reason: {reason}"));
        // §7.9.4's date where the producer wrote a conforming one, and the file's own bytes
        // where it did not — 2.0% of the corpus's dates are the second, and showing nothing
        // there would hide a value a person can read perfectly well.
        let when =
            signature
                .signed_at
                .as_deref()
                .map_or_else(String::new, |stated| match signature.signed_at_date() {
                    Some(date) => format!(", at {date}"),
                    None => format!(", at {stated} (not a §7.9.4 date)"),
                });
        notes.push(format!(
            "this document is signed by {who}{why}{when}{}",
            if signature.certification {
                " (a certification signature)"
            } else {
                ""
            }
        ));
        match signature.coverage(length) {
            pdf_model::signature::Coverage::WholeFile => {}
            // **Two different things wear one shape here, and Table 255 separates them.**
            // §12.8.1's NOTE 1 makes an uncovered tail the ordinary mechanism — an incremental
            // update appended after signing, which is how a signature stays meaningful while a
            // document goes on being used. But for `ETSI.CAdES.detached` and `ETSI.RFC3161` the
            // table says the range "shall cover the entire PDF file", so for those two the same
            // tail is a file breaking a `shall`. `Signature::must_cover_whole_file` has drawn
            // that distinction since it was written and nothing asked it until the
            // two-hundred-and-seventy-eighth session — `doc/todo/01`'s fifth sweep, which asks
            // what the model implements that no host calls. It is still not a verdict on the
            // signature: this program has no trust store and says what the file states.
            pdf_model::signature::Coverage::Unsigned { tail } => {
                if signature.must_cover_whole_file() {
                    notes.push(format!(
                        "{tail} bytes were appended after that signature and are not covered by \
                         it — and its /SubFilter {} requires the signed range to cover the whole \
                         file (Table 255), so this file breaks that requirement",
                        signature.sub_filter.as_deref().unwrap_or("")
                    ));
                } else {
                    notes.push(format!(
                        "{tail} bytes were appended after that signature and are not covered by it"
                    ));
                }
            }
            pdf_model::signature::Coverage::Malformed => {
                notes.push("that signature's /ByteRange does not describe this file".to_owned());
            }
        }
        verdicts(signature, document, notes);
        // §12.8.3.4's rules on a PAdES signature that need no certificate to check. Silent for
        // every other `/SubFilter`, which is §12.8.3.4.1's own scope.
        if let Ok(cms) = signature.signed_data() {
            // §12.8.3.3.2's revocation information, whose object identifier the clause prints
            // itself. Its presence is the fact this program can state: the CRLs and OCSP
            // responses inside it are what a *validator* would use, and using them is question
            // three. `issue17069.pdf` is the corpus's one witness.
            if cms.has_signed_attribute(pdf_model::cms::ADBE_REVOCATION_INFO_ARCHIVAL) {
                notes.push(
                    "that signature carries revocation information with it \
                     (§12.8.3.3.2's adbe-revocationInfoArchival attribute), which this program \
                     does not check — it makes no trust decision about any certificate"
                        .to_owned(),
                );
            }
            for departure in signature.pades_departures(&cms, length) {
                notes.push(format!(
                    "that signature states /SubFilter /ETSI.CAdES.detached, and {}",
                    match departure {
                        PadesDeparture::RangeDoesNotCoverTheFile =>
                            "§12.8.3.4.2 requires its /ByteRange to cover the entire file",
                        PadesDeparture::CertEntryPresent =>
                            "§12.8.3.4.2 says its dictionary shall not contain a /Cert entry",
                        PadesDeparture::BothSigningTimesStated =>
                            "§12.8.3.4.2 permits either its /M or a signing-time attribute, not both",
                        PadesDeparture::ContentTypeIsNotData =>
                            "§12.8.3.4.3 (a) requires its content-type to be id-data",
                        PadesDeparture::NotExactlyOneSigner =>
                            "§12.8.3.4.3 (d) requires exactly one SignerInfo",
                        PadesDeparture::NoMessageDigest =>
                            "§12.8.3.4.3 (e) requires a message-digest attribute",
                        PadesDeparture::CounterSignature =>
                            "§12.8.3.4.3 (i) says a counter-signature attribute shall not be used",
                    }
                ));
            }
        }
    }
}

/// §12.8.6's permissions dictionary, said before a person starts typing rather than after.
fn permissions(document: &Document, notes: &mut Vec<String>) {
    // §12.8.2.2.1's parenthesis is a `shall` addressed to a processor that modifies: "(These
    // changes to the document shall also be prevented if the signature dictionary is referred
    // from the DocMDP entry in the permissions dictionary.)" This program modifies since the
    // hundred-and-thirty-fifth session, so it obeys it — an `Edit` that a level of `/P` does not
    // permit is refused with `Event::Refused` and its reason — and says so here as well, because
    // a field that will not take a value is otherwise a person typing into a document that
    // ignores them, and this is said before they start rather than after.
    match pdf_model::signature::permissions(document).doc_mdp {
        Some(pdf_model::signature::Modification::None) => notes.push(
            "this document's author certified it as final (§12.8.2.2's /P 1), so no change to \
             it is permitted and none will be accepted"
                .to_owned(),
        ),
        Some(pdf_model::signature::Modification::FormFilling) => notes.push(
            "this document's author permitted only form filling and signing (§12.8.2.2's /P 2)"
                .to_owned(),
        ),
        Some(pdf_model::signature::Modification::FormFillingAndAnnotation) => notes.push(
            "this document's author permitted form filling, signing and annotation \
             (§12.8.2.2's /P 3)"
                .to_owned(),
        ),
        Some(pdf_model::signature::Modification::Unknown(level)) => notes.push(format!(
            "this document's /DocMDP states /P {level}, which Table 257 does not define; it is \
             read as permitting rather than as forbidding"
        )),
        None => {}
    }
    // §12.8.2.3's `should` is obeyed silently otherwise, and a signature disappearing from a
    // file is not a thing to do without saying so first: "A PDF processor that modifies a PDF,
    // with a UR signature in excess of the rights that are granted by that signature, should
    // remove that signature prior to writing the newly modified PDF." The note is said when the
    // document opens rather than when it is saved, because that is when a person can still
    // decide not to.
    if let Some(rights) = pdf_model::signature::permissions(document).usage_rights {
        let fills = rights.grants(pdf_model::signature::Right::FillInForm);
        let saves = rights.grants(pdf_model::signature::Right::FullSave);
        if fills && saves {
            notes.push(
                "this document carries a usage rights signature (§12.8.2.3's /UR3, deprecated \
                 in PDF 2.0), and it grants filling in a field and saving"
                    .to_owned(),
            );
        } else {
            notes.push(
                "this document carries a usage rights signature (§12.8.2.3's /UR3) that does \
                 not grant filling in a field and saving, so saving a change will remove it"
                    .to_owned(),
            );
        }
    }
}

/// What this program can say about one signature: §12.8.1's first two questions, in that order.
///
/// Worded per case rather than by printing an enum, because the differences between the cases are
/// the whole of ADR 0215 and ADR 0229. Three of them are worth the words:
///
/// - a **mismatching** digest is a fact about this file and a **matching** one is the absence of
///   one kind of evidence against it, because the recorded digest sits beside the signature;
/// - a signature that **verifies** was made by whoever holds the key in a certificate *the file
///   itself supplied*, which is a real fact and is not "valid";
/// - and where the two disagree — a signature that verifies over a digest the bytes no longer
///   produce — the document is the thing that moved, which neither answer says on its own.
fn verdicts(
    signature: &pdf_model::signature::Signature,
    document: &Document,
    notes: &mut Vec<String>,
) {
    use pdf_model::signature::{Authenticity, Integrity, Signed};
    let integrity = signature.integrity(document.bytes());
    let authenticity = signature.authenticity(document.bytes());
    // **Question 1's answer can come from question 2**, and this is the one place it does. A
    // `SignerInfo` with no signed attributes signs the content itself, which for a detached
    // signature is the byte range — so nothing records the digest in the open
    // (`UnderTheSignersKey`) and the signature verifying *is* the answer. `bug854315.pdf` is the
    // corpus's witness, and saying both sentences there would be this program reporting a
    // question it had just answered as unanswerable.
    let settled_by_the_key = matches!(
        (&integrity, &authenticity),
        (
            Integrity::UnderTheSignersKey,
            Authenticity::Verified {
                over: Signed::TheDocumentsBytes,
                ..
            } | Authenticity::NotUnderThatKey {
                over: Signed::TheDocumentsBytes,
                ..
            }
        )
    );
    if !settled_by_the_key {
        notes.push(changed(integrity));
    }
    if let Some(said) = verifies(&authenticity, integrity) {
        notes.push(said);
    }
}

/// §12.8.1's first question in words: did the bytes under this signature move?
fn changed(integrity: pdf_model::signature::Integrity) -> String {
    use pdf_model::signature::Integrity;
    match integrity {
        Integrity::Changed { digest } => format!(
            "the bytes that signature covers no longer hash to the {} digest it records — this \
             document was modified after it was signed (§12.8.1)",
            digest.name()
        ),
        Integrity::Unchanged { digest } => format!(
            "the bytes that signature covers still hash to the {} digest it records, so nothing \
             changed after signing",
            digest.name()
        ),
        Integrity::UnderTheSignersKey => {
            "that signature records no digest in the open, so whether the document changed could \
             only be answered by checking the signature itself"
                .to_owned()
        }
        Integrity::UnknownDigest => {
            "that signature records a digest made with an algorithm this program does not \
             compute, so whether the document changed was not checked"
                .to_owned()
        }
        Integrity::RangeNotInThisFile => {
            "that signature's /ByteRange names bytes outside this file, so there was nothing to \
             hash"
                .to_owned()
        }
        Integrity::NoSignatureValue => {
            "that signature states no /Contents, which Table 255 requires, so there is nothing to \
             check the document against"
                .to_owned()
        }
        Integrity::Unreadable(error) => {
            format!("that signature's value could not be read: {error}")
        }
    }
}

/// §12.8.1's second question in words, worded against the first one's answer.
///
/// `None` where the sentence would repeat what [`changed`] has already said in the same words —
/// no signature value, no bytes to hash, nothing readable — because a program that says one fact
/// twice teaches a reader to skim.
fn verifies(
    authenticity: &pdf_model::signature::Authenticity,
    integrity: pdf_model::signature::Integrity,
) -> Option<String> {
    use pdf_model::signature::{Authenticity, Integrity, Signed};
    Some(match authenticity {
        Authenticity::Verified {
            key_bits,
            family,
            over: Signed::TheDocumentsBytes,
            ..
        } => format!(
            "and that signature verifies under the {key_bits}-bit {} key in a certificate the \
             file itself carries, directly over the bytes its /ByteRange names — so those bytes \
             are the ones that were signed and nothing has changed since",
            family.name()
        ),
        // **The pairing, and the reason this program computes both answers before saying either.**
        // A signature that verifies over attributes recording a digest the file no longer
        // produces is not a broken signature; it is a real one whose document was re-saved
        // underneath it. Four of the corpus's ten are exactly that.
        Authenticity::Verified {
            key_bits, family, ..
        } if matches!(integrity, Integrity::Changed { .. }) => {
            format!(
                "and that signature does verify under the {key_bits}-bit {} key in a certificate \
                 the file itself carries — but what it signs is the digest above, which these \
                 bytes no longer produce. The signature is a real one and the document under it \
                 is not the document it was made over",
                family.name()
            )
        }
        Authenticity::Verified {
            key_bits, family, ..
        } => format!(
            "and that signature verifies under the {key_bits}-bit {} key in a certificate the \
             file itself carries, over the attributes that record the digest above (RFC 5652 \
            section 5.4) — so that digest is the signer's",
            family.name()
        ),
        Authenticity::NotUnderThatKey {
            key_bits,
            family,
            over,
            ..
        } => format!(
            "and that signature does NOT verify under the {key_bits}-bit {} key in the \
             certificate the file carries{} — the value, the key and the bytes are not three that \
             belong together, and nothing here can say which of them moved",
            family.name(),
            if over.binds_the_document() {
                ", over the bytes its /ByteRange names"
            } else {
                ""
            }
        ),
        Authenticity::NoSignerCertificate { certificates } => format!(
            "and §12.8.3.3.1 requires the signature to carry the signer's X.509 certificate — none \
             of the {certificates} it carries is the one it names, so there is no key to check it \
             against"
        ),
        Authenticity::CertificateUnreadable(error) => format!(
            "and the signer's certificate could not be read, so the signature was not checked \
             against it: {error}"
        ),
        Authenticity::KeyNotVerifiable { algorithm } => format!(
            "and the signer's certificate holds a public key of algorithm {algorithm}, which this \
             program does not verify: it verifies two of Table 260's three families, RSA (PKCS #1 \
             v1.5) and DSA, and not the ECDSA the table names beside them"
        ),
        Authenticity::AlgorithmNotVerifiable { algorithm } => format!(
            "and that signature states signature algorithm {algorithm}, which this program does \
             not verify: it verifies RSASSA-PKCS1-v1_5 and DSA"
        ),
        Authenticity::KeyDoesNotMatchAlgorithm { algorithm, key } => format!(
            "and that signature states signature algorithm {algorithm} while the signer's \
             certificate holds a key of algorithm {key} — two statements by the same producer \
             that contradict each other, so there is nothing to check the signature against"
        ),
        Authenticity::Refused(error) => {
            format!("and that signature was not checked against the signer's key: {error}")
        }
        Authenticity::RefusedDsa(error) => {
            format!("and that signature was not checked against the signer's key: {error}")
        }
        Authenticity::UnknownDigest { algorithm } => format!(
            "and it names digest algorithm {algorithm}, which this program does not compute, so \
             it was not checked against the signer's key either"
        ),
        Authenticity::NoSignatureValue
        | Authenticity::RangeNotInThisFile
        | Authenticity::Unreadable(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::about;

    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1, as `pdf_model::signature`'s tests do.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// The words a person is given about a real document whose signed bytes no longer hash.
    ///
    /// `xfa_filled_imm1344e.pdf` is the corpus's certification signature, and the round that
    /// recomputed its digest found that it does not match: the file was re-saved rather than
    /// incrementally updated, so the bytes before its signature moved. ADR 0215 has the argument
    /// and `pdf-model`'s `tests/signatures.rs` has the measurement over all ten corpus
    /// signatures; this pins the *sentence*, because the whole risk of this round is a program
    /// that says more than it checked.
    #[test]
    fn a_document_whose_signed_bytes_moved_says_so_and_claims_nothing_more() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/pdf.js/test/pdfs/xfa_filled_imm1344e.pdf");
        let Ok(bytes) = std::fs::read(&path) else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        let document = Document::open(bytes).expect("a valid file");
        let said = about(&document).join("\n");

        assert!(
            said.contains("no longer hash to the SHA256 digest it records"),
            "{said}"
        );
        assert!(
            said.contains("modified after it was signed (§12.8.1)"),
            "{said}"
        );
        assert!(
            said.contains("this program answers two: whether the document changed"),
            "the three questions are named and only two are claimed: {said}"
        );
        assert!(
            said.contains("no certificate store and makes no network request"),
            "{said}"
        );
        // **The pairing this round added, and the sentence it exists for.** The signature does
        // verify; the digest it signs is not the one these bytes make. Saying only the first
        // would be a viewer vouching for a document that had been re-saved under its signature.
        assert!(
            said.contains("does verify under the 2048-bit RSA (PKCS #1 v1.5) key"),
            "the family is named since Table 260's second one was implemented: {said}"
        );
        assert!(
            said.contains("the document under it is not the document it was made over"),
            "{said}"
        );
        assert!(
            said.contains("which is not the same as a valid signature. Nothing here says valid"),
            "no sentence here calls a signature valid: {said}"
        );
    }

    /// The one corpus document whose *first* question is answered by its second.
    ///
    /// `bug854315.pdf`'s `SignerInfo` states no signed attributes, so RFC 5652 signs the content
    /// itself — the byte range, for a detached signature — and nothing records the document's
    /// digest in the open. Until this round that was `Integrity::UnderTheSignersKey` and the note
    /// said the key could not be obtained. It could: §12.8.3.3.1 requires the signature to carry
    /// it, and this one does.
    #[test]
    fn a_signature_over_the_documents_own_bytes_answers_both_questions_at_once() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/pdf.js/test/pdfs/bug854315.pdf");
        let Ok(bytes) = std::fs::read(&path) else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        let document = Document::open(bytes).expect("a valid file");
        let said = about(&document).join("\n");
        assert!(
            said.contains(
                "directly over the bytes its /ByteRange names — so those bytes are the ones that \
                 were signed and nothing has changed since"
            ),
            "{said}"
        );
        assert!(
            !said.contains("records no digest in the open"),
            "the question that could not be answered is answered, so it is not also reported \
             as unanswerable: {said}"
        );
    }

    /// An uncovered tail is ordinary under one sub-filter and a broken `shall` under another.
    ///
    /// §12.8.1's NOTE 1 makes bytes appended after signing the *mechanism* by which a signed
    /// document goes on being used, so the plain note says what happened and judges nothing.
    /// Table 255 says that for `ETSI.CAdES.detached` and `ETSI.RFC3161` the range "shall cover
    /// the entire PDF file", and then the same tail is the file breaking a requirement — which
    /// is worth saying to somebody deciding whether to trust what they are looking at.
    ///
    /// **No corpus document exercises this**: all six signatures in the 974 are `adbe.pkcs7.*`.
    /// That is trap 8 exactly — a corpus finds what documents contain, not what the standard
    /// says — and it is why the two files here are built rather than found. They differ in one
    /// name and in nothing else.
    #[test]
    fn a_sub_filter_can_turn_an_uncovered_tail_into_a_broken_requirement() {
        let objects = |sub_filter: &str| {
            vec![
                "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>"
                    .to_owned(),
                "<< /Type /Pages /Count 0 /Kids [] >>".to_owned(),
                "<< /Unused true >>".to_owned(),
                "<< /FT /Sig /T (S) /V 5 0 R >>".to_owned(),
                format!(
                    "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /{sub_filter} \
                     /ByteRange [0 100 200 100] /Name (A. Author) /Contents <00> >>"
                ),
            ]
        };
        let said = |sub_filter: &str| {
            let bodies = objects(sub_filter);
            let borrowed: Vec<&str> = bodies.iter().map(String::as_str).collect();
            about(&document(&borrowed)).join("\n")
        };

        let ordinary = said("adbe.pkcs7.detached");
        assert!(
            ordinary.contains("are not covered by it"),
            "the tail is still reported: {ordinary}"
        );
        assert!(
            !ordinary.contains("Table 255"),
            "§12.8.1's should stays a should: {ordinary}"
        );

        let required = said("ETSI.CAdES.detached");
        assert!(
            required.contains("requires the signed range to cover the whole file (Table 255)"),
            "Table 255 turns the same tail into a broken requirement: {required}"
        );
    }
}
