//! ISO 32000-2 §12.8's digital signatures, as far as a program without a trust store can go.
//!
//! §12.8.1 divides the subject for us. Verifying a signature has two halves, and the clause
//! states them in order:
//!
//! > To verify the signature, an appropriate signature handler is required. … The signer's
//! > certificate shall be determined and verified by the signature handler to match with any of
//! > the validation parameters and other conditions. If the verification fails, the signature
//! > shall be considered invalid. The digest shall be recomputed and compared with the one stored
//! > in the document. Differences between the two indicates that modifications have been made
//! > since the document was signed and thus the signature shall be considered invalid.
//!
//! The first half — a certificate, a chain, a revocation check — is a public-key infrastructure
//! and a threat model, which §7.6.5's public-key security handlers are already refused for (ADR
//! 0031). The second half is a digest over bytes this program has in hand.
//!
//! # What is here: everything a signature says, and one thing it can be *checked* against
//!
//! [`Signature`] is Table 255. [`Permissions`] is §12.8.6's `/Perms` with §12.8.2.2's `/P`
//! level, which is what a document says may be changed without invalidating its author's
//! signature.
//!
//! And [`Signature::coverage`] is the check that needs no cryptography at all. §12.8.1 says what
//! a byte range digest covers:
//!
//! > This range should be the entire PDF file, including the signature dictionary but excluding
//! > the signature value itself (the Contents entry).
//!
//! A `/ByteRange` that stops short of the end of the file therefore names bytes **nobody signed**
//! — an incremental update appended after signing — and saying so costs one comparison against
//! the file's length. It is not a validity verdict and this module never calls it one: a
//! signature whose range covers everything may still be forged, and one that does not may be a
//! perfectly honest later revision. What it is, is the one statement about a signature that a
//! renderer can make on its own evidence.
//!
//! # What is deliberately not here
//!
//! No digest is computed and no `/Contents` is parsed. Computing the digest would be honest and
//! useless without the certificate half — a hash that matches proves the bytes are the bytes the
//! *stored* hash was made from, and nothing about who made it — and it would invite a caller to
//! read "digest matches" as "signature valid", which is the failure mode this module is shaped
//! to avoid. 8 of the 974 corpus documents carry a `/ByteRange`, and 7 a signature dictionary.

use pdf_syntax::{Dictionary, Document, Object};

/// Most signatures read from one document.
///
/// §12.8.1 permits "[o]ne or more approval signatures" and any number of timestamps, so the
/// bound is on a file built to make a reader work rather than on any real workflow.
const MAX_SIGNATURES: usize = 1024;

/// One signature dictionary. Table 255.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// `/Type`: `Sig`, or `DocTimeStamp` for §12.8.5's document timestamp.
    ///
    /// Table 255 makes the entry optional for a signature and required for a timestamp, and
    /// gives the default as `Sig` — so a dictionary stating nothing is a signature.
    pub timestamp: bool,
    /// `/Filter`, "[t]he name of the preferred signature handler to use when validating this
    /// signature".
    pub handler: Option<String>,
    /// `/SubFilter`, the encoding of the signature value — `adbe.pkcs7.detached`,
    /// `ETSI.CAdES.detached`, `ETSI.RFC3161`.
    ///
    /// Read because it decides which of §12.8.3's profiles applies, and because two of its
    /// values tighten `/ByteRange` from a *should* into a *shall*: with `ETSI.CAdES.detached` or
    /// `ETSI.RFC3161` it "shall cover the entire PDF file". [`Signature::must_cover_whole_file`]
    /// is that distinction.
    pub sub_filter: Option<String>,
    /// `/ByteRange`, "an array of pairs of integers (starting byte offset, length in bytes)".
    pub byte_range: Vec<(u64, u64)>,
    /// `/Name`, "[t]he name of the person or authority signing the document".
    pub name: Option<String>,
    /// `/M`, the time of signing, as the §7.9.4 date string the file wrote.
    pub signed_at: Option<String>,
    /// `/Location`, `/Reason` and `/ContactInfo` — the three text strings a signer states about
    /// *why*, kept in the clause's own order.
    pub location: Option<String>,
    /// The reason for signing.
    pub reason: Option<String>,
    /// How to reach the signer.
    pub contact: Option<String>,
    /// `/Changes`: "an array of three integers … the number of pages altered, the number of
    /// fields altered, and the number of fields filled in".
    pub changes: Option<[i64; 3]>,
    /// Whether `/Reference` names a signature reference dictionary with a `DocMDP` transform.
    ///
    /// §12.8.1 makes this what a *certification* signature is: its dictionary "shall contain a
    /// signature reference dictionary … that has a `DocMDP` transform method".
    pub certification: bool,
}

/// What a signature's `/ByteRange` covers, measured against the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// The range starts at the beginning of the file and runs to its end, with one gap — which
    /// is where `/Contents` sits, and which §12.8.1 requires to be excluded.
    WholeFile,
    /// The range stops short: this many bytes at the end of the file are outside it.
    ///
    /// An incremental update applied after signing, in the ordinary case (§7.5.6), which
    /// §12.8.1's NOTE 1 describes as the mechanism that keeps a signature meaningful. Whether
    /// those bytes are a permitted change is §12.8.2.2's question and needs the digest.
    Unsigned {
        /// How many bytes at the end of the file the range does not include.
        tail: u64,
    },
    /// The range is not two ascending pairs covering a prefix and a suffix — so what it names
    /// cannot be compared with the file at all.
    Malformed,
}

impl Signature {
    /// Whether `/SubFilter` makes whole-file coverage a requirement rather than a recommendation.
    ///
    /// Table 255 says that for those two sub-filters "the `ByteRange` shall cover the entire PDF
    /// file". For every other value §12.8.1's "should" stands, and a range that stops short is a
    /// document with a later revision rather than a defective signature.
    #[must_use]
    pub fn must_cover_whole_file(&self) -> bool {
        matches!(
            self.sub_filter.as_deref(),
            Some("ETSI.CAdES.detached" | "ETSI.RFC3161")
        )
    }

    /// What this signature's range covers of a file `length` bytes long.
    ///
    /// The arithmetic is the whole of it: the first pair must start at zero, the pairs must
    /// ascend without overlapping, and the last must end at the file's end. Anything else is
    /// [`Coverage::Unsigned`] with the size of the tail, or [`Coverage::Malformed`].
    #[must_use]
    pub fn coverage(&self, length: u64) -> Coverage {
        let [(first_start, first_length), rest @ ..] = self.byte_range.as_slice() else {
            return Coverage::Malformed;
        };
        if *first_start != 0 {
            // §12.8.1: the range starts "from the \"%PDF-\" comment at the beginning of the PDF
            // document". A range that starts anywhere else has not signed the header.
            return Coverage::Malformed;
        }
        let mut end = first_start.saturating_add(*first_length);
        for (start, size) in rest {
            if *start < end {
                return Coverage::Malformed;
            }
            end = start.saturating_add(*size);
        }
        if end > length {
            return Coverage::Malformed;
        }
        if end == length {
            Coverage::WholeFile
        } else {
            Coverage::Unsigned {
                tail: length.saturating_sub(end),
            }
        }
    }
}

/// §12.8.6's permissions dictionary. Table 263.
///
/// > These permissions are similar to those defined by security handlers … but do not require
/// > that the document be encrypted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Permissions {
    /// `/DocMDP`'s signature, and the `/P` level its transform parameters state.
    pub doc_mdp: Option<Modification>,
    /// Whether `/UR3` is present: a usage rights signature, deprecated in PDF 2.0.
    ///
    /// Read as a flag rather than as a signature, because what it grants is "the enabling of
    /// features of a PDF processor that are not available by default" — a permission about
    /// *this program*, which this program does not have a way to be asked for.
    pub usage_rights: bool,
}

/// §12.8.2.2's `/P`: which changes the author's signature survives. Table 257.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modification {
    /// 1 — "the document shall be final; that is, any changes shall invalidate the signature",
    /// with the exception of later DSS and document-timestamp updates.
    None,
    /// 2 — form filling and signing, the first of the two "modifications that are appropriate
    /// for form field or comment workflows".
    FormFilling,
    /// 3 — form filling, signing and annotation.
    FormFillingAndAnnotation,
    /// A `/P` outside 1..=3, which Table 257 does not define.
    Unknown(i64),
}

/// Every signature the document's interactive form holds, in `/Fields` order.
///
/// §12.8.1 puts them there — a certification or approval signature "shall be the value of a
/// signature field" — so this walks `/AcroForm /Fields` rather than the whole object graph, and
/// a signature reachable no other way is one no field claims.
#[must_use]
pub fn signatures(document: &Document) -> Vec<Signature> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return Vec::new();
    };
    let fields = document.get_key(form, "Fields");
    let Some(fields) = fields.as_array().map(<[Object]>::to_vec) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for field in &fields {
        collect(document, field, &mut out, &mut seen, 0);
    }
    out
}

/// One level of the field walk, gathering the `/V` of every signature field.
fn collect(
    document: &Document,
    field: &Object,
    out: &mut Vec<Signature>,
    seen: &mut std::collections::BTreeSet<pdf_syntax::ObjectId>,
    depth: usize,
) {
    if out.len() >= MAX_SIGNATURES || depth > 32 {
        return;
    }
    if let Some(id) = field.as_reference()
        && !seen.insert(id)
    {
        return;
    }
    let resolved = document.resolve(field);
    let Some(dict) = resolved.as_dict() else {
        return;
    };
    if document
        .get_key(dict, "FT")
        .as_name()
        .is_some_and(|kind| kind.as_bytes() == b"Sig")
        && let Some(value) = document.get_key(dict, "V").as_dict()
        && let Some(signature) = read(document, value)
    {
        out.push(signature);
    }
    if let Some(kids) = document
        .get_key(dict, "Kids")
        .as_array()
        .map(<[Object]>::to_vec)
    {
        for kid in &kids {
            collect(document, kid, out, seen, depth.saturating_add(1));
        }
    }
}

/// Reads one Table 255 signature dictionary.
///
/// `None` where the dictionary states no `/ByteRange` *and* no `/Contents`, which is a field
/// that has been prepared for a signature and not signed — the common shape in an unsigned form.
#[must_use]
pub fn read(document: &Document, dict: &Dictionary) -> Option<Signature> {
    if dict.get("ByteRange").is_none() && dict.get("Contents").is_none() {
        return None;
    }
    let text = |key: &str| match document.get_key(dict, key) {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    };
    let name = |key: &str| {
        document
            .get_key(dict, key)
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    };
    Some(Signature {
        timestamp: name("Type").as_deref() == Some("DocTimeStamp"),
        handler: name("Filter"),
        sub_filter: name("SubFilter"),
        byte_range: byte_range(document, dict),
        name: text("Name"),
        signed_at: text("M"),
        location: text("Location"),
        reason: text("Reason"),
        contact: text("ContactInfo"),
        changes: changes(document, dict),
        certification: has_transform(document, dict, b"DocMDP"),
    })
}

/// §12.8.6's `/Perms`, with the `/P` level behind its `/DocMDP`.
#[must_use]
pub fn permissions(document: &Document) -> Permissions {
    let Ok(catalog) = document.catalog() else {
        return Permissions::default();
    };
    let perms = document.get_key(&catalog, "Perms");
    let Some(perms) = perms.as_dict() else {
        return Permissions::default();
    };
    Permissions {
        doc_mdp: document
            .get_key(perms, "DocMDP")
            .as_dict()
            .and_then(|signature| modification(document, signature)),
        usage_rights: !matches!(document.get_key(perms, "UR3"), Object::Null),
    }
}

/// Table 257's `/P`, found through the signature's `/Reference` chain.
fn modification(document: &Document, signature: &Dictionary) -> Option<Modification> {
    let references = document.get_key(signature, "Reference");
    let references = references.as_array()?.to_vec();
    for reference in &references {
        let resolved = document.resolve(reference);
        let Some(reference) = resolved.as_dict() else {
            continue;
        };
        let is_doc_mdp = document
            .get_key(reference, "TransformMethod")
            .as_name()
            .is_some_and(|method| method.as_bytes() == b"DocMDP");
        if !is_doc_mdp {
            continue;
        }
        let parameters = document.get_key(reference, "TransformParams");
        let level = parameters
            .as_dict()
            .and_then(|parameters| document.get_key(parameters, "P").as_integer())
            // Table 257 gives `/P` a default of 2.
            .unwrap_or(2);
        return Some(match level {
            1 => Modification::None,
            2 => Modification::FormFilling,
            3 => Modification::FormFillingAndAnnotation,
            other => Modification::Unknown(other),
        });
    }
    None
}

/// Whether the signature's `/Reference` names a transform method.
fn has_transform(document: &Document, signature: &Dictionary, method: &[u8]) -> bool {
    let references = document.get_key(signature, "Reference");
    let Some(references) = references.as_array().map(<[Object]>::to_vec) else {
        return false;
    };
    references.iter().any(|reference| {
        document
            .resolve(reference)
            .as_dict()
            .and_then(|reference| {
                document
                    .get_key(reference, "TransformMethod")
                    .as_name()
                    .map(|name| name.as_bytes() == method)
            })
            .unwrap_or(false)
    })
}

/// Table 255's `/ByteRange`, as the pairs the clause states.
fn byte_range(document: &Document, dict: &Dictionary) -> Vec<(u64, u64)> {
    let range = document.get_key(dict, "ByteRange");
    let Some(range) = range.as_array() else {
        return Vec::new();
    };
    range
        .chunks_exact(2)
        .filter_map(|pair| {
            let start = document.resolve(pair.first()?).as_integer()?;
            let length = document.resolve(pair.get(1)?).as_integer()?;
            Some((u64::try_from(start).ok()?, u64::try_from(length).ok()?))
        })
        .collect()
}

/// Table 255's `/Changes`, "an array of three integers".
fn changes(document: &Document, dict: &Dictionary) -> Option<[i64; 3]> {
    let changes = document.get_key(dict, "Changes");
    let changes = changes.as_array()?;
    let [pages, altered, filled, ..] = changes else {
        return None;
    };
    Some([
        document.resolve(pages).as_integer()?,
        document.resolve(altered).as_integer()?,
        document.resolve(filled).as_integer()?,
    ])
}

/// §12.8.4.3's document security store: the material a later validation would need. Table 261.
///
/// The clause's own list of what it holds is a list of *certificates*, and none of it is this
/// program's to interpret: an array of certificates, "an array of all Certificate Revocation
/// Lists (CRL) (see Internet RFC 5280 )", an array of OCSP responses (RFC 6960), and a `/VRI`
/// map keyed by "the base-16-encoded (uppercase) SHA-1 digest of the signature to which it
/// applies".
///
/// So this counts them and stops. Parsing a certificate is X.509 and a trust decision, which is
/// [`Signature`]'s refusal; counting them says the one thing a person might want from a program
/// that cannot validate — **whether the document carries what a validator would need**, which is
/// the whole point of §12.8.4's "long term validation".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SecurityStore {
    /// How many certificates `/Certs` holds.
    pub certificates: usize,
    /// How many certificate revocation lists `/CRLs` holds.
    pub revocation_lists: usize,
    /// How many OCSP responses `/OCSPs` holds.
    pub ocsp_responses: usize,
    /// How many signatures `/VRI` carries validation information for.
    ///
    /// One entry per signature "that a given signature handler or PDF processor has used to
    /// successfully validate the given signature" — the clause is explicit that a VRI records
    /// only successes: "[a] signature VRI dictionary shall not be used to record the information
    /// used in an unsuccessful validation attempt."
    pub validated_signatures: usize,
}

impl SecurityStore {
    /// Whether the document carries any validation material at all.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.certificates == 0
            && self.revocation_lists == 0
            && self.ocsp_responses == 0
            && self.validated_signatures == 0
    }
}

/// §12.8.4.3's `/DSS`, counted.
#[must_use]
pub fn security_store(document: &Document) -> SecurityStore {
    let Ok(catalog) = document.catalog() else {
        return SecurityStore::default();
    };
    let dss = document.get_key(&catalog, "DSS");
    let Some(dss) = dss.as_dict() else {
        return SecurityStore::default();
    };
    let count = |key: &str| {
        document
            .get_key(dss, key)
            .as_array()
            .map_or(0, <[Object]>::len)
    };
    SecurityStore {
        certificates: count("Certs"),
        revocation_lists: count("CRLs"),
        ocsp_responses: count("OCSPs"),
        validated_signatures: document.get_key(dss, "VRI").as_dict().map_or(0, |vri| {
            vri.iter()
                .filter(|(key, _)| key.as_bytes() != b"Type")
                .count()
        }),
    }
}

/// §12.8.7's legal attestation dictionary: what the author says is in the document. Table 264.
///
/// The clause exists because a PDF can lie about itself:
///
/// > The PDF language provides a number of capabilities that can make the rendered appearance of
/// > a PDF document vary. These capabilities could potentially be used to construct a document
/// > that misleads the recipient of a document, intentionally or unintentionally.
///
/// So an author certifying a document is asked to declare the counts of the things that could —
/// scripts, launch actions, alternate images, external streams — and a reviewer can weigh them.
///
/// **This is a document stating a fact this program can check.** [`Legal::disagreements`] counts
/// the same things over the object graph and names every entry where the two differ, which is the
/// habit §12.3.3's `/Count`, an LZW stream's length and §12.4.3's bead arrays all taught: a file
/// that says the same thing twice can be held to it. A disagreement is not proof of anything —
/// the clause states no algorithm for counting, and an author's tool may count a shared action
/// once where this one counts two references — but it is exactly the kind of thing "any
/// questionable content can be reviewed in the context of" means.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Legal {
    /// The counts the dictionary states, by their Table 264 key.
    pub stated: Vec<(String, i64)>,
    /// `/Attestation`, the author's own words about the entries above.
    pub attestation: Option<String>,
}

impl Legal {
    /// Entries whose stated count differs from what this reader finds, as `(key, stated, found)`.
    ///
    /// Only the entries a renderer can count on its own evidence are checked — the action types,
    /// which `action.rs` already names, and the external streams §7.3.8.1 refuses. The rest of
    /// Table 264 is counted by nobody here and is left out rather than guessed at.
    #[must_use]
    pub fn disagreements(&self, document: &Document) -> Vec<(String, i64, i64)> {
        let found = census(document);
        self.stated
            .iter()
            .filter_map(|(key, stated)| {
                let counted = found.iter().find(|(name, _)| name == key)?.1;
                (counted != *stated).then(|| (key.clone(), *stated, counted))
            })
            .collect()
    }
}

/// §12.8.7's `/Legal`, where the catalog states one.
#[must_use]
pub fn legal(document: &Document) -> Option<Legal> {
    let catalog = document.catalog().ok()?;
    let legal = document.get_key(&catalog, "Legal");
    let legal = legal.as_dict()?;
    Some(Legal {
        stated: legal
            .iter()
            .filter_map(|(key, value)| {
                Some((
                    String::from_utf8_lossy(key.as_bytes()).into_owned(),
                    document.resolve(value).as_integer()?,
                ))
            })
            .collect(),
        attestation: match document.get_key(legal, "Attestation") {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        },
    })
}

/// Counts the Table 264 entries this program can count, over every object in the document.
///
/// A whole-object walk, which is why nothing calls it while a page is being drawn: this answers a
/// question about the *document* and is asked once, by somebody who wants to know whether its
/// author's declaration holds.
fn census(document: &Document) -> Vec<(String, i64)> {
    // Every key this program can count, starting at zero — because a *missing* count and a count
    // of zero are different answers: the first means "not countable here" and is left out of the
    // comparison, the second means "the author declared some and there are none".
    let mut counts: std::collections::BTreeMap<&'static str, i64> = [
        "JavaScriptActions",
        "LaunchActions",
        "URIActions",
        "MovieActions",
        "SoundActions",
        "HideAnnotationActions",
        "GoToRemoteActions",
        "AlternateImages",
        "ExternalStreams",
        "TrueTypeFonts",
    ]
    .into_iter()
    .map(|key| (key, 0))
    .collect();
    for number in document.xref().object_numbers() {
        let object = document.get(pdf_syntax::ObjectId {
            number,
            generation: 0,
        });
        let dict = match &object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        if let Some(action) = dict.get("S").and_then(Object::as_name) {
            let key = match action.as_bytes() {
                b"JavaScript" => Some("JavaScriptActions"),
                b"Launch" => Some("LaunchActions"),
                b"URI" => Some("URIActions"),
                b"Movie" => Some("MovieActions"),
                b"Sound" => Some("SoundActions"),
                b"Hide" => Some("HideAnnotationActions"),
                b"GoToR" => Some("GoToRemoteActions"),
                _ => None,
            };
            if let Some(key) = key {
                let slot = counts.entry(key).or_default();
                *slot = slot.saturating_add(1);
            }
        }
        if dict.get("Alternates").is_some() {
            let slot = counts.entry("AlternateImages").or_default();
            *slot = slot.saturating_add(1);
        }
        // §7.3.8.1's external stream: the bytes are in a file rather than in the document, which
        // this tree refuses by name — and which Table 264 counts for the same reason it exists.
        if matches!(&object, Object::Stream(_)) && dict.get("F").is_some() {
            let slot = counts.entry("ExternalStreams").or_default();
            *slot = slot.saturating_add(1);
        }
        if dict
            .get("Subtype")
            .and_then(Object::as_name)
            .is_some_and(|subtype| subtype.as_bytes() == b"TrueType")
        {
            let slot = counts.entry("TrueTypeFonts").or_default();
            *slot = slot.saturating_add(1);
        }
    }
    counts
        .into_iter()
        .map(|(key, count)| (key.to_owned(), count))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Coverage, Modification, legal, permissions, security_store, signatures};
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1.
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

    /// A certification signature, its permissions, and what its range covers.
    #[test]
    fn a_certification_signature_states_what_may_change_after_it() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> /Perms << /DocMDP 5 0 R >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 840 960 240] /Name (A. Author) /M (D:20260801120000+02'00') \
             /Reason (I approve) /Location (Zurich) /Contents <00> /Reference [6 0 R] \
             /Changes [1 2 3] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P 2 /V /1.2 >> >>",
        ]);

        let found = signatures(&doc);
        let [signature] = found.as_slice() else {
            panic!("one signature, got {found:?}");
        };
        assert!(!signature.timestamp);
        assert_eq!(signature.handler.as_deref(), Some("Adobe.PPKLite"));
        assert_eq!(signature.name.as_deref(), Some("A. Author"));
        assert_eq!(signature.reason.as_deref(), Some("I approve"));
        assert_eq!(signature.location.as_deref(), Some("Zurich"));
        assert_eq!(signature.changes, Some([1, 2, 3]));
        assert!(
            signature.certification,
            "a /Reference with a DocMDP transform is what makes it a certification signature"
        );
        assert!(
            !signature.must_cover_whole_file(),
            "adbe.pkcs7.detached leaves §12.8.1's \"should\" as a should"
        );

        assert_eq!(
            signature.coverage(1200),
            Coverage::WholeFile,
            "0..840 and 960..1200 leave one gap, which is where /Contents sits"
        );
        assert_eq!(
            signature.coverage(1400),
            Coverage::Unsigned { tail: 200 },
            "two hundred bytes were appended after signing"
        );
        assert_eq!(
            signature.coverage(1000),
            Coverage::Malformed,
            "a range past the end of the file names bytes that are not there"
        );

        assert_eq!(
            permissions(&doc).doc_mdp,
            Some(Modification::FormFilling),
            "/P 2 permits form filling and signing"
        );
        assert!(!permissions(&doc).usage_rights);
    }

    /// A prepared but unsigned signature field is not a signature.
    ///
    /// The common shape in a blank form: the field exists so that somebody can sign it, and its
    /// `/V` is absent or empty. Reading it as a signature would report every unsigned form as
    /// carrying one.
    #[test]
    fn an_unsigned_signature_field_holds_no_signature() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Empty) /Subtype /Widget >>",
            "<< /FT /Sig /T (AlsoEmpty) /V << /Type /Sig /Filter /Adobe.PPKLite >> >>",
        ]);
        assert!(signatures(&doc).is_empty());
    }

    /// A range that does not start at the beginning of the file has not signed the header.
    #[test]
    fn a_range_that_starts_late_is_malformed() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (S) /V << /ByteRange [8 100] /Contents <00> \
             /SubFilter /ETSI.CAdES.detached >> >>",
        ]);
        let found = signatures(&doc);
        let [signature] = found.as_slice() else {
            panic!("one signature");
        };
        assert!(
            signature.must_cover_whole_file(),
            "ETSI.CAdES.detached turns §12.8.1's should into Table 255's shall"
        );
        assert_eq!(signature.coverage(200), Coverage::Malformed);
    }

    /// §12.8.4's store, counted, and §12.8.7's declaration, checked against the document.
    ///
    /// The `/Legal` dictionary states four counts; three of them are wrong on purpose, and the
    /// fourth is the URI action the document actually holds. What comes back is the difference,
    /// which is the only thing a reader can honestly offer: the clause states no counting
    /// algorithm, so a disagreement is a question rather than a verdict.
    #[test]
    fn a_documents_own_declaration_can_be_held_against_it() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /DSS 3 0 R \
             /Legal << /JavaScriptActions 0 /URIActions 1 /LaunchActions 2 /TrueTypeFonts 0 \
             /Attestation (Nothing here moves.) >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /DSS /Certs [5 0 R 6 0 R] /OCSPs [6 0 R] \
             /VRI << /ABCDEF << /Cert [5 0 R] >> >> >>",
            "<< /S /URI /URI (http://example.invalid/) >>",
            "<< /Length 0 >>\nstream\n\nendstream",
            "<< /Length 0 >>\nstream\n\nendstream",
        ]);

        let store = security_store(&doc);
        assert_eq!(store.certificates, 2);
        assert_eq!(store.revocation_lists, 0);
        assert_eq!(store.ocsp_responses, 1);
        assert_eq!(store.validated_signatures, 1);
        assert!(!store.is_empty());

        let legal = legal(&doc).expect("a /Legal dictionary");
        assert_eq!(legal.attestation.as_deref(), Some("Nothing here moves."));
        assert_eq!(
            legal.disagreements(&doc),
            vec![("LaunchActions".to_owned(), 2, 0)],
            "the URI action agrees; the two launch actions the author declared are not there"
        );
    }
}
