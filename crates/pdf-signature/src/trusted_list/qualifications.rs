//! ETSI TS 119 612 clause 5.5.9.2's `Qualifications` extension, read, and its criteria asked of a
//! certificate.
//!
//! A service of type `CA/QC` that cannot say everything about its certificates inside them says
//! the rest in its list entry: a set of qualification elements, each a list of assertions about a
//! certificate's contents (clause 5.5.9.2.2) and the qualifiers (clause 5.5.9.2.3) that apply to
//! every certificate the assertions identify. TS 119 615 clause 4.4 gathers the qualifiers of the
//! elements that identify the certificate under validation; this module is the reading and the
//! identifying. The ETSI texts are cited by clause and paraphrased, never quoted (ADR 1085).
//!
//! # Three answers, not two
//!
//! An assertion this reader cannot evaluate — a criterion clause 5.5.9.2.2.3 lets a scheme
//! operator add, a key usage bit by a name ITU-T X.509 does not give, an identifier that is not an
//! object identifier, a certificate extension that does not read — is neither met nor unmet, and a
//! list combining it says [`Match::Undecided`] wherever the answer turns on it. Clause 5.5.9.2.0
//! decides what an undecided element costs: under an extension marked critical the certificate is
//! discarded, which `super::determination` reports by name (ADR 2864).
//!
//! # The bounds
//!
//! Every element read here is one of the list's, so [`super::MAX_ENTRIES`] and the XML reader's
//! nesting bound already hold the reading and the recursion of [`CriteriaList::identifies`]: a
//! nested criteria list is a nested element, and no element is read twice.

use crate::trust::names::{self, Match};
use crate::trust::policies;
use crate::x509::{self, Certificate, KeyUsage};
use crate::xml::{Element, Tree};

/// The namespace TS 119 612 Annex C's `19612_sie_xsd.xsd` writes the qualifications extension in.
pub const SIE_NAMESPACE: &str =
    "http://uri.etsi.org/TrstSvc/SvcInfoExt/eSigDir-1999-93-EC-TrustedList/#";

/// Clause 5.5.9.2.3's qualifier stating that every certificate the criteria identify is a
/// qualified certificate for electronic signatures.
pub const QC_FOR_ESIG: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCForESig";
/// The same for electronic seals.
pub const QC_FOR_ESEAL: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCForESeal";
/// The same for web site authentication.
pub const QC_FOR_WSA: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCForWSA";
/// Clause 5.5.9.2.3's qualifier stating that the certificates are not to be taken as qualified.
pub const NOT_QUALIFIED: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/NotQualified";
/// Clause 5.5.9.2.3's qualifier stating that the certificates are issued as qualified ones.
pub const QC_STATEMENT: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCStatement";
/// Clause 5.5.9.2.3's qualifier stating that the certificates are issued to legal persons.
pub const QC_FOR_LEGAL_PERSON: &str =
    "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCForLegalPerson";

/// ITU-T X.509's `KeyUsage` bits by the names clause 5.5.9.2.2.1 and Annex C's
/// `KeyUsageBitType` spell them, in bit order.
const KEY_USAGE_BITS: [&str; 9] = [
    "digitalSignature",
    "nonRepudiation",
    "keyEncipherment",
    "dataEncipherment",
    "keyAgreement",
    "keyCertSign",
    "crlSign",
    "encipherOnly",
    "decipherOnly",
];

/// One `Qualifications` extension: clause 5.5.9.2.0's non-empty sequence of qualification
/// elements, and whether the list marked the extension critical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qualifications {
    /// The `Extension`'s `Critical` attribute (TS 119 612 clause 5.5.9.0).
    pub critical: bool,
    /// Clause 5.5.9.2.1's elements.
    pub elements: Vec<QualificationElement>,
}

/// Clause 5.5.9.2.1's tuple: the assertions, and the qualifiers that apply where they are met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualificationElement {
    /// Clause 5.5.9.2.3's qualifier URIs, as written.
    pub qualifiers: Vec<String>,
    /// Clause 5.5.9.2.2's criteria.
    pub criteria: CriteriaList,
}

/// Clause 5.5.9.2.2.0's matching criteria indicator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assert {
    /// `all`: every assertion is met.
    All,
    /// `atLeastOne`: one assertion or more is met.
    AtLeastOne,
    /// `none`: no assertion is met.
    None,
    /// An indicator the clause does not give, or none at all, as written — which decides nothing.
    Unrecognised(String),
}

/// Clause 5.5.9.2.2.0's list of assertions, which can itself be one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriteriaList {
    /// How the assertions combine.
    pub assert: Assert,
    /// The assertions, in the list's order.
    pub assertions: Vec<Assertion>,
    /// The optional rationale, in UK English.
    pub description: Option<String>,
}

/// One assertion about a certificate (clauses 5.5.9.2.2.1 to 5.5.9.2.2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assertion {
    /// Clause 5.5.9.2.2.1: each bit, by its position (`None` for a name X.509 does not give), with
    /// the value asserted (`None` where it is not an XML Schema boolean).
    KeyUsage(Vec<(Option<u8>, Option<bool>)>),
    /// Clause 5.5.9.2.2.2: certificate policy identifiers, dotted (`None` where one does not read
    /// as an object identifier).
    PolicySet(Vec<Option<String>>),
    /// Clause 5.5.9.2.2.3 1): key purposes, dotted.
    ExtendedKeyUsage(Vec<Option<String>>),
    /// Clause 5.5.9.2.2.3 2): attribute types the subject's name holds, dotted.
    CertSubjectDnAttribute(Vec<Option<String>>),
    /// Clause 5.5.9.2.2.0's recursion: a criteria list as an assertion.
    CriteriaList(CriteriaList),
    /// A criterion clause 5.5.9.2.2.3 leaves to the scheme operator to define, by its expanded
    /// name.
    Unrecognised(String),
}

/// A certificate field the criteria ask about: absent, read, or present and unreadable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field<T> {
    /// The certificate does not carry it.
    Absent,
    /// What it says.
    Read(T),
    /// It is there and does not read.
    Unreadable,
}

/// What the criteria ask of a certificate, read off it once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Examined {
    /// The `keyUsage` extension.
    pub key_usage: Option<KeyUsage>,
    /// The `certificatePolicies` extension's identifiers, dotted.
    pub policies: Field<Vec<String>>,
    /// The `extKeyUsage` extension's purposes, dotted.
    pub purposes: Field<Vec<String>>,
    /// The subject name's attribute types, dotted.
    pub subject_attributes: Field<Vec<String>>,
}

impl Examined {
    /// The fields of `certificate` the criteria ask about.
    #[must_use]
    pub fn of(certificate: &Certificate<'_>) -> Self {
        let dotted_all = |oids: Vec<Vec<u8>>| {
            oids.iter()
                .map(|oid| x509::dotted(oid))
                .collect::<Option<Vec<String>>>()
                .map_or(Field::Unreadable, Field::Read)
        };
        let policies = match certificate.extensions.certificate_policies {
            None => Field::Absent,
            Some(encoded) => {
                policies::read_certificate_policies(encoded).map_or(Field::Unreadable, dotted_all)
            }
        };
        let purposes = match certificate.extensions.extended_key_usage {
            None => Field::Absent,
            Some(encoded) => key_purposes(encoded).map_or(Field::Unreadable, dotted_all),
        };
        let subject_attributes = names::attributes_of(certificate.subject)
            .map_or(Field::Unreadable, |attributes| {
                Field::Read(attributes.into_iter().map(|a| a.kind).collect())
            });
        Self {
            key_usage: certificate.extensions.key_usage,
            policies,
            purposes,
            subject_attributes,
        }
    }
}

/// RFC 5280 section 4.2.1.12's `ExtKeyUsageSyntax`, its purposes as encoded; `None` where it is not
/// a non-empty `SEQUENCE OF KeyPurposeId` or holds more than [`policies::MAX_POLICIES`] of them.
fn key_purposes(encoded: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut reader = crate::der::Reader::new(encoded).ok()?;
    let list = reader.next_value().ok()??;
    if list.identifier != crate::der::SEQUENCE || !matches!(reader.next_value(), Ok(None)) {
        return None;
    }
    let mut children = list.children().ok()?;
    let mut purposes = Vec::new();
    while let Some(purpose) = children.next_value().ok()? {
        if purposes.len() >= policies::MAX_POLICIES {
            return None;
        }
        purposes.push(purpose.object_identifier()?.to_vec());
    }
    (!purposes.is_empty()).then_some(purposes)
}

impl CriteriaList {
    /// Whether these criteria identify the certificate `examined` describes — clause
    /// 5.5.9.2.2.0's indicator over the assertions, three-valued (module documentation).
    #[must_use]
    pub fn identifies(&self, examined: &Examined) -> Match {
        let answers = self
            .assertions
            .iter()
            .map(|assertion| assertion.met(examined));
        let mut met = false;
        let mut unmet = false;
        let mut undecided = false;
        for answer in answers {
            match answer {
                Match::Yes => met = true,
                Match::No => unmet = true,
                Match::Undecided => undecided = true,
            }
        }
        match self.assert {
            Assert::All if unmet => Match::No,
            Assert::AtLeastOne if met => Match::Yes,
            Assert::None if met => Match::No,
            Assert::Unrecognised(_) => Match::Undecided,
            _ if undecided => Match::Undecided,
            Assert::All | Assert::None => Match::Yes,
            Assert::AtLeastOne => Match::No,
        }
    }
}

impl Assertion {
    /// Whether this one assertion is met by the certificate `examined` describes.
    #[must_use]
    pub fn met(&self, examined: &Examined) -> Match {
        match self {
            // Clause 5.5.9.2.2.1: the extension is present and every listed bit has the value the
            // list asserts.
            Self::KeyUsage(bits) => {
                let Some(usage) = examined.key_usage else {
                    return Match::No;
                };
                every(bits.iter().map(|&(bit, value)| match (bit, value) {
                    (Some(bit), Some(value)) => Some(usage.asserts(bit) == value),
                    _ => None,
                }))
            }
            // Clause 5.5.9.2.2.2: the extension is present and holds every identifier listed.
            Self::PolicySet(wanted) => contains_all(&examined.policies, wanted),
            // Clause 5.5.9.2.2.3 1): the same of the extended key usage's purposes.
            Self::ExtendedKeyUsage(wanted) => contains_all(&examined.purposes, wanted),
            // Clause 5.5.9.2.2.3 2): every attribute type is present in the subject's name.
            Self::CertSubjectDnAttribute(wanted) => {
                contains_all(&examined.subject_attributes, wanted)
            }
            Self::CriteriaList(nested) => nested.identifies(examined),
            Self::Unrecognised(_) => Match::Undecided,
        }
    }
}

/// `Yes` where every answer is `Some(true)`, `No` where one is `Some(false)`, and `Undecided`
/// where none is false and one could not be given.
fn every(answers: impl Iterator<Item = Option<bool>>) -> Match {
    let mut undecided = false;
    for answer in answers {
        match answer {
            Some(false) => return Match::No,
            None => undecided = true,
            Some(true) => {}
        }
    }
    if undecided {
        Match::Undecided
    } else {
        Match::Yes
    }
}

/// Whether a certificate field holds every one of `wanted`: an absent field holds none of them.
fn contains_all(field: &Field<Vec<String>>, wanted: &[Option<String>]) -> Match {
    match field {
        Field::Absent => Match::No,
        Field::Unreadable => Match::Undecided,
        Field::Read(held) => every(
            wanted
                .iter()
                .map(|oid| oid.as_ref().map(|oid| held.contains(oid))),
        ),
    }
}

/// Reads one `Extension` element's `Qualifications`, where it holds one.
pub(super) fn read(tree: &Tree, extension: &Element) -> Option<Qualifications> {
    let qualifications = children(tree, extension)
        .find(|child| child.namespace == SIE_NAMESPACE && child.local == "Qualifications")?;
    let critical = matches!(attribute(extension, "Critical"), Some("true" | "1"));
    let elements = sie_children(tree, qualifications, "QualificationElement")
        .map(|element| QualificationElement {
            qualifiers: sie_children(tree, element, "Qualifiers")
                .flat_map(|list| sie_children(tree, list, "Qualifier"))
                .filter_map(|qualifier| attribute(qualifier, "uri"))
                .map(|uri| uri.trim().to_owned())
                .collect(),
            criteria: sie_children(tree, element, "CriteriaList").next().map_or(
                CriteriaList {
                    assert: Assert::Unrecognised(String::new()),
                    assertions: Vec::new(),
                    description: None,
                },
                |list| criteria(tree, list),
            ),
        })
        .collect();
    Some(Qualifications { critical, elements })
}

/// Clause 5.5.9.2.2's criteria list, in Annex C's `CriteriaListType` order.
fn criteria(tree: &Tree, list: &Element) -> CriteriaList {
    let assert = match attribute(list, "assert") {
        Some("all") => Assert::All,
        Some("atLeastOne") => Assert::AtLeastOne,
        Some("none") => Assert::None,
        other => Assert::Unrecognised(other.unwrap_or_default().to_owned()),
    };
    let mut assertions = Vec::new();
    let mut description = None;
    for child in children(tree, list) {
        if child.namespace != SIE_NAMESPACE {
            assertions.push(Assertion::Unrecognised(child.expanded.clone()));
            continue;
        }
        match child.local.as_str() {
            "KeyUsage" => assertions.push(Assertion::KeyUsage(
                sie_children(tree, child, "KeyUsageBit")
                    .map(|bit| {
                        let position = attribute(bit, "name").and_then(|name| {
                            KEY_USAGE_BITS
                                .iter()
                                .position(|known| *known == name)
                                .and_then(|at| u8::try_from(at).ok())
                        });
                        (position, boolean(&bit.text))
                    })
                    .collect(),
            )),
            "PolicySet" => assertions.push(Assertion::PolicySet(identifiers(
                tree,
                child,
                SIE_NAMESPACE,
                "PolicyIdentifier",
            ))),
            "CriteriaList" => assertions.push(Assertion::CriteriaList(criteria(tree, child))),
            "Description" => description = Some(child.text.trim().to_owned()),
            "otherCriteriaList" => {
                for other in children(tree, child) {
                    assertions.push(other_criterion(tree, other));
                }
            }
            _ => assertions.push(Assertion::Unrecognised(child.expanded.clone())),
        }
    }
    CriteriaList {
        assert,
        assertions,
        description,
    }
}

/// Clause 5.5.9.2.2.3's two defined criteria, in Annex C's additional types namespace; any other
/// is the scheme operator's, and unrecognised here.
fn other_criterion(tree: &Tree, other: &Element) -> Assertion {
    if other.namespace == super::ADDITIONAL_TYPES {
        match other.local.as_str() {
            "ExtendedKeyUsage" => {
                return Assertion::ExtendedKeyUsage(identifiers(
                    tree,
                    other,
                    super::ADDITIONAL_TYPES,
                    "KeyPurposeId",
                ));
            }
            "CertSubjectDNAttribute" => {
                return Assertion::CertSubjectDnAttribute(identifiers(
                    tree,
                    other,
                    super::ADDITIONAL_TYPES,
                    "AttributeOID",
                ));
            }
            _ => {}
        }
    }
    Assertion::Unrecognised(other.expanded.clone())
}

/// The object identifiers of `parent`'s `local` children, each `XAdES`'s `ObjectIdentifierType`
/// (ETSI EN 319 132-1): an `Identifier` whose text is the identifier, dotted or as RFC 3061's
/// `urn:oid:` URN.
fn identifiers(tree: &Tree, parent: &Element, namespace: &str, local: &str) -> Vec<Option<String>> {
    children(tree, parent)
        .filter(|child| child.namespace == namespace && child.local == local)
        .map(|identifier| {
            children(tree, identifier)
                .find(|child| child.local == "Identifier")
                .and_then(|text| dotted_text(&text.text))
        })
        .collect()
}

/// An identifier written dotted, with or without RFC 3061's `urn:oid:` in front; `None` where what
/// remains is not arcs of digits.
fn dotted_text(written: &str) -> Option<String> {
    let trimmed = written.trim();
    let bare = trimmed
        .get(..8)
        .filter(|prefix| prefix.eq_ignore_ascii_case("urn:oid:"))
        .map_or(trimmed, |_| &trimmed[8..]);
    let arcs = bare.split('.').count();
    (arcs >= 2
        && bare
            .split('.')
            .all(|arc| !arc.is_empty() && arc.bytes().all(|b| b.is_ascii_digit())))
    .then(|| bare.to_owned())
}

/// XML Schema's `boolean` lexical space.
fn boolean(text: &str) -> Option<bool> {
    match text.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn children<'t>(tree: &'t Tree, parent: &'t Element) -> impl Iterator<Item = &'t Element> + 't {
    parent
        .children
        .iter()
        .filter_map(|&index| tree.elements.get(index))
}

fn sie_children<'t>(
    tree: &'t Tree,
    parent: &'t Element,
    local: &'t str,
) -> impl Iterator<Item = &'t Element> + 't {
    children(tree, parent)
        .filter(move |child| child.namespace == SIE_NAMESPACE && child.local == local)
}

fn attribute<'e>(element: &'e Element, local: &str) -> Option<&'e str> {
    element
        .attributes
        .iter()
        .find(|(name, _)| name == local)
        .map(|(_, value)| value.as_str())
}
