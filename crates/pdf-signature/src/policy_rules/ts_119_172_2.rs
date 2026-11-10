//! The XML signature policy syntax: ETSI TS 119 172-2's `SignaturePolicy`, read out of the policy
//! document a signature is bound to and enforced over that signature under ISO 32000-2
//! §12.8.3.4.4's "shall enforce signature policy constraints".
//!
//! TS 119 172-2 clause 4 defines the building blocks TS 119 172-3 writes again in ASN.1, so a
//! document read here becomes the same rules [`ts_119_172_3`] reads — the same [`BasicRule`]s,
//! [`CommitmentRule`]s and recommended scopes — and is judged by the same code into the same
//! [`Finding`]s (ADR 2604). What differs is the syntax: clause 4.2's root element in the namespace
//! of clause 4.1.2, every identifier an `xs:anyURI`, every enumeration an XML token, and
//! extensions of clause 4.1.3's `AnyType`. ETSI's texts are cited by clause and never quoted (ADR
//! 1085).
//!
//! # Identifiers
//!
//! The policy identifier of clause 4.6 is a URI, an object identifier written as RFC 3061's
//! `urn:oid:` URN; it is compared with the object identifier the signature names. A commitment
//! identifier (clause 4.15) that is such a URN is that object identifier; any other URI is kept as
//! a URI, which the object identifier of a `CAdES` commitment type indication never equals. An
//! algorithm identifier (clause 4.29) is an `urn:oid:` URN or one of the XML Signature algorithm
//! URIs [`ALGORITHM_URIS`] places; any other stays a URI, and a used algorithm only such an entry
//! could admit is not evaluated rather than broken.
//!
//! # Binding
//!
//! Clause 4.3's `Digest` is computed over the *canonicalised* `PolicyComponents` element, under
//! the canonicalisation method it names. A document is enforced where its octets digest to the
//! value the signer signed, or where the signer signed the stated `Digest` and the components,
//! canonicalised by [`super::canonical_xml`] under the method named — inclusive Canonical XML 1.0,
//! Exclusive XML Canonicalization 1.0 with the `PrefixList` its `InclusiveNamespaces` element
//! states, or Canonical XML 1.1, each with or without comments — digest to it; components that do
//! not are [`super::NotEnforced::CanonicalDigestDiffers`], and a method none of those three
//! Recommendations names is said by name ([`super::NotEnforced::CanonicalDigestNotComputed`])
//! rather than enforced on an unchecked commitment (ADRs 2604, 2644, 2710).
//!
//! # Budgets, before the work
//!
//! The document is hostile until bound, and it is read under the first syntax's three bounds:
//! [`MAX_POLICY_OCTETS`] before a byte is tokenized, [`der::MAX_DEPTH`] on element nesting —
//! every structure clause 4 defines nests within it, and only `AnyType` content could go
//! deeper — and the one [`MAX_ENTRIES`] count, charged for every element, every namespace
//! declaration and every token of an `xs:list` before it is kept. The tokenizer is `xmlparser`,
//! the one XMP, XFDF and XFA are read with; a document type declaration is refused by name, so no
//! declared entity is ever expanded, and of the references XML 1.0 defines only the five
//! predefined entities and character references are.

use const_oid::ObjectIdentifier;
use const_oid::db::rfc5912;

use super::certification_path::{TrustPoint, TrustTrees};
use super::duration::Duration;
use super::revocation_checks::{self, Freshness};
use super::ts_119_172_3::{
    self, AlgorithmConstraint, BasicRule, Cardinality, CommitmentRule, Comparison, ExtensionName,
    FormatsAndLevels, Identifier, Matching, MediaTypes, PolicyRule, Position, Scope, ScopedRule,
    TimeEvidence,
};
use super::{
    Budget, CertificateInfo, Finding, MAX_ENTRIES, MAX_POLICY_OCTETS, PolicyDocumentError,
    SignerFacts, SignerTrust, Stated, Syntax, TimestampTrust, Verdict, canonical_xml,
    signing_period,
};
use crate::cms::SignedData;
use crate::der;
use crate::x509;

/// Clause 4.1.2's namespace, as the schema prints it and as the clause's own list prints it —
/// without the final `#` — since a producer may follow either.
pub const NAMESPACE: [&str; 2] = [
    "http://uri.etsi.org/19172/v1.1.1#",
    "http://uri.etsi.org/19172/v1.1.1",
];

/// The XML Signature namespace clause 4.3 takes `DigestMethod`, `DigestValue` and
/// `CanonicalizationMethod` from.
pub const XMLDSIG_NAMESPACE: &str = "http://www.w3.org/2000/09/xmldsig#";

/// `ecdsa-with-SHA1`, RFC 3279 section 2.2.3, which `const_oid`'s database does not carry.
const ECDSA_WITH_SHA_1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.4.1");

/// The XML Signature algorithm URIs a clause 4.29 `AlgId` may name an algorithm by, each beside
/// the object identifier a CMS signature names the same algorithm by.
///
/// The URIs are XML Signature 1.1 sections 6.2 and 6.4's and RFC 9231 section 2's, for the digest
/// functions and the RSA and ECDSA signature algorithms this crate verifies; the object
/// identifiers are RFC 3279's, RFC 4055 section 5's, RFC 5758 section 3.2's and RFC 5754 section
/// 2's, read through `const_oid`'s database. Each algorithm has exactly one URI in those texts.
pub const ALGORITHM_URIS: [(&str, ObjectIdentifier); 15] = [
    ("http://www.w3.org/2000/09/xmldsig#sha1", rfc5912::ID_SHA_1),
    (
        "http://www.w3.org/2001/04/xmldsig-more#sha224",
        rfc5912::ID_SHA_224,
    ),
    (
        "http://www.w3.org/2001/04/xmlenc#sha256",
        rfc5912::ID_SHA_256,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#sha384",
        rfc5912::ID_SHA_384,
    ),
    (
        "http://www.w3.org/2001/04/xmlenc#sha512",
        rfc5912::ID_SHA_512,
    ),
    (
        "http://www.w3.org/2000/09/xmldsig#rsa-sha1",
        rfc5912::SHA_1_WITH_RSA_ENCRYPTION,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha224",
        rfc5912::SHA_224_WITH_RSA_ENCRYPTION,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
        rfc5912::SHA_256_WITH_RSA_ENCRYPTION,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384",
        rfc5912::SHA_384_WITH_RSA_ENCRYPTION,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512",
        rfc5912::SHA_512_WITH_RSA_ENCRYPTION,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha1",
        ECDSA_WITH_SHA_1,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha224",
        rfc5912::ECDSA_WITH_SHA_224,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha256",
        rfc5912::ECDSA_WITH_SHA_256,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha384",
        rfc5912::ECDSA_WITH_SHA_384,
    ),
    (
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha512",
        rfc5912::ECDSA_WITH_SHA_512,
    ),
];

/// Whether a policy document is XML: its first character, past a byte order mark and white
/// space, opens markup. No DER encoding of either ASN.1 syntax can open so, since both begin with
/// a `SEQUENCE`.
#[must_use]
pub fn is_xml(octets: &[u8]) -> bool {
    let body = octets.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(octets);
    body.iter()
        .find(|octet| !matches!(octet, b' ' | b'\t' | b'\r' | b'\n'))
        .is_some_and(|first| *first == b'<')
}

/// Which namespace an element is in, as far as this reader tells them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Space {
    /// Clause 4.1.2's.
    Policy,
    /// XML Signature's.
    Signature,
    /// Any other, or none.
    Other,
}

/// One element of the document.
#[derive(Debug)]
struct Element {
    /// The prefix its tag was written with, until the start tag's declarations are read.
    prefix: String,
    space: Space,
    /// `{namespace}local`, the name an extension is reported by.
    expanded: String,
    local: String,
    /// The unprefixed attributes, by local name, references expanded.
    attributes: Vec<(String, String)>,
    children: Vec<usize>,
    /// Direct character data, references expanded.
    text: String,
}

/// The document's elements in document order; the root is the first.
#[derive(Debug)]
struct Tree {
    elements: Vec<Element>,
}

/// The XML of a policy document, read and bounded, before its rules are.
///
/// Two steps, because the rules borrow what the first step owns: the text of every element, and
/// the contents octets of every object identifier a `urn:oid:` URN spells.
#[derive(Debug)]
pub struct Reading {
    tree: Tree,
    /// Per element, the object identifier its text spells as a URN, where it does.
    oids: Vec<Option<Vec<u8>>>,
    /// What is left of [`MAX_ENTRIES`] once the elements were charged.
    remaining: usize,
}

/// TS 119 172-2 clause 4.2's `SignaturePolicy`, as far as its constraints go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDocument<'a> {
    /// Clause 4.3's `ds:DigestMethod` algorithm URI.
    pub digest_method: &'a str,
    /// Clause 4.3's `ds:DigestValue`, decoded.
    pub digest_value: Vec<u8>,
    /// Clause 4.3's `ds:CanonicalizationMethod` algorithm URI.
    pub canonicalization: &'a str,
    /// The `PrefixList` of an `ec:InclusiveNamespaces` element inside that
    /// `ds:CanonicalizationMethod`: Exclusive XML Canonicalization 1.0 section 4's one parameter,
    /// which the XML Signature schema's open content admits there.
    pub prefix_list: Option<&'a str>,
    /// Clause 4.6's `PolicyIdentifier`.
    pub identifier: &'a str,
    /// Clause 4.13's `SigningPeriod`: `NotBefore` and `NotAfter`.
    pub signing_period: Option<(Stated, Option<Stated>)>,
    /// Clause 4.13's `Other` entries, by name.
    pub other_details: Vec<ExtensionName<'a>>,
    /// Clause 4.14's `PolicyRule`s.
    pub rules: Vec<ScopedRule<'a>>,
}

impl PolicyDocument<'_> {
    /// The object identifier [`Self::identifier`] spells as an RFC 3061 URN, dotted.
    #[must_use]
    pub fn identifier_oid(&self) -> Option<&str> {
        urn_oid(self.identifier)
    }
}

const fn malformed(member: &'static str) -> PolicyDocumentError {
    PolicyDocumentError::ElementMalformed(member)
}

/// The dotted object identifier an RFC 3061 URN spells; RFC 2141 section 2 makes the `urn` and
/// the namespace identifier case-insensitive.
fn urn_oid(uri: &str) -> Option<&str> {
    let prefix = uri.get(..8)?;
    prefix
        .eq_ignore_ascii_case("urn:oid:")
        .then(|| uri.get(8..))
        .flatten()
}

/// The contents octets of the object identifier a URN spells.
fn encoded_oid(uri: &str) -> Option<Vec<u8>> {
    ObjectIdentifier::new(urn_oid(uri)?)
        .ok()
        .map(|oid| oid.as_bytes().to_vec())
}

impl Reading {
    /// Reads a policy document's XML into a bounded tree.
    ///
    /// # Errors
    ///
    /// [`PolicyDocumentError`]: the byte, depth and entry budgets; text that is not UTF-8 or not
    /// well-formed XML; a document type declaration; and a root that is not clause 4.2's
    /// `SignaturePolicy`, which is [`PolicyDocumentError::OtherSyntax`].
    pub fn parse(octets: &[u8]) -> Result<Self, PolicyDocumentError> {
        if octets.len() > MAX_POLICY_OCTETS {
            return Err(PolicyDocumentError::TooLarge {
                octets: octets.len(),
            });
        }
        if !is_xml(octets) {
            return Err(PolicyDocumentError::OtherSyntax(Syntax::Ts1191722));
        }
        let mut budget = Budget(MAX_ENTRIES);
        let tree = read_tree(octets, &mut budget)?;
        let root = tree.elements.first();
        if !root.is_some_and(|root| root.space == Space::Policy && root.local == "SignaturePolicy")
        {
            return Err(PolicyDocumentError::OtherSyntax(Syntax::Ts1191722));
        }
        let oids = tree
            .elements
            .iter()
            .map(|element| match element.local.as_str() {
                "Identifier" | "AlgId" => encoded_oid(element.text.trim()),
                _ => None,
            })
            .collect();
        Ok(Self {
            tree,
            oids,
            remaining: budget.0,
        })
    }

    /// Clause 4.2's `SignaturePolicy` out of the tree.
    ///
    /// # Errors
    ///
    /// [`PolicyDocumentError::ElementMalformed`], naming an element clause 4 requires that is
    /// missing or whose value is not of its type, and [`PolicyDocumentError::TooManyEntries`].
    pub fn document(&self) -> Result<PolicyDocument<'_>, PolicyDocumentError> {
        let mut reader = Reader {
            tree: &self.tree,
            oids: &self.oids,
            budget: Budget(self.remaining),
        };
        reader.document()
    }
}

/// The tokenizer's events folded into a [`Tree`], every budget charged before the work.
fn read_tree(octets: &[u8], budget: &mut Budget) -> Result<Tree, PolicyDocumentError> {
    let body = octets.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(octets);
    // TS 119 172-2 names no character encoding; XML 1.0 section 4.3.3 makes UTF-8 one every
    // processor reads, and a document in another is refused by name rather than misread.
    let text = std::str::from_utf8(body)
        .map_err(|_| PolicyDocumentError::Xml("the document is not UTF-8".to_owned()))?;
    let mut elements: Vec<Element> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut scope = Declarations::default();
    let unbalanced =
        || PolicyDocumentError::Xml("an element is closed without being opened".to_owned());
    for token in xmlparser::Tokenizer::from(text) {
        let token = token.map_err(|error| PolicyDocumentError::Xml(error.to_string()))?;
        match token {
            xmlparser::Token::DtdStart { .. } | xmlparser::Token::EmptyDtd { .. } => {
                return Err(PolicyDocumentError::Xml(
                    "the document declares a document type, which this reader does not process"
                        .to_owned(),
                ));
            }
            xmlparser::Token::ElementStart { prefix, local, .. } => {
                budget.take()?;
                if open.len() >= usize::from(der::MAX_DEPTH) {
                    return Err(PolicyDocumentError::NestedTooDeep);
                }
                let index = elements.len();
                if let Some(parent) = open.last().and_then(|&parent| elements.get_mut(parent)) {
                    parent.children.push(index);
                }
                elements.push(Element {
                    prefix: prefix.as_str().to_owned(),
                    space: Space::Other,
                    expanded: String::new(),
                    local: local.as_str().to_owned(),
                    attributes: Vec::new(),
                    children: Vec::new(),
                    text: String::new(),
                });
                open.push(index);
                scope.declared.push(0);
            }
            xmlparser::Token::Attribute {
                prefix,
                local,
                value,
                ..
            } => {
                let mut expanded = String::with_capacity(value.len());
                unescape(value.as_str(), &mut expanded);
                let (prefix, local) = (prefix.as_str(), local.as_str());
                if prefix == "xmlns" || (prefix.is_empty() && local == "xmlns") {
                    budget.take()?;
                    scope
                        .declare(if prefix.is_empty() { "" } else { local }, expanded)
                        .ok_or_else(unbalanced)?;
                } else if prefix.is_empty() {
                    open.last()
                        .and_then(|&index| elements.get_mut(index))
                        .ok_or_else(unbalanced)?
                        .attributes
                        .push((local.to_owned(), expanded));
                }
            }
            xmlparser::Token::ElementEnd { end, .. } => {
                end_tag(end, &mut elements, &mut open, &mut scope)?;
            }
            xmlparser::Token::Text { text } => {
                if let Some(element) = open.last().and_then(|&index| elements.get_mut(index)) {
                    unescape(text.as_str(), &mut element.text);
                }
            }
            xmlparser::Token::Cdata { text, .. } => {
                if let Some(element) = open.last().and_then(|&index| elements.get_mut(index)) {
                    element.text.push_str(text.as_str());
                }
            }
            _ => {}
        }
    }
    // The tokenizer hands back the tokens of a truncated document without complaint.
    if !open.is_empty() || elements.is_empty() {
        return Err(PolicyDocumentError::Xml(
            "the document's root element is not closed".to_owned(),
        ));
    }
    Ok(Tree { elements })
}

/// The end of a start tag, where its namespace is resolved, or an element's close.
fn end_tag(
    end: xmlparser::ElementEnd<'_>,
    elements: &mut [Element],
    open: &mut Vec<usize>,
    scope: &mut Declarations,
) -> Result<(), PolicyDocumentError> {
    let unbalanced =
        || PolicyDocumentError::Xml("an element is closed without being opened".to_owned());
    if matches!(
        end,
        xmlparser::ElementEnd::Open | xmlparser::ElementEnd::Empty
    ) {
        let element = open
            .last()
            .and_then(|&index| elements.get_mut(index))
            .ok_or_else(unbalanced)?;
        resolve(element, &scope.bindings)?;
    }
    if let xmlparser::ElementEnd::Close(prefix, local) = end {
        let element = open
            .last()
            .and_then(|&index| elements.get(index))
            .ok_or_else(unbalanced)?;
        if element.prefix != prefix.as_str() || element.local != local.as_str() {
            return Err(PolicyDocumentError::Xml(format!(
                "<{}> is closed by another element's end tag",
                element.local
            )));
        }
    }
    if !matches!(end, xmlparser::ElementEnd::Open) {
        open.pop().ok_or_else(unbalanced)?;
        scope.close().ok_or_else(unbalanced)?;
    }
    Ok(())
}

/// The namespace declarations in scope, innermost last, and how many each open element added.
#[derive(Default)]
struct Declarations {
    bindings: Vec<(String, String)>,
    declared: Vec<usize>,
}

impl Declarations {
    /// Binds `prefix` (empty for the default namespace) on the innermost open element; `None`
    /// where no element is open.
    fn declare(&mut self, prefix: &str, uri: String) -> Option<()> {
        let count = self.declared.last_mut()?;
        *count = count.saturating_add(1);
        self.bindings.push((prefix.to_owned(), uri));
        Some(())
    }

    /// Drops the innermost element's declarations as it closes.
    fn close(&mut self) -> Option<()> {
        let added = self.declared.pop()?;
        self.bindings
            .truncate(self.bindings.len().saturating_sub(added));
        Some(())
    }
}

/// An element's namespace, from the declarations in scope at the end of its start tag (XML
/// Namespaces 1.0 section 6.2).
fn resolve(
    element: &mut Element,
    bindings: &[(String, String)],
) -> Result<(), PolicyDocumentError> {
    let uri = bindings
        .iter()
        .rev()
        .find(|(prefix, _)| *prefix == element.prefix)
        .map(|(_, uri)| uri.as_str());
    let uri = match (uri, element.prefix.as_str()) {
        (Some(uri), _) => uri,
        (None, "") => "",
        (None, prefix) => {
            return Err(PolicyDocumentError::Xml(format!(
                "the prefix {prefix} is not declared"
            )));
        }
    };
    element.space = if NAMESPACE.contains(&uri) {
        Space::Policy
    } else if uri == XMLDSIG_NAMESPACE {
        Space::Signature
    } else {
        Space::Other
    };
    element.expanded = format!("{{{uri}}}{}", element.local);
    Ok(())
}

/// Expands the five entities XML 1.0 section 4.6 predefines and section 4.1's character
/// references, appending to `out`; any other reference is kept as written.
fn unescape(text: &str, out: &mut String) {
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return;
        };
        let reference = &rest[1..end];
        let character = match reference {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => reference
                .strip_prefix("#x")
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| reference.strip_prefix('#').map(str::parse::<u32>))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        };
        match character {
            Some(character) => out.push(character),
            None => out.push_str(&rest[..=end]),
        }
        rest = &rest[end.saturating_add(1)..];
    }
    out.push_str(rest);
}

/// The second step: the tree's elements read as clause 4's types.
struct Reader<'t> {
    tree: &'t Tree,
    oids: &'t [Option<Vec<u8>>],
    budget: Budget,
}

impl<'t> Reader<'t> {
    fn element(&self, index: usize) -> Result<&'t Element, PolicyDocumentError> {
        self.tree
            .elements
            .get(index)
            .ok_or(malformed("SignaturePolicy"))
    }

    /// The children of `parent` of this namespace and local name.
    fn all(
        &self,
        parent: usize,
        space: Space,
        local: &'static str,
    ) -> Result<Vec<usize>, PolicyDocumentError> {
        let tree = self.tree;
        Ok(self
            .element(parent)?
            .children
            .iter()
            .copied()
            .filter(|&child| {
                tree.elements
                    .get(child)
                    .is_some_and(|element| element.space == space && element.local == local)
            })
            .collect())
    }

    /// The first child of `parent` in clause 4.1.2's namespace of this local name.
    fn optional(
        &self,
        parent: usize,
        local: &'static str,
    ) -> Result<Option<usize>, PolicyDocumentError> {
        Ok(self.all(parent, Space::Policy, local)?.first().copied())
    }

    /// [`Self::optional`] where clause 4 requires the element; `member` names it.
    fn required(
        &self,
        parent: usize,
        local: &'static str,
        member: &'static str,
    ) -> Result<usize, PolicyDocumentError> {
        self.optional(parent, local)?.ok_or(malformed(member))
    }

    /// The whitespace-collapsed text of an element, as `xs:anyURI`, `xs:token` and the numeric
    /// types read it (XML Schema 1.1 Part 2 section 4.3.6).
    fn text(&self, index: usize) -> Result<&'t str, PolicyDocumentError> {
        Ok(self.element(index)?.text.trim())
    }

    /// The text of a required child.
    fn text_of(
        &self,
        parent: usize,
        local: &'static str,
        member: &'static str,
    ) -> Result<&'t str, PolicyDocumentError> {
        let child = self.required(parent, local, member)?;
        let text = self.text(child)?;
        if text.is_empty() {
            return Err(malformed(member));
        }
        Ok(text)
    }

    /// An `xs:list` value's tokens, each charged to the budget.
    fn tokens(&mut self, value: &'t str) -> Result<Vec<&'t str>, PolicyDocumentError> {
        let mut out = Vec::new();
        for token in value.split_ascii_whitespace() {
            self.budget.take()?;
            out.push(token);
        }
        Ok(out)
    }

    /// The texts of every child of this name, at least one; `member` names the list.
    fn texts(
        &mut self,
        parent: usize,
        local: &'static str,
        member: &'static str,
    ) -> Result<Vec<&'t str>, PolicyDocumentError> {
        let children = self.all(parent, Space::Policy, local)?;
        if children.is_empty() {
            return Err(malformed(member));
        }
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            let text = self.text(child)?;
            if text.is_empty() {
                return Err(malformed(member));
            }
            out.push(text);
        }
        Ok(out)
    }

    /// An identifier: the object identifier an `urn:oid:` URN spells, an algorithm URI
    /// [`ALGORITHM_URIS`] places where `algorithm`, or the URI itself.
    fn identifier(
        &self,
        index: usize,
        algorithm: bool,
        member: &'static str,
    ) -> Result<Identifier<'t>, PolicyDocumentError> {
        let text = self.text(index)?;
        if text.is_empty() {
            return Err(malformed(member));
        }
        if let Some(Some(oid)) = self.oids.get(index) {
            return Ok(Identifier::Oid(oid));
        }
        if algorithm && let Some((_, oid)) = ALGORITHM_URIS.iter().find(|(uri, _)| *uri == text) {
            return Ok(Identifier::Oid(oid.as_bytes()));
        }
        Ok(Identifier::Uri(text))
    }

    /// Clause 4.1.3's `AnyType`, named by its first element.
    fn extension(&self, index: usize) -> Result<ExtensionName<'t>, PolicyDocumentError> {
        let element = self.element(index)?;
        Ok(match element.children.first() {
            Some(&first) => ExtensionName::Element(&self.element(first)?.expanded),
            None => ExtensionName::Element("text"),
        })
    }

    /// Clause 4.24.4's `SigningCertTrustConditions`, read as
    /// [`ts_119_172_3`]'s `read_signer_trust` reads the ASN.1 one: clause 4.22.5's trust points,
    /// one per certificate trust anchor, and clause 4.23.3's revocation trust.
    fn signer_trust(&mut self, conditions: usize) -> Result<SignerTrust<'t>, PolicyDocumentError> {
        let trees = self.required(conditions, "SignerTrustTrees", "SignerTrustTrees")?;
        let trees = self.trust_trees(trees)?;
        let revocation = self.required(conditions, "SignerRevTrust", "SignerRevTrust")?;
        Ok(SignerTrust {
            trees,
            revocation: self.revocation_trust(revocation)?,
        })
    }

    /// Clause 4.22.5's `CertificateTrustTreesType`: one trust point per certificate trust anchor,
    /// and a count of the anchors given by trusted-list reference.
    fn trust_trees(&mut self, trees: usize) -> Result<TrustTrees<'t>, PolicyDocumentError> {
        let mut out = TrustTrees::default();
        for point in self.all(trees, Space::Policy, "CertificateTrustPoint")? {
            self.budget.take()?;
            let path_length = self
                .optional(point, "PathLenConstraint")?
                .map(|limit| self.integer(limit, "PathLenConstraint"))
                .transpose()?;
            if path_length.is_some_and(|limit| limit < 0) {
                return Err(malformed("PathLenConstraint"));
            }
            let mut unprocessed = Vec::new();
            let inputs = self.path_inputs(point, &mut unprocessed)?;
            // Clause 4.22.5.1 item 5: `AsInSignature` is the path the search builds anyway, and the
            // `X509Certificate` list is the path it builds over instead (ADR 2781).
            let mut mandated_path = None;
            if let Some(path) = self.optional(point, "UseCertPath")?
                && self.optional(path, "AsInSignature")?.is_none()
            {
                let mut certificates = Vec::new();
                for certificate in self.all(path, Space::Policy, "X509Certificate")? {
                    self.budget.take()?;
                    let der = base64(&self.element(certificate)?.text)
                        .ok_or(malformed("UseCertPath.X509Certificate"))?;
                    certificates.push(std::borrow::Cow::Owned(der));
                }
                if certificates.is_empty() {
                    return Err(malformed("UseCertPath"));
                }
                mandated_path = Some(certificates);
            }
            let anchors =
                self.required(point, "TrustAnchors", "CertificateTrustPoint.TrustAnchors")?;
            for anchor in self.all(anchors, Space::Policy, "X509CertificateBased")? {
                self.budget.take()?;
                let certificate =
                    base64(&self.element(anchor)?.text).ok_or(malformed("X509CertificateBased"))?;
                let reliable_until = self
                    .element(anchor)?
                    .attributes
                    .iter()
                    .find(|(name, _)| name == "reliableUntil")
                    .map(|(_, spelled)| {
                        date_time(spelled.trim())
                            .map(|instant| Stated {
                                instant,
                                spelled: spelled.trim().to_owned(),
                            })
                            .ok_or(malformed("X509CertificateBased.reliableUntil"))
                    })
                    .transpose()?;
                out.points.push(TrustPoint {
                    certificate: std::borrow::Cow::Owned(certificate),
                    path_length,
                    reliable_until,
                    inputs: inputs.clone(),
                    unprocessed: unprocessed.clone(),
                    mandated_path: mandated_path.clone(),
                });
            }
            let lists = self.all(anchors, Space::Policy, "TAsInTrustedList")?.len();
            out.trusted_lists = out.trusted_lists.saturating_add(lists);
        }
        Ok(out)
    }

    /// Clause 4.22.3's `NameConstraintsType`: each `GeneralSubTree`'s base is the DER of a
    /// `GeneralName` (clause 4.22.3.2). Where a subtree states a distance, which RFC 5280 section
    /// 4.2.1.10 leaves unused and this program does not process, the whole constraint is kept by
    /// name in `unprocessed` and `None` returned, so that no subset of it is evaluated as though it
    /// were all of it.
    fn name_constraints(
        &mut self,
        constraints: usize,
        unprocessed: &mut Vec<&'static str>,
    ) -> Result<Option<crate::trust::names::NameConstraints>, PolicyDocumentError> {
        use crate::trust::names;
        let mut read = names::NameConstraints::default();
        for (local, permitted) in [("PermittedSubtrees", true), ("ExcludedSubtrees", false)] {
            let Some(list) = self.optional(constraints, local)? else {
                continue;
            };
            let mut subtrees = Vec::new();
            for subtree in self.all(list, Space::Policy, "GeneralSubTree")? {
                self.budget.take()?;
                let base = base64(self.text_of(subtree, "Base", "GeneralSubTree.Base")?)
                    .ok_or(malformed("GeneralSubTree.Base"))?;
                let minimum = self
                    .optional(subtree, "Minimum")?
                    .map(|minimum| self.integer(minimum, "GeneralSubTree.Minimum"))
                    .transpose()?
                    .unwrap_or(0);
                let maximum = self
                    .optional(subtree, "Maximum")?
                    .map(|maximum| self.integer(maximum, "GeneralSubTree.Maximum"))
                    .transpose()?;
                match names::subtree_of(&base, minimum, maximum) {
                    Ok(read) if subtrees.len() < names::MAX_SUBTREES => subtrees.push(read),
                    Err(names::Unreadable::DistanceStated) => {
                        unprocessed.push("name constraints");
                        return Ok(None);
                    }
                    _ => return Err(malformed("GeneralSubTree")),
                }
            }
            if subtrees.is_empty() {
                continue;
            }
            if permitted {
                read.permitted = Some(subtrees);
            } else {
                read.excluded = subtrees;
            }
        }
        if read.permitted.is_none() && read.excluded.is_empty() {
            return Err(malformed("NameConstraints"));
        }
        Ok(Some(read))
    }

    /// Clause 4.22.5's `AcceptablePolicySet`, `NameConstraints` and `PolicyConstraints` of one
    /// trust point, as RFC 5280 section 6.1.1's inputs for the path to it (ADR 2724).
    ///
    /// An acceptable policy that is a URI and not an object identifier names nothing a
    /// certificate's `certificatePolicies` can state, so the set is then kept by name in
    /// `unprocessed` rather than evaluated. Clause 4.22.4.2's schema names the mapping member
    /// `InhibitExplicitPolicy`; clause 4.22.4.1's semantics make its second component the one that
    /// restricts policy mapping, and that is how it is read.
    fn path_inputs(
        &mut self,
        point: usize,
        unprocessed: &mut Vec<&'static str>,
    ) -> Result<crate::trust::PathInputs, PolicyDocumentError> {
        use crate::trust::policies;
        let mut inputs = crate::trust::PathInputs::default();
        if let Some(set) = self.optional(point, "AcceptablePolicySet")? {
            let mut oids = Vec::new();
            let mut comparable = true;
            for policy in self.all(set, Space::Policy, "AcceptablePolicy")? {
                self.budget.take()?;
                match self.identifier(policy, false, "AcceptablePolicy")? {
                    Identifier::Oid(oid) if oids.len() < policies::MAX_POLICIES => {
                        oids.push(oid.to_vec());
                    }
                    Identifier::Oid(_) => return Err(malformed("AcceptablePolicySet")),
                    Identifier::Uri(_) => comparable = false,
                }
            }
            if comparable {
                inputs.policy_set = Some(oids);
            } else {
                unprocessed.push("acceptable policy set");
            }
        }
        if let Some(constraints) = self.optional(point, "NameConstraints")? {
            inputs.name_constraints = self.name_constraints(constraints, unprocessed)?;
        }
        if let Some(constraints) = self.optional(point, "PolicyConstraints")? {
            let count =
                |this: &Self, local: &'static str| -> Result<Option<usize>, PolicyDocumentError> {
                    this.optional(constraints, local)?
                        .map(|value| {
                            let value = this.integer(value, local)?;
                            usize::try_from(value).map_err(|_| malformed(local))
                        })
                        .transpose()
                };
            inputs.require_explicit_policy = count(self, "RequireExplicitPolicy")?;
            inputs.inhibit_policy_mapping = count(self, "InhibitExplicitPolicy")?;
            if inputs.require_explicit_policy.is_none() && inputs.inhibit_policy_mapping.is_none() {
                return Err(malformed("PolicyConstraints"));
            }
        }
        Ok(inputs)
    }

    /// Clause 4.23.3's `CertificateRevTrustType`: clause 4.23.2's check for each kind of
    /// certificate, and the members this program does not evaluate by name.
    fn revocation_trust(
        &self,
        revocation: usize,
    ) -> Result<revocation_checks::Requirement, PolicyDocumentError> {
        let requirement = self.required(revocation, "CertificateRevReq", "CertificateRevReq")?;
        let check = |local, member| {
            self.text_of(requirement, local, member).and_then(|text| {
                revocation_checks::Check::of_name(text.trim()).ok_or(malformed(member))
            })
        };
        // Clause 4.23.3.2 types the member `xs:duration`, the lexical form `Freshness` takes.
        let local = "SigCertIssuedByCAKeepsExpiredRevokedCertsInfo";
        let retention = match self.optional(revocation, local)? {
            Some(_) => {
                let member = "CertificateRevTrust.SigCertIssuedByCAKeepsExpiredRevokedCertsInfo";
                let text = self.text_of(revocation, local, member)?;
                Some(Duration::parse(text.trim()).ok_or(malformed(member))?)
            }
            None => None,
        };
        let freshness = match self.optional(revocation, "Freshness")? {
            Some(freshness) => Some(self.freshness(freshness)?),
            None => None,
        };
        Ok(revocation_checks::Requirement {
            end_entity: check("EndRevReq", "CertificateRevReq.EndRevReq")?,
            authorities: check("CACerts", "CertificateRevReq.CACerts")?,
            freshness,
            retention,
        })
    }

    /// Clause 4.23.3.2's `Freshness`: a choice of two `xs:duration` elements.
    fn freshness(&self, freshness: usize) -> Result<Freshness, PolicyDocumentError> {
        let duration = |local, member| {
            self.text_of(freshness, local, member)
                .and_then(|text| Duration::parse(text.trim()).ok_or(malformed(member)))
        };
        match (
            self.optional(freshness, "MaxDifferenceRevocationAndValidation")?,
            self.optional(freshness, "TimeAfterSignature")?,
        ) {
            (Some(_), None) => Ok(Freshness::MaxDifference(duration(
                "MaxDifferenceRevocationAndValidation",
                "Freshness.MaxDifferenceRevocationAndValidation",
            )?)),
            (None, Some(_)) => Ok(Freshness::AfterSignature(duration(
                "TimeAfterSignature",
                "Freshness.TimeAfterSignature",
            )?)),
            _ => Err(malformed("Freshness")),
        }
    }

    fn integer(&self, index: usize, member: &'static str) -> Result<i64, PolicyDocumentError> {
        self.text(index)?
            .strip_prefix('+')
            .unwrap_or(self.text(index)?)
            .parse()
            .map_err(|_| malformed(member))
    }

    fn instant(&self, index: usize, member: &'static str) -> Result<Stated, PolicyDocumentError> {
        let spelled = self.text(index)?;
        let instant = date_time(spelled).ok_or(malformed(member))?;
        Ok(Stated {
            instant,
            spelled: spelled.to_owned(),
        })
    }

    fn document(&mut self) -> Result<PolicyDocument<'t>, PolicyDocumentError> {
        let root = 0;
        let digest = self.required(root, "Digest", "Digest")?;
        let signature_child = |reader: &Self, local: &'static str| {
            reader
                .all(digest, Space::Signature, local)
                .map(|found| found.first().copied())
        };
        let method =
            signature_child(self, "DigestMethod")?.ok_or(malformed("Digest.DigestMethod"))?;
        let value = signature_child(self, "DigestValue")?.ok_or(malformed("Digest.DigestValue"))?;
        let canonicalization = signature_child(self, "CanonicalizationMethod")?
            .ok_or(malformed("Digest.CanonicalizationMethod"))?;
        let algorithm = |reader: &Self, index: usize, member: &'static str| {
            reader
                .element(index)?
                .attributes
                .iter()
                .find(|(name, _)| name == "Algorithm")
                .map(|(_, uri)| uri.trim())
                .filter(|uri| !uri.is_empty())
                .ok_or(malformed(member))
        };
        let digest_method = algorithm(self, method, "Digest.DigestMethod")?;
        let prefix_list = self.prefix_list(canonicalization)?;
        let canonicalization = algorithm(self, canonicalization, "Digest.CanonicalizationMethod")?;
        let digest_value =
            base64(&self.element(value)?.text).ok_or(malformed("Digest.DigestValue"))?;
        let components = self.required(root, "PolicyComponents", "PolicyComponents")?;
        let general = self.required(components, "GeneralDetails", "GeneralDetails")?;
        let details = self.required(general, "SigPolicyDetails", "SigPolicyDetails")?;
        let identifier = self.text_of(
            details,
            "PolicyIdentifier",
            "SigPolicyDetails.PolicyIdentifier",
        )?;
        let mut signing_period = None;
        let mut other_details = Vec::new();
        if let Some(other) = self.optional(general, "OtherDetails")? {
            if let Some(period) = self.optional(other, "SigningPeriod")? {
                let not_before = self.required(period, "NotBefore", "SigningPeriod.NotBefore")?;
                let not_before = self.instant(not_before, "SigningPeriod.NotBefore")?;
                let not_after = match self.optional(period, "NotAfter")? {
                    Some(index) => Some(self.instant(index, "SigningPeriod.NotAfter")?),
                    None => None,
                };
                signing_period = Some((not_before, not_after));
            }
            for entry in self.all(other, Space::Policy, "Other")? {
                other_details.push(self.extension(entry)?);
            }
        }
        let list = self.required(components, "PolicyRules", "PolicyRules")?;
        let mut rules = Vec::new();
        for rule in self.all(list, Space::Policy, "PolicyRule")? {
            rules.push(self.scoped_rule(rule)?);
        }
        if rules.is_empty() {
            return Err(malformed("PolicyRules.PolicyRule"));
        }
        Ok(PolicyDocument {
            digest_method,
            digest_value,
            canonicalization,
            prefix_list,
            identifier,
            signing_period,
            other_details,
            rules,
        })
    }

    /// Clause 4.14.2's `PolicyRule`: one rule of the choice, and its recommended scopes.
    /// The `PrefixList` of the first `InclusiveNamespaces` child of `method` in Exclusive XML
    /// Canonicalization 1.0's namespace, which its section 4 makes the parameter's element.
    fn prefix_list(&self, method: usize) -> Result<Option<&'t str>, PolicyDocumentError> {
        let tree = self.tree;
        let expanded = format!(
            "{{{}}}InclusiveNamespaces",
            canonical_xml::EXC_C14N_NAMESPACE
        );
        Ok(self
            .element(method)?
            .children
            .iter()
            .filter_map(|&child| tree.elements.get(child))
            .find(|element| element.expanded == expanded)
            .and_then(|element| {
                element
                    .attributes
                    .iter()
                    .find(|(name, _)| name == "PrefixList")
                    .map(|(_, list)| list.as_str())
            }))
    }

    fn scoped_rule(&mut self, index: usize) -> Result<ScopedRule<'t>, PolicyDocumentError> {
        let element = self.element(index)?;
        let scope = match element
            .attributes
            .iter()
            .find(|(name, _)| name == "RecommendedScope")
        {
            None => None,
            Some((_, list)) => {
                let mut scopes = Vec::new();
                for token in self.tokens(list)? {
                    scopes.push(match token {
                        "Generation" => Scope::Generation,
                        "Validation" => Scope::Validation,
                        "Augmentation" => Scope::Augmentation,
                        _ => return Err(malformed("PolicyRule.RecommendedScope")),
                    });
                }
                // A rule recommended for validation among other scopes is a rule this program
                // uses; one recommended for other scopes alone is named by the first of them.
                if scopes.contains(&Scope::Validation) {
                    Some(Scope::Validation)
                } else {
                    scopes.first().copied()
                }
            }
        };
        let [only] = element.children.as_slice() else {
            return Err(malformed("PolicyRule"));
        };
        let rule = if self.is(*only, "CommitmentRules")? {
            PolicyRule::Commitments(self.commitment_rules(*only)?)
        } else {
            PolicyRule::Basic(self.basic_rule(*only)?)
        };
        Ok(ScopedRule { rule, scope })
    }

    fn is(&self, index: usize, local: &str) -> Result<bool, PolicyDocumentError> {
        let element = self.element(index)?;
        Ok(element.space == Space::Policy && element.local == local)
    }

    /// Clause 4.15.2's `CommitmentRules`, whose components the schema spells
    /// `ComitmentRulesComponent`.
    fn commitment_rules(
        &mut self,
        index: usize,
    ) -> Result<Vec<CommitmentRule<'t>>, PolicyDocumentError> {
        let mut out = Vec::new();
        for component in self.all(index, Space::Policy, "ComitmentRulesComponent")? {
            let matching = match self
                .element(component)?
                .attributes
                .iter()
                .find(|(name, _)| name == "MatchingIndicator")
                .map(|(_, value)| value.trim())
            {
                Some("All") => Matching::All,
                Some("None") => Matching::None,
                Some("AtLeastOne") => Matching::AtLeastOne,
                _ => return Err(malformed("ComitmentRulesComponent.MatchingIndicator")),
            };
            let mut commitments = Vec::new();
            let mut rules = Vec::new();
            for &child in &self.element(component)?.children {
                if self.is(child, "Commitment")? {
                    let identifier = self.required(child, "Identifier", "Commitment.Identifier")?;
                    commitments.push(self.identifier(
                        identifier,
                        false,
                        "Commitment.Identifier",
                    )?);
                } else {
                    rules.push(self.basic_rule(child)?);
                }
            }
            if commitments.is_empty() || rules.is_empty() {
                return Err(malformed("ComitmentRulesComponent"));
            }
            out.push(CommitmentRule {
                commitments,
                matching,
                rules,
            });
        }
        if out.is_empty() {
            return Err(malformed("CommitmentRules.ComitmentRulesComponent"));
        }
        Ok(out)
    }

    /// One element of clause 4.14.2's `BasicRule` group.
    fn basic_rule(&mut self, index: usize) -> Result<BasicRule<'t>, PolicyDocumentError> {
        let element = self.element(index)?;
        if element.space != Space::Policy {
            return Err(malformed("BasicRule"));
        }
        Ok(match element.local.as_str() {
            "DataToBeSignedRules" => BasicRule::DataToBeSigned(vec![self.media_types(index)?]),
            "SigToDTBSRelationRules" => {
                let cardinality = self.required(
                    index,
                    "DTBSCardinality",
                    "SigToDTBSRelationRules.DTBSCardinality",
                )?;
                let cardinality = self.cardinality(cardinality)?;
                let position = match self.optional(index, "SigDTBSRelativePosition")? {
                    Some(list) => Some(self.positions(list)?),
                    None => None,
                };
                let formats = match self.optional(index, "SigFormatsAndLevels")? {
                    Some(formats) => Some(self.formats(formats)?),
                    None => None,
                };
                BasicRule::Relation {
                    cardinality,
                    position,
                    formats,
                }
            }
            "SigFormatsAndLevels" => BasicRule::FormatsAndLevels(self.formats(index)?),
            "AugmentationRules" => BasicRule::Augmentation,
            "SigningCertRules" => {
                let conditions = self.required(
                    index,
                    "SigningCertTrustConditions",
                    "SigningCertRules.SigningCertTrustConditions",
                )?;
                let trust = self.signer_trust(conditions)?;
                let info = match self.optional(index, "MandatedSigningCertInfo")? {
                    None => None,
                    Some(info) => Some(match self.text(info)? {
                        "signingCertOnly" => CertificateInfo::SignerOnly,
                        "fullPath" => CertificateInfo::FullPath,
                        _ => return Err(malformed("MandatedSigningCertInfo")),
                    }),
                };
                BasicRule::SigningCertificate { trust, info }
            }
            "TimeEvidencesRules" => BasicRule::TimeEvidences(self.time_evidences(index)?),
            "SignerAttributesConstraints" => {
                let none_allowed = self.optional(index, "NoSignerAttributesAllowed")?.is_some();
                let constrained_sets = self
                    .all(index, Space::Policy, "ConstraintsOnOneSetOfAttributes")?
                    .len();
                if none_allowed == (constrained_sets > 0) {
                    return Err(malformed("SignerAttributesConstraints"));
                }
                BasicRule::SignerAttributes {
                    none_allowed,
                    constrained_sets,
                }
            }
            "QualifyingPropertiesRules" => {
                let mut levels = Vec::new();
                for rules in self.all(index, Space::Policy, "LevelRules")? {
                    levels.push(match self.optional(rules, "LevelIdentifier")? {
                        Some(level) => self.text(level)?,
                        None => "(none named)",
                    });
                }
                if levels.is_empty() {
                    return Err(malformed("QualifyingPropertiesRules.LevelRules"));
                }
                BasicRule::QualifyingAttributes(levels)
            }
            "SCDLoARules" => {
                let level = self.text(index)?;
                if level.is_empty() {
                    return Err(malformed("SCDLoARules"));
                }
                BasicRule::CreationDevice(level)
            }
            "CryptoSuitesRules" => BasicRule::CryptoSuites(self.crypto_suites(index)?),
            "OtherRule" => BasicRule::Other(self.extension(index)?),
            _ => return Err(malformed("BasicRule")),
        })
    }

    /// Clause 4.16's `DataToBeSignedRules`, which may not be empty.
    fn media_types(&mut self, index: usize) -> Result<MediaTypes<'t>, PolicyDocumentError> {
        let list = |reader: &mut Self, local: &'static str| -> Result<_, PolicyDocumentError> {
            match reader.optional(index, local)? {
                None => Ok(None),
                Some(child) => {
                    let text = reader.text(child)?;
                    Ok(Some(reader.tokens(text)?))
                }
            }
        };
        let any_of = list(self, "AnyOfMimeType")?;
        let none_of = list(self, "NoneOfMimeType")?;
        if any_of.is_none() && none_of.is_none() {
            return Err(malformed("DataToBeSignedRules"));
        }
        Ok(MediaTypes { any_of, none_of })
    }

    /// Clause 4.18's `DTBSCardinality`.
    fn cardinality(&self, index: usize) -> Result<Cardinality, PolicyDocumentError> {
        let mut bounds = Vec::new();
        for (local, member) in [
            ("MaximumValue", "DTBSCardinality.MaximumValue"),
            ("MinimumValue", "DTBSCardinality.MinimumValue"),
        ] {
            let Some(bound) = self.optional(index, local)? else {
                continue;
            };
            let qualifier = self
                .element(bound)?
                .attributes
                .iter()
                .find(|(name, _)| name == "qualifier")
                .map(|(_, value)| value.trim());
            let comparison = match (local, qualifier) {
                ("MaximumValue", Some("LessThan")) => Comparison::FewerThan,
                ("MaximumValue", Some("LessOrEqualTo")) => Comparison::AtMost,
                ("MinimumValue", Some("HigherThan")) => Comparison::MoreThan,
                ("MinimumValue", Some("HigherOrEqualTo")) => Comparison::AtLeast,
                (_, Some("Equal")) => Comparison::Exactly,
                _ => return Err(malformed(member)),
            };
            let number = self.integer(bound, member)?;
            if i32::try_from(number).is_err() {
                return Err(malformed(member));
            }
            bounds.push((comparison, number));
        }
        Ok(Cardinality { bounds })
    }

    /// Clause 4.19's `SigDTBSRelativePosition`: one to three positions, or `ASiC` alone.
    fn positions(&mut self, index: usize) -> Result<Vec<Position>, PolicyDocumentError> {
        let text = self.text(index)?;
        let mut out = Vec::new();
        for token in self.tokens(text)? {
            out.push(match token {
                "EnvelopingSig" => Position::Enveloping,
                "EnvelopedSig" => Position::Enveloped,
                "DetachedSig" => Position::Detached,
                "ASiC" => Position::Asic,
                _ => return Err(malformed("SigDTBSRelativePosition")),
            });
        }
        let asic_with_another = out.contains(&Position::Asic) && out.len() > 1;
        if out.is_empty() || out.len() > 3 || asic_with_another {
            return Err(malformed("SigDTBSRelativePosition"));
        }
        Ok(out)
    }

    /// Clause 4.20's `SigFormatsAndLevels`, which may not be empty.
    fn formats(&mut self, index: usize) -> Result<FormatsAndLevels<'t>, PolicyDocumentError> {
        let formats = match self.optional(index, "SigFormats")? {
            Some(list) => Some(self.texts(list, "Format", "SigFormats.Format")?),
            None => None,
        };
        let level_lists = self.all(index, Space::Policy, "SigLevels")?;
        let levels = if level_lists.is_empty() {
            None
        } else {
            let mut levels = Vec::new();
            for list in level_lists {
                levels.extend(self.texts(list, "SigLevel", "SigLevels.SigLevel")?);
            }
            Some(levels)
        };
        if formats.is_none() && levels.is_none() {
            return Err(malformed("SigFormatsAndLevels"));
        }
        Ok(FormatsAndLevels { formats, levels })
    }

    /// Clause 4.25's `TimeEvidencesRules`.
    fn time_evidences(
        &mut self,
        index: usize,
    ) -> Result<Vec<TimeEvidence<'t>>, PolicyDocumentError> {
        let mut out = Vec::new();
        for set in self.all(index, Space::Policy, "RulesForSetOfEvidences")? {
            let identifiers = self.required(
                set,
                "EvidenceIdentifiers",
                "RulesForSetOfEvidences.EvidenceIdentifiers",
            )?;
            let evidences =
                self.texts(identifiers, "Identifier", "EvidenceIdentifiers.Identifier")?;
            let level_of_assurance = self.text_of(
                set,
                "LevelOfAssurance",
                "RulesForSetOfEvidences.LevelOfAssurance",
            )?;
            let mut trust = None;
            let mut delay_seconds = None;
            if let Some(condition) = self.optional(set, "TimeStampTrustCondition")? {
                let trees = self
                    .optional(condition, "TstCertificateTrustTrees")?
                    .map(|trees| self.trust_trees(trees))
                    .transpose()?;
                let revocation = self
                    .optional(condition, "TstRevocationTrust")?
                    .map(|revocation| self.revocation_trust(revocation))
                    .transpose()?;
                let mut unprocessed = Vec::new();
                let names = match self.optional(condition, "TstNameConstraints")? {
                    Some(names) => self.name_constraints(names, &mut unprocessed)?,
                    None => None,
                };
                if trees.is_some()
                    || revocation.is_some()
                    || names.is_some()
                    || !unprocessed.is_empty()
                {
                    trust = Some(TimestampTrust {
                        trees,
                        revocation,
                        names,
                        caution_seconds: None,
                        delay_seconds: None,
                        unprocessed,
                    });
                }
                if let Some(delay) = self.optional(condition, "SignatureTimeStampDelay")? {
                    delay_seconds = Some(self.delay(delay)?);
                }
            }
            out.push(TimeEvidence {
                evidences,
                level_of_assurance,
                trust,
                delay_seconds,
            });
        }
        if out.is_empty() {
            return Err(malformed("TimeEvidencesRules.RulesForSetOfEvidences"));
        }
        Ok(out)
    }

    /// Clause 4.25's `DeltaTimeType`, in seconds.
    fn delay(&self, index: usize) -> Result<i64, PolicyDocumentError> {
        let mut total: i64 = 0;
        for (local, member, unit) in [
            ("DeltaSeconds", "DeltaTime.DeltaSeconds", 1_i64),
            ("DeltaMinutes", "DeltaTime.DeltaMinutes", 60),
            ("DeltaHours", "DeltaTime.DeltaHours", 3_600),
            ("DeltaDays", "DeltaTime.DeltaDays", 86_400),
        ] {
            let field = self.required(index, local, member)?;
            total = self
                .integer(field, member)?
                .checked_mul(unit)
                .and_then(|seconds| total.checked_add(seconds))
                .ok_or(malformed(member))?;
        }
        Ok(total)
    }

    /// Clause 4.29's `CryptoSuitesRules`.
    fn crypto_suites(
        &mut self,
        index: usize,
    ) -> Result<Vec<AlgorithmConstraint<'t>>, PolicyDocumentError> {
        let mut out = Vec::new();
        for entry in self.all(index, Space::Policy, "AlgConstraints")? {
            let algorithm = self.required(entry, "AlgId", "AlgConstraints.AlgId")?;
            let algorithm = self.identifier(algorithm, true, "AlgConstraints.AlgId")?;
            // An absent `Usages` is every usage (clause 4.29.1), which the empty list says; a
            // present one names at least one.
            let usages = match self.optional(entry, "Usages")? {
                Some(usages) => {
                    let text = self.text(usages)?;
                    let usages = self.tokens(text)?;
                    if usages.is_empty() {
                        return Err(malformed("AlgConstraints.Usages"));
                    }
                    usages
                }
                None => Vec::new(),
            };
            let expiration =
                self.required(entry, "ExpirationDate", "AlgConstraints.ExpirationDate")?;
            let expiration = Some(self.instant(expiration, "AlgConstraints.ExpirationDate")?);
            let length = |reader: &Self, local: &'static str, member: &'static str| {
                reader
                    .optional(entry, local)?
                    .map(|child| reader.integer(child, member))
                    .transpose()
            };
            let min_key_length = length(self, "MinKeyLength", "AlgConstraints.MinKeyLength")?;
            let min_hash_length = length(self, "MinHashLength", "AlgConstraints.MinHashLength")?;
            let mut others = Vec::new();
            for other in self.all(entry, Space::Policy, "Other")? {
                others.push(self.extension(other)?);
            }
            out.push(AlgorithmConstraint {
                algorithm,
                usages,
                min_key_length,
                min_hash_length,
                others,
                expiration,
            });
        }
        if out.is_empty() {
            return Err(malformed("CryptoSuitesRules.AlgConstraints"));
        }
        Ok(out)
    }
}

/// An `xs:dateTime` (XML Schema 1.1 Part 2 section 3.3.7) as an instant: a four-digit year,
/// fractional seconds dropped, and a time zone required — an instant without one is not an
/// instant — `Z` or an offset of at most fourteen hours.
fn date_time(spelled: &str) -> Option<x509::Instant> {
    let bytes = spelled.as_bytes();
    let fixed = bytes.get(..19)?;
    let shape = |at: usize, separator: u8| fixed.get(at) == Some(&separator);
    if !(shape(4, b'-') && shape(7, b'-') && shape(10, b'T') && shape(13, b':') && shape(16, b':'))
    {
        return None;
    }
    let mut rest = bytes.get(19..)?;
    if let Some(fraction) = rest.strip_prefix(b".") {
        let digits = fraction
            .iter()
            .take_while(|octet| octet.is_ascii_digit())
            .count();
        if digits == 0 {
            return None;
        }
        rest = fraction.get(digits..)?;
    }
    let offset_seconds: i64 = match rest {
        b"Z" => 0,
        [sign @ (b'+' | b'-'), h1, h0, b':', m1, m0] => {
            let number = |pair: [u8; 2]| std::str::from_utf8(&pair).ok()?.parse::<i64>().ok();
            let hours = number([*h1, *h0])?;
            let minutes = number([*m1, *m0])?;
            let total = hours.checked_mul(60)?.checked_add(minutes)?;
            if minutes > 59 || total > 14 * 60 {
                return None;
            }
            let seconds = total.checked_mul(60)?;
            if *sign == b'+' {
                seconds
            } else {
                seconds.checked_neg()?
            }
        }
        _ => return None,
    };
    let mut digits = Vec::with_capacity(15);
    for (index, octet) in fixed.iter().enumerate() {
        if ![4, 7, 10, 13, 16].contains(&index) {
            digits.push(*octet);
        }
    }
    digits.push(b'Z');
    let local = x509::read_time_digits(&digits, true)?;
    // The instant is the local time less its offset from UTC.
    Some(x509::Instant::from_unix_seconds(
        local.unix_seconds().checked_sub(offset_seconds)?,
    ))
}

/// An `xs:base64Binary` value (RFC 4648 section 4), white space ignored as XML Schema's lexical
/// space allows; `None` where it is not one.
fn base64(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let symbols: Vec<u8> = text
        .bytes()
        .filter(|octet| !octet.is_ascii_whitespace())
        .collect();
    if !symbols.len().is_multiple_of(4) {
        return None;
    }
    let last = (symbols.len() / 4).checked_sub(1);
    let mut out = Vec::with_capacity(symbols.len());
    for (index, group) in symbols.chunks_exact(4).enumerate() {
        // `=` pads only the last group, and at most two of its symbols.
        let padding = group
            .iter()
            .rev()
            .take_while(|octet| **octet == b'=')
            .count();
        if padding > 2 || (padding > 0 && Some(index) != last) {
            return None;
        }
        let mut word = 0_u32;
        for (at, symbol) in group.iter().enumerate() {
            let sextet = if at < 4_usize.saturating_sub(padding) {
                u32::try_from(ALPHABET.iter().position(|letter| letter == symbol)?).ok()?
            } else {
                0
            };
            word = (word << 6) | sextet;
        }
        out.extend_from_slice(word.to_be_bytes().get(1..4_usize.saturating_sub(padding))?);
    }
    Some(out)
}

/// §12.8.3.4.4's enforcement over a parsed TS 119 172-2 document: every constraint judged against
/// one signature, by the same judgements as [`ts_119_172_3::findings`].
#[must_use]
pub fn findings(
    document: &PolicyDocument<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Vec<Finding> {
    let mut out = Vec::new();
    if let Some((not_before, not_after)) = &document.signing_period {
        out.push(signing_period(facts, not_before, not_after.as_ref()));
    }
    ts_119_172_3::judge_rules(&document.rules, cms, facts, &mut out);
    for other in &document.other_details {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: super::Constraint::Extension(other.sentence()),
        });
    }
    out
}

#[cfg(test)]
mod tests;
