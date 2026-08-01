//! What a document says about itself, said out loud once when it opens.
//!
//! Four clauses, and none of them is about a page. §12.11's requirements, §12.8's signatures,
//! §7.11.4's embedded files and §7.5's recovered cross-reference table are all claims about the
//! *file*, and a person deciding whether to trust what they are looking at needs them before any
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
            pdf_model::signature::Coverage::Unsigned { tail } => notes.push(format!(
                "{tail} bytes were appended after that signature and are not covered by it"
            )),
            pdf_model::signature::Coverage::Malformed => {
                notes.push("that signature's /ByteRange does not describe this file".to_owned());
            }
        }
    }
    notes.push(
        "signatures are not verified — this program has no certificate store, so it says what a \
         signature claims and never whether it is valid"
            .to_owned(),
    );
}
