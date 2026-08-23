//! Table 260's three algorithm families, counted over every document this tree can reach.
//!
//! Table 260 names three — "RSA Algorithm Support", "DSA Algorithm Support" and "ECDSA Algorithm
//! Support ( defined by Internet RFC 5480 )". Which of them a real file actually carries is a
//! question about the world rather than about the standard, and `CLAUDE.md`'s trap 11 says to
//! count a population before building for it. **This example is what answered that for ADR 0314**,
//! and it is meant to be re-run rather than quoted: the numbers are not written down here.
//!
//! Every identifier is printed as the dotted decimal the file states rather than as a word, for
//! the reason ADR 0229 gives: this tree holds ISO 32000-2 and not the documents that assign those
//! numbers, so the file's own digits are the only claim a reader can check. What the count keys on
//! is therefore an encoding, and a name appears beside it only where this program acts on one.
//!
//! Three identifiers are counted per signature, because they are three different statements a file
//! makes and a producer can get them out of step: the `SignerInfo`'s `signatureAlgorithm`, its
//! `digestAlgorithm`, and the algorithm of the public key in the certificate that `SignerInfo`
//! names.
//!
//! **Two more populations are counted here since the six-hundred-and-forty-first session**, and
//! for `CLAUDE.md`'s reason rather than for a new question: §12.8.5's ledger row said "[n]o corpus
//! document carries a document timestamp" and §12.8.3.3.2's named its one revocation-information
//! witness by file name, and neither number had a command behind it. Both are one read on data
//! this walk already has — `/Type /DocTimeStamp`, and `adbe-revocationInfoArchival` among the
//! signer's signed attributes — and each witness is printed by path so that a row can name one
//! without a round having to remember which.
//!
//! ```sh
//! cargo run --release -p pdf-model --example signature_algorithm_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line. The `SafeDocs` population is
//! 66 211 documents and a command line holds about a fortieth of them, so a run over that one
//! split into forty-three processes and printed forty-three reports — which is forty-two more
//! populations than were being asked about.
//!
//! ```sh
//! find corpus-cache -name '*.pdf' > /tmp/paths
//! cargo run --release -p pdf-model --example signature_algorithm_census -- @/tmp/paths
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_model::cms::{self, SignatureAlgorithm};
use pdf_model::signature::{Authenticity, Signature, permissions, signatures};
use pdf_model::x509::{self, PublicKey};
use pdf_syntax::Document;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents that opened.
    opened: usize,
    /// Documents stating at least one signature dictionary.
    signed: usize,
    /// Signature dictionaries.
    signatures: usize,
    /// Signature values that read as RFC 5652's `SignedData`.
    readable: usize,
    /// Signature values that did not, keyed by what stopped them.
    unreadable: BTreeMap<String, usize>,
    /// `/SubFilter` values, as the file spells them.
    sub_filters: BTreeMap<String, usize>,
    /// `SignerInfo` `signatureAlgorithm` identifiers.
    signature_algorithms: BTreeMap<String, usize>,
    /// `SignerInfo` `digestAlgorithm` identifiers.
    digest_algorithms: BTreeMap<String, usize>,
    /// The signer's certificate's `subjectPublicKeyInfo` algorithm identifiers.
    key_algorithms: BTreeMap<String, usize>,
    /// What [`Authenticity`] answered, by variant name.
    authenticity: BTreeMap<String, usize>,
    /// §12.8.5's document timestamps: dictionaries whose `/Type` is `DocTimeStamp`.
    ///
    /// Counted because §12.8.5's ledger row states there are none and states it as a bare number.
    /// A claim about the population belongs to the command that produces it, and this is that
    /// command: the row's "no corpus document carries a document timestamp" decides whether its
    /// witness may be a fixture (trap 8) and nothing was re-deriving it.
    timestamps: usize,
    /// Signature values carrying §12.8.3.3.2's `adbe-revocationInfoArchival` signed attribute.
    ///
    /// §12.8.3.3.2's row names one witness by file name for the same reason and with the same
    /// gap: the attribute's *presence* is the only thing this program says about revocation, so
    /// how many files say it is the size of what that sentence is about.
    revocation_material: usize,
    /// Signature values whose ASN.1 states X.690 clause 8.1.3.6's indefinite length anywhere.
    ///
    /// `der`'s module comment and §12.8.3.4.2's ledger row both price this reader's tolerance for
    /// the encoding DER forbids, and they price it with a bare number apiece — "four of the ten
    /// signature values" in one and "four corpus documents" in the other, which are not even the
    /// same denominator. This is the command that settles which is which.
    indefinite_lengths: usize,
    /// §12.8.2.2's certification signatures: a `/Perms /DocMDP`, keyed by the `/P` it states.
    ///
    /// §12.8.2.2's row calls one out by name — "[t]he corpus's one certification signature states
    /// `/P 2`" — and that is two claims a command can check and nothing was checking: how many
    /// there are, and which level each asserts.
    certifications: BTreeMap<String, usize>,
    /// The documents those came from, so that a witness is named rather than only counted.
    witnesses: Vec<String>,
    /// Documents carrying a signature whose algorithm this program does not verify.
    unverifiable_documents: Vec<String>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, mut other: Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.signed = self.signed.saturating_add(other.signed);
        self.signatures = self.signatures.saturating_add(other.signatures);
        self.readable = self.readable.saturating_add(other.readable);
        self.timestamps = self.timestamps.saturating_add(other.timestamps);
        self.revocation_material = self
            .revocation_material
            .saturating_add(other.revocation_material);
        self.indefinite_lengths = self
            .indefinite_lengths
            .saturating_add(other.indefinite_lengths);
        self.witnesses.append(&mut other.witnesses);
        for (map, theirs) in [
            (&mut self.certifications, other.certifications),
            (&mut self.unreadable, other.unreadable),
            (&mut self.sub_filters, other.sub_filters),
            (&mut self.signature_algorithms, other.signature_algorithms),
            (&mut self.digest_algorithms, other.digest_algorithms),
            (&mut self.key_algorithms, other.key_algorithms),
            (&mut self.authenticity, other.authenticity),
        ] {
            for (key, count) in theirs {
                let slot = map.entry(key).or_default();
                *slot = slot.saturating_add(count);
            }
        }
        self.unverifiable_documents
            .append(&mut other.unverifiable_documents);
    }
}

/// One identifier, as dotted decimal — or as hexadecimal where it is not a well-formed one.
fn identifier(oid: &[u8]) -> String {
    x509::dotted(oid).unwrap_or_else(|| {
        use std::fmt::Write as _;
        oid.iter().fold(String::from("0x"), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
    })
}

/// The variant [`Authenticity`] answered with, without the fields a count would scatter over.
fn verdict(answer: &Authenticity) -> String {
    match answer {
        Authenticity::Verified {
            key_bits, family, ..
        } => format!("Verified ({key_bits}-bit {})", family.name()),
        Authenticity::NotUnderThatKey { .. } => "NotUnderThatKey".into(),
        Authenticity::NoSignerCertificate { .. } => "NoSignerCertificate".into(),
        Authenticity::CertificateUnreadable(_) => "CertificateUnreadable".into(),
        Authenticity::KeyNotVerifiable { algorithm } => format!("KeyNotVerifiable {algorithm}"),
        Authenticity::AlgorithmNotVerifiable { algorithm } => {
            format!("AlgorithmNotVerifiable {algorithm}")
        }
        Authenticity::PssParametersNotVerifiable { statement } => {
            format!("PssParametersNotVerifiable: {statement}")
        }
        Authenticity::Refused(_) => "Refused".into(),
        Authenticity::UnknownDigest { algorithm } => format!("UnknownDigest {algorithm}"),
        Authenticity::KeyDoesNotMatchAlgorithm { algorithm, key } => {
            format!("KeyDoesNotMatchAlgorithm {algorithm} over {key}")
        }
        Authenticity::RefusedDsa(_) => "RefusedDsa".into(),
        Authenticity::RefusedEcdsa(_) => "RefusedEcdsa".into(),
        Authenticity::RefusedEdDsa(_) => "RefusedEdDsa".into(),
        Authenticity::CurveNotVerifiable { curve } => format!("CurveNotVerifiable {curve}"),
        Authenticity::NoSignatureValue => "NoSignatureValue".into(),
        Authenticity::RangeNotInThisFile => "RangeNotInThisFile".into(),
        Authenticity::Unreadable(_) => "Unreadable".into(),
    }
}

/// Every signature dictionary one document holds, from both places §12.8.1 puts one.
///
/// A field's `/V` is the ordinary route, and §12.8.6's permissions dictionary is the other: it
/// holds a usage rights signature "(not from a signature field)", which [`signatures`] cannot
/// reach. A dictionary reached both ways is returned once, which `Signature`'s equality decides.
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

/// The `SignerInfo`'s `signatureAlgorithm`, named as this census counts it.
///
/// The identifier always, because it is what a reader can check, and the family beside it only
/// where this program acts on one — a word this tree invented would be the thing principle 5
/// forbids.
fn signature_algorithm(cms: &cms::SignedData<'_>) -> String {
    let number = identifier(cms.signature_algorithm);
    match cms.algorithm() {
        SignatureAlgorithm::RsaPkcs1V15 => format!("{number} (RSASSA-PKCS1-v1_5)"),
        SignatureAlgorithm::RsaPss => format!("{number} (RSASSA-PSS)"),
        SignatureAlgorithm::Dsa => format!("{number} (DSA)"),
        SignatureAlgorithm::Ecdsa => format!("{number} (ECDSA)"),
        SignatureAlgorithm::EdDsa => format!("{number} (EdDSA)"),
        SignatureAlgorithm::Unrecognised(oid) => identifier(oid),
    }
}

/// The signer's own key, named as this census counts it.
///
/// The signer's certificate, not every certificate the value carries: a chain holds its issuers'
/// keys too, and those say nothing about how this was signed.
fn signer_key(cms: &cms::SignedData<'_>) -> String {
    let found = cms
        .certificates
        .iter()
        .filter_map(|value| x509::read(*value).ok())
        .find(|certificate| match cms.signer_issuer_and_serial {
            Some((issuer, serial)) => certificate.is_named_by(issuer, serial),
            None => cms
                .signer_key_identifier
                .is_some_and(|id| certificate.key_identifier == Some(id)),
        });
    match found {
        Some(certificate) => match certificate.public_key {
            PublicKey::Rsa(key) => {
                format!("1.2.840.113549.1.1.1 (rsaEncryption, {}-bit)", key.bits())
            }
            PublicKey::Dsa(key) => format!(
                "1.2.840.10040.4.1 (id-dsa, L = {}, N = {})",
                key.bits(),
                key.subgroup_bits()
            ),
            PublicKey::Ec(key) => {
                format!("1.2.840.10045.2.1 (id-ecPublicKey, {})", key.curve.name())
            }
            PublicKey::EcCurveNotVerifiable { curve } => format!(
                "1.2.840.10045.2.1 (id-ecPublicKey, curve {})",
                curve.map_or_else(|| "(not a namedCurve)".to_owned(), identifier)
            ),
            PublicKey::Ed25519(_) => "1.3.101.112 (id-Ed25519)".to_owned(),
            PublicKey::Unverifiable { algorithm } => identifier(algorithm),
        },
        None => "(the signer's certificate was not found)".to_owned(),
    }
}

/// Every signature one document states, read for the three identifiers it carries.
fn census(path: &str, bytes: &[u8], document: &Document) -> Counts {
    let mut counts = Counts {
        opened: 1,
        ..Counts::default()
    };
    // §12.8.2.2's certification: read from the permissions dictionary rather than from a
    // signature, because `/Perms /DocMDP` is what §12.8.6 makes the transform *binding* — a
    // `/DocMDP` transform on a signature nothing points at asserts nothing.
    if let Some(level) = permissions(document).doc_mdp {
        let named = format!("{level:?}");
        let slot = counts.certifications.entry(named).or_default();
        *slot = slot.saturating_add(1);
        counts
            .witnesses
            .push(format!("{path}: §12.8.2.2 certification, /P {level:?}"));
    }

    let signatures = every_signature(document);
    if signatures.is_empty() {
        return counts;
    }
    counts.signed = 1;
    for signature in &signatures {
        counts.signatures = counts.signatures.saturating_add(1);
        let sub_filter = signature
            .sub_filter
            .clone()
            .unwrap_or_else(|| "(none)".into());
        let slot = counts.sub_filters.entry(sub_filter).or_default();
        *slot = slot.saturating_add(1);
        if signature.timestamp {
            counts.timestamps = counts.timestamps.saturating_add(1);
            counts
                .witnesses
                .push(format!("{path}: §12.8.5 document timestamp"));
        }
        if states_indefinite_length(&signature.contents) {
            counts.indefinite_lengths = counts.indefinite_lengths.saturating_add(1);
            counts
                .witnesses
                .push(format!("{path}: §12.8.3.4.2 indefinite ASN.1 length"));
        }
        let answer = signature.authenticity(bytes);
        let unverifiable = matches!(
            answer,
            Authenticity::KeyNotVerifiable { .. }
                | Authenticity::AlgorithmNotVerifiable { .. }
                | Authenticity::KeyDoesNotMatchAlgorithm { .. }
                | Authenticity::PssParametersNotVerifiable { .. }
                | Authenticity::CurveNotVerifiable { .. }
        );
        let slot = counts.authenticity.entry(verdict(&answer)).or_default();
        *slot = slot.saturating_add(1);
        match signature.signed_data() {
            Ok(cms) => {
                counts.readable = counts.readable.saturating_add(1);
                if cms.has_signed_attribute(cms::ADBE_REVOCATION_INFO_ARCHIVAL) {
                    counts.revocation_material = counts.revocation_material.saturating_add(1);
                    counts
                        .witnesses
                        .push(format!("{path}: §12.8.3.3.2 adbe-revocationInfoArchival"));
                }
                let slot = counts
                    .signature_algorithms
                    .entry(signature_algorithm(&cms))
                    .or_default();
                *slot = slot.saturating_add(1);
                let digest = match cms.digest {
                    Some(digest) => format!(
                        "{} ({})",
                        identifier(cms.digest_algorithm),
                        cms::Digest::name(digest)
                    ),
                    None => identifier(cms.digest_algorithm),
                };
                let slot = counts.digest_algorithms.entry(digest).or_default();
                *slot = slot.saturating_add(1);
                let slot = counts.key_algorithms.entry(signer_key(&cms)).or_default();
                *slot = slot.saturating_add(1);
            }
            Err(error) => {
                let slot = counts.unreadable.entry(error.to_string()).or_default();
                *slot = slot.saturating_add(1);
            }
        }
        if unverifiable {
            counts.unverifiable_documents.push(path.to_owned());
        }
    }
    counts
}

/// Whether any value in one encoding states X.690 clause 8.1.3.6's indefinite length.
///
/// The whole tree rather than the outermost value: `der`'s comment says Adobe's handler writes
/// `30 80` for the `ContentInfo`, and that is the *observed* shape rather than the only legal one
/// — an encoder may state a definite length outside and an indefinite one for a `SignerInfo`
/// within. A reader that asked only the first value would confirm the observation and miss the
/// question.
///
/// An unreadable encoding answers `false`: a value that cannot be walked has not been shown to
/// use the indefinite form, and this census counts what a file *states*.
fn states_indefinite_length(contents: &[u8]) -> bool {
    fn any(mut reader: pdf_model::der::Reader<'_>) -> bool {
        while let Ok(Some(value)) = reader.next_value() {
            if value.had_indefinite_length() {
                return true;
            }
            if value.is_constructed()
                && let Ok(children) = value.children()
                && any(children)
            {
                return true;
            }
        }
        false
    }
    pdf_model::der::Reader::new(contents).is_ok_and(any)
}

/// One map printed largest first, which is the order a population reads in.
fn report(title: &str, map: &BTreeMap<String, usize>) {
    println!("\n{title}");
    let mut rows: Vec<_> = map.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    if rows.is_empty() {
        println!("  (none)");
    }
    for (key, count) in rows {
        println!("  {count:6}  {key}");
    }
}

/// The arguments, with each `@list` replaced by the lines of the file it names.
fn paths() -> Vec<String> {
    let mut out = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.strip_prefix('@') {
            Some(list) => match std::fs::read_to_string(list) {
                Ok(text) => out.extend(text.lines().map(str::to_owned)),
                Err(error) => println!("{list}: {error}"),
            },
            None => out.push(argument),
        }
    }
    out
}

fn main() {
    let paths = paths();
    let counts = paths
        .par_iter()
        .map(|path| {
            let mut counts = Counts::default();
            let Ok(bytes) = std::fs::read(path) else {
                return counts;
            };
            let Ok(document) = Document::open(bytes) else {
                return counts;
            };
            // The document's own bytes rather than a second copy of them: `/ByteRange` names
            // offsets in the file, and a 94 GB population is not one to hold twice.
            let bytes = std::sync::Arc::clone(document.bytes());
            counts.absorb(census(path, &bytes, &document));
            counts
        })
        .reduce(Counts::default, |mut total, counts| {
            total.absorb(counts);
            total
        });

    println!(
        "{} paths, {} opened, {} carry a signature dictionary, {} dictionaries between them",
        paths.len(),
        counts.opened,
        counts.signed,
        counts.signatures,
    );
    println!(
        "{} signature values read as RFC 5652 SignedData",
        counts.readable
    );
    println!(
        "{} of those dictionaries are §12.8.5 document timestamps; {} signature values carry \
         §12.8.3.3.2's adbe-revocationInfoArchival",
        counts.timestamps, counts.revocation_material,
    );
    println!(
        "{} signature values state X.690's indefinite length, which DER forbids and \
         §12.8.3.4.2's row prices",
        counts.indefinite_lengths
    );
    for witness in &counts.witnesses {
        println!("  {witness}");
    }
    report(
        "§12.8.2.2 certification signatures, by Table 257 /P",
        &counts.certifications,
    );
    report("what stopped the rest", &counts.unreadable);
    report("/SubFilter", &counts.sub_filters);
    report(
        "SignerInfo signatureAlgorithm",
        &counts.signature_algorithms,
    );
    report("SignerInfo digestAlgorithm", &counts.digest_algorithms);
    report("the signer's certificate's key", &counts.key_algorithms);
    report("Signature::authenticity answered", &counts.authenticity);
    println!("\ndocuments whose signature names an algorithm this program does not verify:");
    if counts.unverifiable_documents.is_empty() {
        println!("  (none)");
    }
    for path in &counts.unverifiable_documents {
        println!("  {path}");
    }
}
