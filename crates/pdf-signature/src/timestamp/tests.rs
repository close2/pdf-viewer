//! RFC 3161 section 2.4.2 and ISO 32000-2 §12.8.5 exercised over tokens this project minted and a
//! chain this project laid out.
//!
//! **Why a token of our own.** `openssl ts` issued [`fixtures::TOKEN`] against the hierarchy
//! [`fixtures::ROOT`] anchors, once, and the DER is pasted in — the footing `crate::trust`'s tests
//! already state. No document in any corpus on this machine can stand in for it: establishing an
//! instant needs a path to an anchor, an anchor is a host's input (ADR 1039), and believing a root
//! a *file* supplies is exactly what a trust store exists to prevent. The crawl's real timestamps
//! are what `tests/signatures.rs` reads, and what they can rank is the format rather than define
//! it (trap 8).
//!
//! **Every positive has its planted negative beside it** (trap 13): the token parses, and a token
//! with one octet of its `TSTInfo` moved must stop establishing anything; the chain covers its
//! material, and a chain whose outer range is shortened must name the object it no longer covers.

#![expect(
    clippy::doc_markdown,
    reason = "RFC 3161's and RFC 5652's sentences are quoted verbatim and their ASN.1 names are \
              camel case throughout; a quotation with backticks added to please a lint is no \
              longer a quotation (the same reasoning `pdf_signature::ecdsa` records)"
)]

use pdf_syntax::Document;

use super::{
    AskedAt, Chain, ChainRefusal, Claim, Time, TokenRefusal, Unestablished, chain, established,
    signature_timestamp_established, token_of, tst_info,
};
use crate::cms::{self, Digest};
use crate::revocation::Material;
use crate::signature::{Integrity, Signature, signatures};
use crate::trust::{Trust, TrustAnchors};
use crate::x509::{Instant, parse};

/// 2026-10-01T00:00:00Z — after [`fixtures::TOKEN`]'s `genTime` and inside both certificates'
/// validity periods.
const AFTER_THE_TOKEN: Instant = Instant::from_unix_seconds(1_790_812_800);

/// `genTime` as [`fixtures::TOKEN`] states it: `20260914215011Z`.
const GEN_TIME: Instant = Instant::from_unix_seconds(1_789_422_611);

/// Hexadecimal to bytes, as the other fixture modules in this crate spell it.
fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// The contents of a DER value, without its identifier and length octets.
fn inside(encoded: &[u8]) -> &[u8] {
    let mut reader = crate::der::Reader::new(encoded).expect("a readable value");
    let value = reader
        .next_value()
        .expect("a readable value")
        .expect("one value");
    value.contents
}

/// A `SEQUENCE` around `contents`, with a length written the one way DER admits.
fn sequence(contents: &[u8]) -> Vec<u8> {
    let mut out = vec![0x30];
    let length = contents.len();
    if length < 128 {
        out.push(u8::try_from(length).expect("under 128"));
    } else if length < 256 {
        out.extend_from_slice(&[0x81, u8::try_from(length).expect("under 256")]);
    } else {
        let bytes = u16::try_from(length)
            .expect("a fixture under 64 KiB")
            .to_be_bytes();
        out.push(0x82);
        out.extend_from_slice(&bytes);
    }
    out.extend_from_slice(contents);
    out
}

/// A document timestamp dictionary carrying `contents`, with no byte range of its own.
///
/// The range is empty because these tests ask what the *token* establishes, which
/// [`super::established`] answers without reading a byte of any file: the signature is over the
/// signer's signed attributes, so RFC 5652 section 5.4 puts nothing of the document under it.
/// What the range decides is [`Integrity`], and the chain tests below are where that is asked.
fn document_timestamp(contents: Vec<u8>) -> Signature {
    Signature {
        timestamp: true,
        handler: Some("Adobe.PPKLite".to_owned()),
        sub_filter: Some("ETSI.RFC3161".to_owned()),
        byte_range: Vec::new(),
        contents,
        certificate_chain: false,
        chain: Vec::new(),
        name: None,
        signed_at: None,
        location: None,
        reason: None,
        contact: None,
        changes: None,
        certification: false,
        format_version: None,
        indirect_values: Vec::new(),
        reference_digests: Vec::new(),
    }
}

/// **Every field RFC 3161 section 2.4.2 defines, off a token an authority actually issued.**
///
/// The grammar is the clause's own — `TSTInfo ::= SEQUENCE { version, policy, messageImprint,
/// serialNumber, genTime, accuracy OPTIONAL, ordering DEFAULT FALSE, nonce OPTIONAL, tsa [0]
/// OPTIONAL, extensions [1] OPTIONAL }` — and this asserts one value per member, because a reader
/// that found the first three and stopped is how the imprint was read for six hundred sessions
/// while `genTime` went unlooked at.
#[test]
fn a_tokens_tst_info_states_every_member_rfc_3161_defines() {
    let token = hex(fixtures::TOKEN);
    let cms = cms::signed_data(&token).expect("the minted token is a SignedData");
    let info = token_of(&cms).expect("its encapsulated content is a TSTInfo");
    assert_eq!(info.version, 1, "RFC 3161's v1");
    assert_eq!(info.policy, [0x2A, 0x03, 0x04, 0x01], "1.2.3.4.1");
    assert_eq!(info.imprint_digest, Digest::Sha256);
    assert_eq!(
        info.imprint,
        Digest::Sha256.compute(&[b"quorra document timestamp fixture"]),
        "the imprint is the digest of what was stamped"
    );
    assert_eq!(info.serial_number, [0x02]);
    assert_eq!(info.gen_time, GEN_TIME);
    let accuracy = info.accuracy.expect("the token states an accuracy");
    // "accuracy = secs:1, millisecs:500, microsecs:100" is what minted it, and RFC 3161 section
    // 2.4.2 makes the three a half-width around `genTime` rather than an offset.
    assert_eq!(accuracy.seconds, 1);
    assert_eq!(accuracy.millis, 500);
    assert_eq!(accuracy.micros, 100);
    assert_eq!(accuracy.micros_total(), 1_500_100);
    assert!(info.ordering, "ordering was asked for and is TRUE");
    assert!(info.nonce.is_some(), "the query carried a nonce");
    assert!(info.tsa.is_some(), "tsa_name = yes");
    assert!(!info.extensions);
}

/// Four shapes a `TSTInfo` can have that are not one, each refused by its own name.
///
/// The calibration for the test above (trap 13): a parser that returned the same `Err` for
/// everything would pass it and say nothing. Each input here is the real token's content with one
/// thing changed, and each must produce a *different* refusal.
#[test]
fn four_things_that_stop_a_tst_info_from_reading_are_each_named() {
    let token = hex(fixtures::TOKEN);
    let cms = cms::signed_data(&token).expect("a SignedData");
    let content = cms.encapsulated.expect("the encapsulated TSTInfo");

    // Not a SEQUENCE at all.
    assert_eq!(
        tst_info(&[0x02, 0x01, 0x01]),
        Err(TokenRefusal::NotATstInfo)
    );

    // The imprint's `hashAlgorithm` moved to an identifier nothing computes: the last octet of
    // the NIST SHA-256 arc turned over.
    let mut unknown = content.to_vec();
    let at = unknown
        .windows(9)
        .position(|window| window == [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01])
        .expect("the sha256 identifier");
    unknown[at.saturating_add(8)] = 0x7F;
    assert_eq!(
        tst_info(&unknown),
        Err(TokenRefusal::ImprintDigestUnknown),
        "a digest this program does not compute is named, not guessed at"
    );

    // `genTime` spelled with a local-time offset instead of RFC 3161's "Z", which the clause
    // forbids outright: "The encoding MUST terminate with a \"Z\"".
    let mut local = content.to_vec();
    let at = local
        .windows(4)
        .position(|window| window == b"2026")
        .expect("genTime's year");
    local[at.saturating_add(14)] = b'+';
    assert_eq!(tst_info(&local), Err(TokenRefusal::GenTimeUnreadable));

    // An indefinite length where RFC 3161 section 2.4.2 requires DER: "The eContent SHALL be the
    // DER-encoded value of TSTInfo."
    let mut indefinite = vec![0x30, 0x80];
    indefinite.extend_from_slice(content.get(2..).unwrap_or_default());
    indefinite.extend_from_slice(&[0x00, 0x00]);
    assert_eq!(tst_info(&indefinite), Err(TokenRefusal::NotDerEncoded));
}

/// `genTime` with RFC 3161's fraction of a second, which RFC 5280's reader refuses by design.
///
/// The clause permits it and says why the two documents differ: "The ASN.1 GeneralizedTime syntax
/// can include fraction-of-second details. Such syntax, without the restrictions from [RFC 2459]
/// Section 4.1.2.5.2, where GeneralizedTime is limited to represent the time with a granularity of
/// one second, may be used here." This program's instants are whole seconds, so the fraction is
/// dropped — and dropping it is a decision rather than an accident, which is what the second
/// assertion pins.
#[test]
fn a_gen_time_with_a_fraction_of_a_second_reads_to_the_whole_second() {
    let token = hex(fixtures::TOKEN);
    let cms = cms::signed_data(&token).expect("a SignedData");
    let content = cms.encapsulated.expect("the TSTInfo").to_vec();
    let at = content
        .windows(15)
        .position(|window| window == b"20260914215011Z")
        .expect("genTime");
    // `18 0F "…Z"` becomes `18 12 "….75Z"`, and the enclosing SEQUENCE is re-wrapped around the
    // result rather than patched: a length octet edited by hand is how a fixture comes to test the
    // reader's tolerance for truncation instead of the thing it was written for.
    let mut inner = inside(&content).to_vec();
    let at = at.saturating_sub(content.len().saturating_sub(inner.len()));
    inner.splice(
        at.saturating_add(14)..at.saturating_add(15),
        b".75Z".iter().copied(),
    );
    inner[at.saturating_sub(1)] = 0x12;
    let fractional = sequence(&inner);
    let info = tst_info(&fractional).expect("a fractional genTime is still a genTime");
    assert_eq!(
        info.gen_time, GEN_TIME,
        "the fraction is dropped, never rounded up"
    );
}

/// **A token whose authority chains to an anchor somebody supplied states an instant.**
///
/// All four of the module comment's steps, end to end: the `TSTInfo` reads, the signer's
/// `message-digest` attribute is the digest of it, the signature verifies under the key in the
/// certificate the token carries, and RFC 5280 section 6.1 validates a path from that certificate
/// to [`fixtures::ROOT`].
///
/// **The anchor here is not a trust decision.** It is an *input*, and this test supplies one
/// precisely because no host in this tree does — which is why every real document's answer is
/// [`Trust::NoAnchorSupplied`] and why the assertion after this one is the one that matters more.
#[test]
fn a_token_that_chains_to_a_supplied_anchor_states_its_instant() {
    let token = hex(fixtures::TOKEN);
    let timestamp = document_timestamp(token);
    let root = hex(fixtures::ROOT);
    let root = parse(&root).expect("the fixture root parses");
    let anchors = TrustAnchors::of(std::slice::from_ref(&root));
    let file = pdf_syntax::FileBytes::from(Vec::new());
    let time = established(
        &timestamp,
        &file,
        &anchors,
        &Material::none(),
        AskedAt::TheCallersInstant(AFTER_THE_TOKEN),
    );
    let Time::Established { at, accuracy, .. } = &time else {
        panic!("the minted token should establish its instant, not {time:?}");
    };
    assert_eq!(*at, GEN_TIME);
    assert_eq!(accuracy.map(|value| value.seconds), Some(1));

    // **And with no anchor it establishes nothing** — this tree's own answer for every document
    // there is (ADR 1039). The `genTime` above is unchanged and unbelieved.
    let none = established(
        &timestamp,
        &file,
        &TrustAnchors::none(),
        &Material::none(),
        AskedAt::TheCallersInstant(AFTER_THE_TOKEN),
    );
    assert_eq!(
        none,
        Time::Unknown(Unestablished::AuthorityNotEstablished(
            Trust::NoAnchorSupplied
        )),
        "no anchor, no instant — and the reason says which of the four steps stopped"
    );
}

/// **§12.8.3.4.8's own instant: the token inside a signature timestamp attribute, established.**
///
/// The clause's first sentence asks for a signature to be verified "at the UTC time in the past
/// indicated in that token", and what had held it was the word *indicated*: an instant a token
/// merely states is a number a stranger wrote. [`signature_timestamp_established`] runs the same
/// four steps a document timestamp goes through over the token inside §12.8.3.4.3 (b)'s unsigned
/// attribute, so the instant either has an authority behind it or is not an instant.
///
/// The token is [`fixtures::TOKEN`], the one `openssl ts` issued, carried this time as an attribute
/// of a detached `SignedData` rather than as a `/DocTimeStamp` dictionary's value. Two things are
/// asserted with it and they are separate questions, which is ETSI EN 319 122-1 clause 5.3's own
/// division: whether the token asserts an instant at all, and whether the token is about *this*
/// signature — its imprint being the digest of the `SignerInfo`'s `signature` field.
///
/// **And the planted negatives beside them** (trap 13): with no anchor the instant is gone and the
/// reason names the step that stopped, and with one octet of `genTime` moved the token establishes
/// nothing at all.
#[test]
fn a_signature_timestamps_own_authority_is_established_or_the_instant_is_not_one() {
    let established_with = |token: Vec<u8>, anchors: &TrustAnchors<'_>| {
        // The digest the attribute is beside does not matter to any of the four steps: the token's
        // own `message-digest` binds its own `TSTInfo`, not the document's.
        let bytes = cms::fixtures::detached_with_signature_timestamp(&[0x00; 32], &token);
        let cms = cms::signed_data(&bytes).expect("a SignedData");
        signature_timestamp_established(
            &cms,
            anchors,
            &Material::none(),
            AskedAt::TheCallersInstant(AFTER_THE_TOKEN),
        )
        .expect("the fixture states the attribute")
    };
    let root = hex(fixtures::ROOT);
    let root = parse(&root).expect("the fixture root parses");
    let anchors = TrustAnchors::of(std::slice::from_ref(&root));
    let time = established_with(hex(fixtures::TOKEN), &anchors);
    let Time::Established { at, asked_at, .. } = &time else {
        panic!("the minted token should establish its instant, not {time:?}");
    };
    assert_eq!(*at, GEN_TIME);
    assert_eq!(*asked_at, AskedAt::TheCallersInstant(AFTER_THE_TOKEN));

    // No anchor, no instant — this tree's answer for every document there is (ADR 1039).
    assert_eq!(
        established_with(hex(fixtures::TOKEN), &TrustAnchors::none()),
        Time::Unknown(Unestablished::AuthorityNotEstablished(
            Trust::NoAnchorSupplied
        ))
    );

    // One octet of `genTime` moved: the signature over the attributes still verifies and the
    // attributes no longer commit to the content, which is where RFC 5652 section 5.6 stops it.
    let mut moved = hex(fixtures::TOKEN);
    let at = moved
        .windows(15)
        .position(|window| window == b"20260914215011Z")
        .expect("genTime inside the encapsulated content");
    moved[at.saturating_add(3)] = b'7';
    assert_eq!(
        established_with(moved, &anchors),
        Time::Unknown(Unestablished::ContentNotBoundToTheSignature)
    );
}

/// **ETSI EN 319 122-1 clause 5.3's imprint, which is the other half of §12.8.3.4.8's question.**
///
/// An established instant from a token about some *other* signature is not this signature's past,
/// so the two answers are kept apart: [`signature_timestamp_established`] says whether the token
/// asserts an instant, and [`SignatureTimestamp::covers_the_signature`] says whether it is about
/// this signature. Clause 5.3 and RFC 3161 Appendix A state the same rule for the imprint, which is
/// the second reading this comparison has.
#[test]
fn a_signature_timestamp_says_separately_whether_it_is_about_this_signature() {
    let about_this = |token: Vec<u8>| {
        let bytes = cms::fixtures::detached_with_signature_timestamp(&[0x00; 32], &token);
        let cms = cms::signed_data(&bytes).expect("a SignedData");
        super::signature_timestamp(&cms)
            .expect("the fixture states the attribute")
            .expect("the fixture's token reads")
            .covers_the_signature
    };
    // `fixtures::TOKEN` was minted over other octets entirely, so it is about no signature here.
    assert!(!about_this(hex(fixtures::TOKEN)));
    // A hand-built token whose imprint *is* the digest of the fixture's signature field. Nothing
    // about its authority is asserted — that is the test above — and nothing needs to be: the
    // imprint rule is arithmetic and needs no key.
    let over_the_signature = Digest::Sha256.compute(&[&[0xDE, 0xAD][..]]);
    assert!(about_this(cms::fixtures::timestamp_token(
        &over_the_signature
    )));
}

/// **The `TSTInfo` swapped under a signature that still verifies.**
///
/// RFC 5652 section 5.6 is what closes this: "For the signature to be valid, the message digest
/// value calculated by the recipient MUST be the same as the value of the messageDigest attribute
/// included in the signedAttributes of the SignedData signerInfo." The signature covers the
/// attributes and the attributes commit to the content, so a content the attributes do not commit
/// to was not signed — and a reader that checked only the signature would read a forged `genTime`
/// as an authority's statement.
///
/// The defect is planted exactly there: one octet of `genTime` is moved and nothing else. The
/// signature over the attributes still verifies, and the instant must not survive.
#[test]
fn a_tst_info_the_attributes_do_not_commit_to_is_not_a_time() {
    let mut token = hex(fixtures::TOKEN);
    let at = token
        .windows(15)
        .position(|window| window == b"20260914215011Z")
        .expect("genTime inside the encapsulated content");
    // 2026-09-14 becomes 2027-09-14: a year of free-floating evidence, if nothing checked.
    token[at.saturating_add(3)] = b'7';
    let timestamp = document_timestamp(token);
    let root = hex(fixtures::ROOT);
    let root = parse(&root).expect("the fixture root parses");
    let anchors = TrustAnchors::of(std::slice::from_ref(&root));
    let file = pdf_syntax::FileBytes::from(Vec::new());
    assert_eq!(
        established(
            &timestamp,
            &file,
            &anchors,
            &Material::none(),
            AskedAt::TheCallersInstant(AFTER_THE_TOKEN),
        ),
        Time::Unknown(Unestablished::ContentNotBoundToTheSignature),
        "the attributes are what the signature covers, and they name the content it covered"
    );
}

/// **A chain of two timestamps, and what each of them covers.**
///
/// The document is two revisions: the first carries one document timestamp, the second adds
/// §12.8.4's store with a CRL in it and a second document timestamp over the whole file. That is
/// §12.8.5.3's shape — the new token "will protect the whole structure" — and what this asserts is
/// the order, the coverage and the material.
#[test]
fn a_later_timestamp_covers_the_earlier_one_and_the_material_beside_it() {
    let bytes = fixtures::two_timestamps(false);
    let document = Document::open(bytes).expect("the fixture opens");
    let found = signatures(&document);
    assert_eq!(found.len(), 2, "two signature dictionaries, got {found:?}");
    let chain = chain(
        &document,
        &TrustAnchors::none(),
        &Material::none(),
        AFTER_THE_TOKEN,
    );
    println!("{chain:#?}");
    assert_eq!(chain.signatures, 0, "both dictionaries are timestamps");
    let [inner, outer] = chain.links.as_slice() else {
        panic!("two links, not {:?}", chain.links.len());
    };
    assert!(
        inner.covers_to < outer.covers_to,
        "the links are ordered by how much of the file each range names"
    );
    assert!(inner.ends_a_revision && outer.ends_a_revision);
    assert_eq!(
        inner.integrity,
        Integrity::Unchanged {
            digest: Digest::Sha256
        },
        "the earlier token's imprint is the digest of the revision it signed"
    );
    assert_eq!(
        outer.integrity,
        Integrity::Unchanged {
            digest: Digest::Sha256
        }
    );
    assert!(
        inner.covers.is_empty(),
        "nothing precedes the first timestamp"
    );
    assert_eq!(outer.covers, vec![0], "the later one covers the earlier");
    assert!(
        inner.material_covered.is_empty(),
        "the CRL is in a revision the first timestamp predates"
    );
    assert_eq!(
        outer.material_covered.len(),
        1,
        "and the second covers it: {:?}",
        outer.material_uncovered
    );
    assert!(
        chain.refused.is_empty(),
        "an honest chain refuses nothing: {:?}",
        chain.refused
    );
    // Both tokens read as `TSTInfo`s and neither establishes anything, which is the one sentence
    // this program may say about a real document's timestamp (ADR 1039). These two stop at step 3
    // rather than step 4 — the hand-built token's `SignerInfo` names an algorithm that is a digest
    // rather than a signature — and the token test above is where all four steps are walked.
    for link in &chain.links {
        assert!(matches!(link.claim, Ok(Claim { .. })));
        assert!(
            matches!(link.time, Time::Unknown(_)),
            "nothing here may be a time: {:?}",
            link.time
        );
    }
}

/// **The same chain with the outer range shortened, which must name what it stopped covering.**
///
/// The calibration for the test above (trap 13). `shrink` pulls the second timestamp's
/// `/ByteRange` back past the CRL's object, so a chain that reported coverage without looking
/// would answer identically and this must not: §12.8.5.3 requires the material "shall be included
/// into the DSS dictionary" under a token that protects it, and material below no token's range is
/// material nothing protects.
#[test]
fn a_range_that_stops_short_of_the_material_names_the_object_it_left_out() {
    let bytes = fixtures::two_timestamps(true);
    let document = Document::open(bytes).expect("the shortened fixture opens");
    let chain = chain(
        &document,
        &TrustAnchors::none(),
        &Material::none(),
        AFTER_THE_TOKEN,
    );
    println!("{:#?}", chain.refused);
    let outer = chain.outermost().expect("two links");
    assert_eq!(
        outer.material_uncovered.len(),
        1,
        "the CRL is now outside the outermost range"
    );
    assert!(
        chain
            .refused
            .iter()
            .any(|refusal| matches!(refusal, ChainRefusal::DoesNotCoverMaterial { index: 1, .. })),
        "the object it left out is named: {:?}",
        chain.refused
    );
    assert!(
        chain
            .refused
            .iter()
            .any(|refusal| matches!(refusal, ChainRefusal::RangeDoesNotCoverTheFile { .. })),
        "and §12.8.5.2's whole-file requirement is named too: {:?}",
        chain.refused
    );
}

/// A document with no timestamp at all yields an empty chain and no refusal.
///
/// The condition a report fires on is as much a part of it as what it says (trap 11): a chain
/// that named a refusal for a document carrying no timestamp would fire on every file there is.
#[test]
fn a_document_with_no_timestamp_has_no_chain_and_nothing_to_refuse() {
    let document = Document::open(fixtures::unsigned()).expect("a valid file");
    let chain: Chain = chain(
        &document,
        &TrustAnchors::none(),
        &Material::none(),
        AFTER_THE_TOKEN,
    );
    assert!(chain.is_empty());
    assert!(chain.outermost().is_none());
    assert_eq!(chain.signatures, 0);
    assert!(chain.refused.is_empty());
}

pub(crate) mod fixtures {
    use std::fmt::Write as _;

    use crate::cms::Digest;

    /// A two-revision document: a timestamp, then a store and a second timestamp over both.
    ///
    /// `material_last` puts §12.8.4's store in a *third* revision instead, which no timestamp
    /// covers — §12.8.5.3's own failure, since that clause has the material go in before the new
    /// token is applied: "The certificates, CRLs or OCSP responses used to demonstrate that the
    /// certificates related to the certification path of the previous timestamp token was not
    /// revoked … shall be included into the DSS dictionary. Then the new timestamp token shall be
    /// placed into a new document timestamp dictionary which will protect the whole structure."
    ///
    /// The offsets are recorded as the bytes are laid down rather than patched afterwards, so the
    /// cross-reference tables are right by construction.
    pub(crate) fn two_timestamps(material_last: bool) -> Vec<u8> {
        /// Hexadecimal characters reserved for each signature value.
        const ROOM: usize = 1024;
        let value = format!("<{}>", "0".repeat(ROOM));
        let timestamp = format!(
            "<< /Type /DocTimeStamp /Filter /Adobe.PPKLite /SubFilter /ETSI.RFC3161 \
             /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents {value} >>"
        );
        let mut file = Layout::default();
        file.bytes.extend_from_slice(b"%PDF-1.7\n");
        file.put(
            1,
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
        );
        file.put(2, "<< /Type /Pages /Count 0 /Kids [] >>");
        file.put(4, "<< /FT /Sig /T (ts1) /V 5 0 R /Subtype /Widget >>");
        file.put(5, &timestamp);
        let first_xref = file.section(6, None);
        let first_revision = file.bytes.len();

        let mut store_at = 0;
        if material_last {
            file.put(
                1,
                "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 7 0 R] /SigFlags 3 >> >>",
            );
        } else {
            file.put(
                1,
                "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 7 0 R] /SigFlags 3 >> \
                 /DSS << /CRLs [6 0 R] >> >>",
            );
            store_at = file.bytes.len();
            file.put(6, "<< /Length 8 >>\nstream\nnot a CRL\nendstream");
        }
        file.put(7, "<< /FT /Sig /T (ts2) /V 8 0 R /Subtype /Widget >>");
        file.put(8, &timestamp);
        let second_xref = file.section(9, Some(first_xref));
        let second_revision = file.bytes.len();

        if material_last {
            file.put(
                1,
                "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 7 0 R] /SigFlags 3 >> \
                 /DSS << /CRLs [6 0 R] >> >>",
            );
            store_at = file.bytes.len();
            file.put(6, "<< /Length 8 >>\nstream\nnot a CRL\nendstream");
            file.section(9, Some(second_xref));
            assert!(
                store_at > second_revision,
                "the planted defect is material the last timestamp cannot reach"
            );
        }
        let _ = store_at;
        for (which, end) in [first_revision, second_revision].into_iter().enumerate() {
            fill(&mut file.bytes, which, end, ROOM);
        }
        file.bytes
    }

    /// A file being laid down, with the offset of every object written so far.
    #[derive(Default)]
    struct Layout {
        bytes: Vec<u8>,
        /// The current revision's objects, cleared by each [`Self::section`].
        offsets: Vec<(u32, usize)>,
    }

    impl Layout {
        /// One indirect object, at wherever the file has reached.
        fn put(&mut self, number: u32, body: &str) {
            self.offsets.push((number, self.bytes.len()));
            self.bytes
                .extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
        }

        /// One cross-reference section and the trailer after it, returning its own offset.
        fn section(&mut self, size: u32, previous: Option<usize>) -> usize {
            let at = self.bytes.len();
            let mut table = String::from("xref\n0 1\n0000000000 65535 f \n");
            for (number, offset) in &self.offsets {
                let _ = write!(table, "{number} 1\n{offset:010} 00000 n \n");
            }
            let _ = write!(table, "trailer\n<< /Size {size} /Root 1 0 R");
            if let Some(previous) = previous {
                let _ = write!(table, " /Prev {previous}");
            }
            let _ = write!(table, " >>\nstartxref\n{at}\n%%EOF\n");
            self.bytes.extend_from_slice(table.as_bytes());
            self.offsets.clear();
            at
        }
    }

    /// Writes the `which`th `/ByteRange` to end at `end`, and the token that commits to it.
    fn fill(bytes: &mut Vec<u8>, which: usize, end: usize, room: usize) {
        let placeholder = b"[0000000000 0000000000 0000000000 0000000000]";
        // The first placeholder left, not the `which`th: each call consumes one, and the values
        // are written in the order the objects stand in the file.
        let range_at = positions(bytes, placeholder)
            .first()
            .copied()
            .expect("a /ByteRange placeholder");
        let value_at = positions(bytes, b"/Contents <")
            .get(which)
            .copied()
            .expect("a /Contents hole")
            .saturating_add(10);
        let before = value_at;
        let after = value_at.saturating_add(room).saturating_add(2);
        assert!(after <= end, "the hole is inside the range it belongs to");
        let tail = end.saturating_sub(after);
        let range = format!("[{:010} {before:010} {after:010} {tail:010}]", 0);
        bytes.splice(
            range_at..range_at.saturating_add(range.len()),
            range.bytes(),
        );
        let digest = Digest::Sha256.compute(&[&bytes[..before], &bytes[after..end]]);
        let token = crate::cms::fixtures::timestamp_token(&digest);
        let hex = token.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
        assert!(hex.len() <= room, "the token fits its hole");
        let at = value_at.saturating_add(1);
        bytes.splice(at..at.saturating_add(hex.len()), hex.bytes());
    }

    /// Every offset at which `needle` occurs.
    fn positions(bytes: &[u8], needle: &[u8]) -> Vec<usize> {
        bytes
            .windows(needle.len())
            .enumerate()
            .filter(|(_, window)| *window == needle)
            .map(|(at, _)| at)
            .collect()
    }

    /// A document with no signature dictionary at all.
    pub(crate) fn unsigned() -> Vec<u8> {
        let mut file = Layout::default();
        file.bytes.extend_from_slice(b"%PDF-1.7\n");
        file.put(1, "<< /Type /Catalog /Pages 2 0 R >>");
        file.put(2, "<< /Type /Pages /Count 0 /Kids [] >>");
        file.section(3, None);
        file.bytes
    }

    /// A time-stamp token `openssl ts` issued against [`ROOT`]'s hierarchy.
    ///
    /// Minted once over the octets `quorra document timestamp fixture`, with every optional
    /// `TSTInfo` member asked for — accuracy, ordering, nonce and the `tsa` hint — so that one
    /// vector exercises the whole of RFC 3161 section 2.4.2's grammar. The signer is an end-entity
    /// certificate asserting `id-kp-timeStamping`, issued by [`ROOT`] and carried inside the token
    /// itself, which is what lets a path be built from the token alone.
    pub(crate) const TOKEN: &str = "\
        3082062306092a864886f70d010702a082061430820610020103310f300d0609\
        60864801650304020105003081af060b2a864886f70d0109100104a0819f0481\
        9c30819902010106042a0304013031300d060960864801650304020105000420\
        0a46e125b889aa3af86e06a8c3c3054aa07cba2dc0793fabb2d8d1836de412b5\
        020102180f32303236303931343231353031315a300a020101800201f4810164\
        0101ff020861c5dd848f008777a02ea42c302a3128302606035504030c1f7175\
        6f72726120746573742074696d657374616d7020617574686f72697479a08203\
        46308203423082022aa003020102020102300d06092a864886f70d01010b0500\
        30253123302106035504030c1a71756f7272612074696d657374616d70207465\
        737420726f6f74301e170d3236303130313030303030305a170d333630313031\
        3030303030305a302a3128302606035504030c1f71756f727261207465737420\
        74696d657374616d7020617574686f7269747930820122300d06092a864886f7\
        0d01010105000382010f003082010a0282010100ab618b0acbed2b4de81a7b3f\
        f0af7f234268aa77a1f6ee7bf7b89b699e5734f7ecff4477065414fdd9e28ed3\
        5f3147d4859eb31961de5fb96704976280f4ed88892c2d68fe548039d4020eb9\
        e97d9306eccd2267cc96ad4a5a1332378653efbf5c6ff6458ef8114286f8799c\
        b583b6c08258dc1d769398c22572986338ab94fd9266a110d391ddac8c9e551b\
        b85d8ca5414befa4b44936d64b5b79f4dbfd40311e193b800f97f08349bb076f\
        437de3cc41af3d87801cbe21a4c2db2ec42687201ee37b0b323f4bfe9ec88755\
        f13a0ec39ef04d3fb202d942b82662797b38a1d3cc7b075316bc59fbd6080b79\
        68dc903760b574723a89f31d621dff2d66a372710203010001a3783076300c06\
        03551d130101ff04023000300e0603551d0f0101ff0404030206c03016060355\
        1d250101ff040c300a06082b06010505070308301d0603551d0e04160414c564\
        1f2a12f3ca3c09de93e1eb69f1065c5e82cc301f0603551d2304183016801466\
        2bd2b95c0bbe1458ea120e2e33810a5adf97ad300d06092a864886f70d01010b\
        05000382010100b705cefce176c9460d11281c14dc684d8675468dfbba78ce00\
        19814aaae3a6426badc52562ec6c7991097a844788299c90254fb329ee12d9e6\
        74182dafab0bc813a2375c7776f3f06d8d38eab3edf9b12af0e804a73b00f501\
        fbd0e574c7ecf93302f1de561b34e33b919004c6e848de57149ae794e90691d7\
        5ab624bbe862dc14fef3fd88e52d209c1a975e0b7bdd322a4203cd3be1899798\
        149926346d82cff73118f278aea1e6f6897baece383c84163ea232b9bce9d17f\
        7553ccf0bc7b8d644f070f1ae0ea2d3b2d989992a0a20c2bcf192983155a8c79\
        a5f2f2769ac3498081890142dedf5e3c1bd139864fbba90ffd1271493d8e5ee8\
        25dff1db745c01318201fc308201f8020101302a30253123302106035504030c\
        1a71756f7272612074696d657374616d70207465737420726f6f74020102300d\
        06096086480165030402010500a081a4301a06092a864886f70d010903310d06\
        0b2a864886f70d0109100104301c06092a864886f70d010905310f170d323630\
        3931343231353031315a302f06092a864886f70d010904312204204f463455c9\
        4803978f0cf552a17e12355b2eb72d65c69c1740162d88e368449f3037060b2a\
        864886f70d010910022f31283026302430220420ed2c81d4dcb473e99d7edaa1\
        54f8ca3ba34ae893459d3d0a6adc557217973809300d06092a864886f70d0101\
        01050004820100869617e4272d1803a3d8d38a312c3a3515ed90fd4c40f50fed\
        890aad75006a0c525d7c815c9d1aeb6308d726dff70843616719679934eb0f5f\
        88339877d9ceb8e8c84c544c18e7d3cf098f6e7d31acb9fac9b59c865e7f2fbd\
        714aefc587c3dd4455e3eec0ffb115e9a5ef65c28289fc14cda9e9c5cea1b047\
        3f7ba11862fac47b8c69eeff3b5bedcc17891d4d46ce7a104a7858cf5dcaf8ae\
        dec0f61841a0d49692f3b9e849442d719cf4e0c025edff76ec499903efa26420\
        e4793a94a2e509b302653900b5309a23d8af0d61d0937eb1c88e503acea32e4e\
        dc42445ab6bdd6d662b6fe5b436c5ad161c9b597ffffc6774d53486ea3df2f68\
        3f0ace953598ea";

    /// The self-signed root [`TOKEN`]'s authority chains to, 2026 to 2036.
    ///
    /// Supplied as RFC 5280 section 6.1.1's input (d) by the tests above and by nothing else in
    /// this tree: a trust anchor is a host's to name, and no host here names one (ADR 1039).
    pub(crate) const ROOT: &str = "\
        30820307308201efa003020102020101300d06092a864886f70d01010b050030\
        253123302106035504030c1a71756f7272612074696d657374616d7020746573\
        7420726f6f74301e170d3236303130313030303030305a170d33363031303130\
        30303030305a30253123302106035504030c1a71756f7272612074696d657374\
        616d70207465737420726f6f7430820122300d06092a864886f70d0101010500\
        0382010f003082010a0282010100d4a8bef122732785e7d582c17579bf94a588\
        1392917b6cfb4f1f518c04034514d5c5c3bbb1d51543546945ad81dbe6796198\
        30056805fd28d42ac4e14725bb8e81711cb219fad9c2f319a08b42aebe57af3b\
        afb77e1b7e8da73d13d17dbe045605c5eeaa3481c11b4a58bac6be4c8688afbb\
        db85490a09c1972c5f5cbef744a0eea37ee2566bb6e1ef3384835de1073f3987\
        0fcf66d7a4c6317a9fe191b339a7ea100b3e174537811edc5f977db0c43ad8f1\
        8f65ed338b8d4d5fe0af031458a0db201884163a62793a8ff8eaaec98c6a7eb1\
        a1323dc1c63bad9369530f7f79bbc789ecd7f5ac2a9701064c93d679488fbee6\
        8e50911ad5dc92bd3f72aa9e75910203010001a3423040300f0603551d130101\
        ff040530030101ff300e0603551d0f0101ff040403020106301d0603551d0e04\
        160414662bd2b95c0bbe1458ea120e2e33810a5adf97ad300d06092a864886f7\
        0d01010b05000382010100b05000a6086339a745cf449dc051507c24b8f06123\
        438269e9f7d2a9b9ffdb6454676f1b16247cf10920e7a22c00ac5bdfbf8eb9c6\
        fa8ccab8b93aca14418ebde993b58cd93925550da669125bd6159446dc8104f6\
        c714d4a03d8f6d8e51728b3fecd565bea285e159f45e29a6d1839e7e28b6f819\
        6818acccc75e6e9d5d79fe27af6488a9f732ae8b692aa727c69e4f734298e4e6\
        ff01841436ece62e42e4c8ca1a129d695a410108029e2332f67391ee22850746\
        e32ef542365865a3c29bf037991fd579f478913c5933c0fdda038f37185cc290\
        b435b486eb79de97ceed65264d3e86cf4338f7619c5c434cfd3ef967c2940ff4\
        26923dac037441af627589";
}
