//! What a document says about itself, said out loud once when it opens.
//!
//! None of what is said here is about a page. §12.11's requirements, §12.8's signatures,
//! §7.11.4's embedded files, §14.13.2's associated files that are *not* embedded, §7.5's
//! recovered cross-reference table, §7.7.3's page tree recovered by scanning, Annex I's version,
//! Annex K's XFA form, §14.8.6.2's namespaces and §14.8.6.3's unenclosed `MathML` are all
//! claims about the *file*, and a person deciding whether to trust what they are looking at needs
//! them whatever page is in front. That is why they are a [`crate::Event::Reported`] with no page
//! rather than part of the page's own report, answered when a host sends
//! [`crate::Command::Report`] once the document's first frame is on its screen (ADR 1044).
//!
//! Nothing here is an error and nothing here stops a document opening.

use pdf_signature::signature::{
    PadesDeparture, PadesProfile, ReferenceDigest, SigningCertificateBinding,
};
use pdf_signature::verdict::{BestSignatureTime, Proof};
use pdf_syntax::Document;

/// What [`about`] answers: the sentences, and the policy documents the signatures say a copy of
/// can be fetched from.
///
/// The second half is not words, because what a host does with it is not saying: ETSI EN 319
/// 122-1 clause 5.2.9.2's URL qualifier names a server, and whether to ask it is the reader's
/// network level, which only a host holds (ADR 1738).
#[derive(Debug, Default)]
pub(crate) struct About {
    /// Everything worth saying, in order.
    pub(crate) notes: Vec<String>,
    /// Each signature's [`pdf_signature::policy::SignaturePolicy::published`] list, in signature
    /// order, with a URL a second signature names under the same commitment listed once.
    pub(crate) published: Vec<pdf_signature::policy::PublishedPolicy>,
    /// The location of every trusted list a bound policy document's `TAsInTrustedList`s
    /// reference, each once (TS 119 172-2 clause 4.22.2.1 b)): what
    /// [`crate::Query::TrustedListReferences`] answers (ADR 2936).
    pub(crate) references: Vec<String>,
}

/// What a document's signatures name that only a host may fetch, gathered as its report is
/// worded: [`About`]'s two lists.
#[derive(Debug, Default)]
struct Fetchable {
    /// As [`About::published`].
    published: Vec<pdf_signature::policy::PublishedPolicy>,
    /// As [`About::references`].
    references: Vec<String>,
}

impl Fetchable {
    /// The report: `notes`, and these lists beside them.
    fn beside(self, notes: Vec<String>) -> About {
        About {
            notes,
            published: self.published,
            references: self.references,
        }
    }
}

/// A copy of a signature policy's document a host fetched from the URL ETSI EN 319 122-1 clause
/// 5.2.9.2's qualifier names and handed back as [`crate::Command::PolicyCopy`].
///
/// Kept beside the document rather than in it — the document is immutable, and what a server
/// answered is not what the file says — and read only through
/// [`pdf_signature::policy_rules::enforce_fetched`], which binds it from the signature's own
/// identifier: [`Self::published`] says which copy a host fetched, never whose it is (ADR 2624).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchedCopy {
    /// The published copy a host was handed in [`crate::Event::SignaturePoliciesPublished`].
    pub(crate) published: pdf_signature::policy::PublishedPolicy,
    /// What the server answered with.
    pub(crate) octets: Vec<u8>,
}

/// What to say of a file whose `/Prev` named bytes that are no section.
///
/// Table 15's `/Prev` names the previous cross-reference section, and where the bytes it names
/// are no section the objects that section described are found by their own headers (§7.3.10)
/// instead. The pages are the file's; that the file's own table could not say where they are is
/// worth saying, as it is for a table that was rebuilt. The same all-or-nothing care applies:
/// object streams found by their own headers may not all read, and what a stream that did not
/// read holds is missing.
fn cut_chain_note(document: &Document) -> Option<String> {
    if !document.cross_reference_chain_was_cut() {
        return None;
    }
    let recovered = document.compressed_objects_recovered();
    let unread_streams = recovered
        .unreadable
        .saturating_add(recovered.beyond_the_budget);
    let compressed = if unread_streams == 0 {
        String::new()
    } else {
        format!(
            "; {unread_streams} of its {} object streams (§7.5.7) could not be read, so what \
             they hold is missing from what you see",
            recovered.streams
        )
    };
    Some(format!(
        "this file's cross-reference sections could not all be read (a /Prev names bytes that \
         are no section, §7.5.5), so the objects they describe were found by scanning for their \
         own headers{compressed}"
    ))
}

/// What a reader is told where a revision 6 document's permissions are its plaintext `/P`
/// because §7.6.4.3.3 step (f)'s `/Perms` check did not pass (A953, ADR 3309).
///
/// The step is the one check of a key unwrapped from `/UE` or `/OE`, and the document opens
/// without it, so the sentence says what was not checked rather than that anything is wrong:
/// the page decrypts either way, and the restrictions are the ones written beside the block.
fn unbound_permissions_note(document: &Document) -> Option<String> {
    use pdf_syntax::UnboundPermissions;
    let why = match document.unbound_permissions()? {
        UnboundPermissions::Absent => "states no /Perms block",
        UnboundPermissions::Short => "states a /Perms block shorter than 16 bytes",
        UnboundPermissions::Unverified => {
            "states a /Perms block that does not decrypt to its \"adb\" marker under the key"
        }
    };
    Some(format!(
        "this file's encryption {why} (§7.6.4.3.3 step (f)), so its permissions are not bound to \
         its encryption key: they are read from the /P entry as written, and nothing checked \
         that the key was the one its producer used"
    ))
}

/// Everything worth saying about a document the moment it opens, with the copies of its
/// signatures' policy documents a host has fetched so far.
pub(crate) fn about(
    document: &Document,
    trust: &crate::TrustPolicy,
    copies: &[FetchedCopy],
) -> About {
    let mut notes = Vec::new();
    let mut fetchable = Fetchable::default();

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

    notes.extend(cut_chain_note(document));
    notes.extend(unbound_permissions_note(document));

    // §7.7.3.1's page tree "defines the ordering of pages in the document", and where it yields
    // no page `pdf_model::Pages` finds them by Table 31's `/Type /Page` instead, in ascending
    // object number. Every page shown is the file's; which pages and in what order is this
    // reader's, and a scan shown as if it were a tree is a recovery said by nobody (trap 5).
    // `Pages::new` is a catalogue lookup and, where the tree is believed, one descent, and this
    // function is off the open path, so the question costs the launch nothing (ADR 1533).
    if let Some(found) = pdf_model::Pages::new(document).found_by_scanning() {
        notes.push(format!(
            "this file's page tree leads to no page (§7.7.3), so its {found} page(s) were found by \
             scanning for objects that declare themselves pages, and are shown in the order of \
             their object numbers rather than an order the file states"
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
    // The sentence names two causes and gives each the same two-part warning: that the
    // document is unlikely to be read successfully, and that the user may not be able to change
    // or save it. The version is the first cause; the requirements after it are the second, and
    // their warning follows the loop that names them (ADR 2815). **No corpus document reaches
    // the version note**: the newest of the 974 states 2.0, which is what this program is
    // written against, so this note exists for the file that has not been written yet.
    if let Some(version) = document.version()
        && version > pdf_syntax::Version::SUPPORTED
    {
        notes.push(format!(
            "this document states PDF {version}, which is newer than the {} this program \
             implements — it is unlikely to be read successfully, and you may not be able to \
             change or save it (Annex I)",
            pdf_syntax::Version::SUPPORTED
        ));
    }

    // §12.11's document requirements. The clause makes this a statement about the *document*
    // rather than about any page — "there is no formal connection between the requirement type
    // and the operation of the associated feature(s)" — so it belongs here rather than in a
    // page's report, and it is the one thing a page report cannot do: tell a person before they
    // trust what they are looking at. What §12.11.6 asks a processor to do about the total is a
    // level this reader sets — `restriction::Operation::Process`, `off` by default — and these
    // lines are said whatever it is, because naming what could not be promised is a report about
    // the file rather than an obedience to it (ADR 1167). The level decides what is done with the
    // total, never which requirements count: each is judged by its feature and by Table 273's
    // `/V`, the minimum PDF version it needs (`Requirement::unmet`, ADR 2966).
    let unmet = pdf_model::requirements::unmet(document);
    for (requirement, reason) in &unmet {
        notes.push(format!(
            "this document requires {} (penalty {}) — {reason}",
            requirement.kind.as_str(),
            requirement.penalty
        ));
    }
    // Annex I's warning for its second cause, said once for every requirement named above.
    if !unmet.is_empty() {
        notes.push(
            "this document states requirements this program is not prepared to process, so it \
             is unlikely to be read successfully, and you may not be able to change or save it \
             (Annex I)"
                .to_owned(),
        );
    }

    // §12.11.3's own threshold, which the clause states:
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
             should not attempt to display it — it is on the screen because this reader's level \
             for processing a document says so, with each requirement named"
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

    external_associated_files(document, &mut notes);
    xfa_form(document, &mut notes);
    tagged_structure(document, &mut notes);
    signatures(document, trust, copies, &mut notes, &mut fetchable);
    fetchable.beside(notes)
}

/// §14.13.2's *other* form, and the reason it is said here rather than listed with [`about`]'s
/// embedded files: an associated file's specification "represents either a file external to the PDF file
/// or an embedded file stream", and an external one has no bytes in the document at all. This
/// program cannot follow it — principle 3 gives the renderer no filesystem — but naming it
/// needs no filesystem, and §7.11's own reading is that refusing a file and being unable to
/// name it are different things. A document that associates a file nobody here can reach is
/// exactly the kind of fact this module exists for: what you are looking at is not the whole of
/// what the producer assembled. **The catalog is the scope**, because this is said once, when
/// the document opens; the attachments panel lists the same files again, as rows with no save
/// action, together with §14.13.4's of the page in view (`Query::ExternalFiles`, the owner's
/// `doc/questions/A45`, ADR 2799). §14.13.5 to §14.13.9's other carriers are read by
/// `attachment::external_associated` and reach no caller. No corpus document states one, so the
/// witness is hand-built (trap 8); ADR 0918 has the counts.
fn external_associated_files(document: &Document, notes: &mut Vec<String>) {
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
}

/// What a print of this document will not do with it, said once per print job before it starts.
///
/// Not part of [`about`], because it is a statement about an *operation* rather than about the
/// file: a reader who never prints is told nothing, and one who prints twice is told twice,
/// once before each job. `crate::Viewer`'s `begin_printing` says it.
///
/// **§14.11.5's output intent, on the way to paper.** The interpreter reads a document's device
/// colours through the intent's profile in its "to CIE" direction, which Table 401 names as the
/// one for "screen preview or hardcopy proofing"; the same table makes the other direction the
/// one a print to the intended device uses:
///
/// > The output transformation uses the profile's "from CIE" information (BToA in ICC
/// > terminology); the "to CIE" (AToB) information may optionally be used to remap source colour
/// > values to some other destination colour space, such as for screen preview or hardcopy
/// > proofing.
///
/// This program has no evaluator for that direction and hands its print system RGB, so a print
/// carries the colours as the screen shows them and the output transformation is not performed —
/// the clause's own permission, "PDF processors are free to disregard it", taken for output. The
/// condition is the interpreter's: an intent in force for some page, read by the selection the
/// interpreter uses, so the device named is the one the colours were drawn through (RFC 0009
/// §2.6, `doc/questions/A254`, ADR 1932).
pub(crate) fn what_a_print_does_not_do(document: &Document) -> Vec<String> {
    let intents = pdf_model::output_intent::in_force_on_pages(document);
    if intents.is_empty() {
        return Vec::new();
    }
    let named: Vec<String> = intents
        .iter()
        .map(|intent| {
            let device = intent
                .condition
                .clone()
                .unwrap_or_else(|| "a device its output intent does not name".to_owned());
            intent
                .subtype
                .as_ref()
                .map_or(device.clone(), |subtype| format!("{device} ({subtype})"))
        })
        .collect();
    let (profile, device) = if named.len() > 1 {
        ("those profiles", "those devices'")
    } else {
        ("that profile", "that device's")
    };
    vec![format!(
        "this document's colours were prepared for {} (§14.11.5's output intent); this program \
         reads them through {profile} and prints them as it shows them — the output \
         transformation into {device} colourants, Table 401's from-CIE direction, is not \
         performed, and this machine's print system manages the colour from there",
        named.join(" and ")
    )]
}

/// Annex K's form, said to be one once per document, with what of it is carried out.
///
/// §K.1's requirement on a processor that supports XFA forms, which this program does for a static
/// form's data (`CLAUDE.md`'s XFA entry, ADR 2430):
///
/// > If an interactive processor supports XFA forms, that processor shall clearly indicate to the
/// > user when they are interacting with an XFA form that they are interacting with an XFA form.
///
/// And the permission it takes for a form whose template generates its pages:
///
/// > The implementation of such a schema driven page generation involves considerable effort
/// > beyond that for a simple PDF viewer and therefore a PDF processor may choose to not
/// > implement this feature.
///
/// Taking it silently is what is owed against: a dynamic form's pages are often a notice
/// addressed to processors that decline, and a hybrid form's own calculations and validations
/// live in the XFA resource, so a reader who is not told sees a notice or a quiet form and no
/// reason (trap 5). The
/// sentence names what *is* done, because §K.2 is what makes it the form: "\[t]he other entries
/// in the interactive form dictionary shall be consistent with the information in the XFA
/// resource". Both conditions are the annex's own entries — Table 224's `/XFA` in either of
/// §K.2's two forms, and Table 29's `/NeedsRendering` stated `true` — and
/// [`pdf_model::xfa::stated`] reads them without decoding a packet (RFC 0009 §4.4, ADR 1932).
/// A save keeps a static form's data consistent with what was filled, by §K.2's writer rule, and
/// names each field it cannot keep where it saves (ADRs 2396, 2414); it writes the data through
/// the form's own pictures and runs its format tests and, where the form asks for its validations
/// before a save, its null and datatype tests (ADRs 2434, 2450, 2474); a submit runs those and the
/// format test, naming each failure on the submission (ADR 2475); the form is put to its format
/// tests when it is shown and an entry to its field's as it is made, and the `submit` an event
/// states in a script's place is made, at every level, since neither is a script (ADRs 2727,
/// 2767), in each packaging XFA 3.3 page 859 defines a body for: PDF, XFDF, URL-encoded, the
/// form's XDP packets and its XML data (ADR 2771). A reader
/// whose level lets scripts run also carries out the form's calculations, its validation scripts
/// and the events its containers raise when it is shown, at a person's entry, focus, click and
/// pointer, at a choice list's selection, as an entry fills a field, at a save, a submit and a
/// print, as it closes and at a change of validation state, with how a person committed or
/// abandoned a field's value, in `FormCalc` and in JavaScript (ADRs 2524, 2525, 2545, 2566, 2584,
/// 2606, 2627, 2647, 2696, 2702, 2706, 2707, 2726, 2738, 2755, 2756, 2777, 2783), as ledger row
/// K.2 lists them; when it is shown is when its layout, the pages the file holds, is complete. So what is not carried out is its other
/// events, a submit that asks to be signed or encrypted, and its scripts where scripts do not run.
fn xfa_form(document: &Document, notes: &mut Vec<String>) {
    let Some(stated) = pdf_model::xfa::stated(document) else {
        return;
    };
    let declined = "this program does not generate a form's pages from its XFA template, which \
                    §K.1 permits a processor to choose";
    notes.push(match (stated.resource, stated.needs_rendering) {
        (true, false) => "this document carries an XFA form (Annex K); the form shown and filled \
                          here is its AcroForm, which §K.2 requires to be consistent with the XFA \
                          resource, and saving it keeps the form's XFA data consistent with what \
                          was filled, in the form's own data formats, naming any field it \
                          cannot and any field the form's validations find empty or of the wrong \
                          type, and submitting it names the same; a value the form's format \
                          test refuses is said when the form is shown and an entry it refuses \
                          as it is made, and a submit the form's events state as a PDF, XFDF, \
                          URL-encoded, XDP or XML body is made at every level, since neither is \
                          a script — \
                          what only the XFA resource states beyond that is not carried out, its \
                          calculations, its validation scripts and the scripts of the events a \
                          person's actions, the form's opening and closing, a save, a submit, a \
                          print, a drop-down list's opening and a change of validation state \
                          raise, in FormCalc and JavaScript, excepted where this reader lets \
                          scripts run, a save, a print or a list's opening those scripts decline \
                          going ahead or not at the level set for it, and its other events and a \
                          submit that asks to be signed or encrypted are not"
            .to_owned(),
        (true, true) => format!(
            "this document is an XFA form that asks to be regenerated when it is opened \
             (Table 29's /NeedsRendering, Annex K); {declined}, so the pages shown are the ones \
             the file holds for processors that do not regenerate it, which are often only a \
             notice, and the form filled here is its AcroForm"
        ),
        (false, _) => format!(
            "this document asks to be regenerated from an XFA form when it is opened (Table \
             29's /NeedsRendering) but carries no XFA resource for it (§K.2); {declined}, and \
             the pages shown are the ones the file holds"
        ),
    });
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
/// **Two conditions, both the clause's.** §14.8.1 is what makes a document tagged — "\[a] tagged
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
/// §C.4 (informative) licenses the scan — "\[w]hen a PDF processor reads a PDF file with a
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
/// **A function of the restrictions rather than of the document** (ADR 0803): the reading and
/// the policy are both `pdf_model::restriction::decide`'s, and what reaches here is what the
/// verdict carried. The same list is worded for all three verdicts a window receives (ADR 0814).
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
            // §12.7.5.5's Table 236 `/P`, whose entry opens "[t]he access permissions granted
            // for this document" and says what they reach: "any incremental changes to the
            // document following the signature of which this key is part". A person is told the
            // signature's own lock rather than the author's certification, because they are two
            // clauses and a reader who would accept one may refuse the other (ADR 1156).
            Restriction::LockPermission { level } => match level {
                Modification::None => format!(
                    "a signature in this document permits no change to it at all (§12.7.5.5's \
                     /Lock /P 1) — {tail}"
                ),
                Modification::FormFilling => format!(
                    "a signature in this document permits only form filling and signing \
                     (§12.7.5.5's /Lock /P 2), which does not include {} — {tail}",
                    operation.as_str()
                ),
                // Level 3 and an undefined level both permit, so `asserted` never names them
                // here; saying which one arrived is better than a sentence that claims a rule.
                other => format!("this document's /Lock /P states {other:?} — {tail}"),
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
                 contents, a form field's value among them, may not be changed — {tail}"
            ),
            // §12.11.6, and the number is §12.11.3's: "if the penalty value exceeds 100 then the
            // PDF processor should not attempt to display or process the document." The sentence
            // names the total rather than only the fact, because the clause's own arithmetic is
            // what a person is being asked to act on — and it says which requirements it is a
            // total of, which `about`'s lines name one by one.
            Restriction::RequirementsUnmet { penalty } => format!(
                "this document requires features this program cannot meet, totalling {penalty} \
                 penalty points over the 100 above which §12.11.3 says a processor should not \
                 attempt to display or process it — {tail}"
            ),
            // Table 286's `/TF`, named by its own value so that a person can see which of the
            // four the author chose (ADR 1961).
            Restriction::MediaTemporaryFile { value } => format!(
                "this document's media permissions say ({}) for the temporary file a player \
                 would be handed (ISO 32000-2 Table 286's /TF) — {tail}",
                value.as_str()
            ),
            // XFA 3.3 page 402's flag, named as the form's own script so that a person can tell
            // it from every statement the file makes in ISO 32000-2's own terms (ADR 2763).
            Restriction::SaveCancelled => format!(
                "this document's XFA form has a preSave script that set $event.cancelAction, \
                 which XFA 3.3 page 402 says causes the save to abort — {tail}"
            ),
            // The same flag under `prePrint`, which page 399 has cancel "whatever action fired
            // the event" (ADR 2777).
            Restriction::PrintCancelled => format!(
                "this document's XFA form has a prePrint script that set $event.cancelAction, \
                 which XFA 3.3 page 399 says cancels the print — {tail}"
            ),
            // The same flag under a drop-down list's `preOpen` (page 392), which declines to show
            // the list the reader asked for (ADR 2791).
            Restriction::OpeningCancelled => format!(
                "this document's XFA form has a preOpen script that set $event.cancelAction, \
                 which XFA 3.3 page 399 says cancels the opening of the field's list — {tail}"
            ),
            // §12.2's `/Enforce` over Table 148's one name (ADR 2840).
            Restriction::PrintScalingEnforced => format!(
                "this document asks that its print scaling not be changed (ISO 32000-2 §12.2, \
                 Table 147's /Enforce names /PrintScaling) — {tail}"
            ),
        })
        .collect()
}

/// §12.8's signatures: what this program can honestly say about one, given what a host supplied.
///
/// **A signature asks three questions** (§12.8.1, ADR 0215). This program says who signed, why,
/// whether the range they signed runs to the end of the file (§12.8.1), whether the bytes that
/// range names still hash to the digest the signature records, and **whether the signature
/// verifies under the public key in the certificate the file itself carries** (§12.8.3.3.1,
/// ADR 0229).
///
/// **The third is answered when, and only when, a host has named an anchor.** RFC 5280 section
/// 6.1.1 makes the anchors input (d) and a matter of policy, ADR 1039 made them a host's to supply
/// and this crate's to receive, and [`crate::Command::Trust`] is the receiving. With none — which
/// is every host's default and what a host that says nothing gets — every sentence below is the
/// one this program prints without an anchor (ADR 0215), the closing paragraph included.
///
/// **And a verdict is never separable from where its anchors came from.** [`anchors_note`] is that
/// sentence: a reader told that a signature is valid is owed *valid according to whom*, and only
/// the host that read the certificates can say.
fn signatures(
    document: &Document,
    trust: &crate::TrustPolicy,
    copies: &[FetchedCopy],
    notes: &mut Vec<String>,
    fetchable: &mut Fetchable,
) {
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
    // ETSI TS 119 615 clause 4.1's "has passed" is asked at the host's instant, the one the
    // anchors carry.
    let list_reading = trust.lists.read(trust.anchors.at());
    let lists = &list_reading.lists;
    // §12.8.4's store and §12.8.5's chain are read once here and lent to the three functions that
    // word them: `Signature::trust` needs the material, the chain needs it too, and a verdict on a
    // signature needs to know whether the document states a time this program could establish.
    let store = pdf_signature::signature::security_store(document);
    let material = store.material();
    let chain = pdf_signature::timestamp::chain(
        document,
        &reading.anchors,
        lists,
        &material,
        trust.anchors.at(),
    );
    let length = document.bytes().len() as u64;
    let timestamps = standing(&chain);
    for (index, signature) in signatures.iter().enumerate() {
        // §12.8.3.4.5 (b) names three ways a signature may be verified against a time other than
        // the current one — the security store, a document timestamp, and "a signature timestamp
        // … present in the signature as an unsigned attribute" — and states no order among them.
        // Which of two established instants to take is not a question of order at all: ETSI
        // EN 319 102-1 clause 5.5.4 step 3) b) takes the *earliest*, because what is wanted is the
        // earliest moment the signature is proven to have existed. Both are instants only once an
        // authority is established, which is a host's anchor away (ADR 1039), and the reader's
        // clock is the ceiling neither may exceed.
        let mut when = BestSignatureTime::now(trust.anchors.at());
        if let Some(at) = established_over(&chain, index) {
            when = when.proven_at(at, Proof::ADocumentTimestamp);
        }
        if let Some(at) = signature_timestamps_instant(
            signature,
            &reading.anchors,
            lists,
            &material,
            trust.anchors.at(),
        ) {
            when = when.proven_at(at, Proof::ThisSignaturesTimestampAttribute);
        }
        let asked = Asked {
            anchors: &reading.anchors,
            material: &material,
            when,
            acceptance: trust.acceptance,
            timestamps,
            copies,
            lists,
            supplied: pdf_signature::trusted_list::anchors::Supplied {
                reading: &list_reading,
                now: trust.anchors.at(),
            },
            chain: &chain,
            index,
            now: trust.anchors.at(),
            signatures: &signatures,
        };
        about_one(signature, document, length, &asked, notes, fetchable);
        modifications(signature, document, &permissions, notes);
    }
    permissions_stated(&permissions, notes);
    security_store(&store, &material, notes);
    document_timestamps(&chain, notes);
    anchors_note(trust, &reading, notes);
    lists_note(trust, &list_reading, notes);
    // The three questions, named, in the order §12.8.1 states them. This paragraph is what stops
    // the sentences above it being read as "the signature is good": where nobody named an anchor
    // two of them are answered, and the third is the one that decides whether a signature means
    // anything about a *person*, because a key that verifies is a key and this program has nothing
    // to say about whose.
    //
    // **Keyed on what *read*, not on what was handed over.** A host that named a directory of six
    // files none of which is a certificate has supplied no anchor, and the paragraph below would
    // otherwise tell a reader that the third question was asked when nothing could ask it.
    notes.push(if reading.anchors.is_empty() && lists.is_empty() {
        "of the three questions a signature asks, this program answers two: whether the document \
         changed since it was signed (§12.8.1's digest, recomputed above) and whether the \
         signature verifies under the public key in the certificate the file itself carries \
         (§12.8.3.3.1). It does not answer the third — it has no certificate store and makes no \
         network request, so it does not know whether that certificate belongs to anyone you have \
         reason to believe. It says whether a certificate was revoked only from the material the \
         file itself carries, and only once somebody names a certification authority to end the \
         chain at, which nobody has done for this reading. A signature that verifies here was \
         made by whoever holds the key in a certificate that arrived with the document, which is \
         not the same as a valid signature. Nothing here says valid"
            .to_owned()
    } else {
        "all three of the questions a signature asks were asked of this document: whether it \
         changed since it was signed (§12.8.1's digest, recomputed above), whether the signature \
         verifies under the public key in the certificate the file carries (§12.8.3.3.1), and \
         whether a certification path from that certificate reaches an authority you named or a \
         service a trusted list you supplied publishes (RFC 5280 section 6.1). The third was asked \
         because you named one; it is your answer this \
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
    ///
    /// ETSI EN 319 102-1 clause 5.5.4's best-signature-time: the reader's clock unless a token
    /// proved an earlier moment. It carries which token did, because a reader told a path was
    /// validated is owed *as of when* and *on whose word*.
    when: BestSignatureTime,
    /// What the host will accept where the material answers nothing.
    acceptance: pdf_signature::verdict::Acceptance,
    /// What §12.8.5's chain contributes, computed once for the document.
    timestamps: pdf_signature::verdict::Timestamps,
    /// The policy documents a host fetched, which §12.8.3.4.4's constraints are enforced from
    /// where the file carries no bound copy of its own.
    copies: &'a [FetchedCopy],
    /// The trusted lists the host supplied that read (ETSI TS 119 612).
    lists: &'a [pdf_signature::trusted_list::TrustedList],
    /// The same reading with the host's names for its files and its instant: what a policy's
    /// trust anchors given by reference to a trusted list are determined from (ETSI TS 119 172-2
    /// clause 4.22.2.1 b), ADR 2874).
    supplied: pdf_signature::trusted_list::anchors::Supplied<'a>,
    /// §12.8.5's chain, whose links name the signatures they cover by position.
    chain: &'a pdf_signature::timestamp::Chain,
    /// Every signature, [`pdf_signature::signature::signatures`]'s first and in its order — the
    /// positions the chain's links name.
    signatures: &'a [pdf_signature::signature::Signature],
    /// This signature's position among the document's signature dictionaries.
    index: usize,
    /// The host's instant, at which a token's authority is asked about.
    now: pdf_signature::x509::Instant,
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
    lists: &[pdf_signature::trusted_list::TrustedList],
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
        lists,
        material,
        pdf_signature::timestamp::AskedAt::TheCallersInstant(now),
    )? {
        // §12.8.3.4.8's three cases on the timestamping unit's path: nothing revoked, and the
        // time "can be considered as reliable"; revoked for "keyCompromise" or "cACompromise" or
        // with no reason, and it "cannot be used"; revoked for any other reason, and it is usable
        // "only if the time of the revocation of that timestamping unit's certificate occurred
        // after the UTC time included in the timestamp token" (ADR 2968).
        pdf_signature::timestamp::Time::Established { at, revocation, .. } => {
            use pdf_signature::revocation::{Revocation, RevocationReason};
            match revocation {
                Revocation::Revoked {
                    reason:
                        None | Some(RevocationReason::KeyCompromise | RevocationReason::CaCompromise),
                    ..
                } => None,
                Revocation::Revoked { at: revoked, .. } => {
                    (revoked.unix_seconds() > at.unix_seconds()).then_some(at)
                }
                _ => Some(at),
            }
        }
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
/// this worth saying at all: "\[a] PDF signature may not be successfully verified unless its
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
                "the §12.8.4.4 validation information for signature {} was recorded at {}, by a \
                 timestamp token (Table 262's TS). When that was is the token's claim, not this \
                 program's finding",
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
/// **The condition is the clause's**: §12.8.5.2 says how a reader finds them — "\[t]he existence of
/// one or more document timestamps shall be determined by examining signature fields" — and
/// §12.8.5.3 says why there is more than one, which is the sentence this report exists to make
/// legible. A later token is applied "before the expiry of the certificate and/or before a
/// cryptographic attack may succeed", over everything the earlier one left, and what a person
/// wants to know is whether the stack actually holds: does the newest one cover the older ones,
/// and does it cover the material §12.8.4's store carries for them.
///
/// **What it may not say is a time.** §12.8.5.1 makes a timestamp's whole point the instant — "\[a]
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
        Some(pdf_signature::timestamp::Time::Established { at, through, .. }) => format!(
            "the outermost of those timestamps establishes an instant: its authority's \
             certification path reaches {}, its token's signature verifies, and the token's \
             contents are bound to that signature (§12.8.5.2, RFC 3161). The instant is {} \
             seconds after the epoch. What it establishes is the state of the file at that moment \
             and nothing about what the file says",
            authority_words(through),
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

/// Whose anchor a token's authority's path ended at, as a phrase.
fn authority_words(through: &pdf_signature::timestamp::Authority) -> String {
    match through {
        pdf_signature::timestamp::Authority::Listed(listed) => format!(
            "the time-stamping service \"{}\" of \"{}\", which the trusted list of {} (sequence \
             number {}) publishes as a service of type {} (ETSI TS 119 615 clause 4.3){}",
            listed.service,
            listed.provider,
            listed.territory,
            listed.sequence_number,
            listed.service_type,
            if listed.authentication
                == pdf_signature::trusted_list::Authentication::VouchedForByTheHost
            {
                ", a list believed because the host that supplied it vouched for it"
            } else {
                ""
            }
        ),
        pdf_signature::timestamp::Authority::ReadersAnchor => "an anchor you supplied".to_owned(),
    }
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
    fetchable: &mut Fetchable,
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
            // **An uncovered tail is §12.8.1's ordinary case, and the `shall` is about where the
            // range stops.** §12.8.1's NOTE 1 makes the tail an incremental update appended after
            // signing, and its multiple-signature sentence says what "the entire PDF file" is
            // once there are several revisions: the bytes up to the `%%EOF` "terminating the
            // incremental update that adds the digital signature dictionary". §12.8.4.3 even
            // requires such a tail for a DSS. So for the dictionaries Table 255 holds to the whole
            // file (`Signature::must_cover_whole_file`), the tail is a departure only where the
            // range does not stop at an end-of-file marker (`Signature::covers_its_revision`,
            // ADR 3230). It is still not a verdict on the signature: this program has no trust
            // store and says what the file states.
            pdf_signature::signature::Coverage::Unsigned { tail } => {
                if signature.must_cover_whole_file()
                    && !signature.covers_its_revision(document.bytes())
                {
                    let held_by = if signature.timestamp {
                        "a document timestamp's".to_owned()
                    } else {
                        format!(
                            "/SubFilter {}'s",
                            signature.sub_filter.as_deref().unwrap_or("")
                        )
                    };
                    notes.push(format!(
                        "{tail} bytes at the end of this file are not covered by that signature, \
                         and its signed range does not stop at the %%EOF of a revision — Table \
                         255 requires {held_by} signed range to cover the entire file, which \
                         §12.8.1 reads as up to the %%EOF ending the update that added it, so \
                         this file breaks that requirement"
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
        // (§12.8.1). §12.8.2.2.2's and §12.8.2.3's second step is taken (ADR 1104) and
        // `modifications` is where a reader is
        // told what it came to — so what this sentence says is that the file asked for that answer
        // to count, and where it does. The condition is the entry's own and nothing is added to it
        // (trap 11); `/V` absent is the table's default 0.
        if signature.reference_is_critical() {
            notes.push(
                "that signature states /V 1, so this file requires its signature reference \
                 dictionary to be considered critical to validating it (Table 255) — what the \
                 transform method's own second step came to is said separately below"
                    .to_owned(),
            );
        }
        // **Table 256's `/DigestMethod` is the other entry of that family a reader is owed**, and
        // it names the function rather than the fact: "[a] name identifying the algorithm that
        // shall be used when computing the digest if not specified in the certificate". The digest
        // it parameterises is §12.8.2's modification analysis and not the byte range digest, and
        // this program's comparison of the two revisions digests nothing: ADR 1043 section 2
        // settles an object's identity from where both cross-reference tables place it, on the byte
        // range digest's own warrant. So a file that states one has named a function that answers
        // a question already answered — worth a sentence for the reason `/V 1` is, and for no
        // other. Two
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
                 modification analysis — and this program's comparison places objects rather than \
                 digesting them, so the name is read and said rather than acted on",
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
            // **The witnesses are counted, not named here**: `issue17069.pdf`, `issue6127.pdf`
            // and `xfa_filled_imm1344e.pdf` carry the attribute in `doc/pdf.js`, and
            // `examples/signature_algorithm_census` counts and names them, so the number is not
            // written down here or in §12.8.3.3.2's ledger row.
            //
            // **And three is a count of `doc/pdf.js` alone** (ADR 0758): the same census over
            // every document this tree holds finds the attribute on hundreds of signature values
            // in hundreds of documents, nearly all of them in the crawl. Its presence is
            // therefore ordinary rather than rare, which changes nothing this code does — the
            // note fires on the attribute and not on a population — and everything about what a
            // round may conclude from the three names above.
            if cms.has_signed_attribute(pdf_signature::cms::ADBE_REVOCATION_INFO_ARCHIVAL) {
                // **What is inside the attribute is read** (ADR 1067), because §12.8.3.3.2 prints
                // the grammar rather than
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
                        "that signature carries a timestamp of its own (§12.8.3.3.1's unsigned \
                         signature timestamp attribute, RFC 3161 Appendix A), stating genTime \
                         {}. The imprint inside it {} the digest of this signature's own bytes. \
                         Whether the authority that issued it is one to believe is the same \
                         unanswered question as above",
                        stamp.claim.stated,
                        if stamp.covers_the_signature {
                            "is"
                        } else {
                            "is not"
                        }
                    ),
                    Err(refusal) => format!(
                        "that signature states §12.8.3.3.1's signature timestamp attribute and \
                         this program will not read what is in it: {refusal}"
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
                        PadesDeparture::NoContentTypeAttribute =>
                            "§12.8.3.4.3 (a) requires a content-type signed attribute, and it \
                             states none",
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
            signature_policy(signature, &cms, asked, notes, fetchable);
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

/// The signature policy a signer committed to, said to the person reading the document.
///
/// §12.8.3.4.4 requires a signature-policy-identifier as a signed attribute and hands the rules
/// for it to ETSI EN 319 122-1 clause 5.2.9, whose text this project cites by clause and never
/// reproduces (ADR 1085). [`pdf_signature::policy`] reads what those clauses define; this is the
/// half a reader is owed, and it is three sentences rather than one. **Which** policy: the
/// identifier names one particular version of a set of rules, and it is the whole of what can be
/// said about which rules were agreed. **Whether this file carries that policy**: clause 5.2.10's
/// store may hold the document itself, and the digest beside the identifier is what binds the two
/// — decisive when it agrees, and not an alteration when it does not, because the digest's input
/// depends on the specification the policy is written under. **The notice**: clause 5.2.9.2 lets
/// a signer attach a notice meant to be shown whenever the signature is validated, and a notice
/// nobody displays is a requirement unmet rather than an attribute unread.
///
/// What is *not* here is enforcement of the policy's own constraints, which §12.8.3.4.4 also
/// requires: a constraint lives inside the policy document, in a syntax whichever specification
/// the signature itself names defines, so the block is one file at a time and the sentence below
/// names the specification that file pointed at. ADR 1219.
fn signature_policy(
    signature: &pdf_signature::signature::Signature,
    cms: &pdf_signature::cms::SignedData<'_>,
    asked: &Asked<'_>,
    notes: &mut Vec<String>,
    fetchable: &mut Fetchable,
) {
    use pdf_signature::policy::Binding;

    let policy = match signature.signature_policy(cms) {
        Ok(Some(policy)) => policy,
        // §12.8.3.4.4's basic profile: a signature under no explicit policy owes this nothing.
        Ok(None) => return,
        Err(refusal) => {
            notes.push(format!(
                "that signature states §12.8.3.4.4's signature-policy-identifier and this program \
                 will not read what is in it: {refusal}"
            ));
            return;
        }
    };
    notes.push(format!(
        "that signature was made under signature policy {}, which is the one version of a set of \
         rules its signer committed to (§12.8.3.4.4's signature-policy-identifier)",
        policy.identifier
    ));
    // Clause 5.2.9.2's URL qualifier is a server, and asking it is a host's question under the
    // reader's network level rather than a sentence (ADRs 1728, 1738). A second signature under
    // the same policy, URL and digest would be the same question asked twice.
    for copy in policy.published() {
        if !fetchable.published.contains(&copy) {
            fetchable.published.push(copy);
        }
    }
    notes.push(match policy.binding() {
        Binding::Matches { digest } => format!(
            "the copy of that policy's document this file carries is the one the signer committed \
             to: it hashes to the {} digest they signed over it",
            digest.name()
        ),
        // Reported as a difference rather than as a substitution, which is the module's own
        // reading: the octets hashed here are the document as it stands, and a specification that
        // prescribes a canonicalisation first would digest the same document to something else.
        Binding::DoesNotMatchTheStoredOctets {
            digest,
            specification,
        } => format!(
            "the copy of that policy's document this file carries does not hash to the {} digest \
             the signer signed over it. That is not by itself an altered policy — what goes into \
             the digest is decided by the specification the document is written under, which this \
             file names as {}",
            digest.name(),
            named(&specification)
        ),
        Binding::PolicyHashNotKnown => "that signature commits to the policy by name and to no \
             particular copy of its document, so there is nothing here to compare a stored copy \
             with"
            .to_owned(),
        Binding::UnderAnotherFunction { algorithm } => format!(
            "the digest that would bind that policy's document to the signature is under {algorithm}, \
             which this program does not compute, so the two were not compared"
        ),
        Binding::NoStoredDocument => "this file carries no copy of that policy's document, so what \
             its rules say is not in the file"
            .to_owned(),
    });
    // Clause 5.2.9.2's notice is addressed to whoever validates, so a reader gets it whatever the
    // rest of the policy came to. The signer may write the words out, or name an organisation and
    // the numbered statements of theirs it means, and both halves are said because a reader
    // holding that organisation's notices can look the second up.
    for notice in policy.notices() {
        let mut said = String::from(
            "that signature's policy carries a notice its signer meant to be shown whenever the \
             signature is validated (ETSI EN 319 122-1 clause 5.2.9.2)",
        );
        if let Some(text) = &notice.text {
            said.push_str(": ");
            said.push_str(text);
        }
        if let Some(organization) = &notice.organization {
            said.push_str(" — it refers to ");
            said.push_str(organization);
            if !notice.numbers.is_empty() {
                said.push_str("'s statement(s) ");
                said.push_str(
                    &notice
                        .numbers
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
        }
        notes.push(said);
    }
    policy_constraints(
        &policy,
        cms,
        asked.copies,
        PolicyAsked {
            path_at: path_instant(asked),
            material: asked.material,
            lists: asked.supplied,
        },
        notes,
        &mut fetchable.references,
    );
}

/// The instant a path to a policy's own trust points is validated at (ADR 2714): the one the
/// verdict's path is asked about where a host supplied anchors and with them a clock, and none
/// where it did not — the policy's search then falls back to a covering signature timestamp's
/// instant, or says it had none.
fn path_instant(asked: &Asked<'_>) -> Option<pdf_signature::x509::Instant> {
    (!asked.anchors.is_empty() || !asked.lists.is_empty()).then(|| asked.when.at())
}

/// What a policy's constraints are enforced under besides the signature: the instant its paths are
/// validated at, the revocation material, and the host's trusted lists.
#[derive(Clone, Copy)]
struct PolicyAsked<'a> {
    path_at: Option<pdf_signature::x509::Instant>,
    material: &'a pdf_signature::revocation::Material<'a>,
    lists: pdf_signature::trusted_list::anchors::Supplied<'a>,
}

/// §12.8.3.4.4's "shall enforce signature policy constraints", said one constraint at a time.
///
/// A copy of the policy document bound to the signature and written in any of the three
/// machine-processable syntaxes — ETSI TR 102 272's or ETSI TS 119 172-3's ASN.1, or ETSI TS
/// 119 172-2's XML — is enforced, each constraint a sentence with its verdict (ADRs 2560, 2580,
/// 2604). The copy is the one clause 5.2.10's store carries where that one is bound, and otherwise
/// one a host fetched from the URL clause 5.2.9.2's qualifier names, bound and enforced the same
/// way (ADR 2624). Anything else is said as the reason nothing was enforced; where the file
/// carries no document and nothing was fetched, the sentence names the specification the signer
/// named and is silent where it named none, since clause 5.2.9.1 leaves the syntax to the context
/// there and a reader outside that context has nothing to name.
fn policy_constraints(
    policy: &pdf_signature::policy::SignaturePolicy<'_>,
    cms: &pdf_signature::cms::SignedData<'_>,
    copies: &[FetchedCopy],
    asked: PolicyAsked<'_>,
    notes: &mut Vec<String>,
    references: &mut Vec<String>,
) {
    use pdf_signature::policy_rules::{Enforcement, NotEnforced, SignerFacts, enforce};

    // A trust condition's mandated revocation checks are answered from §12.8.4's store as well as
    // from the signer's own archival attribute (ADR 2720), and a trust point's anchors given by
    // reference to a trusted list from the host's lists (ADR 2874).
    let facts = SignerFacts::of(cms)
        .with_material(asked.material)
        .with_lists(asked.lists);
    let facts = match asked.path_at {
        Some(at) => facts.validated_at(at),
        None => facts,
    };
    let fetched = fetched_copies(policy, copies);
    let stored = enforce(policy, cms, &facts);
    if let Enforcement::Enforced {
        syntax,
        findings,
        references: stated,
    } = &stored
    {
        referenced(stated, references);
        enforced(*syntax, findings, "the copy this file carries", notes);
        for copy in fetched {
            notes.push(format!(
                "the copy of that policy's document fetched from {} was not enforced beside it: \
                 the copy this file carries is bound to the signature, and its constraints are \
                 the ones above",
                copy.published.url
            ));
        }
        return;
    }
    let mut fetched_enforced = false;
    for copy in &fetched {
        let source = format!("the copy fetched from {}", copy.published.url);
        match enforce_fetched_once(policy, copy, cms, &facts, fetched_enforced) {
            Some(Enforcement::Enforced {
                syntax,
                findings,
                references: stated,
            }) => {
                referenced(&stated, references);
                enforced(syntax, &findings, &source, notes);
                fetched_enforced = true;
            }
            Some(Enforcement::NotEnforced(why)) => notes.push(format!(
                "§12.8.3.4.4 also requires a signature handler to enforce that policy's \
                 constraints, and this program did not enforce {source}: {}",
                not_enforced(&why, &source)
            )),
            None => notes.push(format!(
                "{source} was not enforced as well: another copy of that policy's document is \
                 bound to the signature, and its constraints are the ones above"
            )),
        }
    }
    let Enforcement::NotEnforced(why) = stored else {
        return;
    };
    let why = match why {
        // A fetched copy has said what came of it, and a file that carries none has nothing more
        // to say of its own.
        NotEnforced::NoStoredDocument if !fetched.is_empty() => return,
        NotEnforced::NoStoredDocument => {
            let Some(specification) = policy.specification() else {
                return;
            };
            format!(
                "the constraints are written in {}, and this file carries no copy of them",
                named(specification)
            )
        }
        why => not_enforced(&why, "the copy this file carries"),
    };
    notes.push(format!(
        "§12.8.3.4.4 also requires a signature handler to enforce that policy's constraints, and \
         this program did not: {why}. Everything the signature itself states about the policy is \
         above"
    ));
}

/// The trusted lists an enforced copy's `TAsInTrustedList`s reference, added to the document's
/// once each (TS 119 172-2 clause 4.22.2.1 b)).
///
/// **Only a bound copy's**: a store is unsigned and a server is not the signer, so a copy neither
/// digest binds is nobody's agreement, and a location it names is not one this program would put to
/// the reader as the policy's (ADRs 2580, 2624, 2936).
fn referenced(stated: &[String], references: &mut Vec<String>) {
    for reference in stated {
        if !references.contains(reference) {
            references.push(reference.clone());
        }
    }
}

/// The fetched copies that are of this signature's policy: the ones whose published entry is one
/// [`pdf_signature::policy::SignaturePolicy::published`] lists.
fn fetched_copies<'a>(
    policy: &pdf_signature::policy::SignaturePolicy<'_>,
    copies: &'a [FetchedCopy],
) -> Vec<&'a FetchedCopy> {
    let published = policy.published();
    copies
        .iter()
        .filter(|copy| published.contains(&copy.published))
        .collect()
}

/// A fetched copy enforced, or `None` where an earlier copy already was: one bound copy is the
/// signer's policy document, and a second would only say its constraints again.
fn enforce_fetched_once(
    policy: &pdf_signature::policy::SignaturePolicy<'_>,
    copy: &FetchedCopy,
    cms: &pdf_signature::cms::SignedData<'_>,
    facts: &pdf_signature::policy_rules::SignerFacts,
    already: bool,
) -> Option<pdf_signature::policy_rules::Enforcement> {
    use pdf_signature::policy_rules::{Enforcement, enforce_fetched};
    let enforcement = enforce_fetched(policy, &copy.octets, cms, facts);
    match enforcement {
        Enforcement::Enforced { .. } if already => None,
        enforcement => Some(enforcement),
    }
}

/// The sentences of an enforced copy: the count, then each constraint with its verdict.
fn enforced(
    syntax: pdf_signature::policy_rules::Syntax,
    findings: &[pdf_signature::policy_rules::Finding],
    source: &str,
    notes: &mut Vec<String>,
) {
    let broken = findings
        .iter()
        .filter(|finding| finding.verdict == pdf_signature::policy_rules::Verdict::Broken)
        .count();
    notes.push(format!(
        "§12.8.3.4.4 requires a signature handler to enforce that policy's constraints, and \
         {source} states them in {syntax}: {} constraint(s), {broken} of them broken",
        findings.len()
    ));
    notes.extend(findings.iter().map(ToString::to_string));
}

/// Why a copy's constraints were not enforced, said of `copy` — the one the file carries or the
/// one fetched from a URL.
fn not_enforced(why: &pdf_signature::policy_rules::NotEnforced, copy: &str) -> String {
    use pdf_signature::policy::Published;
    use pdf_signature::policy_rules::NotEnforced;
    match why {
        NotEnforced::NoStoredDocument => "this file carries no copy of them".to_owned(),
        NotEnforced::NotBound(_) => {
            format!("{copy} is not bound to the signature, so its rules are nobody's agreement")
        }
        NotEnforced::FetchedCopyNotBound(Published::DoesNotMatchTheFetchedOctets {
            digest,
            ..
        }) => format!(
            "{copy} does not hash to the {} digest the signer signed over it, and states no digest \
             of its own rules the signer signed, so it is not bound to the signature and its rules \
             are nobody's agreement",
            digest.name()
        ),
        NotEnforced::FetchedCopyNotBound(Published::PolicyHashNotKnown) => format!(
            "the signer committed to the policy by name and to no particular copy of its document \
             (ETSI EN 319 122-1 clause 5.2.9.1's all-zero digest), so {copy} is not bound to the \
             signature"
        ),
        NotEnforced::FetchedCopyNotBound(Published::UnderAnotherFunction { algorithm }) => {
            format!(
                "the digest that would bind {copy} to the signature is under {algorithm}, which \
                 this program does not compute"
            )
        }
        NotEnforced::FetchedCopyNotBound(Published::Matches { .. }) => {
            format!("{copy} is bound to the signature by its octets")
        }
        NotEnforced::Unreadable(refusal) => refusal.to_string(),
        NotEnforced::CanonicalDigestNotComputed { canonicalization } => format!(
            "the signer committed to the digest {copy} states over its policy components \
             canonicalised by {canonicalization}, a method this program does not compute, so it \
             cannot re-compute that digest"
        ),
        NotEnforced::CanonicalDigestDiffers { canonicalization } => format!(
            "the signer committed to the digest {copy} states over its policy components, and \
             those components canonicalised by {canonicalization} do not digest to it, so {copy} \
             is not the document the signer committed to"
        ),
        NotEnforced::IdentifierDiffers { document } => {
            format!("{copy} is the document of policy {document}, not of the one the signer named")
        }
    }
}

/// The findings of the bound policy a signature was made under, or none where nothing was enforced.
///
/// What [`policy_constraints`] says as sentences, handed to the verdict as well, since
/// §12.8.3.4.4's enforcement is a validator's and a broken constraint withholds the word (ADR 2560)
/// — from the stored copy where it is bound, and otherwise from the first fetched copy that is
/// (ADR 2624).
fn policy_findings(
    signature: &pdf_signature::signature::Signature,
    copies: &[FetchedCopy],
    at: pdf_signature::x509::Instant,
    material: &pdf_signature::revocation::Material<'_>,
    lists: pdf_signature::trusted_list::anchors::Supplied<'_>,
) -> Vec<pdf_signature::policy_rules::Finding> {
    use pdf_signature::policy_rules::{Enforcement, SignerFacts, enforce, enforce_fetched};

    let Ok(cms) = signature.signed_data() else {
        return Vec::new();
    };
    let Ok(Some(policy)) = signature.signature_policy(&cms) else {
        return Vec::new();
    };
    let facts = SignerFacts::of(&cms)
        .validated_at(at)
        .with_material(material)
        .with_lists(lists);
    if let Enforcement::Enforced { findings, .. } = enforce(&policy, &cms, &facts) {
        return findings;
    }
    fetched_copies(&policy, copies)
        .into_iter()
        .find_map(
            |copy| match enforce_fetched(&policy, &copy.octets, &cms, &facts) {
                Enforcement::Enforced { findings, .. } => Some(findings),
                Enforcement::NotEnforced(_) => None,
            },
        )
        .unwrap_or_default()
}

/// How a signature named the specification its policy document's syntax is defined by.
///
/// ETSI EN 319 122-1 clause 5.2.9.2 lets a signer name it either way and neither is a name a
/// person would recognise, so the sentence says which kind it is rather than printing a bare
/// string.
fn named(specification: &pdf_signature::policy::Specification) -> String {
    use pdf_signature::policy::Specification;
    match specification {
        Specification::ObjectIdentifier(oid) => format!("the specification identified by {oid}"),
        Specification::Uri(uri) => format!("the specification at {uri}"),
    }
}

/// §12.8.6's permissions dictionary, said before a person starts typing rather than after.
fn permissions_stated(
    permissions: &pdf_signature::signature::Permissions,
    notes: &mut Vec<String>,
) {
    // §12.8.2.2.1's parenthesis is a `shall` addressed to a processor that modifies: "(These
    // changes to the document shall also be prevented if the signature dictionary is referred
    // from the DocMDP entry in the permissions dictionary.)" This program modifies, so it obeys
    // it — an `Edit` that a level of `/P` does not
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
/// that [`verdicts`] recomputes (ADR 0215). This is the second — §12.8.2.2.2's
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
    if asked.anchors.is_empty() && asked.lists.is_empty() {
        return;
    }
    let verdict = listed_or_local(signature, integrity, &authenticity, asked);
    let verdict = verdict.under_policy(&policy_findings(
        signature,
        asked.copies,
        asked.when.at(),
        asked.material,
        asked.supplied,
    ));
    // **Which instant the path was asked about, and on whose word** — said whichever answer it is,
    // because "valid" without it reads as *valid now* and that is the one thing this program never
    // establishes. §12.8.4.2's own reason for a document security store is that a signature
    // outlives its certificate, so the instant is what decides whether an expired certificate or a
    // later revocation stops the word (ETSI EN 319 102-1 clause 5.5.4).
    let asked_at = format!(
        ", as of {} seconds after the epoch, which {}",
        asked.when.at().unix_seconds(),
        match asked.when.proof() {
            Proof::ADocumentTimestamp => "a document timestamp over it established (§12.8.4.2)",
            Proof::ThisSignaturesTimestampAttribute =>
                "this signature's own timestamp attribute established (§12.8.3.4.8)",
            // Every other answer, this build's one and any a later one adds: nothing proved the
            // signature existed earlier, so the instant is simply the clock the host supplied.
            _ => "is the clock this reader was given, nothing having proven an earlier moment",
        }
    );
    let mut report = Vec::new();
    notes.push(match &verdict {
        pdf_signature::verdict::Verdict::Valid(valid) => {
            // ETSI TS 119 172-4 clause 4.4.2's level, which only a listed path has (ADR 2866).
            let level = pdf_signature::verdict::level::determine(
                valid,
                &pdf_signature::verdict::level::Asked {
                    signature,
                    integrity: &integrity,
                    authenticity: &authenticity,
                    lists: asked.lists,
                    material: asked.material,
                    timestamps: asked.timestamps,
                },
            );
            if level.is_some() {
                report =
                    applicability_sentences(&pdf_signature::verdict::applicability::Report::of(
                        &pdf_signature::verdict::applicability::Asked {
                            signature,
                            authenticity: &authenticity,
                            when: valid.when(),
                            chain: asked.chain,
                            index: asked.index,
                            material: asked.material,
                            anchors: asked.anchors,
                            now: asked.now,
                            signatures: asked.signatures,
                            lists: asked.lists,
                            path: valid.keys(),
                            responders: valid.responders(),
                        },
                    ));
            }
            valid_sentence(valid, level.as_ref(), &asked_at)
        }
        pdf_signature::verdict::Verdict::Reserved(reservation) => format!(
            "that signature is not called valid here{asked_at}, and this is the first thing that \
             stopped it: {reservation}"
        ),
    });
    notes.extend(report);
    // ETSI TS 119 172-4 REQ-4.2-03 e), wherever lists were asked: a suite this program cannot
    // compute on leaves the signature indeterminate, and is named, rather than invalidating it.
    if !asked.lists.is_empty()
        && let pdf_signature::verdict::applicability::Cryptography::NotSupported { suite } =
            pdf_signature::verdict::applicability::Cryptography::of(&authenticity)
    {
        notes.push(unsupported_suite_sentence(&suite));
    }
}

/// The verdict on one signature, through a trusted list's service where one issued the signer and
/// through the reader's own anchors otherwise.
///
/// **A listed path is asked first**, because it is the stronger statement and a reader's own anchor
/// may well be the same authority: ETSI TS 119 172-4 REQ-4.2-03 a) makes the services TS 119 615
/// clause 4.3 returns the anchors for an EU qualified signature, and a path to one of them is
/// reported as that. Where none reaches the word, the reader's own anchors are asked, and a path to
/// one of those is *locally valid* (`doc/questions/A800` item 3, ADR 2834). Where neither reaches
/// it, the reservation said is the listed one if a list named a service for this signer — that is
/// the answer the reader most needs to hear — and the local one otherwise.
fn listed_or_local(
    signature: &pdf_signature::signature::Signature,
    integrity: pdf_signature::signature::Integrity,
    authenticity: &pdf_signature::signature::Authenticity,
    asked: &Asked<'_>,
) -> pdf_signature::verdict::Verdict {
    use pdf_signature::verdict::{Anchored, Verdict};
    let mut listed = None;
    // ETSI TS 119 172-4 REQ-4.4.2-01: clause 4.4 is asked at the best signature time.
    let services =
        pdf_signature::trusted_list::listed_anchors(asked.lists, signature, asked.when.at());
    for (listing, identities) in services {
        let parsed: Vec<_> = identities
            .iter()
            .filter_map(|der| pdf_signature::x509::parse(der).ok())
            .collect();
        let anchors = pdf_signature::trust::TrustAnchors::of(&parsed);
        let trust = trust_as_of(signature, &anchors, asked);
        let verdict = match Anchored::listed(&trust, listing) {
            Some(anchored) => Verdict::reached(
                &integrity,
                authenticity,
                &anchored,
                asked.acceptance,
                asked.timestamps,
                asked.when,
            ),
            None => Verdict::of(
                &integrity,
                authenticity,
                &trust,
                asked.acceptance,
                asked.timestamps,
                asked.when,
            ),
        };
        if verdict.valid().is_some() {
            return verdict;
        }
        listed.get_or_insert(verdict);
    }
    let trust = trust_as_of(signature, asked.anchors, asked);
    let local = Verdict::of(
        &integrity,
        authenticity,
        &trust,
        asked.acceptance,
        asked.timestamps,
        asked.when,
    );
    match listed {
        Some(listed) if local.valid().is_none() => listed,
        _ => local,
    }
}

/// The signer's path, checked at the best signature time under the clause that time falls in.
///
/// §12.8.3.4.7 is the check at the reader's clock and §12.8.3.4.8 the check at the instant the
/// signature's own timestamp attribute established, and each states its own window on the
/// revocation material (`pdf_signature::revocation::Basis`). A document timestamp's instant is
/// §12.8.5's, which states neither window, and keeps the one both share (ADR 2968).
fn trust_as_of(
    signature: &pdf_signature::signature::Signature,
    anchors: &pdf_signature::trust::TrustAnchors<'_>,
    asked: &Asked<'_>,
) -> pdf_signature::trust::Trust {
    use pdf_signature::revocation::{Basis, Issuance};
    use pdf_signature::verdict::Proof;
    let basis = match asked.when.proof() {
        Proof::NothingButTheReadersClock => Basis::CurrentTime,
        Proof::ThisSignaturesTimestampAttribute => Basis::PastTime,
        // §12.8.5's document timestamp, and any proof a later build names.
        _ => Basis::Unstated,
    };
    signature.trust_under(
        anchors,
        asked.material,
        asked.when.at(),
        pdf_signature::trust::Purpose::Unstated,
        &pdf_signature::trust::PathInputs {
            revocation_issuance: Issuance {
                basis,
                ..Issuance::default()
            },
            ..pdf_signature::trust::PathInputs::default()
        },
    )
}

/// What the signer's certificate states under ETSI EN 319 412-5, named by the statements' own
/// ASN.1 names and never as a level: those are the issuer's claims, which TS 119 615 clause 4.4
/// weighs against the list ([`qualification_sentence`] says what it found; ADR 2835, ADR 2853).
fn statements_sentence(stated: &pdf_signature::qc_statement::Stated) -> String {
    use pdf_signature::qc_statement::{QcType, Stated};
    match stated {
        Stated::Nothing => {
            "The signer's certificate makes none of the statements of ETSI EN 319 412-5".to_owned()
        }
        Stated::Unreadable(error) => format!(
            "The signer's certificate carries ETSI EN 319 412-5 statements that could not be read \
             ({error})"
        ),
        Stated::Read(read) => {
            let mut named = Vec::new();
            if read.compliance {
                named.push("QcCompliance (clause 4.2.1)".to_owned());
            }
            if read.sscd {
                named.push("QcSSCD (clause 4.2.2)".to_owned());
            }
            if let Some(types) = &read.types {
                let purposes: Vec<String> = types
                    .iter()
                    .map(|purpose| match purpose {
                        QcType::Esign => "esign".to_owned(),
                        QcType::Eseal => "eseal".to_owned(),
                        QcType::Web => "web".to_owned(),
                        QcType::Other(identifier) => identifier.clone(),
                        // A purpose a later build names is said by its absence of a name.
                        _ => "another purpose".to_owned(),
                    })
                    .collect();
                named.push(format!("QcType {} (clause 4.2.3)", purposes.join(", ")));
            }
            if let Some(countries) = &read.legislation {
                named.push(format!(
                    "QcCClegislation {} (clause 4.2.4)",
                    countries.join(", ")
                ));
            }
            for other in &read.others {
                named.push(format!("statement {other}"));
            }
            if named.is_empty() {
                "The signer's certificate carries an empty set of ETSI EN 319 412-5 statements"
                    .to_owned()
            } else {
                format!(
                    "The signer's certificate states, under ETSI EN 319 412-5, {} — its issuer's \
                     claims, which ETSI TS 119 615 clause 4.4 weighs against the list",
                    named.join("; ")
                )
            }
        }
        // Any standing of the statements a later build adds is said in the words that claim least.
        _ => "What the signer's certificate states under ETSI EN 319 412-5 is not said by this \
              build"
            .to_owned(),
    }
}

/// What ETSI TS 119 615 clauses 4.4 and 4.5 found for the signer's certificate, in the clauses'
/// own indications and named as being about the certificate: the signature's level is ETSI TS 119
/// 172-4 clause 4.4.2's join of these with the validation, which [`level_sentence`] says (ADRs
/// 2864, 2865, 2866).
fn qualification_sentence(
    qualification: &pdf_signature::trusted_list::determination::Qualification,
) -> String {
    use pdf_signature::trusted_list::determination::{QcStatus, Qualification};
    match qualification {
        Qualification::Determined {
            territory,
            sequence_number,
            determination,
            qscd,
        } => {
            let results: Vec<&str> = determination
                .results
                .iter()
                .map(|indication| indication.name())
                .collect();
            let status = match determination.status {
                QcStatus::Passed => "PROCESS_PASSED",
                QcStatus::PassedWithWarning => "PROCESS_PASSED_WITH_WARNING",
                QcStatus::Failed => "PROCESS_FAILED",
            };
            let found = if results.is_empty() {
                "no indication".to_owned()
            } else {
                results.join(", ")
            };
            let mut sentence = format!(
                "At the best signature time, ETSI TS 119 615 clause 4.4 over the trusted list of \
                 {territory} (sequence number {sequence_number}) finds the signer's certificate \
                 {found} ({status}"
            );
            if !determination.sub_status.is_empty() {
                let said: Vec<String> = determination
                    .sub_status
                    .iter()
                    .map(pdf_signature::trusted_list::determination::SubStatus::name)
                    .collect();
                sentence.push_str(": ");
                sentence.push_str(&said.join("; "));
            }
            sentence.push(')');
            // Clause 4.5's answer about the key, in its own indications (ADR 2865).
            let qscd_status = match qscd.status {
                QcStatus::Passed => "PROCESS_PASSED",
                QcStatus::PassedWithWarning => "PROCESS_PASSED_WITH_WARNING",
                QcStatus::Failed => "PROCESS_FAILED",
            };
            sentence.push_str(", and clause 4.5 finds its key ");
            sentence.push_str(
                qscd.result
                    .map_or("with no indication", |result| result.name()),
            );
            sentence.push_str(" (");
            sentence.push_str(qscd_status);
            if !qscd.sub_status.is_empty() {
                let said: Vec<String> = qscd
                    .sub_status
                    .iter()
                    .map(pdf_signature::trusted_list::determination::qscd::QscdSubStatus::name)
                    .collect();
                sentence.push_str(": ");
                sentence.push_str(&said.join("; "));
            }
            sentence.push_str(") — indications about the certificate, not the signature's level");
            sentence
        }
        Qualification::NoListForTerritory(territory) => format!(
            "No trusted list of {territory}, the territory the certificate's issuer names, was \
             supplied, so ETSI TS 119 615 clause 4.4 had no list to read"
        ),
        Qualification::IssuerStatesNoCountry => "The certificate's issuer names no country, so \
             ETSI TS 119 615 clause 4.4 names no list to read"
            .to_owned(),
        // The certificate not reading, and any answer a later build adds, is said in the words
        // that claim least.
        _ => "ETSI TS 119 615 clause 4.4 was not run for the signer's certificate".to_owned(),
    }
}

/// The sentence for a [`pdf_signature::verdict::Valid`], which says whose anchor made it sayable.
///
/// **Locally valid is said in those words** (`doc/questions/A800` item 3): a company's own
/// certificate is neither invalid nor the same thing as a service a Member State lists, and the
/// reader is owed which of the two it is. A listed service is named with its list, and how far the
/// list itself was authenticated is said: under the European Commission's list of trusted lists
/// (TS 119 615 clauses 4.1 and 4.2, ADR 2842), or on the word of whoever supplied it. The eIDAS
/// level is said only from TS 119 172-4 clause 4.4.2's own output, `level` ([`level_sentence`]),
/// so that a listing is never read as a qualified signature (ADRs 2835, 2866).
fn valid_sentence(
    valid: &pdf_signature::verdict::Valid,
    level: Option<&pdf_signature::verdict::Level>,
    asked_at: &str,
) -> String {
    use pdf_signature::trusted_list::Authentication;
    use pdf_signature::verdict::Standing;
    let reached = format!(
        "the document has not changed since it was signed, the signature verifies under the \
         signer's key, and a certification path {} certificate(s) long",
        valid.path_length()
    );
    match valid.standing() {
        Standing::Listed(listing) => {
            let believed = match listing.authentication {
                Authentication::Authenticated {
                    lotl_sequence_number,
                } => format!(
                    "That list's own signature was authenticated under the European Commission's \
                     list of trusted lists (sequence number {lotl_sequence_number}), whose own \
                     signature was authenticated against the Official Journal's certificates \
                     (ETSI TS 119 615 clauses 4.1 and 4.2)"
                ),
                // Every other footing, and any a later build adds, is said in the words that
                // claim least.
                _ => "That list's own signature was not checked — it is believed because it came \
                      from the trusted lists you supplied"
                    .to_owned(),
            };
            format!(
                "that signature is valid under the trusted list of {} (sequence number {}): \
                 {reached} reaches the service \"{}\" of {}, which the list publishes as {} with \
                 the status {} when the signer's certificate was issued (ETSI TS 119 615 clause \
                 4.3){asked_at}. {believed}. {}. {}. {}",
                listing.territory,
                listing.sequence_number,
                listing.service,
                listing.provider,
                listing.service_type,
                listing.status_at_issue.as_deref().unwrap_or("none yet"),
                statements_sentence(&listing.statements),
                qualification_sentence(&listing.qualification),
                level_sentence(level)
            )
        }
        // Every other standing — the reader's own anchors, and any a later build adds — is said
        // in the local words, which never claim more than the reader's own setting granted.
        _ => format!(
            "that signature is locally valid: {reached} reaches an authority your own trust \
             setting names, and none a trusted list publishes (RFC 5280 section 6.1){asked_at}. It \
             is valid to whoever trusts that authority — a company's own, for instance — and it is \
             your anchors that make the word sayable"
        ),
    }
}

/// ETSI TS 119 172-4 clause 4.4.2's answer for a listed valid signature, with the requirement that
/// decided it and Annex A's type where the outputs support one — the only place a report says
/// "qualified" about a signature (ADRs 2835, 2866).
fn level_sentence(level: Option<&pdf_signature::verdict::Level>) -> String {
    use pdf_signature::verdict::level::{Answer, Purpose, Requirement, Type};
    let Some(level) = level else {
        // A listed path whose service the level's run did not find again, or any answer a later
        // build adds, is said in the words that claim least.
        return "Whether this is a qualified electronic signature or seal was not determined"
            .to_owned();
    };
    let of = |purpose| match purpose {
        Purpose::Signature => "signatures",
        Purpose::Seal => "seals",
    };
    let requirement = level.decided_by.name();
    let mut sentence = match (level.answer, level.decided_by, level.certificate) {
        (Answer::Qualified(Purpose::Signature), ..) => format!(
            "ETSI TS 119 172-4 clause 4.4.2 determines this signature technically suitable to \
             implement an EU qualified electronic signature (QES, {requirement})"
        ),
        (Answer::Qualified(Purpose::Seal), ..) => format!(
            "ETSI TS 119 172-4 clause 4.4.2 determines this signature technically suitable to \
             implement an EU qualified electronic seal ({requirement})"
        ),
        (Answer::NotQualified, _, certificate) => format!(
            "The signer's certificate is an EU qualified certificate for electronic {} and its key \
             is in a qualified creation device, but the validation under ETSI TS 119 172-4 \
             REQ-4.2-03's constraints (revocation status checked, from information issued after \
             the best signature time) did not pass, so this is not determined as an EU qualified \
             electronic signature or seal ({requirement})",
            certificate.map_or("signatures or seals", of)
        ),
        (_, Requirement::Device, Some(purpose)) => format!(
            "The signer's certificate is an EU qualified certificate for electronic {} \
             (REQ-4.4.2-02), but its key is not determined to be in a qualified creation device, \
             so ETSI TS 119 172-4 determines the signature indeterminate — neither an EU qualified \
             electronic signature nor seal ({requirement})",
            of(purpose)
        ),
        // REQ-4.4.2-02 b), and any branch a later build adds.
        _ => format!(
            "The signer's certificate is not determined an EU qualified certificate for electronic \
             signatures or seals at the best signature time, so ETSI TS 119 172-4 determines the \
             signature indeterminate — neither an EU qualified electronic signature nor seal \
             ({requirement})"
        ),
    };
    if let Some(kind) = level.annex_a() {
        let named = match kind {
            Type::QualifiedSignature => "an EU qualified electronic signature",
            Type::QualifiedSeal => "an EU qualified electronic seal",
            Type::AdvancedSignatureOnQualifiedCertificate => {
                "an advanced electronic signature supported by an EU qualified certificate \
                 (AdES/QC)"
            }
            Type::AdvancedSealOnQualifiedCertificate => {
                "an advanced electronic seal supported by an EU qualified certificate"
            }
            Type::Advanced => "an advanced electronic signature or seal (AdES)",
        };
        sentence.push_str("; its type is ");
        sentence.push_str(named);
        if let Some(identifier) = kind.identifier() {
            sentence.push_str(" (Annex A, ");
            sentence.push_str(identifier);
            sentence.push(')');
        } else {
            sentence.push_str(" (Annex A)");
        }
    }
    sentence
}

/// ETSI TS 119 172-4 REQ-4.5-01's report beside a listed signature's level: a), b) with c), d), f)
/// and h), each a sentence naming its item (ADR 2872).
///
/// a)'s scope is the clause's own heading and sentence, which the owner had carried as printed
/// (`doc/questions/A855`, ADR 2897); h)'s potential security issues are the assessment against
/// ETSI TS 119 312's tables, each naming the table or clause that failed (ADR 2896).
fn applicability_sentences(report: &pdf_signature::verdict::applicability::Report) -> Vec<String> {
    use pdf_signature::verdict::applicability::Cryptography;
    let mut sentences = vec![format!(
        "The level above comes from the check ETSI TS 119 172-4 REQ-4.5-01 a) names, in its \
         words: \"{}\" — \"{}\"",
        report.scope.heading(),
        report.scope.text()
    )];
    sentences.push(signer_sentence(&report.signer));
    sentences.push(times_sentence(&report.times));
    sentences.push(attributes_sentence(&report.attributes));
    sentences.extend(
        report
            .attributes
            .iter()
            .flat_map(|attribute| &attribute.assertions)
            .enumerate()
            .map(|(index, assertion)| assertion_sentence(index.saturating_add(1), assertion)),
    );
    if let Cryptography::Suite {
        digest,
        family,
        key_bits,
    } = &report.cryptography
    {
        let mut sentence = format!(
            "It was made with {} under {} with a {key_bits}-bit key (REQ-4.5-01 h))",
            digest.name(),
            family.name()
        );
        if let Some(assessment) = &report.security {
            sentence.push_str(". ");
            sentence.push_str(&security_sentence(assessment));
        }
        sentences.push(sentence);
    }
    sentences
}

/// REQ-4.5-01 h)'s potential security issues, against ETSI TS 119 312, each naming the table or
/// clause it failed — which the item asks to be indicated clearly.
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive match, one sentence per issue, so that an issue added to TS 119 312's \
              assessment cannot reach the reader unworded"
)]
fn security_sentence(
    assessment: &pdf_signature::verdict::applicability::ts_119_312::Assessment,
) -> String {
    use pdf_signature::verdict::applicability::ts_119_312::{
        Class, Issue, KeyRole, Moment, Parameter, Rule, SOURCE,
    };
    let whose = |role: KeyRole| match role {
        KeyRole::Signer => "signer".to_owned(),
        KeyRole::CertificationAuthority { position } => {
            format!("certification authority at position {position} of the path")
        }
        KeyRole::OcspResponder { about } => {
            format!("OCSP responder about the certificate at position {about} of the path")
        }
        KeyRole::TimeStampingUnit => "time-stamping unit".to_owned(),
        KeyRole::TimeStampingAuthority { position } => {
            format!("certification authority at position {position} of a time-stamping unit's path")
        }
        KeyRole::TimeStampingResponder { about } => format!(
            "OCSP responder about the certificate at position {about} of a time-stamping unit's \
             path"
        ),
    };
    let what = |rule: Rule| match rule {
        Rule::HashFunctions => "digest",
        Rule::SignatureAlgorithms => "signature algorithm",
        Rule::Curves => "curve",
        Rule::SignatureSuites | Rule::EqualLengths => "combination of digest and algorithm",
        Rule::RsaKeySizes => "modulus's size",
        Rule::DsaKeySizes => "key's sizes",
        Rule::TimeStampImprints => "time-stamp token's imprint hash",
        Rule::SignerKeyLifetime => "signer's key",
    };
    let phase_out = |class: Class| match class {
        Class::Recommended => String::new(),
        Class::Legacy {
            year,
            extendable: true,
        } => format!("phased out on 31 December {year} unless a later release extends it"),
        Class::Legacy {
            year,
            extendable: false,
        } => format!("phased out no later than 31 December {year}"),
    };
    if assessment.issues.is_empty() {
        return format!(
            "Against {SOURCE}, every part of that suite is recommended and it raises no potential \
             security issue"
        );
    }
    let issues: Vec<String> = assessment
        .issues
        .iter()
        .map(|issue| match *issue {
            Issue::NotListed(rule) => format!(
                "{} lists no row for its {}, so the document does not admit it",
                rule.citation(),
                what(rule)
            ),
            Issue::Legacy { rule, entry, class } => format!(
                "{} lists its {} {entry} as legacy, {}",
                rule.citation(),
                what(rule),
                phase_out(class)
            ),
            Issue::PhasedOut {
                rule,
                entry,
                class,
                by,
            } => format!(
                "{} lists its {} {entry} as legacy, {}, which had passed by {}",
                rule.citation(),
                what(rule),
                phase_out(class),
                match by {
                    Moment::BestSignatureTime => "the best signature time",
                    Moment::Validation => "this validation, though not by the best signature time",
                }
            ),
            Issue::UnequalLengths {
                hash_bits,
                curve_bits,
            } => format!(
                "{} recommends an elliptic-curve suite only where the digest's output is as long \
                 as the curve's key, and here it is {hash_bits} bits against {curve_bits}",
                Rule::EqualLengths.citation()
            ),
            Issue::SubgroupNotMeasured => format!(
                "{} was not asked, because the DSA key's q could not be measured",
                Rule::DsaKeySizes.citation()
            ),
            Issue::KeyOutlived {
                rule,
                entry,
                class,
                not_after,
            } => format!(
                "{} asks the signer's key to remain suitable until its certificate's notAfter, {} \
                 seconds after the epoch, and {} lists its {} {entry} as legacy, {}, which is \
                 before it",
                Rule::SignerKeyLifetime.citation(),
                not_after.unix_seconds(),
                rule.citation(),
                what(rule),
                phase_out(class)
            ),
            Issue::ValidityNotRead => format!(
                "{} was not asked, because the signer's certificate's validity did not read",
                Rule::SignerKeyLifetime.citation()
            ),
            Issue::OtherKeyOutlived {
                key,
                rule,
                entry,
                class,
                until,
            } => format!(
                "clause 9.5 asks the key of the {} to resist until {} seconds after the epoch, the \
                 latest notAfter of its certificate and of those relying on it, and {} lists its \
                 {} {entry} as legacy, {}, which is before it",
                whose(key),
                until.unix_seconds(),
                rule.citation(),
                what(rule),
                phase_out(class)
            ),
            Issue::OtherKeyNotListed { key, rule } => format!(
                "{} lists no row for the {} of the {}, so the document does not admit it and clause \
                 9.5 could not ask how long it resists",
                rule.citation(),
                what(rule),
                whose(key)
            ),
            Issue::OtherKeyValidityNotRead { key } => format!(
                "clause 9.5 was not asked of the key of the {}, because its certificate's validity did \
                 not read",
                whose(key)
            ),
            Issue::WeakerThanCertified {
                certifier,
                certified,
                parameter,
                certifier_bits,
                certified_bits,
            } => format!(
                "clause 9.5 requires the key of the {} to be at least as strong as the key of \
                 the {} it certifies, and its {} is {certifier_bits} bits against {certified_bits}",
                whose(certifier),
                whose(certified),
                match parameter {
                    Parameter::Modulus => "modulus",
                    Parameter::Prime => "prime p",
                    Parameter::Subgroup => "subgroup order q",
                    Parameter::CurveKeySize => "curve's key size",
                }
            ),
            Issue::NotCompared {
                certifier,
                certified,
            } => format!(
                "clause 9.5 requires the key of the {} to be at least as strong as the key of \
                 the {} it certifies, and the two are of families the document gives no common measure \
                 for, so they were not compared",
                whose(certifier),
                whose(certified)
            ),
        })
        .collect();
    format!(
        "Against {SOURCE}, its potential security issues: {}",
        issues.join("; ")
    )
}

/// REQ-4.5-01 b) and c): the signer as its certificate represents it, and whether it is a
/// pseudonym.
fn signer_sentence(
    signer: &Result<
        pdf_signature::verdict::applicability::Signer,
        pdf_signature::trust::names::Unreadable,
    >,
) -> String {
    use pdf_signature::trust::names::{Name, form_word};
    let signer = match signer {
        Ok(signer) => signer,
        Err(why) => {
            return format!(
                "The signer's certificate's names could not be read for the report ({why:?}), so \
                 who it represents is not said here (REQ-4.5-01 b))"
            );
        }
    };
    let subject = signer
        .subject
        .iter()
        .map(|attribute| {
            format!(
                "{}={}",
                attribute_short_name(&attribute.kind),
                attribute
                    .text
                    .as_deref()
                    .unwrap_or("(a value that is not text)")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let alternative = if signer.alternative_names.is_empty() {
        "no subject alternative name".to_owned()
    } else {
        let names = signer
            .alternative_names
            .iter()
            .map(|name| match name {
                Name::Rfc822(text) | Name::Dns(text) | Name::Uri(text) => {
                    format!("{} {text}", form_word(name.form()))
                }
                // A directory name, an address, or a form this program does not decode is named
                // by its form alone.
                _ => form_word(name.form()).to_owned(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("the subject alternative names {names}")
    };
    let pseudonym = match &signer.pseudonym {
        Some(pseudonym) => format!(
            "The signer used a pseudonym, \"{pseudonym}\": the certificate's subject states it in \
             place of a name (REQ-4.5-01 c))"
        ),
        None => "The certificate states no pseudonym (REQ-4.5-01 c))".to_owned(),
    };
    format!(
        "The signer, as the signing certificate represents it: the subject {subject}, and \
         {alternative} (REQ-4.5-01 b)). {pseudonym}"
    )
}

/// The short names of RFC 4514 section 3 and of X.520 for the attributes a qualified certificate's
/// subject carries (ETSI EN 319 412-2 clause 4.2.4, EN 319 412-3 clause 4.2.1), and the dotted
/// identifier for any other.
fn attribute_short_name(kind: &str) -> &str {
    match kind {
        "2.5.4.3" => "CN",
        "2.5.4.4" => "surname",
        "2.5.4.5" => "serialNumber",
        "2.5.4.6" => "C",
        "2.5.4.7" => "L",
        "2.5.4.8" => "ST",
        "2.5.4.10" => "O",
        "2.5.4.11" => "OU",
        "2.5.4.12" => "title",
        "2.5.4.42" => "givenName",
        "2.5.4.65" => "pseudonym",
        "2.5.4.97" => "organizationIdentifier",
        "1.2.840.113549.1.9.1" => "emailAddress",
        other => other,
    }
}

/// REQ-4.5-01 d) iii) 3): whether a token is an EU qualified time stamp, as ETSI TS 119 615 clause
/// 4.7 answered — its `QTST-Results`, `QTST-Status` and `QTST-Sub-Status` in the clause's own
/// spelling — or why it was not asked.
fn eu_qualified_words(eu_qualified: &pdf_signature::verdict::applicability::EuQualified) -> String {
    use pdf_signature::trusted_list::time_stamp::Answer;
    use pdf_signature::verdict::applicability::EuQualified;
    match eu_qualified {
        EuQualified::NoListSupplied => {
            "whether it is an EU qualified time stamp was not determined: no trusted list was \
             given (ETSI TS 119 615 clause 4.7)"
                .to_owned()
        }
        EuQualified::TokenNotEstablished => {
            "whether it is an EU qualified time stamp was not determined, because ETSI TS 119 615 \
             clause 4.7 answers only for a token that is valid and this one's time was not \
             established"
                .to_owned()
        }
        EuQualified::NoCertificate => {
            "whether it is an EU qualified time stamp was not determined: the token names no \
             certificate this reader can read (ETSI TS 119 615 clause 4.6's CERT)"
                .to_owned()
        }
        EuQualified::Determined(Answer::NoListForTerritory(territory)) => format!(
            "whether it is an EU qualified time stamp was not determined: its authority's \
             certificate names {territory}, and no trusted list of {territory} was given (ETSI TS \
             119 615 clause 4.7)"
        ),
        EuQualified::Determined(Answer::Determined(outputs)) => {
            let sub: Vec<String> = outputs
                .sub_status
                .iter()
                .map(pdf_signature::trusted_list::time_stamp::SubStatus::name)
                .collect();
            format!(
                "ETSI TS 119 615 clause 4.7 answers {} with {}{}{}",
                outputs.results.name(),
                match outputs.status {
                    pdf_signature::trusted_list::Status::Passed => "PROCESS_PASSED",
                    pdf_signature::trusted_list::Status::Failed => "PROCESS_FAILED",
                },
                if sub.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", sub.join(", "))
                },
                outputs
                    .country
                    .as_deref()
                    .map_or_else(String::new, |country| format!(
                        " by the trusted list of {country}"
                    )),
            )
        }
        _ => "whether it is an EU qualified time stamp was not determined".to_owned(),
    }
}

/// REQ-4.5-01 d) i): the time-stamping services the host's lists publish for a token's authority,
/// or their absence, which the item asks the report to express; nothing where no list was given.
fn listed_words(
    eu_qualified: &pdf_signature::verdict::applicability::EuQualified,
    listed: &[pdf_signature::trusted_list::time_stamp::ListedAuthority],
) -> String {
    if *eu_qualified == pdf_signature::verdict::applicability::EuQualified::NoListSupplied {
        return "whether its authority is in an EU trusted list was not asked: no list was given"
            .to_owned();
    }
    if listed.is_empty() {
        return "its authority is absent from every trusted list given as a time-stamping service \
                (of type TSA/QTST or TSA, REQ-4.5-01 d) i), ETSI TS 119 615 clause 4.3)"
            .to_owned();
    }
    let services: Vec<String> = listed
        .iter()
        .map(|service| {
            format!(
                "\"{}\" of \"{}\" in the list of {}, of type {}{}",
                service.service,
                service.provider,
                service.territory,
                service.service_type,
                service.status_at_generation.as_deref().map_or_else(
                    || ", with no status at the token's generation time".to_owned(),
                    |status| format!(", {status} at the token's generation time")
                ),
            )
        })
        .collect();
    format!(
        "its authority is listed as the time-stamping service {} (REQ-4.5-01 d) i), ETSI TS 119 \
         615 clause 4.3)",
        services.join("; ")
    )
}

/// REQ-4.5-01 d): each time that bears on the signature with its evidential relevance and
/// assurance, in the clause's order.
fn times_sentence(times: &[pdf_signature::verdict::applicability::Timing]) -> String {
    use pdf_signature::verdict::applicability::{Assurance, ClaimedIn, TimeKind};
    let said = times
        .iter()
        .map(|time| {
            let at = time.at.unix_seconds();
            let what = match time.kind {
                TimeKind::ClaimedSigningTime(ClaimedIn::SigningTimeAttribute) => {
                    format!("the claimed signing time {at} in its signingTime attribute")
                }
                TimeKind::ClaimedSigningTime(ClaimedIn::Dictionary) => {
                    format!("the claimed signing time {at} in its dictionary's /M")
                }
                TimeKind::DocumentTimestamp => format!("a document timestamp's time {at}"),
                TimeKind::SignatureTimestamp => format!("its signature timestamp's time {at}"),
                TimeKind::Revocation { suspended: true } => {
                    format!("the suspension of the signer's certificate at {at}")
                }
                TimeKind::Revocation { suspended: false } => {
                    format!("the revocation of the signer's certificate at {at}")
                }
                TimeKind::ListIssued { next_update } => format!(
                    "a revocation list about the signer's certificate issued at {at}{}",
                    next(next_update)
                ),
                TimeKind::ResponseProduced { next_update } => format!(
                    "an OCSP response about the signer's certificate produced at {at}{}",
                    next(next_update)
                ),
                TimeKind::BestSignatureTime => format!("the best signature time {at}"),
            };
            let assurance = match &time.assurance {
                Assurance::Claimed => "a claim by the signer, which proves nothing".to_owned(),
                Assurance::Token {
                    established,
                    policy,
                    accuracy,
                    eu_qualified,
                    listed,
                } => format!(
                    "{}, under the time-stamping policy {}, {}; {}; {}",
                    if *established {
                        "established by the token's validation"
                    } else {
                        "not established"
                    },
                    if policy.is_empty() {
                        "that does not decode"
                    } else {
                        policy
                    },
                    accuracy.map_or_else(
                        || "stating no accuracy".to_owned(),
                        |accuracy| format!("accurate to {} microseconds", accuracy.micros_total())
                    ),
                    eu_qualified_words(eu_qualified),
                    listed_words(eu_qualified, listed),
                ),
                Assurance::StatedBy(evidence) => format!("as {} states it", evidence.name()),
                Assurance::Best(proof) => match proof {
                    Proof::ADocumentTimestamp => "fixed by a document timestamp".to_owned(),
                    Proof::ThisSignaturesTimestampAttribute => {
                        "fixed by this signature's timestamp attribute".to_owned()
                    }
                    _ => "the clock this reader was given, nothing having proven an earlier one"
                        .to_owned(),
                },
            };
            format!("{what} ({assurance})")
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("The times that bear on it, in seconds after the epoch (REQ-4.5-01 d)): {said}")
}

/// A `nextUpdate`, where one is stated.
fn next(next_update: Option<pdf_signature::x509::Instant>) -> String {
    next_update.map_or_else(
        || ", stating no next update".to_owned(),
        |at| format!(", next update {}", at.unix_seconds()),
    )
}

/// REQ-4.5-01 f): the signer's attributes, signed and unsigned.
fn attributes_sentence(
    attributes: &[pdf_signature::verdict::applicability::SignatureAttribute],
) -> String {
    let list = |signed: bool| {
        let named: Vec<String> = attributes
            .iter()
            .filter(|attribute| attribute.signed == signed)
            .map(|attribute| {
                let mut said = attribute
                    .name
                    .map_or_else(|| attribute.identifier.clone(), str::to_owned);
                if let Some((identifier, name)) = &attribute.commitment {
                    said = format!(
                        "{said}, committing to {}",
                        name.map_or_else(|| identifier.clone(), str::to_owned)
                    );
                }
                said
            })
            .collect();
        if named.is_empty() {
            "none".to_owned()
        } else {
            named.join(", ")
        }
    };
    format!(
        "Its attributes (REQ-4.5-01 f)): signed — {}; unsigned — {}",
        list(true),
        list(false)
    )
}

/// One SAML assertion a `signer-attributes-v2` carries, as ETSI EN 319 102-1 clause 5.2.8.4.2.7
/// makes it available: what it says, and whether its signature was verified (ADR 2922). An
/// assertion whose signature is invalid has its contents withheld, as SAML 2.0 core section 2.3.3
/// forbids relying on them.
fn assertion_sentence(
    ordinal: usize,
    assertion: &pdf_signature::verdict::applicability::CarriedAssertion,
) -> String {
    use pdf_signature::saml::SignatureCheck;
    use pdf_signature::verdict::applicability::AssertionKind;
    let member = match &assertion.kind {
        AssertionKind::Claimed => "a claimed SAML assertion (ETSI EN 319 122-1 clause 5.2.6.2)",
        AssertionKind::Signed => "a signed SAML assertion (ETSI EN 319 122-1 clause 5.2.6.3)",
        AssertionKind::OtherForm(identifier) => {
            return format!(
                "Signer assertion {ordinal}: a signed assertion of form {identifier}, which ETSI \
                 EN 319 122-1 does not define, so it is not read"
            );
        }
    };
    let Some(saml) = &assertion.saml else {
        return format!("Signer assertion {ordinal}: {member} that carries no assertion");
    };
    let read = match &saml.assertion {
        Ok(read) => read,
        Err(why) => {
            return format!("Signer assertion {ordinal}: {member} that cannot be read: {why}");
        }
    };
    let signature = match &saml.signature {
        SignatureCheck::Unsigned => "it is not signed, so it is the signer's claim".to_owned(),
        SignatureCheck::Verified(_) => {
            let by = assertion.signer.as_ref().map_or_else(
                || "a certificate whose names do not read".to_owned(),
                |signer| {
                    signer
                        .subject
                        .iter()
                        .map(|attribute| {
                            format!(
                                "{}={}",
                                attribute_short_name(&attribute.kind),
                                attribute
                                    .text
                                    .as_deref()
                                    .unwrap_or("(a value that is not text)")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            );
            format!(
                "its signature verifies under the certificate it carries, of {by}, which this \
                 report does not chain to any anchor"
            )
        }
        SignatureCheck::Failed(why) if saml.signature.is_invalid() => {
            return format!(
                "Signer assertion {ordinal}: {member} issued by {}, whose signature is invalid \
                 ({why}); SAML 2.0 core section 2.3.3 forbids relying on its contents, so they \
                 are not shown",
                read.issuer
            );
        }
        SignatureCheck::Failed(why) => format!("its signature is unverified, not valid: {why}"),
    };
    let interval = match (&read.not_before, &read.not_on_or_after) {
        (None, None) => String::new(),
        (start, end) => format!(
            ", valid from {} until {}",
            start
                .as_ref()
                .map_or("any time", |(spelled, _)| spelled.as_str()),
            end.as_ref()
                .map_or("no expiry", |(spelled, _)| spelled.as_str())
        ),
    };
    let attributes = if read.attributes.is_empty() {
        "no attribute".to_owned()
    } else {
        read.attributes
            .iter()
            .map(|attribute| {
                let name = attribute.friendly_name.as_ref().map_or_else(
                    || attribute.name.clone(),
                    |friendly| format!("{friendly} ({})", attribute.name),
                );
                format!("{name} = {}", attribute.values.join(" | "))
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    let encrypted = if read.encrypted_attributes > 0 {
        format!(
            ", and {} encrypted attribute(s) this program does not open",
            read.encrypted_attributes
        )
    } else {
        String::new()
    };
    format!(
        "Signer assertion {ordinal}: {member} issued by {} at {} about {}{interval}: \
         {attributes}{encrypted}; {signature}",
        read.issuer,
        read.issue_instant,
        read.subject.as_deref().unwrap_or("no stated subject")
    )
}

/// ETSI TS 119 172-4 REQ-4.2-03 e)'s warning.
fn unsupported_suite_sentence(suite: &str) -> String {
    format!(
        "Warning: this signature's cryptographic suite, {suite}, is not supported by this \
         program. Under ETSI TS 119 172-4 REQ-4.2-03 e) that does not invalidate the signature: \
         its result against the trusted lists is INDETERMINATE"
    )
}

/// The trusted lists a host supplied: how many read, which did not, and on whose word they are
/// believed — the list-side sentence to [`anchors_note`]'s. Where ETSI TS 119 615 clause 4.1 ran,
/// its outcome is said with the clause's own indications, and every list it did not authenticate
/// is named with why.
fn lists_note(
    trust: &crate::TrustPolicy,
    reading: &pdf_signature::trusted_list::Reading,
    notes: &mut Vec<String>,
) {
    if trust.lists.is_empty() {
        return;
    }
    let territories: Vec<&str> = reading
        .lists
        .iter()
        .map(|list| list.territory.as_str())
        .collect();
    notes.push(match &reading.lotl {
        None => format!(
            "the trusted lists this reader searched for the signers' certification authorities \
             were supplied by whoever started it, from {}: {} of {} read as ETSI TS 119 612 lists \
             ({}). Neither the European Commission's list of trusted lists nor the Official \
             Journal's certificates were among them, so their own signatures were not checked \
             (ETSI TS 119 615 clauses 4.1 and 4.2) and they are believed for where they came \
             from, as an anchor is",
            trust.lists.source(),
            reading.lists.len(),
            trust.lists.lists().len(),
            territories.join(", "),
        ),
        Some((_, outcome)) if !outcome.passed() => {
            format!(
                "the European Commission's list of trusted lists supplied from {} was not \
                 authenticated against the Official Journal's certificates (ETSI TS 119 615 \
                 clause 4.1: {outcome}), so no trusted list was searched",
                trust.lists.source(),
            )
        }
        Some((sequence_number, outcome)) => format!(
            "the trusted lists this reader searched for the signers' certification authorities \
             were supplied from {}, and {} of them were authenticated ({}) under the European \
             Commission's list of trusted lists, sequence number {sequence_number}, which was \
             itself authenticated against the Official Journal's certificates (ETSI TS 119 615 \
             clauses 4.1 and 4.2: {outcome})",
            trust.lists.source(),
            reading.lists.len(),
            territories.join(", "),
        ),
    });
    for (name, outcome) in &reading.warnings {
        notes.push(format!(
            "{name} was authenticated with a warning (ETSI TS 119 615 clause 4.2): {outcome}"
        ));
    }
    for (name, error) in &reading.refused {
        notes.push(format!(
            "{name} is not a trusted list this reader can use: {error}"
        ));
    }
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
        Integrity::NoDigestOfTheDocument => {
            "that signature records no digest of this document and is not over its bytes, so \
             nothing its signer signed says what the document was (§12.8.1, §12.8.3.3.1)"
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
             RFC 8017's paddings, DSA and ECDSA — and the EdDSA row ISO/TS 32002 adds to that \
             table"
        ),
        Authenticity::CurveNotVerifiable { curve } => format!(
            "and the signer's certificate holds an elliptic-curve key on curve {curve}, which \
             this program does not compute on: it computes on the six curves ISO/TS 32002 \
             Table 3 names, and this is none of them"
        ),
        Authenticity::AlgorithmNotVerifiable { algorithm } => format!(
            "and that signature states signature algorithm {algorithm}, which this program does \
             not verify: it verifies RSASSA-PKCS1-v1_5, RSASSA-PSS, DSA, ECDSA and EdDSA"
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

    /// [`super::about`] with no fetched policy copy, which is what every test but the ones about
    /// fetched copies asks.
    fn about(document: &Document, trust: &crate::TrustPolicy) -> super::About {
        super::about(document, trust, &[])
    }

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

    /// Pages found by scanning are said out loud with their count, and a tree's pages are not.
    ///
    /// A pair differing in one entry: the catalogue's `/Pages` points at the node in the first and
    /// at an object the file does not hold in the second, so the second's page is found by its
    /// own `/Type /Page` and shown in an order the file never stated (ADR 1533).
    #[test]
    fn pages_found_by_scanning_are_said_with_their_count_and_a_tree_is_not() {
        let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>";
        let tree = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            page,
        ]);
        let said = about(&tree, &crate::TrustPolicy::default())
            .notes
            .join("\n");
        assert!(!said.contains("found by scanning"), "{said}");

        let scanned = document(&[
            "<< /Type /Catalog /Pages 9 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            page,
        ]);
        let said = about(&scanned, &crate::TrustPolicy::default())
            .notes
            .join("\n");
        assert!(
            said.contains("its 1 page(s) were found by scanning"),
            "{said}"
        );
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
        let said = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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
        let said = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");
        assert!(
            said.contains("every one of those changes is one §12.8.2.2's /P 3 permits"),
            "{said}"
        );
        assert!(!said.contains("does not permit"), "{said}");
        // **The word no sentence of this report may reach**, at either level: the ranking is a
        // statement about objects and §12.8.1's third question has no trust store behind it.
        assert!(
            !said.contains("signature is valid") && !said.contains("signature is locally valid"),
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
        let said = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");

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
        let said = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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
        let said = about(&stamped, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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
        let quiet = about(&plain, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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
        let said = about(&with_store, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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
                 nobody has done for this reading"
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
                .notes
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
        let said = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");

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
        let said = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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
                .notes
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
        let said = about(&document, &crate::TrustPolicy::default()).notes;
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
            critical[0].contains("the transform method's own second step came to is said"),
            "the sentence points at the answer the file asked to be counted, which is made since \
             ADR 1104: {critical:?}"
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
        let said = about(&document, &crate::TrustPolicy::default()).notes;
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
            stated[0].contains("Sha512") && stated[0].contains("places objects rather than"),
            "the sentence names the function and what this program's comparison does instead: \
             {stated:?}"
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
        let said = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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
    /// the entire PDF file", and then the same tail, where the range does not stop at the `%%EOF`
    /// of a revision (this one stops at byte 300 of a longer file), is the file breaking a
    /// requirement — which is worth saying to somebody deciding whether to trust what they are
    /// looking at. `tests/pades_range_end.rs` has the later-revision side (ADR 3230).
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
            about(&document(&borrowed), &crate::TrustPolicy::default())
                .notes
                .join("\n")
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
            required.contains(
                "Table 255 requires /SubFilter ETSI.CAdES.detached's signed range to cover the \
                 entire file"
            ),
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
        .notes
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
            .notes
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
            .notes
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
            .notes
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
            .notes
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
        .notes
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
            .notes
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
            .notes
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
        .notes.join("\n");
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
            lists: pdf_signature::trusted_list::Lists::none(),
        };
        let said = about(&document, &trust).notes.join("\n");

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
            said.contains("and this is the first thing that stopped it"),
            "{said}"
        );
        assert!(
            said.contains("no certification path from the signer's certificate reaches any anchor"),
            "the corpus's authorities are not this project's to anchor at (trap 8): {said}"
        );
        // **And *as of when*, whichever answer it is.** ETSI EN 319 102-1 clause 5.5.4's instant
        // is the reader's clock unless a token proves an earlier moment, and this document's
        // tokens prove nothing because no anchor of theirs is supplied — so the sentence names the
        // clock, which is the answer a reader must not have to assume.
        assert!(
            said.contains(
                "as of 1790812800 seconds after the epoch, which is the clock this \
                 reader was given, nothing having proven an earlier moment"
            ),
            "a verdict is asserted as of an instant and says which: {said}"
        );
        assert!(
            !said.contains("that signature is valid")
                && !said.contains("that signature is locally valid"),
            "nothing in the crawl reaches the word: {said}"
        );

        // **And with no anchor the report is the one this program prints without the supply**,
        // which is what makes the supply an input rather than a change of behaviour.
        let quiet = about(&document, &crate::TrustPolicy::default())
            .notes
            .join("\n");
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

    /// One DER value: a tag, X.690's shortest definite length, and the contents.
    fn der(tag: u8, contents: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        let length = contents.len();
        if length < 0x80 {
            out.push(u8::try_from(length).expect("under 128"));
        } else if length < 0x100 {
            out.extend([0x81, u8::try_from(length).expect("under 256")]);
        } else {
            let length = u16::try_from(length).expect("the fixture is short");
            out.push(0x82);
            out.extend(length.to_be_bytes());
        }
        out.extend_from_slice(contents);
        out
    }

    /// A CMS `SignedData` whose one `SignerInfo` signs §12.8.3.4.4's signature-policy-identifier
    /// (ETSI EN 319 122-1 clause 5.2.9.1) under `2.16.724.1.3.1.1.2.1.9`, the corpus's thirteen
    /// signatures' policy, with SHA-1 `digest` and, where `url` is given, clause 5.2.9.2's URL
    /// qualifier. Nothing in it verifies: the policy is read whether or not the signature does.
    fn policy_cms(digest: &[u8], url: Option<&str>) -> Vec<u8> {
        policy_cms_carrying(digest, url, &[], (&der(0x30, &[]), &[1]))
    }

    /// [`policy_cms`] carrying `certificates`, its signer named by `signer`'s issuer `Name` and
    /// serial number contents.
    fn policy_cms_carrying(
        digest: &[u8],
        url: Option<&str>,
        certificates: &[&[u8]],
        signer: (&[u8], &[u8]),
    ) -> Vec<u8> {
        const SIGNED_DATA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02];
        const DATA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01];
        const SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
        const SHA1: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];
        const RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
        const SIG_POLICY_ID: &[u8] = &[
            0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x02, 0x0f,
        ];
        const SPQ_URI: &[u8] = &[
            0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x05, 0x01,
        ];
        const POLICY: &[u8] = &[0x60, 0x85, 0x54, 0x01, 0x03, 0x01, 0x01, 0x02, 0x01, 0x09];
        let algorithm = |oid: &[u8]| der(0x30, &der(0x06, oid));
        let mut policy = [
            der(0x06, POLICY),
            der(0x30, &[algorithm(SHA1), der(0x04, digest)].concat()),
        ]
        .concat();
        if let Some(url) = url {
            let qualifier = der(
                0x30,
                &[der(0x06, SPQ_URI), der(0x16, url.as_bytes())].concat(),
            );
            policy.extend(der(0x30, &qualifier));
        }
        let attribute = der(
            0x30,
            &[der(0x06, SIG_POLICY_ID), der(0x31, &der(0x30, &policy))].concat(),
        );
        let signer_info = der(
            0x30,
            &[
                der(0x02, &[1]),
                der(0x30, &[signer.0.to_vec(), der(0x02, signer.1)].concat()),
                algorithm(SHA256),
                der(0xa0, &attribute),
                der(0x30, &[der(0x06, RSA), der(0x05, &[])].concat()),
                der(0x04, &[0; 8]),
            ]
            .concat(),
        );
        let mut members = vec![
            der(0x02, &[1]),
            der(0x31, &algorithm(SHA256)),
            algorithm(DATA),
        ];
        if !certificates.is_empty() {
            members.push(der(0xa0, &certificates.concat()));
        }
        members.push(der(0x31, &signer_info));
        let signed_data = der(0x30, &members.concat());
        der(
            0x30,
            &[der(0x06, SIGNED_DATA), der(0xa0, &signed_data)].concat(),
        )
    }

    /// A one-page document signed under [`policy_cms`].
    fn signed_under_a_policy(url: Option<&str>) -> Vec<u8> {
        signed_with_cms(&policy_cms(&[0x5a; 20], url))
    }

    /// A one-page document whose one signature's `/Contents` is `cms`.
    fn signed_with_cms(cms: &[u8]) -> Vec<u8> {
        use std::fmt::Write as _;
        let mut contents = String::new();
        for octet in cms {
            let _ = write!(contents, "{octet:02x}");
        }
        signed_document(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>",
            &format!(
                "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached \
                 /ByteRange [0000000000 0000000000 0000000000 0000000000] \
                 /Contents <{contents}> >>"
            ),
        )
    }

    /// §12.8.3.4.4's policy copy reaches a host as data, not as a sentence: the URL clause
    /// 5.2.9.2's qualifier names, with the identifier and the digest the signer committed to.
    ///
    /// **The calibration is the same signature without the qualifier** (trap 13): the policy is
    /// still said, and nothing is handed over, so a list that filled from the identifier alone
    /// would fail the second half.
    #[test]
    fn a_policy_s_published_copy_is_handed_over_beside_the_sentence_naming_the_policy() {
        let url = "http://127.0.0.1:9/politica.pdf";
        let document = Document::open(signed_under_a_policy(Some(url))).expect("a valid file");
        let said = about(&document, &crate::TrustPolicy::default());
        assert!(
            said.notes
                .iter()
                .any(|note| note.contains("signature policy 2.16.724.1.3.1.1.2.1.9")),
            "{:?}",
            said.notes
        );
        assert_eq!(
            said.published,
            vec![pdf_signature::policy::PublishedPolicy {
                identifier: "2.16.724.1.3.1.1.2.1.9".to_owned(),
                url: url.to_owned(),
                commitment: pdf_signature::policy::Commitment::Stated {
                    digest: pdf_signature::cms::Digest::Sha1,
                    value: vec![0x5a; 20],
                },
                specification: None,
            }]
        );

        let unpublished = Document::open(signed_under_a_policy(None)).expect("a valid file");
        let said = about(&unpublished, &crate::TrustPolicy::default());
        assert!(
            said.notes
                .iter()
                .any(|note| note.contains("signature policy 2.16.724.1.3.1.1.2.1.9")),
            "the policy is said without a qualifier too: {:?}",
            said.notes
        );
        assert!(said.published.is_empty(), "{:?}", said.published);
    }

    /// A root `openssl` issued for this test, 2026 to 2036, and the signer it issued (serial
    /// `0x2029`), every key generated for this project (`/home/quorra/scratch/agent/r2029/gen/gen.sh`).
    const A_LISTED_ROOT: &str = "\
    3082032930820211a00302010202145ac2d793dc00b8d54a4e000a20dbd702cc\
    c02925300d06092a864886f70d01010b0500301c311a301806035504030c1172\
    32303239206c697374656420726f6f74301e170d323631313136303034373032\
    5a170d3336313131333030343730325a301c311a301806035504030c11723230\
    3239206c697374656420726f6f7430820122300d06092a864886f70d01010105\
    000382010f003082010a0282010100be2eb4b5e8b2d0c12b43b5c91c592a8af6\
    27e4e9763928bf13d8603837e4a32f324e14663870f46e6f8764a197d14ff593\
    30ebe6a9012bca294e07a5a37b5501bcc00c71a01f4c7f5e4e65500ac0ccfc64\
    f42c71832b93d8c7496fce08573620b82e32a37b7ab624adf5f427d2e6a3fdf1\
    9079442c923f78f7627a2ed9e90b0c32df88ce7372e3b926d621c0f4e9beb1ec\
    de27c9f1544e4ffd458b2a13bbc736705cb36c0f6a475de6ba602992c84deec4\
    10b6500a0042eb10ac27fdf28409798b51a9d4fa9421bc8a3053a6f69a1f007f\
    6095a68b713b86630a90fd5378babd91cfa17b7dd7dec7332a4f0ed918855547\
    18e57513ba3d4de87031d69373de6b0203010001a3633061301d0603551d0e04\
    16041477f60d6688813408bbf3bcb08250463c238b69fc301f0603551d230418\
    3016801477f60d6688813408bbf3bcb08250463c238b69fc300f0603551d1301\
    01ff040530030101ff300e0603551d0f0101ff040403020106300d06092a8648\
    86f70d01010b050003820101001f90164639c8bc7fec6085936e2826e7929f77\
    be5d3dc634d53352698a42a83bd98fc8f95e8cc3027b2c16091bbd341e745c59\
    b2c1fd3048b4f9c6209064c349b98365325eaf7138b6208eac161eec08cd6d52\
    5168c3651f49199e6a7ecad839be4bca6e2a8a4298ec3ee55a25527781d0169a\
    e1161dccdd51d73f6907d697621fa453f20d1dc71db0bc8ae066f6fcd4fc729e\
    209dc70b2e05d34d561b7aad68cdabfbc330b7fde3e5d0e58cda4566f2ccbb6a\
    dd1c1bb2ef43f82e5541b55e9fd7184fe58cdffd94874d7a96673a8697b8d3b5\
    3f7f0b6bdb4ebe1e2bcf1b7d516b540a4a46820d6668d09ce30bc007d382f62a\
    e0e99b891b979002f7bf0955fb\
    ";
    const ITS_SIGNER: &str = "\
    3082030f308201f7a00302010202022029300d06092a864886f70d01010b0500\
    301c311a301806035504030c117232303239206c697374656420726f6f74301e\
    170d3236313131363030343730325a170d3335303230323030343730325a3017\
    3115301306035504030c0c7232303239207369676e657230820122300d06092a\
    864886f70d01010105000382010f003082010a0282010100b9517bcef8d9e106\
    bd8fc9adb9f49d66e8db49b1cdb3d0286316ad0fc4b596448d1a76566c72dd5d\
    f0352423cb1b39cf72240ab07f0a4e75a617dfe1a37e15e1d8b10fd62766dff2\
    b7d4ec666c7137e9e9092f48bebdf7196a3eada7b45566580dd64bc8b7794176\
    d7a4eb7caeec0944058be1f50879ca35923003b3b72100fc71e621258f7d6803\
    e3dd7f2aad7c5914ca8bd58a48a6e6c3d72c6671fa5f500f39f22c67d92eba87\
    5d09ff8104619a5f8665e8a734f5c22a698088039b0e4217cd0238678f131108\
    bf1aff26fc0a1fc14f2381d8336905b98dee227d0edf7e1de25c2289d1ad2bcd\
    dd18693ac5cbe19a090402c81db4b4de29880b202e4035d30203010001a36030\
    5e300c0603551d130101ff04023000300e0603551d0f0101ff0404030206c030\
    1d0603551d0e041604142913d3b45cc729914dcadf9db57d9a597d867dd7301f\
    0603551d2304183016801477f60d6688813408bbf3bcb08250463c238b69fc30\
    0d06092a864886f70d01010b05000382010100a1cbc1371925f1702bfbc599a3\
    bcd47e45db2e2c30515bc2f3d56ee3357a22a2320baa980f6fa5c15a4e377e35\
    b77476f1f814b4f4f9599bf1f4d3dbe375ca64ce517fde3401774f7bea2764c9\
    5a8ae3a538517dae386404b07a99452e1243394efec92bacdb543adb7cbbd66c\
    a2287ddca3906fb50cff63ae08020e887359ae3df4ac8b3a1f2d83db947e60f1\
    506cf0c4ece517d081b517a5c05859c71496bd21f2f0227c7930049d9de46480\
    c54c022d3406d66b607aa398f0bb2712f3d03ff3e04962c938acc476da706e89\
    e8c319927b87f76a542e4c9e343237a2112c03052009ff6baaaa778df7f36cf9\
    4e7cc96eb95d5504cf851e8e4addbbd47023b5\
    ";

    /// RFC 4648 section 4's alphabet, padded, which TS 119 612 clause 5.5.3 and XML Schema's
    /// `base64Binary` both write.
    fn base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let mut padded = [0_u8; 3];
            padded[..chunk.len()].copy_from_slice(chunk);
            let word = u32::from_be_bytes([0, padded[0], padded[1], padded[2]]);
            for (at, shift) in [18_u32, 12, 6, 0].into_iter().enumerate() {
                out.push(if at <= chunk.len() {
                    char::from(ALPHABET[(word >> shift & 63) as usize])
                } else {
                    '='
                });
            }
        }
        out
    }

    /// **ETSI TS 119 172-2 clause 4.22.2.1 b)'s anchors are determined from the lists a host
    /// supplied** (ADR 2874): a signature made under a policy whose one trust point is a reference
    /// to Austria's list, a copy of that policy fetched and bound by the digest the signer signed,
    /// and the list naming the root that issued the signer as a granted `CA/QC` service's identity.
    /// The policy's signing-certificate trust condition is met, and the sentence names the list.
    ///
    /// **The calibration is the same reading with no list supplied** (trap 13): the reference is
    /// then an anchor not examined, so the condition is not evaluated, and the sentence says why.
    #[test]
    fn a_policys_trusted_list_anchor_is_judged_against_the_hosts_lists() {
        let root = hex(A_LISTED_ROOT);
        let signer = hex(ITS_SIGNER);
        let leaf = pdf_signature::x509::parse(&signer).expect("the fixture signer");
        let policy = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<SignaturePolicy xmlns="http://uri.etsi.org/19172/v1.1.1#" xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
  <Digest>
    <ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
    <ds:DigestValue>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</ds:DigestValue>
    <ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>
  </Digest>
  <PolicyComponents>
    <GeneralDetails>
      <SigPolicyDetails>
        <PolicyIdentifier>urn:oid:2.16.724.1.3.1.1.2.1.9</PolicyIdentifier>
        <PolicyName><tsl:Name xmlns:tsl="http://uri.etsi.org/02231/v2#" xml:lang="en">tests</tsl:Name></PolicyName>
      </SigPolicyDetails>
      <OtherDetails><DateOfIssue>2025-12-01T00:00:00Z</DateOfIssue>
        <SigningPeriod><NotBefore>2026-01-01T00:00:00Z</NotBefore></SigningPeriod></OtherDetails>
    </GeneralDetails>
    <PolicyRules><PolicyRule><SigningCertRules><SigningCertTrustConditions><SignerTrustTrees>
      <CertificateTrustPoint><TrustAnchors><TAsInTrustedList>
        <TLReference>https://example.at/tl.xml</TLReference>
        <ServiceTypes><tsl:URI xmlns:tsl="http://uri.etsi.org/02231/v2#">{}</tsl:URI></ServiceTypes>
      </TAsInTrustedList></TrustAnchors></CertificateTrustPoint></SignerTrustTrees>
      <SignerRevTrust><CertificateRevReq><EndRevReq>nocheck</EndRevReq><CACerts>nocheck</CACerts>
      </CertificateRevReq></SignerRevTrust></SigningCertTrustConditions></SigningCertRules>
    </PolicyRule></PolicyRules>
  </PolicyComponents>
</SignaturePolicy>"#,
            pdf_signature::trusted_list::CA_QC
        );
        let digest = pdf_signature::cms::Digest::Sha1.compute(&[policy.as_bytes()]);
        let url = "http://127.0.0.1:9/policy.xml";
        let document = Document::open(signed_with_cms(&policy_cms_carrying(
            &digest,
            Some(url),
            &[&signer],
            (&der(0x30, leaf.issuer), leaf.serial_number),
        )))
        .expect("a valid file");
        let list = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <tsl:TrustServiceStatusList xmlns:tsl=\"{}\" TSLTag=\"http://uri.etsi.org/19612/TSLTag\">\
             <tsl:SchemeInformation><tsl:TSLVersionIdentifier>6</tsl:TSLVersionIdentifier>\
             <tsl:TSLSequenceNumber>1</tsl:TSLSequenceNumber><tsl:SchemeTerritory>AT</tsl:SchemeTerritory>\
             <tsl:ListIssueDateTime>2026-11-01T00:00:00Z</tsl:ListIssueDateTime>\
             <tsl:NextUpdate><tsl:dateTime>2027-05-01T00:00:00Z</tsl:dateTime></tsl:NextUpdate>\
             </tsl:SchemeInformation><tsl:TrustServiceProviderList><tsl:TrustServiceProvider>\
             <tsl:TSPInformation><tsl:TSPName><tsl:Name xml:lang=\"en\">A provider</tsl:Name></tsl:TSPName>\
             </tsl:TSPInformation><tsl:TSPServices><tsl:TSPService><tsl:ServiceInformation>\
             <tsl:ServiceTypeIdentifier>{}</tsl:ServiceTypeIdentifier>\
             <tsl:ServiceName><tsl:Name xml:lang=\"en\">Qualified certificates</tsl:Name></tsl:ServiceName>\
             <tsl:ServiceDigitalIdentity><tsl:DigitalId><tsl:X509Certificate>{}</tsl:X509Certificate>\
             </tsl:DigitalId></tsl:ServiceDigitalIdentity><tsl:ServiceStatus>{}</tsl:ServiceStatus>\
             <tsl:StatusStartingTime>2024-01-01T00:00:00Z</tsl:StatusStartingTime>\
             </tsl:ServiceInformation></tsl:TSPService></tsl:TSPServices></tsl:TrustServiceProvider>\
             </tsl:TrustServiceProviderList></tsl:TrustServiceStatusList>",
            pdf_signature::trusted_list::NAMESPACE,
            pdf_signature::trusted_list::CA_QC,
            base64(&root),
            pdf_signature::trusted_list::GRANTED,
        );
        // 2027-01-01T00:00:00Z: inside the signer's validity, the list's period and the grant.
        let at = pdf_signature::x509::Instant::from_unix_seconds(1_798_761_600);
        let policy_of = |lists: Vec<(String, Vec<u8>)>| crate::TrustPolicy {
            // A host given lists and no anchor directory carries its clock with them (ADR 2842).
            anchors: pdf_signature::trust::Supply::of(Vec::new(), "no directory".to_owned(), at),
            acceptance: pdf_signature::verdict::Acceptance::RevocationMustBeGood,
            lists: pdf_signature::trusted_list::Lists::of(lists, "the tests".to_owned()),
        };
        let published = about(&document, &policy_of(Vec::new())).published;
        let copies = [super::FetchedCopy {
            published: published.first().expect("the URL qualifier's copy").clone(),
            octets: policy.clone().into_bytes(),
        }];
        let trust_sentence = |lists: Vec<(String, Vec<u8>)>| {
            super::about(&document, &policy_of(lists), &copies)
                .notes
                .into_iter()
                .find(|note| note.contains("signing certificate trust condition"))
                .expect("the trust condition is said")
        };
        let listed = trust_sentence(vec![("tl.xml".to_owned(), list.into_bytes())]);
        assert!(listed.contains("trust condition is met"), "{listed}");
        assert!(
            listed.contains("the trusted list https://example.at/tl.xml the policy references"),
            "{listed}"
        );
        let unsupplied = trust_sentence(Vec::new());
        assert!(
            unsupplied.contains("trust condition is not evaluated"),
            "{unsupplied}"
        );
        assert!(
            unsupplied.contains("1 of its trust anchors were not searched"),
            "{unsupplied}"
        );
    }

    /// **The reference a bound policy names is what a host is asked to fetch** (ADR 2936): ETSI
    /// TS 119 172-2 clause 4.22.2.1 b) determines the trust point's anchors from the list its
    /// `TLReference` names. A focused document answers `Query::TrustedListReferences` with an
    /// empty list until a bound copy of its policy is held, and with the reference once it is.
    ///
    /// **The calibration is a copy no digest binds** (trap 13): the same policy with one octet
    /// added is nobody's agreement, so a location it names is not put to the reader.
    #[test]
    fn a_bound_policys_list_reference_is_answered_to_a_host() {
        use crate::{Answer, Command, DocumentId, Query, Viewer};
        let signer = hex(ITS_SIGNER);
        let leaf = pdf_signature::x509::parse(&signer).expect("the fixture signer");
        let policy = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<SignaturePolicy xmlns="http://uri.etsi.org/19172/v1.1.1#" xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
  <Digest>
    <ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
    <ds:DigestValue>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</ds:DigestValue>
    <ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>
  </Digest>
  <PolicyComponents>
    <GeneralDetails>
      <SigPolicyDetails>
        <PolicyIdentifier>urn:oid:2.16.724.1.3.1.1.2.1.9</PolicyIdentifier>
        <PolicyName><tsl:Name xmlns:tsl="http://uri.etsi.org/02231/v2#" xml:lang="en">tests</tsl:Name></PolicyName>
      </SigPolicyDetails>
      <OtherDetails><DateOfIssue>2025-12-01T00:00:00Z</DateOfIssue>
        <SigningPeriod><NotBefore>2026-01-01T00:00:00Z</NotBefore></SigningPeriod></OtherDetails>
    </GeneralDetails>
    <PolicyRules><PolicyRule><SigningCertRules><SigningCertTrustConditions><SignerTrustTrees>
      <CertificateTrustPoint><TrustAnchors><TAsInTrustedList>
        <TLReference>https://example.at/tl.xml</TLReference>
        <ServiceTypes><tsl:URI xmlns:tsl="http://uri.etsi.org/02231/v2#">{}</tsl:URI></ServiceTypes>
      </TAsInTrustedList></TrustAnchors></CertificateTrustPoint></SignerTrustTrees>
      <SignerRevTrust><CertificateRevReq><EndRevReq>nocheck</EndRevReq><CACerts>nocheck</CACerts>
      </CertificateRevReq></SignerRevTrust></SigningCertTrustConditions></SigningCertRules>
    </PolicyRule></PolicyRules>
  </PolicyComponents>
</SignaturePolicy>"#,
            pdf_signature::trusted_list::CA_QC
        );
        let digest = pdf_signature::cms::Digest::Sha1.compute(&[policy.as_bytes()]);
        let bytes = signed_with_cms(&policy_cms_carrying(
            &digest,
            Some("http://127.0.0.1:9/policy.xml"),
            &[&signer],
            (&der(0x30, leaf.issuer), leaf.serial_number),
        ));
        let published = about(
            &Document::open(bytes.clone()).expect("a valid file"),
            &crate::TrustPolicy::default(),
        )
        .published;
        let published = published.first().expect("the URL qualifier's copy").clone();
        let answered = |copy: Option<Vec<u8>>| {
            let mut viewer = Viewer::new(200, 200, 1.0);
            let _ = viewer
                .handle(Command::Open {
                    id: DocumentId(1),
                    bytes: bytes.clone().into(),
                    password: None,
                    fragment: None,
                })
                .count();
            if let Some(copy) = copy {
                let _ = viewer
                    .handle(Command::PolicyCopy {
                        document: DocumentId(1),
                        policy: published.clone(),
                        copy,
                    })
                    .count();
            }
            match viewer.query(Query::TrustedListReferences) {
                Answer::TrustedListReferences(locations) => locations.to_vec(),
                other => panic!("an open document answers with a list: {other:?}"),
            }
        };
        assert_eq!(answered(None), Vec::<String>::new());
        assert_eq!(
            answered(Some(policy.clone().into_bytes())),
            ["https://example.at/tl.xml"]
        );
        let mut unbound = policy.into_bytes();
        unbound.push(b'\n');
        assert_eq!(answered(Some(unbound)), Vec::<String>::new());
        assert!(matches!(
            Viewer::new(200, 200, 1.0).query(Query::TrustedListReferences),
            Answer::None
        ));
    }

    /// The event comes from `Command::Report`, after the sentences, once per asking, and not at
    /// all where nothing is published (ADR 1738).
    #[test]
    fn the_report_hands_a_host_the_published_policies_after_its_sentences() {
        use crate::{Command, DocumentId, Event, Viewer};
        let kinds = |bytes: Vec<u8>| {
            let mut viewer = Viewer::new(200, 200, 1.0);
            let _ = viewer
                .handle(Command::Open {
                    id: DocumentId(1),
                    bytes: bytes.into(),
                    password: None,
                    fragment: None,
                })
                .count();
            viewer
                .handle(Command::Report)
                .filter_map(|event| match event {
                    Event::Reported { page: None, .. } => Some("reported".to_owned()),
                    Event::SignaturePoliciesPublished { document, policies } => Some(format!(
                        "published {} for {}",
                        policies
                            .iter()
                            .map(|policy| policy.url.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                        document.0
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            kinds(signed_under_a_policy(Some("http://127.0.0.1:9/p.pdf"))),
            ["reported", "published http://127.0.0.1:9/p.pdf for 1"]
        );
        assert_eq!(kinds(signed_under_a_policy(None)), ["reported"]);
    }

    /// Annex K's form is said to be one, once per kind, on the annex's own two entries.
    ///
    /// §K.1: a processor that supports XFA forms "shall clearly indicate to the user when they are
    /// interacting with an XFA form that they are interacting with an XFA form". A hybrid form's
    /// sentence says what is carried out, and §K.1's permission — "a PDF processor may choose to
    /// not implement this feature" — is said to be taken where the form's template would generate
    /// its pages. The three kinds are the two entries' combinations a document can state; **the
    /// control is the same form without `/XFA`**, which is an `AcroForm` and owes nothing (trap
    /// 11). ADRs 1932, 2430.
    #[test]
    fn an_xfa_form_is_said_to_be_one_and_what_of_it_is_carried_out() {
        let said = |catalog: &str| {
            about(
                &document(&[
                    catalog,
                    "<< /Type /Pages /Kids [] /Count 0 >>",
                    "<< /Length 9 >>\nstream\n<xdp:xdp>\nendstream",
                ]),
                &crate::TrustPolicy::default(),
            )
            .notes
            .join("\n")
        };
        let hybrid =
            said("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] /XFA 3 0 R >> >>");
        assert!(
            hybrid.contains("carries an XFA form (Annex K)")
                && hybrid.contains("its AcroForm")
                && hybrid.contains("keeps the form's XFA data consistent")
                && hybrid.contains("an entry it refuses")
                && hybrid.contains("is made at every level")
                && hybrid.contains("URL-encoded, XDP or XML body")
                && !hybrid.contains("§K.1"),
            "{hybrid}"
        );
        let dynamic = said(
            "<< /Type /Catalog /Pages 2 0 R /NeedsRendering true \
             /AcroForm << /Fields [] /XFA [(xdp:xdp) 3 0 R] >> >>",
        );
        assert!(
            dynamic.contains("asks to be regenerated when it is opened")
                && dynamic.contains("§K.1 permits")
                && dynamic.contains("often only a notice"),
            "{dynamic}"
        );
        let flag_alone = said("<< /Type /Catalog /Pages 2 0 R /NeedsRendering true >>");
        assert!(
            flag_alone.contains("carries no XFA resource"),
            "{flag_alone}"
        );
        let acroform = said("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] >> >>");
        assert!(!acroform.contains("XFA"), "{acroform}");
    }

    /// **What ETSI TS 119 615 clause 4.1 said is said**, in the clause's own indications, and no
    /// list is searched under a list of trusted lists it did not authenticate (PRO-4.2.4-02); a
    /// directory holding neither that list nor the Official Journal's certificates is believed on
    /// the host's word and says which of the two it was (ADR 2842).
    #[test]
    fn a_list_of_trusted_lists_that_fails_clause_4_1_is_said_and_nothing_is_searched() {
        let policy = |files: Vec<(&str, &[u8])>| crate::TrustPolicy {
            anchors: pdf_signature::trust::Supply::none(),
            acceptance: pdf_signature::verdict::Acceptance::RevocationMustBeGood,
            lists: pdf_signature::trusted_list::Lists::of(
                files
                    .into_iter()
                    .map(|(name, octets)| (name.to_owned(), octets.to_vec()))
                    .collect(),
                "the directory a test named".to_owned(),
            ),
        };
        let said = |trust: &crate::TrustPolicy| {
            let reading = trust
                .lists
                .read(pdf_signature::x509::Instant::from_unix_seconds(
                    1_790_812_800,
                ));
            let mut notes = Vec::new();
            super::lists_note(trust, &reading, &mut notes);
            notes.join("\n")
        };
        let failed = said(&policy(vec![("eu-lotl.xml", b"<not-a-list/>")]));
        assert!(
            failed.contains(
                "was not authenticated against the Official Journal's certificates (ETSI TS 119 \
                 615 clause 4.1: LOTL_VERIFICATION_FAILED (LOTL_UNREADABLE"
            ),
            "{failed}"
        );
        assert!(
            failed.contains("so no trusted list was searched"),
            "{failed}"
        );
        let vouched = said(&policy(vec![("at.xml", b"<not-a-list/>")]));
        assert!(
            vouched.contains(
                "Neither the European Commission's list of trusted lists nor the Official \
                 Journal's certificates were among them"
            ),
            "{vouched}"
        );
        assert!(
            vouched.contains("at.xml is not a trusted list this reader can use"),
            "{vouched}"
        );
    }

    /// ETSI EN 319 412-5's statements are named by their own ASN.1 names with their clauses, as
    /// the issuer's claims, and no level is worded from them (ADR 2835): none of "QES", "qualified
    /// electronic signature" or "qualified certificate" appears in what they produce.
    #[test]
    fn a_certificates_statements_are_named_and_never_a_level() {
        use pdf_signature::qc_statement::{QcStatements, QcType, Stated};
        let read = Stated::Read(QcStatements {
            compliance: true,
            sscd: true,
            types: Some(vec![QcType::Esign]),
            legislation: Some(vec!["CH".to_owned()]),
            others: vec!["0.4.0.1862.1.3".to_owned()],
        });
        let said = super::statements_sentence(&read);
        for named in [
            "QcCompliance (clause 4.2.1)",
            "QcSSCD (clause 4.2.2)",
            "QcType esign (clause 4.2.3)",
            "QcCClegislation CH (clause 4.2.4)",
            "statement 0.4.0.1862.1.3",
            "its issuer's claims",
        ] {
            assert!(said.contains(named), "{named} in {said}");
        }
        for level in ["QES", "qualified"] {
            assert!(!said.contains(level), "{level} in {said}");
        }
        let none = super::statements_sentence(&Stated::Nothing);
        assert!(none.contains("makes none of the statements"), "{none}");
    }

    /// ETSI TS 119 172-4 REQ-4.5-01 d) iii) 3): ETSI TS 119 615 clause 4.7's outputs are said in
    /// the clause's own spelling with the list they came from, and a token not established is
    /// said as not determined for clause 4.7.1's proviso. d) i): a listed service is named with
    /// its type and its status at the token's generation time, and with lists given and none
    /// listing the authority, the absence is said.
    #[test]
    fn a_tokens_qualification_and_listing_are_said_in_the_clauses_words() {
        use pdf_signature::trusted_list::Authentication;
        use pdf_signature::trusted_list::time_stamp::{
            Answer, Indication, ListedAuthority, Outputs, SubStatus, TSA_QTST,
        };
        use pdf_signature::verdict::applicability::EuQualified;
        let failed = EuQualified::Determined(Answer::Determined(Outputs {
            results: Indication::Qualified,
            status: pdf_signature::trusted_list::Status::Failed,
            sub_status: vec![SubStatus::ResultsDifferAtGeneration(
                Indication::NotQualified,
            )],
            country: Some("AT".to_owned()),
            tsp_name: None,
        }));
        assert_eq!(
            super::eu_qualified_words(&failed),
            "ETSI TS 119 615 clause 4.7 answers Qualified with PROCESS_FAILED (at the token's \
             generation time the answer was Not_Qualified) by the trusted list of AT"
        );
        assert!(
            super::eu_qualified_words(&EuQualified::TokenNotEstablished)
                .contains("only for a token that is valid")
        );
        let listed = ListedAuthority {
            territory: "AT".to_owned(),
            sequence_number: 42,
            provider: "A provider".to_owned(),
            service: "A stamp".to_owned(),
            service_type: TSA_QTST.to_owned(),
            status_at_generation: Some(
                "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted".to_owned(),
            ),
            authentication: Authentication::VouchedForByTheHost,
        };
        let said = super::listed_words(&failed, std::slice::from_ref(&listed));
        assert!(
            said.contains(
                "\"A stamp\" of \"A provider\" in the list of AT, of type \
                 http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST, \
                 http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted at the token's \
                 generation time"
            ),
            "{said}"
        );
        assert!(super::listed_words(&failed, &[]).contains("absent from every trusted list"));
        assert!(super::listed_words(&EuQualified::NoListSupplied, &[]).contains("was not asked"));
    }

    /// ETSI TS 119 615 clause 4.4's outputs are said in the clause's own indications, with the
    /// list they came from, and named as being about the certificate: no level of the signature
    /// ("QES", "qualified electronic signature") is worded from them (ADR 2835, ADR 2864). A host
    /// that supplied no list of the issuer's territory hears that, not `Not_Qualified`.
    #[test]
    fn a_certificates_qualification_is_said_in_the_clauses_indications_and_never_a_level() {
        use pdf_signature::trusted_list::determination::qscd::{Qscd, QscdIndication};
        use pdf_signature::trusted_list::determination::{
            Determination, Indication, Purpose, QcStatus, Qualification, SubStatus,
        };
        let determined = Qualification::Determined {
            territory: "AT".to_owned(),
            sequence_number: 42,
            determination: Determination {
                status: QcStatus::PassedWithWarning,
                sub_status: vec![SubStatus::NotEnoughInfoOnQcType(Purpose::ESig)],
                results: vec![
                    Indication::IndetQcForESig,
                    Indication::NotQualifiedForESeal,
                    Indication::NotQwac,
                ],
                sets_of_qe: [Vec::new(), Vec::new(), Vec::new()],
            },
            qscd: Qscd {
                status: QcStatus::Passed,
                sub_status: Vec::new(),
                result: Some(QscdIndication::Indeterminate),
            },
        };
        let said = super::qualification_sentence(&determined);
        for named in [
            "trusted list of AT (sequence number 42)",
            "INDET_QC_For_eSig, Not_Qualified_For_eSeal, Not_QWAC",
            "PROCESS_PASSED_WITH_WARNING: WARNING_T1_Not_Enough_Info_on_QC_Type",
            "clause 4.5 finds its key QSCD_INDETERMINATE (PROCESS_PASSED)",
            "not the signature's level",
        ] {
            assert!(said.contains(named), "{named} in {said}");
        }
        for level in ["QES", "qualified electronic signature"] {
            assert!(!said.contains(level), "{level} in {said}");
        }
        let absent = super::qualification_sentence(&Qualification::NoListForTerritory("AT".into()));
        assert!(absent.contains("No trusted list of AT"), "{absent}");
        assert!(!absent.contains("Not_Qualified"), "{absent}");
    }

    /// ETSI EN 319 102-1 clause 5.2.8.4.2.7 makes an unconstrained assertion available, so a
    /// claimed one is said with what it states; SAML 2.0 core section 2.3.3 forbids relying on one
    /// whose signature is invalid, so its contents are withheld and the reason said (ADR 2922).
    #[test]
    fn a_signers_saml_assertion_is_said_and_an_invalid_one_withheld() {
        use pdf_signature::saml::Saml;
        use pdf_signature::verdict::applicability::{AssertionKind, CarriedAssertion};
        let root = "<saml:Assertion xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\" \
                    ID=\"A1\" IssueInstant=\"2026-01-01T00:00:00Z\" Version=\"2.0\">\
                    <saml:Issuer>https://attributes.example</saml:Issuer>";
        let statement = "<saml:AttributeStatement><saml:Attribute FriendlyName=\"role\" \
                         Name=\"urn:oid:2.5.4.72\" \
                         NameFormat=\"urn:oasis:names:tc:SAML:2.0:attrname-format:uri\">\
                         <saml:AttributeValue>sales director</saml:AttributeValue>\
                         </saml:Attribute></saml:AttributeStatement></saml:Assertion>";
        let claimed = CarriedAssertion {
            kind: AssertionKind::Claimed,
            saml: Some(Saml::of(format!("{root}{statement}").as_bytes())),
            signer: None,
        };
        let said = super::assertion_sentence(1, &claimed);
        assert!(said.contains("clause 5.2.6.2"), "{said}");
        assert!(said.contains("https://attributes.example"), "{said}");
        assert!(
            said.contains("role (urn:oid:2.5.4.72) = sales director"),
            "{said}"
        );
        assert!(said.contains("not signed"), "{said}");
        // A signature with no `SignedInfo` is a shape XML Signature forbids: invalid, withheld.
        let broken = format!(
            "{root}<ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\">\
             </ds:Signature>{statement}"
        );
        let signed = CarriedAssertion {
            kind: AssertionKind::Signed,
            saml: Some(Saml::of(broken.as_bytes())),
            signer: None,
        };
        let said = super::assertion_sentence(2, &signed);
        assert!(said.contains("is invalid"), "{said}");
        assert!(!said.contains("sales director"), "{said}");
    }

    /// ETSI TS 119 172-4 clause 4.4.2's answers worded (ADR 2866): "qualified" about the signature
    /// only where REQ-4.4.2-06 holds, the requirement that decided every answer named, and Annex
    /// A's type and identifier only where clause 4.2's validation passed.
    #[test]
    fn a_level_is_worded_from_the_joins_own_output() {
        use pdf_signature::verdict::Level;
        use pdf_signature::verdict::level::{Answer, Purpose, Requirement};
        let qualified = super::level_sentence(Some(&Level {
            answer: Answer::Qualified(Purpose::Signature),
            decided_by: Requirement::Join,
            certificate: Some(Purpose::Signature),
            total_passed: true,
        }));
        for named in [
            "technically suitable to implement an EU qualified electronic signature (QES, \
             REQ-4.4.2-06)",
            "0.4.0.191724.1.2.1",
        ] {
            assert!(qualified.contains(named), "{named} in {qualified}");
        }
        let on_no_device = super::level_sentence(Some(&Level {
            answer: Answer::Indeterminate,
            decided_by: Requirement::Device,
            certificate: Some(Purpose::Seal),
            total_passed: true,
        }));
        for named in [
            "EU qualified certificate for electronic seals",
            "indeterminate",
            "REQ-4.4.2-05",
            "advanced electronic seal supported by an EU qualified certificate",
            "0.4.0.191724.1.2.5",
        ] {
            assert!(on_no_device.contains(named), "{named} in {on_no_device}");
        }
        assert!(!on_no_device.contains("suitable"), "{on_no_device}");
        let not_qualified = super::level_sentence(Some(&Level {
            answer: Answer::Indeterminate,
            decided_by: Requirement::Certificate,
            certificate: None,
            total_passed: false,
        }));
        assert!(not_qualified.contains("REQ-4.4.2-02"), "{not_qualified}");
        assert!(!not_qualified.contains("type"), "{not_qualified}");
        let joined_without_validation = super::level_sentence(Some(&Level {
            answer: Answer::NotQualified,
            decided_by: Requirement::Join,
            certificate: Some(Purpose::Signature),
            total_passed: false,
        }));
        assert!(
            joined_without_validation.contains("not determined as an EU qualified")
                && joined_without_validation.contains("REQ-4.4.2-06"),
            "{joined_without_validation}"
        );
        assert!(
            super::level_sentence(None).contains("was not determined"),
            "a listed path with no level claims nothing"
        );
    }

    /// Clause 9.5's two kinds of issue, for the test below: an authority's key outlived over its
    /// period, and a pair of families not compared.
    fn with_clause_9_5(
        assessment: &mut pdf_signature::verdict::applicability::ts_119_312::Assessment,
    ) {
        use pdf_signature::verdict::applicability::ts_119_312;
        use pdf_signature::x509::Instant;
        let ca = ts_119_312::KeyRole::CertificationAuthority { position: 1 };
        let rsa = pdf_signature::trust::KeyParameters::Rsa {
            modulus_bits: 2_048,
            pss: Some(true),
        };
        assessment.other_keys(&[ts_119_312::OtherKey {
            role: ca,
            key: rsa,
            until: Some(Instant::from_unix_seconds(1_798_761_600)),
        }]);
        assessment.certifications(&[ts_119_312::Certification {
            certifier: (
                ca,
                pdf_signature::trust::KeyParameters::Ecdsa(pdf_signature::ecdsa::Curve::P256),
            ),
            certified: (ts_119_312::KeyRole::Signer, rsa),
        }]);
    }

    /// A report holding one of each item, for the test below.
    fn applicability_report() -> pdf_signature::verdict::applicability::Report {
        use pdf_signature::cms::Digest;
        use pdf_signature::signature::Family;
        use pdf_signature::timestamp::Accuracy;
        use pdf_signature::trust::names::Name;
        use pdf_signature::verdict::Proof;
        use pdf_signature::verdict::applicability::{
            Assurance, ClaimedIn, Cryptography, EuQualified, Report, Scope, SignatureAttribute,
            Signer, TimeKind, Timing, ts_119_312,
        };
        use pdf_signature::x509::Instant;
        let subject = pdf_signature::trust::names::attributes_of(&[
            0x31, 0x0B, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x41, 0x0C, 0x02, 0x46, 0x34,
        ])
        .expect("a one-attribute name reads");
        Report {
            scope: Scope::EuQualifiedUsingTrustedLists,
            signer: Ok(Signer {
                subject,
                alternative_names: vec![Name::Rfc822("f4@example.org".to_owned())],
                pseudonym: Some("F4".to_owned()),
            }),
            times: vec![
                Timing {
                    kind: TimeKind::ClaimedSigningTime(ClaimedIn::SigningTimeAttribute),
                    at: Instant::from_unix_seconds(100),
                    assurance: Assurance::Claimed,
                },
                Timing {
                    kind: TimeKind::SignatureTimestamp,
                    at: Instant::from_unix_seconds(200),
                    assurance: Assurance::Token {
                        established: true,
                        policy: "1.2.3.4".to_owned(),
                        accuracy: Some(Accuracy {
                            seconds: 1,
                            millis: 0,
                            micros: 0,
                        }),
                        eu_qualified: EuQualified::NoListSupplied,
                        listed: Vec::new(),
                    },
                },
                Timing {
                    kind: TimeKind::BestSignatureTime,
                    at: Instant::from_unix_seconds(200),
                    assurance: Assurance::Best(Proof::ThisSignaturesTimestampAttribute),
                },
            ],
            attributes: vec![
                SignatureAttribute {
                    identifier: "1.2.840.113549.1.9.16.2.16".to_owned(),
                    name: Some("commitment-type-indication (ETSI EN 319 122-1)"),
                    signed: true,
                    commitment: Some((
                        "1.2.840.113549.1.9.16.6.1".to_owned(),
                        Some("proof of origin"),
                    )),
                    assertions: Vec::new(),
                },
                SignatureAttribute {
                    identifier: "1.2.3.5".to_owned(),
                    name: None,
                    signed: false,
                    commitment: None,
                    assertions: Vec::new(),
                },
            ],
            cryptography: Cryptography::Suite {
                digest: Digest::Sha256,
                family: Family::Rsa,
                key_bits: 2048,
            },
            // ETSI TS 119 312 at a best signature time in 2026 and a validation in 2027:
            // `RSA-PKCS#1v1_5` and `sha256-with-rsa` are a bare `L`, and a 2048-bit modulus is
            // its Table 6's `L[2026]`, passed by the validation.
            // Clause 9.3: a certificate valid until 2027-01-01 outlives Table 6's `L[2026]`.
            security: Some({
                let mut assessment = ts_119_312::Assessment::of(&ts_119_312::Asked {
                    digest: Digest::Sha256,
                    family: Family::Rsa,
                    key_bits: 2048,
                    subgroup_bits: None,
                    signed: Instant::from_unix_seconds(1_790_812_800),
                    now: Instant::from_unix_seconds(1_798_761_600),
                });
                assessment.signer_key(Some(Instant::from_unix_seconds(1_798_761_600)));
                with_clause_9_5(&mut assessment);
                assessment
            }),
        }
    }

    /// ETSI TS 119 172-4 REQ-4.5-01: each item of the report is a sentence naming it — a)'s scope
    /// in the clause's own words, b)'s subject and alternative names, c)'s pseudonym, d)'s times
    /// each with its assurance (a claim as a claim, a token's policy and accuracy, its EU
    /// qualification said as not determined), f)'s attributes by set with a commitment named, and
    /// h)'s suite with its potential security issues against ETSI TS 119 312 — a legacy algorithm
    /// and suite, a legacy modulus size, a phase-out passed by validation, and clause 9.3's key
    /// outlived by its certificate, and clause 9.5's authority's key outlived over its period and
    /// not compared across families with the signer's. REQ-4.2-03 e)'s
    /// warning names the suite and says INDETERMINATE, never invalid.
    #[test]
    fn the_applicability_report_says_each_item_of_req_4_5_01() {
        let report = applicability_report();
        let said = super::applicability_sentences(&report).join(". ");
        for named in [
            "REQ-4.5-01 a) names, in its words: \"Signature applicability rules checking \
             (validation rules) for European qualified electronic signatures/seals using trusted \
             lists\" — \"Validation of digital signature",
            "i.e. either Directive 1999/93/EC or Regulation (EU) No 910/2014\"",
            "the subject pseudonym=F4, and the subject alternative names rfc822Name f4@example.org \
             (REQ-4.5-01 b))",
            "The signer used a pseudonym, \"F4\"",
            "the claimed signing time 100 in its signingTime attribute (a claim by the signer, \
             which proves nothing)",
            "its signature timestamp's time 200 (established by the token's validation, under the \
             time-stamping policy 1.2.3.4, accurate to 1000000 microseconds; whether it is an EU \
             qualified time stamp",
            "the best signature time 200 (fixed by this signature's timestamp attribute)",
            "signed — commitment-type-indication (ETSI EN 319 122-1), committing to proof of \
             origin; unsigned — 1.2.3.5",
            "SHA256 under RSA (PKCS #1 v1.5) with a 2048-bit key (REQ-4.5-01 h)). Against ETSI TS \
             119 312 V2.1.1 (2026-06), its potential security issues: ",
            "Table 2, clause 6.2.1 lists its signature algorithm RSA-PKCS#1v1_5 as legacy, phased \
             out on 31 December 2034 unless a later release extends it",
            "Table 4, clause 7.3 lists its combination of digest and algorithm sha256-with-rsa as \
             legacy",
            "Table 6, clause 8.4 lists its modulus's size at least 1 900 and below 3 000 bits as \
             legacy, phased out no later than 31 December 2026, which had passed by this \
             validation, though not by the best signature time",
            "clause 9.3 asks the signer's key to remain suitable until its certificate's \
             notAfter, 1798761600 seconds after the epoch, and Table 6, clause 8.4 lists its \
             modulus's size at least 1 900 and below 3 000 bits as legacy, phased out no later \
             than 31 December 2026, which is before it",
            "clause 9.5 asks the key of the certification authority at position 1 of the path \
             to resist until 1798761600 seconds after the epoch, the latest notAfter of its \
             certificate and of those relying on it, and Table 6, clause 8.4 lists its modulus's \
             size at least 1 900 and below 3 000 bits as legacy",
            "clause 9.5 requires the key of the certification authority at position 1 of the \
             path to be at least as strong as the key of the signer it certifies, and the two are of families the \
             document gives no common measure for, so they were not compared",
        ] {
            assert!(said.contains(named), "{named} in {said}");
        }
        let warning = super::unsupported_suite_sentence("1.2.3");
        assert!(
            warning.contains("1.2.3, is not supported")
                && warning.contains("REQ-4.2-03 e)")
                && warning.contains("INDETERMINATE"),
            "{warning}"
        );
    }
}

#[cfg(test)]
#[path = "notes/timestamp_revocation_spec_audit.rs"]
mod timestamp_revocation_spec_audit;
