//! SAML 2.0 core's assertion, read and its signature verified, over assertions written here.
//!
//! **Every expected value is derived by hand from SAML 2.0 core and XML Signature, not computed by
//! the code under test.** A signed fixture is written already in its exclusive canonical form, as
//! `crate::xml_signature::tests` writes its lists: the root declares the one prefix it uses, its
//! attributes are in order (`ID`, `IssueInstant`, `Version`), and no element is empty in the short
//! form. So the octets the reference digests are the fixture's text with the `ds:Signature`
//! element cut out (XML Signature 1.1 section 6.6.4), and `ds:SignedInfo`'s canonical form is its
//! text with the `ds` declaration written on it.

use super::*;
use crate::cms::Digest;
use crate::policy_rules::canonical_xml::EXC_C14N;
use crate::trusted_list::tests::base64_of;
use crate::xml_signature::tests::{Key, LOTL_KEY, MEMBER_KEY};
use crate::xml_signature::{DSIG, ENVELOPED_SIGNATURE};

/// The assertion's start tag, in canonical form.
pub(crate) const ROOT: &str = "<saml:Assertion xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\" \
                               ID=\"A1\" IssueInstant=\"2026-01-01T00:00:00Z\" Version=\"2.0\">";

/// The issuer, section 2.3.3's first child.
const ISSUER: &str = "<saml:Issuer>https://attributes.example</saml:Issuer>";

/// A subject, an interval of 2026, and one attribute statement: `id-at-role` named as SAML 2.0
/// profiles section 8.2.2 names an X.500 type, and a `basic`-format attribute beside it.
pub(crate) const BODY: &str = "<saml:Subject><saml:NameID>signer@example</saml:NameID></saml:Subject>\
    <saml:Conditions NotBefore=\"2026-01-01T00:00:00Z\" NotOnOrAfter=\"2027-01-01T00:00:00Z\">\
    </saml:Conditions><saml:AttributeStatement>\
    <saml:Attribute FriendlyName=\"role\" Name=\"urn:oid:2.5.4.72\" \
    NameFormat=\"urn:oasis:names:tc:SAML:2.0:attrname-format:uri\">\
    <saml:AttributeValue>sales director</saml:AttributeValue></saml:Attribute>\
    <saml:Attribute Name=\"department\" \
    NameFormat=\"urn:oasis:names:tc:SAML:2.0:attrname-format:basic\">\
    <saml:AttributeValue>sales</saml:AttributeValue>\
    <saml:AttributeValue>export</saml:AttributeValue></saml:Attribute>\
    </saml:AttributeStatement>";

/// The assertion unsigned.
pub(crate) fn unsigned(body: &str) -> String {
    format!("{ROOT}{ISSUER}{body}</saml:Assertion>")
}

/// The assertion signed by `key` under section 5.4's profile, the reference naming `uri`, with or
/// without the signer's certificate in `ds:KeyInfo`.
pub(crate) fn signed_with(body: &str, key: Key, uri: &str, key_info: bool) -> String {
    let covered = unsigned(body);
    let digest = base64_of(&Digest::Sha256.compute(&[covered.as_bytes()]));
    let inner = format!(
        "<ds:CanonicalizationMethod Algorithm=\"{EXC_C14N}\"></ds:CanonicalizationMethod>\
         <ds:SignatureMethod Algorithm=\"http://www.w3.org/2001/04/xmldsig-more#rsa-sha256\">\
         </ds:SignatureMethod><ds:Reference URI=\"{uri}\"><ds:Transforms>\
         <ds:Transform Algorithm=\"{ENVELOPED_SIGNATURE}\"></ds:Transform>\
         <ds:Transform Algorithm=\"{EXC_C14N}\"></ds:Transform></ds:Transforms>\
         <ds:DigestMethod Algorithm=\"http://www.w3.org/2001/04/xmlenc#sha256\"></ds:DigestMethod>\
         <ds:DigestValue>{digest}</ds:DigestValue></ds:Reference>"
    );
    let canonical_info = format!("<ds:SignedInfo xmlns:ds=\"{DSIG}\">{inner}</ds:SignedInfo>");
    let value = base64_of(&key.sign(canonical_info.as_bytes()));
    let key_info = if key_info {
        format!(
            "<ds:KeyInfo><ds:X509Data><ds:X509Certificate>{}</ds:X509Certificate></ds:X509Data>\
             </ds:KeyInfo>",
            base64_of(&key.certificate())
        )
    } else {
        String::new()
    };
    format!(
        "{ROOT}{ISSUER}<ds:Signature xmlns:ds=\"{DSIG}\"><ds:SignedInfo>{inner}</ds:SignedInfo>\
         <ds:SignatureValue>{value}</ds:SignatureValue>{key_info}</ds:Signature>{body}\
         </saml:Assertion>"
    )
}

/// The assertion signed as section 5.4.2 requires, by the fixture key `key`.
pub(crate) fn signed(body: &str, key: Key) -> String {
    signed_with(body, key, "#A1", true)
}

fn at(seconds: i64) -> Instant {
    Instant::from_unix_seconds(seconds)
}

/// 2026-01-01T00:00:00Z and 2027-01-01T00:00:00Z.
const START: i64 = 1_767_225_600;
const END: i64 = 1_798_761_600;

/// Section 2.3.3's members, section 2.5.1's interval and section 2.7.3.1's attributes, as written.
#[test]
fn an_assertion_is_read_as_section_2_3_3_states_it() {
    let read = read(unsigned(BODY).as_bytes()).expect("an assertion");
    assert_eq!(read.id, "A1");
    assert_eq!(read.issue_instant, "2026-01-01T00:00:00Z");
    assert_eq!(read.issuer, "https://attributes.example");
    assert_eq!(read.subject.as_deref(), Some("signer@example"));
    assert_eq!(
        read.not_before,
        Some(("2026-01-01T00:00:00Z".to_owned(), at(START)))
    );
    assert_eq!(
        read.not_on_or_after,
        Some(("2027-01-01T00:00:00Z".to_owned(), at(END)))
    );
    assert!(!read.signed);
    assert_eq!(
        read.attributes,
        vec![
            Attribute {
                name: "urn:oid:2.5.4.72".to_owned(),
                name_format: Some(URI_NAME_FORMAT.to_owned()),
                friendly_name: Some("role".to_owned()),
                values: vec!["sales director".to_owned()],
            },
            Attribute {
                name: "department".to_owned(),
                name_format: Some("urn:oasis:names:tc:SAML:2.0:attrname-format:basic".to_owned()),
                friendly_name: None,
                values: vec!["sales".to_owned(), "export".to_owned()],
            },
        ]
    );
    // Profiles section 8.2.2: `urn:oid:2.5.4.72` under the URI format is `id-at-role`, whose
    // contents octets are 2*40+5 = 0x55, 4, 72 = 0x48; a `basic` name is no object identifier.
    assert_eq!(
        read.attributes[0].object_identifier(),
        Some(vec![0x55, 0x04, 0x48])
    );
    assert_eq!(read.attributes[1].object_identifier(), None);
}

/// What is not an assertion is said, by the member that fails.
#[test]
fn what_is_not_section_2_3_3s_assertion_is_named() {
    let version = unsigned("").replace("Version=\"2.0\"", "Version=\"1.1\"");
    assert_eq!(
        read(version.as_bytes()),
        Err(Unreadable::Version("1.1".to_owned()))
    );
    let no_issuer = unsigned("").replace(ISSUER, "");
    assert_eq!(
        read(no_issuer.as_bytes()),
        Err(Unreadable::Malformed("Issuer"))
    );
    let other = "<saml:Issuer xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\">x</saml:Issuer>";
    assert_eq!(read(other.as_bytes()), Err(Unreadable::NotAnAssertion));
    // Section 1.3.3: a time value in UTC; an offset is not that form.
    let offset = unsigned("").replace("00:00:00Z", "00:00:00+01:00");
    assert_eq!(
        read(offset.as_bytes()),
        Err(Unreadable::Malformed("IssueInstant"))
    );
    // Section 2.5.1.2: the start earlier than the end.
    let reversed = unsigned(
        "<saml:Conditions NotBefore=\"2027-01-01T00:00:00Z\" \
         NotOnOrAfter=\"2026-01-01T00:00:00Z\"></saml:Conditions>",
    );
    assert_eq!(
        read(reversed.as_bytes()),
        Err(Unreadable::Malformed("Conditions"))
    );
    // The bounds hold before the work.
    let long = " ".repeat(MAX_OCTETS + 1);
    assert_eq!(
        read(long.as_bytes()),
        Err(Unreadable::TooLong(MAX_OCTETS + 1))
    );
    let entity = format!("<!DOCTYPE a [<!ENTITY x \"y\">]>{}", unsigned(""));
    assert!(matches!(read(entity.as_bytes()), Err(Unreadable::Xml(_))));
}

/// Section 2.5.1.2's interval includes its start and excludes its end; section 2.5.1.4's audience
/// restriction names no verifier, so it cannot be evaluated (section 2.5.1.1 rule 3).
#[test]
fn conditions_hold_from_not_before_until_not_on_or_after() {
    let read = read(unsigned(BODY).as_bytes()).expect("an assertion");
    assert_eq!(read.conditions_at(at(START - 1)), Conditions::Invalid);
    assert_eq!(read.conditions_at(at(START)), Conditions::Valid);
    assert_eq!(read.conditions_at(at(END - 1)), Conditions::Valid);
    assert_eq!(read.conditions_at(at(END)), Conditions::Invalid);
    let audience = unsigned(
        "<saml:Conditions><saml:AudienceRestriction><saml:Audience>https://rp.example\
         </saml:Audience></saml:AudienceRestriction><saml:OneTimeUse></saml:OneTimeUse>\
         </saml:Conditions>",
    );
    let read = super::read(audience.as_bytes()).expect("an assertion");
    assert!(matches!(
        read.conditions_at(at(START)),
        Conditions::Indeterminate(_)
    ));
    // Section 2.5.1.5: `<OneTimeUse>` alone is always valid.
    let once = unsigned("<saml:Conditions><saml:OneTimeUse></saml:OneTimeUse></saml:Conditions>");
    let read = super::read(once.as_bytes()).expect("an assertion");
    assert_eq!(read.conditions_at(at(START)), Conditions::Valid);
}

/// Section 5.4: the one reference names the assertion's `ID`, and the value verifies under the
/// certificate `ds:KeyInfo` carries.
#[test]
fn a_signed_assertion_verifies_under_section_5_4s_profile() {
    let saml = Saml::of(signed(BODY, MEMBER_KEY).as_bytes());
    assert_eq!(
        saml.signature,
        SignatureCheck::Verified(MEMBER_KEY.certificate())
    );
    assert!(saml.assertion.as_ref().is_ok_and(|read| read.signed));
    assert_eq!(saml.attributes_relied_on().len(), 2);
}

/// The plant: an attribute value changed after signing fails the reference's digest, and section
/// 2.3.3 forbids relying on the contents, so no attribute is handed on.
#[test]
fn an_assertion_changed_after_signing_is_not_relied_on() {
    let changed = signed(BODY, MEMBER_KEY).replace("sales director", "chief executive");
    let saml = Saml::of(changed.as_bytes());
    assert_eq!(
        saml.signature,
        SignatureCheck::Failed(XmlSignatureError::DigestDiffers("#A1".to_owned()))
    );
    assert!(saml.signature.is_invalid());
    assert_eq!(saml.attributes_relied_on(), []);
}

/// A value made by another key does not verify under the carried certificate.
#[test]
fn a_value_by_another_key_is_invalid() {
    let value_of = |text: &str| {
        let start = text.find("<ds:SignatureValue>").expect("a value");
        let end = text.find("</ds:SignatureValue>").expect("a value");
        text.get(start..end).expect("a value").to_owned()
    };
    let ours = signed(BODY, MEMBER_KEY);
    let theirs = signed(BODY, LOTL_KEY);
    let forged = ours.replace(&value_of(&ours), &value_of(&theirs));
    assert_eq!(
        Saml::of(forged.as_bytes()).signature,
        SignatureCheck::Failed(XmlSignatureError::SignatureDiffers)
    );
}

/// Section 5.4.5 lets `ds:KeyInfo` be absent; then no key is known, and the signature is said as
/// unverified, which is not invalid, so the attributes are shown as read.
#[test]
fn without_a_certificate_the_signature_is_unverified_not_invalid() {
    let saml = Saml::of(signed_with(BODY, MEMBER_KEY, "#A1", false).as_bytes());
    assert_eq!(
        saml.signature,
        SignatureCheck::Failed(XmlSignatureError::NoKey)
    );
    assert!(!saml.signature.is_invalid());
    assert_eq!(saml.attributes_relied_on().len(), 2);
    // The reference is still checked: a changed assertion is invalid with or without a key.
    let changed = signed_with(BODY, MEMBER_KEY, "#A1", false).replace("sales director", "ceo");
    let saml = Saml::of(changed.as_bytes());
    assert_eq!(
        saml.signature,
        SignatureCheck::Failed(XmlSignatureError::DigestDiffers("#A1".to_owned()))
    );
    assert_eq!(saml.attributes_relied_on(), []);
}

/// Section 5.4.2: the reference names the root's `ID`; one naming the whole document instead is
/// not the profile's, whatever its digest.
#[test]
fn a_reference_not_to_the_assertions_id_is_refused() {
    let saml = Saml::of(signed_with(BODY, MEMBER_KEY, "", true).as_bytes());
    assert_eq!(
        saml.signature,
        SignatureCheck::Failed(XmlSignatureError::AssertionNotCovered)
    );
    assert!(saml.signature.is_invalid());
}
