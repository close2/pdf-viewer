//! W3C XML Signature 1.1's core validation of the one enveloped signature ETSI TS 119 612 puts
//! on a trusted list, under the certificate its `ds:KeyInfo` carries.
//!
//! TS 119 612 clause 5.7.1 requires a list to be signed by its scheme operator, in `XAdES` (ETSI EN
//! 319 132-1), with the operator's certificate in `ds:KeyInfo` and no other certificate beside it;
//! Annex B.1.0 narrows XML Signature to an enveloped signature whose `ds:SignedInfo` holds a
//! `ds:Reference` to the enveloping `TrustServiceStatusList`, transformed by the enveloped
//! signature transform and then exclusive canonicalisation, and whose own canonicalisation method
//! is exclusive canonicalisation. ETSI TS 119 615 clauses 4.1 and 4.2 then validate that signature
//! "considering" the certificate "as a trust anchor" (PRO-4.1.4-07, PRO-4.2.4-08), so what is
//! left to compute is XML Signature's own core validation (section 3.2): every reference's digest
//! (section 3.2.1), then the signature over the canonical `ds:SignedInfo` (section 3.2.2). The ETSI
//! texts and the Recommendation are cited by clause and paraphrased (ADR 1085).
//!
//! # The node sets, and the one element that is meant
//!
//! A reference names its data by a same-document URI (section 4.4.3.3): the empty URI is the
//! whole document without its comments, and `#id` the element carrying that identifier, again
//! without comments. Each is located in `crate::xml`'s tree and handed to the canonicaliser by
//! its ordinal in document order, never by its name: a document that carries a second
//! `ds:Signature` or a second element with one identifier is the shape of a signature-wrapping
//! attack, where the signed element and the believed one differ, so an identifier stated twice,
//! or a list with two signatures beside its root's children, is refused rather than resolved (ADR
//! 2843).
//!
//! # What is refused by name
//!
//! A transform other than the enveloped signature transform and the three canonicalisations
//! (`XPath`, `XSLT`, base64 decoding), a URI that is not same-document or that is an `xpointer`, a
//! digest or signature algorithm this crate does not compute, and more than one certificate in
//! `ds:KeyInfo` (clause 5.7.1). Each leaves the list unauthenticated and says why, rather than
//! digesting something other than what the signer signed.
//!
//! # The signing certificate's own reference
//!
//! EN 319 132-1 clause 5.2.2's `SigningCertificateV2`, which TS 119 612 Annex B.1.1 says shall be
//! used to secure the operator's identity: where the signed properties state one, its first
//! reference is the signing certificate's, and its digest is compared with the certificate
//! `ds:KeyInfo` carries. The older `SigningCertificate` of `XAdES` 1.3.2 is read the same way.

use crate::cms::Digest;
use crate::policy_rules::PolicyDocumentError;
use crate::policy_rules::canonical_xml::{
    self, Algorithm, Comments, EXC_C14N, Method, Subset, Target,
};
use crate::x509::{self, PublicKey};
use crate::xml::{self, Element, Tree, XmlError, base64};
use crate::{ecdsa, pkcs1, pss};

/// XML Signature's namespace.
pub const DSIG: &str = "http://www.w3.org/2000/09/xmldsig#";

/// Section 6.6.4's enveloped signature transform.
pub const ENVELOPED_SIGNATURE: &str = "http://www.w3.org/2000/09/xmldsig#enveloped-signature";

/// `XAdES`'s namespace (ETSI EN 319 132-1 clause 4.1).
const XADES: &str = "http://uri.etsi.org/01903/v1.3.2#";

/// Why a list's signature does not authenticate it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum XmlSignatureError {
    /// The XML cannot be read, or is past the bounds the caller set; the reason is said.
    #[error("the document's XML cannot be read: {0}")]
    Unreadable(String),
    /// The root has no `ds:Signature` child: the list is not signed (TS 119 612 clause 5.7.1).
    #[error("the list carries no enveloped signature (ETSI TS 119 612 clause 5.7.1)")]
    Unsigned,
    /// The root has more than one `ds:Signature` child, or an identifier is stated twice.
    #[error(
        "the list states {0} more than once, so which one was signed cannot be told (a \
         signature-wrapping shape, refused)"
    )]
    Ambiguous(String),
    /// An element or attribute XML Signature requires is missing or not of its form; it is named.
    #[error("the signature's {0} is missing or is not what XML Signature 1.1 gives it")]
    Malformed(&'static str),
    /// `ds:KeyInfo` does not carry exactly one certificate (TS 119 612 clause 5.7.1).
    #[error(
        "the signature's KeyInfo carries {0} certificates where ETSI TS 119 612 clause 5.7.1 \
         requires the scheme operator's alone"
    )]
    Certificates(usize),
    /// The certificate in `ds:KeyInfo` cannot be read as an X.509 certificate.
    #[error("the signature's certificate cannot be read: {0}")]
    Certificate(String),
    /// `ds:SignedInfo` holds no reference of the shape TS 119 612 Annex B.1.0 rule 2 requires.
    #[error(
        "no reference covers the whole list under the enveloped signature transform and \
         exclusive canonicalisation (ETSI TS 119 612 Annex B.1.0 rule 2)"
    )]
    ListNotCovered,
    /// `ds:SignedInfo`'s canonicalisation method is not exclusive canonicalisation (Annex B.1.0
    /// rule 3); the URI is named.
    #[error(
        "the signature's canonicalisation method is {0}, where ETSI TS 119 612 Annex B.1.0 rule 3 \
         requires exclusive canonicalisation"
    )]
    Canonicalization(String),
    /// An algorithm, transform or URI form this crate does not compute; it is named.
    #[error("the signature uses {0}, which this program does not compute")]
    Unsupported(String),
    /// A reference's digest does not match the data it names (section 3.2.1).
    #[error("the digest of the signature's reference {0:?} does not match what it covers")]
    DigestDiffers(String),
    /// The signature value does not verify over `ds:SignedInfo` under the certificate's key
    /// (section 3.2.2).
    #[error("the signature value does not verify under the certificate's key")]
    SignatureDiffers,
    /// The signed `SigningCertificateV2` names another certificate (EN 319 132-1 clause 5.2.2).
    #[error(
        "the signed properties name another signing certificate than the one in KeyInfo (ETSI \
         EN 319 132-1 clause 5.2.2)"
    )]
    SigningCertificateDiffers,
}

impl From<XmlError> for XmlSignatureError {
    fn from(error: XmlError) -> Self {
        match error {
            XmlError::Unreadable(reason) => Self::Unreadable(reason),
            XmlError::NestedTooDeep => Self::Unreadable("elements nest too deep".to_owned()),
            XmlError::TooManyEntries => Self::Unreadable("too many elements".to_owned()),
        }
    }
}

impl From<PolicyDocumentError> for XmlSignatureError {
    fn from(error: PolicyDocumentError) -> Self {
        match error {
            PolicyDocumentError::Xml(reason) => Self::Unreadable(reason),
            other => Self::Unreadable(other.to_string()),
        }
    }
}

/// A list whose enveloped signature verified: the certificate that signed it, as DER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// The `ds:X509Certificate` of `ds:KeyInfo` — TS 119 615's `LOTL-Signer-Cert` or
    /// `EUTL-Signer-Cert`.
    pub signer: Vec<u8>,
}

/// The certificate in a list's `ds:KeyInfo`, read without verifying anything, which TS 119 615
/// PRO-4.2.4-06 and -07 compare with a pointer's set before the signature is validated.
///
/// # Errors
///
/// As [`verify_enveloped`] for the XML, the signature's absence and `ds:KeyInfo`.
pub fn signer_of(octets: &[u8], entries: usize) -> Result<Vec<u8>, XmlSignatureError> {
    let mut remaining = entries;
    let tree = xml::read_tree(octets, &mut remaining)?;
    let found = Found::in_tree(&tree)?;
    found.certificate()
}

/// XML Signature 1.1 section 3.2's core validation of the enveloped signature on the list in
/// `octets`, under the key of the one certificate its `ds:KeyInfo` carries, with `entries` as the
/// budget every element and declaration is charged to (the caller bounds the octets).
///
/// # Errors
///
/// [`XmlSignatureError`]: what stopped the validation, named; a digest or signature that does not
/// match is an error too, because nothing is built on a list whose validation did not pass.
pub fn verify_enveloped(octets: &[u8], entries: usize) -> Result<Verified, XmlSignatureError> {
    let mut remaining = entries;
    let tree = xml::read_tree(octets, &mut remaining)?;
    let found = Found::in_tree(&tree)?;
    let signer = found.certificate()?;
    let certificate =
        x509::parse(&signer).map_err(|error| XmlSignatureError::Certificate(error.to_string()))?;
    let signed_info = found.child(found.signature, "SignedInfo")?;
    let canonicalization = found.child(signed_info, "CanonicalizationMethod")?;
    let uri = algorithm(canonicalization)?;
    if uri != EXC_C14N {
        return Err(XmlSignatureError::Canonicalization(uri.to_owned()));
    }
    let method = Method::of(EXC_C14N, prefix_list(&found, canonicalization))
        .ok_or_else(|| XmlSignatureError::Unsupported(uri.to_owned()))?;
    let signature_method = algorithm(found.child(signed_info, "SignatureMethod")?)?;
    let (family, digest) = signature_algorithm(signature_method)?;

    // Section 3.2.1: every reference, before the signature value is computed over them.
    let mut covers_list = false;
    let mut signed_ids = Vec::new();
    let references = found.children(signed_info, "Reference");
    if references.is_empty() {
        return Err(XmlSignatureError::Malformed("Reference"));
    }
    for reference in references {
        let checked = found.reference(octets, reference, entries)?;
        covers_list |= checked.covers_list;
        if let Some(id) = checked.id {
            signed_ids.push(id);
        }
    }
    if !covers_list {
        return Err(XmlSignatureError::ListNotCovered);
    }
    found.signing_certificate(&signer, &signed_ids)?;

    // Section 3.2.2: `ds:SignedInfo` canonicalised by its own method, and the value verified.
    let mut canonical = Vec::new();
    let subset = Subset {
        apex: Target::Ordinal(found.index_of(signed_info)?),
        excluded: None,
        whole_document: false,
    };
    canonical_xml::canonicalize_subset(octets, &subset, &method, entries, &mut |piece| {
        canonical.extend_from_slice(piece);
    })?;
    let value = base64(&found.child(found.signature, "SignatureValue")?.text)
        .ok_or(XmlSignatureError::Malformed("SignatureValue"))?;
    let computed = digest.compute(&[&canonical]);
    let verifies = match (family, certificate.public_key) {
        (Family::Pkcs1, PublicKey::Rsa(key)) => pkcs1::verify(key, &value, digest, &computed)
            .map_err(|error| XmlSignatureError::Certificate(error.to_string()))?,
        // RFC 4055 section 1.2: a key restricted to RSASSA-PSS is the same RSA key, and it is
        // how Germany's list signer states its key.
        (Family::Pss, PublicKey::Rsa(key) | PublicKey::RsaPss { key, .. }) => {
            // RFC 6931 section 2.3.10: the hash names MGF1's too, and the salt is the hash's length.
            let parameters = pss::Parameters {
                hash: digest,
                mgf_hash: digest,
                salt_length: digest.compute(&[]).len(),
            };
            // RFC 4055 section 3.3: where the certificate states the parameters its key is
            // restricted to, the method's must be those.
            pss::admitted_by_key(parameters, certificate.public_key.pss_parameters()).map_err(
                |problem| {
                    XmlSignatureError::Unsupported(format!(
                        "{signature_method} under a key RFC 4055 section 3.3 restricts otherwise: \
                     {problem:?}"
                    ))
                },
            )?;
            pss::verify(key, &value, parameters, &computed)
                .map_err(|error| XmlSignatureError::Certificate(error.to_string()))?
        }
        (Family::Ecdsa, PublicKey::Ec(key)) => {
            ecdsa::verify_concatenated(key, &value, &computed)
                .map_err(|error| XmlSignatureError::Certificate(error.to_string()))?
        }
        _ => {
            return Err(XmlSignatureError::Unsupported(format!(
                "{signature_method} over the certificate's kind of key"
            )));
        }
    };
    if verifies {
        Ok(Verified { signer })
    } else {
        Err(XmlSignatureError::SignatureDiffers)
    }
}

/// The arithmetic a signature method names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Pkcs1,
    Pss,
    Ecdsa,
}

/// Section 6.4's signature methods this crate computes: RSA PKCS#1 v1.5 (section 6.4.2), ECDSA
/// (section 6.4.3), and RFC 6931 section 2.3.10's RSASSA-PSS without parameters.
fn signature_algorithm(uri: &str) -> Result<(Family, Digest), XmlSignatureError> {
    Ok(match uri {
        "http://www.w3.org/2000/09/xmldsig#rsa-sha1" => (Family::Pkcs1, Digest::Sha1),
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256" => (Family::Pkcs1, Digest::Sha256),
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384" => (Family::Pkcs1, Digest::Sha384),
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512" => (Family::Pkcs1, Digest::Sha512),
        "http://www.w3.org/2007/05/xmldsig-more#sha256-rsa-MGF1" => (Family::Pss, Digest::Sha256),
        "http://www.w3.org/2007/05/xmldsig-more#sha384-rsa-MGF1" => (Family::Pss, Digest::Sha384),
        "http://www.w3.org/2007/05/xmldsig-more#sha512-rsa-MGF1" => (Family::Pss, Digest::Sha512),
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha1" => (Family::Ecdsa, Digest::Sha1),
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha256" => (Family::Ecdsa, Digest::Sha256),
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha384" => (Family::Ecdsa, Digest::Sha384),
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha512" => (Family::Ecdsa, Digest::Sha512),
        other => return Err(XmlSignatureError::Unsupported(other.to_owned())),
    })
}

/// Section 6.2's message digests this crate computes.
fn digest_algorithm(uri: &str) -> Result<Digest, XmlSignatureError> {
    Ok(match uri {
        "http://www.w3.org/2000/09/xmldsig#sha1" => Digest::Sha1,
        "http://www.w3.org/2001/04/xmlenc#sha256" => Digest::Sha256,
        "http://www.w3.org/2001/04/xmldsig-more#sha384" => Digest::Sha384,
        "http://www.w3.org/2001/04/xmlenc#sha512" => Digest::Sha512,
        other => return Err(XmlSignatureError::Unsupported(other.to_owned())),
    })
}

/// An element's `Algorithm` attribute.
fn algorithm(element: &Element) -> Result<&str, XmlSignatureError> {
    attribute(element, "Algorithm").ok_or(XmlSignatureError::Malformed("Algorithm"))
}

/// An unprefixed attribute's value.
fn attribute<'e>(element: &'e Element, local: &str) -> Option<&'e str> {
    element
        .attributes
        .iter()
        .find(|(name, _)| name == local)
        .map(|(_, value)| value.as_str())
}

/// Exclusive canonicalisation's `InclusiveNamespaces` `PrefixList` (its section 4), where the
/// method element states one.
fn prefix_list<'t>(found: &Found<'t>, method: &Element) -> Option<&'t str> {
    method
        .children
        .iter()
        .filter_map(|&index| found.at(index))
        .find(|element| {
            element.namespace == canonical_xml::EXC_C14N_NAMESPACE
                && element.local == "InclusiveNamespaces"
        })
        .and_then(|element| attribute(element, "PrefixList"))
}

/// What one reference established.
struct Checked {
    /// Whether it is Annex B.1.0 rule 2's reference to the whole list.
    covers_list: bool,
    /// The identifier it named, where it named one.
    id: Option<String>,
}

/// The signature located in the tree.
struct Found<'t> {
    tree: &'t Tree,
    signature: &'t Element,
}

impl<'t> Found<'t> {
    /// The one `ds:Signature` among the root's children (Annex B.1.0 rule 1's enveloped signature).
    fn in_tree(tree: &'t Tree) -> Result<Self, XmlSignatureError> {
        let root = tree
            .elements
            .first()
            .ok_or(XmlSignatureError::Malformed("root"))?;
        let mut signatures = root
            .children
            .iter()
            .filter_map(|&index| tree.elements.get(index))
            .filter(|child| child.namespace == DSIG && child.local == "Signature");
        let signature = signatures.next().ok_or(XmlSignatureError::Unsigned)?;
        if signatures.next().is_some() {
            return Err(XmlSignatureError::Ambiguous("ds:Signature".to_owned()));
        }
        Ok(Self { tree, signature })
    }

    fn at(&self, index: usize) -> Option<&'t Element> {
        self.tree.elements.get(index)
    }

    /// The element's index in the tree, which is its start tag's ordinal.
    fn index_of(&self, element: &Element) -> Result<usize, XmlSignatureError> {
        self.tree
            .elements
            .iter()
            .position(|candidate| std::ptr::eq(candidate, element))
            .ok_or(XmlSignatureError::Malformed("element"))
    }

    /// Every child of `parent` in XML Signature's namespace named `local`.
    fn children(&self, parent: &Element, local: &str) -> Vec<&'t Element> {
        self.children_in(parent, DSIG, local)
    }

    fn children_in(&self, parent: &Element, namespace: &str, local: &str) -> Vec<&'t Element> {
        parent
            .children
            .iter()
            .filter_map(|&index| self.at(index))
            .filter(|child| child.namespace == namespace && child.local == local)
            .collect()
    }

    fn child(
        &self,
        parent: &Element,
        local: &'static str,
    ) -> Result<&'t Element, XmlSignatureError> {
        self.children(parent, local)
            .into_iter()
            .next()
            .ok_or(XmlSignatureError::Malformed(local))
    }

    /// Clause 5.7.1's one certificate in `ds:KeyInfo`, as DER.
    fn certificate(&self) -> Result<Vec<u8>, XmlSignatureError> {
        let key_info = self.child(self.signature, "KeyInfo")?;
        let certificates: Vec<&Element> = self
            .children(key_info, "X509Data")
            .into_iter()
            .flat_map(|data| self.children(data, "X509Certificate"))
            .collect();
        match certificates.as_slice() {
            [one] => base64(&one.text).ok_or(XmlSignatureError::Malformed("X509Certificate")),
            other => Err(XmlSignatureError::Certificates(other.len())),
        }
    }

    /// The one element carrying the unprefixed attribute `Id` with `id`: XML Signature section
    /// 4.4.3.3's bare-name reference, which `XAdES` and TS 119 612's schemas declare as `xsd:ID`.
    fn identified(&self, id: &str) -> Result<usize, XmlSignatureError> {
        let mut carriers = self
            .tree
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| attribute(element, "Id") == Some(id))
            .map(|(index, _)| index);
        let first = carriers
            .next()
            .ok_or(XmlSignatureError::Malformed("Reference URI"))?;
        if carriers.next().is_some() {
            return Err(XmlSignatureError::Ambiguous(format!(
                "the identifier {id:?}"
            )));
        }
        Ok(first)
    }

    /// Section 3.2.1 for one `ds:Reference`: its data dereferenced (section 4.4.3.3), its
    /// transforms applied (section 4.4.3.2), and the digest compared.
    fn reference(
        &self,
        octets: &[u8],
        reference: &Element,
        entries: usize,
    ) -> Result<Checked, XmlSignatureError> {
        let uri = attribute(reference, "URI").ok_or_else(|| {
            XmlSignatureError::Unsupported("a reference without a URI".to_owned())
        })?;
        let root_id = self
            .tree
            .elements
            .first()
            .and_then(|root| attribute(root, "Id"));
        let (apex, whole_document, id) = if uri.is_empty() {
            (0, true, None)
        } else if let Some(id) = uri.strip_prefix('#') {
            if id.starts_with("xpointer(") {
                return Err(XmlSignatureError::Unsupported(format!("the URI {uri:?}")));
            }
            (self.identified(id)?, false, Some(id.to_owned()))
        } else {
            return Err(XmlSignatureError::Unsupported(format!(
                "the URI {uri:?}, which is not in the list itself"
            )));
        };
        let transforms: Vec<&str> = match self.children(reference, "Transforms").as_slice() {
            [] => Vec::new(),
            [one] => self
                .children(one, "Transform")
                .into_iter()
                .map(algorithm)
                .collect::<Result<_, _>>()?,
            _ => return Err(XmlSignatureError::Malformed("Transforms")),
        };
        let mut excluded = None;
        // Section 4.4.3.2: a node set that reaches the digest is converted by Canonical XML 1.0,
        // so that is the method where no canonicalisation ends the chain.
        let mut method = Method::plain(Algorithm::Canonical10, Comments::Omitted);
        let last = transforms.len().saturating_sub(1);
        for (at, transform) in transforms.iter().enumerate() {
            if *transform == ENVELOPED_SIGNATURE {
                excluded = Some(self.index_of(self.signature)?);
                continue;
            }
            let transform_element = self
                .children(reference, "Transforms")
                .first()
                .and_then(|list| self.children(list, "Transform").get(at).copied());
            let prefixes = transform_element.and_then(|element| prefix_list(self, element));
            match Method::of(transform, prefixes) {
                // A canonicalisation's output is octets, so only the last transform may be one;
                // and section 4.4.3.3 has already removed the comments either URI form names.
                Some(chosen) if at == last => {
                    method = Method {
                        comments: Comments::Omitted,
                        ..chosen
                    };
                }
                _ => {
                    return Err(XmlSignatureError::Unsupported(format!(
                        "the transform {transform}"
                    )));
                }
            }
        }
        let digest = digest_algorithm(algorithm(self.child(reference, "DigestMethod")?)?)?;
        let stated = base64(&self.child(reference, "DigestValue")?.text)
            .ok_or(XmlSignatureError::Malformed("DigestValue"))?;
        let subset = Subset {
            apex: Target::Ordinal(apex),
            excluded,
            whole_document,
        };
        let mut hasher = digest.hasher();
        canonical_xml::canonicalize_subset(octets, &subset, &method, entries, &mut |piece| {
            hasher.update(piece);
        })?;
        if hasher.finish() != stated {
            return Err(XmlSignatureError::DigestDiffers(uri.to_owned()));
        }
        // Annex B.1.0 rule 2: the list itself, enveloped transform first, exclusive
        // canonicalisation second, and nothing else.
        let names_list = apex == 0
            && (uri.is_empty() || root_id.is_some_and(|root| Some(root) == id.as_deref()));
        let covers_list = names_list && transforms == [ENVELOPED_SIGNATURE, EXC_C14N];
        Ok(Checked { covers_list, id })
    }

    /// EN 319 132-1 clause 5.2.2: where a signed `SignedProperties` states a signing certificate
    /// reference, its first is the certificate in `ds:KeyInfo`.
    fn signing_certificate(
        &self,
        signer: &[u8],
        signed_ids: &[String],
    ) -> Result<(), XmlSignatureError> {
        for element in &self.tree.elements {
            let is_signed = element.namespace == XADES
                && element.local == "SignedProperties"
                && attribute(element, "Id").is_some_and(|id| signed_ids.iter().any(|s| s == id));
            if !is_signed {
                continue;
            }
            for properties in self.children_in(element, XADES, "SignedSignatureProperties") {
                let references = self
                    .children_in(properties, XADES, "SigningCertificateV2")
                    .into_iter()
                    .chain(self.children_in(properties, XADES, "SigningCertificate"));
                for reference in references {
                    let first = self
                        .children_in(reference, XADES, "Cert")
                        .into_iter()
                        .next()
                        .ok_or(XmlSignatureError::Malformed("Cert"))?;
                    let digest_of = self
                        .children_in(first, XADES, "CertDigest")
                        .into_iter()
                        .next()
                        .ok_or(XmlSignatureError::Malformed("CertDigest"))?;
                    let digest =
                        digest_algorithm(algorithm(self.child(digest_of, "DigestMethod")?)?)?;
                    let stated = base64(&self.child(digest_of, "DigestValue")?.text)
                        .ok_or(XmlSignatureError::Malformed("DigestValue"))?;
                    if digest.compute(&[signer]) != stated {
                        return Err(XmlSignatureError::SigningCertificateDiffers);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests;
