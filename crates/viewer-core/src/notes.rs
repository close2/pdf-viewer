//! What a document says about itself, said out loud once when it opens.
//!
//! Five clauses, and none of them is about a page. §12.11's requirements, §12.8's signatures,
//! §7.11.4's embedded files, §7.5's recovered cross-reference table and Annex I's version are all
//! claims about the *file*, and a person deciding whether to trust what they are looking at needs them before any
//! page is drawn. That is why they are a [`crate::Event::Reported`] with no page rather than
//! part of the page's own report.
//!
//! Nothing here is an error and nothing here stops a document opening.

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

/// §12.8's signatures: what a program with no trust store can honestly say about one.
///
/// Three things are said and a fourth is refused: who signed, why, whether the range they signed
/// runs to the end of the file (§12.8.1) — and that this program does not verify anything,
/// because verification needs a certificate chain and a trust store, which is §7.6.5's refusal
/// one clause over (ADR 0031).
fn signatures(document: &Document, notes: &mut Vec<String>) {
    let signatures = pdf_model::signature::signatures(document);
    if signatures.is_empty() {
        return;
    }
    let length = document.bytes().len() as u64;
    for signature in &signatures {
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
    }
    // §12.8.2.2.1's parenthesis is a `shall` addressed to a processor that modifies: "(These
    // changes to the document shall also be prevented if the signature dictionary is referred
    // from the DocMDP entry in the permissions dictionary.)" This program modifies since the
    // hundred-and-thirty-fifth session, so it obeys it — `ViewState::set_field` refuses — and
    // says so here, because a field that will not take a value is otherwise a person typing
    // into a document that ignores them.
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
    notes.push(
        "signatures are not verified — this program has no certificate store, so it says what a \
         signature claims and never whether it is valid"
            .to_owned(),
    );
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
