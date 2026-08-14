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
        for (map, theirs) in [
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

/// Every signature one document states, read for the three identifiers it carries.
fn census(path: &str, bytes: &[u8], document: &Document) -> Counts {
    let mut counts = Counts {
        opened: 1,
        ..Counts::default()
    };
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
        let answer = signature.authenticity(bytes);
        let unverifiable = matches!(
            answer,
            Authenticity::KeyNotVerifiable { .. }
                | Authenticity::AlgorithmNotVerifiable { .. }
                | Authenticity::KeyDoesNotMatchAlgorithm { .. }
                | Authenticity::PssParametersNotVerifiable { .. }
        );
        let slot = counts.authenticity.entry(verdict(&answer)).or_default();
        *slot = slot.saturating_add(1);
        match signature.signed_data() {
            Ok(cms) => {
                counts.readable = counts.readable.saturating_add(1);
                let named = match cms.algorithm() {
                    SignatureAlgorithm::RsaPkcs1V15 => {
                        format!(
                            "{} (RSASSA-PKCS1-v1_5)",
                            identifier(cms.signature_algorithm)
                        )
                    }
                    SignatureAlgorithm::RsaPss => {
                        format!("{} (RSASSA-PSS)", identifier(cms.signature_algorithm))
                    }
                    SignatureAlgorithm::Dsa => {
                        format!("{} (DSA)", identifier(cms.signature_algorithm))
                    }
                    SignatureAlgorithm::Unrecognised(oid) => identifier(oid),
                };
                let slot = counts.signature_algorithms.entry(named).or_default();
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
                // The signer's own certificate, not every certificate the value carries: a chain
                // holds its issuers' keys too, and those say nothing about how this was signed.
                let key = match cms
                    .certificates
                    .iter()
                    .filter_map(|value| x509::read(*value).ok())
                    .find(|certificate| match cms.signer_issuer_and_serial {
                        Some((issuer, serial)) => certificate.is_named_by(issuer, serial),
                        None => cms
                            .signer_key_identifier
                            .is_some_and(|id| certificate.key_identifier == Some(id)),
                    }) {
                    Some(certificate) => match certificate.public_key {
                        PublicKey::Rsa(key) => {
                            format!("1.2.840.113549.1.1.1 (rsaEncryption, {}-bit)", key.bits())
                        }
                        PublicKey::Dsa(key) => format!(
                            "1.2.840.10040.4.1 (id-dsa, L = {}, N = {})",
                            key.bits(),
                            key.subgroup_bits()
                        ),
                        PublicKey::Unverifiable { algorithm } => identifier(algorithm),
                    },
                    None => "(the signer's certificate was not found)".to_owned(),
                };
                let slot = counts.key_algorithms.entry(key).or_default();
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
