//! What a conversion does to a signed document: `doc/pdf-a-conversion-limits.md` section 3.6.
//!
//! # The assertion cannot cross, and the choice is what the output says about that
//!
//! A signature covers a byte range of one file — ISO 32000-2 §12.8.1's `/ByteRange` "should be
//! the entire PDF file, including the signature dictionary but excluding the signature value
//! itself" — and a conversion is a rewrite: every offset moves. So the cryptographic assertion a
//! signature makes is gone from the output whatever else the conversion does, and the only choice
//! left is between an output that *says so* and one that carries the signature dictionary into
//! bytes it no longer covers. ADR 1006 found this verb had been writing the second kind for as
//! long as it existed. This module is the first kind.
//!
//! What goes, and the sentence deciding each:
//!
//! - **Each signature field's value.** §12.7.5.5: "the field value ( V ), if present, shall be a
//!   signature dictionary containing the signature". The dictionary *is* the assertion, so it is
//!   what leaves. Its NOTE 2 says what signing touched — "at least the V entry and usually also
//!   the AP entry" — and only the first of those is undone: the field, its widget and its
//!   appearance stay, because the appearance "is there strictly for the purpose of providing a
//!   way for a human verifier to perform their own verification of the visual representation"
//!   and because ISO 19005-2 section 6.3.3 requires an annotation to carry one. §12.7.5.5 also
//!   forbids a processor to "incorporate the validation status of a signature … into the
//!   appearance", so an appearance kept after its signature is gone asserts nothing false.
//! - **The permissions dictionary's `DocMDP` and `UR3` entries.** Table 263 makes each one a
//!   signature: `DocMDP` has a processor "enforce the permissions specified by the P entry … and
//!   … also validate the corresponding signature", and `UR3`'s rights are granted "[i]f the
//!   signature is valid". Neither rests on anything once the signature is gone, and §12.8.2.3
//!   says of the second outright that a processor which modifies the document beyond the rights
//!   "should remove that signature prior to writing the newly modified PDF". The dictionary
//!   itself stays, empty where nothing else was in it: Table 263 makes every entry optional and
//!   an empty one states nothing.
//! - **Table 225's `AppendOnly` bit** of the form's `/SigFlags`: "[i]f set, the document contains
//!   signatures that may be invalidated if the PDF file is saved … in a way that alters its
//!   previous contents". The output contains none, so the flag is cleared; the
//!   `SignaturesExist` bit beside it stays, because the *fields* do.
//!
//! What stays, and why: the field's `/Lock` and `/SV` (Table 235), which constrain the *next*
//! signing rather than describe the last one; §12.8.4.3's `/DSS`, which holds certificates and
//! revocation data that name no byte of this file; §12.8.7's `/Legal` attestation, which counts
//! features of the document and is as true after the rewrite as before.
//!
//! # A signature is reported before it is removed, and against the source
//!
//! Section 3.6 asks that "[t]he report names each signature, its signer and whether it currently
//! validates". Each [`SourceSignature`] is that line: `/Name`, `/M`, and the answers to §12.8.1's
//! first two questions — [`pdf_model::signature::Signature::integrity`] and
//! [`Signature::authenticity`] — computed over the *source's* bytes, which is the only file the
//! signature was ever a statement about. Its `/ByteRange` coverage is stated the same way, so a
//! range the source got wrong is written into the report rather than dropped with the rest (ADR
//! 1003). Nothing here uses the word *valid*, for the reason `pdf_model::signature` gives: a
//! matching digest and a verifying value prove the file and the certificate belong together, and
//! say nothing about who the signer is.
//!
//! # The population, and the proof
//!
//! Three routes reach a signature dictionary and all three are walked: the form's field tree
//! (§12.8.1: an approval or certification signature "shall be the value of a signature field"),
//! every page's `/Annots` — for a widget carrying `/FT /Sig` and a signed `/V` outside any field
//! tree, which the corpus holds — and the catalog's `/Perms`. After the conversion the output is
//! walked the same three ways, and a signature still found refuses the file: a report that
//! promised the assertion was gone must not sit beside an output that still carries it.
//!
//! # A key nobody defines is not a signature
//!
//! ISO 19005-2 section 6.1.12 and ISO 19005-4 section 6.1.11 allow a permissions dictionary no
//! key but `UR3` and `DocMDP`. §12.8.6 says what any other key would be: "[e]ach entry in this
//! dictionary … shall specify the name of a permission handler", and Table 263 lists "the
//! currently defined entries", which are those two. A key naming a handler the standard does not
//! define is one no conforming processor of either edition can consult, so nothing any reader of
//! the file did ever turned on it, and removing it changes what no reader can see — a mechanical
//! rewrite, [`foreign_handlers`], rather than a loss. The report names the key.

use std::collections::BTreeSet;

use pdf_model::Pages;
use pdf_model::signature::{
    self, Authenticity, Coverage, Integrity, Modification, Signature, Signed,
};
use pdf_syntax::object::{Dictionary, Object, ObjectId};
use pdf_syntax::{Document, text_string};

use super::decision::Because;

/// How deep the field tree and the `/Parent` chain are followed.
///
/// §12.7.4.2's fully qualified name is built up the chain; a document controls how deep its own
/// tree is, so the walk is bounded rather than trusted.
const MAX_DEPTH: usize = 32;

/// Most signatures one conversion reports and removes.
///
/// Each one is hashed over the whole source, so a thousand of them is a thousand reads of the
/// file; a document past this many is refused with the count rather than converted in silence.
const MAX_SIGNATURES: usize = 256;

/// Why a signature field written as a direct object cannot be unsigned.
const FIELD_IS_DIRECT: &str = "a signature field is written directly into its parent's Kids or \
     the form's Fields array rather than as an object of its own, and this rewrite edits objects, \
     so its value cannot be removed without rewriting the array that holds it; §12.7.3's Table \
     224 makes Fields an array of references and §12.7.4.1 says the same of Kids, so this shape \
     is the producer's departure rather than a construction this converter reads. Not built";

/// Why a widget written as a direct object cannot be unsigned.
const WIDGET_IS_DIRECT: &str = "a widget carrying a signature is written directly into a page's \
     Annots array rather than as an object of its own, and this rewrite edits objects; §12.5.2's \
     Table 166 makes an annotation a dictionary a page's Annots array refers to. Not built";

/// Where a dictionary the rewrite edits stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Site {
    /// Written directly inside the catalog, so it is edited where the catalog is rewritten.
    InCatalog,
    /// An object of its own, edited when the walk reaches it.
    Object(ObjectId),
}

impl Site {
    /// The site of a catalog entry: its object where the entry refers to one, the catalog
    /// itself where the entry is written directly.
    fn of(entry: &Object) -> Self {
        entry.as_reference().map_or(Self::InCatalog, Self::Object)
    }
}

/// Which route reached a signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reached {
    /// The value of a field in the form's field tree (§12.8.1).
    Field,
    /// The value of a widget a page's `/Annots` names, outside any field tree.
    Widget,
    /// The permissions dictionary's `DocMDP` entry (Table 263), and no field's value.
    DocMdp,
    /// The permissions dictionary's `UR3` entry (Table 263), which §12.8.1 puts in no field.
    UsageRights,
}

impl Reached {
    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Field => "field",
            Self::Widget => "widget",
            Self::DocMdp => "doc-mdp",
            Self::UsageRights => "usage-rights",
        }
    }
}

/// One signature the source carries, worded for the report.
///
/// `doc/pdf-a-conversion-limits.md` section 3.6's line: who signed, when, and what verifying the
/// signature over the source found — all computed before the rewrite, because the source is the
/// only file the signature was ever about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSignature {
    /// Where the signature was reached.
    pub reached: Reached,
    /// What holds it: the field's fully qualified name (§12.7.4.2), or the permissions entry.
    pub at: String,
    /// `/Name`, "[t]he name of the person or authority signing the document".
    pub name: Option<String>,
    /// `/M`, the time of signing as the signer's clock wrote it.
    pub signed_at: Option<String>,
    /// `/Reason`, where the signer stated one.
    pub reason: Option<String>,
    /// `/Type /DocTimeStamp` (§12.8.5) rather than a signature.
    pub timestamp: bool,
    /// The changes a certification signature's `DocMDP` transform permitted, in words, where
    /// the permissions dictionary enforced one — the restriction the output no longer states.
    pub permitted: Option<String>,
    /// What `/ByteRange` covered of the source, in one sentence.
    pub coverage: String,
    /// §12.8.1's first question over the source, in one sentence: did the bytes change?
    pub integrity: String,
    /// §12.8.1's second question over the source, in one sentence: does the value verify under
    /// the certificate the file carries?
    pub authenticity: String,
}

/// Every signature the source carries, where each is, and what the rewrite has to reach.
#[derive(Debug, Default)]
pub(super) struct Signatures {
    /// Each signature, worded for the report, in the order the walk met them.
    pub(super) each: Vec<SourceSignature>,
    /// The field and widget objects whose `/V` goes.
    pub(super) values_at: BTreeSet<ObjectId>,
    /// The permissions dictionary, where it states `DocMDP` or `UR3`.
    pub(super) permissions: Option<Site>,
    /// The interactive form dictionary, where the catalog states one.
    pub(super) form: Option<Site>,
    /// Why the rewrite cannot reach every signature, where it cannot.
    ///
    /// Kept beside the list rather than replacing it, so that a refused document's report still
    /// names the signatures it carries.
    pub(super) obstacle: Option<Because>,
}

/// Every signature in `document`, each verified over the document's own bytes.
pub(super) fn find(document: &Document) -> Signatures {
    let mut found = Signatures::default();
    let Ok(catalog) = document.catalog() else {
        return found;
    };
    let mut walk = Walk {
        document,
        length: u64::try_from(document.bytes().len()).unwrap_or(u64::MAX),
        permissions: signature::permissions(document),
        seen_values: BTreeSet::new(),
        seen_fields: BTreeSet::new(),
    };
    walk.form(&catalog, &mut found);
    walk.annotations(&mut found);
    walk.permissions(&catalog, &mut found);
    found
}

/// Why a document with more signatures than [`MAX_SIGNATURES`] is refused.
const TOO_MANY_SIGNATURES: &str = "this document carries more signatures than this converter \
     reports and removes in one conversion; each is verified over the whole source before it \
     goes, and a document with hundreds of them is refused with the count rather than \
     half-reported";

/// Whether one more signature may be reported, recording the refusal where it may not.
///
/// Asked *before* the signature is verified, because the bound exists to cap the hashing: a
/// thousand signatures would be a thousand reads of the whole source.
fn room_for_another(found: &mut Signatures) -> bool {
    if found.each.len() < MAX_SIGNATURES {
        return true;
    }
    found
        .obstacle
        .get_or_insert(Because::NotBuiltYet(TOO_MANY_SIGNATURES));
    false
}

/// The three routes, and what they share: the file's length, its permissions, and which
/// objects have been met already.
struct Walk<'a> {
    /// The source.
    document: &'a Document,
    /// The source's length in bytes, which `/ByteRange` is measured against.
    length: u64,
    /// §12.8.6's `/Perms`, read once for the `DocMDP` level and the entries' own signatures.
    permissions: signature::Permissions,
    /// The signature dictionaries reported already, so one reached twice is named once.
    seen_values: BTreeSet<ObjectId>,
    /// The fields and widgets visited already, so a cycle ends and a merged dictionary that
    /// is both a field and an annotation is unsigned once.
    seen_fields: BTreeSet<ObjectId>,
}

impl Walk<'_> {
    /// §12.8.1's route: the form's field tree.
    fn form(&mut self, catalog: &Dictionary, found: &mut Signatures) {
        let Some(form) = catalog.get("AcroForm") else {
            return;
        };
        found.form = Some(Site::of(form));
        let resolved = self.document.resolve(form);
        let Some(fields) = resolved.as_dict().and_then(|form| {
            self.document
                .get_key(form, "Fields")
                .as_array()
                .map(<[Object]>::to_vec)
        }) else {
            return;
        };
        for field in &fields {
            self.descend(field, 0, found);
        }
    }

    /// One node of the field tree: the signature it holds, and its `/Kids`.
    fn descend(&mut self, entry: &Object, depth: usize, found: &mut Signatures) {
        if depth >= MAX_DEPTH {
            return;
        }
        let resolved = self.document.resolve(entry);
        let Some(dict) = resolved.as_dict() else {
            return;
        };
        if !self.visit(entry, dict, Reached::Field, FIELD_IS_DIRECT, found) {
            return;
        }
        if let Some(kids) = self
            .document
            .get_key(dict, "Kids")
            .as_array()
            .map(<[Object]>::to_vec)
        {
            for kid in &kids {
                self.descend(kid, depth.saturating_add(1), found);
            }
        }
    }

    /// The second route: every page's `/Annots`, for a signed widget in no field tree.
    fn annotations(&mut self, found: &mut Signatures) {
        let tree = Pages::new(self.document);
        for index in 0..tree.len() {
            let Some(page) = tree.get(index) else {
                continue;
            };
            let Some(annotations) = self
                .document
                .get_key(&page.dict, "Annots")
                .as_array()
                .map(<[Object]>::to_vec)
            else {
                continue;
            };
            for annotation in &annotations {
                let resolved = self.document.resolve(annotation);
                if let Some(dict) = resolved.as_dict() {
                    self.visit(annotation, dict, Reached::Widget, WIDGET_IS_DIRECT, found);
                }
            }
        }
    }

    /// One field or widget: whether it is new, and the signature it holds if any.
    ///
    /// `false` where the object was visited already, which ends a cycle and stops a merged
    /// dictionary from being reported twice.
    fn visit(
        &mut self,
        entry: &Object,
        dict: &Dictionary,
        reached: Reached,
        if_direct: &'static str,
        found: &mut Signatures,
    ) -> bool {
        let id = entry.as_reference();
        if let Some(id) = id
            && !self.seen_fields.insert(id)
        {
            return false;
        }
        if inherited_name(self.document, dict, "FT").as_deref() != Some("Sig") {
            return true;
        }
        let Some(value) = signed_value(self.document, dict) else {
            return true;
        };
        let Some(id) = id else {
            found
                .obstacle
                .get_or_insert(Because::NotBuiltYet(if_direct));
            return true;
        };
        found.values_at.insert(id);
        let reported_already = dict
            .get("V")
            .and_then(Object::as_reference)
            .is_some_and(|value_id| !self.seen_values.insert(value_id));
        if !reported_already && room_for_another(found) {
            found.each.push(describe(
                self.document,
                &value,
                reached,
                qualified_name(self.document, dict).unwrap_or_else(|| format!("object {id}")),
                &self.permissions,
                self.length,
            ));
        }
        true
    }

    /// The third route: the catalog's `/Perms`, whose two defined entries are signatures.
    fn permissions(&mut self, catalog: &Dictionary, found: &mut Signatures) {
        let Some(perms) = catalog.get("Perms") else {
            return;
        };
        let resolved = self.document.resolve(perms);
        let Some(dict) = resolved.as_dict() else {
            return;
        };
        let entries = [
            (
                "DocMDP",
                Reached::DocMdp,
                "the permissions dictionary's DocMDP entry",
            ),
            (
                "UR3",
                Reached::UsageRights,
                "the permissions dictionary's UR3 entry",
            ),
        ];
        for (key, reached, at) in entries {
            let Some(entry) = dict.get(key) else {
                continue;
            };
            found.permissions = Some(Site::of(perms));
            if let Some(id) = entry.as_reference()
                && !self.seen_values.insert(id)
            {
                // The same object a field already holds: reported once, at the field.
                continue;
            }
            if !room_for_another(found) {
                continue;
            }
            let value = self.document.resolve(entry);
            let read = value
                .as_dict()
                .and_then(|value| signature::read(self.document, value));
            found.each.push(match read {
                Some(value) => describe(
                    self.document,
                    &value,
                    reached,
                    at.to_owned(),
                    &self.permissions,
                    self.length,
                ),
                None => unsigned_entry(reached, at.to_owned()),
            });
        }
    }
}

/// The signature a field's own `/V` holds, where it holds one.
///
/// [`signature::read`] declines a dictionary stating neither `/ByteRange` nor `/Contents`, which
/// is a field prepared for signing and not signed: nothing to report and nothing to remove.
fn signed_value(document: &Document, field: &Dictionary) -> Option<Signature> {
    let value = document.get_key(field, "V");
    signature::read(document, value.as_dict()?)
}

/// A name entry, on this dictionary or inherited up its `/Parent` chain (§12.7.4.1).
fn inherited_name(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    let mut current = dict.clone();
    for _ in 0..MAX_DEPTH {
        if let Some(name) = document.get_key(&current, key).as_name() {
            return Some(String::from_utf8_lossy(name.as_bytes()).into_owned());
        }
        let parent = document.get_key(&current, "Parent");
        current = parent.as_dict()?.clone();
    }
    None
}

/// §12.7.4.2's fully qualified field name: each ancestor's `/T`, joined with a full stop.
fn qualified_name(document: &Document, field: &Dictionary) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = field.clone();
    for _ in 0..MAX_DEPTH {
        if let Object::String(bytes) = document.get_key(&current, "T") {
            parts.push(text_string(&bytes));
        }
        let parent = document.get_key(&current, "Parent");
        let Some(parent) = parent.as_dict() else {
            break;
        };
        current = parent.clone();
    }
    if parts.is_empty() {
        return None;
    }
    parts.reverse();
    Some(parts.join("."))
}

/// One signature's report line, computed over the source's bytes.
fn describe(
    document: &Document,
    value: &Signature,
    reached: Reached,
    at: String,
    permissions: &signature::Permissions,
    length: u64,
) -> SourceSignature {
    let file = document.bytes();
    let integrity = value.integrity(file);
    let authenticity = value.authenticity(file);
    let enforced = permissions
        .doc_mdp_signature
        .as_ref()
        .is_some_and(|enforced| enforced == value);
    let permitted = match (value.certification, enforced, permissions.doc_mdp) {
        (true, true, Some(level)) => Some(permitted_changes(level)),
        (true, true, None) => Some(
            "changes at a level the transform parameters do not state, \
                                   which Table 257 defaults to form filling and signing"
                .to_owned(),
        ),
        _ => None,
    };
    SourceSignature {
        reached,
        at,
        name: value.name.clone(),
        signed_at: value.signed_at.clone(),
        reason: value.reason.clone(),
        timestamp: value.timestamp,
        permitted,
        coverage: coverage_sentence(value, length),
        integrity: integrity_sentence(integrity),
        authenticity: authenticity_sentence(&authenticity, integrity),
    }
}

/// A permissions entry naming no signature: Table 263 makes both entries signature dictionaries,
/// and one stating neither `/ByteRange` nor `/Contents` is not one.
///
/// Reported all the same, because the entry goes with the rest — it enforced, or granted, on
/// the authority of a signature that was never there.
fn unsigned_entry(reached: Reached, at: String) -> SourceSignature {
    SourceSignature {
        reached,
        at,
        name: None,
        signed_at: None,
        reason: None,
        timestamp: false,
        permitted: None,
        coverage: "the entry names a dictionary stating neither ByteRange nor Contents, which \
                   is not a signature"
            .to_owned(),
        integrity: "there is nothing to hash".to_owned(),
        authenticity: "there is no value to verify".to_owned(),
    }
}

/// Table 257's `/P` in words: what the certification signature permitted after itself.
fn permitted_changes(level: Modification) -> String {
    match level {
        Modification::None => "no change at all".to_owned(),
        Modification::FormFilling => "form filling and signing".to_owned(),
        Modification::FormFillingAndAnnotation => {
            "form filling, signing, and annotation creation, deletion and modification".to_owned()
        }
        Modification::Unknown(level) => {
            format!("changes at a P of {level}, which Table 257 does not define")
        }
    }
}

/// What the `/ByteRange` covered of the source, in one sentence.
fn coverage_sentence(value: &Signature, length: u64) -> String {
    match value.coverage(length) {
        Coverage::WholeFile => "its ByteRange covers the whole of the source file".to_owned(),
        Coverage::Unsigned { tail } => format!(
            "its ByteRange stops {tail} bytes before the end of the source file, which is the \
             shape of a revision appended after signing (§7.5.6)"
        ),
        Coverage::Malformed => {
            let end = value
                .byte_range
                .iter()
                .map(|(start, size)| start.saturating_add(*size))
                .max()
                .unwrap_or(0);
            if end > length {
                format!(
                    "its ByteRange runs {} bytes past the end of the source file, so no digest \
                     over that file was computed with it",
                    end.saturating_sub(length)
                )
            } else {
                "its ByteRange is not a range of the source file's bytes that starts at the \
                 header"
                    .to_owned()
            }
        }
    }
}

/// §12.8.1's first question in words, over the source.
fn integrity_sentence(integrity: Integrity) -> String {
    match integrity {
        Integrity::Unchanged { digest } => format!(
            "the bytes it covers still hash to the {} digest it records",
            digest.name()
        ),
        Integrity::Changed { digest } => format!(
            "the bytes it covers no longer hash to the {} digest it records, so the source was \
             modified after it was signed",
            digest.name()
        ),
        Integrity::UnderTheSignersKey => {
            "it records no digest in the open, so whether the source changed is answered only \
             by the signature itself"
                .to_owned()
        }
        Integrity::UnknownDigest => {
            "it records a digest made with an algorithm this program does not compute".to_owned()
        }
        Integrity::RangeNotInThisFile => {
            "its ByteRange names bytes outside the source, so nothing was hashed".to_owned()
        }
        Integrity::RangeNotReadable => {
            "the source's bytes could not be read for the digest".to_owned()
        }
        Integrity::NoSignatureValue => "it states no Contents, which Table 255 requires".to_owned(),
        Integrity::Unreadable(error) => format!("its value could not be read: {error}"),
    }
}

/// §12.8.1's second question in words, over the source, worded against the first's answer.
///
/// The pairing `viewer_core::notes` makes and for the same reason: a signature that verifies
/// over attributes recording a digest the file no longer produces is a real signature whose
/// document was re-saved underneath it, and saying "verifies" alone would misstate it.
fn authenticity_sentence(authenticity: &Authenticity, integrity: Integrity) -> String {
    let certificate = "a certificate the source itself carries";
    match authenticity {
        Authenticity::Verified {
            key_bits,
            family,
            over: Signed::TheDocumentsBytes,
            ..
        } => format!(
            "it verifies under the {key_bits}-bit {} key in {certificate}, directly over the \
             bytes its ByteRange names",
            family.name()
        ),
        Authenticity::Verified {
            key_bits, family, ..
        } if matches!(integrity, Integrity::Changed { .. }) => format!(
            "it does verify under the {key_bits}-bit {} key in {certificate}, but what it signs \
             is the digest the source no longer produces",
            family.name()
        ),
        Authenticity::Verified {
            key_bits, family, ..
        } => format!(
            "it verifies under the {key_bits}-bit {} key in {certificate}",
            family.name()
        ),
        Authenticity::NotUnderThatKey {
            key_bits, family, ..
        } => format!(
            "it does not verify under the {key_bits}-bit {} key in {certificate}",
            family.name()
        ),
        Authenticity::NoSignerCertificate { certificates } => format!(
            "no certificate among the {certificates} the source carries is the signer's, so it \
             was not checked against a key"
        ),
        Authenticity::CertificateUnreadable(error) => {
            format!("the signer's certificate would not parse ({error}), so it was not checked")
        }
        Authenticity::KeyNotVerifiable { algorithm } => {
            format!("its key's algorithm ({algorithm}) is one this program does not verify under")
        }
        Authenticity::CurveNotVerifiable { curve } => {
            format!("its key is on a curve ({curve}) this program does not compute on")
        }
        Authenticity::AlgorithmNotVerifiable { algorithm } => {
            format!("its signature algorithm ({algorithm}) is one this program does not verify")
        }
        Authenticity::PssParametersNotVerifiable { statement } => {
            format!(
                "its RSASSA-PSS parameters are not ones this program verifies under: {statement}"
            )
        }
        Authenticity::KeyDoesNotMatchAlgorithm { algorithm, key } => format!(
            "its signature algorithm ({algorithm}) and its key's ({key}) are of two families, so \
             it was not checked"
        ),
        Authenticity::Refused(error) => {
            format!("it was not checked against the signer's key: {error}")
        }
        Authenticity::RefusedDsa(error) => {
            format!("it was not checked against the signer's key: {error}")
        }
        Authenticity::RefusedEcdsa(error) => {
            format!("it was not checked against the signer's key: {error}")
        }
        Authenticity::RefusedEdDsa(error) => {
            format!("it was not checked against the signer's key: {error}")
        }
        Authenticity::UnknownDigest { algorithm } => format!(
            "its digest algorithm ({algorithm}) is one this program does not compute, so it was \
             not checked against a key"
        ),
        Authenticity::NoSignatureValue => "there is no value to verify".to_owned(),
        Authenticity::RangeNotInThisFile => {
            "its ByteRange names bytes outside the source, so there was nothing to verify over"
                .to_owned()
        }
        Authenticity::RangeNotReadable => {
            "the source's bytes could not be read for the verification".to_owned()
        }
        Authenticity::Unreadable(error) => {
            format!("its value could not be read as a CMS object: {error}")
        }
    }
}

/// Every signature `document` still carries, each named — the proof a converted file is held to.
///
/// The same three routes as [`find`], without the verification: what is asked here is whether
/// anything is left, and a name is enough to refuse the file with.
pub(super) fn remaining(document: &Document) -> Vec<String> {
    find_without_verifying(document)
}

/// [`find`]'s walk, reporting names only.
fn find_without_verifying(document: &Document) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let mut visit = |dict: &Dictionary, id: Option<ObjectId>, out: &mut Vec<String>| {
        if let Some(id) = id
            && !seen.insert(id)
        {
            return;
        }
        if inherited_name(document, dict, "FT").as_deref() == Some("Sig")
            && signed_value(document, dict).is_some()
        {
            out.push(qualified_name(document, dict).unwrap_or_else(|| {
                id.map_or_else(
                    || "an unnamed field".to_owned(),
                    |id| format!("object {id}"),
                )
            }));
        }
    };
    let form = document.get_key(&catalog, "AcroForm");
    if let Some(fields) = form.as_dict().and_then(|form| {
        document
            .get_key(form, "Fields")
            .as_array()
            .map(<[Object]>::to_vec)
    }) {
        let mut queue: Vec<(Object, usize)> = fields.into_iter().map(|f| (f, 0)).collect();
        while let Some((entry, depth)) = queue.pop() {
            if depth >= MAX_DEPTH {
                continue;
            }
            let resolved = document.resolve(&entry);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            visit(dict, entry.as_reference(), &mut out);
            if let Some(kids) = document
                .get_key(dict, "Kids")
                .as_array()
                .map(<[Object]>::to_vec)
            {
                queue.extend(kids.into_iter().map(|kid| (kid, depth.saturating_add(1))));
            }
        }
    }
    let tree = Pages::new(document);
    for index in 0..tree.len() {
        let Some(page) = tree.get(index) else {
            continue;
        };
        let Some(annotations) = document
            .get_key(&page.dict, "Annots")
            .as_array()
            .map(<[Object]>::to_vec)
        else {
            continue;
        };
        for annotation in &annotations {
            let resolved = document.resolve(annotation);
            if let Some(dict) = resolved.as_dict() {
                visit(dict, annotation.as_reference(), &mut out);
            }
        }
    }
    let perms = document.get_key(&catalog, "Perms");
    if let Some(perms) = perms.as_dict() {
        for key in ["DocMDP", "UR3"] {
            if perms.get(key).is_some() {
                out.push(format!("the permissions dictionary's {key} entry"));
            }
        }
    }
    out
}

/// The keys a permissions dictionary states that ISO 19005 does not allow, and where they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ForeignHandlers {
    /// The permissions dictionary.
    pub(super) at: Site,
    /// Every key other than `UR3` and `DocMDP`, as the file spells it.
    pub(super) keys: Vec<String>,
}

/// ISO 19005-2 section 6.1.12 and ISO 19005-4 section 6.1.11: the permissions dictionary's
/// keys outside Table 263, which go.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where the catalog states no permissions dictionary this rewrite can
/// edit, which a document failing the requirement cannot be.
pub(super) fn foreign_handlers(document: &Document) -> Result<ForeignHandlers, Because> {
    let catalog = document
        .catalog()
        .map_err(|_| Because::NotBuiltYet(NO_PERMISSIONS))?;
    let entry = catalog
        .get("Perms")
        .ok_or(Because::NotBuiltYet(NO_PERMISSIONS))?;
    let resolved = document.resolve(entry);
    let dict = resolved
        .as_dict()
        .ok_or(Because::NotBuiltYet(NO_PERMISSIONS))?;
    let keys: Vec<String> = dict
        .iter()
        .map(|(key, _)| String::from_utf8_lossy(key.as_bytes()).into_owned())
        .filter(|key| key != "UR3" && key != "DocMDP")
        .collect();
    Ok(ForeignHandlers {
        at: Site::of(entry),
        keys,
    })
}

/// Why a permissions rewrite finds nothing to edit.
const NO_PERMISSIONS: &str = "the catalog states no permissions dictionary this rewrite can \
     reach, which a document failing the requirement cannot be; the finding and the file disagree";

#[cfg(test)]
mod tests {
    use super::*;

    /// A document whose form holds one signed field, one prepared field, and a `/Perms`
    /// naming the same signature as `DocMDP`.
    fn signed_form() -> Vec<u8> {
        let objects: Vec<&[u8]> = vec![
            b"<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R /Perms << /DocMDP 7 0 R >> >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [6 0 R 8 0 R] >>",
            b"<< /Length 0 >>\nstream\n\nendstream",
            b"<< /Fields [6 0 R 8 0 R] /SigFlags 3 >>",
            b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (Signed) /Rect [0 0 0 0] /V 7 0 R >>",
            b"<< /Type /Sig /ByteRange [0 10 20 100000] /Contents <00> /Name (A Signer) \
              /M (D:20260912) /Reference [<< /TransformMethod /DocMDP \
              /TransformParams << /P 2 >> >>] >>",
            b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (Prepared) /Rect [0 0 0 0] >>",
        ];
        build(&objects)
    }

    fn build(objects: &[&[u8]]) -> Vec<u8> {
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let start = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", objects.len().saturating_add(1)).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
                objects.len().saturating_add(1)
            )
            .as_bytes(),
        );
        out
    }

    #[test]
    fn a_signed_field_is_found_once_though_the_permissions_dictionary_names_it_too() {
        let document = Document::open(signed_form()).expect("the fixture opens");
        let found = find(&document);
        assert_eq!(found.each.len(), 1, "one signature, reached by two routes");
        let signature = &found.each[0];
        assert_eq!(signature.reached, Reached::Field);
        assert_eq!(signature.at, "Signed");
        assert_eq!(signature.name.as_deref(), Some("A Signer"));
        assert_eq!(signature.signed_at.as_deref(), Some("D:20260912"));
        assert_eq!(
            signature.permitted.as_deref(),
            Some("form filling and signing"),
            "the DocMDP entry enforced Table 257's P 2"
        );
        assert!(
            signature
                .coverage
                .contains("past the end of the source file"),
            "a range the source got wrong is stated: {}",
            signature.coverage
        );
        assert_eq!(
            found.values_at.len(),
            1,
            "the prepared field has nothing to remove"
        );
        assert_eq!(found.permissions, Some(Site::InCatalog));
        assert_eq!(found.form, Some(Site::Object(ObjectId::new(5, 0))));
        assert_eq!(found.obstacle, None);
    }

    #[test]
    fn a_document_with_no_signature_finds_none() {
        let objects: Vec<&[u8]> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [] /Count 0 >>",
        ];
        let document = Document::open(build(&objects)).expect("the fixture opens");
        let found = find(&document);
        assert!(found.each.is_empty());
        assert!(remaining(&document).is_empty());
    }

    #[test]
    fn what_remains_names_every_route() {
        let document = Document::open(signed_form()).expect("the fixture opens");
        let left = remaining(&document);
        assert_eq!(
            left,
            vec![
                "Signed".to_owned(),
                "the permissions dictionary's DocMDP entry".to_owned()
            ]
        );
    }

    #[test]
    fn a_key_the_standard_does_not_define_is_named_and_the_defined_ones_are_not() {
        let objects: Vec<&[u8]> = vec![
            b"<< /Type /Catalog /Pages 2 0 R /Perms 3 0 R >>",
            b"<< /Type /Pages /Kids [] /Count 0 >>",
            b"<< /XX (value) /DocMDP << >> >>",
        ];
        let document = Document::open(build(&objects)).expect("the fixture opens");
        let handlers = foreign_handlers(&document).expect("a permissions dictionary is stated");
        assert_eq!(handlers.at, Site::Object(ObjectId::new(3, 0)));
        assert_eq!(handlers.keys, vec!["XX".to_owned()]);
    }
}
