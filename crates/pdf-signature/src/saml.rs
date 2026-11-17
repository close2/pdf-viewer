//! The SAML 2.0 assertions a signature's `signer-attributes-v2` carries: ETSI EN 319 122-1 clause
//! 5.2.6.2's `claimed-SAML-assertion` among the claimed attributes, and clause 5.2.6.3's
//! `signed-SAML-assertion` among the signed assertions (ADR 2922). Both are an `OCTET STRING`
//! holding the byte representation of a SAML assertion, the second of a signed one. The ETSI text
//! is cited by clause and paraphrased (ADR 1085); OASIS SAML 2.0 core (`saml-core-2.0-os`) and
//! profiles (`saml-profiles-2.0-os`) are cited by section.
//!
//! # What is read
//!
//! Core section 2.3.3's `<Assertion>`: its `Version` (`2.0`), `ID` and `IssueInstant`, its
//! `<Issuer>` (section 2.2.5), its `<Subject>`'s name identifier (section 2.4.1), its
//! `<Conditions>` (section 2.5.1) and every `<AttributeStatement>`'s `<Attribute>` with its
//! `Name`, `NameFormat`, `FriendlyName` and the character data of each `<AttributeValue>`
//! (sections 2.7.3 and 2.7.3.1). Time values are UTC (section 1.3.3). Nothing else of the
//! assertion is interpreted, and an `<EncryptedAttribute>` is counted, never opened: its key is
//! not this reader's.
//!
//! # What is decided
//!
//! - **The signature.** Section 2.3.3 says that where an assertion is signed the relying party
//!   verifies the signature by XML Signature, and does not rely on the contents of one whose
//!   signature is invalid. So a signed assertion is verified by section 5.4's profile
//!   ([`crate::xml_signature::verify_saml`]), and where the signature does not verify,
//!   [`Saml::attributes_relied_on`] is empty and the reason is said. A signature that cannot be
//!   verified here (no key carried, an algorithm not computed) is said as unverified, never as
//!   valid.
//! - **The conditions.** Section 2.5.1.1's rules over section 2.5.1.2's interval at a given
//!   instant ([`Assertion::conditions_at`]). Section 2.5.1.4's audience restriction is valid only
//!   for a relying party among its audiences, and a signature's verifier is named by none, so it
//!   cannot be evaluated; section 2.5.1.5's `<OneTimeUse>` and section 2.5.1.6's
//!   `<ProxyRestriction>` are always valid; a `<Condition>` of an extension type is not
//!   understood. The latter two kinds make the conditions indeterminate.
//!
//! # Bounds
//!
//! The bytes are a stranger's: an assertion longer than [`MAX_OCTETS`] is not read, every element
//! and namespace declaration is charged to [`MAX_ENTRIES`] by the crate's bounded XML reader (which
//! refuses a document type declaration), and at most [`MAX_ATTRIBUTES`] attributes of at most
//! [`MAX_VALUES`] values each are kept.

use crate::x509::Instant;
use crate::xml::{self, Element, Tree};
use crate::xml_signature::{self, DSIG, XmlSignatureError};

/// SAML 2.0 core section 2.1's assertion namespace.
pub const ASSERTION_NAMESPACE: &str = "urn:oasis:names:tc:SAML:2.0:assertion";

/// SAML 2.0 core section 8.2.2's attribute name format of a URI reference.
pub const URI_NAME_FORMAT: &str = "urn:oasis:names:tc:SAML:2.0:attrname-format:uri";

/// EN 319 122-1 clause 5.2.6.2's `id-aa-ets-claimedSAML`, 0.4.0.19122.1.2, as contents octets.
pub const ID_AA_ETS_CLAIMED_SAML: &[u8] = &[0x04, 0x00, 0x81, 0x95, 0x32, 0x01, 0x02];

/// EN 319 122-1 clause 5.2.6.3's `id-ets-signedSAML`, 0.4.0.19122.3.0, as contents octets.
pub const ID_ETS_SIGNED_SAML: &[u8] = &[0x04, 0x00, 0x81, 0x95, 0x32, 0x03, 0x00];

/// The longest assertion read, in octets.
pub const MAX_OCTETS: usize = 64 * 1024;

/// The elements and namespace declarations one assertion may hold.
pub const MAX_ENTRIES: usize = 4096;

/// The attributes kept of one assertion.
pub const MAX_ATTRIBUTES: usize = 64;

/// The values kept of one attribute.
pub const MAX_VALUES: usize = 64;

/// Why an assertion's bytes are not read as a SAML 2.0 assertion.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Unreadable {
    /// Longer than [`MAX_OCTETS`]; how long.
    #[error("it is {0} octets long, past the {MAX_OCTETS} this program reads")]
    TooLong(usize),
    /// Not well-formed XML, or past the entry budget; the reason is said.
    #[error("its XML cannot be read: {0}")]
    Xml(String),
    /// The root is not SAML 2.0 core section 2.3.3's `<Assertion>`.
    #[error("its root is not a SAML 2.0 Assertion element")]
    NotAnAssertion,
    /// `Version` is not `2.0` (SAML 2.0 core section 4.1); what it is.
    #[error("it states SAML version {0:?} where SAML 2.0 core section 2.3.3 defines \"2.0\"")]
    Version(String),
    /// A member section 2.3.3 requires is missing or not of its form; which.
    #[error("its {0} is missing or is not of SAML 2.0 core's form")]
    Malformed(&'static str),
    /// More of something than this program keeps; what.
    #[error("it holds more {0} than this program keeps")]
    TooMany(&'static str),
}

/// One `<Attribute>` (SAML 2.0 core section 2.7.3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// `Name`.
    pub name: String,
    /// `NameFormat`, where stated; section 2.7.3.1 makes its absence section 8.2.1's
    /// `unspecified`.
    pub name_format: Option<String>,
    /// `FriendlyName`, where stated.
    pub friendly_name: Option<String>,
    /// The character data of each `<AttributeValue>`, in order.
    pub values: Vec<String>,
}

impl Attribute {
    /// The object identifier the attribute names, as contents octets: SAML 2.0 profiles section
    /// 8.2.2 names a directory attribute type by its RFC 3061 URN under the URI name format.
    #[must_use]
    pub fn object_identifier(&self) -> Option<Vec<u8>> {
        self.uri()
            .and_then(urn_oid)
            .and_then(|dotted| const_oid::ObjectIdentifier::new(dotted).ok())
            .map(|oid| oid.as_bytes().to_vec())
    }

    /// The URI the attribute is named by, where its name format is section 8.2.2's.
    #[must_use]
    pub fn uri(&self) -> Option<&str> {
        (self.name_format.as_deref() == Some(URI_NAME_FORMAT)).then_some(self.name.as_str())
    }
}

/// The dotted object identifier an RFC 3061 URN spells; RFC 2141 section 2 makes the `urn` and the
/// namespace identifier case-insensitive.
fn urn_oid(uri: &str) -> Option<&str> {
    let prefix = uri.get(..8)?;
    prefix
        .eq_ignore_ascii_case("urn:oid:")
        .then(|| uri.get(8..))
        .flatten()
}

/// SAML 2.0 core section 2.5.1.1's three outcomes of condition processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conditions {
    /// Every condition is valid.
    Valid,
    /// A condition is invalid: the instant is outside the interval.
    Invalid,
    /// None is invalid and one cannot be evaluated or is not understood; which.
    Indeterminate(&'static str),
}

/// A SAML 2.0 assertion, as far as SAML 2.0 core section 2.3.3 says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assertion {
    /// `ID`.
    pub id: String,
    /// `IssueInstant`, as written.
    pub issue_instant: String,
    /// `<Issuer>`'s name.
    pub issuer: String,
    /// `<Subject>`'s `<NameID>`, where it has one.
    pub subject: Option<String>,
    /// `<Conditions>`'s `NotBefore`, as written and as an instant.
    pub not_before: Option<(String, Instant)>,
    /// `<Conditions>`'s `NotOnOrAfter`, as written and as an instant.
    pub not_on_or_after: Option<(String, Instant)>,
    /// How many `<AudienceRestriction>` conditions it states.
    pub audience_restrictions: usize,
    /// How many `<Condition>` elements of an extension type it states.
    pub extension_conditions: usize,
    /// Every attribute of every `<AttributeStatement>`, in order.
    pub attributes: Vec<Attribute>,
    /// How many `<EncryptedAttribute>` elements, which are counted and not opened.
    pub encrypted_attributes: usize,
    /// Whether it carries a `ds:Signature` child (section 2.3.3).
    pub signed: bool,
}

impl Assertion {
    /// Section 2.5.1.1's processing at `at`: section 2.5.1.2's interval, its start included and its
    /// end excluded, and the conditions this program cannot evaluate.
    #[must_use]
    pub fn conditions_at(&self, at: Instant) -> Conditions {
        let before = self
            .not_before
            .as_ref()
            .is_some_and(|(_, start)| at < *start);
        let after = self
            .not_on_or_after
            .as_ref()
            .is_some_and(|(_, end)| at >= *end);
        if before || after {
            Conditions::Invalid
        } else if self.audience_restrictions > 0 {
            Conditions::Indeterminate(
                "an audience restriction, whose audiences name no verifier of a signature",
            )
        } else if self.extension_conditions > 0 {
            Conditions::Indeterminate("a condition of an extension type this program does not know")
        } else {
            Conditions::Valid
        }
    }
}

/// What became of an assertion's signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureCheck {
    /// It carries no `ds:Signature`.
    Unsigned,
    /// Section 5.4's validation passed under the key of this certificate, as DER, which the
    /// signature itself carried; who the certificate belongs to is a separate question.
    Verified(Vec<u8>),
    /// The validation did not pass; why.
    Failed(XmlSignatureError),
}

impl SignatureCheck {
    /// Whether the failure is a finding about the signature rather than a limit of this program:
    /// a digest or a value that does not match, or a structure section 5.4 forbids.
    #[must_use]
    pub const fn is_invalid(&self) -> bool {
        matches!(
            self,
            Self::Failed(
                XmlSignatureError::DigestDiffers(_)
                    | XmlSignatureError::SignatureDiffers
                    | XmlSignatureError::NotOneReference(_)
                    | XmlSignatureError::AssertionNotCovered
                    | XmlSignatureError::Ambiguous(_)
                    | XmlSignatureError::Malformed(_)
            )
        )
    }
}

/// One SAML assertion a signature carries, read and its signature checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saml {
    /// The assertion, or why it is not one.
    pub assertion: Result<Assertion, Unreadable>,
    /// Its signature; [`SignatureCheck::Unsigned`] where the assertion was not read.
    pub signature: SignatureCheck,
}

impl Saml {
    /// Reads `octets` and, where the assertion is signed, verifies the signature.
    #[must_use]
    pub fn of(octets: &[u8]) -> Self {
        let assertion = read(octets);
        let signature = match &assertion {
            Ok(read) if read.signed => match xml_signature::verify_saml(octets, MAX_ENTRIES) {
                Ok(verified) => SignatureCheck::Verified(verified.signer),
                Err(why) => SignatureCheck::Failed(why),
            },
            _ => SignatureCheck::Unsigned,
        };
        Self {
            assertion,
            signature,
        }
    }

    /// The attributes a relying party may rely on: none of an assertion whose signature is
    /// invalid (section 2.3.3), every one otherwise.
    #[must_use]
    pub fn attributes_relied_on(&self) -> &[Attribute] {
        match (&self.assertion, self.signature.is_invalid()) {
            (Ok(assertion), false) => &assertion.attributes,
            _ => &[],
        }
    }
}

/// Reads `octets` as SAML 2.0 core section 2.3.3's `<Assertion>`.
///
/// # Errors
///
/// [`Unreadable`]: past a bound, not XML, or not an assertion, named.
pub fn read(octets: &[u8]) -> Result<Assertion, Unreadable> {
    if octets.len() > MAX_OCTETS {
        return Err(Unreadable::TooLong(octets.len()));
    }
    let mut remaining = MAX_ENTRIES;
    let tree = xml::read_tree(octets, &mut remaining).map_err(|error| {
        Unreadable::Xml(match error {
            xml::XmlError::Unreadable(reason) => reason,
            xml::XmlError::NestedTooDeep => "elements nest too deep".to_owned(),
            xml::XmlError::TooManyEntries => "too many elements".to_owned(),
        })
    })?;
    let root = tree.elements.first().ok_or(Unreadable::NotAnAssertion)?;
    if !is(root, "Assertion") {
        return Err(Unreadable::NotAnAssertion);
    }
    let version = attribute(root, "Version").ok_or(Unreadable::Malformed("Version"))?;
    if version != "2.0" {
        return Err(Unreadable::Version(version.to_owned()));
    }
    let id = attribute(root, "ID").ok_or(Unreadable::Malformed("ID"))?;
    let issue_instant = attribute(root, "IssueInstant")
        .filter(|spelled| utc(spelled).is_some())
        .ok_or(Unreadable::Malformed("IssueInstant"))?;
    let children: Vec<&Element> = root
        .children
        .iter()
        .filter_map(|&index| tree.elements.get(index))
        .collect();
    // Section 2.3.3's sequence opens with the one required `<Issuer>`.
    let issuer = children
        .first()
        .filter(|first| is(first, "Issuer"))
        .ok_or(Unreadable::Malformed("Issuer"))?;
    let signed = children
        .iter()
        .any(|child| child.namespace == DSIG && child.local == "Signature");
    let subject = children
        .iter()
        .find(|child| is(child, "Subject"))
        .and_then(|subject| first_child(&tree, subject, "NameID"))
        .map(|name| name.text.trim().to_owned());
    let mut out = Assertion {
        id: id.to_owned(),
        issue_instant: issue_instant.to_owned(),
        issuer: issuer.text.trim().to_owned(),
        subject,
        not_before: None,
        not_on_or_after: None,
        audience_restrictions: 0,
        extension_conditions: 0,
        attributes: Vec::new(),
        encrypted_attributes: 0,
        signed,
    };
    if let Some(conditions) = children.iter().find(|child| is(child, "Conditions")) {
        read_conditions(&tree, conditions, &mut out)?;
    }
    for statement in children
        .iter()
        .filter(|child| is(child, "AttributeStatement"))
    {
        read_statement(&tree, statement, &mut out)?;
    }
    Ok(out)
}

/// Section 2.5.1's `<Conditions>`.
fn read_conditions(
    tree: &Tree,
    conditions: &Element,
    out: &mut Assertion,
) -> Result<(), Unreadable> {
    let instant = |name: &'static str| -> Result<Option<(String, Instant)>, Unreadable> {
        attribute(conditions, name)
            .map(|spelled| {
                utc(spelled)
                    .map(|at| (spelled.to_owned(), at))
                    .ok_or(Unreadable::Malformed(name))
            })
            .transpose()
    };
    out.not_before = instant("NotBefore")?;
    out.not_on_or_after = instant("NotOnOrAfter")?;
    // Section 2.5.1.2: where both are present, the start is earlier than the end.
    if let (Some((_, start)), Some((_, end))) = (&out.not_before, &out.not_on_or_after)
        && start >= end
    {
        return Err(Unreadable::Malformed("Conditions"));
    }
    for child in conditions
        .children
        .iter()
        .filter_map(|&index| tree.elements.get(index))
    {
        if is(child, "AudienceRestriction") {
            out.audience_restrictions = out.audience_restrictions.saturating_add(1);
        } else if is(child, "Condition") {
            out.extension_conditions = out.extension_conditions.saturating_add(1);
        } else if !(is(child, "OneTimeUse") || is(child, "ProxyRestriction")) {
            // The schema's choice admits nothing else.
            return Err(Unreadable::Malformed("Conditions"));
        }
    }
    Ok(())
}

/// Section 2.7.3's `<AttributeStatement>`.
fn read_statement(tree: &Tree, statement: &Element, out: &mut Assertion) -> Result<(), Unreadable> {
    for child in statement
        .children
        .iter()
        .filter_map(|&index| tree.elements.get(index))
    {
        if is(child, "EncryptedAttribute") {
            out.encrypted_attributes = out.encrypted_attributes.saturating_add(1);
            continue;
        }
        if !is(child, "Attribute") {
            return Err(Unreadable::Malformed("AttributeStatement"));
        }
        if out.attributes.len() >= MAX_ATTRIBUTES {
            return Err(Unreadable::TooMany("attributes"));
        }
        let name = attribute(child, "Name").ok_or(Unreadable::Malformed("Attribute Name"))?;
        let mut values = Vec::new();
        for value in child
            .children
            .iter()
            .filter_map(|&index| tree.elements.get(index))
            .filter(|value| is(value, "AttributeValue"))
        {
            if values.len() >= MAX_VALUES {
                return Err(Unreadable::TooMany("attribute values"));
            }
            values.push(value.text.trim().to_owned());
        }
        out.attributes.push(Attribute {
            name: name.to_owned(),
            name_format: attribute(child, "NameFormat").map(str::to_owned),
            friendly_name: attribute(child, "FriendlyName").map(str::to_owned),
            values,
        });
    }
    Ok(())
}

/// Whether `element` is the assertion namespace's `local`.
fn is(element: &Element, local: &str) -> bool {
    element.namespace == ASSERTION_NAMESPACE && element.local == local
}

/// The first child of `parent` that is the assertion namespace's `local`.
fn first_child<'t>(tree: &'t Tree, parent: &Element, local: &str) -> Option<&'t Element> {
    parent
        .children
        .iter()
        .filter_map(|&index| tree.elements.get(index))
        .find(|child| is(child, local))
}

/// An unprefixed attribute's value.
fn attribute<'e>(element: &'e Element, local: &str) -> Option<&'e str> {
    element
        .attributes
        .iter()
        .find(|(name, _)| name == local)
        .map(|(_, value)| value.as_str())
}

/// Section 1.3.3's time value: an `xs:dateTime` in UTC. The section's "no time zone component"
/// is read as the UTC form either way, so a value written with `Z` or with no zone at all is the
/// instant, and one with any other offset is not of the form.
fn utc(spelled: &str) -> Option<Instant> {
    let has_offset = spelled
        .get(19..)
        .is_some_and(|rest| rest.contains(['+', '-']));
    if has_offset {
        None
    } else if spelled.ends_with('Z') {
        xml::date_time(spelled)
    } else {
        xml::date_time(&format!("{spelled}Z"))
    }
}

#[cfg(test)]
pub(crate) mod tests;
