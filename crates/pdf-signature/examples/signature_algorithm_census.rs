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
//! **Three more since the thousand-and-fifty-third session**, and for ADR 1067's reason: §12.8.4's
//! document security store is the whole supply a revocation check has, because no host here has a
//! network — so how many documents carry one, how much of what they carry reads as an RFC 5280 or
//! RFC 6960 structure, and what section 6.1.3 (a)(3) then answers are the three facts that say how
//! large the new capability's subject is. All three are counted over the same walk.
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

use pdf_signature::cms::{self, SignatureAlgorithm};
use pdf_signature::revocation::Revocation;
use pdf_signature::signature::{
    Authenticity, PadesDeparture, Signature, SigningCertificateBinding, permissions,
    security_store, signatures, signing_certificate_bindings,
};
use pdf_signature::trust::{Trust, TrustAnchors};
use pdf_signature::verdict::{Acceptance, Timestamps, Verdict};
use pdf_signature::x509::{self, Instant, PublicKey};
use pdf_syntax::Document;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// §12.8.3.3.1's signature timestamp attribute, counted and named where a signature states one.
///
/// Its own function because RFC 3161 Appendix A gives it a *check* rather than a presence: the
/// imprint is over "the value of signature field within SignerInfo", so a witness is worth naming
/// with whether it matched.
#[expect(
    clippy::doc_markdown,
    reason = "RFC 3161 Appendix A is quoted verbatim and names a field in camel case; a quotation \
              with backticks added to please a lint is no longer a quotation"
)]
fn count_signature_timestamp(path: &str, cms: &cms::SignedData<'_>, counts: &mut Counts) {
    let Some(stamped) = pdf_signature::timestamp::signature_timestamp(cms) else {
        return;
    };
    counts.signature_timestamps = counts.signature_timestamps.saturating_add(1);
    let covers = stamped.is_ok_and(|stamp| stamp.covers_the_signature);
    if covers {
        counts.signature_timestamps_covering =
            counts.signature_timestamps_covering.saturating_add(1);
    }
    counts.witnesses.push(format!(
        "{path}: §12.8.3.3.1 signature timestamp attribute, covers the signature: {covers}"
    ));
}

/// ITU-T X.690 clause 10 over one signer's `SignedAttrs`, counted and named.
///
/// The region asked about is the set's *contents*, which is what RFC 5652 section 5.4 digests
/// after replacing the `[0] IMPLICIT` header with an EXPLICIT `SET OF` tag — so this is the same
/// question `Signature::authenticity` asks before it digests anything, put to every readable
/// signature rather than only to the ones that get that far.
fn count_signed_attribute_der(path: &str, cms: &cms::SignedData<'_>, counts: &mut Counts) {
    let Some(contents) = cms.signed_attributes else {
        let slot = counts
            .signed_attribute_der
            .entry("(the signer states no signed attributes)".into())
            .or_default();
        *slot = slot.saturating_add(1);
        return;
    };
    let answer = match pdf_signature::der::is_canonical(contents) {
        Ok(None) => "DER throughout, so the digest is over the octets the file holds".to_owned(),
        Ok(Some(rule)) => {
            counts
                .signed_attribute_witnesses
                .push(format!("{path}: {rule}"));
            rule.to_string()
        }
        Err(error) => format!("the region would not read: {error}"),
    };
    let slot = counts.signed_attribute_der.entry(answer).or_default();
    *slot = slot.saturating_add(1);
}

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
    /// What §12.8.3.4's structural rules answered, by [`PadesDeparture`] variant name.
    ///
    /// **The population is `ETSI.CAdES.detached` alone** — §12.8.3.4.1 scopes the whole subclause
    /// to that sub-filter, and `Signature::pades_departures` answers nothing for anything else — so
    /// the `/SubFilter` report above is what says how many signatures this one is about. Counted
    /// because §12.8.3.4.3's row states there are none of them and states it as a bare sentence: a
    /// claim about a population belongs to the command that produces it (trap 8).
    pades_departures: BTreeMap<String, usize>,
    /// Which of §12.8.3.4.4's two profiles each `PAdES` signature presents itself as.
    pades_profiles: BTreeMap<String, usize>,
    /// Which of §12.8.3.4.3's attributes each `PAdES` signature actually states.
    ///
    /// **Without this the report above cannot be read.** A rule that produced no departure did so
    /// either because every file met it or because no file states the attribute it is about, and
    /// those are opposite facts about the corpus — the first ranks the rule, the second says the
    /// corpus cannot rank it at all (trap 8). Counted in both sets, because which set an attribute
    /// is in is what three of the rules are about.
    pades_attributes: BTreeMap<String, usize>,
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
    /// Signature values carrying §12.8.3.3.1's signature timestamp attribute.
    ///
    /// The clause's other timestamp — "Timestamp information as an unsigned attribute ( PDF 1.6 )"
    /// — and a population nothing here had counted: §12.8.3.3.1's row owes the attribute and
    /// §12.8.3.4.8's row owes the clock that could be read out of it, and neither knew how many
    /// files carry one.
    signature_timestamps: usize,
    /// How many of those commit to the signature they sit on, as RFC 3161 Appendix A requires.
    signature_timestamps_covering: usize,
    /// §12.8.3.4.5 (a)'s answer, by variant name, for every signature stating one of §12.8.3.4.3
    /// (f)'s two attributes.
    ///
    /// Counted because the step is now *made* rather than deferred, and what a check does to a
    /// population is the fact a round needs before it may believe the check is safe: a `Differs`
    /// or a refusal here is a signature this program stopped calling verified, and the ledger's
    /// §12.8.3.4.5 row says how to produce the number rather than what it was.
    signing_certificate: BTreeMap<String, usize>,
    /// The documents that stated one, named rather than only counted, each with its answer.
    ///
    /// Every one of them and not only the failures, because the population is the thing in doubt:
    /// a clean column here reads as "the check is safe" and reads identically as "nothing reached
    /// it", and only a named witness tells the two apart (trap 13).
    signing_certificate_witnesses: Vec<String>,
    /// Signature values whose ASN.1 states X.690 clause 8.1.3.6's indefinite length anywhere.
    ///
    /// `der`'s module comment and §12.8.3.4.2's ledger row both price this reader's tolerance for
    /// the encoding DER forbids, and they price it with a bare number apiece — "four of the ten
    /// signature values" in one and "four corpus documents" in the other, which are not even the
    /// same denominator. This is the command that settles which is which.
    indefinite_lengths: usize,
    /// What ITU-T X.690 clause 10 answered about each signer's `SignedAttrs` region, by rule.
    ///
    /// **RFC 5652 section 5.4 is why this is a population worth having**: the signature is over
    /// "the message digest of the complete DER encoding of the SignedAttrs value", so a region
    /// that departs from any of the rules holds octets no conforming signer digested, and this
    /// program refuses it by name rather than reporting a signature that does not verify. Whether
    /// that refusal is about real files or about nothing is a question about the world, and only a
    /// walk over the world answers it (trap 8). The conforming answer is counted in the same map,
    /// because a table of departures alone cannot say whether anything reached the check.
    signed_attribute_der: BTreeMap<String, usize>,
    /// The documents whose signed attributes departed, named one by one with the rule.
    signed_attribute_witnesses: Vec<String>,
    /// Table 255's `/V`, as the file states it — the format version, keyed by its own integer.
    ///
    /// The entry's 1 means "the Reference dictionary shall be considered critical to the
    /// validation of the signature" (§12.8.1), which is a sentence addressed to a validator and
    /// therefore the size of what a program that evaluates no transform method is not doing.
    /// Counted because the row that names it states a population, and a population belongs to
    /// the command that produces it.
    format_versions: BTreeMap<String, usize>,
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
    /// Documents stating a §12.8.4.3 document security store with anything in it.
    ///
    /// **The denominator ADR 1067 rests on.** The revocation reader can say only as much as the
    /// documents supply, because no host here has a network and the DSS is the whole supply — so
    /// how many documents carry one at all is the size of what it is for, and that is a fact about
    /// the world rather than about the standard (trap 8). The corpus gate counts this over the 963
    /// documents of `doc/pdf.js`; this counts it over every document the tree can reach.
    security_stores: usize,
    /// How many CRLs, OCSP responses and certificates those stores hold between them.
    store_crls: usize,
    /// Likewise for `/OCSPs`.
    store_ocsps: usize,
    /// Likewise for `/Certs`.
    store_certs: usize,
    /// How many of those CRLs and OCSP responses read as RFC 5280 and RFC 6960 structures.
    material_read: usize,
    /// What the rest were refused as, by the refusal's own words.
    material_refused: BTreeMap<String, usize>,
    /// What RFC 5280 section 6.1.3 (a)(3) answered, by variant, over the signatures in a document
    /// that carries a store.
    ///
    /// **The anchor is the file's own self-signed certificate, and that is not a trust decision.**
    /// Believing a root a document supplies is precisely what a trust store exists to prevent; what
    /// this exercises is the *algorithm* over chains and revocation material nobody here could have
    /// made, which is the half a fixture cannot reach (ADR 1067). Nothing here says valid.
    revocation: BTreeMap<String, usize>,
    /// What [`pdf_signature::verdict::Verdict`] answered over the same arrangement, by reason.
    ///
    /// **The one measurement that says how far the corpus can get once an anchor exists.** The
    /// anchor is again the file's own self-signed certificate — a file vouching for itself, which
    /// is exactly what a trust store prevents and is said out loud here for that reason (trap 8) —
    /// and the acceptance is the most permissive one, so this is an *upper bound* on what the
    /// crawl could reach rather than a verdict anybody should act on. What it measures is which of
    /// §12.8.1's three questions actually stops these documents when the third is no longer the
    /// first thing in the way. ADR 1076.
    verdicts: BTreeMap<String, usize>,
    /// What §12.8.5's four steps answered for each document timestamp, with the token's own
    /// self-signed certificate supplied as the anchor — [`count_established`]'s caveat and all.
    established: BTreeMap<String, usize>,
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
        self.signature_timestamps = self
            .signature_timestamps
            .saturating_add(other.signature_timestamps);
        self.signature_timestamps_covering = self
            .signature_timestamps_covering
            .saturating_add(other.signature_timestamps_covering);
        self.indefinite_lengths = self
            .indefinite_lengths
            .saturating_add(other.indefinite_lengths);
        self.security_stores = self.security_stores.saturating_add(other.security_stores);
        self.store_crls = self.store_crls.saturating_add(other.store_crls);
        self.store_ocsps = self.store_ocsps.saturating_add(other.store_ocsps);
        self.store_certs = self.store_certs.saturating_add(other.store_certs);
        self.material_read = self.material_read.saturating_add(other.material_read);
        self.witnesses.append(&mut other.witnesses);
        self.signing_certificate_witnesses
            .append(&mut other.signing_certificate_witnesses);
        for (map, theirs) in [
            (&mut self.signing_certificate, other.signing_certificate),
            (&mut self.certifications, other.certifications),
            (&mut self.format_versions, other.format_versions),
            (&mut self.unreadable, other.unreadable),
            (&mut self.sub_filters, other.sub_filters),
            (&mut self.pades_departures, other.pades_departures),
            (&mut self.pades_profiles, other.pades_profiles),
            (&mut self.pades_attributes, other.pades_attributes),
            (&mut self.signature_algorithms, other.signature_algorithms),
            (&mut self.digest_algorithms, other.digest_algorithms),
            (&mut self.key_algorithms, other.key_algorithms),
            (&mut self.authenticity, other.authenticity),
            (&mut self.signed_attribute_der, other.signed_attribute_der),
            (&mut self.material_refused, other.material_refused),
            (&mut self.revocation, other.revocation),
            (&mut self.verdicts, other.verdicts),
            (&mut self.established, other.established),
        ] {
            for (key, count) in theirs {
                let slot = map.entry(key).or_default();
                *slot = slot.saturating_add(count);
            }
        }
        self.unverifiable_documents
            .append(&mut other.unverifiable_documents);
        self.signed_attribute_witnesses
            .append(&mut other.signed_attribute_witnesses);
    }
}

/// §12.8.3.4's structural rules and §12.8.3.4.4's profile, over one signature.
///
/// Costs nothing on a signature that is not a `PAdES` one: `Signature::pades_departures` reads
/// `/SubFilter` first and answers with an empty list, which is §12.8.3.4.1's own scoping.
fn count_pades(
    path: &str,
    signature: &Signature,
    bytes: &pdf_syntax::FileBytes,
    counts: &mut Counts,
) {
    let Ok(cms) = signature.signed_data() else {
        return;
    };
    for departure in signature.pades_departures(&cms, bytes) {
        let slot = counts
            .pades_departures
            .entry(format!("{departure:?}"))
            .or_default();
        *slot = slot.saturating_add(1);
        // Named as well as counted, for trap 11's reason: a count says a rule fired and a name is
        // what lets somebody open the file and check that it fired on the right thing. The three
        // §12.8.3.4.2 departures are left out because they are about the dictionary rather than
        // about an attribute, and hundreds of files state them.
        if !matches!(
            departure,
            PadesDeparture::RangeDoesNotCoverTheFile
                | PadesDeparture::CertEntryPresent
                | PadesDeparture::BothSigningTimesStated
        ) {
            counts
                .witnesses
                .push(format!("{path}: §12.8.3.4 {departure:?}"));
        }
    }
    let Some(profile) = signature.pades_profile(&cms) else {
        return;
    };
    let slot = counts
        .pades_profiles
        .entry(format!("{profile:?}"))
        .or_default();
    *slot = slot.saturating_add(1);
    for (name, oid) in [
        ("signature-time-stamp", cms::ID_AA_TIME_STAMP_TOKEN),
        ("content-time-stamp", cms::ID_AA_ETS_CONTENT_TIMESTAMP),
        ("signer-attributes-v2", cms::ID_AA_ETS_SIGNER_ATTR_V2),
        ("signature-policy-identifier", cms::ID_AA_ETS_SIG_POLICY_ID),
        ("signature-policy-store", cms::ID_AA_ETS_SIG_POLICY_STORE),
        ("commitment-type-indication", cms::ID_AA_ETS_COMMITMENT_TYPE),
        ("mime-type", cms::ID_AA_ETS_MIME_TYPE),
        ("signer-location", cms::ID_AA_ETS_SIGNER_LOCATION),
    ] {
        let stated = cms.attribute_count(oid);
        if stated > 0 {
            let slot = counts.pades_attributes.entry(name.to_owned()).or_default();
            *slot = slot.saturating_add(stated);
        }
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
        Authenticity::RangeNotReadable => "RangeNotReadable".into(),
        Authenticity::Unreadable(_) => "Unreadable".into(),
        // A row of its own rather than a share of `Unreadable`: RFC 5652 section 5.3's one DER
        // region is the whole reason the reader's BER tolerance has a boundary, and how many real
        // signatures cross it is the count that prices it.
        Authenticity::SignedAttributesNotDer { rule } => {
            format!("SignedAttributesNotDer ({rule})")
        }
        Authenticity::SigningCertificateMismatch { version, digest } => format!(
            "SigningCertificateMismatch ({}, {})",
            version.attribute_name(),
            cms::Digest::name(*digest)
        ),
        Authenticity::SigningCertificateUnverifiable { version, statement } => format!(
            "SigningCertificateUnverifiable ({}): {statement}",
            version.attribute_name()
        ),
    }
}

/// [`SigningCertificateBinding`] as this census counts it: the attribute, then the answer.
fn binding(answer: &SigningCertificateBinding) -> String {
    let attribute = answer.version().attribute_name();
    match *answer {
        SigningCertificateBinding::Matches { digest, .. } => {
            format!("{attribute}: Matches ({})", cms::Digest::name(digest))
        }
        SigningCertificateBinding::Differs { digest, .. } => {
            format!("{attribute}: Differs ({})", cms::Digest::name(digest))
        }
        SigningCertificateBinding::Unreadable { ref error, .. } => {
            format!("{attribute}: Unreadable ({error})")
        }
        SigningCertificateBinding::CertificateNotDer { .. } => {
            format!("{attribute}: CertificateNotDer")
        }
        SigningCertificateBinding::NoSignerCertificate { .. } => {
            format!("{attribute}: NoSignerCertificate")
        }
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

/// §12.8.3.4.5 (a)'s answers for one signature, counted and named.
fn count_bindings(path: &str, cms: &cms::SignedData<'_>, counts: &mut Counts) {
    for answer in signing_certificate_bindings(cms) {
        let named = binding(&answer);
        counts
            .signing_certificate_witnesses
            .push(format!("{path}: §12.8.3.4.5 (a) {named}"));
        let slot = counts.signing_certificate.entry(named).or_default();
        *slot = slot.saturating_add(1);
    }
}

/// Every signature one document states, read for the three identifiers it carries.
fn census(path: &str, bytes: &pdf_syntax::FileBytes, document: &Document) -> Counts {
    let mut counts = Counts {
        opened: 1,
        ..Counts::default()
    };
    count_certification(path, document, &mut counts);

    let store = security_store(document);
    let has_store = !store.is_empty();
    count_store(path, &store, &mut counts);

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
        let version = signature.format_version.map_or_else(
            || "(absent, so Table 255's default 0)".to_owned(),
            |value| value.to_string(),
        );
        let slot = counts.format_versions.entry(version).or_default();
        *slot = slot.saturating_add(1);
        if signature.reference_is_critical() {
            counts.witnesses.push(format!(
                "{path}: Table 255 /V 1, reference dictionary critical"
            ));
        }
        if signature.timestamp {
            counts.timestamps = counts.timestamps.saturating_add(1);
            counts
                .witnesses
                .push(format!("{path}: §12.8.5 document timestamp"));
            count_established(signature, bytes, &mut counts);
        }
        count_pades(path, signature, bytes, &mut counts);
        if states_indefinite_length(&signature.contents) {
            counts.indefinite_lengths = counts.indefinite_lengths.saturating_add(1);
            counts
                .witnesses
                .push(format!("{path}: §12.8.3.4.2 indefinite ASN.1 length"));
        }
        if has_store {
            count_revocation(&store, signature, bytes, &mut counts);
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
                count_signed_attribute_der(path, &cms, &mut counts);
                count_signature_timestamp(path, &cms, &mut counts);
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
                count_bindings(path, &cms, &mut counts);
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

/// §12.8.2.2's certification level, read from the permissions dictionary rather than from a
/// signature.
///
/// `/Perms /DocMDP` is what §12.8.6 makes the transform *binding*, so a `/DocMDP` transform on a
/// signature nothing points at asserts nothing and is not counted here.
fn count_certification(path: &str, document: &Document, counts: &mut Counts) {
    if let Some(level) = permissions(document).doc_mdp {
        let named = format!("{level:?}");
        let slot = counts.certifications.entry(named).or_default();
        *slot = slot.saturating_add(1);
        counts
            .witnesses
            .push(format!("{path}: §12.8.2.2 certification, /P {level:?}"));
    }
}

/// §12.8.4.3's store, counted whether or not the document is signed.
///
/// Counted for an unsigned document too: a DSS where there is no signature is a producer's
/// mistake this census would otherwise never see, and the question it answers — how many documents
/// carry the material a validator needs — is about documents rather than about signatures.
fn count_store(path: &str, store: &pdf_signature::signature::SecurityStore, counts: &mut Counts) {
    if store.is_empty() {
        return;
    }
    counts.security_stores = 1;
    counts.store_certs = store.certificates.len();
    counts.store_crls = store.revocation_lists.len();
    counts.store_ocsps = store.ocsp_responses.len();
    let material = store.material();
    counts.material_read = material
        .lists
        .len()
        .saturating_add(material.responses.len());
    for refusal in &material.refused {
        let slot = counts
            .material_refused
            .entry(refusal.to_string())
            .or_default();
        *slot = slot.saturating_add(1);
    }
    counts.witnesses.push(format!(
        "{path}: §12.8.4.3 store, {} certs, {} CRLs, {} OCSPs, {} VRI",
        store.certificates.len(),
        store.revocation_lists.len(),
        store.ocsp_responses.len(),
        store.validation_information.len(),
    ));
}

/// What §12.8.4's material says about one signature's certification path.
///
/// The instant is fixed rather than the clock's, for the reason the corpus gate's is: a census
/// whose answer changed with the day would be measuring the calendar. 2026-06-01 is inside the
/// period of the certificates most of these files carry and past the `thisUpdate` of most of their
/// material; a `Stale` here is therefore a real answer about a document rather than an artefact.
fn count_revocation(
    store: &pdf_signature::signature::SecurityStore,
    signature: &Signature,
    bytes: &pdf_syntax::FileBytes,
    counts: &mut Counts,
) {
    let at = Instant::from_unix_seconds(1_780_272_000);
    let Ok(cms) = signature.signed_data() else {
        return;
    };
    let roots: Vec<_> = cms
        .certificates
        .iter()
        .filter_map(|entry| x509::read(*entry).ok())
        .filter(|certificate| certificate.subject == certificate.issuer)
        .collect();
    if roots.is_empty() {
        return;
    }
    let anchors = TrustAnchors::of(&roots);
    let material = store.material();
    let named = match signature.trust(&anchors, &material, at) {
        Trust::Anchored { revocation, .. } => match revocation {
            Revocation::NotChecked => "NotChecked".to_owned(),
            Revocation::Good { from, .. } => format!("Good, from {}", from.name()),
            Revocation::Revoked { reason, from, .. } => format!(
                "Revoked ({}), from {}",
                reason.map_or("no reason stated", |reason| reason.name()),
                from.name()
            ),
            Revocation::Unknown { why, .. } => format!("Unknown: {why}"),
            // Both enumerations are `#[non_exhaustive]`, which is what makes a new variant a
            // compiler error inside the crate and a census row here rather than a silent gap.
            ref other => format!("(a revocation answer this census does not name: {other:?})"),
        },
        other => format!("(no path: {})", variant(&other)),
    };
    let slot = counts.revocation.entry(named).or_default();
    *slot = slot.saturating_add(1);

    // And what that path, plus the two questions before it, adds up to. `Timestamps::None` because
    // this is asked per *signature*: a document's chain is `timestamp::chain`'s and is counted a
    // few lines up rather than folded into a signature's own verdict here.
    let trust = signature.trust(&anchors, &material, at);
    let verdict = Verdict::of(
        &signature.integrity(bytes),
        &signature.authenticity(bytes),
        &trust,
        Acceptance::UnknownRevocationAccepted,
        Timestamps::None,
        at,
    );
    let named = match &verdict {
        Verdict::Valid(valid) => format!("valid (path of {})", valid.path_length()),
        Verdict::Reserved(reservation) => format!("not valid: {reservation}"),
    };
    let slot = counts.verdicts.entry(named).or_default();
    *slot = slot.saturating_add(1);
}

/// What §12.8.5's four steps answer for one document timestamp, with the file's own root supplied.
///
/// **The same arrangement `count_revocation` uses and the same caveat** (trap 8): the anchor is a
/// self-signed certificate the *token* carries, so this is a file vouching for its own authority.
/// What it measures is which of ADR 1071's four steps the crawl's real tokens actually reach once
/// step 4 is no longer refused for want of anybody to end a path at — which until the
/// one-thousand-and-sixty-second session was every one of them, by name.
fn count_established(timestamp: &Signature, bytes: &pdf_syntax::FileBytes, counts: &mut Counts) {
    let at = Instant::from_unix_seconds(1_780_272_000);
    let Ok(cms) = timestamp.signed_data() else {
        return;
    };
    let roots: Vec<_> = cms
        .certificates
        .iter()
        .filter_map(|entry| x509::read(*entry).ok())
        .filter(|certificate| certificate.subject == certificate.issuer)
        .collect();
    let anchors = TrustAnchors::of(&roots);
    let named = match pdf_signature::timestamp::established(
        timestamp,
        bytes,
        &anchors,
        &pdf_signature::revocation::Material::none(),
        pdf_signature::timestamp::AskedAt::TheCallersInstant(at),
    ) {
        pdf_signature::timestamp::Time::Established { .. } => "an instant established".to_owned(),
        // The variant's name rather than its payload: a census groups, and a refusal carrying a
        // whole `Authenticity` or a whole `Trust` would make one row per document.
        pdf_signature::timestamp::Time::Unknown(why) => match why {
            pdf_signature::timestamp::Unestablished::AuthorityNotEstablished(ref trust) => {
                format!("no path to the token's own root: {}", variant(trust))
            }
            ref other => format!("unknown: {other:?}"),
        },
        ref other => format!("(an answer this census does not name: {other:?})"),
    };
    let slot = counts.established.entry(named).or_default();
    *slot = slot.saturating_add(1);
}

/// A [`Trust`] without the fields a count would scatter over.
fn variant(answer: &Trust) -> String {
    match *answer {
        Trust::NoAnchorSupplied => "NoAnchorSupplied".to_owned(),
        Trust::Anchored { .. } => "Anchored".to_owned(),
        Trust::NoPathToAnyAnchor { .. } => "NoPathToAnyAnchor".to_owned(),
        Trust::Refused { ref refusal, .. } => format!("Refused: {refusal}"),
        ref other => format!("(a verdict this census does not name: {other:?})"),
    }
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
    fn any(mut reader: pdf_signature::der::Reader<'_>) -> bool {
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
    pdf_signature::der::Reader::new(contents).is_ok_and(any)
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

#[expect(
    clippy::too_many_lines,
    reason = "one function per line of the report, in the order it prints: a split would put the \
              population's figures out of sight of the counts they were taken from"
)]
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
            let bytes = document.bytes().clone();
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
        "{} signature values carry §12.8.3.3.1's signature timestamp attribute; {} of those \
         commit to the signature they sit on, as RFC 3161 Appendix A requires",
        counts.signature_timestamps, counts.signature_timestamps_covering,
    );
    println!(
        "{} signature values state X.690's indefinite length, which DER forbids and \
         §12.8.3.4.2's row prices",
        counts.indefinite_lengths
    );
    println!(
        "{} documents carry a §12.8.4.3 document security store, holding {} certificates, {} CRLs \
         and {} OCSP responses between them; {} of those CRLs and responses read",
        counts.security_stores,
        counts.store_certs,
        counts.store_crls,
        counts.store_ocsps,
        counts.material_read,
    );
    for witness in &counts.witnesses {
        println!("  {witness}");
    }
    report(
        "§12.8.4.3 material this reader would not take, by its own words",
        &counts.material_refused,
    );
    report(
        "RFC 5280 section 6.1.3 (a)(3) over the signatures in a document carrying a store",
        &counts.revocation,
    );
    report(
        "§12.8.1's three questions together, with each file's own self-signed root as the anchor \
         (a file vouching for itself: an upper bound, not a verdict)",
        &counts.verdicts,
    );
    report(
        "§12.8.5's four steps per document timestamp, with the token's own self-signed root as the \
         anchor (a file vouching for its own authority)",
        &counts.established,
    );
    report(
        "§12.8.2.2 certification signatures, by Table 257 /P",
        &counts.certifications,
    );
    report(
        "Table 255 /V, the signature dictionary format version (1 = reference dictionary critical)",
        &counts.format_versions,
    );
    report("what stopped the rest", &counts.unreadable);
    report("/SubFilter", &counts.sub_filters);
    report(
        "§12.8.3.4's structural rules over every ETSI.CAdES.detached signature, by departure",
        &counts.pades_departures,
    );
    report(
        "§12.8.3.4.4's two profiles, as each PAdES signature presents itself",
        &counts.pades_profiles,
    );
    report(
        "§12.8.3.4.3's attributes as PAdES signatures actually state them, which is what says \
         whether a rule above was met or was never reached",
        &counts.pades_attributes,
    );
    report(
        "SignerInfo signatureAlgorithm",
        &counts.signature_algorithms,
    );
    report("SignerInfo digestAlgorithm", &counts.digest_algorithms);
    report("the signer's certificate's key", &counts.key_algorithms);
    report("Signature::authenticity answered", &counts.authenticity);
    report(
        "ITU-T X.690 clause 10 over each signer's SignedAttrs, the region RFC 5652 section 5.4 \
         digests",
        &counts.signed_attribute_der,
    );
    println!("signed-attribute regions that depart from a rule, by document:");
    if counts.signed_attribute_witnesses.is_empty() {
        println!("  (none)");
    }
    for witness in &counts.signed_attribute_witnesses {
        println!("  {witness}");
    }
    report(
        "§12.8.3.4.5 (a), the signer's certificate against the hash the signer signed over it",
        &counts.signing_certificate,
    );
    println!("documents stating one of §12.8.3.4.3 (f)'s two attributes:");
    if counts.signing_certificate_witnesses.is_empty() {
        println!("  (none)");
    }
    for witness in &counts.signing_certificate_witnesses {
        println!("  {witness}");
    }
    println!("\ndocuments whose signature names an algorithm this program does not verify:");
    if counts.unverifiable_documents.is_empty() {
        println!("  (none)");
    }
    for path in &counts.unverifiable_documents {
        println!("  {path}");
    }
}
