//! What a document says about itself, said out loud once when it opens.
//!
//! Eight clauses, and none of them is about a page. §12.11's requirements, §12.8's signatures,
//! §7.11.4's embedded files, §14.13.2's associated files that are *not* embedded, §7.5's
//! recovered cross-reference table, Annex I's version,
//! §14.8.6.2's namespaces and §14.8.6.3's unenclosed `MathML` are all
//! claims about the *file*, and a person deciding whether to trust what they are looking at needs them before any
//! page is drawn. That is why they are a [`crate::Event::Reported`] with no page rather than
//! part of the page's own report.
//!
//! Nothing here is an error and nothing here stops a document opening.

use pdf_signature::signature::{
    PadesDeparture, PadesProfile, ReferenceDigest, SigningCertificateBinding,
};
use pdf_syntax::Document;

/// Everything worth saying about a document the moment it opens.
pub(crate) fn about(document: &Document, trust: &crate::TrustPolicy) -> Vec<String> {
    let mut notes = Vec::new();

    if document.was_recovered() {
        // Worth saying: the file's own cross-reference table was unusable and the document was
        // reconstructed by scanning. It may still be missing content.
        //
        // And where the file packs its objects into §7.5.7's streams, the rebuild has a second
        // half whose result is not all-or-nothing: it enters what each stream's own header names
        // and reports the streams it could not read. **A rebuild that recovered some of a file
        // must not read like one that recovered all of it**, so the sentence carries both counts
        // rather than stopping at the good news.
        let recovered = document.compressed_objects_recovered();
        let compressed = if recovered.is_empty() {
            String::new()
        } else if recovered.is_whole() {
            format!(
                ", including {} object(s) stored inside {} object stream(s) (§7.5.7)",
                recovered.objects, recovered.read
            )
        } else {
            format!(
                ", including {} object(s) stored inside {} of its {} object streams (§7.5.7) — \
                 the other {} could not be read, so what they hold is missing from what you see",
                recovered.objects,
                recovered.read,
                recovered.streams,
                recovered
                    .unreadable
                    .saturating_add(recovered.beyond_the_budget),
            )
        };
        notes.push(format!(
            "this file's cross-reference table was broken and was rebuilt by scanning{compressed}"
        ));
    }

    // §7.5.8's cross-reference stream states its own length twice over — Table 17 makes `/W`'s
    // sum "the total length of each entry" and `/Index` the number of entries — and §7.3.8.2
    // requires the two to agree with the data: "[a]ll of these constraints shall be consistent."
    // Where they do not, the entries the data carries are whole and are the producer's own, and
    // the rest name object numbers this reader knows *nothing* about. That is worth saying
    // because everywhere else in this reader a number with no entry has been deleted (§7.5.6),
    // and these were not: what a page cannot find may be in the file and unreachable. ADR 0366.
    let entries_lost = document.cross_reference_entries_lost();
    if entries_lost > 0 {
        notes.push(format!(
            "this file's cross-reference stream states {entries_lost} more entries than its data \
             carries (§7.5.8), so that many object numbers are unknown here rather than deleted — \
             anything they name is missing from what you see"
        ));
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

    // §12.11.3's own threshold, which this note said nothing about until the
    // six-hundred-and-twenty-sixth session because three places in this tree recorded that the
    // clause states none:
    //
    // > In the situation where the penalty values are being used to evaluate the presentation of
    // > the base PDF document, and there exist no other alternates, if the penalty value exceeds
    // > 100 then the PDF processor should not attempt to display or process the document.
    //
    // `requirements::penalty_total` performs the computation §12.11.6 sends a processor to
    // §12.11.3 for. Saying the number rather than acting on it is this program's choice, and it
    // is the shape `CLAUDE.md` principle 3 asks a document's restriction to take: a host that
    // later wants the clause's `should` obeyed, or wants to ask first, has the number here and
    // needs nothing from `pdf-model`. The `should` is the standard's own word, so declining it
    // costs conformance nothing.
    let total = pdf_model::requirements::penalty_total(document);
    if total > pdf_model::requirements::PENALTY_LIMIT {
        notes.push(format!(
            "the requirements this document states and this program cannot meet total {total} \
             penalty points (§12.11.3), over the 100 above which the clause says a processor \
             should not attempt to display it — it is being displayed anyway, with each one named"
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

    // §14.13.2's *other* form, and the reason it is said here rather than listed with the files
    // above: an associated file's specification "represents either a file external to the PDF file
    // or an embedded file stream", and an external one has no bytes in the document at all. This
    // program cannot follow it — principle 3 gives the renderer no filesystem — but naming it
    // needs no filesystem, and §7.11's own reading is that refusing a file and being unable to
    // name it are different things. A document that associates a file nobody here can reach is
    // exactly the kind of fact this module exists for: what you are looking at is not the whole of
    // what the producer assembled. **The catalog is the scope, which is the scope the embedded
    // list above already has** — §14.13.4 to §14.13.9's other carriers are read by
    // `attachment::external_associated` and reach no caller. No corpus document states one, so the
    // witness is hand-built (trap 8); ADR 0918 has the counts.
    if let Ok(catalog) = document.catalog() {
        for file in pdf_model::attachment::external_associated(document, &catalog) {
            let named = if file.name.is_empty() {
                "a file it does not name"
            } else {
                file.name.as_str()
            };
            notes.push(format!(
                "this document associates a file that is not inside it: {named} \
                 ({}, §14.13.2) — it is outside this program's reach and nothing of it is shown",
                file.relationship.as_str()
            ));
        }
    }

    tagged_structure(document, &mut notes);
    signatures(document, trust, &mut notes);
    notes
}

/// §14.8.6's two requirements on the file, said where the file is what is being described.
///
/// > In a tagged PDF, all structure elements shall be in at least one of the standard structure
/// > namespaces or in a namespace identified in 14.8.6.3 , ' Other namespaces '.
///
/// and, in the subclause that sentence points at, Errata Collection 3's replacement for
/// §14.8.6.3's `MathML` sentence (Issues #72 and #719), which requires the `math` structure element
/// type to be used to enclose the formula under the `Formula` structure element type —
/// `pdf_model::structure::Tree::mathml_outside_a_formula` quotes the 2020 sentence it replaced
/// and says what the erratum changed.
///
/// Every other sentence of §14.8.6 is addressed to a reader and is carried out — which map
/// applies to which element, the default namespace for an element that states none, and what a
/// name in a foreign namespace *means* — and these two are addressed to whoever wrote the file.
/// So they belong here rather than in a page's report, for the reason this module exists: each is
/// a claim about the document, and neither costs a mark on any page, so a page report would take
/// a page out of the oracle's diagnosed set to say something that is not about it.
///
/// **The second one used to be filed as a producer's and therefore nobody's**, on `CLAUDE.md`'s
/// closed authoring exclusion. The exclusion says this tree does not *write* such a tagging; it
/// says nothing about reading one, and a `shall` a file breaks is answered by a report here — the
/// sentence above is the precedent, one subclause away. ADR 0786.
///
/// **Two conditions, both the clause's.** §14.8.1 is what makes a document tagged — "[a] tagged
/// PDF document shall contain a mark information dictionary … with a value of true for the Marked
/// entry" — and a document with a structure tree that does not claim to be tagged is outside the
/// sentence above, which says *in a tagged PDF*. And the elements are read only where the
/// structure tree root declares a namespace outside the permitted set, which
/// [`pdf_model::structure::Tree::namespaces_outside_the_standard`] argues from the clause and
/// prices in milliseconds.
fn tagged_structure(document: &Document, notes: &mut Vec<String>) {
    if !pdf_model::structure::MarkInfo::read(document).marked {
        return;
    }
    let Some(tree) = pdf_model::structure::Tree::of(document) else {
        return;
    };
    // §14.8.6.3's first `shall`, asked of the same tree and gated the same way. It is a separate
    // sentence about a separate namespace, so it is a separate note rather than a clause of the
    // one below: a document can break either without breaking the other.
    let unenclosed = tree.mathml_outside_a_formula(document);
    if unenclosed > 0 {
        notes.push(format!(
            "this document says it is tagged (§14.8.1), and {unenclosed} of its structure \
             elements are §14.8.6.3's MathML `math` type with no Formula element above them — \
             the subclause requires the math type to be used to enclose the formula under the \
             Formula structure element type, so a reader is told this is mathematics without \
             being told which formula it is"
        ));
    }
    for foreign in tree.namespaces_outside_the_standard(document) {
        let which = foreign.name.as_deref().map_or_else(
            || {
                "a namespace whose dictionary states no name of its own (§14.7.4.2's Table 356 \
                requires one)"
                    .to_owned()
            },
            |name| format!("the namespace {name}"),
        );
        notes.push(format!(
            "this document says it is tagged (§14.8.1), and {} of its structure elements end in \
             {which} — §14.8.6.2 requires every one of them to be in a standard structure \
             namespace, in §14.8.6.3's MathML, or role mapped into one, so this reader cannot \
             tell what those elements are and reads them by the names the document wrote",
            foreign.elements
        ));
    }
}

/// What a damaged object stream has cost this document so far, said once per loss.
///
/// **The third channel, and the reason it is not one of the two above.** §7.5.7 lets a file store
/// indirect objects inside a stream, and a stream that decodes only in part yields the objects its
/// prefix wholly carries and no others — the refusal is `pdf_syntax`'s and ADR 0366 argues it. What
/// is lost is a fact about the *file*, like [`about`]'s rebuilt table, but it cannot be said when
/// the file opens: nothing expands an object stream until an object inside it is asked for, and
/// making that eager is exactly what `CLAUDE.md`'s startup rule forbids. So it is said when it
/// becomes known, which is after whichever page first reaches into such a stream.
///
/// Deliberately **not** part of a page's report: `pdf_model::interpret` is a pure function of the
/// file and the page, and a note that fired there would depend on which pages had been read
/// before it. The cost of that choice is that no gate sees this sentence, and the benefit is that
/// no page leaves the oracle's judged set for a fact that is not about a page.
pub(crate) fn losses(open: &mut crate::open::Open) -> Vec<String> {
    let mut notes = Vec::new();
    // **A scan the process could not hold the file for**, said once, when the reader first
    // records it (ADR 0812). The same channel as the object-stream losses below and for the same
    // reason: a scan runs when an object the table names is not where it says, which is at
    // whichever page first asks for one — and a document this reader opened on disk at the cost
    // of its trailer is a document it may not be able to hold whole when that happens. What the
    // reader did instead is read the file as far as its table was right, and no further; the
    // object it could not find is nothing, and the page draws without it. The sentence carries
    // the length because the length is the reason, and it is the pattern the locked and the
    // pageless document already use: a fact about the file, worded once, on the document's
    // report rather than a page's.
    if !open.scan_refusal_said
        && let Some(refused) = open.document.scan_refused()
    {
        open.scan_refusal_said = true;
        notes.push(scan_refused(refused));
    }

    let lost = open.document.objects_lost_to_damage();
    if lost.count() <= open.losses_said {
        return notes;
    }
    open.losses_said = lost.count();
    let named = if lost.objects.is_empty() {
        String::new()
    } else {
        format!(", among them {:?}", lost.objects)
    };
    notes.push(format!(
        "{} object(s) this file stores inside an object stream (§7.5.7) could not be read, \
         because the stream decoded only as far as its damage and the rest of each object is not \
         in the file{named} — what refers to them draws without them",
        lost.count(),
    ));
    notes
}

/// The sentence for a scan the process could not hold the file for.
///
/// §C.4 (informative) licenses the scan — "[w]hen a PDF processor reads a PDF file with a
/// damaged or missing cross-reference table" — and a scan reads every byte, which the reader
/// asks for whole and is refused by name (`pdf_syntax::NoRoom`, ADR 0809). Worded here so that
/// the words say what the reader did rather than what it could not do: read as far as the table
/// was right.
fn scan_refused(refused: pdf_syntax::NoRoom) -> String {
    format!(
        "this file's cross-reference table names an object that is not where it says, and the \
         scan that would find it needs the whole file in memory — {} bytes, which this process \
         cannot hold — so the file was read as far as its table was right and what the table \
         misplaces draws as nothing (§7.5.4, §C.4)",
        refused.length
    )
}

/// What became of an operation a document restricts, for the sentences that say why.
///
/// One value per verdict a window can receive, because the *tail* of every sentence
/// [`restricted`] words depends on it: a reason that ends "it was not done" is a lie under
/// *warn*, where it was, and premature under *ask*, where nobody has answered yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    /// `Level::On`: the operation was not performed.
    Refused,
    /// `Level::Ask`: the operation is held until the person answers.
    Asked,
    /// `Level::Warn`: the operation was performed and the reasons are said after it.
    Warned,
}

impl Standing {
    /// The clause of the sentence that says what happened to the operation.
    fn tail(self, operation: pdf_model::restriction::Operation) -> String {
        match self {
            Self::Refused => format!("{} was not done", operation.as_str()),
            Self::Asked => format!("{} is waiting on your answer", operation.as_str()),
            Self::Warned => format!(
                "{} was done anyway, because this reader is set to warn rather than obey",
                operation.as_str()
            ),
        }
    }
}

/// Why a document does not permit an operation, in sentences a host can show, and what became
/// of it.
///
/// The other half of [`about`], and the same vocabulary: what a document asserts is read by
/// `pdf_model::restriction`, which answers with clauses and levels, and the words are here
/// because words about a document belong where the rest of this program's words about one are.
/// One sentence per restriction, in the order `pdf_model::restriction::asserted` found them,
/// each ending in [`Standing`]'s clause.
///
/// **A function of the restrictions rather than of the document**, since the
/// eight-hundred-and-seventy-second session: the reading and the policy are both
/// `pdf_model::restriction::decide`'s, and what reaches here is what the verdict carried. Since
/// the eight-hundred-and-eighty-fifth the same list is worded for all three verdicts a window
/// receives, which is what that session's comment here said it would need (ADR 0814).
pub(crate) fn restricted(
    operation: pdf_model::restriction::Operation,
    restrictions: &[pdf_model::restriction::Restriction],
    standing: Standing,
) -> Vec<String> {
    use pdf_model::restriction::Restriction;
    use pdf_signature::signature::Modification;

    let tail = standing.tail(operation);
    restrictions
        .iter()
        .map(|restriction| match *restriction {
            // §12.8.2.2.1's parenthesis is a `shall` addressed to a processor that modifies:
            // "(These changes to the document shall also be prevented if the signature
            // dictionary is referred from the DocMDP entry in the permissions dictionary.)"
            Restriction::Certified { level } => match level {
                Modification::None => format!(
                    "this document's author certified it as final (§12.8.2.2's /P 1), so it \
                     permits no change at all — {tail}"
                ),
                Modification::FormFilling => format!(
                    "this document's author permitted only form filling and signing (§12.8.2.2's \
                     /P 2), which does not include {} — {tail}",
                    operation.as_str()
                ),
                // Level 3 and an undefined level both permit, so `asserted` never names them
                // here; saying which one arrived is better than a sentence that claims a rule.
                other => format!("this document's /DocMDP states {other:?} — {tail}"),
            },
            // §7.6.4.2's Table 22, and §7.6.4.1's sentence about it: "PDF readers shall respect
            // the intent of the document creator by restricting user access to an encrypted PDF
            // file according to the permissions contained in the file."
            Restriction::AccessDenied { bit } => format!(
                "this document's encryption does not grant {} (§7.6.4.2's Table 22, bit {}) — \
                 {tail}",
                operation.as_str(),
                bit.position()
            ),
            // §12.7.5.5: "The signature field lock dictionary … contains the names of form
            // fields whose values shall no longer be changed after this signature has been
            // signed." The sentence names the *signature* as what locks the field, which is
            // what a person is owed here: the refusal is somebody's signature rather than the
            // document's encryption or its author's certification.
            Restriction::FieldLocked => format!(
                "a signature in this document locks this field against further change \
                 (§12.7.5.5's /Lock) — {tail}"
            ),
            // §12.8.2.4 states a consequence where §12.7.5.5 states a prohibition, and the
            // sentence keeps them apart: "any modifications to specific form fields shall
            // invalidate that recipient's signature". So this one does not say the document
            // forbids the edit — it says what the edit costs, which is what the clause says.
            Restriction::FieldCovered => format!(
                "a signature in this document covers this field, and changing it invalidates \
                 that signature (§12.8.2.4's FieldMDP) — {tail}"
            ),
            // §12.5.3's Table 167 bit 10: "If set, do not allow the contents of the annotation to
            // be modified by the user." Bit 8's `Locked` is named in the same breath because it
            // is the flag a person would expect to be the reason and is not — its own row says it
            // "does not restrict changes to the annotation's contents".
            Restriction::AnnotationLocked => format!(
                "this annotation is marked LockedContents (§12.5.3's Table 167, bit 10), so its \
                 text may not be changed — {tail}"
            ),
        })
        .collect()
}

/// §12.8's signatures: what this program can honestly say about one, given what a host supplied.
///
/// **A signature asks three questions** (§12.8.1, ADR 0215), and until the
/// one-thousand-and-sixty-second session this program answered two of them for every document
/// there was. It says who signed, why, whether the range they signed runs to the end of the file
/// (§12.8.1), whether the bytes that range names still hash to the digest the signature records,
/// and — since the three-hundred-and-ninety-second session — **whether the signature verifies
/// under the public key in the certificate the file itself carries** (§12.8.3.3.1).
///
/// **The third is answered when, and only when, a host has named an anchor.** RFC 5280 section
/// 6.1.1 makes the anchors input (d) and a matter of policy, ADR 1039 made them a host's to supply
/// and this crate's to receive, and [`crate::Command::Trust`] is the receiving. With none — which
/// is every host's default and what a host that says nothing gets — every sentence below is the
/// one this program has printed since the three-hundred-and-seventy-seventh session, the closing
/// paragraph included.
///
/// **And a verdict is never separable from where its anchors came from.** [`anchors_note`] is that
/// sentence: a reader told that a signature is valid is owed *valid according to whom*, and only
/// the host that read the certificates can say.
fn signatures(document: &Document, trust: &crate::TrustPolicy, notes: &mut Vec<String>) {
    // Read once and lent on: §12.8.6's dictionary says which signature carries §12.8.2.2's
    // `/DocMDP` and which carries §12.8.2.3's `/UR3`, which is what decides whether a comparison of
    // two revisions has a transform to be ranked against — and which two of the signatures below
    // are reachable from nowhere else.
    let permissions = pdf_signature::signature::permissions(document);
    let signatures = every_signature(&permissions, document);
    if signatures.is_empty() {
        return;
    }
    let reading = trust.anchors.read();
    // §12.8.4's store and §12.8.5's chain are read once here and lent to the three functions that
    // word them: `Signature::trust` needs the material, the chain needs it too, and a verdict on a
    // signature needs to know whether the document states a time this program could establish.
    let store = pdf_signature::signature::security_store(document);
    let material = store.material();
    let chain =
        pdf_signature::timestamp::chain(document, &reading.anchors, &material, trust.anchors.at());
    let length = document.bytes().len() as u64;
    let timestamps = standing(&chain);
    for (index, signature) in signatures.iter().enumerate() {
        // §12.8.3.4.5 (b) names three ways a signature may be verified against a time other than
        // the current one — the security store, a document timestamp, and "a signature timestamp
        // … present in the signature as an unsigned attribute" — and states no order among them.
        // §12.8.4.2 settles the second and §12.8.3.4.8's first sentence settles the third, so the
        // chain is asked first because it is a statement about the *document* and the attribute
        // second because it is a statement about one signature. Both are instants only once an
        // authority is established, which is a host's anchor away (ADR 1039).
        let established = established_over(&chain, index)
            .map(|at| (at, "a document timestamp over it established (§12.8.4.2)"))
            .or_else(|| {
                signature_timestamps_instant(
                    signature,
                    &reading.anchors,
                    &material,
                    trust.anchors.at(),
                )
                .map(|at| {
                    (
                        at,
                        "this signature's own timestamp attribute established (§12.8.3.4.8)",
                    )
                })
            });
        let asked = Asked {
            anchors: &reading.anchors,
            material: &material,
            at: established.map_or_else(|| trust.anchors.at(), |(at, _)| at),
            from_a_token: established.map(|(_, source)| source),
            acceptance: trust.acceptance,
            timestamps,
        };
        about_one(signature, document, length, &asked, notes);
        modifications(signature, document, &permissions, notes);
    }
    permissions_stated(&permissions, notes);
    security_store(&store, &material, notes);
    document_timestamps(&chain, notes);
    anchors_note(trust, &reading, notes);
    // The three questions, named, in the order §12.8.1 states them. This paragraph is what stops
    // the sentences above it being read as "the signature is good": where nobody named an anchor
    // two of them are answered, and the third is the one that decides whether a signature means
    // anything about a *person*, because a key that verifies is a key and this program has nothing
    // to say about whose.
    //
    // **Keyed on what *read*, not on what was handed over.** A host that named a directory of six
    // files none of which is a certificate has supplied no anchor, and the paragraph below would
    // otherwise tell a reader that the third question was asked when nothing could ask it.
    notes.push(if reading.anchors.is_empty() {
        "of the three questions a signature asks, this program answers two: whether the document \
         changed since it was signed (§12.8.1's digest, recomputed above) and whether the \
         signature verifies under the public key in the certificate the file itself carries \
         (§12.8.3.3.1). It does not answer the third — it has no certificate store and makes no \
         network request, so it does not know whether that certificate belongs to anyone you have \
         reason to believe. It can now say whether a certificate was revoked, but only from the \
         material the file itself carries and only once somebody names a certification authority \
         to end the chain at, which nothing here does. A signature that verifies here was \
         made by whoever holds the key in a certificate that arrived with the document, which is \
         not the same as a valid signature. Nothing here says valid"
            .to_owned()
    } else {
        "all three of the questions a signature asks were asked of this document: whether it \
         changed since it was signed (§12.8.1's digest, recomputed above), whether the signature \
         verifies under the public key in the certificate the file carries (§12.8.3.3.1), and \
         whether a certification path from that certificate reaches an authority you named (RFC \
         5280 section 6.1). The third was asked because you named one; it is your answer this \
         program is applying and not one of its own, it still makes no network request, and \
         whether a certificate was revoked is answered from the material this file itself carries \
         and from nothing else"
            .to_owned()
    });
}

/// What §12.8.1's third question needs, gathered once per document and lent to every signature.
///
/// A struct rather than a row of parameters because these travel together and are meaningless
/// apart: the anchors without the instant cannot validate a period, and the acceptance without the
/// material decides nothing.
struct Asked<'a> {
    /// RFC 5280 section 6.1.1's input (d), as the host supplied it.
    anchors: &'a pdf_signature::trust::TrustAnchors<'a>,
    /// §12.8.4's CRLs and OCSP responses, out of this document.
    material: &'a pdf_signature::revocation::Material<'a>,
    /// Section 6.1.1's input (b), and which instant it is depends on the document.
    at: pdf_signature::x509::Instant,
    /// Where [`Self::at`] came from, where it is not the host's clock — the sentence naming it.
    ///
    /// §12.8.3.4.5 (b) admits three sources for a past instant and this says which one answered,
    /// because a reader told a path was validated is owed *as of when* and *on whose word*.
    from_a_token: Option<&'static str>,
    /// What the host will accept where the material answers nothing.
    acceptance: pdf_signature::verdict::Acceptance,
    /// What §12.8.5's chain contributes, computed once for the document.
    timestamps: pdf_signature::verdict::Timestamps,
}

/// What §12.8.5's chain contributes to a verdict on any signature in this document.
///
/// §12.8.5 is optional, so a document with no chain reserves nothing. A document *with* one states
/// a time, and a time this program could not establish is a statement it could not check — which
/// ADR 1071 makes a reservation rather than a detail.
fn standing(chain: &pdf_signature::timestamp::Chain) -> pdf_signature::verdict::Timestamps {
    use pdf_signature::verdict::Timestamps;
    match chain.outermost() {
        None => Timestamps::None,
        // Anything but an established instant is a time this program did not establish, which is
        // the conservative reading and the one a variant added later should get by default.
        Some(link) => match link.time {
            pdf_signature::timestamp::Time::Established { at, .. } => Timestamps::Established(at),
            _ => Timestamps::NotEstablished,
        },
    }
}

/// The instant a signature's certification path is validated at, where the document fixes one.
///
/// **§12.8.4.2's second `shall`, and it is the whole reason long-term validation works**: "the UTC
/// time included in that timestamp token shall be used as the time reference to check the
/// revocation status of the signer's certificate and of all the intermediate CA certificates, up to
/// a trusted root". A signature whose certificate expired years ago cannot be checked against
/// today's clock — RFC 5280 section 6.1.3 (a)(2) refuses the path before revocation is reached —
/// and §12.8.5's stack of tokens exists to say what the file looked like while it was still current.
///
/// The innermost established timestamp covering this signature is the one taken, because that is
/// the closest statement anybody made about the moment the signature was still being relied on. A
/// token whose own authority this program could not establish supplies nothing (ADR 1071), which is
/// why this answers `None` without an anchor and every verdict falls back to the host's clock.
fn established_over(
    chain: &pdf_signature::timestamp::Chain,
    signature: usize,
) -> Option<pdf_signature::x509::Instant> {
    chain
        .links
        .iter()
        .filter(|link| link.covers.contains(&signature))
        .find_map(|link| match link.time {
            pdf_signature::timestamp::Time::Established { at, .. } => Some(at),
            _ => None,
        })
}

/// The instant a signature's own timestamp attribute establishes, where it carries one.
///
/// §12.8.3.4.8:
///
/// > When a timestamp token is already present in the CAdES signature as a signature timestamp
/// > attribute (it is an unsigned attribute), the signer's signature shall be verified at the UTC
/// > time in the past indicated in that token.
///
/// Two conditions, not one, and ETSI EN 319 122-1 clause 5.3 is why the second exists: the clause
/// puts the token's imprint over the `SignerInfo`'s `signature` field, so a token whose imprint is
/// a digest of something else is a statement about a different signature and says nothing about
/// this one's past. `None` unless the token both establishes an instant and is about this
/// signature — which, with no anchor supplied, is every document in this tree.
///
/// **`AskedAt::TheCallersInstant` is the authority's own path being asked about *now*.** The
/// reasoning `pdf_signature::timestamp::AskedAt` records for a document timestamp does not reach
/// here: there is no later token in the file that established this one, so there is nothing to ask
/// it at but the moment the question is being put.
fn signature_timestamps_instant(
    signature: &pdf_signature::signature::Signature,
    anchors: &pdf_signature::trust::TrustAnchors<'_>,
    material: &pdf_signature::revocation::Material<'_>,
    now: pdf_signature::x509::Instant,
) -> Option<pdf_signature::x509::Instant> {
    let cms = signature.signed_data().ok()?;
    if !pdf_signature::timestamp::signature_timestamp(&cms)?
        .ok()?
        .covers_the_signature
    {
        return None;
    }
    match pdf_signature::timestamp::signature_timestamp_established(
        &cms,
        anchors,
        material,
        pdf_signature::timestamp::AskedAt::TheCallersInstant(now),
    )? {
        pdf_signature::timestamp::Time::Established { at, .. } => Some(at),
        // Every other answer, including one a later build adds: an instant is established or it is
        // not, and a variant this build has no name for is not a reason to believe a `genTime`.
        _ => None,
    }
}

/// Where the anchors came from, said once, beside every verdict that rests on them.
///
/// **A verdict is not separable from its source and this is why the sentence is not optional.** RFC
/// 5280 section 6.1 says an anchor is believable because "it was delivered to the path processing
/// procedure by some trustworthy out-of-band procedure" — a procedure this program did not perform
/// and cannot describe. So the host describes it, `pdf_signature::trust::Supply::source` carries the
/// description, and a reader who wants to know *valid according to whom* reads it here.
///
/// The certificates a host handed over that this reader would not take are named one by one rather
/// than counted (trap 5): a person who pointed at a directory of six and got four anchors would
/// otherwise be reading a verdict computed under a store they did not supply.
fn anchors_note(
    trust: &crate::TrustPolicy,
    reading: &pdf_signature::trust::Reading<'_>,
    notes: &mut Vec<String>,
) {
    if trust.anchors.is_empty() {
        return;
    }
    notes.push(format!(
        "the certification authorities this reader will end a path at were supplied by whoever \
         started it, from {}: {} of {} read as RFC 5280 certificates. Nothing about them was \
         checked here — an anchor is trusted because of where it came from, which is not this \
         program's to know",
        trust.anchors.source(),
        reading.anchors.len(),
        trust.anchors.len(),
    ));
    for refusal in &reading.refused {
        notes.push(format!("{refusal}"));
    }
    if trust.acceptance == pdf_signature::verdict::Acceptance::UnknownRevocationAccepted {
        notes.push(
            "and this reader was told to accept a signature whose revocation status it could not \
             determine — §12.8.4's material in the file answers what it answers, and what it does \
             not answer is being passed over by your instruction rather than by this program"
                .to_owned(),
        );
    }
}

/// §12.8.4's document security store: what the file carries for a validation later on.
///
/// **The condition is the clause's and the sentence is a count**, which is what §12.8.4.2 makes
/// this worth saying at all: "[a] PDF signature may not be successfully verified unless its
/// collateral validation components are preserved, e.g., certificates, CRLs, timestamp tokens,
/// revocation lists, and OCSP responses." Whether a document preserved them is a fact about the
/// document, it decides whether a signature outlives its certificate, and no other sentence in
/// this report says it.
///
/// **What it may not say is that a signature is unrevoked.** The material is read and applied
/// (ADR 1067), but RFC 5280 section 6.3.3 applies it to a *certification path*, and a path ends at
/// an anchor nobody in this tree supplies (ADR 1039). So this names what is there and what could
/// not be read, and the closing paragraph says what is still missing.
fn security_store(
    store: &pdf_signature::signature::SecurityStore,
    material: &pdf_signature::revocation::Material<'_>,
    notes: &mut Vec<String>,
) {
    if store.is_empty() {
        return;
    }
    notes.push(format!(
        "this document carries a §12.8.4 document security store — the material a validator needs \
         after the signer's certificate has expired: {} certificate(s), {} certificate revocation \
         list(s) and {} OCSP response(s), with §12.8.4.4 validation information recorded for {} \
         signature(s). {} of those lists and responses read as RFC 5280 and RFC 6960 structures",
        store.certificates.len(),
        store.revocation_lists.len(),
        store.ocsp_responses.len(),
        store.validation_information.len(),
        material
            .lists
            .len()
            .saturating_add(material.responses.len()),
    ));
    // Each refusal by name rather than a count of them, for the reason `pdf_signature::revocation`
    // states: material this program could only half read is the difference between an answer about
    // a certificate and no answer at all, and which piece was lost decides which certificate.
    for refusal in &store.refused {
        notes.push(format!("{refusal}"));
    }
    for refusal in &material.refused {
        notes.push(format!(
            "one of this document's security store entries is not usable: {refusal}"
        ));
    }
    // Table 262's `/TS`: "A stream containing the DER-encoded timestamp (see Internet RFC 3161 as
    // updated by Internet RFC 5816 ) that contains the date/time at which this signature VRI
    // dictionary was created." What the table says it is *for* is the sentence this reports:
    // "NOTE 1 The date/time contained in the timestamp token can be used for audit purposes."
    for vri in &store.validation_information {
        let Some(token) = &vri.timestamp else {
            continue;
        };
        let inner = pdf_signature::cms::signed_data(token)
            .ok()
            .as_ref()
            .map(pdf_signature::timestamp::token_of);
        notes.push(match inner {
            Some(Ok(info)) => format!(
                "the §12.8.4.4 validation information for signature {} was recorded at {}, by a                  timestamp token (Table 262's TS). When that was is the token's claim, not this                  program's finding",
                vri.signature_digest,
                String::from_utf8_lossy(info.gen_time_as_written)
            ),
            Some(Err(refusal)) => format!(
                "the §12.8.4.4 validation information for signature {} carries a Table 262 TS                  timestamp this program will not read: {refusal}",
                vri.signature_digest
            ),
            None => format!(
                "the §12.8.4.4 validation information for signature {} carries a Table 262 TS                  entry that is not an RFC 5652 SignedData",
                vri.signature_digest
            ),
        });
    }
}

/// §12.8.5's document timestamps, in the order their ranges nest, and what each one covers.
///
/// **The condition is the clause's**: §12.8.5.2 says how a reader finds them — "[t]he existence of
/// one or more document timestamps shall be determined by examining signature fields" — and
/// §12.8.5.3 says why there is more than one, which is the sentence this report exists to make
/// legible. A later token is applied "before the expiry of the certificate and/or before a
/// cryptographic attack may succeed", over everything the earlier one left, and what a person
/// wants to know is whether the stack actually holds: does the newest one cover the older ones,
/// and does it cover the material §12.8.4's store carries for them.
///
/// **What it may not say is a time.** §12.8.5.1 makes a timestamp's whole point the instant — "[a]
/// document timestamp dictionary establishes the exact contents of the complete PDF file at the
/// time indicated in the timestamp token" — and *establishing* one needs an authority somebody
/// trusts, which is §12.8.1's third question and nobody's answer here (ADR 1039, ADR 1071). So the
/// token's `genTime` is reported as the token's own characters, the way `/M` is, and the sentence
/// after it says what is missing.
fn document_timestamps(chain: &pdf_signature::timestamp::Chain, notes: &mut Vec<String>) {
    if chain.is_empty() {
        return;
    }
    notes.push(format!(
        "this document carries {} §12.8.5 document timestamp(s), which §12.8.5.3 stacks so that \
         each protects the structure the one before it left",
        chain.links.len()
    ));
    for (index, link) in chain.links.iter().enumerate() {
        let claim = match &link.claim {
            Ok(claim) => format!(
                "states genTime {}{}",
                claim.stated,
                claim.accuracy.map_or_else(String::new, |accuracy| format!(
                    ", accurate to {} microsecond(s)",
                    accuracy.micros_total()
                ))
            ),
            Err(refusal) => format!("carries a token this reader will not read: {refusal}"),
        };
        let material_total = link
            .material_covered
            .len()
            .saturating_add(link.material_uncovered.len());
        notes.push(format!(
            "document timestamp {} of {} {claim}; its byte range runs to byte {} of this file, \
             covers {} of the document's earlier signature dictionaries and {} of the \
             {material_total} piece(s) of §12.8.4 validation material, and the bytes it names {}",
            index.saturating_add(1),
            chain.links.len(),
            link.covers_to,
            link.covers.len(),
            link.material_covered.len(),
            match link.integrity {
                pdf_signature::signature::Integrity::Unchanged { .. } =>
                    "still hash to the imprint inside the token (Table 255)",
                pdf_signature::signature::Integrity::Changed { .. } =>
                    "no longer hash to the imprint inside the token, so they moved after it was applied",
                _ => "could not be hashed against the imprint inside the token",
            }
        ));
    }
    // Each refusal by name, for the reason the store's refusals are named one function up: which
    // piece a chain fails to protect decides what a person can conclude about which signature.
    for refusal in &chain.refused {
        notes.push(format!("{refusal}"));
    }
    // **And whether any of those claims is an instant.** §12.8.5.2's authority is a *trusted* one,
    // so the four steps ADR 1071 states end at an anchor — and which of the two sentences below is
    // true of this run is decided by whether a host named one.
    notes.push(match chain.outermost().map(|link| &link.time) {
        Some(pdf_signature::timestamp::Time::Established { at, .. }) => format!(
            "the outermost of those timestamps establishes an instant: its authority's \
             certification path reaches an anchor you supplied, its token's signature verifies, \
             and the token's contents are bound to that signature (§12.8.5.2, RFC 3161). The \
             instant is {} seconds after the epoch. What it establishes is the state of the file \
             at that moment and nothing about what the file says",
            at.unix_seconds()
        ),
        _ => "none of those timestamps tells this program *when* the document was in that state. A \
              token's genTime is a statement by a timestamp authority, and believing it means \
              establishing that the authority is one to believe — §12.8.5.2's \"trusted timestamp \
              authority\", which is §12.8.1's third question. This program verifies the token's own \
              signature and builds the authority's certification path, and stops where that path \
              would end: nobody has named a certification authority to end it at. So the times \
              above are claims the file makes, and nothing here says otherwise"
            .to_owned(),
    });
}

/// Every signature dictionary the document holds, from both places §12.8.1 puts one.
///
/// §12.8.1 puts a usage rights signature's dictionary in the permissions dictionary "(not from a
/// signature field)", so the field walk cannot reach one and three corpus documents carry nothing
/// else. A certification signature is normally in both and is said once.
fn every_signature(
    permissions: &pdf_signature::signature::Permissions,
    document: &Document,
) -> Vec<pdf_signature::signature::Signature> {
    let mut signatures = pdf_signature::signature::signatures(document);
    for extra in [
        permissions.usage_rights_signature.clone(),
        permissions.doc_mdp_signature.clone(),
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

/// What a signature's `/ByteRange` leaves out, and where it stops.
///
/// Separated from [`about_one`] because these two readings of §12.8.1 are one sentence of the
/// standard and grow together; the report they build is the same list.
#[expect(
    clippy::match_same_arms,
    reason = "the two empty arms are different claims and each carries its own reason: the \
first is the clause satisfied and nothing to report, the last is a malformed range the \
`Coverage::Malformed` arm has already reported in words a person can act on. Merging them \
would delete that distinction"
)]
fn range_notes(
    signature: &pdf_signature::signature::Signature,
    document: &Document,
    notes: &mut Vec<String>,
) {
    // **What the range leaves *out*, which the match above cannot see.** `Coverage` is
    // arithmetic over the pairs; §12.8.1 and Table 255 also say what may sit in the region
    // between two of them — "the signature value itself (the Contents entry)" — and a region
    // holding anything else is unsigned content the reader parses. Said out loud because
    // every other sentence in this report stays green on such a file: the pairs cover it, the
    // digest matches, the signature verifies.
    match signature.excluded(document.bytes()) {
        pdf_signature::signature::Excluded::TheSignatureValue => {}
        // §12.8.3.3.1: the value "shall fit precisely in the space between the ranges
        // specified by ByteRange", and here it does not — the digits are in the hole and a
        // delimiter is under the digest. Nothing is hidden, so the sentence says what the
        // file did rather than warning about it.
        pdf_signature::signature::Excluded::TheDigitsOfTheSignatureValue => {
            notes.push(
                "that signature's /ByteRange leaves out the digits of its signature value \
                     without the angle brackets around them, which §12.8.3.3.1 asks to fit \
                     precisely in that space — nothing of this file is left unsigned by it"
                    .to_owned(),
            );
        }
        pdf_signature::signature::Excluded::NotTheSignatureValue { at, length } => {
            notes.push(format!(
                "that signature's /ByteRange leaves out {length} bytes at offset {at} that are \
                     not its own signature value — §12.8.1 excludes the /Contents entry and \
                     nothing else, so those bytes are in this file and under no digest"
            ));
        }
        pdf_signature::signature::Excluded::MoreThanOneRegion { regions } => {
            notes.push(format!(
                "that signature's /ByteRange leaves out {regions} separate regions of this \
                     file — §12.8.1 excludes the /Contents entry and nothing else"
            ));
        }
        pdf_signature::signature::Excluded::Nothing => {
            notes.push(
                "that signature's /ByteRange leaves nothing out, so it claims to cover the \
                     signature value it records (§12.8.1 excludes the /Contents entry)"
                    .to_owned(),
            );
        }
        // The range does not describe this file, which the `Coverage::Malformed` arm above
        // has already said in the words a person can act on.
        pdf_signature::signature::Excluded::RangeNotInThisFile
        | pdf_signature::signature::Excluded::RangeNotReadable => {}
    }
    // §12.8.1's other end of the same sentence: the range runs "to the end of the \"%%EOF\"
    // comment, possibly followed by an optional EOL marker, terminating the incremental
    // update that adds the digital signature dictionary". A range stopping anywhere else has
    // signed a prefix of a revision rather than a revision.
    if signature.signed_end(document.bytes()) == pdf_signature::signature::SignedEnd::Elsewhere {
        notes.push(
            "that signature's signed bytes do not stop at an %%EOF marker, so what it signed \
                 is part of a revision rather than a whole one (§12.8.1)"
                .to_owned(),
        );
    }
    // §12.8.1: "When a byte range digest is present, all values in the signature dictionary
}

/// What one signature says, what it covers, and whether the bytes under it moved.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per sentence of §12.8 that this reader can answer about a signature, \
each with the clause it rests on beside it. The length is the size of that vocabulary; \
splitting it further scatters one clause's reading across functions and hides which \
sentences are answered and which are not"
)]
fn about_one(
    signature: &pdf_signature::signature::Signature,
    document: &Document,
    length: u64,
    asked: &Asked<'_>,
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
            pdf_signature::signature::Coverage::WholeFile => {}
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
            pdf_signature::signature::Coverage::Unsigned { tail } => {
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
            pdf_signature::signature::Coverage::Malformed => {
                notes.push("that signature's /ByteRange does not describe this file".to_owned());
            }
        }
        range_notes(signature, document, notes);
        // shall be direct objects." The condition is the clause's — a dictionary with no
        // `/ByteRange` states no byte range digest — and the entries are named rather than
        // counted, because which one was written indirectly is what decides whether it matters.
        if !signature.byte_range.is_empty() && !signature.indirect_values.is_empty() {
            notes.push(format!(
                "that signature writes {} as indirect references, and §12.8.1 requires every \
                 value in a signature dictionary carrying a byte range digest to be a direct \
                 object — an indirect one can be redefined by a later update without moving a \
                 signed byte",
                signature.indirect_values.join(", ")
            ));
        }
        verdicts(signature, document, asked, notes);
        // **Table 255's `/V` is the file saying which part of the validation matters**, and it is
        // the one sentence of that entry addressed to whoever validates: "[t]he value is 1 if the
        // Reference dictionary shall be considered critical to the validation of the signature"
        // (§12.8.1). This program evaluates no transform method — §12.8.2.2.2's comparison of two
        // revisions is what that would take and the ledger records it as not done — so on a file
        // that writes `/V 1` the closing paragraph's "answers two of the three questions" is
        // weaker than it sounds, and the file itself is what says so. The condition is the
        // entry's own and nothing is added to it (trap 11); `/V` absent is the table's default 0.
        if signature.reference_is_critical() {
            notes.push(
                "that signature states /V 1, so this file requires its signature reference \
                 dictionary to be considered critical to validating it (Table 255) — and this \
                 program evaluates no transform method, so nothing said above took it into account"
                    .to_owned(),
            );
        }
        // **Table 256's `/DigestMethod` is the other entry of that family a reader is owed**, and
        // it names the function rather than the fact: "[a] name identifying the algorithm that
        // shall be used when computing the digest if not specified in the certificate". The digest
        // it parameterises is §12.8.2's modification analysis and not the byte range digest, so a
        // file that states one has named the function for a comparison this program does not make
        // — which is worth a sentence for the same reason `/V 1` is, and for no other. Two
        // conditions, both the entry's own and neither added to (trap 11): a name outside the six
        // the table admits is the file departing from its value list, and a name inside them is
        // the file parameterising a comparison that is not made. A dictionary stating no
        // `/DigestMethod` says nothing, which Errata Collection 3's issue #117 makes conforming.
        let outside: Vec<&str> = signature
            .reference_digests
            .iter()
            .filter_map(|digest| match digest {
                ReferenceDigest::NotInTheTable(name) => Some(name.as_str()),
                ReferenceDigest::Stated(_) | ReferenceDigest::Absent => None,
            })
            .collect();
        if !outside.is_empty() {
            notes.push(format!(
                "that signature's reference dictionary names {} as its /DigestMethod, which is \
                 not among the six Table 256 admits — MD5, SHA1, SHA256, SHA384, SHA512 and \
                 RIPEMD160 — so the file has not said which function its modification analysis \
                 is computed under",
                outside.join(", ")
            ));
        }
        let mut stated: Vec<String> = signature
            .reference_digests
            .iter()
            .filter_map(|digest| match digest {
                ReferenceDigest::Stated(digest) => Some(format!("{digest:?}")),
                ReferenceDigest::NotInTheTable(_) | ReferenceDigest::Absent => None,
            })
            .collect();
        stated.dedup();
        if !stated.is_empty() {
            notes.push(format!(
                "that signature's reference dictionary states /DigestMethod {}, which Table 256 \
                 makes the algorithm for computing the digest of its transform method's \
                 modification analysis — and this program makes no such comparison, so the name \
                 is read and said rather than acted on",
                stated.join(", ")
            ));
        }
        // §12.8.3.4's rules on a PAdES signature that need no certificate to check. Silent for
        // every other `/SubFilter`, which is §12.8.3.4.1's own scope.
        if let Ok(cms) = signature.signed_data() {
            // §12.8.3.3.2's revocation information, whose object identifier the clause prints
            // itself. Its presence is the fact this program can state: the CRLs and OCSP
            // responses inside it are what a *validator* would use, and using them is question
            // three.
            //
            // **This comment named `issue17069.pdf` as "the corpus's one witness" and there are
            // three in `doc/pdf.js`**, which the six-hundred-and-forty-first session found by giving the sentence
            // a command: `issue6127.pdf` and `xfa_filled_imm1344e.pdf` carry the attribute too.
            // `examples/signature_algorithm_census` counts and names them, so the number is not
            // written down here or in §12.8.3.3.2's ledger row again.
            //
            // **And "three" was a count of `doc/pdf.js`**, which is what the correction did not
            // say and what the eight-hundred-and-thirtieth session widened: the same census over
            // every document this tree holds finds the attribute on hundreds of signature values
            // in hundreds of documents, nearly all of them in the crawl. Its presence is
            // therefore ordinary rather than rare, which changes nothing this code does — the
            // note fires on the attribute and not on a population — and everything about what a
            // round may conclude from the three names above.
            if cms.has_signed_attribute(pdf_signature::cms::ADBE_REVOCATION_INFO_ARCHIVAL) {
                // **What is inside the attribute is read since the thousand-and-fifty-third
                // session** (ADR 1067), because §12.8.3.3.2 prints the grammar rather than
                // pointing at a document this tree does not hold: `RevocationInfoArchival` with
                // its `crl [0]` and `ocsp [1]` members. So the sentence now says how much material
                // there is, and still says the one thing that has not changed — no certificate
                // here is trusted by anybody, because nothing has named an anchor (ADR 1039).
                let archived = pdf_signature::revocation::archived(&cms);
                notes.push(format!(
                    "that signature carries revocation information with it (§12.8.3.3.2's \
                     adbe-revocationInfoArchival attribute): {} certificate revocation list(s) \
                     and {} OCSP response(s) this program reads, and {} piece(s) it would not \
                     take. It makes no trust decision about any certificate",
                    archived.lists.len(),
                    archived.responses.len(),
                    archived.refused.len(),
                ));
            }
            // §12.8.3.3.1's *other* timestamp, and the one a reader can check without a
            // certificate: RFC 3161 Appendix A puts the imprint over "the value of signature
            // field within SignerInfo", which is a digest this program already has the input to.
            // Whether the authority is anybody is the same third question as everywhere else.
            if let Some(stamped) = pdf_signature::timestamp::signature_timestamp(&cms) {
                notes.push(match stamped {
                    Ok(stamp) => format!(
                        "that signature carries a timestamp of its own (§12.8.3.3.1's unsigned                          signature timestamp attribute, RFC 3161 Appendix A), stating genTime {}.                          The imprint inside it {} the digest of this signature's own bytes.                          Whether the authority that issued it is one to believe is the same                          unanswered question as above",
                        stamp.claim.stated,
                        if stamp.covers_the_signature {
                            "is"
                        } else {
                            "is not"
                        }
                    ),
                    Err(refusal) => format!(
                        "that signature states §12.8.3.3.1's signature timestamp attribute and                          this program will not read what is in it: {refusal}"
                    ),
                });
            }
            for departure in signature.pades_departures(&cms, document.bytes()) {
                // The one departure that names an attribute rather than a rule, worded before the
                // match so that every arm of it can be a sentence rather than a `Cow`.
                let repeated = match departure {
                    PadesDeparture::AttributeStatedMoreThanOnce(attribute) => format!(
                        "ETSI EN 319 122-2 Table 1, which §12.8.3.4.4's two profiles are defined \
                         against, admits at most one {} attribute, and this signature states more",
                        attribute.name()
                    ),
                    // The other departure that names something rather than a rule: which of ITU-T
                    // X.690's distinguished encoding rules the value breaks, worded here for the
                    // same reason.
                    PadesDeparture::ValueNotDerEncoded(rule) => format!(
                        "§12.8.3.4.2 requires the value of /Contents to be a DER-encoded CMS \
                         SignedData object, and this one is not — {rule}"
                    ),
                    _ => String::new(),
                };
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
                        PadesDeparture::NoSigningCertificateAttribute =>
                            "§12.8.3.4.3 (f) requires a signing-certificate or \
                             signing-certificate-v2 signed attribute, and it states neither",
                        PadesDeparture::SignerLocationAndLocationEntry =>
                            "§12.8.3.4.3 (h) says a signature stating a signer-location attribute \
                             shall not also state a /Location entry",
                        PadesDeparture::CounterSignature =>
                            "§12.8.3.4.3 (i) says a counter-signature attribute shall not be used",
                        PadesDeparture::ContentReference =>
                            "§12.8.3.4.3 (i) says a content-reference attribute shall not be used",
                        PadesDeparture::ContentIdentifier =>
                            "§12.8.3.4.3 (i) says a content-identifier attribute shall not be used",
                        PadesDeparture::ContentHints =>
                            "§12.8.3.4.3 (i) says a content-hints attribute shall not be used",
                        // (b), (c) and (j) each state their rule by naming a clause of ETSI
                        // EN 319 122-1 and stating nothing themselves, so every sentence below
                        // names both documents: the one that placed the requirement on a PDF, and
                        // the one that says what the requirement is.
                        PadesDeparture::SignatureTimestampIsSigned =>
                            "§12.8.3.4.3 (b) hands its signature timestamp to ETSI EN 319 122-1 \
                             clause 5.3, which makes that attribute an unsigned one, and this \
                             signature states it among its signed attributes",
                        PadesDeparture::SignatureTimestampNotOneValue =>
                            "§12.8.3.4.3 (b)'s ETSI EN 319 122-1 clause 5.3 gives its signature \
                             timestamp attribute exactly one AttributeValue, and this one holds a \
                             different number",
                        PadesDeparture::SignatureTimestampNotOverTheSignature =>
                            "§12.8.3.4.3 (b)'s ETSI EN 319 122-1 clause 5.3 puts the token's \
                             imprint over the SignerInfo's signature field, and this token's \
                             imprint is a digest of something else",
                        PadesDeparture::SignerLocationIsUnsigned =>
                            "§12.8.3.4.3 (h) hands signer-location to ETSI EN 319 122-1 clause \
                             5.2.5, which makes that attribute a signed one, and this signature \
                             states it among its unsigned attributes",
                        PadesDeparture::SignerLocationNotOneValue =>
                            "§12.8.3.4.3 (h)'s ETSI EN 319 122-1 clause 5.2.5 gives signer-location \
                             exactly one AttributeValue, and this one holds a different number",
                        PadesDeparture::SignerLocationEmpty =>
                            "§12.8.3.4.3 (h)'s ETSI EN 319 122-1 clause 5.2.5 requires a \
                             signer-location to name a country, a locality or a postal address, \
                             and this one names none of the three",
                        PadesDeparture::ContentTimestampIsUnsigned =>
                            "§12.8.3.4.3 (c) hands its content timestamp to ETSI EN 319 122-1 \
                             clause 5.2.8, which makes that attribute a signed one, and this \
                             signature states it among its unsigned attributes",
                        PadesDeparture::ContentTimestampNotOneValue =>
                            "§12.8.3.4.3 (c)'s ETSI EN 319 122-1 clause 5.2.8 gives its content \
                             timestamp attribute exactly one AttributeValue, and this one holds a \
                             different number",
                        PadesDeparture::ContentTimestampNotOverTheSignedBytes =>
                            "§12.8.3.4.3 (c)'s ETSI EN 319 122-1 clause 5.2.8 puts the token's \
                             imprint over the external data of a detached signature, which here \
                             is what /ByteRange covers, and this token's imprint is a digest of \
                             something else",
                        PadesDeparture::ContentTimestampUnreadable =>
                            "that signature states §12.8.3.4.3 (c)'s content timestamp attribute \
                             and this program will not read what is in it, so ETSI EN 319 122-1 \
                             clause 5.2.8's imprint rule was not applied to it",
                        PadesDeparture::SignerAttributesIsUnsigned =>
                            "§12.8.3.4.3 (j) hands signer-attributes-v2 to ETSI EN 319 122-1 \
                             clause 5.2.6.1, which makes that attribute a signed one, and this \
                             signature states it among its unsigned attributes",
                        PadesDeparture::SignerAttributesNotOneValue =>
                            "§12.8.3.4.3 (j)'s ETSI EN 319 122-1 clause 5.2.6.1 gives \
                             signer-attributes-v2 exactly one AttributeValue, and this one holds \
                             a different number",
                        PadesDeparture::SignerAttributesEmpty =>
                            "§12.8.3.4.3 (j)'s ETSI EN 319 122-1 clause 5.2.6.1 forbids an empty \
                             signer-attributes-v2, and this one states no attribute at all",
                        PadesDeparture::CommitmentTypeAndReason =>
                            "§12.8.3.4.4 says that where a commitment-type-indication attribute \
                             is present a /Reason entry shall not be used, and this signature \
                             states both",
                        PadesDeparture::BothSigningCertificateAttributes =>
                            "ETSI EN 319 122-2 Table 1, which §12.8.3.4.4's two profiles are \
                             defined against, admits one of §12.8.3.4.3 (f)'s two attributes, and \
                             this signature states both",
                        PadesDeparture::PolicyStoreWithoutPolicyDigest =>
                            "ETSI EN 319 122-2 Table 1's requirement (c) admits a \
                             signature-policy-store only beside a signature-policy-identifier \
                             carrying the policy document's digest, and this signature has none",
                        PadesDeparture::SignaturePolicyImplied =>
                            "ETSI EN 319 122-1 clause 5.2.9.1 forbids the signaturePolicyImplied \
                             alternative, and this signature's signature-policy-identifier is it",
                        PadesDeparture::SignaturePolicyIdentifierIsUnsigned =>
                            "§12.8.3.4.4 requires a signature-policy-identifier as a signed \
                             attribute, and this signature states it among its unsigned ones",
                        PadesDeparture::SigningCertificateV2StatesSha1 =>
                            "ETSI EN 319 122-2 Table 1's requirement (a) reserves SHA-1 for the \
                             signing-certificate attribute, and this signature's \
                             signing-certificate-v2 states it",
                        PadesDeparture::AttributeStatedMoreThanOnce(_)
                        | PadesDeparture::ValueNotDerEncoded(_) => repeated.as_str(),
                    }
                ));
            }
            // §12.8.3.4.4's profiles, said whether or not anything departs from them: which of the
            // two a signature follows decides which of ETSI EN 319 122-2 Table 1's columns its
            // attributes were read against, and a reader of the departures above needs to know
            // which column that was.
            if let Some(profile) = signature.pades_profile(&cms) {
                notes.push(format!(
                    "that signature presents itself as §12.8.3.4.4's {}, because it states {} \
                     signature-policy-identifier among its signed attributes",
                    match profile {
                        PadesProfile::BasicElectronicSignature => "PAdES-E-BES profile",
                        PadesProfile::ExplicitPolicyElectronicSignature => "PAdES-E-EPES profile",
                        _ => "profile this build has no name for",
                    },
                    match profile {
                        PadesProfile::ExplicitPolicyElectronicSignature => "a",
                        _ => "no",
                    }
                ));
            }
            // §12.8.3.4.5 (a)'s first sentence, said whether or not the signature verifies and
            // whatever the `/SubFilter` is: RFC 5035 section 5.4.1 puts the same rule on any CMS
            // object carrying the attribute, and `Signature::authenticity` already refuses on a
            // mismatch. What this adds is the *positive* half, which the verdict above cannot
            // carry — that the signer signed a statement about which certificate it used and this
            // is that certificate — and the two refusals, which say the comparison was not made.
            for binding in pdf_signature::signature::signing_certificate_bindings(&cms) {
                let attribute = binding.version().attribute_name();
                notes.push(match binding {
                    SigningCertificateBinding::Matches { .. } => format!(
                        "that signature's {attribute} attribute names the certificate it was \
                         checked under (§12.8.3.4.5 (a)); who issued that certificate, and \
                         whether anyone should trust it, is the question this program does not \
                         answer"
                    ),
                    SigningCertificateBinding::Differs { .. } => format!(
                        "that signature's {attribute} attribute names a different certificate \
                         from the one it carries, which §12.8.3.4.5 (a) says makes it invalid"
                    ),
                    SigningCertificateBinding::Unreadable { ref error, .. } => format!(
                        "that signature states a {attribute} attribute this program could not \
                         read ({error}), so §12.8.3.4.5 (a)'s comparison was not made"
                    ),
                    SigningCertificateBinding::CertificateNotDer { .. } => format!(
                        "that signature states a {attribute} attribute and its certificate is not \
                         written in DER, so §12.8.3.4.5 (a)'s comparison was not made"
                    ),
                    SigningCertificateBinding::NoSignerCertificate { .. } => format!(
                        "that signature states a {attribute} attribute and carries no certificate \
                         answering to its signer, so §12.8.3.4.5 (a)'s comparison was not made"
                    ),
                });
            }
        }
    }
}

/// §12.8.6's permissions dictionary, said before a person starts typing rather than after.
fn permissions_stated(
    permissions: &pdf_signature::signature::Permissions,
    notes: &mut Vec<String>,
) {
    // §12.8.2.2.1's parenthesis is a `shall` addressed to a processor that modifies: "(These
    // changes to the document shall also be prevented if the signature dictionary is referred
    // from the DocMDP entry in the permissions dictionary.)" This program modifies since the
    // hundred-and-thirty-fifth session, so it obeys it — an `Edit` that a level of `/P` does not
    // permit is refused with `Event::Refused` and its reason — and says so here as well, because
    // a field that will not take a value is otherwise a person typing into a document that
    // ignores them, and this is said before they start rather than after.
    match permissions.doc_mdp {
        Some(pdf_signature::signature::Modification::None) => notes.push(
            "this document's author certified it as final (§12.8.2.2's /P 1), so no change to \
             it is permitted and none will be accepted"
                .to_owned(),
        ),
        Some(pdf_signature::signature::Modification::FormFilling) => notes.push(
            "this document's author permitted only form filling and signing (§12.8.2.2's /P 2)"
                .to_owned(),
        ),
        Some(pdf_signature::signature::Modification::FormFillingAndAnnotation) => notes.push(
            "this document's author permitted form filling, signing and annotation \
             (§12.8.2.2's /P 3)"
                .to_owned(),
        ),
        Some(pdf_signature::signature::Modification::Unknown(level)) => notes.push(format!(
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
    if let Some(rights) = permissions.usage_rights.as_ref() {
        let fills = rights.grants(pdf_signature::signature::Right::FillInForm);
        let saves = rights.grants(pdf_signature::signature::Right::FullSave);
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

/// §12.8.2.2.2's and §12.8.2.3's *second* step, said to the person reading the document.
///
/// # Why this is a report and not a verdict
///
/// Both clauses state two steps in the same shape, and the first of each is the byte range digest
/// that [`verdicts`] has recomputed since the three-hundred-and-seventy-seventh session. This is
/// the second — §12.8.2.2.2's
///
/// > Next, it shall verify that any modifications that have been made to the document are
/// > permitted by the transform parameters.
///
/// — and `pdf_signature::revision` has computed it since ADR 1043 with no program asking. What
/// that left is the shape ADR 1076 built the anchor supply against: a reading the tree performs
/// and nobody is told. So the sentences below say what the update *did* and what the transform's
/// own parameters say about it, and no sentence of them says a signature is valid — the word is
/// `pdf_signature::verdict::Valid`'s, it needs an anchor nobody supplies by default, and neither
/// `revision::Judgement` nor `revision::RightsJudgement` has a variant for it.
///
/// # What decides that a sentence is said at all
///
/// Trap 11's rule, and each condition is the clause's own. The comparison runs where §12.8.1's
/// signed range stops short of the file's end — which is where an incremental update was appended
/// after signing and there is a second state to compare — or where §12.8.6's permissions
/// dictionary names this signature in `/DocMDP` or `/UR3`, because a transform that states what
/// may change is owed an answer whether or not anything did. The rankings are two and they fire
/// separately: Table 257's levels are asked of the `/DocMDP` signature and Table 258's rights of
/// the `/UR3` one, which is where §12.8.6's table puts each.
fn modifications(
    signature: &pdf_signature::signature::Signature,
    document: &Document,
    permissions: &pdf_signature::signature::Permissions,
    notes: &mut Vec<String>,
) {
    use pdf_signature::revision::Comparison;

    let level = if permissions.doc_mdp_signature.as_ref() == Some(signature) {
        permissions.doc_mdp
    } else {
        None
    };
    let rights = if permissions.usage_rights_signature.as_ref() == Some(signature) {
        permissions.usage_rights.as_ref()
    } else {
        None
    };
    // §12.8.1 puts the end of a signed range at "the end of the \"%%EOF\" comment, possibly
    // followed by an optional EOL marker, terminating the incremental update that adds the digital
    // signature dictionary", so a signature covering the whole file has nothing appended after it
    // and no second state to be put beside the first.
    let covers_everything = signature.coverage(document.bytes().len() as u64)
        == pdf_signature::signature::Coverage::WholeFile;
    if covers_everything && level.is_none() && rights.is_none() {
        return;
    }
    // **Two clauses state the same second step and a sentence names the one that applies to
    // *this* signature**: §12.8.2.3 states it for a usage rights signature and §12.8.2.2.2 for
    // every other, and citing the wrong one would point a reader at a table saying nothing about
    // what they are holding.
    let clause = if rights.is_some() {
        "§12.8.2.3"
    } else {
        "§12.8.2.2.2"
    };
    let comparison = match Comparison::of(signature, document) {
        Ok(comparison) => comparison,
        Err(refusal) => {
            notes.push(format!(
                "what happened to this document after that signature was not compared with the \
                 revision it signed: {refusal}. So the second of the two steps {clause} states was \
                 not taken, and nothing below is a statement about it"
            ));
            return;
        }
    };
    let counted = comparison.kinds();
    let changed: u64 = counted
        .iter()
        .map(|(_, objects)| objects.count())
        .fold(0_u64, u64::saturating_add);
    if changed == 0 {
        notes.push(format!(
            "nothing in this document changed after that signature: the current file states the \
             same objects in the same places, under the same catalog, as the revision it signed \
             ({clause}'s comparison, object by object)"
        ));
    } else {
        let said: Vec<String> = counted
            .iter()
            .map(|(kind, objects)| format!("{} {}", objects.count(), what_changed(*kind)))
            .collect();
        notes.push(format!(
            "{changed} object(s) changed after that signature, in {} incremental update(s) \
             appended after the bytes it signed: {}",
            comparison.updates_after(),
            said.join("; ")
        ));
    }
    if let Some(level) = level {
        ranked_by_level(&comparison, level, notes);
    }
    if let Some(rights) = rights {
        ranked_by_rights(&comparison, rights, notes);
    }
}

/// Table 257's sentences: how the changes after a certification signature rank against the
/// `/DocMDP` level it states (§12.8.2.2).
fn ranked_by_level(
    comparison: &pdf_signature::revision::Comparison,
    level: pdf_signature::signature::Modification,
    notes: &mut Vec<String>,
) {
    use pdf_signature::revision::Judgement;

    let ranking = comparison.rank(level);
    notes.push(match ranking.judgement() {
        // Table 257 ranks changes and there are none, which is a different sentence from
        // "every change is permitted" and is said as one.
        Judgement::NoChangeToRank => format!(
            "that signature is this document's certification signature and it states \
             §12.8.2.2's {}, and there is no modification for Table 257 to rank",
            stated_level(level)
        ),
        Judgement::WithinWhatIsPermitted {
            objects,
            disregarded,
            ..
        } => format!(
            "every one of those changes is one §12.8.2.2's {} permits: {objects} object(s) \
             ranked against Table 257's own operations{}. That is a statement about objects \
             and not a verdict on the signature",
            stated_level(level),
            if disregarded == 0 {
                String::new()
            } else {
                format!(
                    ", and {disregarded} more in update(s) Table 257 does not count as \
                     changes to the document at all"
                )
            }
        ),
        Judgement::NotPermitted { objects, .. } => format!(
            "{objects} of those changes are ones §12.8.2.2's {} does not permit, and Table \
             257 says other changes shall invalidate the signature — object(s) {}",
            stated_level(level),
            numbers(ranking.not_permitted.named())
        ),
        Judgement::NotClassified { objects, .. } => format!(
            "Table 257's {} forbids none of those changes and this program will not say they \
             are permitted either: {objects} of them are none of the operations the table \
             names, so they are refused rather than ranked — object(s) {}",
            stated_level(level),
            numbers(ranking.unrankable.named())
        ),
    });
}

/// Table 258's sentences: how the changes after a usage rights signature rank against the rights
/// its `/UR3` grants (§12.8.2.3), and which granted rights the ranking could not have been about.
fn ranked_by_rights(
    comparison: &pdf_signature::revision::Comparison,
    rights: &pdf_signature::signature::UsageRights,
    notes: &mut Vec<String>,
) {
    use pdf_signature::revision::RightsJudgement;

    let ranking = comparison.against_usage_rights(rights);
    notes.push(match ranking.judgement() {
        RightsJudgement::NoChangeToRank => {
            "that signature is this document's usage rights signature and there is no \
             modification for Table 258's rights to rank (§12.8.2.3)"
                .to_owned()
        }
        RightsJudgement::WithinTheRightsGranted { objects } => format!(
            "every one of those changes is an operation this document's /UR3 grants: \
             {objects} object(s) ranked against Table 258's own rights (§12.8.2.3){}. That is \
             a statement about objects and not a verdict on the signature",
            match ranking.not_a_modification.count() {
                0 => String::new(),
                counted => format!(
                    ", and {counted} more that modify nothing in the document — an object \
                     restated as it stood, or an update's own cross-reference stream"
                ),
            }
        ),
        RightsJudgement::OutsideTheRightsGranted { objects } => format!(
            "{objects} of those changes are modifications Table 258's rights do not permit, \
             which §12.8.2.3 says is what invalidates a usage rights signature — object(s) {}",
            numbers(ranking.not_granted.named())
        ),
        RightsJudgement::NotClassified { objects } => format!(
            "this document's /UR3 permits none of those changes any less, and this program \
             will not say it permits them either: {objects} of them are none of the rights \
             Table 258 names, so they are refused rather than ranked — object(s) {}",
            numbers(ranking.unrecognised.named())
        ),
    });
    // **The rights this file grants that the ranking above could not have been about.** Trap
    // 5's rule at the one place it bites here: a reader told a change is inside a `/UR3`'s
    // rights would otherwise have no way to know which of the rights that `/UR3` names were
    // never a candidate. The condition is what the *file* states, so a document granting only
    // rights this comparison recognises says nothing.
    let unreadable: Vec<String> = pdf_signature::revision::RIGHTS_NOT_RECOGNISED
        .iter()
        .filter(|(array, name)| granted_name(rights, array, name))
        .map(|(array, name)| format!("/{array} {name}"))
        .collect();
    if !unreadable.is_empty() {
        notes.push(format!(
            "that /UR3 also grants {}, and this program cannot recognise {} in a changed \
             object — Table 258 names {} rights and a comparison of two revisions can see \
             eight of them, so the ranking above is silent about the rest rather than \
             having ranked them",
            unreadable.join(", "),
            if unreadable.len() == 1 {
                "that right"
            } else {
                "those rights"
            },
            23
        ));
    }
}

/// Table 258's array entry, asked of what the file actually granted.
fn granted_name(rights: &pdf_signature::signature::UsageRights, array: &str, name: &str) -> bool {
    let list = match array {
        "Document" => &rights.document,
        "Annots" => &rights.annots,
        "Form" => &rights.form,
        "Signature" => &rights.signature,
        _ => &rights.embedded_files,
    };
    list.iter().any(|entry| entry == name)
}

/// §12.8.2.2's `/P`, named as the table numbers it.
fn stated_level(level: pdf_signature::signature::Modification) -> String {
    use pdf_signature::signature::Modification;
    match level {
        Modification::None => "/P 1".to_owned(),
        Modification::FormFilling => "/P 2".to_owned(),
        Modification::FormFillingAndAnnotation => "/P 3".to_owned(),
        Modification::Unknown(level) => format!("/P {level}, which Table 257 does not define,"),
    }
}

/// What one changed object was taken to be, in words rather than as an enumeration.
///
/// The vocabulary is Table 257's, because that is the vocabulary the operations are stated in;
/// `pdf_signature::revision::Kind` carries the clause each of them rests on.
fn what_changed(kind: pdf_signature::revision::Kind) -> &'static str {
    use pdf_signature::revision::Kind;
    match kind {
        Kind::RestatedUnchanged => "written again saying exactly what they said",
        Kind::CrossReferenceStream => "an update's own cross-reference stream (§7.5.8)",
        Kind::FieldFilledIn => "a form field filled in",
        Kind::Signing => "a signature applied",
        Kind::TemplateInstantiated => "a page instantiated from a template (§12.7.7)",
        Kind::FieldAppearance => "the appearance stream of a field filled in",
        Kind::Annotation => "an annotation created, deleted or modified",
        Kind::AnnotationAppearance => "an annotation's appearance stream",
        Kind::ValidationMaterial => {
            "validation material or a document timestamp (§12.8.4, §12.8.5)"
        }
        Kind::CatalogReplaced => "the document's catalog, replaced by another",
        Kind::Unplaceable => "an object neither state's cross-reference table places",
        Kind::Unclassified => "a change this program will not put a name to",
    }
}

/// Object numbers for a person, bounded the way the set that holds them is.
fn numbers(named: &[u32]) -> String {
    named
        .iter()
        .map(u32::to_string)
        .collect::<Vec<String>>()
        .join(", ")
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
    signature: &pdf_signature::signature::Signature,
    document: &Document,
    asked: &Asked<'_>,
    notes: &mut Vec<String>,
) {
    use pdf_signature::signature::{Authenticity, Integrity, Signed};
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
        // The reader's own sentence about *why* the range could not be read, where it has one:
        // `RangeNotReadable` is the refusal by name and this is the name (ADR 0812).
        if integrity == Integrity::RangeNotReadable
            && let Some(failure) = document.bytes().read_failure()
        {
            notes.push(format!("the file on disk refused the read: {failure}"));
        }
    }
    if let Some(said) = verifies(&authenticity, integrity) {
        notes.push(said);
    }
    // **§12.8.1's third question, asked only where a host answered whom to believe.**
    // `Verdict::of` is the one function in this tree that can produce the word *valid*, and an
    // empty anchor set reaches `Reservation::NoAnchorSupplied` rather than any other answer — so
    // with no host input this adds nothing at all, which is what it added before anchors existed.
    if asked.anchors.is_empty() {
        return;
    }
    let trust = signature.trust(asked.anchors, asked.material, asked.at);
    let verdict = pdf_signature::verdict::Verdict::of(
        &integrity,
        &authenticity,
        &trust,
        asked.acceptance,
        asked.timestamps,
        asked.at,
    );
    // Which instant the path was asked about, where it is not simply now: §12.8.4.2's own reason
    // for a document security store is that a signature outlives its certificate, and a reader
    // told a path validated is owed *as of when*.
    let asked_at = asked.from_a_token.map_or_else(String::new, |source| {
        format!(
            ", as of {} seconds after the epoch, which {source}",
            asked.at.unix_seconds()
        )
    });
    notes.push(match &verdict {
        pdf_signature::verdict::Verdict::Valid(valid) => format!(
            "that signature is valid: the document has not changed since it was signed, the \
             signature verifies under the signer's key, and a certification path {} certificate(s) \
             long reaches an authority you supplied (RFC 5280 section 6.1){asked_at}. This is the \
             one sentence this program says the word in, and it is your anchors that make it \
             sayable",
            valid.path_length()
        ),
        pdf_signature::verdict::Verdict::Reserved(reservation) => format!(
            "that signature is not called valid here{asked_at}, and this is the first thing that \
             stopped it: {reservation}"
        ),
    });
}

/// §12.8.1's first question in words: did the bytes under this signature move?
fn changed(integrity: pdf_signature::signature::Integrity) -> String {
    use pdf_signature::signature::Integrity;
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
        Integrity::RangeNotReadable => {
            "that signature's /ByteRange names bytes the file on disk would not give — it shrank \
             under this reader or a read of it failed — so whether the document changed was not \
             checked rather than reported wrongly"
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

/// One sentence for every family's "the arithmetic never ran", whatever the family.
///
/// Four variants say this and each carries its own error type — the budgets and encodings are
/// per-module and their *names* are the product (ADR 0331) — but what a reader needs is the same
/// sentence with that name in it.
fn not_checked(error: &dyn std::fmt::Display) -> String {
    format!("and that signature was not checked against the signer's key: {error}")
}

/// §12.8.1's second question in words, worded against the first one's answer.
///
/// `None` where the sentence would repeat what [`changed`] has already said in the same words —
/// no signature value, no bytes to hash, nothing readable — because a program that says one fact
/// twice teaches a reader to skim.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per `Authenticity` variant, and the variants are a closed vocabulary \
of why a signature is not verified; a reader needs them in one place to see that none of \
them says valid"
)]
fn verifies(
    authenticity: &pdf_signature::signature::Authenticity,
    integrity: pdf_signature::signature::Integrity,
) -> Option<String> {
    use pdf_signature::signature::{Authenticity, Integrity, Signed};
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
             program does not verify: it verifies Table 260's three families — RSA under both of \
             RFC 8017's paddings, DSA and ECDSA — and the Ed25519 half of the EdDSA row ISO/TS \
             32002 adds to that table"
        ),
        Authenticity::CurveNotVerifiable { curve } => format!(
            "and the signer's certificate holds an elliptic-curve key on curve {curve}, which \
             this program does not compute on: of the six curves ISO/TS 32002 Table 3 names it \
             computes on P-256, P-384 and P-521"
        ),
        Authenticity::AlgorithmNotVerifiable { algorithm } => format!(
            "and that signature states signature algorithm {algorithm}, which this program does \
             not verify: it verifies RSASSA-PKCS1-v1_5, RSASSA-PSS, DSA, ECDSA and Ed25519"
        ),
        Authenticity::PssParametersNotVerifiable { statement } => format!(
            "and that signature states id-RSASSA-PSS with parameters this program cannot verify \
             under — {statement} — so it was not checked against the signer's key"
        ),
        Authenticity::KeyDoesNotMatchAlgorithm { algorithm, key } => format!(
            "and that signature states signature algorithm {algorithm} while the signer's \
             certificate holds a key of algorithm {key} — two statements by the same producer \
             that contradict each other, so there is nothing to check the signature against"
        ),
        Authenticity::SigningCertificateMismatch { version, digest } => format!(
            "and its {} attribute states a {digest:?} hash that is not the hash of the certificate \
             it carries — §12.8.3.4.5 (a) says that makes the signature invalid, and the check is \
             made before any arithmetic because the certificate a signature is judged against is \
             exactly what that attribute pins down",
            version.attribute_name(),
        ),
        Authenticity::SigningCertificateUnverifiable { version, statement } => format!(
            "and its {} attribute could not be acted on — {statement} — so §12.8.3.4.5 (a)'s \
             comparison was not made and the signature was not checked against any key; a pass \
             here would be this program reporting a check it did not make",
            version.attribute_name(),
        ),
        Authenticity::Refused(error) => not_checked(error),
        Authenticity::RefusedDsa(error) => not_checked(error),
        Authenticity::RefusedEcdsa(error) => not_checked(error),
        Authenticity::RefusedEdDsa(error) => not_checked(error),
        Authenticity::SignedAttributesNotDer { rule } => format!(
            "and its signed attributes are not DER encoded — {rule} — which RFC 5652 section 5.3 \
             requires of them even where the rest of a CMS object is BER, so the bytes the signer \
             digested are not the bytes this file holds and the signature was not checked against \
             any key"
        ),
        Authenticity::UnknownDigest { algorithm } => format!(
            "and it names digest algorithm {algorithm}, which this program does not compute, so \
             it was not checked against the signer's key either"
        ),
        Authenticity::NoSignatureValue
        | Authenticity::RangeNotInThisFile
        | Authenticity::RangeNotReadable
        | Authenticity::Unreadable(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::about;

    use pdf_syntax::Document;

    /// The sentence for a scan the process could not hold the file for names the length, says
    /// what was read instead, and claims nothing about what the misplaced object held.
    ///
    /// Wording only, and honestly so: provoking `pdf_syntax::Document::scan_refused` needs a
    /// file the process cannot hold whole, which a unit test cannot make. The wiring — the
    /// reader's refusal reaching a host's report, once, on the page that first needed the scan
    /// — was exercised by hand on a 4.6 GB file through the confined viewer, whose worker runs
    /// under a 4 GiB ceiling; ADR 0812 records the run.
    #[test]
    fn a_scan_the_process_could_not_hold_the_file_for_is_said_with_its_length() {
        let said = super::scan_refused(pdf_syntax::NoRoom {
            length: 6_001_925_614,
        });
        assert!(
            said.contains("6001925614 bytes, which this process cannot hold"),
            "{said}"
        );
        assert!(
            said.contains("read as far as its table was right"),
            "{said}"
        );
        assert!(said.contains("§C.4"), "{said}");
    }

    /// Builds a document from object bodies numbered from 1, as `pdf_signature::signature`'s tests do.
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

    /// A one-revision signed document whose `/ByteRange` names its own bytes, ending at `%%EOF`.
    ///
    /// The same construction `pdf_signature::revision`'s own fixtures use and for the same reason:
    /// §12.8.2.2.2's comparison opens the *prefix* the range names, so a range that does not stop
    /// at a revision boundary is refused and the report under test never runs. Object 1 is the
    /// catalog, 2 the page tree, 3 a page, 4 the signature and 5 its field.
    fn signed_document(catalog: &str, signature: &str) -> Vec<u8> {
        use std::fmt::Write as _;
        let objects = [
            catalog,
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            signature,
            "<< /FT /Sig /T (Signature1) /V 4 0 R /Subtype /Widget >>",
        ];
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let mut bytes = out.into_bytes();
        let text = String::from_utf8(bytes.clone()).expect("the fixture is ASCII");
        let value = text.find("/Contents <").expect("a signature value");
        let start = value.saturating_add("/Contents ".len());
        let end = text[start..]
            .find('>')
            .map(|at| start.saturating_add(at).saturating_add(1))
            .expect("a closing delimiter");
        let range = text.find("/ByteRange [").expect("a placeholder range");
        let at = range.saturating_add("/ByteRange [".len());
        let filled = format!(
            "{:010} {:010} {:010} {:010}",
            0,
            start,
            end,
            text.len().saturating_sub(end)
        );
        bytes.splice(at..at.saturating_add(filled.len()), filled.into_bytes());
        bytes
    }

    /// §7.5.6's incremental update appended to one of those: the objects, a section, a trailer.
    fn append(bytes: &mut Vec<u8>, objects: &[(u32, &str)]) {
        use std::fmt::Write as _;
        let text = String::from_utf8(bytes.clone()).expect("ascii");
        let at = text.rfind("startxref\n").expect("a previous section");
        let previous: usize = text[at..]
            .lines()
            .nth(1)
            .and_then(|line| line.trim().parse().ok())
            .expect("the previous section's offset");
        let mut out = String::new();
        let mut placed = Vec::new();
        let mut highest = 0;
        for (number, body) in objects {
            placed.push((*number, bytes.len().saturating_add(out.len())));
            highest = highest.max(*number);
            let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
        }
        let xref_at = bytes.len().saturating_add(out.len());
        out.push_str("xref\n");
        for (number, offset) in &placed {
            let _ = write!(out, "{number} 1\n{offset:010} 00000 n \n");
        }
        let size = usize::try_from(highest).unwrap_or(0).saturating_add(1);
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R /Prev {previous} >>\n\
             startxref\n{xref_at}\n%%EOF\n"
        );
        bytes.extend_from_slice(out.as_bytes());
    }

    /// §12.8.2.2.2's second step reaches a person, and Table 257's level is what decides it.
    ///
    /// **The calibration is the same file twice** (trap 13): one update, one annotation created,
    /// and the only difference between the two runs is the `/P` the certification states. A report
    /// that said the same thing at both levels would be reporting that a comparison happened
    /// rather than what it found — and until this round nothing in the program said either, which
    /// is the debt ADR 1043 named and ADR 1104 closes.
    #[test]
    fn what_an_update_did_after_a_certification_is_ranked_against_the_level_it_states() {
        let certification = |level: u8| {
            format!(
                "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
                 /Reference [ << /Type /SigRef /TransformMethod /DocMDP /TransformParams \
                 << /Type /TransformParams /P {level} >> >> ] \
                 /ByteRange [0000000000 0000000000 0000000000 0000000000] \
                 /Contents <00112233445566778899aabbccddeeff> >>"
            )
        };
        let catalog = "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> \
                       /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>";

        let mut bytes = signed_document(catalog, &certification(2));
        append(
            &mut bytes,
            &[(6, "<< /Type /Annot /Subtype /Text /Rect [1 1 6 6] >>")],
        );
        let document = Document::open(bytes).expect("a valid file");
        let said = about(&document, &crate::TrustPolicy::default()).join("\n");
        assert!(
            said.contains("1 object(s) changed after that signature, in 1 incremental update(s)"),
            "{said}"
        );
        assert!(
            said.contains("1 an annotation created, deleted or modified"),
            "{said}"
        );
        // Table 257's level 2 permits "filling in forms, instantiating page templates, and
        // signing" and this is none of the three, so the sentence has to be the table's own
        // consequence and has to name the object.
        assert!(
            said.contains("1 of those changes are ones §12.8.2.2's /P 2 does not permit"),
            "{said}"
        );
        assert!(said.contains("object(s) 6"), "{said}");

        // The same file at level 3, which adds "annotation creation, deletion, and modification".
        let mut bytes = signed_document(catalog, &certification(3));
        append(
            &mut bytes,
            &[(6, "<< /Type /Annot /Subtype /Text /Rect [1 1 6 6] >>")],
        );
        let document = Document::open(bytes).expect("a valid file");
        let said = about(&document, &crate::TrustPolicy::default()).join("\n");
        assert!(
            said.contains("every one of those changes is one §12.8.2.2's /P 3 permits"),
            "{said}"
        );
        assert!(!said.contains("does not permit"), "{said}");
        // **The word no sentence of this report may reach**, at either level: the ranking is a
        // statement about objects and §12.8.1's third question has no trust store behind it.
        assert!(
            !said.contains("the signature is valid") && !said.contains("signature is valid"),
            "{said}"
        );
    }

    /// §12.8.2.3's rights reach a person too, and the rights the ranking could not be about.
    ///
    /// A `/UR3` granting `/Annots [/Create]` and `/EF [/Delete]`: the annotation created is inside
    /// what was granted, and the `/EF` right is one `revision::RIGHTS_NOT_RECOGNISED` refuses by
    /// name — so a reader is told both, which is trap 5's rule at the place a partial answer would
    /// otherwise read as a whole one.
    #[test]
    fn what_an_update_did_after_a_usage_rights_signature_is_ranked_against_table_258() {
        let signature = "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
                         /Reference [ << /Type /SigRef /TransformMethod /UR3 /TransformParams \
                         << /Type /TransformParams /V /2.2 /P true /Annots [/Create] \
                         /EF [/Delete] >> >> ] \
                         /ByteRange [0000000000 0000000000 0000000000 0000000000] \
                         /Contents <00112233445566778899aabbccddeeff> >>";
        let catalog = "<< /Type /Catalog /Pages 2 0 R /Perms << /UR3 4 0 R >> \
                       /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>";
        let mut bytes = signed_document(catalog, signature);
        append(
            &mut bytes,
            &[(6, "<< /Type /Annot /Subtype /Text /Rect [1 1 6 6] >>")],
        );
        let document = Document::open(bytes).expect("a valid file");
        let said = about(&document, &crate::TrustPolicy::default()).join("\n");

        assert!(
            said.contains("every one of those changes is an operation this document's /UR3 grants"),
            "{said}"
        );
        assert!(
            said.contains("that /UR3 also grants /EF Delete, and this program cannot recognise"),
            "{said}"
        );
        assert!(
            !said.contains("/Annots Create, and this program cannot"),
            "{said}"
        );

        // The control, and it moves exactly one answer (trap 13): the same update under a `/UR3`
        // that names `Delete` alone is outside the rights rather than inside them.
        let refusing = signature.replace("/Annots [/Create]", "/Annots [/Delete]");
        let mut bytes = signed_document(catalog, &refusing);
        append(
            &mut bytes,
            &[(6, "<< /Type /Annot /Subtype /Text /Rect [1 1 6 6] >>")],
        );
        let document = Document::open(bytes).expect("a valid file");
        let said = about(&document, &crate::TrustPolicy::default()).join("\n");
        assert!(
            said.contains("1 of those changes are modifications Table 258's rights do not permit"),
            "{said}"
        );
    }

    /// §12.8.5's chain, named to a person, and the two things the sentences may not say.
    ///
    /// **A hand-built document, and by the same argument as the store's below**: no document in
    /// `doc/pdf.js` carries a `/DocTimeStamp`, `pdf-signature`'s census is the command that
    /// establishes it, and the crawl's real ones are where `pdf_signature::timestamp`'s own tests
    /// take their witnesses (trap 8).
    ///
    /// The two sentences that may not appear are the point of the test: the report must never
    /// state the `genTime` as a *time*, and must never fire for a document with no timestamp.
    #[test]
    fn a_document_carrying_a_timestamp_says_what_it_covers_and_calls_it_no_time() {
        let stamped = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /SigFlags 3 /Fields [3 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /FT /Sig /T (a timestamp) /V 4 0 R >>",
            "<< /Type /DocTimeStamp /Filter /Adobe.PPKLite /SubFilter /ETSI.RFC3161 \
             /ByteRange [0 0 0 0] /Contents <> >>",
        ]);
        let said = about(&stamped, &crate::TrustPolicy::default()).join("\n");
        assert!(
            said.contains("carries 1 §12.8.5 document timestamp(s)"),
            "{said}"
        );
        assert!(
            said.contains(
                "§12.8.5.3 stacks so that each protects the structure the one before it \
                           left"
            ),
            "{said}"
        );
        // An empty `/Contents` is not a token, and the sentence says which of the two it could
        // not do rather than staying silent about both (`CLAUDE.md` principle 1).
        assert!(
            said.contains("carries a token this reader will not read"),
            "{said}"
        );
        assert!(
            said.contains("none of those timestamps tells this program *when*"),
            "{said}"
        );
        // **The word that may not be there.** §12.8.5.2's authority is a *trusted* one and nothing
        // here establishes trust, so no sentence about a timestamp may call it valid or verified.
        assert!(
            !said.contains("the timestamp is valid") && !said.contains("timestamp verifies"),
            "{said}"
        );

        // The same document with the `/Type` taken off its signature dictionary: Table 255 makes
        // the default `Sig`, so this is no longer a timestamp and none of the sentences above may
        // appear. A report that fired anyway would be describing a chain that is not there.
        let plain = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /SigFlags 3 /Fields [3 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /FT /Sig /T (a timestamp) /V 4 0 R >>",
            "<< /Filter /Adobe.PPKLite /SubFilter /ETSI.RFC3161 /ByteRange [0 0 0 0] \
             /Contents <> >>",
        ]);
        let quiet = about(&plain, &crate::TrustPolicy::default()).join("\n");
        assert!(!quiet.contains("§12.8.5 document timestamp(s)"), "{quiet}");
        assert!(
            !quiet.contains("none of those timestamps tells this program"),
            "{quiet}"
        );
    }

    /// §12.8.4's store, named to a person, and the one thing the sentence may not say.
    ///
    /// **A hand-built document because no corpus document carries one.** `pdf-signature`'s census
    /// is the command that establishes that, and it is why this witness is a fragment: a store
    /// exists in the world — the wider crawl finds them — and in `doc/pdf.js` there is not one, so
    /// nothing here could exercise the sentence at all otherwise (trap 8).
    ///
    /// The mutation is the same document with its `/DSS` removed, which must say nothing: a report
    /// that fired on every signed document would be telling a person about a store that is not
    /// there.
    #[test]
    fn a_document_carrying_a_security_store_says_what_is_in_it_and_claims_nothing_from_it() {
        let with_store = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /SigFlags 1 /Fields [3 0 R] >> /DSS 5 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /FT /Sig /T (a signature) /V 4 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 0 0 0] /Contents <> >>",
            "<< /Type /DSS /Certs [6 0 R] /CRLs [6 0 R] /OCSPs [6 0 R 7 0 R] \
             /VRI << /ABCDEF << /Cert [6 0 R] >> >> >>",
            "<< /Length 0 >>\nstream\n\nendstream",
            "<< /Length 0 >>\nstream\n\nendstream",
        ]);
        let said = about(&with_store, &crate::TrustPolicy::default()).join("\n");
        assert!(
            said.contains(
                "carries a §12.8.4 document security store — the material a validator needs after \
                 the signer's certificate has expired: 1 certificate(s), 1 certificate revocation \
                 list(s) and 2 OCSP response(s)"
            ),
            "{said}"
        );
        assert!(
            said.contains("§12.8.4.4 validation information recorded for 1 signature(s)"),
            "{said}"
        );
        // Three empty streams are not a CRL and not an OCSP response, and saying so is the whole
        // difference between an answer about a certificate and no answer at all.
        assert!(
            said.contains("0 of those lists and responses read"),
            "{said}"
        );
        assert!(
            said.contains("is not usable"),
            "each piece this reader would not take is named: {said}"
        );
        // Nothing about a certificate is claimed from material no path was validated with, and
        // the closing paragraph is where that is said in words rather than by omission.
        assert!(
            said.contains(
                "only once somebody names a certification authority to end the chain at, which \
                 nothing here does"
            ),
            "{said}"
        );
        assert!(!said.contains("is revoked"), "{said}");

        let without = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /SigFlags 1 /Fields [3 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /FT /Sig /T (a signature) /V 4 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 0 0 0] /Contents <> >>",
        ]);
        assert!(
            !about(&without, &crate::TrustPolicy::default())
                .join("\n")
                .contains("document security store"),
            "a signed document with no store is told nothing about one"
        );
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
        let said = about(&document, &crate::TrustPolicy::default()).join("\n");

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

    /// §12.8.3.3.2's material is named where a file supplies it, and never anywhere else.
    ///
    /// The clause's whole content for a program with no network is one fact — that the signer put
    /// CRLs and OCSP responses in the signature — and stating it is what §12.8.3.3.2's `reported`
    /// row rests on. **That row cited three tests and not one of them reached this sentence**:
    /// they are `pdf-model`'s, and the naming happens two crates up, here. `doc/todo/01`'s "the
    /// row is right and its evidence is not" for the sixth round running.
    ///
    /// `issue6127.pdf` is the witness rather than the `issue17069.pdf` the row named, because it
    /// is the plainer of the three — unencrypted, one signature — and the census that found all
    /// three is `pdf-model`'s `examples/signature_algorithm_census`. The negative half is the
    /// mutation: `bug854315.pdf` is signed, its `SignerInfo` states no signed attributes at all,
    /// and it must not be told it carries revocation material.
    #[test]
    fn a_signature_carrying_revocation_information_says_so_and_claims_no_more() {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
        let Ok(bytes) = std::fs::read(directory.join("issue6127.pdf")) else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        let document = Document::open(bytes).expect("a valid file");
        let said = about(&document, &crate::TrustPolicy::default()).join("\n");
        assert!(
            said.contains(
                "carries revocation information with it (§12.8.3.3.2's \
                 adbe-revocationInfoArchival attribute)"
            ),
            "{said}"
        );
        assert!(
            said.contains("It makes no trust decision about any certificate"),
            "the presence is stated and no verdict is drawn from it: {said}"
        );
        // **And the material inside it is counted rather than only named.** The condition this
        // fires on is the attribute; what it says is how much of §12.8.3.3.2's
        // `RevocationInfoArchival` this reader took, which is the difference between a sentence
        // about a document and a sentence about this program (trap 11).
        assert!(
            said.contains("OCSP response(s) this program reads"),
            "{said}"
        );

        let plain = Document::open(
            std::fs::read(directory.join("bug854315.pdf")).expect("a signed corpus document"),
        )
        .expect("a valid file");
        assert!(
            !about(&plain, &crate::TrustPolicy::default())
                .join("\n")
                .contains("carries revocation information"),
            "a signature with no signed attributes carries none"
        );
    }

    /// Table 255's `/V 1` is said, and a signature that omits the entry is told nothing.
    ///
    /// **No corpus document states the entry**, so the witness is hand-built (trap 8) and the
    /// population is measured rather than guessed: `pdf-model`'s
    /// `examples/signature_algorithm_census` counts Table 255's `/V` over every document this
    /// tree can reach, and the `CC-MAIN-2021-31` crawl is where the two files that write `/V 1`
    /// are — both of them certification signatures carrying a `DocMDP` and a `FieldMDP`
    /// reference dictionary, which is exactly the material this program does not evaluate.
    ///
    /// The other two signatures are the calibration the sweep rule asks for: the same reference
    /// dictionary under an explicit `/V 0` — which six crawled files write — and under no `/V` at
    /// all, and the sentence must appear for neither. A guard widened to "the entry is present"
    /// fails on the first of those, and one removed altogether fails on the third.
    #[test]
    fn a_signature_calling_its_reference_dictionary_critical_says_so() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R \
             /AcroForm << /Fields [4 0 R 6 0 R 8 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Critical) /V 5 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /V 1 \
             /Name (A. Author) /ByteRange [0 10 20 10] /Contents <00> /Reference [10 0 R] >>",
            "<< /FT /Sig /T (Stated) /V 7 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /V 0 \
             /Name (B. Author) /ByteRange [0 10 20 10] /Contents <00> /Reference [10 0 R] >>",
            "<< /FT /Sig /T (Silent) /V 9 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /Name (C. Author) /ByteRange [0 10 20 10] /Contents <00> /Reference [10 0 R] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP \
             /TransformParams << /Type /TransformParams /P 2 /V /1.2 >> >>",
        ]);
        let said = about(&document, &crate::TrustPolicy::default());
        let critical: Vec<_> = said
            .iter()
            .filter(|note| note.contains("considered critical to validating it (Table 255)"))
            .collect();
        assert_eq!(
            critical.len(),
            1,
            "one of the three signatures states /V 1: {said:?}"
        );
        assert!(
            critical[0].contains("this program evaluates no transform method"),
            "the sentence names what was not done rather than only what the file asked for: \
             {critical:?}"
        );
    }

    /// Table 256's `/DigestMethod` is said, and the calibration is a signature that states none.
    ///
    /// Two sentences on two conditions the entry itself states - a name among the six the table
    /// admits, and a name that is not - and three signatures, the third of which states no
    /// `/DigestMethod` at all and must produce neither. A guard widened to "the signature has a
    /// reference dictionary" fails on that third one, which is what makes it a control rather
    /// than a third assertion (trap 11, trap 13).
    #[test]
    fn a_reference_dictionary_naming_a_digest_for_an_analysis_nobody_makes_says_so() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R \
             /AcroForm << /Fields [4 0 R 6 0 R 8 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Named) /V 5 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /Name (A. Author) /ByteRange [0 10 20 10] /Contents <00> /Reference [10 0 R] >>",
            "<< /FT /Sig /T (Outside) /V 7 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /Name (B. Author) /ByteRange [0 10 20 10] /Contents <00> /Reference [11 0 R] >>",
            "<< /FT /Sig /T (Silent) /V 9 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /Name (C. Author) /ByteRange [0 10 20 10] /Contents <00> /Reference [12 0 R] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP /DigestMethod /SHA512 >>",
            "<< /Type /SigRef /TransformMethod /DocMDP /DigestMethod /SHA2 >>",
            "<< /Type /SigRef /TransformMethod /DocMDP >>",
        ]);
        let said = about(&document, &crate::TrustPolicy::default());
        let stated: Vec<_> = said
            .iter()
            .filter(|note| note.contains("states /DigestMethod"))
            .collect();
        assert_eq!(
            stated.len(),
            1,
            "one of the three names a digest the table admits: {said:?}"
        );
        assert!(
            stated[0].contains("Sha512") && stated[0].contains("makes no such comparison"),
            "the sentence names the function and what was not done with it: {stated:?}"
        );
        let outside: Vec<_> = said
            .iter()
            .filter(|note| note.contains("not among the six Table 256 admits"))
            .collect();
        assert_eq!(
            outside.len(),
            1,
            "one of the three names something outside the value list: {said:?}"
        );
        assert!(
            outside[0].contains("SHA2"),
            "the sentence names what the file wrote: {outside:?}"
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
        let said = about(&document, &crate::TrustPolicy::default()).join("\n");
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
            about(&document(&borrowed), &crate::TrustPolicy::default()).join("\n")
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

    /// §14.8.6.2's file-addressed `shall`, calibrated against the defect and against each of the
    /// clause's own three ways out.
    ///
    /// `doc/traps/instruments-and-reports.md` trap 13: a report is a measurement, and one that
    /// has not been run against the thing it looks for is a sentence about a condition rather
    /// than about a document. So the first fixture *is* the violation — a tagged document whose
    /// element names a namespace that is neither of §14.8.6.1's two nor §14.8.6.3's one — and the
    /// four beside it are the conditions the clause states as satisfying it, each of which must
    /// leave the note unsaid.
    #[test]
    fn a_tagged_document_whose_elements_leave_the_standard_namespaces_is_said_out_loud() {
        let tagged = "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
                      /StructTreeRoot 4 0 R >>";
        let pages = "<< /Type /Pages /Kids [] /Count 0 >>";
        let foreign = "<< /Type /Namespace /NS (http://example.invalid/tagset) >>";
        let root = "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [3 0 R] >>";
        let element = "<< /Type /StructElem /S /Widget /NS 3 0 R >>";

        let said = about(
            &document(&[tagged, pages, foreign, root, element]),
            &crate::TrustPolicy::default(),
        )
        .join("\n");
        assert!(
            said.contains(
                "1 of its structure elements end in the namespace \
                           http://example.invalid/tagset"
            ),
            "the planted violation is named, with the namespace and the count: {said}"
        );
        assert!(said.contains("§14.8.6.2"), "{said}");

        // Way out 1, and it is §14.8.1's rather than §14.8.6's: the sentence says *in a tagged
        // PDF*, and a document with a structure tree that does not claim to be tagged has not
        // taken on the requirement at all.
        let untagged = "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>";
        assert!(
            !about(
                &document(&[untagged, pages, foreign, root, element]),
                &crate::TrustPolicy::default()
            )
            .join("\n")
            .contains("§14.8.6.2"),
            "a document that does not say it is tagged is outside the clause's sentence"
        );

        // Way out 2: "they are role mapped into the namespace, either directly or transitively",
        // through the namespace's own `/RoleMapNS` — §14.8.6.2's rule about which map applies. A
        // bare name leaves whatever namespace it was in, so `Div` lands in the default standard
        // one and the element is conforming.
        let mapped = "<< /Type /Namespace /NS (http://example.invalid/tagset) \
                      /RoleMapNS << /Widget /Div >> >>";
        assert!(
            !about(
                &document(&[tagged, pages, mapped, root, element]),
                &crate::TrustPolicy::default()
            )
            .join("\n")
            .contains("§14.8.6.2"),
            "a role map into the default standard namespace satisfies the third bullet"
        );

        // Way out 3: §14.8.6.3's one domain-specific namespace, which the clause names as an
        // alternative to the standard ones and exempts from role mapping outright.
        let mathml = "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>";
        let math = "<< /Type /StructElem /S /math /NS 3 0 R >>";
        assert!(
            !about(
                &document(&[tagged, pages, mathml, root, math]),
                &crate::TrustPolicy::default()
            )
            .join("\n")
            .contains("§14.8.6.2"),
            "MathML is a namespace §14.8.6.3 identifies"
        );

        // Way out 4: "they are in the default standard structure namespace", which is every
        // element that states no `/NS` at all — and the case the cheap gate is for, since such a
        // document's root declares no namespace and its elements are never read.
        let plain_root = "<< /Type /StructTreeRoot /K [5 0 R] >>";
        let plain = "<< /Type /StructElem /S /P >>";
        assert!(
            !about(
                &document(&[tagged, pages, foreign, plain_root, plain]),
                &crate::TrustPolicy::default()
            )
            .join("\n")
            .contains("§14.8.6.2"),
            "an element with no /NS is in the default standard structure namespace"
        );
    }

    /// §14.8.6.3's file-addressed `shall`, calibrated the same way one subclause over.
    ///
    /// The planted violation is a tagged document whose `math` element in §14.8.6.3's namespace
    /// has no `Formula` above it; the pair is the same file with one inserted. The second is what
    /// says the note is about the *enclosure* rather than about `MathML` being present at all —
    /// which matters, because §14.8.6.2 names that namespace as one an element may be in, so a
    /// note that fired on its presence would contradict the subclause beside it.
    #[test]
    fn mathml_that_no_formula_encloses_is_said_out_loud() {
        let tagged = "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
                      /StructTreeRoot 4 0 R >>";
        let pages = "<< /Type /Pages /Kids [] /Count 0 >>";
        let mathml = "<< /Type /Namespace /NS (http://www.w3.org/1998/Math/MathML) >>";
        let root = "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [3 0 R] >>";
        let math = "<< /Type /StructElem /S /math /NS 3 0 R >>";

        let said = about(
            &document(&[tagged, pages, mathml, root, math]),
            &crate::TrustPolicy::default(),
        )
        .join("\n");
        assert!(
            said.contains("1 of its structure elements are §14.8.6.3's MathML"),
            "the planted violation is named, with the count: {said}"
        );

        let formula = "<< /Type /StructElem /S /Formula /K [6 0 R] >>";
        assert!(
            !about(
                &document(&[tagged, pages, mathml, root, formula, math]),
                &crate::TrustPolicy::default()
            )
            .join("\n")
            .contains("§14.8.6.3"),
            "a Formula around it is the clause satisfied"
        );

        // And §14.8.1's condition is this note's as well: the subclause is inside §14.8, whose
        // requirements are on a tagged PDF, and a file that does not claim to be one has not
        // taken them on.
        let untagged = "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>";
        assert!(
            !about(
                &document(&[untagged, pages, mathml, root, math]),
                &crate::TrustPolicy::default()
            )
            .join("\n")
            .contains("§14.8.6.3"),
            "a document that does not say it is tagged is outside the clause"
        );
    }

    /// A namespace dictionary with no `/NS` of its own is named as what it is, not as a name.
    ///
    /// Table 356 makes the entry required, so such a dictionary identifies nothing — and the two
    /// sentences a person can be given here are different: *your elements are in somebody's
    /// tagset* and *your elements are in a namespace your file does not name*. `Tree::namespace`
    /// already refuses to answer the default for one; this is the same refusal one layer up.
    #[test]
    fn a_namespace_that_states_no_name_is_reported_as_naming_nothing() {
        let said = about(
            &document(&[
                "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>",
                "<< /Type /Pages /Kids [] /Count 0 >>",
                "<< /Type /Namespace /Schema 9 0 R >>",
                "<< /Type /StructTreeRoot /K [5 0 R] /Namespaces [3 0 R] >>",
                "<< /Type /StructElem /S /Widget /NS 3 0 R >>",
            ]),
            &crate::TrustPolicy::default(),
        )
        .join("\n");
        assert!(
            said.contains("a namespace whose dictionary states no name of its own"),
            "{said}"
        );
    }

    /// **A host names an anchor, and every sentence about the third question changes.**
    ///
    /// The corpus's certification signature is the witness and its answer is the honest one: its
    /// signer's certificate was issued by a real authority, this project holds no root of that
    /// authority's, and no path from it reaches the root supplied here. So the report says the
    /// third question *was* asked, says where the answer's authorities came from, and says the
    /// signature is not called valid — naming the step that stopped it rather than going quiet.
    ///
    /// **This is trap 8 written as a test.** No document in any corpus on this machine can supply a
    /// path whose positive outcome is known in advance; the only anchors that exist for the crawl's
    /// signed documents are their own issuers' roots, which nobody here holds. A *positive* verdict
    /// is `pdf_signature::verdict`'s own fixture, over a hierarchy this project issued.
    #[test]
    fn an_anchor_a_host_supplies_changes_what_the_third_question_is_answered_with() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/pdf.js/test/pdfs/xfa_filled_imm1344e.pdf");
        let Ok(bytes) = std::fs::read(&path) else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        let document = Document::open(bytes).expect("a valid file");
        let trust = crate::TrustPolicy {
            anchors: pdf_signature::trust::Supply::of(
                vec![
                    ("a-root.der".to_owned(), hex(A_ROOT)),
                    ("notes.txt".to_owned(), b"not a certificate at all".to_vec()),
                ],
                "the directory a test named".to_owned(),
                pdf_signature::x509::Instant::from_unix_seconds(1_790_812_800),
            ),
            acceptance: pdf_signature::verdict::Acceptance::RevocationMustBeGood,
        };
        let said = about(&document, &trust).join("\n");

        assert!(
            said.contains("were supplied by whoever started it, from the directory a test named"),
            "a verdict is never separable from where its anchors came from: {said}"
        );
        assert!(
            said.contains("1 of 2 read as RFC 5280 certificates"),
            "{said}"
        );
        assert!(
            said.contains("notes.txt is not a certificate this reader can use as a trust anchor"),
            "what a host handed over and this reader would not take is named (trap 5): {said}"
        );
        assert!(
            said.contains("all three of the questions a signature asks were asked"),
            "the third question is asked once somebody names an anchor: {said}"
        );
        assert!(
            said.contains("not called valid here, and this is the first thing that stopped it"),
            "{said}"
        );
        assert!(
            said.contains("no certification path from the signer's certificate reaches any anchor"),
            "the corpus's authorities are not this project's to anchor at (trap 8): {said}"
        );
        assert!(
            !said.contains("that signature is valid"),
            "nothing in the crawl reaches the word: {said}"
        );

        // **And with no anchor the report is the one this program has printed since the
        // three-hundred-and-seventy-seventh session**, which is what makes the supply an input
        // rather than a change of behaviour.
        let quiet = about(&document, &crate::TrustPolicy::default()).join("\n");
        assert!(
            quiet.contains("no certificate store and makes no network request"),
            "{quiet}"
        );
        assert!(
            !quiet.contains("were supplied by whoever started it"),
            "{quiet}"
        );
        assert!(!quiet.contains("not called valid here"), "{quiet}");
    }

    /// Hexadecimal to bytes, as `pdf-signature`'s fixture modules spell it.
    fn hex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks(2)
            .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
            .collect()
    }

    /// A self-signed root `openssl` issued for `pdf_signature::verdict`'s fixtures, 2026 to 2036.
    ///
    /// Held here as well as there because a `#[cfg(test)]` fixture is not public API and a crate
    /// boundary is not a thing to punch a hole in for a test. What it stands for is a *host*: the
    /// party that reads certificates off a disk and hands them in, which is the whole subject of
    /// the test above.
    const A_ROOT: &str = "\
    308203373082021fa00302010202140ac4cbde908c14cb6d200f4a1e388b4d07\
    4edd96300d06092a864886f70d01010b050030233121301f06035504030c1871\
    756f7272612076657264696374207465737420726f6f74301e170d3236303130\
    313030303030305a170d3336303130313030303030305a30233121301f060355\
    04030c1871756f7272612076657264696374207465737420726f6f7430820122\
    300d06092a864886f70d01010105000382010f003082010a0282010100ac4a44\
    39adb83d24ac0f79a4476d321390049d5c125fb796420bbbcc21c38c47b41bec\
    5342583558dfb7eda004baa3efb29cce4e57babd406201a008912d10035bb669\
    7e77ab7ed9d398bb009b74f36fdd1db77ccd5ab90617a94ed0f8f99306255e47\
    6b3c1fb26d448e704091ba89451c9d7b6f49ab19a9051578b2cf28ffe516d304\
    216242babafc99949123da92f14d680939c9a5d9b9413e7293629c2f9a1af506\
    7177df72d3fd9c8ea0404fc656d269c890a32f9b864f4b650a8fbb0b2a65000f\
    9d09f3c625d595fa3ebec5b9acd3d420ce7a1b81990ea248dda086d310080d8c\
    eb975fa405150048ba50ab72d4907b0e8db725a684153c984519a33bc3020301\
    0001a3633061301d0603551d0e041604146a7d48af4c1be8f361759023106e3d\
    88080f143f301f0603551d230418301680146a7d48af4c1be8f361759023106e\
    3d88080f143f300f0603551d130101ff040530030101ff300e0603551d0f0101\
    ff040403020106300d06092a864886f70d01010b050003820101006b5285e666\
    1185000da52beb7756bf6b42f7fb36def8d7761d76754a8bfab627d6fa2832bc\
    db4abc7462aed0eaf56c10613ac357e0843c7eb943d51143f13221784725ec18\
    8be7b847295209fb8e225b6d8a3f2db9b038ef7f1280d35eaa552d496efd2746\
    ec1df0f38f473f374afac0798f0809058cd167fde883e58a1b85c67a90fe73dc\
    d288e6f66c6f4f0e4164f55d813676213bfb96b4210e3dda72bf317f1c66e29e\
    01145ff95d50573d9f49ce9a3ffacfc3be2129836e13390ec4d42d7da886f8ab\
    40be249e1ede625527e447d12d26ab4e7e1b9358621fa709fc05dde4fb1dfdb4\
    87de183084bf0c50f5693a4a5a35b3a7d0d81bae8a12ecbe42e482";
}
