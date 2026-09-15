//! The word said once, and then taken away one input at a time.
//!
//! **The calibration is the whole point of this file** (trap 13). A type that refuses to say
//! *valid* is trivially correct and says nothing about whether the refusal is load-bearing, so
//! every test here starts from the one arrangement that *does* say it — [`fixtures::signed`], a
//! document with an `adbe.pkcs7.detached` signature by a certificate `openssl` issued under
//! [`fixtures::ROOT`] — and changes exactly one input: a different anchor, no anchor, a moved
//! byte, an unaccepted revocation answer. Each must move the answer off [`Verdict::Valid`] and
//! name which of §12.8.1's questions it fell at.
//!
//! **And the anchor is an input rather than a judgement.** No corpus document can furnish a path
//! whose positive outcome is known in advance, because that needs a hierarchy whose private keys
//! somebody here held (trap 8, and `crate::trust::tests`' own opening). The crawl's real
//! signatures rank the format; they cannot define it.

use pdf_syntax::Document;

use super::{
    Acceptance, Anchored, BestSignatureTime, Proof, Reservation, Timestamps, Valid, Verdict,
};
use crate::revocation::{Material, Revocation, Undetermined};
use crate::signature::{Integrity, Signature, signatures};
use crate::trust::{Supply, Trust, TrustAnchors};
use crate::x509::{Instant, parse};

/// 2026-10-01T00:00:00Z — inside every fixture certificate's validity period.
const AT: Instant = Instant::from_unix_seconds(1_790_812_800);

/// Hexadecimal to bytes, as every fixture module in this crate spells it.
fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// The fixture document, opened, with its one signature.
fn opened(bytes: Vec<u8>) -> (Document, Signature) {
    let document = Document::open(bytes).expect("the fixture opens");
    let signature = signatures(&document)
        .into_iter()
        .next()
        .expect("the fixture carries one signature");
    (document, signature)
}

/// The verdict on the fixture's signature under `roots`, at [`AT`].
fn verdict_under(roots: &[&str], acceptance: Acceptance) -> Verdict {
    let (document, signature) = opened(fixtures::signed());
    let der: Vec<Vec<u8>> = roots.iter().map(|root| hex(root)).collect();
    let parsed: Vec<_> = der
        .iter()
        .map(|bytes| parse(bytes).expect("a fixture root parses"))
        .collect();
    let anchors = TrustAnchors::of(&parsed);
    let trust = signature.trust(&anchors, &Material::none(), AT);
    Verdict::of(
        &signature.integrity(document.bytes()),
        &signature.authenticity(document.bytes()),
        &trust,
        acceptance,
        Timestamps::None,
        BestSignatureTime::now(AT),
    )
}

/// **The one arrangement in this tree that says *valid*, and what it took.**
///
/// §12.8.1's three questions, all of them: the bytes `/ByteRange` names still hash to the digest
/// the signature records, the signature verifies under the key in the certificate the file carries,
/// and RFC 5280 section 6.1 validates a path from that certificate to the root supplied here as
/// input (d). Revocation is `NotChecked` — no §12.8.4 material is supplied — which
/// [`Acceptance::UnknownRevocationAccepted`] is what admits, in the open, and ADR 1067's rule is
/// untouched by it: an absence never becomes a [`Revocation::Good`].
#[test]
fn a_signature_whose_path_reaches_a_supplied_anchor_is_valid() {
    let verdict = verdict_under(&[fixtures::ROOT], Acceptance::UnknownRevocationAccepted);
    let Verdict::Valid(valid) = &verdict else {
        panic!("the fixture's signature should be valid, not {verdict:?}");
    };
    assert_eq!(valid.path_length(), 1, "the signer, with the root above it");
    assert_eq!(*valid.revocation(), Revocation::NotChecked);
    assert_eq!(valid.at(), AT);
    assert_eq!(valid.timestamps(), Timestamps::None);
}

/// **A different root, and the same signature is not valid.**
///
/// The one input changed. `crate::trust::tests`' hierarchy is a real root this project issued and
/// it signed nothing in this document, so the search reaches no anchor and the word is not said —
/// which is what stops the test above being a sentence about the fixture rather than about the
/// algorithm.
#[test]
fn a_signature_under_somebody_elses_anchor_is_not_valid() {
    let verdict = verdict_under(
        &[crate::trust::tests::fixtures::ROOT],
        Acceptance::UnknownRevocationAccepted,
    );
    assert!(
        matches!(
            verdict,
            Verdict::Reserved(Reservation::NoPathToAnyAnchor { .. })
        ),
        "a root that issued nothing here reaches nothing, not {verdict:?}"
    );
}

/// **And with no anchor it is this program's own answer, unchanged since the three-hundred-and-
/// seventy-seventh session.**
///
/// ADR 1039's decision is what this asserts is still true: an empty set is the default, the
/// reservation says so by name, and nothing about a host existing changes what a host that supplies
/// nothing is told.
#[test]
fn a_signature_under_no_anchor_at_all_says_nobody_named_one() {
    let verdict = verdict_under(&[], Acceptance::UnknownRevocationAccepted);
    assert_eq!(
        verdict,
        Verdict::Reserved(Reservation::NoAnchorSupplied),
        "no anchor, no verdict — and the reservation is about this program, not the file"
    );
}

/// **The host's acceptance decides an unknown revocation, and the default decides against.**
///
/// ADR 1067 section 2's rule is untouched: nothing here turns an absence into a
/// [`Revocation::Good`]. What moves is whether a *host* will act on a verdict resting on one, and
/// [`Acceptance::RevocationMustBeGood`] — the default — will not.
#[test]
fn the_default_acceptance_will_not_have_an_unchecked_revocation() {
    let verdict = verdict_under(&[fixtures::ROOT], Acceptance::RevocationMustBeGood);
    assert_eq!(
        verdict,
        Verdict::Reserved(Reservation::RevocationNotChecked),
        "a host that has not decided gets the conservative answer"
    );
    assert_eq!(Acceptance::default(), Acceptance::RevocationMustBeGood);
}

/// **A moved byte under a path that still reaches the anchor.**
///
/// §12.8.1's question 1 planted back: the anchor is right, the path validates, and the document no
/// longer hashes to what the signature recorded. A verdict that survived this would be saying
/// *valid* about a file that moved.
#[test]
fn a_document_that_moved_is_not_valid_however_good_the_path_is() {
    let mut bytes = fixtures::signed();
    let at = bytes
        .windows(12)
        .position(|window| window == b"/Type /Pages")
        .expect("the fixture states a page tree");
    bytes[at.saturating_add(7)] = b'X';
    let (document, signature) = opened(bytes);
    let root = hex(fixtures::ROOT);
    let root = parse(&root).expect("the fixture root parses");
    let anchors = TrustAnchors::of(std::slice::from_ref(&root));
    let trust = signature.trust(&anchors, &Material::none(), AT);
    let anchored = Anchored::of(&trust).expect("a moved page byte does not move the signer's path");
    let verdict = Verdict::reached(
        &signature.integrity(document.bytes()),
        &signature.authenticity(document.bytes()),
        &anchored,
        Acceptance::UnknownRevocationAccepted,
        Timestamps::None,
        BestSignatureTime::now(AT),
    );
    assert_eq!(
        verdict,
        Verdict::Reserved(Reservation::TheDocumentChanged),
        "the path is somebody's and the document is not what was signed"
    );
}

/// **A revoked certificate on the path, and an unestablished timestamp chain.**
///
/// Both reservations are reached through [`Verdict::reached`] with an [`Anchored`] in hand, which
/// is the case the type exists for: having the proof is necessary and is not sufficient.
#[test]
fn a_proof_in_hand_is_not_a_verdict() {
    let integrity = Integrity::Unchanged {
        digest: crate::cms::Digest::Sha256,
    };
    let authenticity = crate::signature::Authenticity::Verified {
        digest: crate::cms::Digest::Sha256,
        family: crate::signature::Family::Rsa,
        key_bits: 2048,
        over: crate::signature::Signed::SignedAttributes,
    };
    let revoked = Anchored::of(&Trust::Anchored {
        length: 2,
        revocation: Revocation::Revoked {
            at: AT,
            invalid_from: None,
            reason: None,
            from: crate::revocation::Evidence::CertificateRevocationList,
            position: 1,
        },
    })
    .expect("an anchored trust is a proof");
    assert_eq!(
        Verdict::reached(
            &integrity,
            &authenticity,
            &revoked,
            Acceptance::UnknownRevocationAccepted,
            Timestamps::None,
            BestSignatureTime::now(AT),
        ),
        Verdict::Reserved(Reservation::Revoked { position: 1 }),
        "a revocation is never accepted by any acceptance level"
    );

    let unknown = Anchored::of(&Trust::Anchored {
        length: 2,
        revocation: Revocation::Unknown {
            why: Undetermined::NoMaterial,
            position: 0,
        },
    })
    .expect("an anchored trust is a proof");
    assert_eq!(
        Verdict::reached(
            &integrity,
            &authenticity,
            &unknown,
            Acceptance::UnknownRevocationAccepted,
            Timestamps::NotEstablished,
            BestSignatureTime::now(AT),
        ),
        Verdict::Reserved(Reservation::TimestampNotEstablished),
        "a document that states a time this program cannot establish has said something unchecked"
    );
}

/// **A revocation dated after a proven instant does not reach the signature** — and the four
/// things that must each stop that carve-out (trap 13).
///
/// ETSI EN 319 102-1 clause 5.5.4 step 4) a). The arrangement is one revoked certificate on an
/// anchored path, held still while one input moves at a time: the instant is proven or it is not,
/// and the revocation falls after it or before it. Only one of the four says the word, which is
/// what makes the carve-out load-bearing rather than a branch that accepts everything.
///
/// The clause is the whole authority for it: RFC 5280 section 6.3.3's steps (i) to (k) set
/// `cert_status` from a CRL entry without ever reading its `revocationDate`, so nothing in the
/// path-validation profile would have produced this rule.
#[test]
fn a_revocation_after_a_proven_instant_does_not_reach_the_signature() {
    /// A year after [`AT`], which is when the certificate was revoked.
    const REVOKED_AT: Instant = Instant::from_unix_seconds(1_822_348_800);
    /// A year before it, which is when a token proves the signature existed.
    const STAMPED_AT: Instant = Instant::from_unix_seconds(1_759_276_800);

    let integrity = Integrity::Unchanged {
        digest: crate::cms::Digest::Sha256,
    };
    let authenticity = crate::signature::Authenticity::Verified {
        digest: crate::cms::Digest::Sha256,
        family: crate::signature::Family::Rsa,
        key_bits: 2048,
        over: crate::signature::Signed::SignedAttributes,
    };
    let revoked = |at: Instant, invalid_from: Option<Instant>| {
        Anchored::of(&Trust::Anchored {
            length: 2,
            revocation: Revocation::Revoked {
                at,
                invalid_from,
                reason: None,
                from: crate::revocation::Evidence::CertificateRevocationList,
                position: 1,
            },
        })
        .expect("an anchored trust is a proof")
    };
    let verdict = |revoked_at: Instant, when: BestSignatureTime| {
        Verdict::reached(
            &integrity,
            &authenticity,
            &revoked(revoked_at, None),
            Acceptance::RevocationMustBeGood,
            Timestamps::None,
            when,
        )
    };

    let proven = BestSignatureTime::now(AT).proven_at(STAMPED_AT, Proof::ADocumentTimestamp);
    let valid = verdict(REVOKED_AT, proven);
    let Verdict::Valid(said) = &valid else {
        panic!("a revocation a year after a proven instant does not reach it: {valid:?}");
    };
    assert_eq!(said.at(), STAMPED_AT);
    assert_eq!(said.when().proof(), Proof::ADocumentTimestamp);

    // 1. Nothing proved an earlier moment, so there is no evidence the signature predates the
    //    revocation — and the clause's carve-out is about a proof of existence, not a date.
    assert_eq!(
        verdict(REVOKED_AT, BestSignatureTime::now(AT)),
        Verdict::Reserved(Reservation::Revoked { position: 1 }),
    );
    // 2. The revocation is before the proven instant, so the signature was made under a revoked
    //    certificate.
    assert_eq!(
        verdict(
            Instant::from_unix_seconds(STAMPED_AT.unix_seconds() - 1),
            proven
        ),
        Verdict::Reserved(Reservation::Revoked { position: 1 }),
    );
    // 3. The revocation is *at* the proven instant, which the clause's "posterior to" excludes.
    assert_eq!(
        verdict(STAMPED_AT, proven),
        Verdict::Reserved(Reservation::Revoked { position: 1 }),
    );
    // 4. The CA processed the revocation after the proven instant, but says the certificate was
    //    already invalid before it — RFC 5280 section 5.3.2's `invalidityDate`, which "may be
    //    earlier than the revocation date in the CRL entry". The earlier instant is the one the
    //    question is about, so the word is not said.
    assert_eq!(
        Verdict::reached(
            &integrity,
            &authenticity,
            &revoked(
                REVOKED_AT,
                Some(Instant::from_unix_seconds(STAMPED_AT.unix_seconds() - 1)),
            ),
            Acceptance::RevocationMustBeGood,
            Timestamps::None,
            proven,
        ),
        Verdict::Reserved(Reservation::Revoked { position: 1 }),
    );
    // 5. The carve-out removes one reservation and no other: the document still has to be the one
    //    that was signed.
    assert_eq!(
        Verdict::reached(
            &Integrity::Changed {
                digest: crate::cms::Digest::Sha256,
            },
            &authenticity,
            &revoked(REVOKED_AT, None),
            Acceptance::RevocationMustBeGood,
            Timestamps::None,
            proven,
        ),
        Verdict::Reserved(Reservation::TheDocumentChanged),
    );
}

/// **The proof cannot be made from anything but a path that reached an anchor.**
///
/// The type's whole guarantee in one assertion: three of [`Trust`]'s four variants answer `None`,
/// so no caller anywhere can hold a [`Valid`] without one having been built from the fourth.
#[test]
fn only_an_anchored_trust_is_a_proof() {
    assert!(Anchored::of(&Trust::NoAnchorSupplied).is_none());
    assert!(Anchored::of(&Trust::NoPathToAnyAnchor { examined: 3 }).is_none());
    assert!(
        Anchored::of(&Trust::Refused {
            refusal: crate::trust::PathRefusal::NotACertificationAuthority,
            examined: 1,
        })
        .is_none()
    );
    assert!(
        Anchored::of(&Trust::Anchored {
            length: 1,
            revocation: Revocation::NotChecked,
        })
        .is_some()
    );
}

/// And [`Valid`] has no public constructor: what a caller outside this crate can do with one is
/// exactly these three accessors, which is what makes the guarantee above the only route to it.
fn _accessors(valid: &Valid) -> (usize, Instant, Timestamps) {
    (valid.path_length(), valid.at(), valid.timestamps())
}

/// **The host's end of it: DER in, anchors out, and the ones this reader will not take named.**
///
/// [`Supply`] is what a host holds, because [`TrustAnchors`] borrows from certificate bytes
/// somebody has to own. What this asserts is the two halves of trap 5 on that path: the
/// certificates that read become anchors, and the one that does not is named rather than dropped.
#[test]
fn a_supply_reads_what_it_can_and_names_what_it_cannot() {
    let supply = Supply::of(
        vec![
            ("root.der".to_owned(), hex(fixtures::ROOT)),
            ("notes.txt".to_owned(), b"not a certificate".to_vec()),
        ],
        "the directory /etc/quorra/anchors".to_owned(),
        AT,
    );
    let reading = supply.read();
    assert_eq!(reading.anchors.len(), 1, "one of the two is a certificate");
    let [refusal] = reading.refused.as_slice() else {
        panic!("the other is named, not dropped: {:?}", reading.refused);
    };
    assert_eq!(refusal.name, "notes.txt");
    assert_eq!(supply.at(), AT);
    assert_eq!(supply.source(), "the directory /etc/quorra/anchors");
    assert!(Supply::none().is_empty());
    assert!(Supply::none().read().anchors.is_empty());

    // And the verdict is the same whichever way the anchor arrived, which is what makes `Supply` a
    // carrier rather than a second policy.
    let (document, signature) = opened(fixtures::signed());
    let reading = supply.read();
    let trust = signature.trust(&reading.anchors, &Material::none(), supply.at());
    assert!(
        matches!(
            Verdict::of(
                &signature.integrity(document.bytes()),
                &signature.authenticity(document.bytes()),
                &trust,
                Acceptance::UnknownRevocationAccepted,
                Timestamps::None,
                BestSignatureTime::now(supply.at()),
            ),
            Verdict::Valid(_)
        ),
        "the same anchor, carried by the type a host can hold"
    );
}

/// **The fixture's provenance, re-walked rather than asserted from memory.**
///
/// The signature was made over [`fixtures::signed_range`] with `openssl cms -sign -binary -md
/// sha256`, and RFC 5652 section 5.4 is what makes that checkable from inside: the `message-digest`
/// signed attribute "must" be the digest of the content, so recomputing it here is the same
/// arithmetic a verifier does and the same the layout would have to survive if it ever moved.
#[test]
fn the_signature_was_made_over_the_bytes_the_range_names() {
    let range = fixtures::signed_range();
    let der = hex(fixtures::SIGNATURE);
    let cms = crate::cms::signed_data(&der).expect("the fixture value is a SignedData");
    assert_eq!(
        cms.message_digest,
        Some(crate::cms::Digest::Sha256.compute(&[&range]).as_slice()),
        "the layout above is the layout openssl signed"
    );
}

/// A signed document this project issued the hierarchy for, and the hierarchy's root.
///
/// **Why a document of our own** (trap 8, and `crate::trust::tests`' own opening): a *positive*
/// verdict needs a path whose private keys somebody here held, and every certificate in the
/// corpus's signatures was issued by a real authority under a key nobody here has. The corpus can
/// rank what a signed document looks like; it cannot furnish one that validates to a named anchor.
///
/// The order the layout is built in is the load-bearing part: the `/Contents` hole is a fixed size
/// filled with zeros, so **the bytes `/ByteRange` names do not depend on the signature that goes in
/// the hole**. That is what let the digest be taken once, signed once with `openssl cms -sign
/// -binary -md sha256`, and the DER pasted in here the way every other fixture in this crate is.
pub(crate) mod fixtures {
    use std::fmt::Write as _;

    /// Hexadecimal characters reserved for the signature value.
    const ROOM: usize = 8192;

    /// A one-revision document with one `adbe.pkcs7.detached` signature field.
    ///
    /// `signature` is the DER to put in the hole, as hexadecimal; empty leaves the zeros, which is
    /// how [`signed_range`] takes the bytes [`SIGNATURE`] was made over.
    fn document(signature: &str) -> Vec<u8> {
        let value = format!("<{}>", "0".repeat(ROOM));
        let mut bytes = Vec::new();
        let mut offsets: Vec<(u32, usize)> = Vec::new();
        let put =
            |bytes: &mut Vec<u8>, offsets: &mut Vec<(u32, usize)>, number: u32, body: &str| {
                offsets.push((number, bytes.len()));
                bytes.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
            };
        bytes.extend_from_slice(b"%PDF-1.7\n");
        put(
            &mut bytes,
            &mut offsets,
            1,
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
        );
        put(
            &mut bytes,
            &mut offsets,
            2,
            "<< /Type /Pages /Count 0 /Kids [] >>",
        );
        put(
            &mut bytes,
            &mut offsets,
            4,
            "<< /FT /Sig /T (one) /V 5 0 R /Subtype /Widget >>",
        );
        put(
            &mut bytes,
            &mut offsets,
            5,
            &format!(
                "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /Name \
                 (quorra verdict test signer) /ByteRange [0000000000 0000000000 0000000000 \
                 0000000000] /Contents {value} >>"
            ),
        );
        let at = bytes.len();
        let mut table = String::from("xref\n0 1\n0000000000 65535 f \n");
        for (number, offset) in &offsets {
            let _ = write!(table, "{number} 1\n{offset:010} 00000 n \n");
        }
        let _ = write!(
            table,
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        bytes.extend_from_slice(table.as_bytes());

        // The range, then the value: the first depends on the layout alone and the second on the
        // first, which is the order that makes a digest computable before the signature exists.
        let end = bytes.len();
        let hole = position(&bytes, b"/Contents <")
            .expect("a /Contents hole")
            .saturating_add(10);
        let after = hole.saturating_add(ROOM).saturating_add(2);
        let range_at = position(&bytes, b"[0000000000 0000000000 0000000000 0000000000]")
            .expect("a /ByteRange placeholder");
        let range = format!(
            "[{:010} {hole:010} {after:010} {:010}]",
            0,
            end.saturating_sub(after)
        );
        bytes.splice(
            range_at..range_at.saturating_add(range.len()),
            range.bytes(),
        );
        if !signature.is_empty() {
            assert!(signature.len() <= ROOM, "the signature fits its hole");
            let at = hole.saturating_add(1);
            bytes.splice(at..at.saturating_add(signature.len()), signature.bytes());
        }
        bytes
    }

    /// The first offset at which `needle` occurs.
    fn position(bytes: &[u8], needle: &[u8]) -> Option<usize> {
        bytes
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// The document with its real signature in place.
    pub(crate) fn signed() -> Vec<u8> {
        document(SIGNATURE)
    }

    /// The bytes `/ByteRange` names, which are what [`SIGNATURE`] was made over.
    ///
    /// Kept beside the fixture rather than deleted after the minting: it is how the signature would
    /// be re-made if the layout above ever changed, and a fixture whose provenance cannot be
    /// re-walked is a magic number.
    pub(crate) fn signed_range() -> Vec<u8> {
        let bytes = document("");
        let hole = position(&bytes, b"/Contents <")
            .expect("a /Contents hole")
            .saturating_add(10);
        let mut out = bytes[..hole].to_vec();
        out.extend_from_slice(&bytes[hole.saturating_add(ROOM).saturating_add(2)..]);
        out
    }

    /// The signature value: `openssl cms -sign -binary -md sha256 -outform DER` over
    /// [`signed_range`], by a certificate this project issued under [`ROOT`].
    pub(crate) const SIGNATURE: &str = "\
        3082059906092a864886f70d010702a082058a30820586020101310d300b0609\
        608648016503040201300b06092a864886f70d010701a0820327308203233082\
        020ba003020102020102300d06092a864886f70d01010b050030233121301f06\
        035504030c1871756f7272612076657264696374207465737420726f6f74301e\
        170d3236303931343233333233395a170d3336303931313233333233395a3025\
        3123302106035504030c1a71756f727261207665726469637420746573742073\
        69676e657230820122300d06092a864886f70d01010105000382010f00308201\
        0a0282010100d58412a16dacbe88e57590c2db3feb660ab8c8e0d457f1e70255\
        fee6e01fa56c6808eddd192d49dfd00ec2c622534cb8f72349d454950a6d37c0\
        b854a3dc670172d95a8b8f3bc7d4dda7452d04518f7fc10b1b112d927c4f90c7\
        d1a5f8e1aa0bf6150977cd3b48e75109680487cf6654f476ab69a4d73f81eae2\
        347bc4779704541388a828d0eef8196be5e1f0d001987a5d6bb8acf526598a68\
        cad3c09ac2b6fb06cf5d0f55c780f38d83ec3af1df37f71a83a4e7266f5b1b49\
        5647c4385f24070b97979e45d62ff3c5447f0bf7a5f16e3a5d1e74b80e779ee3\
        240bf65c2eb03cb2624685aee490086b17139efa3e4479092f6ae29f54e60f5d\
        5f6d84d2b5510203010001a360305e300c0603551d130101ff04023000300e06\
        03551d0f0101ff0404030206c0301d0603551d0e041604146e4fc54272d213a0\
        26bf5d5b8b83bbd301ed45e4301f0603551d230418301680146a7d48af4c1be8\
        f361759023106e3d88080f143f300d06092a864886f70d01010b050003820101\
        007d7b30792bf616e727b695289b90a8cedc4d666f7e63f5d253160ba89935e8\
        54fc0b9f9e2f2a80e18e4e567a879ff24faad8e2951548ddd9519aeb30a278ff\
        484a423f02e0d4d14d90ac0ae18febb8fc875a24bcd4cba63abb15b344d47001\
        5bca7ad6f0d559af060f40b2a3aa8ca7f3af0ac72846b4cb8c8941c2432d4dc1\
        6096ad5d5acdb107d10b2e004482f1535906313168e5dd17ac2ecddace92d5bc\
        af9c004a9ee547a7f7ad28f303399ddb23ff00fe0877168e7f11faf1417b448b\
        74685df269ac6ac852fc8930a6c6469158580bcb26b354c030ecccf6e98711fc\
        66a780578cec85bac4e967c6637a73c054793328882283ed6ca40b4f752397de\
        da3182023830820234020101302830233121301f06035504030c1871756f7272\
        612076657264696374207465737420726f6f74020102300b0609608648016503\
        040201a081e4301806092a864886f70d010903310b06092a864886f70d010701\
        301c06092a864886f70d010905310f170d3236303931343233333334335a302f\
        06092a864886f70d01090431220420a4a4c68c931b0e4f52b432969673d37c54\
        5bc784291231a7aff3f2568fc45797307906092a864886f70d01090f316c306a\
        300b060960864801650304012a300b0609608648016503040116300b06096086\
        48016503040102300a06082a864886f70d0307300e06082a864886f70d030202\
        020080300d06082a864886f70d0302020140300706052b0e030207300d06082a\
        864886f70d0302020128300d06092a864886f70d010101050004820100743e39\
        3b999f98a3ae7292c3c44c42711a203f6b3fbb589601357b3195c0e3f2295853\
        e47f1c1c295872ff460060a1138d08d7bbdb88f797491901167841697cca3cc2\
        b75f30152b3c0930f8c2811b1e4f3a6f23bd065b304e084aab6c4face1906a57\
        dca540314cdc2b33e19184ad1b17175753f69a0f7887ce0c30d7269ac9e19435\
        29868108dcd724bc25ba346f8be94074cbdd515cfc4acdba4c863bb994591c42\
        bab5983eb95b9fd11a08c4b07173f583aa4817f66058012b624222c0d4c0ee40\
        59bba9f9ef2c1689857897b786e4b18c5397a553cbdaed43cff63071a8dde0bc\
        2e13d9a545da58bba17afa71f18e3fe0cf7db412a932e3b8080b0936bf";

    /// The self-signed root [`SIGNATURE`]'s signer chains to, 2026 to 2036.
    ///
    /// Supplied as RFC 5280 section 6.1.1's input (d) by the tests above and by nothing else in
    /// this tree: a trust anchor is a host's to name (ADR 1039), and this is a test standing in for
    /// the host a reader would be.
    pub(crate) const ROOT: &str = "\
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
